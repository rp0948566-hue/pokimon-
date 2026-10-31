# GUI Phase 1 • Task 06: Known Blockers & Backend Deficiencies

## 1. Backend Persistence Blockers (Codex Ownership)

| Subsystem | Missing Capability in Codex Crates | Impact on GUI | Current GUI Handling |
| :--- | :--- | :--- | :--- |
| **SQLCipher Store** | No indexed query/filter API in `retinacare-persistence` (`list_patients` does full table scan). | In-memory filtering required on client. | Filter implemented in `PatientService` adapter. |
| **Authentication Flow** | No credential dialog to pass unlock passphrase to `DatabaseController`. | Store remains in `Database locked` state by default. | Truthful status displayed; dev fixtures used in dev mode. |
| **Patient Demographics** | Table schema has only `patient_code` and `display_name`. No DOB, sex, or clinical notes. | Cannot collect or display full clinical profile. | Only supported fields displayed. |
| **Audit Persistence** | No `audit_events` table in `retinacare-persistence`. | Cannot persist patient access logs. | Truthful `AuditHistoryUnavailable` status displayed. |
| **PDF Generation Engine** | No rasterizer/PDF engine in `retinacare-reports`. | Cannot export report PDF. | Truthful `BackendUnavailable` status displayed. |
