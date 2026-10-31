"""Segmentation task (MA / HE / EX / SE lesions, vessels): BCE+Dice loss, tile-based pixel-count validation."""
from __future__ import annotations

import torch
import torch.nn.functional as F

from trainlab.utils.metrics import counts_to_scores


def dice_loss(logits: torch.Tensor, target: torch.Tensor, eps: float = 1.0) -> torch.Tensor:
    p = torch.sigmoid(logits)
    inter = (p * target).sum((1, 2, 3))
    return (1 - (2 * inter + eps) / (p.sum((1, 2, 3)) + target.sum((1, 2, 3)) + eps)).mean()


def make_loss_fn(pos_weight: float = 1.0, dice_w: float = 0.5):
    def loss_fn(model, batch, device):
        x, y = batch
        logits = model(x.to(device, non_blocking=True)).float()
        y = y.to(device)
        bce = F.binary_cross_entropy_with_logits(logits, y, pos_weight=torch.tensor(pos_weight, device=device))
        return (1 - dice_w) * bce + dice_w * dice_loss(logits, y)
    return loss_fn


def make_eval_fn(threshold: float = 0.5):
    """Dice/IoU pooled over all val tiles (micro). Tiles of images with empty GT still count for FP."""
    def eval_fn(model, loader, device) -> dict:
        tp = fp = fn = 0
        vloss, n = 0.0, 0
        use_amp = device.type == "cuda"
        for x, y in loader:
            with torch.autocast(device.type, dtype=torch.float16, enabled=use_amp):
                logits = model(x.to(device)).float()
            y = y.to(device)
            vloss += float(F.binary_cross_entropy_with_logits(logits, y)); n += 1
            pred = torch.sigmoid(logits) >= threshold
            gt = y > 0.5
            tp += int((pred & gt).sum()); fp += int((pred & ~gt).sum()); fn += int((~pred & gt).sum())
        s = counts_to_scores(tp, fp, fn)
        return {"val_loss": vloss / max(n, 1), "val_dice": s["dice"] or 0.0, "val_iou": s["iou"] or 0.0,
                "val_sensitivity": s["sensitivity"] or 0.0, "val_precision": s["precision"] or 0.0}
    return eval_fn
