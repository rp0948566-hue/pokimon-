# 04. Implementation Plan

## Step 1: Typed Integration Layer (`rust/src/engine/backend.rs`)
- Define `MedicalInferenceService` trait with `submit(&self, request: RetinalAnalysisRequest) -> Result<(), InferenceError>` and `try_recv(&self) -> Option<InferenceEvent>`.
- Implement `CodexBackendService`:
  - Wraps `retinacare_app::models::ModelService`.
  - Discovers manifest in `/home/claude/Desktop/Dird/models/stage1_convnextv2l_768/`.
  - Reads model manifest to extract verified metadata.
  - Submits nonblocking `InferStage1Image` requests to the worker thread.
  - Translates `ModelEvent::Stage1Inference` into strongly typed `InferenceEvent` with request tracking.
- Implement `MockMedicalInferenceService` for fast deterministic unit tests in CI environments without ONNX Runtime.

## Step 2: GUI State and Event Machine (`rust/src/main.rs`)
- Add `backend_service: Arc<dyn MedicalInferenceService + Send + Sync>` to `RetinaCareApp`.
- Maintain `backend_readiness: BackendReadiness` state in GUI.
- Poll `backend_service.try_recv()` at the start of each egui frame.
- Add "Run Stage I Screening" button to Report page toolbar when an image is selected and backend is `Ready`.
- Add active processing spinner, elapsed timer, and Cancel button.
- Display `Stage1ScreeningResult` panel with authentic model metrics, routing decision, threshold, and non-diagnostic research caveat.
- Keep Stage II / Stage III panels in truthful `ArtifactUnavailable` state.

## Step 3: Automated Testing & Verification
- Test readiness detection.
- Test request submission with ID validation.
- Test stale result rejection on examination switch.
- Test cancellation.
- Run `cargo fmt`, `cargo clippy`, and `cargo test`.
- Launch application on desktop and capture 6 verified screenshots.
