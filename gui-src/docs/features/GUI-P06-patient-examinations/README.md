# GUI Phase 1 • Task 06: Patient & Examination Management

## 1. Executive Summary

Task 06 implements the native Rust desktop interface for Patient and Examination Management in RetinaCare. It bridges the screening workflow between the initial home composer, previous chats, and the quad-panel/clinical report views.

```
Directory (Patients) -> Patient Detail -> Examination History -> Quad Viewer / Report Overview
```

## 2. Technology & Architecture Guardrails

- **Framework**: 100% Native Rust application using `egui` and `eframe` 0.37.
- **Prohibitions**: Zero HTML, CSS, JavaScript, TypeScript, React, Electron, Tauri, or WebView.
- **Durable Storage Integrity**: Live SQLCipher encrypted storage is owned and governed by Codex backend crates (`retinacare-persistence` and `retinacare-app`). Antigravity implements typed frontend service adapters and state machines without inventing an ad-hoc unencrypted database.
- **Patient Isolation**: Strict separation of Patient ID, Examination ID, Image Artifact ID, and Inference Request ID. Switching patients invalidates pending asynchronous requests and purges prior patient context to eliminate cross-patient data leakage.
- **Truthful Status Reporting**: Because SQLCipher database unlock and full persistence APIs are not initialized in default production desktop sessions without clinician credentials, the GUI truthfully communicates the locked/unavailable database status. Synthetic test fixtures are strictly restricted to `#[cfg(test)]` and isolated developer demonstration mode (`RETINACARE_DEV_FIXTURES=1`).

## 3. Directory Structure

- `01-current-state-audit.md`: Audit of existing types in GUI and Codex backend crates.
- `02-clinical-workflow-research.md`: Clinical workflow analysis from reception to screening.
- `03-requirements.md`: Formal functional, privacy, and UX requirements.
- `04-implementation-plan.md`: Phased execution roadmap.
- `05-data-contracts.md`: Typed Rust data models and service traits.
- `06-privacy-and-isolation.md`: Patient isolation, credential security, and session sanitization.
- `07-view-state-machines.md`: View states for directory, detail, and examination screens.
- `08-test-plan.md`: Unit and integration test coverage specification.
- `09-test-results.md`: Complete automated test execution results.
- `10-desktop-verification.md`: Native desktop screenshots under Linux X11.
- `11-file-change-report.md`: Quantitative and qualitative file change breakdown.
- `12-known-blockers.md`: Backend database and API gaps.
- `13-codex-api-handoff.md`: Exact Rust signatures and schema required from Codex.
- `14-task07-handoff.md`: Handoff prerequisites for Task 07.
