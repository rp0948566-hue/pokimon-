# GUI Phase 1 • Task 06: Codex Backend API Handoff

## 1. Required Codex Service APIs

To promote Task 06 from **PARTIAL** to **COMPLETE**, the Codex backend team must implement the following APIs:

### A. Patient Repository APIs (`retinacare-persistence`)
```rust
impl SqlCipherStore {
    /// Search patients by code or name substring using SQLite LIKE
    pub fn search_patients(&self, query: &str) -> Result<Vec<PatientRecord>, StoreError>;

    /// Fetch a single patient by ID
    pub fn get_patient(&self, id: i64) -> Result<Option<PatientRecord>, StoreError>;

    /// Soft-delete or archive a patient
    pub fn archive_patient(&self, id: i64) -> Result<(), StoreError>;
}
```

### B. Examination Repository APIs (`retinacare-persistence`)
```rust
impl SqlCipherStore {
    /// List examinations scoped to a specific patient ID
    pub fn list_examinations_for_patient(&self, patient_id: i64) -> Result<Vec<Examination>, StoreError>;

    /// Fetch a single examination by ID
    pub fn get_examination(&self, id: i64) -> Result<Option<Examination>, StoreError>;
}
```

### C. Background Controller Actions (`retinacare-app`)
```rust
pub enum DatabaseAction {
    // Existing actions
    Open(Zeroizing<String>),
    AddPatient(String, String),
    UpdatePatient(i64, String, String),
    AddExamination(i64, String, String, String, String, String),
    Lock,
    Stop,
    // Required additions
    SearchPatients(String),
    GetPatient(i64),
    ListExaminationsForPatient(i64),
}
```
