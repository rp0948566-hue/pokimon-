"""Shared manifest-building and audit helpers for the prepare_* scripts."""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pandas as pd
from PIL import Image

from trainlab.data import splits as S
from trainlab.data import transforms as T
from trainlab.data.validators import near_duplicate_pairs, validate_image, validate_mask


def scan_images(paths: dict[str, Path]) -> tuple[pd.DataFrame, list[dict]]:
    """Validate every image. Returns (meta rows indexed by image_id, errors). Bad files are listed, never dropped silently."""
    rows, errors = [], []
    for iid, p in paths.items():
        try:
            rows.append({"image_id": iid, "image_path": str(p), **validate_image(p)})
        except ValueError as e:
            errors.append({"image_id": iid, "path": str(p), "error": str(e)})
    return pd.DataFrame(rows), errors


def add_duplicate_groups(df: pd.DataFrame, min_corr: float = 0.985) -> tuple[pd.DataFrame, list]:
    """group_id = connected component of exact-sha256 and near-duplicate (thumbnail correlation >= min_corr) images."""
    near = near_duplicate_pairs(dict(zip(df["image_id"], df["thumb"])), min_corr)
    df = df.copy()
    parent = {i: i for i in df["image_id"]}

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for a, b, _ in near:
        parent[find(b)] = find(a)
    df["near_key"] = df["image_id"].map(find)
    df["group_id"] = S.assign_duplicate_groups(df, hash_cols=("sha256", "near_key"))
    return df, near


def cache_squares(df: pd.DataFrame, out_dir: Path, size: int) -> pd.Series:
    out_dir.mkdir(parents=True, exist_ok=True)
    out = []
    for r in df.itertuples():
        p = out_dir / f"{r.image_id}.png"
        if not p.exists():
            Image.fromarray(T.preprocess_grading(r.image_path, size)).save(p)
        out.append(str(p))
    return pd.Series(out, index=df.index)


def contact_sheet(paths: list[str], out: Path, n: int = 12, seed: int = 0, thumb: int = 192) -> Path | None:
    if not paths:
        return None
    rng = np.random.default_rng(seed)
    pick = [paths[i] for i in rng.permutation(len(paths))[:n]]
    cols = 4
    sheet = Image.new("RGB", (cols * thumb, ((len(pick) + cols - 1) // cols) * thumb))
    for k, p in enumerate(pick):
        sheet.paste(Image.open(p).convert("RGB").resize((thumb, thumb)), ((k % cols) * thumb, (k // cols) * thumb))
    sheet.save(out)
    return out


def overlay_sheet(pairs: list[tuple[str, str]], out: Path, n: int = 12, seed: int = 0, thumb: int = 256) -> Path | None:
    """Image with mask outline in red: visual alignment audit."""
    if not pairs:
        return None
    rng = np.random.default_rng(seed)
    pick = [pairs[i] for i in rng.permutation(len(pairs))[:n]]
    cols = 4
    sheet = Image.new("RGB", (cols * thumb, ((len(pick) + cols - 1) // cols) * thumb))
    for k, (ip, mp) in enumerate(pick):
        im = np.asarray(Image.open(ip).convert("RGB").resize((thumb, thumb), Image.BILINEAR)).copy()
        m = np.asarray(Image.open(mp).convert("L").resize((thumb, thumb), Image.NEAREST)) > 0
        edge = m & ~(np.roll(m, 1, 0) & np.roll(m, -1, 0) & np.roll(m, 1, 1) & np.roll(m, -1, 1))
        im[edge | m & (np.arange(thumb)[:, None] % 2 == 0)] = (255, 0, 0)
        sheet.paste(Image.fromarray(im), ((k % cols) * thumb, (k // cols) * thumb))
    sheet.save(out)
    return out


def write_audit(name: str, manifests: dict[str, pd.DataFrame], errors: list, near: list, out_dir: Path,
                extra: dict | None = None) -> dict:
    out_dir.mkdir(parents=True, exist_ok=True)
    near = [(a, b, round(c, 4)) for a, b, c in near]
    allm = pd.concat(manifests.values(), ignore_index=True)
    rep = {"dataset": name, "n_images": int(len(allm)), "n_errors": len(errors), "errors": errors[:50],
           "near_duplicate_pairs": [(a, b, d) for a, b, d in near][:100], "n_near_duplicate_pairs": len(near),
           "split_counts": {k: int(len(v)) for k, v in manifests.items()},
           "class_distribution": {k: S.class_distribution(v) for k, v in manifests.items() if "label" in v},
           "leakage": S.leakage_report(manifests), **(extra or {})}
    (out_dir / f"{name}_audit.json").write_text(json.dumps(rep, indent=1, default=str))
    return rep


def check_masks(df: pd.DataFrame, path_col: str = "mask_path") -> pd.DataFrame:
    """Add mask_status (ok/missing/invalid) and mask_empty. Missing != empty."""
    status, empty = [], []
    for r in df.itertuples():
        mp = getattr(r, path_col)
        if not isinstance(mp, str) or not Path(mp).is_file():
            status.append("missing"); empty.append(None); continue
        try:
            info = validate_mask(mp, (r.width, r.height))
            status.append("ok"); empty.append(info["mask_empty"])
        except ValueError as e:
            status.append(f"invalid: {e}"[:120]); empty.append(None)
    df = df.copy()
    df["mask_status"], df["mask_empty"] = status, empty
    return df
