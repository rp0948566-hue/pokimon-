#!/usr/bin/env python
"""P7: export a trained CANDIDATE (grading) to ONNX, verify PyTorch-vs-ONNX Runtime parity, write the export bundle.

Output goes to exports/<run>/ inside the lab only. Never writes to the deployed models directory and
never promotes. Parity inputs are real prepared images from the val manifest (not random noise) when available.
"""
import argparse
import json
import sys
from pathlib import Path

import numpy as np

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

import pandas as pd  # noqa: E402
import torch  # noqa: E402
from PIL import Image  # noqa: E402

from trainlab.data import transforms as T  # noqa: E402
from trainlab.export.onnx_export import export_onnx, write_export_bundle  # noqa: E402
from trainlab.export.parity import parity_check  # noqa: E402
from trainlab.models.registry import build_grading_model  # noqa: E402
from trainlab.utils.hashing import sha256_file  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--run", required=True, help="grading run directory name under experiments/grading")
    ap.add_argument("--val-manifest", default=str(LAB / "data/manifests/val/aptos_val.csv"))
    ap.add_argument("--n-parity", type=int, default=5, help="real val images per class used for parity (max)")
    a = ap.parse_args()
    run = LAB / "experiments/grading" / a.run
    info = json.loads((run / "run_info.json").read_text())
    if info["kind"] != "train":
        sys.exit("only real training runs can be exported (kind != train)")
    ck = info["best_ckpt"]
    assert sha256_file(ck["path"]) == ck["sha256"], "checkpoint hash mismatch"
    size = info["meta"]["image_size"]
    model = build_grading_model(info["meta"]["architecture"], 5, pretrained=False)
    model.load_state_dict(torch.load(ck["path"], map_location="cpu", weights_only=False)["model"])
    out = LAB / "exports" / a.run
    out.mkdir(parents=True, exist_ok=False)
    onnx_path = export_onnx(model, torch.zeros(1, 3, size, size), out / "candidate.onnx")
    df = pd.read_csv(a.val_manifest)
    pick = df.groupby("label").head(a.n_parity)
    xs = [T.to_tensor_array(np.asarray(Image.open(p).convert("RGB").resize((size, size))))[None] for p in pick["prepared_path"]]
    par = parity_check(model, onnx_path, xs)
    par["input_kind"] = f"{len(xs)} real val images (classes {sorted(pick['label'].unique().tolist())})"
    write_export_bundle(out, onnx_path, model_name=info["meta"]["architecture"], ckpt_path=ck["path"],
                        preprocessing={"size": size, "crop": "fundus black-border crop + letterbox", "normalization": "ImageNet mean/std", "color": "RGB"},
                        label_map={"0": "No DR", "1": "Mild", "2": "Moderate", "3": "Severe", "4": "Proliferative"},
                        metrics={"val_best": info.get(f"best_{info['meta'].get('monitor', 'val_qwk')}"), "run_info": str(run / "run_info.json")}, parity=par)
    print(f"exported {onnx_path}\nparity: max_abs_logit_diff={par['max_abs_logit_diff']:.2e} passed={par['passed']}")
    return 0 if par["passed"] else 3


if __name__ == "__main__":
    sys.exit(main())
