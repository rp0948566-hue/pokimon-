# Phase 1 • Task 06.1: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

## Architecture & File Mapping

- `rust/src/domain.rs`:
  - Add `PatientSubroute` enum (`Directory`, `Profile { patient_id: i64 }`, `ExamDetail { patient_id: i64, examination_id: i64 }`).
  - Add `ReportSubroute` enum (`QuadViewer`, `ReportOverview`, `PdfExport`, `SessionExport`, `AuditHistory`).
- `rust/src/ui/patients.rs`:
  - Update `PatientsViewState` with `current_subroute: PatientSubroute` and `selected_patient_preview: Option<i64>`.
  - Update `PatientsAction` enum with `NavigateTo(PatientSubroute)`, `BackToDirectory`, `BackToProfile`.
  - Implement `render_patient_directory()` redesign: Master-detail desktop layout with contextual preview panel, search/filter, and synthetic test badges.
  - Implement `render_patient_profile()`: Dedicated screen with breadcrumbs, Back button, demographic editor, and chronological examination timeline.
  - Implement `render_examination_detail()`: Dedicated screen with breadcrumbs, Back button, image inspection, Stage I results, truthful Stage II/III unavailable badges, and "Open in Report/Quad" actions.
- `rust/src/ui/viewer.rs`:
  - Update `ExamWorkspaceState` with `current_subroute: ReportSubroute`.
  - Update `ViewerAction` with `NavigateReportSubroute(ReportSubroute)` and `BackToReportWorkspace`.
  - Refactor `render_top_toolbar()`: Retain `[⊞ Quad Grid]` / `[📋 Report Overview]`, keep contextual viewport controls near viewport, and provide distinct workflow buttons (`[📄 PDF Export]`, `[📦 Session Dossier]`, `[📋 Audit History]`) that navigate to subroutes.
  - Implement `render_pdf_export_screen()`: Dedicated routed PDF export screen with report preview, local destination picker, and truthful `BackendUnavailable` banner.
  - Implement `render_session_export_screen()`: Dedicated routed session export screen with scope guard, plaintext unencrypted warnings, and verified local JSON export trigger.
  - Implement `render_audit_history_screen()`: Dedicated routed audit history screen showing event list and truthful unavailable state.
- `rust/src/main.rs`:
  - Wire actions from `patients.rs` and `viewer.rs` into `RetinaCareApp`.
  - Enforce cross-patient isolation and ID preservation across all route transitions.
  - Add integration tests proving route transitions, back stack, search/filter, and isolation invariants.

---

## Tasks

### Task 1: Navigation Subroutes & Data Models (`rust/src/domain.rs`)
- [ ] Add `PatientSubroute` and `ReportSubroute` enums with serialization and equality traits.
- [ ] Write unit tests verifying enum default values and conversions.
- [ ] Verify compilation with `cargo check`.

### Task 2: Patient Directory Master-Detail Redesign (`rust/src/ui/patients.rs`)
- [ ] Implement responsive master-detail desktop layout (master list on left, contextual preview card on right).
- [ ] Add search query filtering across MRN, given name, and family name with clear button and counts.
- [ ] Implement fallback single-column scrolling for narrow windows ($< 900\text{px}$).
- [ ] Add prominent amber `[DEV TEST FIXTURES ONLY - NON-CLINICAL]` chip when fixtures are active.
- [ ] Verify clean empty states when search yields no matches.

### Task 3: Dedicated Patient Profile & Examination Detail Screens (`rust/src/ui/patients.rs`)
- [ ] Implement `render_patient_profile()` screen with breadcrumb, `← Back to Directory` button, patient editor, and chronological examination timeline.
- [ ] Implement `render_examination_detail()` screen with breadcrumb, `← Back to Patient Profile` button, image preview card, Stage I screening results card, and Stage II/III honest unavailable badges.
- [ ] Add direct "Open in Report" and "Open in Quad Viewer" buttons that transition seamlessly to the correct exam context.

### Task 4: Report Workspace Toolbar Cleanup & Dedicated Export/Audit Routes (`rust/src/ui/viewer.rs`)
- [ ] Refactor `render_top_toolbar()` to eliminate mixed controls: Keep Quad/Report view switcher and contextual viewport controls (OD/OS, Sync, Fit, Reset) on the image viewer.
- [ ] Add distinct navigation buttons for `[📄 PDF Export]`, `[📦 Session Dossier]`, and `[📋 Audit History]`.
- [ ] Implement dedicated `render_pdf_export_screen()` with truthful `BackendUnavailable` notice.
- [ ] Implement dedicated `render_session_export_screen()` with scope guard and sensitive unencrypted JSON warnings.
- [ ] Implement dedicated `render_audit_history_screen()` with event timeline and truthful backend state.

### Task 5: Application Shell Integration & State Invariants (`rust/src/main.rs`)
- [ ] Connect `PatientSubroute` and `ReportSubroute` navigation actions in `RetinaCareApp`.
- [ ] Enforce strict cross-patient isolation on patient switch (purge textures, invalidate active requests, reset results).
- [ ] Preserve examination context and active IDs during Back navigation.
- [ ] Add comprehensive automated tests for:
  - Route transitions: Directory &rarr; Profile &rarr; Exam Detail &rarr; Report &rarr; Back
  - Cross-patient isolation (Patient A &rarr; Patient B texture and result purging)
  - Report / Quad mode retention
  - Truthful PDF export unavailable state
  - Truthful audit ledger unavailable state
  - Scoped JSON export isolation
  - Small window responsiveness

### Task 6: Visual Verification & Quality Gates
- [ ] Run `cargo fmt --manifest-path rust/Cargo.toml --all -- --check`.
- [ ] Run `cargo clippy --workspace --all-targets --manifest-path rust/Cargo.toml -- -D warnings`.
- [ ] Run `cargo test --workspace --manifest-path rust/Cargo.toml`.
- [ ] Run `cargo build --release --manifest-path rust/Cargo.toml`.
- [ ] Launch live desktop app under Linux X11 and capture the 7 required screenshots:
  - `patients_directory_verified.png`
  - `patient_profile_verified.png`
  - `exam_detail_verified.png`
  - `report_navigation_verified.png`
  - `export_unavailable_verified.png`
  - `audit_unavailable_verified.png`
  - `narrow_window_verified.png`

### Task 7: Final Documentation & Obsidian Vault Updates
- [ ] Create complete feature docs in `docs/features/GUI-P06-patient-navigation-redesign/`:
  - `05-test-results.md`
  - `06-desktop-verification.md`
  - `07-file-change-report.md`
  - `08-known-blockers.md`
  - `09-task07-handoff.md`
  - `README.md`
- [ ] Update Obsidian memory vault files in `memory/` ($\le 80$ lines per file, one fact per line, zero emoji).
- [ ] Deliver final structured report covering Sections A through L.
