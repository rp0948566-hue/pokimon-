"""Frozen DEPLOYED cascade (Stage I gate -> Stage II masks -> Stage III 5-fold LG-DRG) via ONNX Runtime, read-only.

Pre/post-processing is imported from the upstream HierarchiRetina code (MIT, ai_engine/.venv/hierarchiretina-src) so the
inputs match the export-parity scripts. Single forward pass (no TTA), matching the single-input ONNX graphs.
Protocol follows upstream cascade.py: gate-blocked images are scored as Grade 0; Stage III decode uses the paper's fixed
CORN thresholds from the fold manifests; predicted Grade 5 (ungradable) is reported separately and mapped to 4 for QWK.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import cv2
import numpy as np

from trainlab.models.adapters import BACKEND, UPSTREAM_SRC, Stage1Baseline, load_production_manifest

if str(UPSTREAM_SRC) not in sys.path:
    sys.path.insert(0, str(UPSTREAM_SRC))

M = BACKEND / "models"
LESIONS = ("MA", "HE", "EX", "CWS")


class LazySession:
    """Creates the ORT session on first use and drops it on release(): 10 deployed graphs (~8 GB) do not fit in RAM together."""

    def __init__(self, path: Path, providers):
        self.path, self.providers, self._s = path, providers, None

    def run(self, names, feed):
        if self._s is None:
            import gc

            import onnxruntime as ort

            opts = ort.SessionOptions()
            opts.enable_cpu_mem_arena = False
            gc.collect()
            self._s = ort.InferenceSession(str(self.path), sess_options=opts, providers=self.providers)
        return self._s.run(names, feed)

    def release(self):
        self._s = None


def _sess(path: Path, providers):
    return LazySession(path, providers)


class DeployedCascade:
    def __init__(self, providers=None, folds=range(5)):
        import onnxruntime as ort

        avail = ort.get_available_providers()
        self.providers = providers or [p for p in ("CUDAExecutionProvider", "CPUExecutionProvider") if p in avail]
        self.stage1 = Stage1Baseline(providers=self.providers, lazy=True)
        self.lesion = {}
        for lz in LESIONS:
            d = M / "stage2_hsmoe_aunet" / lz.lower()
            man = load_production_manifest(d)
            self.lesion[lz] = (_sess(d / man["checkpoint"], self.providers), float(man["validation"]["deployed_probability_threshold"]), man)
        vd = M / "stage2_swinhrunetpp"
        self.vessel_man = load_production_manifest(vd)
        self.vessel_sess = _sess(vd / self.vessel_man["checkpoint"], self.providers)
        self.vessel_thr = float(self.vessel_man["validation"]["deployed_probability_threshold"])
        self.folds = []
        self.corn_t, self.grad_t = None, None
        for k in folds:
            man = load_production_manifest_named(M / "stage3_lgdrg", f"manifest_fold{k}.json")
            self.folds.append(_sess(M / "stage3_lgdrg" / man["checkpoint"], self.providers))
            self.corn_t, self.grad_t = tuple(man["validation"]["corn_thresholds"]), float(man["validation"]["gradability_threshold"])

    # -- Stage II -------------------------------------------------------------------------------
    def lesion_mask(self, lz: str, img_bgr: np.ndarray) -> np.ndarray:
        from hierarchiretina.stage2.hsmoe_aunet import get_lesion_config
        from hierarchiretina.stage2.lesion_inference import postprocess_to_full_mask, preprocess_for_deployment

        sess, thr, _ = self.lesion[lz]
        cfg = get_lesion_config(lz)
        t, meta = preprocess_for_deployment(img_bgr, cfg)
        logits = sess.run(None, {"image": t.numpy()})[0][0, 0]
        sess.release()
        prob = 1.0 / (1.0 + np.exp(-logits.astype(np.float32)))
        return postprocess_to_full_mask(prob, meta, cfg, thr)  # uint8 0/255, original resolution

    def vessel_mask(self, img_bgr: np.ndarray) -> np.ndarray:
        from hierarchiretina.stage2.vessel_inference import (MIN_COMPONENT_AREA, MORPH_OPEN_RADIUS, generate_fov_mask, postprocess_mask,
                                                              preprocess_for_inference, to_original_resolution)

        t, meta = preprocess_for_inference(img_bgr, 512)
        logits = self.vessel_sess.run(None, {"image": t.numpy()})[0][0, 0]
        self.vessel_sess.release()
        prob = to_original_resolution(1.0 / (1.0 + np.exp(-logits.astype(np.float32))), meta)
        fov = generate_fov_mask(img_bgr, is_rgb=True).astype(np.float32)
        return (postprocess_mask(prob, self.vessel_thr, MORPH_OPEN_RADIUS, MIN_COMPONENT_AREA, fov=fov) > 0).astype(np.uint8) * 255

    # -- Stage III ------------------------------------------------------------------------------
    def stage3_full5(self, img_bgr: np.ndarray, masks: dict[str, np.ndarray]) -> np.ndarray:
        """Mean of the per-fold 5-way vectors (grades 1..5) for one image."""
        import torch

        from hierarchiretina.stage3.corn import lgdrg_predict
        from hierarchiretina.stage3.preprocessing import build_aug, clahe_lab, crop_fundus_circle

        im = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2RGB)
        im, box = crop_fundus_circle(im)
        im = clahe_lab(im)
        ms = []
        for name in ("MA", "HE", "EX", "CWS", "VESSEL"):
            m = masks[name]
            if box is not None:
                r0, r1, c0, c1 = box
                m = m[r0:r1 + 1, c0:c1 + 1]
            ms.append((m > 127).astype(np.float32))
        aug = build_aug(512, 5, train=False)(image=im, **{f"mask{i}": m for i, m in enumerate(ms)})
        x = torch.cat([aug["image"], torch.stack([aug[f"mask{i}"].float() for i in range(5)])], 0)[None].numpy()
        fulls = []
        for sess in self.folds:
            out = sess.run(None, {"image": x})[0]
            sess.release()
            _, full = lgdrg_predict(torch.from_numpy(out[:, :3]), torch.from_numpy(out[:, 3]))
            fulls.append(full.numpy()[0])
        return np.mean(fulls, 0)

    def decode(self, full5: np.ndarray) -> int:
        """Paper decode -> grade 1..5 (5 = ungradable)."""
        from hierarchiretina.stage3.corn import corn_from_full, decode_consecutive

        p1, p2, p3, pg = corn_from_full(full5[None])
        return int(decode_consecutive(p1, p2, p3, pg, self.corn_t, self.grad_t)[0])

    # -- full cascade ---------------------------------------------------------------------------
    def run(self, img_bgr: np.ndarray, force_stage3: bool = False) -> dict:
        g = self.stage1.predict(img_bgr)
        self.stage1.release()
        res = {"gate_score": g["score"], "gate_positive": bool(g["positive"]), "stage3_run": False, "grade_pred": 0, "ungradable": False}
        if not (g["positive"] or force_stage3):
            return res
        masks = {lz: self.lesion_mask(lz, img_bgr) for lz in LESIONS}
        masks["VESSEL"] = self.vessel_mask(img_bgr)
        full5 = self.stage3_full5(img_bgr, masks)
        d = self.decode(full5)
        res.update({"stage3_run": True, "grade_decoded": d, "ungradable": d == 5, "grade_pred": min(d - 1, 4),
                    "full5": [float(v) for v in full5], "mask_px": {k: int((v > 127).sum()) for k, v in masks.items()}})
        return res


def load_production_manifest_named(model_dir: Path, name: str) -> dict:
    """Stage III keeps one manifest per fold (manifest_foldN.json); verify the ONNX hash like the single-manifest loader."""
    from trainlab.utils.hashing import sha256_file

    man = json.loads((model_dir / name).read_text())
    actual = sha256_file(model_dir / man["checkpoint"])
    if actual != man["sha256"]:
        raise RuntimeError(f"{man['checkpoint']}: sha256 mismatch")
    return man
