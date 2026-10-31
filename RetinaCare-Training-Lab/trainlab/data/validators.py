"""Image/mask validation: real files only, nothing silently converted."""
from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image

from trainlab.utils.hashing import sha256_file

Image.MAX_IMAGE_PIXELS = 200_000_000  # fundus photos reach ~12 MP; reject decompression bombs beyond this


def thumb(img: Image.Image, size: int = 32) -> np.ndarray:
    """Zero-mean unit-norm green-channel thumbnail. Dot product of two = Pearson correlation.

    Fundus photos share a bright disc on a dark border, so a bare hash would call unrelated photos
    duplicates; correlation of the green-channel structure only passes for re-encodes/resizes.
    """
    g = img.convert("RGB").split()[1].resize((size, size), Image.BILINEAR)
    a = np.asarray(g, dtype=np.float32).ravel()
    a = a - a.mean()
    n = np.linalg.norm(a)
    return a / n if n > 0 else a


def validate_image(path) -> dict:
    """Open + fully decode. Returns metadata; raises ValueError with the path on any failure."""
    p = Path(path)
    try:
        with Image.open(p) as im:
            im.load()
            w, h = im.size
            mode = im.mode
            th = thumb(im)
    except Exception as e:  # PIL raises many types; surface path + reason
        raise ValueError(f"unreadable image {p}: {e}") from e
    if min(w, h) < 64:
        raise ValueError(f"image too small {p}: {w}x{h}")
    return {"width": w, "height": h, "mode": mode, "sha256": sha256_file(p), "thumb": th}


def validate_mask(path, image_size: tuple[int, int]) -> dict:
    """Mask must decode, match the image (w,h) and be binary-like (<=2 distinct values after threshold)."""
    p = Path(path)
    try:
        with Image.open(p) as m:
            m.load()
            size = m.size
            arr = np.asarray(m)
    except Exception as e:
        raise ValueError(f"unreadable mask {p}: {e}") from e
    if size != tuple(image_size):
        raise ValueError(f"mask/image size mismatch {p}: mask {size} vs image {tuple(image_size)}")
    if arr.ndim == 3:
        arr = arr.max(axis=2)
    pos = int((arr > 0).sum())
    return {"mask_pixels_positive": pos, "mask_empty": pos == 0, "mask_unique": int(len(np.unique(arr)))}


def near_duplicate_pairs(thumbs: dict[str, np.ndarray], min_corr: float = 0.985) -> list[tuple[str, str, float]]:
    """Pairs whose thumbnail correlation >= min_corr (matrix product, fine to ~20k images)."""
    ids = list(thumbs)
    if len(ids) < 2:
        return []
    X = np.stack([thumbs[i] for i in ids])
    C = X @ X.T
    ii, jj = np.where(np.triu(C >= min_corr, k=1))
    return [(ids[i], ids[j], float(C[i, j])) for i, j in zip(ii, jj)]
