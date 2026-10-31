"""Shared driver for train_lesions.py and train_vessels.py (patch-based segmentation fine-tuning)."""
from __future__ import annotations

import argparse
from pathlib import Path

import torch
import yaml
from torch.utils.data import DataLoader

from trainlab.data import splits as S
from trainlab.data.datasets import SegDataset, read_manifest
from trainlab.models.registry import build_seg_model
from trainlab.train import segmentation as SG
from trainlab.train.engine import FitConfig, fit
from trainlab.utils.device import pick_device
from trainlab.utils.logging import make_run_dir
from trainlab.utils.seeds import seed_everything

LAB = Path(__file__).resolve().parents[2]


def add_common_args(ap: argparse.ArgumentParser, default_cfg: str) -> None:
    ap.add_argument("--config", default=str(LAB / "configs" / default_cfg))
    for k, t in (("epochs", int), ("batch_size", int), ("patch", int), ("lr", float), ("scale", float), ("seed", int),
                 ("encoder", str), ("patches_per_image", int), ("patience", int), ("num_workers", int)):
        ap.add_argument(f"--{k.replace('_', '-')}", type=t)
    ap.add_argument("--no-pretrained", action="store_true")
    ap.add_argument("--smoke", action="store_true", help="3 steps x 1 epoch; plumbing check only, NOT training evidence")
    ap.add_argument("--cpu", action="store_true")


def run(a: argparse.Namespace, tr, va, task: str, tag: str) -> dict:
    """tr/va: manifest DataFrames with image_path + mask_path (rows with a real mask only)."""
    cfg = yaml.safe_load(Path(a.config).read_text())
    cfg.update({k: v for k, v in vars(a).items() if v is not None and k in cfg})
    if a.no_pretrained:
        cfg["pretrained"] = False
    if len(tr) == 0 or len(va) == 0:
        raise RuntimeError(f"{task}/{tag}: need >=1 train and >=1 val image with a real mask (got {len(tr)}/{len(va)})")
    S.assert_no_leakage({"train": tr, "val": va})
    seed_everything(cfg["seed"])
    device = pick_device("cpu" if a.cpu else "cuda")
    nw = cfg["num_workers"]
    ppi = 2 if a.smoke else cfg["patches_per_image"]
    train_ds = SegDataset(tr, cfg["patch"], True, cfg["scale"], ppi, cfg["pos_bias"], cfg["seed"])
    val_ds = SegDataset(va, cfg["patch"], False, cfg["scale"])
    train_loader = DataLoader(train_ds, batch_size=cfg["batch_size"], shuffle=True, drop_last=len(train_ds) >= cfg["batch_size"], num_workers=nw,
                              pin_memory=device.type == "cuda", persistent_workers=nw > 0)
    val_loader = DataLoader(val_ds, batch_size=cfg["batch_size"], shuffle=False, num_workers=0)
    run_dir = make_run_dir(LAB / "experiments" / task, f"{cfg['name']}_{tag}" + ("_SMOKE" if a.smoke else ""))
    ckpt_dir = LAB / "checkpoints" / task / run_dir.name
    fcfg = FitConfig(epochs=1 if a.smoke else cfg["epochs"], lr=cfg["lr"], weight_decay=cfg["weight_decay"], amp=cfg["amp"], grad_accum=cfg["grad_accum"],
                     warmup_epochs=cfg["warmup_epochs"], freeze_epochs=cfg["freeze_epochs"], patience=cfg["patience"], monitor="val_dice",
                     max_steps_per_epoch=3 if a.smoke else None, seed=cfg["seed"])
    meta = {"task": f"{task}:{tag}", "architecture": f"UNet({cfg['encoder']})", "checkpoint_source": "timm pretrained ImageNet encoder" if cfg["pretrained"] else "random init",
            "NOT_production_model": True, "patch": cfg["patch"], "scale": cfg["scale"], "batch_size": cfg["batch_size"],
            "precision": "fp16-autocast" if cfg["amp"] and device.type == "cuda" else "fp32", "train_images": len(tr), "val_images": len(va),
            "train_patches_per_epoch": len(train_ds), "val_tiles": len(val_ds), "torch": torch.__version__}
    model = build_seg_model(cfg["encoder"], cfg["pretrained"])
    info = fit(model, train_loader, val_loader, SG.make_loss_fn(), SG.make_eval_fn(), fcfg, run_dir, ckpt_dir, device, meta,
               kind="smoke" if a.smoke else "train")
    print(f"{info['kind']} run {run_dir.name}: status={info['status']} epochs={info['epochs_run']} steps={info['steps']} "
          f"best_val_dice={info.get('best_val_dice')} peak_vram_mib={info['gpu_peak'].get('peak_allocated_mib')}")
    return info
