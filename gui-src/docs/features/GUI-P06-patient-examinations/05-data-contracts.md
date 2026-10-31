# GUI Phase 1 • Task 06: Data Contracts

## 1. Domain Entities (`rust/src/engine/patient.rs`)

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatientSummary {
    pub id: i64,
    pub patient_code: String,
    pub display_name: String,
    pub examination_count: usize,
    pub latest_exam_date: Option<String>,
    pub is_synthetic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatientDetail {
    pub id: i64,
    pub patient_code: String,
    pub display_name: String,
    pub created_at: Option<String>,
    pub is_synthetic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExaminationSummary {
    pub id: i64,
    pub patient_id: i64,
    pub examined_at: String,
    pub operator: String,
    pub notes: String,
    pub eye: String, // "OD" / "OS" / "Unknown"
    pub image_path: String,
    pub analysis_json: String,
    pub is_synthetic: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PatientQuery {
    pub filter_text: String,
    pub eye_filter: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatientServiceReadiness {
    Ready,
    DatabaseLocked,
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatientServiceError {
    DatabaseLocked,
    BackendUnavailable(String),
    InvalidInput(String),
    NotFound(String),
    Storage(String),
}
```

## 2. Service Trait

```rust
pub trait PatientService: Send + Sync {
    fn readiness(&self) -> PatientServiceReadiness;
    fn list_patients(&self, query: &PatientQuery) -> Result<Vec<PatientSummary>, PatientServiceError>;
    fn get_patient(&self, id: i64) -> Result<PatientDetail, PatientServiceError>;
    fn list_examinations(&self, patient_id: i64) -> Result<Vec<ExaminationSummary>, PatientServiceError>;
    fn add_patient(&self, code: &str, name: &str) -> Result<i64, PatientServiceError>;
    fn update_patient(&self, id: i64, code: &str, name: &str) -> Result<(), PatientServiceError>;
    fn add_examination(&self, exam: &ExaminationSummary) -> Result<i64, PatientServiceError>;
}
```
