"""Metrics shared by training and evaluation. No external prediction is ever invented here."""
from __future__ import annotations

import numpy as np


def confusion(y_true, y_pred, n: int) -> np.ndarray:
    cm = np.zeros((n, n), dtype=np.int64)
    for t, p in zip(np.asarray(y_true, int), np.asarray(y_pred, int)):
        cm[t, p] += 1
    return cm


def qwk(y_true, y_pred, n: int = 5) -> float:
    """Quadratic weighted kappa; 0.0 when undefined (single class on both sides)."""
    cm = confusion(y_true, y_pred, n).astype(float)
    if cm.sum() == 0:
        return 0.0
    w = (np.arange(n)[:, None] - np.arange(n)[None, :]) ** 2 / (n - 1) ** 2
    exp = np.outer(cm.sum(1), cm.sum(0)) / cm.sum()
    den = (w * exp).sum()
    return float(1.0 - (w * cm).sum() / den) if den > 0 else 0.0


def macro_f1(y_true, y_pred, n: int = 5) -> float:
    cm = confusion(y_true, y_pred, n)
    f1s = []
    for c in range(n):
        if cm[c].sum() == 0:  # class absent from truth: excluded, support reported separately
            continue
        tp = cm[c, c]
        prec = tp / cm[:, c].sum() if cm[:, c].sum() else 0.0
        rec = tp / cm[c].sum()
        f1s.append(2 * prec * rec / (prec + rec) if prec + rec else 0.0)
    return float(np.mean(f1s)) if f1s else 0.0


def binary_report(y_true, y_pred) -> dict:
    y_true, y_pred = np.asarray(y_true, bool), np.asarray(y_pred, bool)
    tp = int((y_true & y_pred).sum()); fn = int((y_true & ~y_pred).sum())
    tn = int((~y_true & ~y_pred).sum()); fp = int((~y_true & y_pred).sum())
    div = lambda a, b: float(a / b) if b else None  # noqa: E731  None = undefined, not 0
    return {"tp": tp, "fp": fp, "tn": tn, "fn": fn, "sensitivity": div(tp, tp + fn),
            "specificity": div(tn, tn + fp), "precision": div(tp, tp + fp),
            "f1": div(2 * tp, 2 * tp + fp + fn), "n": int(len(y_true))}


def auroc(y_true, score) -> float | None:
    from sklearn.metrics import roc_auc_score

    y_true = np.asarray(y_true)
    return float(roc_auc_score(y_true, score)) if len(np.unique(y_true)) == 2 else None


def bootstrap_ci(fn, y_true, y_pred, n_boot: int = 1000, seed: int = 0, alpha: float = 0.05):
    """Percentile CI of fn(y_true, y_pred) over image-level resamples; None if <20 samples."""
    y_true, y_pred = np.asarray(y_true), np.asarray(y_pred)
    if len(y_true) < 20:
        return None
    rng = np.random.default_rng(seed)
    vals = []
    for _ in range(n_boot):
        idx = rng.integers(0, len(y_true), len(y_true))
        v = fn(y_true[idx], y_pred[idx])
        if v is not None and not np.isnan(v):
            vals.append(v)
    if not vals:
        return None
    return [float(np.percentile(vals, 100 * alpha / 2)), float(np.percentile(vals, 100 * (1 - alpha / 2)))]


def dice_iou_counts(pred, gt, eps: float = 1e-7) -> dict:
    """Pixel counts -> Dice/IoU/sensitivity/precision. Empty GT and empty pred: Dice reported as None."""
    pred, gt = np.asarray(pred, bool), np.asarray(gt, bool)
    tp = int((pred & gt).sum()); fp = int((pred & ~gt).sum()); fn = int((~pred & gt).sum())
    return counts_to_scores(tp, fp, fn)


def counts_to_scores(tp: int, fp: int, fn: int) -> dict:
    div = lambda a, b: float(a / b) if b else None  # noqa: E731
    return {"tp": tp, "fp": fp, "fn": fn, "dice": div(2 * tp, 2 * tp + fp + fn),
            "iou": div(tp, tp + fp + fn), "sensitivity": div(tp, tp + fn), "precision": div(tp, tp + fp)}
