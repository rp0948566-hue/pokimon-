import sys
from pathlib import Path

import numpy as np
import pandas as pd
import pytest

from trainlab.models.adapters import GRADE_NAMES, screening_label

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import prepare_aptos  # noqa: E402


def test_screening_label_matches_stage1_definition():
    assert screening_label([0, 1, 2, 3, 4]).tolist() == [0, 1, 1, 1, 1]
    assert len(GRADE_NAMES) == 5


def _build(raw, tmp_path):
    return prepare_aptos.build(raw, 32, 7, {"train": 0.6, "val": 0.2, "locked_test": 0.2}, None,
                               tmp_path / "prep", tmp_path / "man", tmp_path / "audit")


def test_aptos_build_manifests_labels_and_duplicate_grouping(aptos_raw, tmp_path):
    rep = _build(aptos_raw, tmp_path)
    assert rep["leakage"]["ok"] and rep["n_errors"] == 0 and rep["n_images"] == 41
    tr = pd.read_csv(tmp_path / "man/train/aptos_train.csv")
    va = pd.read_csv(tmp_path / "man/val/aptos_val.csv")
    te = pd.read_csv(tmp_path / "man/test/aptos_locked_test.csv")
    assert len(tr) + len(va) + len(te) == 41 and set(tr["label"]) <= set(range(5))
    d = pd.concat([tr, va, te])
    assert d[d.image_id.isin(["img000", "img040"])]["split"].nunique() == 1  # exact copy stays together
    assert not (tmp_path / "man/test/aptos_locked_test.csv").stat().st_mode & 0o222  # read-only


def test_aptos_rejects_invalid_label_instead_of_converting(aptos_raw, tmp_path):
    csv = aptos_raw / "train.csv"
    d = pd.read_csv(csv)
    d.loc[3, "diagnosis"] = 7
    d.to_csv(csv, index=False)
    with pytest.raises(ValueError, match="outside 0..4"):
        _build(aptos_raw, tmp_path)
