import numpy as np
import pandas as pd
from PIL import Image

from trainlab.evaluate import cascade_eval as C


class FakeCascade:  # stands in for the deployed models: tests the metric plumbing only
    def run(self, img):
        return {"gate_score": 0.9, "gate_positive": True, "stage3_run": True, "ungradable": False, "grade_pred": 2}


def test_cascade_grading_counts_and_stage_a(tmp_path):
    for i in range(3):
        Image.new("RGB", (80, 80), (50 + i, 0, 0)).save(tmp_path / f"{i}.png")
    df = pd.DataFrame({"image_id": list("abc"), "image_path": [str(tmp_path / f"{i}.png") for i in range(3)], "label": [2, 0, 3]})
    pred, m = C.cascade_grading(df, FakeCascade())
    assert m["stage_B_all_images"]["n"] == 3 and m["stage_A_reached_stage3"]["n"] == 2 and m["n_stage3_run"] == 3


def test_counts_threshold_semantics():
    p = np.zeros((4, 4), np.uint8); p[:2] = 255
    g = np.zeros((4, 4), np.uint8); g[:, :2] = 1
    assert C._counts(p, g) == {"tp": 4, "fp": 4, "fn": 4}
