# 03. Integration Requirements

## Core Architectural Requirements
1. **Zero GUI Freezing**:
   - Model discovery, manifest parsing, ONNX session loading, image preprocessing, and inference must execute strictly off the egui rendering thread.
   - UI thread interacts only via nonblocking channels (`try_send`, `try_recv`).
2. **Stable Typed Identity**:
   - Every inference request must carry a stable UUID/correlation ID (`InferenceRequestId`).
   - Every response or event must validate matching `request_id` and `examination_id`.
   - Stale events arriving after an examination switch or image replacement must be dropped silently.
3. **Truthful Medical AI State**:
   - `BackendReadiness`: Reflects genuine backend capability (`NotConfigured`, `ModelMissing`, `Initializing`, `Ready(Provider)`, `Processing`, `Failed`).
   - Stage I status: Available only when `manifest.json` and `stage1.onnx` are valid and loaded.
   - Stage II / Stage III status: Truthfully reported as `Unavailable` until real backend models exist.
   - Chat separation: Conversational LLM remains separated from vision screening gate.
4. **Authentic Provenance Display**:
   - The UI must display genuine model metadata from `manifest.json`:
     - Model ID: `hierarchiretina-stage1-convnextv2l-768`
     - Architecture: `convnextv2_large+gem+attn_pool+mlp_head`
     - Threshold: `0.2391716`
     - Actual execution provider: CPU (or verified CUDA)
     - Raw logit and Sigmoid screening score
     - Gate decision: `NegativeRoute` (Grade 0) or `ContinueCascade` (Grades 1-5 / ungradable)
     - Non-diagnostic research caveat: "Research prototype only. Not certified for clinical diagnosis."
5. **Patient Privacy & Safety**:
   - Fundus images remain strictly on local disk; zero remote network transmission.
   - UI never discloses sensitive system paths to users.
