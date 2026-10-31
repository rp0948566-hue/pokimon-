"""Run DEPLOYED models on labelled manifests (inference only; no weights touched)."""
from __future__ import annotations

import cv2
import pandas as pd

NOT_RUN = {
    "stage2_lesions": "NOT_RUN: Stage II baseline needs IDRiD lesion masks (not present) and the upstream lesion crop/threshold pipeline wired in the lab",
    "stage2_vessel": "NOT_RUN: Stage II vessel baseline needs DRIVE references (not present)",
    "stage3_lgdrg": "NOT_RUN: Stage III needs Stage II mask outputs for every image (5 lesion/vessel models) and 5-fold ensemble decoding; cascade not implemented in the lab",
}


def stage1_predictions(df: pd.DataFrame, baseline, path_col: str = "image_path") -> pd.DataFrame:
    """One row per image: real model output only. Unreadable images are recorded with error, not scored."""
    rows = []
    for r in df.itertuples():
        img = cv2.imread(getattr(r, path_col))
        if img is None:
            rows.append({"image_id": r.image_id, "label": r.label, "error": "unreadable"})
            continue
        p = baseline.predict(img)
        rows.append({"image_id": r.image_id, "label": int(r.label), "score": p["score"], "logit": p["logit"], "pred_positive": int(p["positive"])})
    return pd.DataFrame(rows)
