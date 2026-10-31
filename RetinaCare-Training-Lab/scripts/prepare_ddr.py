#!/usr/bin/env python
"""P3: DDR (OIA-DDR, CC BY-NC-SA 4.0) -> grading and lesion manifests, official partitions preserved.

Assumed layout (author repo; verify with --dry-run on the real archive, it prints what it found):
  grading:      <raw>/**/DR_grading/{train,valid,test}/*.jpg + {train,valid,test}.txt  ("name.jpg grade", grade 0-4, 5 = ungradable)
  segmentation: <raw>/**/lesion_segmentation/{train,valid,test}/image/*  and  .../label/{MA,HE,EX,SE}/<same stem>.tif
Official train -> train, valid -> val, test -> external test. Grade 5 (ungradable) is EXCLUDED and counted, never relabelled.
Images in train/valid that are exact/near duplicates of a test image are quarantined (listed in the audit), not silently kept.
A lesion mask that is absent is mask_status=missing, not an empty mask. Only lesion keys documented by DDR are used (no CWS/vessel).
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

LESIONS = ("MA", "HE", "EX", "SE")
PART = {"train": "train", "valid": "val", "val": "val", "test": "test"}
IMG_EXT = {".jpg", ".jpeg", ".png", ".tif", ".tiff"}


def _part(p: Path, root_name: str) -> str | None:
    parts = [x.lower() for x in p.parts]
    if root_name not in "/".join(parts):
        return None
    for x in parts:
        if x in PART:
            return PART[x]
    return None


def find_grading(raw: Path) -> tuple[pd.DataFrame, dict]:
    rows, excluded = [], {"ungradable_5": 0, "bad_label": 0, "image_missing": 0}
    idx = {p.name: p for p in raw.rglob("*") if p.suffix.lower() in IMG_EXT and "grading" in "/".join(p.parts).lower()}
    for txt in raw.rglob("*.txt"):
        sp = PART.get(txt.stem.lower())
        if sp is None or "grading" not in "/".join(txt.parts).lower():
            continue
        for line in txt.read_text().splitlines():
            bits = line.split()
            if len(bits) != 2:
                continue
            name, g = bits[0], bits[1]
            if not g.isdigit() or int(g) > 5:
                excluded["bad_label"] += 1
            elif int(g) == 5:
                excluded["ungradable_5"] += 1
            elif name not in idx:
                excluded["image_missing"] += 1
            else:
                rows.append({"image_id": f"ddr_{Path(name).stem}", "image_path": str(idx[name]), "label": int(g), "official_split": sp})
    return pd.DataFrame(rows), excluded


def quarantine_cross_partition(df: pd.DataFrame) -> tuple[pd.DataFrame, list]:
    """Drop train/val rows whose duplicate group also contains a test image."""
    test_groups = set(df.loc[df["official_split"] == "test", "group_id"])
    bad = df["official_split"].ne("test") & df["group_id"].isin(test_groups)
    return df[~bad], df.loc[bad, "image_id"].tolist()


def build_grading(raw: Path, size: int, seed: int, limit: int | None, dry: bool, prepared: Path, manifests: Path, audit: Path) -> dict:
    lab, excluded = find_grading(raw)
    if lab.empty:
        raise FileNotFoundError(f"no DDR grading list/images found under {raw} (run --dry-run, check layout)")
    if limit:
        lab = lab.groupby("official_split", group_keys=False).head(limit)
    meta, errors = P.scan_images(dict(zip(lab["image_id"], map(Path, lab["image_path"]))))
    df = lab.drop(columns=["image_path"]).merge(meta, on="image_id")
    df, near = P.add_duplicate_groups(df)
    df, quarantined = quarantine_cross_partition(df)
    df["split"] = df["official_split"]
    df["source"], df["annotation_type"] = "ddr_grading", "dr_grade_0_4"
    if dry:
        return {"dry_run": True, "n_images": len(df), "excluded": excluded, "errors": len(errors), "split_counts": df["split"].value_counts().to_dict(),
                "class_distribution": S.class_distribution(df), "quarantined": len(quarantined)}
    df["prepared_path"] = P.cache_squares(df, prepared / f"ddr_grading_{size}", size)
    S.assert_no_leakage({k: v for k, v in df.groupby("split")})
    parts = {k: v[S.SPLIT_COLS + ["official_split"]] for k, v in df.groupby("split")}
    out = {}
    for k, v in parts.items():
        out.update(S.write_split_manifests(v, manifests / ("external" if k == "test" else "."), "ddr_grading", test_dir="."))
    rep = P.write_audit("ddr_grading", parts, errors, near, audit, {"excluded": excluded, "quarantined_cross_partition": quarantined[:100],
                        "n_quarantined": len(quarantined), "manifests": out, "patient_ids_available": False,
                        "leakage_note": "DDR ships no patient IDs; only exact/near-duplicate images are grouped."})
    P.contact_sheet(df["prepared_path"].tolist(), audit / "ddr_grading_samples.png", seed=seed)
    return rep


def build_lesions(raw: Path, seed: int, limit: int | None, dry: bool, manifests: Path, audit: Path) -> dict:
    imgs, masks = {}, {}
    for p in raw.rglob("*"):
        if not p.is_file() or p.suffix.lower() not in IMG_EXT:
            continue
        low = "/".join(x.lower() for x in p.parts)
        if "segmentation" not in low:
            continue
        sp = _part(p, "segmentation")
        if sp is None:
            continue
        lz = next((x for x in p.parts if x.upper() in LESIONS), None)
        if lz and "label" in low:
            masks[(p.stem, lz.upper(), sp)] = p
        elif "image" in low:
            imgs[(p.stem, sp)] = p
    if not imgs:
        raise FileNotFoundError(f"no DDR lesion-segmentation images under {raw} (run --dry-run, check layout)")
    base = pd.DataFrame([{"image_id": f"ddr_seg_{stem}", "image_path": str(p), "official_split": sp, "stem": stem} for (stem, sp), p in imgs.items()])
    if limit:
        base = base.groupby("official_split", group_keys=False).head(limit)
    meta, errors = P.scan_images(dict(zip(base["image_id"], map(Path, base["image_path"]))))
    base = base.drop(columns=["image_path"]).merge(meta, on="image_id")
    rows = []
    for lz in LESIONS:
        d = base.copy()
        d["lesion"] = lz
        d["mask_path"] = [str(masks[(r.stem, lz, r.official_split)]) if (r.stem, lz, r.official_split) in masks else None for r in d.itertuples()]
        rows.append(P.check_masks(d))
    df = pd.concat(rows, ignore_index=True)
    df["split"], df["group_id"] = df["official_split"], df["image_id"]
    df["label"], df["prepared_path"], df["source"] = df["lesion"], None, "ddr_segmentation"
    df["annotation_type"] = "pixel_mask_" + df["lesion"]
    summary = df.groupby(["lesion", "mask_status"]).size().unstack(fill_value=0).to_dict("index")
    if dry:
        return {"dry_run": True, "n_images": len(base), "split_counts": base["official_split"].value_counts().to_dict(), "mask_status": summary, "errors": len(errors)}
    S.assert_no_leakage({k: v for k, v in df.groupby("split")})
    parts = {k: v for k, v in df.groupby("split")}
    out = {}
    for k, v in parts.items():
        out.update(S.write_split_manifests(v, manifests / ("external" if k == "test" else "."), "ddr_lesions", test_dir="."))
    rep = P.write_audit("ddr_lesions", {k: v.assign(label=0) for k, v in parts.items()}, errors, [], audit, {"mask_status": summary, "manifests": out,
                        "note": "SE = soft exudate; mapping to deployed CWS is NOT assumed. No vessel references in DDR."})
    ok = df[df["mask_status"] == "ok"]
    P.overlay_sheet(list(zip(ok["image_path"], ok["mask_path"])), audit / "ddr_lesion_overlays.png", n=12, seed=seed)
    return rep


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--raw", default=str(LAB / "data/raw/ddr"))
    ap.add_argument("--size", type=int, default=384)
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--task", choices=["grading", "lesions", "both"], default="both")
    ap.add_argument("--limit", type=int, help="N images per official partition (dev only)")
    ap.add_argument("--dry-run", action="store_true", help="scan + validate + count only; write nothing")
    a = ap.parse_args()
    raw = Path(a.raw)
    if a.task in ("grading", "both"):
        r = build_grading(raw, a.size, a.seed, a.limit, a.dry_run, LAB / "data/prepared/grading", LAB / "data/manifests", LAB / "reports/data_audit")
        print("grading", {k: r.get(k) for k in ("n_images", "split_counts", "class_distribution", "n_errors", "excluded", "dry_run")})
    if a.task in ("lesions", "both"):
        r = build_lesions(raw, a.seed, a.limit, a.dry_run, LAB / "data/manifests", LAB / "reports/data_audit")
        print("lesions", {k: r.get(k) for k in ("n_images", "split_counts", "mask_status", "n_errors", "dry_run")})
    return 0


if __name__ == "__main__":
    sys.exit(main())
