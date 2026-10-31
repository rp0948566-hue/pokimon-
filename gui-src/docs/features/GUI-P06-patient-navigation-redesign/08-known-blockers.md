# Phase 1 • Task 06.1: Known Blockers & Backend Deficiencies

## 1. Executive Status: PARTIAL

While the native Rust GUI implementation, navigation hierarchy, subrouting, and desktop presentation are 100% complete, verified, and free of defects, this task is designated as **PARTIAL** in strict adherence to RetinaCare clinical integrity policies due to missing upstream Codex backend crates.

---

## 2. Unimplemented Codex Backend Components

### A. PDF Generation Service (`retinacare-reports`)
- **Current GUI State**: Screen routes cleanly to `ReportSubroute::PdfExport`. Displays an amber informational banner and the required engine interface contract. Export button is safely disabled.
- **Backend Blocker**: Codex backend has not yet implemented the `retinacare-reports` crate or any native Rust PDF compilation pipeline (e.g., `printpdf`, `genpdf`, or headless rendering).
- **Integrity Rule**: The GUI will never fabricate a dummy PDF file or present a synthetic success toast without genuine binary compilation by the backend.

### B. Durable Audit Persistence Ledger (`retinacare-persistence`)
- **Current GUI State**: Screen routes cleanly to `ReportSubroute::AuditHistory`. Displays an amber informational banner, lists in-memory session events (or dev test fixtures when enabled), and details the required SQLite schema.
- **Backend Blocker**: Codex backend SQLCipher schema lacks an `audit_events` or `export_audit_ledger` table with SHA-256 HMAC tamper evidence.
- **Integrity Rule**: The GUI will never claim durable audit trail persistence until an authenticated database table acknowledges write commits.

### C. Stage II / III Advanced Clinical AI Models
- **Current GUI State**: Screens display verified Stage I ConvNeXt-V2-Large gating metrics (referral score, image quality, confidence). Stage II (lesion segmentation) and Stage III (calibrated DR severity grading) are explicitly marked with `UNAVAILABLE` badges.
- **Backend Blocker**: Only Stage I binary gate model is currently converted to ONNX and verified in `retinacare-core`.
- **Integrity Rule**: No speculative DR Grade 0-4 or fake lesion segmentations are generated or displayed.
