"""Generic GPU training loop: AMP, grad accumulation, cosine+warmup, early stopping, resume, CSV logs.

This file contains the actual gradient-descent loop used by grading, lesion and vessel training.
"""
from __future__ import annotations

import json
import math
import signal
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Callable

import torch
import yaml

from trainlab.models.registry import set_backbone_trainable
from trainlab.utils.device import gpu_snapshot, nvidia_smi
from trainlab.utils.hashing import sha256_file
from trainlab.utils.logging import CsvLogger


@dataclass
class FitConfig:
    epochs: int = 5
    lr: float = 1e-4
    weight_decay: float = 1e-2
    amp: bool = True
    grad_accum: int = 1
    grad_clip: float = 1.0
    warmup_epochs: int = 1
    freeze_epochs: int = 0          # epochs with backbone frozen (head only) before unfreezing
    patience: int = 4
    monitor: str = "val_qwk"        # key returned by eval_fn; higher is better
    max_steps_per_epoch: int | None = None  # None = full epoch. Set only for smoke runs.
    max_oom_skips: int = 5
    seed: int = 42
    extra: dict = field(default_factory=dict)


class Cancelled(Exception):
    pass


def _lr_at(epoch_f: float, cfg: FitConfig) -> float:
    if epoch_f < cfg.warmup_epochs:
        return cfg.lr * (epoch_f + 1e-3) / cfg.warmup_epochs
    t = (epoch_f - cfg.warmup_epochs) / max(cfg.epochs - cfg.warmup_epochs, 1e-9)
    return cfg.lr * (0.01 + 0.99 * 0.5 * (1 + math.cos(math.pi * min(t, 1.0))))


def _save(path: Path, model, opt, scaler, epoch: int, best: float, cfg: FitConfig, meta: dict) -> None:
    tmp = path.with_suffix(".tmp")
    torch.save({"model": model.state_dict(), "opt": opt.state_dict(), "scaler": scaler.state_dict(),
                "epoch": epoch, "best": best, "cfg": asdict(cfg), "meta": meta}, tmp)
    tmp.replace(path)


def fit(model, train_loader, val_loader, loss_fn: Callable, eval_fn: Callable, cfg: FitConfig,
        run_dir, ckpt_dir, device, meta: dict | None = None, resume: bool = False,
        label_dist: dict | None = None, kind: str = "train") -> dict:
    """Train; return run_info dict (also written to run_dir/run_info.json).

    loss_fn(model, batch, device) -> scalar loss tensor.
    eval_fn(model, val_loader, device) -> dict of floats containing cfg.monitor.
    kind: "train" for real runs, "smoke" if the caller capped steps (labelled in run_info).
    """
    run_dir, ckpt_dir = Path(run_dir), Path(ckpt_dir)
    run_dir.mkdir(parents=True, exist_ok=True)
    ckpt_dir.mkdir(parents=True, exist_ok=True)
    meta = meta or {}
    model.to(device)
    use_amp = cfg.amp and device.type == "cuda"
    params = [p for p in model.parameters()]
    opt = torch.optim.AdamW(params, lr=cfg.lr, weight_decay=cfg.weight_decay)
    scaler = torch.amp.GradScaler("cuda", enabled=use_amp)
    train_log, val_log = CsvLogger(run_dir / "train.csv"), CsvLogger(run_dir / "val.csv")
    (run_dir / "config_resolved.yaml").write_text(yaml.safe_dump({"fit": asdict(cfg), "meta": meta}, sort_keys=False))
    if label_dist is not None:
        (run_dir / "label_distributions.json").write_text(json.dumps(label_dist, indent=1))

    start_epoch, best, bad = 0, -float("inf"), 0
    last_ckpt, best_ckpt = ckpt_dir / "last.ckpt", ckpt_dir / "best.ckpt"
    if resume and last_ckpt.exists():
        st = torch.load(last_ckpt, map_location=device, weights_only=False)
        model.load_state_dict(st["model"]); opt.load_state_dict(st["opt"]); scaler.load_state_dict(st["scaler"])
        start_epoch, best = st["epoch"] + 1, st["best"]

    cancel = {"flag": False}
    old = signal.signal(signal.SIGINT, lambda *_: cancel.__setitem__("flag", True))
    if device.type == "cuda":
        torch.cuda.reset_peak_memory_stats()
    info = {"kind": kind, "status": "running", "started": time.strftime("%Y-%m-%dT%H:%M:%S"), "device": str(device),
            "amp": use_amp, "meta": meta, "epochs_planned": cfg.epochs, "epochs_run": 0, "steps": 0,
            "samples_seen": 0, "oom_skips": 0, "gpu_start": nvidia_smi()}
    t0 = time.time()
    status = "completed"
    try:
        for epoch in range(start_epoch, cfg.epochs):
            if hasattr(train_loader.dataset, "set_epoch"):
                train_loader.dataset.set_epoch(epoch)
            set_backbone_trainable(model, epoch >= cfg.freeze_epochs)
            model.train()
            tot, n_b, ep_t0 = 0.0, 0, time.time()
            opt.zero_grad(set_to_none=True)
            for i, batch in enumerate(train_loader):
                if cancel["flag"]:
                    raise Cancelled()
                if cfg.max_steps_per_epoch and i >= cfg.max_steps_per_epoch:
                    break
                for g in opt.param_groups:
                    g["lr"] = _lr_at(epoch + i / max(len(train_loader), 1), cfg)
                try:
                    with torch.autocast(device.type, dtype=torch.float16, enabled=use_amp):
                        loss = loss_fn(model, batch, device) / cfg.grad_accum
                    scaler.scale(loss).backward()
                except torch.cuda.OutOfMemoryError:
                    opt.zero_grad(set_to_none=True)
                    torch.cuda.empty_cache()
                    info["oom_skips"] += 1
                    if info["oom_skips"] > cfg.max_oom_skips:
                        raise RuntimeError("repeated CUDA OOM: lower batch size / image size / patch size") from None
                    continue
                if (i + 1) % cfg.grad_accum == 0:
                    scaler.unscale_(opt)
                    torch.nn.utils.clip_grad_norm_(params, cfg.grad_clip)
                    scaler.step(opt); scaler.update(); opt.zero_grad(set_to_none=True)
                tot += float(loss.detach()) * cfg.grad_accum
                n_b += 1
                info["steps"] += 1
                info["samples_seen"] += int(batch[0].shape[0])
            if n_b == 0:
                raise RuntimeError("no batches processed in epoch")
            model.eval()
            with torch.no_grad():
                vm = eval_fn(model, val_loader, device)
            if cfg.monitor not in vm:
                raise KeyError(f"eval_fn must return '{cfg.monitor}', got {list(vm)}")
            gpu = gpu_snapshot()
            train_log.write({"epoch": epoch, "train_loss": tot / n_b, "lr": opt.param_groups[0]["lr"], "batches": n_b,
                             "seconds": round(time.time() - ep_t0, 1), "peak_vram_mib": gpu.get("peak_allocated_mib")})
            val_log.write({"epoch": epoch, **{k: v for k, v in vm.items() if isinstance(v, (int, float))}})
            info["epochs_run"] += 1
            score = float(vm[cfg.monitor])
            _save(last_ckpt, model, opt, scaler, epoch, max(best, score), cfg, meta)
            if score > best:
                best, bad = score, 0
                _save(best_ckpt, model, opt, scaler, epoch, best, cfg, meta)
                info["best_epoch"] = epoch
            else:
                bad += 1
                if bad >= cfg.patience:
                    status = "early_stopped"
                    break
    except Cancelled:
        status = "cancelled"
    except Exception as e:
        status = f"failed: {type(e).__name__}: {e}"
        raise
    finally:
        signal.signal(signal.SIGINT, old)
        info.update({"status": status, "finished": time.strftime("%Y-%m-%dT%H:%M:%S"),
                     "seconds": round(time.time() - t0, 1), "best_" + cfg.monitor: None if best == -float("inf") else best,
                     "gpu_peak": gpu_snapshot(), "gpu_end": nvidia_smi()})
        for tag, p in (("best", best_ckpt), ("last", last_ckpt)):
            if p.exists():
                info[f"{tag}_ckpt"] = {"path": str(p), "sha256": sha256_file(p), "bytes": p.stat().st_size}
                link = run_dir / f"{tag}.ckpt"
                if link.is_symlink() or link.exists():
                    link.unlink()
                link.symlink_to(p)
        try:
            from trainlab.evaluate.plots import training_curves
            training_curves(run_dir)
        except Exception as e:  # plotting must never hide the training result
            info["plot_error"] = str(e)
        (run_dir / "run_info.json").write_text(json.dumps(info, indent=1, default=str))
    return info
