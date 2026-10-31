import sys
import zipfile
from pathlib import Path

import numpy as np
import pandas as pd
import pytest
from PIL import Image

from trainlab.data.archive import safe_extract
from trainlab.data.prepare import check_masks
from trainlab.data.validators import near_duplicate_pairs, thumb, validate_image, validate_mask

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import download_datasets  # noqa: E402
import prepare_drive  # noqa: E402


def test_corrupt_and_tiny_images_rejected(tmp_path):
    bad = tmp_path / "bad.png"; bad.write_bytes(b"not an image")
    with pytest.raises(ValueError, match="unreadable"):
        validate_image(bad)
    tiny = tmp_path / "t.png"; Image.new("RGB", (8, 8)).save(tiny)
    with pytest.raises(ValueError, match="too small"):
        validate_image(tiny)


def test_mask_size_mismatch_rejected_and_missing_is_not_empty(tmp_path):
    Image.new("L", (100, 100)).save(tmp_path / "m.png")
    with pytest.raises(ValueError, match="mismatch"):
        validate_mask(tmp_path / "m.png", (80, 100))
    assert validate_mask(tmp_path / "m.png", (100, 100))["mask_empty"] is True
    df = pd.DataFrame({"mask_path": [str(tmp_path / "m.png"), None], "width": [100, 100], "height": [100, 100]})
    out = check_masks(df)
    assert out["mask_status"].tolist() == ["ok", "missing"] and out["mask_empty"].tolist()[0] is True and out["mask_empty"].isna().tolist()[1]


def test_thumb_correlation_flags_resize_not_other_images():
    rng = np.random.default_rng(0)
    mk = lambda: Image.fromarray(np.kron(rng.integers(0, 255, (8, 8, 3), dtype=np.uint8), np.ones((16, 16, 1), np.uint8)))  # noqa: E731
    a, b = mk(), mk()
    pairs = near_duplicate_pairs({"a": thumb(a), "a2": thumb(a.resize((96, 96))), "b": thumb(b)})
    assert [(x, y) for x, y, _ in pairs] == [("a", "a2")]


def test_zip_slip_rejected(tmp_path):
    z = tmp_path / "evil.zip"
    with zipfile.ZipFile(z, "w") as f:
        f.writestr("../escape.txt", "x")
    with pytest.raises(ValueError, match="unsafe"):
        safe_extract(z, tmp_path / "out")
    assert not (tmp_path / "escape.txt").exists()


def test_download_check_is_blocked_without_credentials(monkeypatch, tmp_path):
    monkeypatch.setattr(download_datasets, "kaggle_credentials", lambda: {"present": False})
    monkeypatch.setattr(download_datasets, "LAB", tmp_path)
    src = {"url": "https://example/x/data", "access": "a", "est_gib": 1, "method": "auto_kaggle", "kaggle_competition": "c"}
    res = download_datasets.check("aptos", src)
    assert res["status"] == "BLOCKED" and "kaggle.json" in res["blocker"]
    assert download_datasets.download_aptos(src, accepted=False)["status"] == "BLOCKED"


def test_drive_withheld_test_references_not_fabricated(tmp_path):
    raw = tmp_path / "drive"
    for part, n in (("training", 20), ("test", 4)):
        for sub in ("images", "mask") + (("1st_manual",) if part == "training" else ()):
            (raw / part / sub).mkdir(parents=True)
        for k in range(1, n + 1):
            i = k + (20 if part == "training" else 0)
            Image.fromarray(np.random.default_rng(k + (50 if part == "test" else 0)).integers(0, 255, (96, 96, 3), dtype=np.uint8)).save(raw / part / "images" / f"{i:02d}_{part}.png")
            Image.new("L", (96, 96), 255).save(raw / part / "mask" / f"{i:02d}_{part}_mask.png")
            if part == "training":
                m = np.zeros((96, 96), np.uint8); m[10:20, 10:50] = 255
                Image.fromarray(m).save(raw / part / "1st_manual" / f"{i:02d}_manual1.png")
    rep = prepare_drive.build(raw, 1, tmp_path / "man", tmp_path / "audit")
    assert rep["split_counts"] == {"test": 4, "train": 16, "val": 4} and rep["test_references_present"] == 0 and rep["leakage"]["ok"]
    te = pd.read_csv(tmp_path / "man/external/drive_vessels_test.csv")
    assert (te["annotation_type"] == "withheld").all()
