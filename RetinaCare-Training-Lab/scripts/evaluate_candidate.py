#!/usr/bin/env python
"""P6: evaluate a trained grading candidate against the deployed Stage I baseline on the SAME images.

  default (--split val): checkpoint/threshold selection data. Safe to repeat.
  --final: runs the LOCKED test split exactly once (lock record experiments/locked_test_used.json).
Promotion is never performed here; the verdict only says whether the candidate would be worth proposing.
"""
from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np
import pandas as pd
import torch
from torch.utils.data import DataLoader

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

from trainlab.data.datasets import GradingDataset  # noqa: E402
from trainlab.evaluate.baseline import stage1_predictions  # noqa: E402
from trainlab.evaluate.classification import screening_metrics  # noqa: E402
from trainlab.evaluate.grading import grading_metrics  # noqa: E402
from trainlab.evaluate.plots import confusion_matrix_png  # noqa: E402
from trainlab.models.adapters import Stage1Baseline, screening_label  # noqa: E402
from trainlab.models.registry import build_grading_model  # noqa: E402
from trainlab.train.grading import predict_logits  # noqa: E402
from trainlab.utils.device import gpu_snapshot, pick_device  # noqa: E402
from trainlab.utils.hashing import sha256_file  # noqa: E402

LOCK = LAB / "experiments/locked_test_used.json"


def load_candidate(run_dir: Path, device):
    info = json.loads((run_dir / "run_info.json").read_text())
    if info.get("kind") != "train":
        sys.exit(f"{run_dir.name} is kind={info.get('kind')}: smoke/failed runs cannot be evaluated as candidates")
    ck = info["best_ckpt"]
    if sha256_file(ck["path"]) != ck["sha256"]:
        sys.exit("best.ckpt hash differs from run_info.json: checkpoint modified after training")
    st = torch.load(ck["path"], map_location="cpu", weights_only=False)
    meta = info["meta"]
    model = build_grading_model(meta["architecture"], 5, pretrained=False)
    model.load_state_dict(st["model"])
    return model.to(device).eval(), info


def verdict(base: dict, cand: dict) -> dict:
    """Reject unless the candidate is not worse on sensitivity AND specificity (screening) and QWK is reported."""
    reasons = []
    for k in ("sensitivity", "specificity"):
        if base[k] is None or cand[k] is None:
            reasons.append(f"{k} undefined")
        elif cand[k] < base[k]:
            reasons.append(f"candidate {k} {cand[k]:.3f} < baseline {base[k]:.3f}")
    return {"recommend_promotion_review": not reasons, "reasons": reasons or ["no regression on screening sensitivity/specificity (point estimates only; see CIs)"],
            "promotion_performed": False}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--run", required=True, help="grading run directory name under experiments/grading")
    ap.add_argument("--manifest", help="override manifest (default: val, or locked_test with --final)")
    ap.add_argument("--final", action="store_true", help="evaluate locked test once")
    ap.add_argument("--cpu", action="store_true")
    ap.add_argument("--limit", type=int, help="first N images (dev only, recorded)")
    a = ap.parse_args()
    device = pick_device("cpu" if a.cpu else "cuda")
    run_dir = LAB / "experiments/grading" / a.run
    manifest = Path(a.manifest) if a.manifest else LAB / ("data/manifests/test/aptos_locked_test.csv" if a.final else "data/manifests/val/aptos_val.csv")
    if a.final:
        if LOCK.exists():
            sys.exit(f"locked test already used: {LOCK.read_text()}")
        if "locked_test" not in manifest.name:
            sys.exit("--final requires the locked-test manifest")
    df = pd.read_csv(manifest)
    if a.limit:
        df = df.head(a.limit)
    model, info = load_candidate(run_dir, device)
    size = info["meta"]["image_size"]
    t0 = time.time()
    logits, y = predict_logits(model, DataLoader(GradingDataset(df, size, False), batch_size=8, num_workers=4), device)
    pred = logits.argmax(1).numpy()
    cand_scr = torch.softmax(logits, 1).numpy()
    cand_pos_score = 1 - cand_scr[:, 0]
    ref = screening_label(y.numpy())
    cand_screen = screening_metrics(ref, (pred >= 1).astype(int), cand_pos_score)
    cand_grade = grading_metrics(y.numpy(), pred)
    base_pred = stage1_predictions(df, Stage1Baseline(providers=["CPUExecutionProvider"] if a.cpu else None))
    ok = base_pred.dropna(subset=["score"])
    base_screen = screening_metrics(screening_label(ok["label"]), ok["pred_positive"], ok["score"])
    res = {"split_manifest": str(manifest), "final": a.final, "n_images": int(len(df)), "limit": a.limit, "candidate_run": a.run,
           "candidate_best_ckpt": info["best_ckpt"], "candidate_screening": cand_screen, "candidate_grading": cand_grade,
           "baseline_stage1_screening": base_screen, "baseline_grading": "NOT_RUN: deployed Stage III cascade not implemented in lab",
           "verdict": verdict(base_screen, cand_screen), "seconds": round(time.time() - t0, 1), "gpu": gpu_snapshot(),
           "limitations": ["research only, no clinical claim", "single dataset (APTOS) split; no patient IDs, same-patient leakage not excluded",
                           "domain shift not measured unless an external manifest was also evaluated",
                           "baseline Stage I is binary; 5-class baseline NOT_RUN"]}
    outd = LAB / "reports/comparisons"
    outd.mkdir(parents=True, exist_ok=True)
    tag = f"{a.run}_{'FINAL_TEST' if a.final else 'val'}"
    (outd / f"{tag}.json").write_text(json.dumps(res, indent=1))
    confusion_matrix_png(np.array(cand_grade["confusion_matrix"]), ["0", "1", "2", "3", "4"], LAB / "reports/figures" / f"{tag}_confusion.png", f"candidate {a.run}")
    if a.final:
        LOCK.write_text(json.dumps({"run": a.run, "when": time.strftime("%Y-%m-%dT%H:%M:%S"), "manifest_sha256": sha256_file(manifest)}))
        (LAB / "reports/SCIENTIFIC_VALIDATION_REPORT.md").write_text(render_report(res))
    print(f"{tag}: n={len(df)} cand_sens={cand_screen['sensitivity']} cand_spec={cand_screen['specificity']} qwk={cand_grade['qwk']:.3f} | "
          f"base_sens={base_screen['sensitivity']} base_spec={base_screen['specificity']} | verdict={res['verdict']['recommend_promotion_review']}")
    return 0


def render_report(r: dict) -> str:
    c, b, g = r["candidate_screening"], r["baseline_stage1_screening"], r["candidate_grading"]
    f = lambda v: "n/a" if v is None else f"{v:.3f}"  # noqa: E731
    return (f"# Scientific validation report (research only)\n\nCandidate run `{r['candidate_run']}`; locked test n={r['n_images']}.\n\n"
            f"| metric | baseline Stage I | candidate |\n|---|---|---|\n| sensitivity | {f(b['sensitivity'])} {b['sensitivity_ci95']} | {f(c['sensitivity'])} {c['sensitivity_ci95']} |\n"
            f"| specificity | {f(b['specificity'])} {b['specificity_ci95']} | {f(c['specificity'])} {c['specificity_ci95']} |\n"
            f"| AUROC | {f(b.get('auroc'))} | {f(c.get('auroc'))} |\n\nCandidate 5-class: QWK {f(g['qwk'])} {g['qwk_ci95']}, macro-F1 {f(g['macro_f1'])}, supports {g['support']}.\n\n"
            f"Verdict: {r['verdict']}\n\n## Limitations\n" + "".join(f"- {x}\n" for x in r["limitations"]))


if __name__ == "__main__":
    sys.exit(main())
