"""Metrics for the deployed cascade and Stage II masks against real expert labels (inference only)."""
from __future__ import annotations

import cv2
import numpy as np
import pandas as pd

from trainlab.evaluate.grading import grading_metrics
from trainlab.evaluate.segmentation import segmentation_metrics
from trainlab.utils.metrics import counts_to_scores


def cascade_grading(df: pd.DataFrame, casc) -> tuple[pd.DataFrame, dict]:
    """Stage B (all images; gate-blocked -> grade 0) and Stage A (true grade>=1 that reached Stage III). Ungradable pred -> 4 for QWK, counted."""
    rows = []
    for r in df.itertuples():
        img = cv2.imread(r.image_path)
        if img is None:
            rows.append({"image_id": r.image_id, "label": int(r.label), "error": "unreadable"})
            continue
        o = casc.run(img)
        rows.append({"image_id": r.image_id, "label": int(r.label), **{k: v for k, v in o.items()}})
    pred = pd.DataFrame(rows)
    ok = pred.dropna(subset=["gate_score"])
    stage_b = grading_metrics(ok["label"], ok["grade_pred"])
    a = ok[(ok["label"] >= 1) & ok["stage3_run"]]
    stage_a = grading_metrics(a["label"], a["grade_pred"]) if len(a) else "NOT_RUN: no true-positive image reached Stage III"
    return pred, {"stage_B_all_images": stage_b, "stage_A_reached_stage3": stage_a, "n_stage3_run": int(ok["stage3_run"].sum()),
                  "n_ungradable_pred": int(ok["ungradable"].sum()), "n_unreadable": int(len(pred) - len(ok)),
                  "selection_note": "Stage III grades exist only for gate-positive images; Stage A conditions on that selection"}


def _counts(pred_mask: np.ndarray, gt_mask: np.ndarray) -> dict:
    p, g = pred_mask > 127, gt_mask > 0
    return {"tp": int((p & g).sum()), "fp": int((p & ~g).sum()), "fn": int((~p & g).sum())}


def segmentation_baseline(casc, lesion_manifests, vessel_manifests, limit=None) -> dict:
    out = {}
    if lesion_manifests:
        df = pd.concat([pd.read_csv(m) for m in lesion_manifests], ignore_index=True)
        df = df[df["mask_status"] == "ok"]
        for lz in ("MA", "HE", "EX", "SE"):
            sub = df[df["lesion"] == lz]
            model_key = "CWS" if lz == "SE" else lz
            if lz == "SE":
                out["SE"] = "NOT_RUN: deployed CWS (cotton-wool) vs dataset SE (soft exudate) mapping not verified"
                continue
            sub = sub.head(limit) if limit else sub
            per = []
            for r in sub.itertuples():
                img, gt = cv2.imread(r.image_path), cv2.imread(r.mask_path, cv2.IMREAD_GRAYSCALE)
                if img is None or gt is None or gt.shape != img.shape[:2]:
                    continue
                per.append(_counts(casc.lesion_mask(model_key, img), gt))
            out[lz] = segmentation_metrics(per) if per else "NOT_RUN: no images with real masks"
    if vessel_manifests:
        df = pd.concat([pd.read_csv(m) for m in vessel_manifests], ignore_index=True)
        df = df[df["mask_status"] == "ok"]
        df = df.head(limit) if limit else df
        per = []
        for r in df.itertuples():
            img, gt = cv2.imread(r.image_path), cv2.imread(r.mask_path, cv2.IMREAD_GRAYSCALE)
            if img is not None and gt is not None and gt.shape == img.shape[:2]:
                per.append(_counts(casc.vessel_mask(img), gt))
        out["vessel"] = segmentation_metrics(per) if per else "NOT_RUN: no vessel references"
    out["caveat"] = "deployed models' training data may overlap these images; only published test partitions are plausibly unseen"
    return out
