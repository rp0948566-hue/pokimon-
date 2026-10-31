# Phase 1 • Task 06.1: Screen-by-Screen Requirements

## Workflow A: Patient Directory Screen (`PatientSubroute::Directory`)

1. **Header & Context**:
   - Title: `Patient Directory & Records`
   - Subtitle: `Searchable screening directory, clinical history, and examination profiles`
   - Database readiness indicator chip:
     - Production: `🔒 SQLCipher Locked (Auth Required)` (amber)
     - Dev mode: `[DEV TEST FIXTURES ONLY - NON-CLINICAL]` (amber)
2. **Search & Filter Controls**:
   - Search input supporting real-time filtering across Medical Record Number (MRN), Given Name, and Family Name.
   - Filter chips: `All`, `With Exams`, `OD`, `OS`.
   - Clear search button `✕`.
   - `+ Register Patient` primary action button.
3. **Master-Detail Desktop Layout ($\ge 900\text{px}$)**:
   - **Left Master Column (~55% width)**:
     - Card list of matching patient records.
     - Each card displays: Patient Code, Full Name, Sex, Birth Year, Exam count badge.
     - Highlight indicator on hover and active selection.
     - Direct `Profile →` button.
   - **Right Detail Preview Column (~45% width)**:
     - Sticky card presenting the currently highlighted patient:
       - Patient demographic summary card.
       - Quick stats: Total examinations, registered laterality counts.
       - Recent examination summary with OD/OS badge and date.
       - Primary action button: `Open Patient Profile →` navigating to `PatientProfile`.
       - Secondary action: `+ Add Examination`.
     - When no patient is selected, displays a clean clinical empty-state graphic with guidance text.
4. **Responsive Narrow Layout ($< 900\text{px}$ / $700\times 650$ / $680\times 580$)**:
   - Single-column vertical scroll.
   - Master list cards with embedded quick actions (`View Profile →`).

---

## Workflow B: Patient Profile Screen (`PatientSubroute::Profile`)

1. **Breadcrumbs & Back Navigation**:
   - Breadcrumb: `Patients / {patient_code} - {display_name}`
   - `← Back to Directory` button.
2. **Patient Metadata Editor**:
   - Demographics form: Patient Code, Family Name, Given Name, Birth Year, Sex, Clinical Notes.
   - Dirty tracking: Tracks unsaved modifications, activates `Save Changes` and `Discard` buttons, and shows an unsaved badge.
   - Validation: Birth year range [1900..2026], non-empty code.
3. **Examination Timeline**:
   - Chronological list of registered examinations.
   - Each examination card displays:
     - Exam ID (`EXAM-001`)
     - Laterality Badge: `OD (Right Eye)` in cyan or `OS (Left Eye)` in blue.
     - Operator name and clinical date.
     - Retinal image attachment badge.
     - Preliminary triage summary.
     - Direct Action Buttons:
       - `View Details →` &rarr; Navigates to `PatientSubroute::ExamDetail`.
       - `Open in Report` &rarr; Navigates to `AppPage::Report` in `ReportOverview` mode.
       - `Open in Quad` &rarr; Navigates to `AppPage::Report` in `QuadViewer` mode.
4. **Actions**:
   - `+ Register New Examination` modal trigger.

---

## Workflow C: Examination Detail Screen (`PatientSubroute::ExamDetail`)

1. **Breadcrumbs & Back Navigation**:
   - Breadcrumb: `Patients / {patient_code} / {exam_id}`
   - `← Back to Patient Profile` button.
2. **Two-Column Clinical Inspection**:
   - **Left Column: Examination Metadata & Image**:
     - Exam ID, Patient Code, Laterality (OD vs OS) chip.
     - Operator, Clinical Notes, Registration Timestamp.
     - Retinal fundus image card:
       - 200x200 preview thumbnail if image exists.
       - File path, resolution dimensions, and format.
       - `Change Image` / `Attach Image` button.
   - **Right Column: AI Screening Gate & Pipeline Integrity**:
     - Stage I Screening Card:
       - Model: `HierarchiRetina ConvNeXt-V2-Large (768px)`
       - Decision Gate: `Negative Route` vs `Continue Cascade (Referral Suggested)`
       - Calibrated Score & Threshold (`0.2391716`)
       - Logit value, Preprocessing latency, Inference latency.
     - Stage II (Vessels & Lesions) Status:
       - Truthful `ArtifactUnavailable` badge with explanation: "Model weights not deployed."
     - Stage III (5-Class DR Grading) Status:
       - Truthful `ArtifactUnavailable` badge with explanation: "Model weights not deployed."
     - Mandatory research disclaimer.
3. **Workspace Jump Buttons**:
   - `⊞ Open in Quad-Panel Viewer` &rarr; Switches to `Report` in Quad mode.
   - `📋 Open Clinical Report Overview` &rarr; Switches to `Report` in Overview mode.

---

## Workflow D: Report Workspace Navigation (`ReportSubroute::QuadViewer` vs `ReportOverview`)

1. **Dedicated View Switcher in Toolbar**:
   - Clean top bar with:
     - View Mode: `[⊞ Quad Grid]` / `[📋 Report Overview]`
     - Contextual Viewport Controls (rendered only in Quad Grid):
       - `[OD]` / `[OS]`
       - `[🔗 Sync]`
       - `[⛶ Fit]`
       - `[↺ Reset]`
     - Distinct Workflow Buttons:
       - `[📄 PDF Export]` &rarr; Navigates to `ReportSubroute::PdfExport`.
       - `[📦 Session Dossier]` &rarr; Navigates to `ReportSubroute::SessionExport`.
       - `[📋 Audit History]` &rarr; Navigates to `ReportSubroute::AuditHistory`.
2. **Quad-Panel Viewer**:
   - Original, Lesions, Vessels, and Explainability panels with synchronized pan/zoom.
3. **Clinical Report Overview**:
   - Patient header, Stage I findings card, retinal thumbnail, and research disclaimer.

---

## Workflow E: Dedicated Export & Audit Screens

1. **PDF Export Screen (`ReportSubroute::PdfExport`)**:
   - Breadcrumb: `Report / PDF Export (EXAM-xxx)` with `← Back to Report`.
   - Real Report preview card.
   - PDF Engine Readiness:
     - Truthfully reports: `⚠️ Backend PDF Engine Unavailable`.
     - Explicit notice: "Backend crate retinacare-reports lacks rasterization/printpdf engine."
     - Local save destination selector.
     - Save button disabled with truthful explanatory tooltip.
2. **Session Dossier Export Screen (`ReportSubroute::SessionExport`)**:
   - Breadcrumb: `Report / Session Dossier Export (EXAM-xxx)` with `← Back to Report`.
   - Scope Guard: Confirms export is strictly scoped to active patient and exam.
   - Privacy Warning: `⚠️ PLAINTEXT SENSITIVE EXPORT (UNENCRYPTED JSON)`.
   - Save path input.
   - `Export Plaintext JSON` button invoking verified engine function `export_scoped_patient_session()`.
3. **Audit History Screen (`ReportSubroute::AuditHistory`)**:
   - Breadcrumb: `Report / Audit Ledger (EXAM-xxx)` with `← Back to Report`.
   - Truthful readiness:
     - Production: `⚠️ Audit Persistence Unavailable (retinacare-persistence lacks audit_events table)`.
     - Dev mode / test mock: Displays filtered list of audit events with `[NON-CLINICAL TEST MOCK]` badge.
