# Phase 1 • Task 06.1: File Change Report

## 1. Summary of Changes

This corrective pass addressed the navigation architecture deficiencies documented in Screenshots A & B by introducing typed navigation subroutes, replacing crowded toolbars and modal dialogs with dedicated application views, and implementing responsive desktop layouts.

---

## 2. Modified Files Detail

### `rust/src/domain.rs`
- **Created Enums**:
  - `PatientSubroute`: `Directory`, `Profile { patient_id: i64 }`, `ExamDetail { patient_id: i64, examination_id: i64 }`. Derives `Debug, Clone, Copy, PartialEq, Eq, Default`.
  - `ReportSubroute`: `QuadViewer`, `ReportOverview`, `PdfExport`, `SessionExport`, `AuditHistory`. Derives `Debug, Clone, Copy, PartialEq, Eq, Default`.
- **Modified Struct**:
  - `RetinalExaminationView`: Added `pub current_subroute: ReportSubroute`.
- **Rationale**: Provides stable, compile-time verified routing state for both Patient management and Clinical Report workspaces without mutating backend entities or database structures.

### `rust/src/ui/patients.rs`
- **Modified Struct**: `PatientsViewState`
  - Added `pub current_subroute: PatientSubroute`.
  - Added `pub preview_patient_id: Option<i64>`.
- **Modified Enum**: `PatientsAction`
  - Added `NavigateSubroute(PatientSubroute)`.
  - Added `PreviewPatient(Option<i64>)`.
- **Refactored Rendering**:
  - `render_top_navigation_bar`: Renders hierarchical breadcrumbs (`Patients` → `Patient Code` → `Exam ID`) and `← Back` buttons.
  - `render_patients_view`: Routes based on `current_subroute`. On `Directory`, uses available width to conditionally render a two-column master-detail layout (54% master patient list, 44% preview summary card) if $\ge 900\text{px}$, falling back to vertical scroll for narrower screens.
  - `render_examination_detail`: Two-column layout rendering fundus scan metadata and Stage I AI gate results, with explicit disabled badges for unavailable Stage II/III models and direct jump buttons (`Open in Quad Viewer`, `Open in Report Overview`).

### `rust/src/ui/viewer.rs`
- **Modified Enum**: `ViewerAction`
  - Added `NavigateReportSubroute(ReportSubroute)`.
  - Added `BackToReportWorkspace`.
- **Refactored Toolbar (`render_workspace_toolbar`)**:
  - Decoupled viewport tools (`OD/OS`, `Sync`, `Fit`, `Reset`) so they render ONLY when `exam.current_subroute == ReportSubroute::QuadViewer`.
  - In `ReportOverview` mode, toolbar is clean and uncluttered, exposing explicit workflow buttons for `PDF Export`, `Session Export`, and `Audit History`.
- **Added Dedicated Route Renderers**:
  - `render_report_subroute_toolbar`: Standardized breadcrumb header with `← Back to Report Overview`.
  - `render_pdf_export_screen`: Dedicated screen displaying truthful unavailable notice for `retinacare-reports`, required API contract, and disabled action buttons.
  - `render_session_export_screen`: Dedicated screen displaying unencrypted sensitive JSON warnings, scope guards, and export trigger.
  - `render_audit_history_screen`: Dedicated screen displaying truthful notice of missing `audit_events` SQLite table and table schema requirements.

### `rust/src/main.rs`
- **Application State Handling**:
  - Wired `ViewerAction::NavigateReportSubroute` and `ViewerAction::BackToReportWorkspace`.
  - Wired `PatientsAction::NavigateSubroute` and `PatientsAction::PreviewPatient`.
  - Synchronized `current_subroute` with active examination state on patient load.
  - Added support for environment variables `RETINACARE_SUBROUTE` and `RETINACARE_REPORT_SUBROUTE` for automated testing and CI verification.
- **Test Suite Updates**:
  - Added `test_navigation_subroute_enum_and_defaults`.
  - Added `test_navigation_report_subroutes_pdf_session_audit_back`.
  - Added `test_patients_view_preview_selection`.
