"""5-class grading evaluation: confusion matrix, QWK, macro-F1, supports (+ bootstrap CIs)."""
from __future__ import annotations

import numpy as np

from trainlab.utils import metrics as M


def grading_metrics(y_true, y_pred, n_classes: int = 5, n_boot: int = 1000) -> dict:
    y_true, y_pred = np.asarray(y_true, int), np.asarray(y_pred, int)
    cm = M.confusion(y_true, y_pred, n_classes)
    return {"n": int(len(y_true)), "support": cm.sum(1).tolist(), "confusion_matrix": cm.tolist(),
            "accuracy": float((y_true == y_pred).mean()) if len(y_true) else None,
            "qwk": M.qwk(y_true, y_pred, n_classes), "qwk_ci95": M.bootstrap_ci(lambda t, p: M.qwk(t, p, n_classes), y_true, y_pred, n_boot),
            "macro_f1": M.macro_f1(y_true, y_pred, n_classes),
            "macro_f1_ci95": M.bootstrap_ci(lambda t, p: M.macro_f1(t, p, n_classes), y_true, y_pred, n_boot)}
