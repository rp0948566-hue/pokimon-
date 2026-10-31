"""Label maps and read-only adapters to the DEPLOYED RetinaCare models (baseline evaluation only)."""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

from trainlab.utils.hashing import sha256_file

BACKEND = Path("/home/claude/Desktop/Dird")
UPSTREAM_SRC = BACKEND / "ai_engine/.venv/hierarchiretina-src"  # MIT code of HierarchiRetina, read-only

GRADE_NAMES = ["No DR", "Mild", "Moderate", "Severe", "Proliferative"]
# Stage I manifest: positive route = DR grades 1..5 (any DR), negative = grade 0.
SCREEN_POSITIVE_MIN_GRADE = 1


def screening_label(grade) -> np.ndarray:
    """Reference binary screening label under the Stage I definition (grade >= 1)."""
    return (np.asarray(grade) >= SCREEN_POSITIVE_MIN_GRADE).astype(int)


def load_production_manifest(model_dir) -> dict:
    """Read manifest.json and verify the checkpoint hash on disk (read-only)."""
    mdir = Path(model_dir)
    man = json.loads((mdir / "manifest.json").read_text())
    ckpt = mdir / man["checkpoint"]
    actual = sha256_file(ckpt)
    if actual != man["sha256"]:
        raise RuntimeError(f"{ckpt}: sha256 {actual} != manifest {man['sha256']}")
    return man


class Stage1Baseline:
    """Deployed Stage I gate: upstream preprocess_gate -> ImageNet norm -> ONNX -> sigmoid >= threshold."""

    def __init__(self, model_dir=BACKEND / "models/stage1_convnextv2l_768", providers=None, lazy: bool = False):
        import onnxruntime as ort

        self.man = load_production_manifest(model_dir)
        self.threshold = float(self.man["outputs"]["gate_threshold"])
        self.size = int(self.man["input"]["shape"][-1])
        if str(UPSTREAM_SRC) not in sys.path:
            sys.path.insert(0, str(UPSTREAM_SRC))
        from hierarchiretina.stage1.preprocessing import preprocess_gate  # noqa: E402

        self._pre = preprocess_gate
        avail = ort.get_available_providers()
        providers = providers or [p for p in ("CUDAExecutionProvider", "CPUExecutionProvider") if p in avail]
        self._path, self._providers, self.providers, self.sess = Path(model_dir) / self.man["checkpoint"], providers, providers, None
        if not lazy:
            self._open()

    def _open(self):
        import onnxruntime as ort

        opts = ort.SessionOptions()
        opts.enable_cpu_mem_arena = False
        self.sess = ort.InferenceSession(str(self._path), sess_options=opts, providers=self._providers)
        self.providers = self.sess.get_providers()

    def release(self):
        self.sess = None

    def prepare(self, img_bgr: np.ndarray) -> np.ndarray:
        from trainlab.data.transforms import IMAGENET_MEAN, IMAGENET_STD

        rgb = self._pre(img_bgr, target_size=self.size)[:, :, ::-1].astype(np.float32) / 255.0
        return np.ascontiguousarray(((rgb - IMAGENET_MEAN) / IMAGENET_STD).transpose(2, 0, 1)[None])

    def predict(self, img_bgr: np.ndarray) -> dict:
        if self.sess is None:
            self._open()
        logit = float(self.sess.run(None, {"image": self.prepare(img_bgr)})[0].reshape(-1)[0])
        prob = float(1 / (1 + np.exp(-logit)))
        return {"logit": logit, "score": prob, "positive": prob >= self.threshold}
