# 13. Codex Backend API Requirements & Handoff

## Purpose
This document provides Freebuff (Codex) with the exact, verified technical requirements, interfaces, and minimal modifications needed to finalize and extend the RetinaCare medical AI backend.

## 1. Verified Working Backend Capabilities
- `retinacare-ai-bridge::model_manifest::verify_manifest` correctly validates model JSON schema and SHA-256 (`9ad51e1f...`).
- `retinacare-ai-bridge::onnx_session::VerifiedSession` successfully binds to ONNX Runtime via `ort = "2.0.0-rc.15"`.
- `retinacare-ai-bridge::stage1::decode_stage1_gate` correctly decodes raw logit into sigmoid probability and routes based on `STAGE1_GATE_THRESHOLD = 0.2391716`.
- `retinacare-app::models::ModelService` provides nonblocking background thread execution over `mpsc` channels (`InferStage1Image`, `ModelEvent`).

## 2. Immediate Backend Bug To Fix (Blocking Backend Unit Tests)
- **Location**: `/home/claude/Desktop/Dird/native/crates/retinacare-ai-bridge/src/preprocessing.rs:450:37`
- **Issue**:
  ```rust
  imgcodecs::imwrite(&directory.join(format!("{name}.png")).to_string_lossy(), stage, &Vector::new()).unwrap();
  ```
  In Rust, `&to_string_lossy()` produces `&Cow<'_, str>`, which does not satisfy OpenCV's `impl AsRef<OsStr>` trait bound.
- **Required Minimal Fix**:
  ```rust
  imgcodecs::imwrite(&directory.join(format!("{name}.png")), stage, &Vector::new()).unwrap();
  // OR
  imgcodecs::imwrite(&*directory.join(format!("{name}.png")).to_string_lossy(), stage, &Vector::new()).unwrap();
  ```
  `PathBuf` already implements `AsRef<OsStr>`. Passing the `PathBuf` directly resolves the compilation error immediately.

## 3. Required Public API Extensions for Phase 2
1. **Stage II Lesion & Vessel Segmentation**:
   - Provide `infer_stage2_lesions(&mut self, model_id: &str, image_path: &Path) -> Result<LesionMaskArtifact, Stage2Error>`.
   - Output typed bitmask or polygon coordinates matching `[1, 1, H, W]` tensor layout.
   - Return registered coordinates aligned with original fundus image dimensions.
2. **Stage III 5-Class Diabetic Retinopathy Grading**:
   - Provide `infer_stage3_grading(&mut self, model_id: &str, image_path: &Path, masks: Option<&LesionMaskArtifact>) -> Result<DrGradingResult, Stage3Error>`.
   - Return discrete classes 0–4 (No DR, Mild NPDR, Moderate NPDR, Severe NPDR, PDR) with softmax probabilities.
3. **Cancellation Semantics**:
   - Support fine-grained cancellation tokens for in-flight preprocessing and inference kernels so background worker threads can abort before long loops complete.
4. **Hardware Provider Selection**:
   - Enable runtime fallback from CUDA to CPU if TensorRT/CUDA fails to allocate device memory.
