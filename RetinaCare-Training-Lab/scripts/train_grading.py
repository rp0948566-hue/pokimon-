#!/usr/bin/env python
"""P5: GPU fine-tuning of a 5-class DR grading classifier (transfer-learning candidate, NOT the deployed model).

Training loop lives in trainlab/train/engine.py (fit). --smoke caps steps and is labelled kind=smoke in
run_info.json: it is a plumbing check, never training evidence. Selection uses VALIDATION only.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

import yaml

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

import torch  # noqa: E402
from torch.utils.data import DataLoader  # noqa: E402

from trainlab.data import splits as S  # noqa: E402
from trainlab.data.datasets import GradingDataset, read_manifest  # noqa: E402
from trainlab.models.registry import build_grading_model  # noqa: E402
from trainlab.train import grading as G  # noqa: E402
from trainlab.train.engine import FitConfig, fit  # noqa: E402
from trainlab.utils.device import pick_device, probe_batch_size  # noqa: E402
from trainlab.utils.logging import make_run_dir  # noqa: E402
from trainlab.utils.seeds import seed_everything  # noqa: E402


def parse() -> argparse.Namespace:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--config", default=str(LAB / "configs/grading_aptos.yaml"))
    for k, t in (("epochs", int), ("image-size", int), ("lr", float), ("model", str), ("seed", int), ("grad-accum", int),
                 ("freeze-epochs", int), ("patience", int), ("num-workers", int), ("train-manifest", str), ("val-manifest", str), ("name", str)):
        ap.add_argument(f"--{k}", type=t)
    ap.add_argument("--batch-size", help="int or 'auto' (measured VRAM probe)")
    ap.add_argument("--no-pretrained", action="store_true")
    ap.add_argument("--resume", metavar="RUN_NAME", help="resume last.ckpt of an earlier run directory name")
    ap.add_argument("--smoke", action="store_true", help="3 steps x 1 epoch on <=64 TRAIN images; plumbing check only")
    ap.add_argument("--cpu", action="store_true")
    return ap.parse_args()


def main() -> int:
    a = parse()
    cfg = yaml.safe_load(Path(a.config).read_text())
    cfg.update({k: v for k, v in vars(a).items() if v is not None and k in cfg})
    if a.no_pretrained:
        cfg["pretrained"] = False
    seed_everything(cfg["seed"])
    device = pick_device("cpu" if a.cpu else "cuda")
    tr = read_manifest(LAB / cfg["train_manifest"], ("prepared_path", "label", "split"))
    va = read_manifest(LAB / cfg["val_manifest"], ("prepared_path", "label", "split"))
    S.assert_no_leakage({"train": tr, "val": va})
    if a.smoke:
        tr, va = tr.sample(n=min(64, len(tr)), random_state=0), va.sample(n=min(32, len(va)), random_state=0)
        cfg["epochs"] = 1
    size, nw = cfg["image_size"], cfg["num_workers"]
    mk = lambda df, train, bs: DataLoader(GradingDataset(df, size, train, cfg["seed"]), batch_size=bs, shuffle=train, drop_last=train,  # noqa: E731
                                          num_workers=nw, pin_memory=device.type == "cuda", persistent_workers=nw > 0)
    new_model = lambda: build_grading_model(cfg["model"], 5, cfg["pretrained"])  # noqa: E731
    bs = cfg["batch_size"]
    probe = None
    if str(bs) == "auto":
        probe = probe_batch_size(new_model, lambda b: (torch.randn(b, 3, size, size), None), device=device, amp=cfg["amp"])
        bs = probe["batch_size"]
    bs = int(bs)
    name = cfg["name"] + ("_SMOKE" if a.smoke else "")
    run_dir = LAB / "experiments/grading" / (a.resume or "") if a.resume else make_run_dir(LAB / "experiments/grading", name)
    ckpt_dir = LAB / "checkpoints/grading" / run_dir.name
    train_loader, val_loader = mk(tr, True, bs), mk(va, False, max(bs, 4))
    fcfg = FitConfig(epochs=cfg["epochs"], lr=cfg["lr"], weight_decay=cfg["weight_decay"], amp=cfg["amp"], grad_accum=cfg["grad_accum"],
                     warmup_epochs=cfg["warmup_epochs"], freeze_epochs=cfg["freeze_epochs"], patience=cfg["patience"], monitor="val_qwk",
                     max_steps_per_epoch=3 if a.smoke else None, seed=cfg["seed"])
    meta = {"task": "grading_5class", "architecture": cfg["model"], "checkpoint_source": "timm pretrained ImageNet weights" if cfg["pretrained"] else "random init",
            "NOT_production_model": True, "image_size": size, "batch_size": bs, "grad_accum": cfg["grad_accum"], "precision": "fp16-autocast" if fcfg.amp and device.type == "cuda" else "fp32",
            "train_samples": len(tr), "val_samples": len(va), "train_manifest": cfg["train_manifest"], "val_manifest": cfg["val_manifest"],
            "batch_probe": probe, "torch": torch.__version__}
    model = new_model()
    info = fit(model, train_loader, val_loader, G.make_loss_fn(G.class_weights(tr["label"])), G.eval_fn, fcfg, run_dir, ckpt_dir, device,
               meta, resume=bool(a.resume), label_dist={"train": S.class_distribution(tr), "val": S.class_distribution(va)},
               kind="smoke" if a.smoke else "train")
    print(f"{info['kind']} run {run_dir.name}: status={info['status']} epochs={info['epochs_run']} steps={info['steps']} "
          f"best_val_qwk={info.get('best_val_qwk')} peak_vram_mib={info['gpu_peak'].get('peak_allocated_mib')}")
    return 0 if not info["status"].startswith("failed") else 1


if __name__ == "__main__":
    sys.exit(main())
