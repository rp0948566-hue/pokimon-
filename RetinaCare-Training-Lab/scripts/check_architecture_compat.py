#!/usr/bin/env python
"""Phase 0/5: prove which deployed PyTorch checkpoints load STRICTLY into the upstream architecture (read-only).

For each deployed stage this builds the upstream model (MIT code in ai_engine/.venv/hierarchiretina-src, no
ImageNet download), loads the .pth/.pt with strict=True, counts parameters and compares output against the
deployed ONNX on one seeded input. Checkpoints are the user's own sha256-verified local files, so
weights_only=False is used (pickle). Nothing is written to the models directory.
Writes reports/architecture_compat.json.
"""
from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

import numpy as np  # noqa: E402
import torch  # noqa: E402

from trainlab.models.adapters import BACKEND, UPSTREAM_SRC  # noqa: E402

sys.path.insert(0, str(UPSTREAM_SRC))
M = BACKEND / "models"


def onnx_diff(model, onnx_path: Path, x: torch.Tensor, pick=lambda o: o) -> float:
    import onnxruntime as ort

    sess = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])
    with torch.no_grad():
        ref = pick(model(x)).float().numpy()
    got = sess.run(None, {"image": x.numpy()})[0]
    return float(np.abs(ref - got).max())


def stage1() -> dict:
    from hierarchiretina.stage1.config import get_preset
    from hierarchiretina.stage1.model import build_model, count_parameters, load_eval_weights

    cfg = get_preset("convnextv2_768")
    model = build_model(cfg, pretrained=False)
    meta = load_eval_weights(model, M / "stage1_convnextv2l_768_best_auc_model.pth", strict=True)
    x = torch.randn(1, 3, cfg.img_size, cfg.img_size, generator=torch.Generator().manual_seed(0))
    return {"params": count_parameters(model), "img_size": cfg.img_size, "ckpt_meta": {k: v for k, v in meta.items() if isinstance(v, (int, float, str, bool))},
            "onnx_max_abs_logit_diff": onnx_diff(model, M / "stage1_convnextv2l_768/stage1.onnx", x)}


def lesion(kind: str) -> dict:
    from hierarchiretina.stage2.hsmoe_aunet import count_parameters, load_hsmoe_aunet

    d = M / "stage2_hsmoe_aunet" / kind.lower()
    model = load_hsmoe_aunet(kind, str(next(d.glob("*.pth"))), strict=True).eval()
    onnx = d / f"stage2_{kind.lower()}.onnx"
    x = torch.randn(1, 3, 1024, 1024, generator=torch.Generator().manual_seed(0))
    pick = lambda o: o[0] if isinstance(o, (tuple, list)) else (o["main"] if isinstance(o, dict) else o)  # noqa: E731
    return {"params_total_trainable": count_parameters(model), "onnx_max_abs_logit_diff": onnx_diff(model, onnx, x, pick)}


def vessel() -> dict:
    from hierarchiretina.stage2.engine_vessel import load_best_model_and_threshold
    from hierarchiretina.stage2.swinhrunetpp import build_swinhrunetpp

    model = build_swinhrunetpp(pretrained=False)
    model, thr, info = load_best_model_and_threshold(M / "stage2_swinhrunetpp/stage2_swinhrunetpp_best_dice.pth", model)
    x = torch.randn(1, 3, 512, 512, generator=torch.Generator().manual_seed(0))
    pick = lambda o: o[0] if isinstance(o, (tuple, list)) else o  # noqa: E731
    return {"params": sum(p.numel() for p in model.parameters()), "threshold": thr, "info": info,
            "onnx_max_abs_logit_diff": onnx_diff(model, M / "stage2_swinhrunetpp/stage2_vessel.onnx", x, pick)}


def stage3(fold: int) -> dict:
    from hierarchiretina.stage3.engine import LGDRGConfig
    from hierarchiretina.stage3.model import load_lgdrg

    model = load_lgdrg(M / f"stage3_lgdrg/stage3_lgdrg_best_fold{fold}.pt", LGDRGConfig())
    g = torch.Generator().manual_seed(0)
    x = torch.cat([torch.randn(1, 3, 512, 512, generator=g), (torch.rand(1, 5, 512, 512, generator=g) > 0.9).float()], 1)
    import onnxruntime as ort
    sess = ort.InferenceSession(str(M / f"stage3_lgdrg/stage3_lgdrg_fold{fold}.onnx"), providers=["CPUExecutionProvider"])
    with torch.no_grad():
        sev, grad = model(x)
    ref = torch.cat([sev, grad.reshape(1, -1)], 1).numpy()
    got = sess.run(None, {"image": x.numpy()})[0]
    return {"params": sum(p.numel() for p in model.parameters()), "onnx_max_abs_logit_diff": float(np.abs(ref - got).max())}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--only", nargs="*", help="subset: stage1 MA HE EX CWS vessel fold0..fold4")
    a = ap.parse_args()
    jobs = {"stage1": stage1, **{k: (lambda k=k: lesion(k)) for k in ("MA", "HE", "EX", "CWS")}, "vessel": vessel,
            **{f"fold{i}": (lambda i=i: stage3(i)) for i in range(5)}}
    out = {}
    for name, fn in jobs.items():
        if a.only and name not in a.only:
            continue
        t = time.time()
        try:
            out[name] = {"strict_load": True, **fn(), "seconds": round(time.time() - t, 1)}
        except Exception as e:  # report the exact incompatibility instead of hiding it
            out[name] = {"strict_load": False, "error": f"{type(e).__name__}: {str(e)[:400]}"}
        print(name, json.dumps(out[name])[:300], flush=True)
    (LAB / "reports/architecture_compat.json").write_text(json.dumps(out, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
