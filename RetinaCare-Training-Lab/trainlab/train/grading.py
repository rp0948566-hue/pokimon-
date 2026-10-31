"""Grading task: 5-class DR fine-tuning (loss + validation) on top of train/engine.fit."""
from __future__ import annotations

import numpy as np
import torch
import torch.nn.functional as F

from trainlab.utils.metrics import macro_f1, qwk


def class_weights(labels, n: int = 5) -> torch.Tensor:
    """Inverse-sqrt-frequency weights (mild rebalancing; absent classes get weight 1)."""
    cnt = np.bincount(np.asarray(labels, int), minlength=n).astype(float)
    w = np.where(cnt > 0, 1.0 / np.sqrt(np.maximum(cnt, 1)), 0.0)
    w = np.where(cnt > 0, w / w[cnt > 0].mean(), 1.0)
    return torch.tensor(w, dtype=torch.float32)


def make_loss_fn(weights: torch.Tensor | None, label_smoothing: float = 0.05):
    def loss_fn(model, batch, device):
        x, y = batch
        logits = model(x.to(device, non_blocking=True))
        w = weights.to(device) if weights is not None else None
        return F.cross_entropy(logits.float(), y.to(device), weight=w, label_smoothing=label_smoothing)
    return loss_fn


@torch.no_grad()
def predict_logits(model, loader, device, amp: bool = True):
    model.eval()
    outs, ys = [], []
    for x, y in loader:
        with torch.autocast(device.type, dtype=torch.float16, enabled=amp and device.type == "cuda"):
            outs.append(model(x.to(device)).float().cpu())
        ys.append(y)
    return torch.cat(outs), torch.cat(ys)


def eval_fn(model, loader, device) -> dict:
    logits, y = predict_logits(model, loader, device)
    pred = logits.argmax(1).numpy()
    y = y.numpy()
    loss = float(F.cross_entropy(logits, torch.from_numpy(y)))
    return {"val_loss": loss, "val_acc": float((pred == y).mean()), "val_qwk": qwk(y, pred), "val_macro_f1": macro_f1(y, pred),
            "val_n": int(len(y))}
