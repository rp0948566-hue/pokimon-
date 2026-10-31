# 11. File Change Report (Task 04)

## Summary of Changes
Task 04 establishes the typed native Rust integration boundary connecting the RetinaCare desktop GUI to Codex's verified medical AI backend.

## Modified & Created Files

### 1. `rust/Cargo.toml`
- **Type**: Dependency Manifest
- **Changes**: Added path dependencies to Codex backend crates:
  ```toml
  retinacare-ai-bridge = { path = "../../Dird/native/crates/retinacare-ai-bridge" }
  retinacare-app = { path = "../../Dird/native/crates/retinacare-app" }
  serde_json = "1.0"
  ```
- **Rationale**: Links verified ONNX Runtime session, OpenCV CLAHE preprocessing, and background `ModelService` without duplicate crates or HTTP network overhead.

### 2. `rust/src/engine/backend.rs`
- **Type**: New Integration Module (590 lines)
- **Changes**:
  - Implemented `MedicalInferenceService` trait.
  - Implemented `BackendReadiness` state machine.
  - Implemented `RetinalAnalysisRequest` with UUID correlation and examination isolation.
  - Implemented `Stage1ScreeningResult` capturing raw logit, sigmoid probability, threshold (0.2391716), elapsed time, and non-diagnostic caveat.
  - Implemented `LiveCodexBackendService` wrapping `retinacare_app::models::ModelService` with mutex synchronization.
  - Implemented `MockMedicalInferenceService` for fast deterministic testing.
  - Implemented multi-path manifest discovery utility `discover_default_manifest()`.

### 3. `rust/src/engine/mod.rs`
- **Type**: Engine Module Root
- **Changes**: Re-exported `backend` module types.

### 4. `rust/src/ui/viewer.rs`
- **Type**: Retinal Quad-Panel UI Component
- **Changes**:
  - Extended `ViewerAction` with `RunStage1Inference`, `CancelInference`, and `DismissResult`.
  - Added backend readiness status pill to toolbar.
  - Added `[▶ Run Stage I Screening]` and `[✕ Cancel Screening]` buttons to toolbar.
  - Implemented `render_stage1_result_card()` displaying authentic decision badges, scores, logits, latency, and caveat.
  - Implemented `render_stage1_error_card()` for clean error handling.

### 5. `rust/src/main.rs`
- **Type**: Application Root
- **Changes**:
  - Added `backend_service`, `backend_readiness`, `active_screening_request`, `stage1_result`, and `stage1_error` fields to `RetinaCareApp`.
  - Added nonblocking event loop draining `backend_service.try_recv()` every frame.
  - Added action handlers for Stage 1 inference submission, cancellation, and dismissal.
  - Enforced examination and session isolation on image load and consultation switch.
  - Added 6 new unit tests for backend integration (81 total tests passing).

### 6. Documentation Suite (`docs/features/GUI-P04-backend-integration/`)
- Created 15 detailed documentation files covering architecture, compatibility, contracts, and handoff.
