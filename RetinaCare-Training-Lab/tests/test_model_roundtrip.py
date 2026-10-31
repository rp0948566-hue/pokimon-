import json

import numpy as np
import torch
from torch.utils.data import DataLoader, TensorDataset

from trainlab.export.onnx_export import export_onnx
from trainlab.export.parity import parity_check
from trainlab.models.registry import UNet, build_grading_model, set_backbone_trainable
from trainlab.train import grading as G
from trainlab.train import segmentation as SG
from trainlab.train.engine import FitConfig, fit


def _loader(n=24, size=32, seed=0):
    g = torch.Generator().manual_seed(seed)
    y = torch.arange(n) % 5
    x = torch.randn(n, 3, size, size, generator=g) + y.view(-1, 1, 1, 1).float()  # separable synthetic toy
    return DataLoader(TensorDataset(x, y), batch_size=8, shuffle=True)


def test_fit_runs_real_gradient_steps_saves_resumable_checkpoints(tmp_path):
    model = build_grading_model("resnet18", 5, pretrained=False)
    w0 = model.fc.weight.detach().clone()
    cfg = FitConfig(epochs=2, lr=1e-3, amp=False, warmup_epochs=0, freeze_epochs=1, patience=5)
    info = fit(model, _loader(), _loader(seed=1), G.make_loss_fn(None), G.eval_fn, cfg, tmp_path / "run", tmp_path / "ck", torch.device("cpu"), {"t": 1})
    assert info["status"] == "completed" and info["epochs_run"] == 2 and info["steps"] == 6
    assert not torch.equal(w0, model.fc.weight)  # weights actually changed
    assert (tmp_path / "run/train.csv").exists() and (tmp_path / "run/val.csv").exists() and (tmp_path / "run/training_curves.png").exists()
    st = torch.load(tmp_path / "ck/best.ckpt", weights_only=False)
    m2 = build_grading_model("resnet18", 5, pretrained=False); m2.load_state_dict(st["model"])
    assert json.loads((tmp_path / "run/run_info.json").read_text())["best_ckpt"]["sha256"]
    info2 = fit(m2, _loader(), _loader(seed=1), G.make_loss_fn(None), G.eval_fn, FitConfig(epochs=3, lr=1e-3, amp=False, warmup_epochs=0),
                tmp_path / "run2", tmp_path / "ck", torch.device("cpu"), resume=True)
    assert info2["epochs_run"] == 1  # resumed after epoch 1, ran epoch 2 only


def test_backbone_freeze_keeps_head_trainable():
    m = build_grading_model("resnet18", 5, pretrained=False)
    n = set_backbone_trainable(m, False)
    assert n == sum(p.numel() for p in m.fc.parameters()) and all(not p.requires_grad for p in m.layer1.parameters())


def test_unet_shape_and_loss_decreases_on_toy_segmentation():
    torch.manual_seed(0)
    net = UNet("resnet18", pretrained=False)
    x = torch.randn(2, 3, 64, 64)
    y = (x[:, :1] > 0.5).float()
    assert net(x).shape == (2, 1, 64, 64)
    opt = torch.optim.Adam(net.parameters(), 1e-2)
    lf = SG.make_loss_fn()
    first = float(lf(net, (x, y), torch.device("cpu"))); 
    for _ in range(15):
        opt.zero_grad(); loss = lf(net, (x, y), torch.device("cpu")); loss.backward(); opt.step()
    assert float(loss) < first


def test_onnx_export_parity_and_refuses_production_path(tmp_path):
    m = build_grading_model("resnet18", 5, pretrained=False)
    out = export_onnx(m, torch.zeros(1, 3, 64, 64), tmp_path / "m.onnx")
    par = parity_check(m, out, [np.random.default_rng(i).standard_normal((1, 3, 64, 64)).astype(np.float32) for i in range(2)])
    assert par["passed"] and par["max_abs_logit_diff"] < 1e-3
    import pytest
    with pytest.raises(ValueError, match="production"):
        export_onnx(m, torch.zeros(1, 3, 64, 64), "/home/claude/Desktop/Dird/models/x.onnx")
