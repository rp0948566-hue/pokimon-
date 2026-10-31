#!/usr/bin/env python
"""P4: benchmark the DEPLOYED models on labelled images BEFORE any candidate exists.

Stage I is evaluated (binary screening, reference = grade>=1, deployed threshold/operator). Stage II/III
are recorded NOT_RUN with the reason. Output experiments/baseline-<timestamp>/ is made read-only.
Locked-test images are refused here; they are only evaluated once, by evaluate_candidate.py --final.
"""
from __future__ import annotations

import argparse
import json
import os
import stat
import sys
import time
from pathlib import Path

import pandas as pd

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

from trainlab.evaluate.baseline import NOT_RUN, stage1_predictions  # noqa: E402
from trainlab.evaluate.classification import screening_metrics  # noqa: E402
from trainlab.evaluate.plots import confusion_matrix_png  # noqa: E402
from trainlab.models.adapters import Stage1Baseline, screening_label  # noqa: E402
from trainlab.utils import metrics as M  # noqa: E402
from trainlab.utils.device import gpu_snapshot, nvidia_smi  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--manifest", nargs="+", default=[str(LAB / "data/manifests/val/aptos_val.csv")], help="labelled manifest CSV(s) with image_path,label")
    ap.add_argument("--limit", type=int, help="first N images per manifest (dev only; recorded in methodology)")
    ap.add_argument("--cpu", action="store_true", help="ONNX Runtime CPU provider")
    ap.add_argument("--cascade", action="store_true", help="also run Stage II masks + Stage III 5-fold on gate-positive images (slow on CPU)")
    ap.add_argument("--lesion-manifest", nargs="*", default=[], help="lesion manifests (image_path,mask_path,lesion,mask_status) for Stage II segmentation baseline")
    ap.add_argument("--vessel-manifest", nargs="*", default=[], help="vessel manifests (image_path,mask_path,mask_status) for Stage II vessel baseline")
    a = ap.parse_args()
    for m in a.manifest:
        if "/test/" in str(Path(m).resolve()) and "locked_test" in Path(m).name:
            sys.exit(f"refusing locked test manifest {m}: use evaluate_candidate.py --final")
    df = pd.concat([pd.read_csv(m) for m in a.manifest], ignore_index=True)
    if a.limit:
        df = df.groupby(df["source"]).head(a.limit)
    out = LAB / "experiments/baseline" / f"baseline-{time.strftime('%Y%m%d-%H%M%S')}"
    (out / "figures").mkdir(parents=True)
    b = Stage1Baseline(providers=["CPUExecutionProvider"] if a.cpu else None, lazy=a.cascade)
    t0 = time.time()
    extra = {}
    if a.cascade or a.lesion_manifest or a.vessel_manifest:
        from trainlab.evaluate.cascade_eval import cascade_grading, segmentation_baseline
        from trainlab.evaluate.deployed_cascade import DeployedCascade
        casc = DeployedCascade(providers=["CPUExecutionProvider"] if a.cpu else None)
    if a.cascade:
        pred, extra["stage3_cascade"] = cascade_grading(df, casc)
    else:
        pred = stage1_predictions(df, b)
    if a.lesion_manifest or a.vessel_manifest:
        extra["stage2_segmentation"] = segmentation_baseline(casc, a.lesion_manifest, a.vessel_manifest, a.limit)
    ok = pred.dropna(subset=["gate_score" if "gate_score" in pred else "score"]).rename(columns={"gate_score": "score", "gate_positive": "pred_positive"})
    ref = screening_label(ok["label"])
    met = screening_metrics(ref, ok["pred_positive"], ok["score"])
    cm = M.confusion(ref, ok["pred_positive"].astype(int), 2)
    confusion_matrix_png(cm, ["neg", "pos"], out / "figures/confusion_matrix.png", "Stage I deployed gate (ref: grade>=1)")
    per_grade = ok.groupby("label")["pred_positive"].agg(["mean", "count"]).rename(columns={"mean": "positive_rate", "count": "n"}).reset_index().to_dict("records")
    metrics = {"stage1_screening": {**met, "confusion_matrix": cm.tolist(), "per_true_grade_positive_rate": per_grade,
                                    "n_unreadable": int(len(pred) - len(ok)), "threshold": b.threshold, "operator": ">=", "providers": b.providers,
                                    "model_sha256_verified": b.man["sha256"], "seconds": round(time.time() - t0, 1)},
               **{k: v for k, v in NOT_RUN.items() if (k == "stage3_lgdrg" and not a.cascade) or (k == "stage2_lesions" and not a.lesion_manifest) or (k == "stage2_vessel" and not a.vessel_manifest)}, **extra}
    pred.drop(columns=["logit", "full5", "mask_px"], errors="ignore").to_csv(out / "predictions.csv", index=False)  # image_id is the dataset id, not a patient identifier
    (out / "metrics.json").write_text(json.dumps(metrics, indent=1))
    (out / "hardware.json").write_text(json.dumps({"nvidia_smi": nvidia_smi(), "onnx_providers": b.providers}, indent=1))
    (out / "methodology.md").write_text(
        f"# Baseline methodology\n\n- Model: deployed Stage I gate `{b.man['model_id']}` sha256 `{b.man['sha256']}` (verified before load)\n"
        f"- Preprocessing: upstream `preprocess_gate` @768 + ImageNet normalisation (same as production export script)\n"
        f"- Reference: screening positive = DR grade >= 1; prediction positive = sigmoid(logit) >= {b.threshold} (deployed, not tuned here)\n"
        f"- Manifests: {', '.join(a.manifest)} (n={len(df)}, scored={len(ok)}, limit={a.limit})\n"
        f"- CIs: 1000-resample percentile bootstrap, only if n>=20. Scores are model outputs, not calibrated probabilities.\n"
        f"- Stage II/III: see metrics.json (NOT_RUN entries state why). Research only; no clinical claim.\n")
    for p in out.rglob("*"):  # immutable after creation
        if p.is_file():
            os.chmod(p, stat.S_IRUSR | stat.S_IRGRP)
    print(f"baseline -> {out}\nsens={met['sensitivity']} spec={met['specificity']} auroc={met.get('auroc')} n={met['n']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
