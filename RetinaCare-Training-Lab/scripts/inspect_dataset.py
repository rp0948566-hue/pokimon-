#!/usr/bin/env python
"""P3: quick integrity inspection of a manifest CSV: files exist, sizes decode, labels valid, no cross-row duplicates."""
import argparse
import sys
from pathlib import Path

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

import pandas as pd  # noqa: E402

from trainlab.data.validators import validate_image  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("manifest", help="manifest CSV (columns image_id,image_path,label,...)")
    ap.add_argument("--path-col", default="image_path")
    ap.add_argument("--max-label", type=int, default=4)
    ap.add_argument("--sample", type=int, default=50, help="decode N random images fully (0 = only check existence)")
    a = ap.parse_args()
    df = pd.read_csv(a.manifest)
    problems = []
    missing = df[~df[a.path_col].map(lambda p: isinstance(p, str) and Path(p).is_file())]
    if len(missing):
        problems.append(f"{len(missing)} files missing")
    if "label" in df and pd.api.types.is_numeric_dtype(df["label"]) and not df["label"].between(0, a.max_label).all():
        problems.append("labels outside 0..%d" % a.max_label)
    if df["image_id"].duplicated().any():
        problems.append("duplicate image_id")
    for p in df.drop(missing.index)[a.path_col].sample(n=min(a.sample, len(df) - len(missing)), random_state=0) if a.sample else []:
        try:
            validate_image(p)
        except ValueError as e:
            problems.append(str(e))
    print(f"{a.manifest}: rows={len(df)} problems={problems or 'none'}")
    if "label" in df:
        print(df["label"].value_counts().sort_index().to_dict())
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
