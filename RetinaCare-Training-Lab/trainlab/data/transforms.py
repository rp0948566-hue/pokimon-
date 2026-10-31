"""Image preprocessing and paired image+mask augmentation (masks: nearest, same geometry)."""
from __future__ import annotations

import numpy as np
from PIL import Image

IMAGENET_MEAN = np.array([0.485, 0.456, 0.406], dtype=np.float32)
IMAGENET_STD = np.array([0.229, 0.224, 0.225], dtype=np.float32)


def crop_fundus(img: np.ndarray, tol: int = 10) -> tuple[np.ndarray, tuple[int, int, int, int]]:
    """Crop black border; returns (crop, (x0, y0, x1, y1)). Whole image if nothing exceeds tol."""
    gray = img.max(axis=2) if img.ndim == 3 else img
    ys, xs = np.where(gray > tol)
    if len(xs) == 0:
        return img, (0, 0, img.shape[1], img.shape[0])
    x0, x1, y0, y1 = xs.min(), xs.max() + 1, ys.min(), ys.max() + 1
    return img[y0:y1, x0:x1], (int(x0), int(y0), int(x1), int(y1))


def letterbox(img: np.ndarray, size: int, resample=Image.BILINEAR, fill: int = 0) -> np.ndarray:
    """Aspect-preserving resize into a size x size canvas (centered)."""
    h, w = img.shape[:2]
    s = size / max(h, w)
    nh, nw = max(1, round(h * s)), max(1, round(w * s))
    resized = np.asarray(Image.fromarray(img).resize((nw, nh), resample))
    canvas = np.full((size, size) + img.shape[2:], fill, dtype=img.dtype)
    y0, x0 = (size - nh) // 2, (size - nw) // 2
    canvas[y0:y0 + nh, x0:x0 + nw] = resized
    return canvas


def preprocess_grading(path, size: int) -> np.ndarray:
    """RGB uint8 square: fundus crop + letterbox. Used to build the prepared cache."""
    img = np.asarray(Image.open(path).convert("RGB"))
    crop, _ = crop_fundus(img)
    return letterbox(crop, size)


def to_tensor_array(img_u8: np.ndarray) -> np.ndarray:
    """HWC uint8 -> CHW float32 ImageNet-normalised."""
    x = (img_u8.astype(np.float32) / 255.0 - IMAGENET_MEAN) / IMAGENET_STD
    return np.ascontiguousarray(x.transpose(2, 0, 1))


def augment_image(img: np.ndarray, rng: np.random.Generator) -> np.ndarray:
    """Cls augmentation on HWC uint8: flips, 90-degree-free small rotation, brightness/contrast."""
    if rng.random() < 0.5:
        img = img[:, ::-1]
    if rng.random() < 0.5:
        img = img[::-1]
    angle = rng.uniform(-20, 20)
    img = np.asarray(Image.fromarray(np.ascontiguousarray(img)).rotate(angle, resample=Image.BILINEAR))
    a, b = rng.uniform(0.85, 1.15), rng.uniform(-12, 12)
    return np.clip(img.astype(np.float32) * a + b, 0, 255).astype(np.uint8)


def paired_augment(img: np.ndarray, mask: np.ndarray, rng: np.random.Generator):
    """Identical geometry on image and mask (flips, rot90). Photometric jitter touches image only."""
    if rng.random() < 0.5:
        img, mask = img[:, ::-1], mask[:, ::-1]
    if rng.random() < 0.5:
        img, mask = img[::-1], mask[::-1]
    k = int(rng.integers(0, 4))
    img, mask = np.rot90(img, k), np.rot90(mask, k)
    a, b = rng.uniform(0.9, 1.1), rng.uniform(-8, 8)
    img = np.clip(img.astype(np.float32) * a + b, 0, 255).astype(np.uint8)
    return np.ascontiguousarray(img), np.ascontiguousarray(mask)


def random_patch(img: np.ndarray, mask: np.ndarray, size: int, rng: np.random.Generator,
                 pos_bias: float = 0.5):
    """Crop a size x size patch. With prob pos_bias centre on a random positive pixel (tiny lesions)."""
    h, w = mask.shape[:2]
    size_h, size_w = min(size, h), min(size, w)
    ys, xs = np.nonzero(mask) if (rng.random() < pos_bias and mask.any()) else (None, None)
    if ys is not None and len(ys):
        i = int(rng.integers(0, len(ys)))
        cy, cx = ys[i], xs[i]
        y0 = int(np.clip(cy - size_h // 2, 0, h - size_h))
        x0 = int(np.clip(cx - size_w // 2, 0, w - size_w))
    else:
        y0 = int(rng.integers(0, h - size_h + 1))
        x0 = int(rng.integers(0, w - size_w + 1))
    return img[y0:y0 + size_h, x0:x0 + size_w], mask[y0:y0 + size_h, x0:x0 + size_w]


def tile_grid(h: int, w: int, size: int) -> list[tuple[int, int]]:
    """Top-left corners covering the image with size x size tiles (last row/col shifted back)."""
    ys = list(range(0, max(h - size, 0) + 1, size)) or [0]
    xs = list(range(0, max(w - size, 0) + 1, size)) or [0]
    if ys[-1] + size < h:
        ys.append(h - size)
    if xs[-1] + size < w:
        xs.append(w - size)
    return [(y, x) for y in ys for x in xs]
