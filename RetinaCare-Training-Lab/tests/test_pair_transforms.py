import numpy as np

from trainlab.data import transforms as T


def test_paired_augment_keeps_image_and_mask_aligned():
    rng = np.random.default_rng(0)
    for _ in range(50):
        mask = np.zeros((64, 64), np.uint8)
        y, x = rng.integers(0, 40, 2)
        mask[y:y + 12, x:x + 20] = 1  # asymmetric block
        img = np.repeat((mask * 200)[..., None], 3, 2)
        i2, m2 = T.paired_augment(img, mask, rng)
        assert ((i2[..., 0] > 100) == (m2 > 0)).all()
        assert m2.dtype == np.uint8 and set(np.unique(m2)) <= {0, 1}  # nearest/no interpolation


def test_random_patch_positive_bias_and_tiles_cover_image():
    rng = np.random.default_rng(1)
    img = np.zeros((200, 300, 3), np.uint8); mask = np.zeros((200, 300), np.uint8)
    mask[150, 250] = 1
    hits = sum(T.random_patch(img, mask, 64, rng, pos_bias=1.0)[1].any() for _ in range(20))
    assert hits == 20
    cover = np.zeros((200, 300), bool)
    for y, x in T.tile_grid(200, 300, 64):
        cover[y:y + 64, x:x + 64] = True
    assert cover.all()


def test_letterbox_preserves_aspect_and_crop_removes_border():
    img = np.zeros((100, 200, 3), np.uint8); img[20:80, 50:150] = 120
    crop, box = T.crop_fundus(img)
    assert crop.shape[:2] == (60, 100) and box == (50, 20, 150, 80)
    lb = T.letterbox(crop, 64)
    assert lb.shape == (64, 64, 3) and lb[0].sum() == 0 and lb[32].sum() > 0
