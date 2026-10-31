# 01. GUI and Backend Repository Audit

## Audit Date
2026-10-28

## 1. GUI Repository Audit (`/home/claude/Desktop/Hackthon dezine`)
- **Git Branch**: `dezine`
- **Application Structure**:
  - `rust/Cargo.toml`: Package `retinacare-desktop` v0.1.0, 100% native Rust desktop binary.
  - `rust/src/main.rs`: `RetinaCareApp` implementing `eframe::App`. Contains Home page, Chat interface, Viewer workspace, Report generation, and Plugins deck.
  - `rust/src/domain.rs`: Local UI domain models (`PatientId`, `ExaminationId`, `EyeLaterality`, `ViewerPanelId`, `ArtifactLoadStatus`, etc.).
  - `rust/src/ui/`:
    - `viewer.rs`: Quad-panel retinal workspace with pan/zoom synchronization, coordinate inspection, lesion/vessel/Grad-CAM overlays, and truthful artifact unavailability states.
    - `composer.rs`: Central prompt composer with attachments and model selector.
    - `components.rs`: Status pills, buttons, cards, and modal elements.
- **Frontend Capabilities**:
  - Task 01: Home page redesign, dark minimal aesthetic, nonblocking state.
  - Task 02: AI Chat interface, streaming simulation, message bubbles, retry/copy/stop.
  - Task 03: Four-panel retinal workspace with pan/zoom, coordinate tracking, independent/synchronized modes.

## 2. Medical Backend Repository Audit (`/home/claude/Desktop/Dird`)
- **Git Branch**: `main`
- **Crate Architecture (`native/crates/`)**:
  - `retinacare-ai-bridge`:
    - `model_manifest.rs`: Schema validation and SHA-256 verification for model artifacts.
    - `onnx_session.rs`: Live ONNX session runtime via `ort 2.0.0-rc.15` (pinned).
    - `preprocessing.rs`: CLAHE, Ben Graham color normalization, and crop preprocessing via `opencv 0.101.1`.
    - `stage1.rs`: Stage I binary screening gate decoding (`STAGE1_GATE_THRESHOLD = 0.2391716`, `NegativeRoute` vs `ContinueCascade`).
    - `model_runtime.rs`: `ModelRuntime` managing loaded ONNX sessions, `infer_stage1_image(model_id, image_path)`.
  - `retinacare-app`:
    - `models.rs`: `ModelService` nonblocking background worker thread communicating over `mpsc` channels (`InferStage1Image`, `ModelEvent`).
    - `database.rs`: SQLCipher database controller for patients and examinations.
  - `retinacare-domain`: Patient, Examination, and Annotation domain models.
  - `retinacare-persistence`: SQLCipher-encrypted SQLite store.
- **Model Checkpoints (`models/stage1_convnextv2l_768/`)**:
  - `manifest.json`: Model ID `hierarchiretina-stage1-convnextv2l-768`, version `1.0.0`, input shape `[1, 3, 768, 768]`, threshold `0.2391716`.
  - `stage1.onnx`: 807,176,466 bytes, SHA-256: `9ad51e1f428dc59ab49f1f49e8f68a84cd23d8fd4c2c461b45b06bcc726880ac`.
- **Status of Missing Pipelines**:
  - Stage II lesion segmentation: No model checkpoint or decoder present in backend.
  - Stage II vessel segmentation: No model checkpoint or decoder present in backend.
  - Stage III DR grading: No model checkpoint or 5-class decoder present in backend.
  - Conversational LLM: No chat language model present in backend.
