"""Segmentation evaluation on full-resolution-in-RAM images using the same tiling as validation."""
from __future__ import annotations

import numpy as np
import torch

from trainlab.data import transforms as T
from trainlab.utils.metrics import counts_to_scores


@torch.no_grad()
def predict_full(model, img_u8: np.ndarray, patch: int, device, amp: bool = True) -> np.ndarray:
    """Tile an HWC uint8 image, return float probability map (H, W); overlapping tiles are averaged."""
    h, w = img_u8.shape[:2]
    acc, cnt = np.zeros((h, w), np.float32), np.zeros((h, w), np.float32)
    model.eval()
    for y, x in T.tile_grid(h, w, patch):
        tile = img_u8[y:y + patch, x:x + patch]
        t = torch.from_numpy(T.to_tensor_array(tile))[None].to(device)
        with torch.autocast(device.type, dtype=torch.float16, enabled=amp and device.type == "cuda"):
            p = torch.sigmoid(model(t).float())[0, 0].cpu().numpy()
        acc[y:y + tile.shape[0], x:x + tile.shape[1]] += p
        cnt[y:y + tile.shape[0], x:x + tile.shape[1]] += 1
    return acc / np.maximum(cnt, 1)


def segmentation_metrics(per_image_counts: list[dict]) -> dict:
    """Pooled (micro) pixel scores plus image-level mean Dice over images with non-empty GT.

    Policy for empty GT: image excluded from mean Dice, counted in 'n_empty_gt'; its FPs still enter the pooled counts.
    """
    tp = sum(c["tp"] for c in per_image_counts); fp = sum(c["fp"] for c in per_image_counts)
    fn = sum(c["fn"] for c in per_image_counts)
    nonempty = [c for c in per_image_counts if c["tp"] + c["fn"] > 0]
    dices = [counts_to_scores(c["tp"], c["fp"], c["fn"])["dice"] or 0.0 for c in nonempty]
    return {"pooled": counts_to_scores(tp, fp, fn), "mean_dice_nonempty_gt": float(np.mean(dices)) if dices else None,
            "n_images": len(per_image_counts), "n_empty_gt": len(per_image_counts) - len(nonempty)}
