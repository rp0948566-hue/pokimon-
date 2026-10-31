#!/usr/bin/env python
"""P3: DRIVE -> vessel manifests. Official partition kept: 20 training (split 16/4 train/val), 20 test.

Test rows get mask_path only if a test reference file exists locally; the challenge withholds them, so
no local test Dice is ever fabricated (rows without reference are marked annotation_type=withheld).
Expected layout: <raw>/{training,test}/{images,1st_manual,mask}/ (Grand Challenge / original DRIVE).
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

import pandas as pd

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

from trainlab.data import prepare as P  # noqa: E402
from trainlab.data import splits as S  # noqa: E402


def collect(raw: Path, part: str) -> pd.DataFrame:
    imgs = sorted(p for p in raw.rglob(f"{part}/images/*") if p.is_file())
    rows = []
    for p in imgs:
        num = re.match(r"(\d+)_", p.stem)
        if not num:
            continue
        n = num.group(1)
        find = lambda sub, pat: next(iter(sorted((p.parent.parent / sub).glob(pat))), None)  # noqa: E731
        man, fov = find("1st_manual", f"{n}_manual1.*"), find("mask", f"{n}_{part}_mask.*")
        rows.append({"image_id": f"drive_{part}_{n}", "image_path": str(p), "mask_path": str(man) if man else None,
                     "fov_path": str(fov) if fov else None, "official_split": part})
    return pd.DataFrame(rows)


def build(raw: Path, seed: int, manifests: Path, audit: Path) -> dict:
    df = pd.concat([collect(raw, "training"), collect(raw, "test")], ignore_index=True)
    if df.empty:
        raise FileNotFoundError(f"no DRIVE images under {raw}")
    meta, errors = P.scan_images(dict(zip(df["image_id"], map(Path, df["image_path"]))))
    df = df.drop(columns=["image_path"]).merge(meta, on="image_id")
    df, near = P.add_duplicate_groups(df)
    df = P.check_masks(df)
    df["split"] = "test"
    tr = df["official_split"] == "training"
    df.loc[tr, "split"] = S.stratified_group_split(df[tr].assign(label=0), "label", "group_id", {"train": 0.8, "val": 0.2}, seed)
    wh = (df["split"] == "test") & (df["mask_status"] != "ok")
    df["annotation_type"] = "vessel_mask"
    df.loc[wh, "annotation_type"] = "withheld"
    df["label"], df["prepared_path"], df["source"] = 0, None, "drive"
    S.assert_no_leakage({k: v for k, v in df.groupby("split")})
    parts = {k: v for k, v in df.groupby("split")}
    out = {}
    for k, v in parts.items():
        out.update(S.write_split_manifests(v.assign(split=k), manifests / ("external" if k == "test" else "."), "drive_vessels",
                                           test_dir="." if k == "test" else "test"))
    rep = P.write_audit("drive_vessels", parts, errors, near, audit, {"test_references_present": int((df["split"] == "test").sum() - wh.sum()),
                        "manifests": out, "note": "official test references withheld unless present locally; no local test Dice is claimed otherwise"})
    ok = df[df["mask_status"] == "ok"]
    P.overlay_sheet(list(zip(ok["image_path"], ok["mask_path"])), audit / "drive_vessel_overlays.png", n=12, seed=seed)
    return rep


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--raw", default=str(LAB / "data/raw/drive"))
    ap.add_argument("--seed", type=int, default=42)
    a = ap.parse_args()
    rep = build(Path(a.raw), a.seed, LAB / "data/manifests", LAB / "reports/data_audit")
    print(rep["split_counts"], "errors=", rep["n_errors"], "leakage_ok=", rep["leakage"]["ok"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
