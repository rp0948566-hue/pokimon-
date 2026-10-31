"""Synthetic fixtures: logic tests only. Nothing generated here is ever evidence of model quality."""
import numpy as np
import pandas as pd
import pytest
from PIL import Image


def fake_fundus(path, seed, size=96):
    rng = np.random.default_rng(seed)
    yy, xx = np.mgrid[:size, :size]
    disc = ((yy - size / 2) ** 2 + (xx - size / 2) ** 2) < (size * 0.45) ** 2
    blobs = np.kron(rng.integers(30, 220, (6, 6, 3)), np.ones((size // 6, size // 6, 1), int))
    img = (blobs * disc[..., None]).astype(np.uint8)
    Image.fromarray(img).save(path)


@pytest.fixture
def aptos_raw(tmp_path):
    """40 distinct fake images over labels 0..4 + one exact copy (duplicate) of image 0."""
    d = tmp_path / "raw/aptos"
    (d / "train_images").mkdir(parents=True)
    rows = []
    for i in range(40):
        fake_fundus(d / "train_images" / f"img{i:03d}.png", i)
        rows.append({"id_code": f"img{i:03d}", "diagnosis": i % 5})
    (d / "train_images" / "img040.png").write_bytes((d / "train_images" / "img000.png").read_bytes())
    rows.append({"id_code": "img040", "diagnosis": 0})
    pd.DataFrame(rows).to_csv(d / "train.csv", index=False)
    return d
