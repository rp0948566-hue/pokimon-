#!/usr/bin/env python
"""P5: GPU fine-tuning of a vessel segmenter on DRIVE TRAINING references (16 train / 4 val); test refs are withheld."""
import argparse
import sys
from pathlib import Path

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

from trainlab.data.datasets import read_manifest  # noqa: E402
from trainlab.train import segmentation_run as R  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    R.add_common_args(ap, "vessel_drive.yaml")
    a = ap.parse_args()
    load = lambda split: (lambda d: d[d["mask_status"] == "ok"])(  # noqa: E731
        read_manifest(LAB / f"data/manifests/{split}/drive_vessels_{split}.csv", ("image_path", "mask_path", "mask_status")))
    info = R.run(a, load("train"), load("val"), "vessels", "drive")
    return 0 if not info["status"].startswith("failed") else 1


if __name__ == "__main__":
    sys.exit(main())
