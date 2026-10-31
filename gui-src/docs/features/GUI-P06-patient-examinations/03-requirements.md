# GUI Phase 1 • Task 06: Requirements Specification

## 1. Functional Requirements

- **REQ-01: Patient Directory View**:
  - Filterable by patient code / identifier.
  - Distinct UI representations for: `Loading`, `Empty`, `DatabaseLocked`, `Error`, and `Populated`.
  - Synthetic test fixtures allowed ONLY in `#[cfg(test)]` and isolated dev mode (`RETINACARE_DEV_FIXTURES=1`), clearly labeled.
- **REQ-02: Patient Detail View**:
  - Display immutable internal ID and editable `patient_code` / `display_name`.
  - Dirty-state tracking with `Cancel` / `Save` buttons.
  - Truthful save state: If persistence is unavailable, the save button must be disabled or report `BackendUnavailable`.
- **REQ-03: Examination List & Detail**:
  - Filtered strictly to the selected patient ID.
  - Displays Examination ID, study timestamp, laterality (`OD` / `OS` / `Unknown`), operator, and notes.
  - Status indicators for fundus image presence and Stage I analysis result.
- **REQ-04: Image Association**:
  - Validates image file existence, extension (`.png`, `.jpg`, `.jpeg`), and minimum dimensions.
  - Prompts for confirmation before replacing an existing image.
  - Scopes image artifact to the exact examination ID.
- **REQ-05: Navigation**:
  - Sidebar item `Patients` added to `AppPage::Patients`.
  - Seamless transitions: `Directory` -> `Detail` -> `Examinations` -> `[📋 Report]` / `[⊞ Viewer]`.
  - Breadcrumb and return navigation.

---

## 2. Privacy & Security Requirements

- **SEC-01: Cross-Patient Isolation**:
  - Switching patient immediately invalidates active inference requests, drops previous fundus textures, and clears examination view state.
- **SEC-02: Plaintext Export Labeling**:
  - Any JSON export from Task 05 (`export_scoped_patient_session`) must be explicitly labeled as "Plaintext Sensitive Export" with security warnings.
- **SEC-03: Zero Plaintext Logging**:
  - No logging of patient names, medical codes, or fundus image bytes to terminal or unencrypted disk.
- **SEC-04: Non-Diagnostic Framing**:
  - No synthetic DR grades or fabricated lesion counts in patient or examination cards.
