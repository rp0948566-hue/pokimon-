# GUI Phase 1 • Task 06: Current State Audit

## 1. Environment & Repository Structure

- **GUI Workspace**: `/home/claude/Desktop/Hackthon dezine/rust` (Branch: `dezine`).
- **Backend Workspace**: `/home/claude/Desktop/Dird/native/crates` (Branch: `main`, read-only for Antigravity).
- **GUI Manifest (`rust/Cargo.toml`)**: Path-linked to `retinacare-ai-bridge` and `retinacare-app`.

---

## 2. Verified vs Reported Baseline

### Task 05 Handoff Verification
- **Reported**: Clinical report overview, local PDF export dialog, patient session JSON export, audit ledger drawer, 94 passing tests.
- **Audited Truth**:
  - `rust/src/engine/report.rs`: Implements `ReportViewModel` and mandatory research disclaimer.
  - `rust/src/engine/export.rs`: Implements `LiveCodexPdfService` (returns `BackendUnavailable`) and `LiveCodexAuditService` (returns `Unavailable`).
  - `export_scoped_patient_session()` produces unencrypted plaintext JSON. Per instruction 12, it must be explicitly labeled as "Plaintext Sensitive Export" and never called an "encrypted dossier".
  - Total automated tests passing: 94 tests.
  - Overall status of Task 05: **PARTIAL** due to missing backend PDF engine and audit table.

---

## 3. Codex Backend Crates Audit

Direct inspection of `/home/claude/Desktop/Dird/native/crates/` revealed the following:

### A. `retinacare-domain` (`native/crates/retinacare-domain/src/lib.rs`)
- `pub struct Patient`:
  - `id: Option<i64>`
  - `patient_code: String`
  - `display_name: String`
- `pub struct Examination`:
  - `id: Option<i64>`
  - `patient_id: i64`
  - `examined_at: String`
  - `operator: String`
  - `notes: String`
  - `eye: String` (`"left"`, `"right"`, `""`)
  - `image_path: String`
  - `analysis_json: String`

### B. `retinacare-persistence` (`native/crates/retinacare-persistence/src/lib.rs`)
- `pub struct PatientRecord`:
  - `id: i64`
  - `patient_code: String`
  - `display_name: String`
- `pub struct SqlCipherStore`:
  - `open(database_path, password) -> Result<Self, StoreError>`
  - `add_patient(&self, patient_code: &str, display_name: &str) -> Result<i64, StoreError>`
  - `list_patients(&self) -> Result<Vec<PatientRecord>, StoreError>`
  - `update_patient(&self, id: i64, patient_code: &str, display_name: &str) -> Result<(), StoreError>`
  - `add_examination(&self, patient_id: i64, operator: &str, notes: &str) -> Result<i64, StoreError>`
  - `add_screening_examination(&self, patient_id: i64, operator: &str, notes: &str, eye: &str, image_path: &str, analysis_json: &str) -> Result<i64, StoreError>`
  - `list_examinations(&self) -> Result<Vec<Examination>, StoreError>`

### C. Backend Gaps & Deficiencies
1. **No SQL Search / Filter**: `list_patients` and `list_examinations` perform unindexed full-table scans. Filtering must be performed in memory.
2. **No Patient Demographics**: Only `patient_code` and `display_name` exist in schema. No DOB, sex, or contact fields.
3. **No Deletion / Archiving**: No soft delete or active status column.
4. **No Audit Persistence**: No `audit_events` table exists in schema.
5. **Database Locked by Default**: Database requires explicit password opening. Without clinician authentication, background worker returns `"Database locked"`.
