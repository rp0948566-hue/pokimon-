# Phase 1 • Task 06.1: Screenshot and Source Baseline Audit

## 1. Screenshot Ground Truth Audit

### Screenshot A: Report Workspace Top Toolbar
- **Observed Toolbar Controls**:
  - `⊞ Quad Grid` / `📋 Report` (View switcher)
  - `OD` / `OS` (Laterality toggle)
  - `📄 PDF`, `📦 Session`, `📋 Audit` (Export and ledger workflows)
  - `🔗 Sync`, `⛶ Fit`, `↺ Reset` (Image viewer viewport controls)
- **Defect Identified**:
  - Major multi-step workflows (PDF Generation, Sensitive JSON Dossier Export, Audit Ledger Review) are jammed into the same horizontal line as granular local image manipulation controls (zoom fit, reset, laterality).
  - Activating PDF or Audit opened modal popups on top of the viewer rather than providing full, dedicated navigable workflow screens with document previews, parameter controls, and audit logs.

### Screenshot B: Existing Patients Sidebar Screen
- **Observed Page Layout**:
  - `Patients` route is already a functioning top-level item in the persistent left sidebar.
  - The screen lists three synthetic test patients (`P-1001`, `P-1002`, `P-1003`) near the top.
  - Below the short list, a massive empty void occupies ~70% of the viewport.
  - Selecting a patient merely expands the record in-place rather than presenting a dedicated Patient Profile destination.
  - Examination detail cards are nested directly under the expanded patient without a dedicated inspection route.
- **Defect Identified**:
  - Weak information hierarchy, excessive empty space, absence of a desktop master-detail layout, and lack of dedicated subroutes for `PatientProfile` and `ExaminationDetail`.

---

## 2. Current Source Audit

- **Frontend Root**: `/home/claude/Desktop/Hackthon dezine/rust/`
- **Application Routing (`rust/src/main.rs`)**:
  - Top-level `AppPage` enum currently contains `Home`, `Report`, `Patients`, `PreviousChats`, `Plugins`.
  - Navigation state currently tracks `active_patient_id: Option<i64>` and `active_examination_id: Option<i64>`.
- **Patients UI (`rust/src/ui/patients.rs`)**:
  - `PatientsViewState` tracks `selected_patient_id`, `search_query`, edit form state, and modals.
  - Lacks explicit subroute enum for `Directory`, `Profile`, and `ExamDetail`.
- **Viewer UI (`rust/src/ui/viewer.rs`)**:
  - `ExamWorkspaceState` tracks `report_view_mode: bool`, `export_dialog_open: bool`, and `audit_ledger_open: bool`.
  - Toolbar combines view controls and workflow modals.
  - Lacks dedicated routed screens for `PdfExport`, `SessionExport`, and `AuditHistory`.

---

## 3. Technology & Ownership Boundary

- **Framework**: 100% Native Rust desktop application using `egui` and `eframe` (v0.37.0).
- **Prohibited**: Zero HTML, CSS, JavaScript, TypeScript, React, Electron, Tauri, or WebView.
- **Backend Ownership**: Claude/Codex owns backend medical AI, database, model conversion, and PDF rasterization in `/home/claude/Desktop/Hackthon dezine/Hackthon Backend` and `/home/claude/Desktop/Dird`. Antigravity does NOT edit backend source files.
