"""Synthetic layout test of the ASSUMED DDR structure: logic only, not evidence about the real archive."""
import sys
from pathlib import Path

import numpy as np
import pandas as pd
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import prepare_ddr  # noqa: E402

from conftest import fake_fundus  # noqa: E402


def _layout(raw):
    g = raw / "DDR/DR_grading"
    labels = {"train": [], "valid": [], "test": []}
    k = 0
    for sp, n in (("train", 15), ("valid", 5), ("test", 5)):
        (g / sp).mkdir(parents=True)
        for i in range(n):
            fake_fundus(g / sp / f"{k}.jpg", k); labels[sp].append(f"{k}.jpg {i % 5}"); k += 1
    # one ungradable (grade 5) train image and one train image that duplicates a test image
    fake_fundus(g / "train/u.jpg", 900); labels["train"].append("u.jpg 5")
    (g / "train/dup.jpg").write_bytes((g / "test/20.jpg").read_bytes()); labels["train"].append("dup.jpg 2")
    for sp in labels:
        (g / f"{sp}.txt").write_text("\n".join(labels[sp]))
    s = raw / "DDR/lesion_segmentation"
    for sp, n in (("train", 3), ("valid", 1), ("test", 1)):
        (s / sp / "image").mkdir(parents=True)
        for lz in prepare_ddr.LESIONS:
            (s / sp / "label" / lz).mkdir(parents=True)
        for i in range(n):
            fake_fundus(s / sp / "image" / f"s{sp}{i}.jpg", 500 + i + 10 * {"train": 0, "valid": 1, "test": 2}[sp])
            m = np.zeros((96, 96), np.uint8); m[10:20, 10:20] = 255
            Image.fromarray(m).save(s / sp / "label/MA" / f"s{sp}{i}.tif")  # only MA masks exist


def test_ddr_grading_and_lesions(tmp_path):
    _layout(tmp_path / "raw")
    raw = tmp_path / "raw"
    dry = prepare_ddr.build_grading(raw, 32, 1, None, True, tmp_path / "p", tmp_path / "m", tmp_path / "a")
    assert dry["excluded"]["ungradable_5"] == 1 and dry["quarantined"] == 1 and not (tmp_path / "m").exists()
    rep = prepare_ddr.build_grading(raw, 32, 1, None, False, tmp_path / "p", tmp_path / "m", tmp_path / "a")
    assert rep["leakage"]["ok"] and rep["split_counts"] == {"test": 5, "train": 15, "val": 5}
    assert (tmp_path / "m/external/ddr_grading_test.csv").exists()
    les = prepare_ddr.build_lesions(raw, 1, None, False, tmp_path / "m", tmp_path / "a")
    ms = les["mask_status"]
    assert ms["MA"]["ok"] == 5 and ms["HE"]["missing"] == 5 and ms["HE"]["ok"] == 0  # missing != empty
