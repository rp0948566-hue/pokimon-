"""Manifest-driven PyTorch datasets. Every sample is a real file listed in a manifest CSV."""
from __future__ import annotations

from pathlib import Path

import numpy as np
import pandas as pd
import torch
from PIL import Image
from torch.utils.data import Dataset

from trainlab.data import transforms as T


def read_manifest(path, require_cols=()) -> pd.DataFrame:
    df = pd.read_csv(path)
    missing = [c for c in require_cols if c not in df]
    if missing:
        raise ValueError(f"{path}: missing columns {missing}")
    return df


class GradingDataset(Dataset):
    """Classification from the prepared square cache (prepared_path). label: int class index."""

    def __init__(self, df: pd.DataFrame, size: int, train: bool, seed: int = 0, label_col: str = "label"):
        bad = df["prepared_path"].isna() | ~df["prepared_path"].map(lambda p: isinstance(p, str) and Path(p).is_file())
        if bad.any():
            raise FileNotFoundError(f"{int(bad.sum())} prepared images missing, e.g. {df.loc[bad, 'image_id'].head(3).tolist()}")
        self.df, self.size, self.train, self.label_col = df.reset_index(drop=True), size, train, label_col
        self.seed = seed
        self._epoch = 0

    def set_epoch(self, epoch: int) -> None:
        self._epoch = epoch

    def __len__(self) -> int:
        return len(self.df)

    def __getitem__(self, i: int):
        row = self.df.iloc[i]
        img = np.asarray(Image.open(row["prepared_path"]).convert("RGB"))
        if self.train:
            rng = np.random.default_rng([self.seed, self._epoch, i])
            img = T.augment_image(img, rng)
        if img.shape[0] != self.size:
            img = np.asarray(Image.fromarray(img).resize((self.size, self.size), Image.BILINEAR))
        return torch.from_numpy(T.to_tensor_array(img)), int(row[self.label_col])


def load_scaled(path, scale: float, mask: bool) -> np.ndarray:
    im = Image.open(path)
    if mask:
        im = im.convert("L")
    else:
        im = im.convert("RGB")
    if scale != 1.0:
        im = im.resize((round(im.width * scale), round(im.height * scale)), Image.NEAREST if mask else Image.BILINEAR)
    a = np.asarray(im)
    return (a > 0).astype(np.uint8) if mask else a


class SegDataset(Dataset):
    """Binary segmentation. Train: random positive-biased patches. Eval: deterministic tile grid.

    Rows must have image_path and mask_path (rows without a mask are the caller's to drop; a
    missing mask is NOT an empty mask). Images are held in RAM downscaled by `scale`.
    """

    def __init__(self, df: pd.DataFrame, patch: int, train: bool, scale: float = 0.5,
                 patches_per_image: int = 8, pos_bias: float = 0.6, seed: int = 0):
        if df["mask_path"].isna().any():
            raise ValueError("SegDataset got rows with missing mask_path; filter them before training")
        self.items = [(load_scaled(r.image_path, scale, False), load_scaled(r.mask_path, scale, True), r.image_id)
                      for r in df.itertuples()]
        self.patch, self.train, self.ppi, self.pos_bias, self.seed = patch, train, patches_per_image, pos_bias, seed
        self._epoch = 0
        self.tiles = [] if train else [(k, y, x) for k, (im, _, _) in enumerate(self.items)
                                       for (y, x) in T.tile_grid(im.shape[0], im.shape[1], patch)]

    def set_epoch(self, epoch: int) -> None:
        self._epoch = epoch

    def __len__(self) -> int:
        return len(self.items) * self.ppi if self.train else len(self.tiles)

    def __getitem__(self, i: int):
        if self.train:
            k = i % len(self.items)
            img, msk, _ = self.items[k]
            rng = np.random.default_rng([self.seed, self._epoch, i])
            img, msk = T.random_patch(img, msk, self.patch, rng, self.pos_bias)
            img, msk = T.paired_augment(img, msk, rng)
        else:
            k, y, x = self.tiles[i]
            img, msk, _ = self.items[k]
            img = img[y:y + self.patch, x:x + self.patch]
            msk = msk[y:y + self.patch, x:x + self.patch]
        return torch.from_numpy(T.to_tensor_array(img)), torch.from_numpy(np.ascontiguousarray(msk)).float()[None]
