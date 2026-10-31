# 09. Integration Test Results

## Test Suite Execution
- **Command**: `cargo test --bin retinacare --manifest-path rust/Cargo.toml`
- **Scope**: Native Rust unit tests for domain models, UI components, viewer quad-panel sync/pan/zoom, security filters, PDF extraction, chat state machine, and medical AI backend service adapter.

## Backend Integration Test Cases
1. `mock_service_submits_and_returns_stage1_result`:
   - Validates `RetinalAnalysisRequest` creation with UUID correlation ID.
   - Submits request to `MockMedicalInferenceService`.
   - Verifies `InferenceEvent::Stage1Completed` returns with matching `request_id`, `examination_id`, logit (1.295), screening score (0.785), threshold (0.2391716), and `ContinueCascade` routing decision.
   - Status: PASS.
2. `unavailable_service_rejects_submission`:
   - Verifies that when backend is `Unavailable`, requests are immediately rejected with `InferenceError::BackendNotReady`.
   - Status: PASS.
3. `unavailable_pipeline_stages_are_rejected`:
   - Verifies that requests for unintegrated Stage II / III models are rejected with `InferenceError::StageUnavailable`.
   - Status: PASS.
4. `mock_service_cancellation_emits_cancelled_event`:
   - Verifies prompt cancellation emits `InferenceEvent::Cancelled`.
   - Status: PASS.
5. `mock_service_failure_mode`:
   - Verifies that runtime failures cleanly emit `InferenceEvent::InferenceFailed` without crashing the application.
   - Status: PASS.
6. `discover_default_manifest_resolves_path`:
   - Verifies automatic multi-path discovery locates `/home/claude/Desktop/Dird/models/stage1_convnextv2l_768/manifest.json`.
   - Status: PASS.
