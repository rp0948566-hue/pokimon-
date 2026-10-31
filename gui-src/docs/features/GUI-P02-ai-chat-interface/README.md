# GUI Phase 1 • Task 02 — Complete the AI Chat Interface

## 1. Overview
This feature completes the conversational AI interface in RetinaCare. Building upon the Home composer created in Task 01, Task 02 implements full conversation rendering, user and assistant message differentiation, image/PDF attachment tiles in messages, nonblocking streaming event accumulation, status indicators (Queued, Streaming, Completed, Cancelled, Failed), message action controls (Copy, Retry, Stop), smart auto-scrolling with user-position preservation, and typed backend contract boundaries.

## 2. Team Boundaries
- **Antigravity Ownership (Frontend GUI)**:
  - 100% Rust (`egui`/`eframe`) desktop rendering under `rust/src/`.
  - Message rendering, scroll mechanics, clipboard copying, streaming UI integration, state machine.
- **Codex / Freebuff Ownership (Backend & AI Engine)**:
  - `native/crates/`, `ai_engine/`, PyTorch-to-ONNX models, CUDA inference, SQLCipher.
  - Zero backend code modified or duplicated by Antigravity.

## 3. Key Standards
- **Zero Hallucination / Fabrication**: No simulated AI answers, fake lesion counts, or fabricated DR severity grades.
- **Truthful Status**: Unconnected AI engine clearly reports "AI service is not connected." with retry capability.
- **Zero Web Technologies**: 100% Rust desktop binary; zero HTML, CSS, JS, Electron, or WebView.
