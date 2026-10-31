"""PyTorch vs ONNX Runtime numerical parity on supplied inputs."""
from __future__ import annotations

import numpy as np
import torch


def parity_check(model, onnx_path, inputs: list[np.ndarray], tol: float = 1e-3) -> dict:
    import onnxruntime as ort

    sess = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])
    model.eval().cpu()
    diffs = []
    for x in inputs:
        with torch.no_grad():
            ref = model(torch.from_numpy(x)).numpy()
        got = sess.run(None, {"image": x})[0]
        diffs.append(float(np.abs(ref - got).max()))
    return {"n_inputs": len(inputs), "max_abs_logit_diff": max(diffs), "per_input": diffs, "tolerance": tol,
            "passed": max(diffs) <= tol, "provider": "CPUExecutionProvider"}
