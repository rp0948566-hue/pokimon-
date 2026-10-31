#!/usr/bin/env python
"""P3: APTOS 2019 -> validated manifests with deterministic, duplicate-grouped train/val/locked_test split.

Reads data/raw/aptos (Kaggle layout: train.csv + train_images/). Only labelled training images are used;
the Kaggle public test images have no labels and are ignored. Locked-test manifest is written read-only.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

import pandas as pd

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

from trainlab.data import prepare as P  # noqa: E402
from trainlab.data import splits as S  # noqa: E402


def find_layout(raw: Path) -> tuple[Path, Path]:
    csvs = sorted(raw.rglob("train.csv"))
    if not csvs:
        raise FileNotFoundError(f"no train.csv under {raw}: run download_datasets.py first")
    imgs = [d for d in raw.rglob("train_images") if d.is_dir()]
    if not imgs:
        raise FileNotFoundError(f"no train_images/ under {raw}")
    return csvs[0], imgs[0]


def build(raw: Path, size: int, seed: int, fractions: dict, limit: int | None, prepared: Path, manifests: Path, audit: Path) -> dict:
    csv, img_dir = find_layout(raw)
    lab = pd.read_csv(csv)
    if not {"id_code", "diagnosis"} <= set(lab.columns):
        raise ValueError(f"{csv}: expected columns id_code, diagnosis, got {list(lab.columns)}")
    bad = lab[~lab["diagnosis"].isin(range(5))]
    if len(bad):
        raise ValueError(f"{len(bad)} rows with diagnosis outside 0..4 (not silently converted): {bad.head(3).to_dict('records')}")
    if lab["id_code"].duplicated().any():
        raise ValueError("duplicate id_code in train.csv")
    if limit:
        lab = lab.sample(n=min(limit, len(lab)), random_state=seed)
    paths, missing = {}, []
    for iid in lab["id_code"]:
        cand = next((img_dir / f"{iid}{e}" for e in (".png", ".jpg", ".jpeg") if (img_dir / f"{iid}{e}").is_file()), None)
        (paths.__setitem__(iid, cand) if cand else missing.append({"image_id": iid, "error": "file missing"}))
    meta, errors = P.scan_images(paths)
    errors += missing
    df = lab.rename(columns={"id_code": "image_id", "diagnosis": "label"}).merge(meta, on="image_id", how="inner")
    df, near = P.add_duplicate_groups(df)
    df["split"] = S.stratified_group_split(df, "label", "group_id", fractions, seed)
    df["prepared_path"] = P.cache_squares(df, prepared / f"aptos_{size}", size)
    df["source"], df["annotation_type"] = "aptos2019", "dr_grade_0_4"
    S.assert_no_leakage({k: v for k, v in df.groupby("split")})
    parts = {k: v[S.SPLIT_COLS] for k, v in df.groupby("split")}
    paths_out = S.write_split_manifests(pd.concat(parts.values()), manifests, "aptos")
    rep = P.write_audit("aptos", parts, errors, near, audit, {"patient_ids_available": False,
                        "leakage_note": "APTOS has no patient IDs: same-eye/same-patient leakage cannot be excluded; only exact and near-duplicate images are grouped.",
                        "manifests": paths_out, "seed": seed, "fractions": fractions, "cache_size": size})
    P.contact_sheet(df["prepared_path"].tolist(), audit / "aptos_samples.png")
    return rep


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--raw", default=str(LAB / "data/raw/aptos"))
    ap.add_argument("--size", type=int, default=384, help="prepared square cache size")
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--fractions", type=float, nargs=3, default=[0.70, 0.15, 0.15], metavar=("TRAIN", "VAL", "TEST"))
    ap.add_argument("--limit", type=int, help="use only N random labelled images (dev only)")
    a = ap.parse_args()
    fr = dict(zip(("train", "val", "locked_test"), a.fractions))
    rep = build(Path(a.raw), a.size, a.seed, fr, a.limit, LAB / "data/prepared/grading", LAB / "data/manifests", LAB / "reports/data_audit")
    print({k: rep[k] for k in ("n_images", "n_errors", "split_counts", "class_distribution")}, "leakage_ok=", rep["leakage"]["ok"])
    return 0 if rep["leakage"]["ok"] else 2


if __name__ == "__main__":
    sys.exit(main())
