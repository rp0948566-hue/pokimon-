# 08. Testing Strategy

## Test Matrix
1. **Unit Tests (`rust/src/engine/backend.rs`)**:
   - `MockMedicalInferenceService` handles state transitions deterministically.
   - `RetinalAnalysisRequest` construction and UUID validation.
   - Rejection of invalid image paths.
   - Matching of `request_id` and `examination_id`.
   - Stale event rejection when `examination_id` mismatches.
   - Cancellation transitions.
   - Bounded channel queue capacity handling (`QueueFull`).
2. **Integration Tests (`rust/tests/integration_tests.rs`)**:
   - Manifest parsing against `/home/claude/Desktop/Dird/models/stage1_convnextv2l_768/manifest.json`.
   - Verification of model ID and SHA-256 integrity.
   - Nonblocking `ModelService` request submission and event receiving.
   - Verification of Stage I routing decision decoder logic (`STAGE1_GATE_THRESHOLD = 0.2391716`).
3. **GUI Component & State Tests (`rust/src/main.rs`)**:
   - Toolbar reflects `BackendReadiness`.
   - Screening button disabled when backend is `Processing` or `NotConfigured`.
   - Honest display of non-diagnostic disclaimer.
   - Regression prevention: Home page, AI chat, viewer quad-panel pan/zoom remain intact.
