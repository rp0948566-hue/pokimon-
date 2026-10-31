# 03 - Feature Requirements: GUI Phase 1 • Task 05

## 1. Functional Requirements

### FR-01: Report Overview Layout
- The Report screen must present a clear, scannable clinical summary:
  - Patient Code & Examination Identifier.
  - Eye Laterality with clear visual badge (`OD` / `OS`).
  - Original fundus preview thumbnail.
  - Stage I Screening Result Card (Score, Logit, Threshold, Route Recommendation, Provenance, Latency).
  - Pipeline Stage Availability matrix (Stage I available; Stages II & III unavailable).
  - Prominent non-diagnostic research caveat.

### FR-02: Local PDF Export Workflow
- An `[📄 Export Clinical PDF]` button triggers the export workflow.
- Native save dialog (`rfd::FileDialog`) prompts the user for destination path.
- When backend PDF service is unavailable:
  - Truthfully display `BackendUnavailable` state with detailed technical rationale.
  - Present the exact Codex API requirements without claiming false success.
- When backend PDF service is present:
  - Generate PDF on worker thread without blocking GUI frames.
  - Verify file creation, file size, and PDF signature before indicating success.

### FR-03: Patient Session Export (Scoped Dossier)
- An `[📦 Export Session Dossier]` button opens an explicit scope confirmation dialog.
- Scoped strictly to the selected examination (no cross-patient contamination).
- User options:
  - Include original fundus image file.
  - Include genuine Stage I screening findings JSON.
  - Include model provenance metadata.
- Clear encryption status notice: *"Unencrypted local JSON export - store securely in compliance with institutional data governance."*

### FR-04: Audit Ledger Presentation
- An `[📋 Audit Ledger]` button toggles a sliding or drawer view of recorded events.
- Displays: Event Timestamp, Event Kind, Examination ID, Request ID, Status, and Non-Sensitive Details.
- When backend audit persistence is absent:
  - Displays `"Audit history unavailable from backend"` banner.
  - Outlines minimal required SQLCipher persistence API for Codex.

## 2. Non-Functional & Security Requirements
- **NFR-01 (Native Rust)**: 100% native Rust (`egui`/`eframe`). Zero web runtimes.
- **NFR-02 (Nonblocking UI)**: All file I/O and generation operations run asynchronously over nonblocking channels.
- **NFR-03 (Privacy & Isolation)**: No network telemetry, no automatic cloud transfers, and strict examination boundary isolation.
