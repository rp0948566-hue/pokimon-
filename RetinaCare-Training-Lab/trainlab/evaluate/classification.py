"""Binary screening evaluation from a predictions table (image_id, label, score, pred)."""
from __future__ import annotations

import numpy as np

from trainlab.utils import metrics as M


def screening_metrics(y_true, y_pred, score=None, n_boot: int = 1000) -> dict:
    """Sensitivity/specificity/precision/F1 (+AUROC if scores) with 95% bootstrap CIs (n>=20)."""
    y_true, y_pred = np.asarray(y_true, int), np.asarray(y_pred, int)
    rep = M.binary_report(y_true, y_pred)
    sens = lambda t, p: M.binary_report(t, p)["sensitivity"]  # noqa: E731
    spec = lambda t, p: M.binary_report(t, p)["specificity"]  # noqa: E731
    rep["sensitivity_ci95"] = M.bootstrap_ci(sens, y_true, y_pred, n_boot)
    rep["specificity_ci95"] = M.bootstrap_ci(spec, y_true, y_pred, n_boot)
    if score is not None:
        rep["auroc"] = M.auroc(y_true, score)
    return rep
