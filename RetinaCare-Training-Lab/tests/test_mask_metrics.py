import numpy as np

from trainlab.evaluate.segmentation import segmentation_metrics
from trainlab.utils import metrics as M


def test_dice_iou_known_values_and_empty_policy():
    gt = np.zeros((10, 10), bool); gt[:5] = True
    pred = np.zeros((10, 10), bool); pred[:5, :5] = True
    s = M.dice_iou_counts(pred, gt)
    assert (s["tp"], s["fp"], s["fn"]) == (25, 0, 25)
    assert abs(s["dice"] - 2 * 25 / (2 * 25 + 25)) < 1e-9 and abs(s["iou"] - 0.5) < 1e-9
    e = M.dice_iou_counts(np.zeros((4, 4), bool), np.zeros((4, 4), bool))
    assert e["dice"] is None  # undefined, never 1.0 or 0.0


def test_pooled_metrics_excludes_empty_gt_from_mean_but_keeps_fp():
    imgs = [{"tp": 8, "fp": 2, "fn": 2}, {"tp": 0, "fp": 5, "fn": 0}]
    r = segmentation_metrics(imgs)
    assert r["n_empty_gt"] == 1 and r["pooled"]["fp"] == 7
    assert abs(r["mean_dice_nonempty_gt"] - 16 / 20) < 1e-9


def test_qwk_and_macro_f1():
    y = [0, 1, 2, 3, 4, 2, 1]
    assert M.qwk(y, y) == 1.0 and M.macro_f1(y, y) == 1.0
    assert M.qwk([0, 0, 4, 4], [4, 4, 0, 0]) < 0
    assert M.qwk([0, 0], [0, 0]) == 0.0  # undefined -> 0.0


def test_binary_report_and_bootstrap():
    r = M.binary_report([1, 1, 0, 0, 1], [1, 0, 0, 1, 1])
    assert (r["tp"], r["fn"], r["tn"], r["fp"]) == (2, 1, 1, 1)
    assert M.binary_report([0, 0], [0, 0])["sensitivity"] is None
    assert M.bootstrap_ci(lambda t, p: float((t == p).mean()), [1] * 5, [1] * 5) is None  # n<20
    ci = M.bootstrap_ci(lambda t, p: float((t == p).mean()), np.arange(100) % 2, np.arange(100) % 2, n_boot=50)
    assert ci == [1.0, 1.0]
