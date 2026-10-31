"""Deterministic, group-aware splits and leakage checks."""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pandas as pd

SPLIT_COLS = ["image_id", "image_path", "prepared_path", "label", "split", "group_id", "source",
              "sha256", "width", "height", "annotation_type"]


def stratified_group_split(df: pd.DataFrame, label_col: str, group_col: str,
                           fractions: dict[str, float], seed: int = 42) -> pd.Series:
    """Assign each row a split name; rows sharing a group never straddle splits.

    Greedy per-label allocation over shuffled groups so each split gets ~its fraction of every
    class. Deterministic for a given seed. A group takes the label of its first row.
    """
    if abs(sum(fractions.values()) - 1) > 1e-9:
        raise ValueError(f"fractions must sum to 1, got {fractions}")
    rng = np.random.default_rng(seed)
    g = df.groupby(group_col, sort=True)[label_col].first()
    names = list(fractions)
    out: dict = {}
    for lab in sorted(g.unique()):
        groups = list(g.index[g == lab])
        rng.shuffle(groups)
        n = len(groups)
        cuts = np.round(np.cumsum([fractions[k] for k in names]) * n).astype(int)
        start = 0
        for name, end in zip(names, cuts):
            for grp in groups[start:end]:
                out[grp] = name
            start = end
    return df[group_col].map(out)


def assign_duplicate_groups(df: pd.DataFrame, hash_cols=("sha256", "ahash")) -> pd.Series:
    """Union-find over exact/near-duplicate keys -> one group id per connected component."""
    parent = list(range(len(df)))

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    for col in hash_cols:
        if col not in df:
            continue
        first: dict = {}
        for i, v in enumerate(df[col].tolist()):
            if v in first:
                parent[find(i)] = find(first[v])
            else:
                first[v] = i
    roots = [find(i) for i in range(len(df))]
    # group id = id of first member, stable across runs for the same input order
    first_member: dict = {}
    for i, r in enumerate(roots):
        first_member.setdefault(r, df["image_id"].iloc[i])
    return pd.Series([first_member[r] for r in roots], index=df.index)


def leakage_report(manifests: dict[str, pd.DataFrame]) -> dict:
    """Cross-split overlap on image_id, sha256, group_id. Any non-empty list == leakage."""
    names = list(manifests)
    rep: dict = {"overlaps": [], "ok": True}
    for i, a in enumerate(names):
        for b in names[i + 1:]:
            for col in ("image_id", "sha256", "group_id"):
                if col not in manifests[a] or col not in manifests[b]:
                    continue
                common = set(manifests[a][col].dropna()) & set(manifests[b][col].dropna())
                if common:
                    rep["ok"] = False
                    rep["overlaps"].append({"a": a, "b": b, "column": col, "count": len(common),
                                            "examples": sorted(map(str, common))[:5]})
    return rep


def assert_no_leakage(manifests: dict[str, pd.DataFrame]) -> None:
    rep = leakage_report(manifests)
    if not rep["ok"]:
        raise RuntimeError(f"split leakage detected: {json.dumps(rep['overlaps'])[:500]}")


def class_distribution(df: pd.DataFrame, label_col: str = "label") -> dict:
    return {str(k): int(v) for k, v in df[label_col].value_counts().sort_index().items()}


def write_split_manifests(df: pd.DataFrame, root, prefix: str, test_dir: str = "test") -> dict:
    """Write manifests/<split>/<prefix>_<split>.csv; locked test is made read-only (0444)."""
    root = Path(root)
    paths = {}
    for split, sub in df.groupby("split"):
        d = root / (test_dir if split in ("test", "locked_test") else split)
        d.mkdir(parents=True, exist_ok=True)
        p = d / f"{prefix}_{split}.csv"
        if p.exists():
            p.chmod(0o644)
        sub.to_csv(p, index=False)
        if split in ("test", "locked_test"):
            p.chmod(0o444)
        paths[split] = str(p)
    return paths
