# RetinaCare GUI Phase 1 • Task 05: Clinical Report UI, PDF Export, Session Export & Audit Ledger

## Overview
This feature implements the Clinical Report Workspace, Local PDF Export Workflow, Patient Session Export, and Audit Ledger Viewer for RetinaCare within native Rust (`egui`/`eframe`).

## Architecture & Ownership Boundaries
- **Frontend (Antigravity)**: Owns `rust/src/` GUI code, report rendering, local export dialogs, typed service adapters, and audit viewers.
- **Backend (Codex / Freebuff)**: Owns `/home/claude/Desktop/Dird/native/crates/` (read-only to Antigravity).
- **Core Principle**: Medical truthfulness above all. Never fabricate DR grades (Grade 0-4), lesion masks, or simulated audit logs. Show genuine Stage I ConvNeXt-V2 screening results and truthfully display unavailable states for missing backend services.

## Audit Findings Summary
1. **Stage I Screening**: Active & connected via `retinacare-app` & `retinacare-ai-bridge` using `models/stage1_convnextv2l_768/manifest.json`.
2. **Stage II & III Models**: Unavailable (weights not packaged by Codex).
3. **Backend PDF Generation**: `retinacare-reports` provides only a basic `ReportData` data struct without a rendering engine (`generate_pdf`). Truthful `BackendUnavailable` state is displayed in GUI with exact API specifications provided for Codex.
4. **Backend Audit Persistence**: `retinacare-persistence` (`SqlCipherStore`) does not expose an audit ledger table or query API. Truthful `AuditHistoryUnavailable` state is displayed in GUI.

## Documentation Index
- `01-audit.md`: Detailed audit of existing GUI and Codex backend crates.
- `02-research.md`: Medical report design standards and non-diagnostic research disclaimers.
- `03-requirements.md`: Functional and security requirements for Task 05.
- `04-implementation-plan.md`: Step-by-step implementation roadmap.
- `05-report-data-contract.md`: Typed Rust contracts for report view models and export requests.
- `06-export-state-machine.md`: Nonblocking async state machine for PDF and session export.
- `07-audit-contract.md`: Typed Rust contracts for audit events and ledger queries.
- `08-tests.md`: Test strategy and automated verification suites.
- `09-test-results.md`: Complete automated test output.
- `10-desktop-verification.md`: Linux desktop launch and visual capture evidence.
- `11-file-change-report.md`: Detailed file modification log.
- `12-limitations-and-backend-gaps.md`: Gaps and technical debt in backend integration.
- `13-task06-handoff.md`: Specification and handoff for Task 06 (Patient & Examination Management).
