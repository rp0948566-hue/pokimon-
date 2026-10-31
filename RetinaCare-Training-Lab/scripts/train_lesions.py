#!/usr/bin/env python
"""P5: GPU fine-tuning of one lesion segmenter (MA | HE | EX | SE) on IDRiD pixel masks (candidate, not deployed).

Only images with a real mask (mask_status=ok) are used; missing masks are never treated as empty.
SE is soft exudate; mapping to the deployed CWS model is unverified and not assumed.
"""
import argparse
import sys
from pathlib import Path

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

from trainlab.data.datasets import read_manifest  # noqa: E402
from trainlab.train import segmentation_run as R  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--lesion", required=True, choices=["MA", "HE", "EX", "SE"])
    R.add_common_args(ap, "lesion_idrid.yaml")
    a = ap.parse_args()
    load = lambda split: (lambda d: d[(d["lesion"] == a.lesion) & (d["mask_status"] == "ok")])(  # noqa: E731
        read_manifest(LAB / f"data/manifests/{split}/idrid_lesions_{split}.csv", ("image_path", "mask_path", "lesion", "mask_status")))
    info = R.run(a, load("train"), load("val"), "lesions", a.lesion)
    return 0 if not info["status"].startswith("failed") else 1


if __name__ == "__main__":
    sys.exit(main())
