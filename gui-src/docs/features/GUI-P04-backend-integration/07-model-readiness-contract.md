# 07. Model Readiness Contract

## Readiness States
1. `NotConfigured`: No model path specified.
2. `ModelMissing`: Model manifest or checkpoint cannot be located on disk.
3. `Initializing`: Validating manifest, checking SHA-256, and compiling ONNX session.
4. `Ready`: Model is loaded in memory and ready for immediate inference.
5. `Processing`: Actively executing preprocessing or ONNX forward pass.
6. `Cancelling`: Worker cancellation requested.
7. `Unavailable`: Hardware or platform incompatibility (e.g. CUDA requested but unsupported).
8. `Failed`: Error during session creation or execution.

## Verification Policy
- Never show a green "Ready" state unless the session has been instantiated in `ort`.
- Never claim CUDA/GPU acceleration without verifying that `ort::cuda` initialized successfully.
- Distinctly show `CPU Execution Provider` to guarantee truth in medical software reporting.
- Pipeline stages beyond Stage I are reported as `Unavailable (Pending Backend Implementation)`:
  - Stage II Lesion Segmentation: `Unavailable`
  - Stage II Vessel Segmentation: `Unavailable`
  - Stage III DR Grading: `Unavailable`
  - Conversational LLM: `Unavailable`
