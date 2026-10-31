#!/usr/bin/env python
"""P3: IDRiD -> grading manifests (official 413/103) and lesion manifests (official 54/27).

Official partitions are preserved; official training is split into train/val (seeded). Official
test annotations are only written to the read-only test manifests. A lesion mask that does not exist
is recorded as mask_status=missing (NOT as an all-zero mask).
Lesion keys: MA (microaneurysms), HE (haemorrhages), EX (hard exudates), SE (soft exudates).
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
IMG_EXT = {".jpg", ".jpeg", ".png", ".tif", ".tiff"}


def part(p: Path) -> str | None:
    s = "/".join(p.parts).lower()
    return "train" if "training" in s else "test" if "testing" in s else None


def find_grading(raw: Path) -> pd.DataFrame:
    rows = []
    for csv in raw.rglob("*.csv"):
        if "grading" not in csv.name.lower().replace("_", " "):
            continue
        sp = part(csv) or ("train" if "train" in csv.name.lower() else "test" if "test" in csv.name.lower() else None)
        d = pd.read_csv(csv)
        d.columns = [c.strip() for c in d.columns]
        name_col = next((c for c in d.columns if c.lower().startswith("image")), None)
        grade_col = next((c for c in d.columns if "retinopathy" in c.lower()), None)
        if not (name_col and grade_col and sp):
            continue
        for n, g in zip(d[name_col], d[grade_col]):
            rows.append({"image_id": str(n).strip(), "label": g, "official_split": sp})
    if not rows:
        raise FileNotFoundError(f"no IDRiD grading label CSV found under {raw}")
    return pd.DataFrame(rows)


def index_images(raw: Path, grading_only: bool) -> dict[str, Path]:
    """stem -> path, restricted to the Disease Grading or Segmentation image folders."""
    out = {}
    for p in raw.rglob("*"):
        if p.suffix.lower() not in IMG_EXT or not p.is_file():
            continue
        s = "/".join(p.parts).lower()
        is_grade = "disease grading" in s and "original" in s
        is_seg = "segmentation" in s and "original" in s
        if (grading_only and is_grade) or (not grading_only and is_seg):
            out[p.stem] = p
    return out


def build_grading(raw, size, seed, prepared, manifests, audit) -> dict:
    lab = find_grading(raw)
    bad = lab[~lab["label"].isin(range(5))]
    if len(bad):
        raise ValueError(f"{len(bad)} grades outside 0..4: {bad.head(3).to_dict('records')}")
    idx = index_images(raw, True)
    paths = {i: idx[i] for i in lab["image_id"] if i in idx}
    errors = [{"image_id": i, "error": "image file missing"} for i in lab["image_id"] if i not in idx]
    meta, e2 = P.scan_images(paths)
    df = lab.merge(meta, on="image_id")
    df, near = P.add_duplicate_groups(df)
    df["split"] = "test"
    tr = df["official_split"] == "train"
    df.loc[tr, "split"] = S.stratified_group_split(df[tr], "label", "group_id", {"train": 0.85, "val": 0.15}, seed)
    df["prepared_path"] = P.cache_squares(df, prepared / f"idrid_grading_{size}", size)
    df["source"], df["annotation_type"] = "idrid_grading", "dr_grade_0_4"
    S.assert_no_leakage({k: v for k, v in df.groupby("split")})
    parts = {k: v[S.SPLIT_COLS + ["official_split"]] for k, v in df.groupby("split")}
    # IDRiD test is an EXTERNAL set for APTOS-trained models -> manifests/external
    out = {}
    for k, v in parts.items():
        out.update(S.write_split_manifests(v.assign(split=k), manifests / ("external" if k == "test" else "."), "idrid_grading",
                                           test_dir="." if k == "test" else "test"))
    rep = P.write_audit("idrid_grading", parts, errors + e2, near, audit, {"official_counts": df["official_split"].value_counts().to_dict(), "manifests": out})
    return rep


def build_lesions(raw, seed, manifests, audit) -> dict:
    idx = index_images(raw, False)
    masks: dict[tuple[str, str], Path] = {}
    for p in raw.rglob("*"):
        if p.suffix.lower() in IMG_EXT and p.is_file() and "groundtruth" in "/".join(p.parts).lower():
            for lz in LESIONS:
                if p.stem.upper().endswith(f"_{lz}"):
                    masks[(p.stem[: -len(lz) - 1], lz)] = p
    if not idx:
        raise FileNotFoundError(f"no IDRiD segmentation images under {raw}")
    meta, errors = P.scan_images(idx)
    meta["official_split"] = meta["image_path"].map(lambda s: part(Path(s)))
    rows = []
    for lz in LESIONS:
        d = meta.copy()
        d["lesion"] = lz
        d["mask_path"] = d["image_id"].map(lambda i: str(masks[(i, lz)]) if (i, lz) in masks else None)
        rows.append(P.check_masks(d))
    df = pd.concat(rows, ignore_index=True)
    df["group_id"] = df["image_id"]  # one image per patient/eye is not guaranteed; image is the unit
    df["split"] = "test"
    for lz in LESIONS:
        sel = (df["lesion"] == lz) & (df["official_split"] == "train")
        df.loc[sel, "split"] = S.stratified_group_split(df[sel].assign(label=0), "label", "group_id", {"train": 0.8, "val": 0.2}, seed)
    df["label"], df["prepared_path"], df["source"] = df["lesion"], None, "idrid_segmentation"
    df["annotation_type"] = "pixel_mask_" + df["lesion"]
    S.assert_no_leakage({k: v for k, v in df.groupby("split")})
    parts = {k: v for k, v in df.groupby("split")}
    out = {}
    for k, v in parts.items():
        out.update(S.write_split_manifests(v.assign(split=k), manifests / ("external" if k == "test" else "."), "idrid_lesions",
                                           test_dir="." if k == "test" else "test"))
    extra = {"mask_status": df.groupby(["lesion", "mask_status"]).size().unstack(fill_value=0).to_dict("index"), "manifests": out}
    rep = P.write_audit("idrid_lesions", {k: v.assign(label=0) for k, v in parts.items()}, errors, [], audit, extra)
    ok = df[df["mask_status"] == "ok"]
    P.overlay_sheet(list(zip(ok["image_path"], ok["mask_path"])), audit / "idrid_lesion_overlays.png", n=12, seed=seed)
    return rep


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--raw", default=str(LAB / "data/raw/idrid"))
    ap.add_argument("--size", type=int, default=384)
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--task", choices=["grading", "lesions", "both"], default="both")
    a = ap.parse_args()
    raw = Path(a.raw)
    for t, fn in (("grading", lambda: build_grading(raw, a.size, a.seed, LAB / "data/prepared/grading", LAB / "data/manifests", LAB / "reports/data_audit")),
                  ("lesions", lambda: build_lesions(raw, a.seed, LAB / "data/manifests", LAB / "reports/data_audit"))):
        if a.task in (t, "both"):
            rep = fn()
            print(t, rep["split_counts"], "errors=", rep["n_errors"], "leakage_ok=", rep["leakage"]["ok"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
