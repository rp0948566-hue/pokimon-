"""Export a trained candidate to ONNX (CANDIDATE file only; never written next to deployed weights)."""
from __future__ import annotations

import json
from pathlib import Path

import torch

from trainlab.utils.hashing import sha256_file

DEPLOYED_ROOT = Path("/home/claude/Desktop/Dird")
LAB_ROOT = Path(__file__).resolve().parents[2]  # the lab now lives inside DEPLOYED_ROOT; only the lab is writable


def export_onnx(model, example: torch.Tensor, out_path, opset: int = 17, dynamic_batch: bool = True) -> Path:
    out = Path(out_path).resolve()
    if DEPLOYED_ROOT in out.parents and LAB_ROOT not in out.parents:
        raise ValueError(f"refusing to write inside production repo: {out}")
    out.parent.mkdir(parents=True, exist_ok=True)
    model.eval().cpu()
    dyn = {"image": {0: "batch"}, "logits": {0: "batch"}} if dynamic_batch else None
    torch.onnx.export(model, example.cpu(), str(out), input_names=["image"], output_names=["logits"],
                      dynamic_axes=dyn, opset_version=opset, dynamo=False)
    return out


def write_export_bundle(out_dir, onnx_path, *, model_name: str, ckpt_path, preprocessing: dict, label_map: dict,
                        metrics: dict, parity: dict) -> None:
    out_dir = Path(out_dir)
    sha = sha256_file(onnx_path)
    files = {
        "preprocessing.json": preprocessing, "label_map.json": label_map, "metrics.json": metrics,
        "version.json": {"model": model_name, "onnx_sha256": sha, "checkpoint": str(ckpt_path),
                         "checkpoint_sha256": sha256_file(ckpt_path), "parity": parity, "promoted": False},
    }
    for name, obj in files.items():
        (out_dir / name).write_text(json.dumps(obj, indent=1))
    (out_dir / "model_card.md").write_text(
        f"# {model_name} (CANDIDATE, research only)\n\nNot the deployed RetinaCare model. Not clinically validated. "
        f"Not promoted; see reports/PROMOTION_PLAN.md.\n\n- onnx sha256: `{sha}`\n- parity max abs logit diff: "
        f"{parity.get('max_abs_logit_diff')}\n- metrics: see metrics.json\n")
    (out_dir / "integrity.sha256").write_text(
        "".join(f"{sha256_file(out_dir / n)}  {n}\n" for n in ["candidate.onnx", *files, "model_card.md"]))
