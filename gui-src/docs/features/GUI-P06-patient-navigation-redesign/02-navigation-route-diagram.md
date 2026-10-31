# Phase 1 • Task 06.1: Navigation Route Diagram

## 1. Top-Level Shell & Persistent Sidebar

The application runs inside a single native OS window with a persistent left sidebar containing:
1. `Home` (`AppPage::Home`)
2. `Patients` (`AppPage::Patients`)
3. `Report` (`AppPage::Report`)
4. `Previous Chats` (`AppPage::PreviousChats`)
5. `Plugins` (`AppPage::Plugins`)

```
+---------------+-------------------------------------------------------------+
| RETINACARE    | Top Breadcrumb & Readiness Banner                           |
|               +-------------------------------------------------------------+
| [Home]        |                                                             |
| [Patients] *  |                                                             |
| [Report]      |                Routed Content Workspace                     |
| [Chats]       |             (One Desktop Window, Zero WebViews)             |
| [Plugins]     |                                                             |
|               |                                                             |
| [Collapse]    |                                                             |
+---------------+-------------------------------------------------------------+
```

---

## 2. Patients Workflow Route Hierarchy

```
AppPage::Patients
       |
       +---> PatientSubroute::Directory (Default)
       |        |
       |        +-- [Master Column]: Searchable list of patients with badges
       |        +-- [Detail Column]: Contextual preview card & quick actions
       |        +-- [Action]: Click "View Profile"
       |
       +---> PatientSubroute::Profile { patient_id: i64 }
       |        |
       |        +-- [Breadcrumb]: "Patients / {Patient Code} - {Name}"
       |        +-- [Back Action]: "← Back to Directory"
       |        +-- [Cards]: Patient Metadata Editor (dirty-tracked)
       |        +-- [Timeline]: Chronological Examination list with OD/OS chips
       |        +-- [Actions]: "View Details", "Open in Report", "Open in Quad"
       |
       +---> PatientSubroute::ExamDetail { patient_id: i64, examination_id: i64 }
                |
                +-- [Breadcrumb]: "Patients / {Patient Code} / {Exam ID}"
                +-- [Back Action]: "← Back to Patient Profile"
                +-- [Cards]: Retinal Image Thumbnail, Laterality (OD/OS), Metadata
                +-- [AI Results]: Stage I Screening Findings Card (Real ConvNeXt-V2)
                +-- [Truthful Badges]: Stage II/III Artifact Unavailable Badges
                +-- [Direct Route]: "Open in Quad Viewer" / "Open in Report Overview"
```

---

## 3. Report Workflow Route Hierarchy

```
AppPage::Report
       |
       +---> ReportSubroute::QuadViewer (Focused Retinal Inspection)
       |        |
       |        +-- Viewport controls: OD / OS, Sync, Fit, Reset, Panel Mode
       |        +-- Quad Panels: Original, Lesions, Vessels, Explainability
       |
       +---> ReportSubroute::ReportOverview (Clinical Overview)
       |        |
       |        +-- Patient header, fundus thumbnail, Stage I screening metrics
       |        +-- Direct workflow actions: [📄 PDF Export], [📦 Session], [📋 Audit]
       |
       +---> ReportSubroute::PdfExport { examination_id: String }
       |        |
       |        +-- [Breadcrumb]: "Report / PDF Export"
       |        +-- [Back Action]: "← Back to Report"
       |        +-- Report preview, readiness status, destination selector
       |        +-- Truthful BackendUnavailable state banner
       |
       +---> ReportSubroute::SessionExport { patient_id: i64, examination_id: i64 }
       |        |
       |        +-- [Breadcrumb]: "Report / Session Dossier Export"
       |        +-- [Back Action]: "← Back to Report"
       |        +-- Scope guard, unencrypted sensitive JSON warnings
       |        +-- Verified local file generation
       |
       +---> ReportSubroute::AuditHistory { examination_id: String }
                |
                +-- [Breadcrumb]: "Report / Audit Ledger History"
                +-- [Back Action]: "← Back to Report"
                +-- Event timeline, filter chips
                +-- Truthful BackendUnavailable banner (live) or Test Mock (dev)
```

---

## 4. State Transition & Isolation Invariants

1. **Isolation on Patient Transition**:
   - Navigating between different patients clears all cached image textures (`self.mode_textures[0] = None`), active inference request IDs (`self.active_request_id = None`), and prior screening results (`self.stage1_result = None`).
2. **Context Preservation on Back Navigation**:
   - `ExamDetail` &rarr; `Back` returns to `Profile(patient_id)` without losing patient context.
   - `Profile` &rarr; `Back` returns to `Directory` while retaining search query and selection cursor.
   - `PdfExport` / `SessionExport` / `AuditHistory` &rarr; `Back` returns to previous `Report` mode (Quad or Overview) with image and examination context intact.
3. **Mismatched Guard**:
   - Loading `ExaminationDetail` or `SessionExport` validates that `examination_id` belongs to `patient_id`.
