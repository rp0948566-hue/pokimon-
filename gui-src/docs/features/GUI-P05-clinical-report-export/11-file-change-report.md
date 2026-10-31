# GUI Phase 1 • Task 05: File Change Report

## 1. Summary of Changes

| Category | File | Action | Lines Changed | Description |
| :--- | :--- | :---: | :---: | :--- |
| **Engine** | `rust/src/engine/report.rs` | Added | +218 | `ReportViewModel`, `PipelineStageStatus`, `MANDATORY_RESEARCH_CAVEAT`, isolation validator, unit tests |
| **Engine** | `rust/src/engine/export.rs` | Added | +446 | `ReportExportRequest`, `ExportState`, `ExportEvent`, `PdfExportService`, `AuditLedgerService`, `export_scoped_patient_session()`, unit tests |
| **Engine** | `rust/src/engine/mod.rs` | Modified | +9 / -0 | Re-exported report and export domain contracts and service traits |
| **Domain** | `rust/src/domain.rs` | Modified | +8 / -0 | Added `patient_code`, `report_view_mode`, `export_dialog_open`, `audit_ledger_open` to `RetinalExaminationView` |
| **UI** | `rust/src/ui/viewer.rs` | Modified | +1,770 / -167 | Clinical Report Overview renderer, PDF export modal, Audit ledger modal, view switcher buttons |
| **Main** | `rust/src/main.rs` | Modified | +415 / -0 | Service lifecycle state, asynchronous non-blocking event loop channel drain, modal event handling, integration tests |
| **Docs** | `docs/features/GUI-P05-clinical-report-export/` | Added | 14 files | Audit, research, specifications, architecture, tests, verification screenshots, limitations, handoff contracts |

---

## 2. Quantitative Diff Metrics

```
 rust/src/domain.rs     |    8 +
 rust/src/engine/mod.rs |    9 +
 rust/src/main.rs       |  415 ++++++++++++
 rust/src/ui/viewer.rs  | 1770 +++++++++++++++++++++++++++++++++++++++++++-----
 rust/src/engine/export.rs | 446 ++++++++++++++++++++++++++++++++++++++++++++++
 rust/src/engine/report.rs | 218 ++++++++++++++++++++++++++++++++++++++++++++++
 6 files changed, 2866 insertions(+), 167 deletions(-)
```

---

## 3. Detailed Component Breakdown

### A. New Engine Modules
- **`rust/src/engine/report.rs`**:
  - `ReportViewModel`: Pre-computed presentation data model for the clinical overview.
  - `PipelineStageStatus`: Typed status (`Available`, `Unavailable`, `Processing`, `Failed`) for modular pipeline stages.
  - `MANDATORY_RESEARCH_CAVEAT`: Constant string enforcing non-diagnostic screening framing.
  - `ReportViewModel::from_stage1_result()`: Pure mapper from genuine Stage I result into view model.
  - `ReportViewModel::validate_patient_isolation()`: Security validator ensuring no cross-patient pollution.

- **`rust/src/engine/export.rs`**:
  - `ReportExportRequest`: Typed parameters for PDF export (patient, exam, path, stage 1 result).
  - `ExportState`: Typed state machine (`Idle`, `Preparing`, `Generating`, `SaveRequested`, `Saved`, `Failed`, `Cancelled`, `BackendUnavailable`).
  - `ExportEvent`: Asynchronous notification dispatched over bounded channels to the UI thread.
  - `PdfServiceReadiness`: Enum signaling whether the backend PDF engine is ready.
  - `PdfExportService`: Trait abstracting PDF export generation.
  - `LiveCodexPdfService`: Production implementation that truthfully reports `PdfServiceReadiness::BackendUnavailable`.
  - `MockPdfExportService`: Mock implementation for offline GUI simulation and testing.
  - `AuditEvent`: Typed audit log entry (`event_id`, `event_time`, `event_type`, `patient_id`, `exam_id`, `actor_id`, `status`).
  - `AuditLedgerReadiness`: Enum signaling audit persistence availability.
  - `AuditLedgerService`: Trait abstracting audit logging and ledger querying.
  - `LiveCodexAuditService`: Production implementation that truthfully reports `AuditLedgerReadiness::Unavailable`.
  - `MockAuditLedgerService`: In-memory mock for audit ledger simulation and GUI testing.
  - `export_scoped_patient_session()`: Scoped JSON exporter isolating active patient and examination data.

### B. Modified UI Components
- **`rust/src/ui/viewer.rs`**:
  - Added `ToggleReportViewMode`, `OpenExportPdfDialog`, `OpenSessionExportDialog`, `ToggleAuditLedger`, `SubmitPdfExport`, `SubmitSessionExport`, `CloseModals` actions to `ViewerAction`.
  - Added workspace toolbar switcher: `[⊞ Quad-Panel]` vs `[📋 Report Overview]`.
  - Added workspace action buttons: `[📄 Export PDF]`, `[📦 Export Session]`, `[📋 Audit Ledger]`.
  - Implemented `render_clinical_report_overview()`:
    - Header card with patient and study metadata chips.
    - Centered fundus scan preview with proper aspect ratio containment.
    - Stage I ConvNeXt-V2 Screening hero card with gate decision, calibrated score, threshold `0.2391716`, logit, latency, and provenance.
    - Availability Matrix detailing the 3-stage modular pipeline status.
    - Research disclaimer caveat banner.
    - Actions bar with quick export and ledger buttons.
  - Implemented `render_pdf_export_modal()`:
    - Modal dialog with truthful backend unavailable status notice.
    - Explanation of missing `retinacare-reports` generation engine.
    - Clear handoff specifications for Codex backend team.
  - Implemented `render_audit_ledger_modal()`:
    - Modal dialog with truthful audit history unavailable status notice.
    - Explanation of missing `audit_events` persistence table.
    - Clear handoff specifications for Codex backend team.

### C. Application State Wiring
- **`rust/src/main.rs`**:
  - Integrated `pdf_export_service`, `audit_ledger_service`, `export_state`, `export_event_rx`, and `cached_audit_events` into `RetinaCareApp`.
  - Non-blocking channel drain in `update()` loop.
  - Wired event handlers for all new `ViewerAction` variants.
  - Added 4 integration tests verifying modal toggles, view mode switches, and session exports.
