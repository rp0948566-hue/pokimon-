//! Patient & Examination Management service layer and typed contracts.
//!
//! # Architecture & Boundary Notice (Phase 1 • Task 06)
//! Durable persistence and encryption are owned exclusively by Codex backend crates
//! (`retinacare-persistence` and `retinacare-app`). This engine layer provides typed
//! frontend adapters, data validation, deterministic test mocks, and truthful status
//! reporting for the native Rust GUI.

use std::sync::{Arc, Mutex};

/// Summary representation of a patient for the directory listing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatientSummary {
    pub id: i64,
    pub patient_code: String,
    pub display_name: String,
    pub examination_count: usize,
    pub latest_exam_date: Option<String>,
    pub is_synthetic: bool,
}

/// Aggregate counts over every authorized record. Never estimated: computed from the store snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectoryStats {
    pub total_patients: usize,
    pub total_examinations: usize,
    pub od_examinations: usize,
    pub os_examinations: usize,
    pub recent_30_days: usize,
}

/// Detailed patient record for viewing and editing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatientDetail {
    pub id: i64,
    pub patient_code: String,
    pub display_name: String,
    pub created_at: Option<String>,
    pub is_synthetic: bool,
}

/// Summary of an individual retinal examination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExaminationSummary {
    pub id: i64,
    pub patient_id: i64,
    pub patient_code: String,
    pub examined_at: String,
    pub operator: String,
    pub notes: String,
    pub eye: String, // "OD", "OS", or "Unknown"
    pub image_path: String,
    pub has_image: bool,
    pub has_analysis: bool,
    pub analysis_summary: Option<String>,
    pub is_synthetic: bool,
}

/// Query and filter parameters for searching the patient directory.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PatientQuery {
    pub filter_text: String,
    pub eye_filter: Option<String>,
}

/// Health and readiness state of the patient persistence service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatientServiceReadiness {
    /// Connected to live, unlocked SQLCipher backend.
    Ready,
    /// Backend SQLCipher store exists but requires password unlock.
    DatabaseLocked,
    /// Backend service is offline or unavailable.
    Unavailable(String),
    /// Isolated development fixture mode (strictly non-clinical).
    DevFixtureMode,
}

/// Errors occurring during patient or examination operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatientServiceError {
    DatabaseLocked,
    BackendUnavailable(String),
    InvalidInput(String),
    NotFound(String),
    Storage(String),
}

impl std::fmt::Display for PatientServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseLocked => {
                write!(f, "SQLCipher database is locked. Master password required.")
            }
            Self::BackendUnavailable(reason) => write!(f, "Patient backend unavailable: {reason}"),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
            Self::NotFound(msg) => write!(f, "Record not found: {msg}"),
            Self::Storage(msg) => write!(f, "Storage error: {msg}"),
        }
    }
}

impl std::error::Error for PatientServiceError {}

/// Abstract interface for patient directory, detail, and examination operations.
pub trait PatientService: Send + Sync {
    /// Reports the current operational status of the service.
    fn readiness(&self) -> PatientServiceReadiness;

    /// Lists patients matching the supplied query.
    fn list_patients(
        &self,
        query: &PatientQuery,
    ) -> Result<Vec<PatientSummary>, PatientServiceError>;

    /// Fetches detailed record for a specific patient.
    fn get_patient(&self, id: i64) -> Result<PatientDetail, PatientServiceError>;

    /// Lists all examinations for a specific patient.
    fn list_examinations(
        &self,
        patient_id: i64,
    ) -> Result<Vec<ExaminationSummary>, PatientServiceError>;

    /// Registers a new patient record.
    fn add_patient(&self, code: &str, name: &str) -> Result<i64, PatientServiceError>;

    /// Updates an existing patient record.
    fn update_patient(&self, id: i64, code: &str, name: &str) -> Result<(), PatientServiceError>;

    /// Creates a new examination associated with a patient.
    fn add_examination(&self, exam: &ExaminationSummary) -> Result<i64, PatientServiceError>;

    /// `None` for services that never lock (dev fixtures). Otherwise the current unlock state.
    fn unlock_status(&self) -> Option<super::store::UnlockStatus> {
        None
    }

    /// Counts over ALL authorized records (not the search filter). `None` when unavailable.
    fn directory_stats(&self) -> Option<DirectoryStats> {
        None
    }

    /// Re-read the records on the next query.
    fn refresh_directory(&self) {}

    /// False on first use: unlocking then creates the database.
    fn database_exists(&self) -> bool {
        true
    }

    /// Start unlocking on a worker thread; poll `unlock_status`. No-op for services that never lock.
    fn begin_unlock(&self, _password: String, _confirm: Option<String>) {}

    /// Store the compact analysis summary (JSON, no pixels) with a saved examination.
    fn save_examination_analysis(
        &self,
        _examination_id: i64,
        _analysis_json: &str,
    ) -> Result<(), PatientServiceError> {
        Err(PatientServiceError::BackendUnavailable(
            "this patient service does not persist analysis results".to_string(),
        ))
    }

    /// The reserved, detail-free owner of draft examinations analysed before a patient was chosen.
    fn ensure_unassigned_patient(&self) -> Result<i64, PatientServiceError> {
        Err(PatientServiceError::BackendUnavailable(
            "this patient service does not keep unassigned drafts".to_string(),
        ))
    }

    /// Move a draft examination (with its masks and explanations) to a real patient.
    fn assign_examination(
        &self,
        _examination_id: i64,
        _patient_id: i64,
    ) -> Result<(), PatientServiceError> {
        Err(PatientServiceError::BackendUnavailable(
            "this patient service does not keep unassigned drafts".to_string(),
        ))
    }

    /// Attach the eye and source image path to a saved examination.
    fn set_examination_image(
        &self,
        _examination_id: i64,
        _eye: &str,
        _image_path: &str,
    ) -> Result<(), PatientServiceError> {
        Err(PatientServiceError::BackendUnavailable(
            "this patient service does not persist examination images".to_string(),
        ))
    }

    /// The stored analysis summary JSON of a saved examination, if one exists.
    fn load_analysis(&self, _examination_id: i64) -> Option<String> {
        None
    }

    /// Store the five verified masks of a completed run (all or nothing). The eye uses the UI
    /// spelling (`OD`/`OS`).
    fn save_examination_masks(
        &self,
        _run: retinacare_persistence::MaskRunInput,
    ) -> Result<(), PatientServiceError> {
        Err(PatientServiceError::BackendUnavailable(
            "this patient service does not persist masks".to_string(),
        ))
    }

    /// The stored masks of a saved examination, verified against the given patient, eye (UI
    /// spelling) and source image hash. `Ok(None)` for an examination without stored masks.
    fn load_examination_masks(
        &self,
        _examination_id: i64,
        _patient_id: i64,
        _eye: &str,
        _source_image_sha256: Option<&str>,
    ) -> Result<Option<retinacare_persistence::StoredMaskRun>, PatientServiceError> {
        Ok(None)
    }

    /// Store one verified Phase 4 attribution (raw map + metadata) in the encrypted database.
    fn save_attribution(
        &self,
        _input: retinacare_persistence::AttributionInput,
    ) -> Result<(), PatientServiceError> {
        Err(PatientServiceError::BackendUnavailable(
            "this patient service does not persist explanations".to_string(),
        ))
    }

    /// Saved explanations of one examination, newest first (summaries only).
    fn list_attributions(
        &self,
        _examination_id: i64,
        _patient_id: i64,
    ) -> Result<Vec<retinacare_persistence::AttributionSummary>, PatientServiceError> {
        Ok(Vec::new())
    }

    /// `(meta_json, raw payload, payload sha256)` after identity and hash checks; `Ok(None)` if absent.
    fn load_attribution(
        &self,
        _artifact_id: &str,
        _patient_id: i64,
        _eye: &str,
        _image_sha256: &str,
    ) -> Result<Option<(String, Vec<u8>, String)>, PatientServiceError> {
        Ok(None)
    }

    /// An analysis re-run makes earlier explanations stale.
    fn mark_attributions_stale(&self, _examination_id: i64) {}

    /// The encrypted store for conversations (Phase 5). `None` for services without one.
    fn chat_store(&self) -> Option<super::store::SharedStore> {
        None
    }
}

// ---------------------------------------------------------------------------
// Live Patient Service: encrypted SQLCipher store (retinacare-persistence)
// ---------------------------------------------------------------------------

/// Read-only copy of the database taken once per change, so the UI can ask for the directory every
/// frame without querying SQLCipher every frame.
struct Snapshot {
    generation: u64,
    patients: Vec<retinacare_persistence::PatientRecord>,
    examinations: Vec<retinacare_domain::Examination>,
}

/// Production patient service on the encrypted local database. Truthfully `DatabaseLocked` until the
/// user supplies the password; never serves fixture data.
pub struct LiveCodexPatientService {
    store: super::store::SharedStore,
    generation: std::sync::atomic::AtomicU64,
    snapshot: Mutex<Option<Arc<Snapshot>>>,
}

impl Default for LiveCodexPatientService {
    fn default() -> Self {
        Self::new()
    }
}

fn eye_to_ui(db_eye: &str) -> &'static str {
    match db_eye {
        "right" => "OD",
        "left" => "OS",
        _ => "Unknown",
    }
}

fn eye_to_db(ui_eye: &str) -> &'static str {
    match ui_eye {
        "OD" => "right",
        "OS" => "left",
        _ => "",
    }
}

fn analysis_summary(analysis_json: &str) -> Option<String> {
    if analysis_json.trim().is_empty() {
        return None;
    }
    match serde_json::from_str::<super::cascade::CascadeOutcome>(analysis_json) {
        Ok(outcome) => Some(outcome.headline),
        Err(_) => Some("Stored analysis record (older or unrecognised format)".to_string()),
    }
}

impl LiveCodexPatientService {
    pub fn new() -> Self {
        Self::with_store(super::store::SharedStore::new(
            super::store::default_db_path(),
        ))
    }

    pub fn with_store(store: super::store::SharedStore) -> Self {
        Self {
            store,
            generation: std::sync::atomic::AtomicU64::new(1),
            snapshot: Mutex::new(None),
        }
    }

    /// The database handle, for services that share it (audit ledger).
    pub fn store(&self) -> super::store::SharedStore {
        self.store.clone()
    }

    fn bump(&self) {
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn snapshot(&self) -> Result<Arc<Snapshot>, PatientServiceError> {
        let generation = self.generation.load(std::sync::atomic::Ordering::SeqCst);
        let mut cache = self
            .snapshot
            .lock()
            .map_err(|_| PatientServiceError::Storage("snapshot lock poisoned".to_string()))?;
        if let Some(existing) = cache.as_ref() {
            if existing.generation == generation && self.store.is_open() {
                return Ok(Arc::clone(existing));
            }
        }
        let fresh = self.store.with(|s| {
            Ok(Snapshot {
                generation,
                patients: s.list_patients()?,
                examinations: s.list_examinations()?,
            })
        });
        match fresh {
            Ok(snapshot) => {
                let snapshot = Arc::new(snapshot);
                *cache = Some(Arc::clone(&snapshot));
                Ok(snapshot)
            }
            Err(error) => {
                *cache = None;
                Err(error)
            }
        }
    }
}

impl PatientService for LiveCodexPatientService {
    fn readiness(&self) -> PatientServiceReadiness {
        match self.store.status() {
            super::store::UnlockStatus::Open => PatientServiceReadiness::Ready,
            _ => PatientServiceReadiness::DatabaseLocked,
        }
    }

    fn unlock_status(&self) -> Option<super::store::UnlockStatus> {
        Some(self.store.status())
    }

    fn database_exists(&self) -> bool {
        self.store.database_exists()
    }

    fn begin_unlock(&self, password: String, confirm: Option<String>) {
        self.store.begin_unlock(password, confirm);
    }

    fn directory_stats(&self) -> Option<DirectoryStats> {
        let snapshot = self.snapshot().ok()?;
        // ISO-8601 UTC strings compare correctly as text.
        let cutoff = super::report::utc_timestamp(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().saturating_sub(30 * 86_400))
                .unwrap_or(0),
        );
        Some(DirectoryStats {
            total_patients: snapshot.patients.len(),
            total_examinations: snapshot.examinations.len(),
            od_examinations: snapshot
                .examinations
                .iter()
                .filter(|e| e.eye == "right")
                .count(),
            os_examinations: snapshot
                .examinations
                .iter()
                .filter(|e| e.eye == "left")
                .count(),
            recent_30_days: snapshot
                .examinations
                .iter()
                .filter(|e| e.examined_at >= cutoff)
                .count(),
        })
    }

    fn refresh_directory(&self) {
        self.bump();
    }

    fn list_patients(
        &self,
        query: &PatientQuery,
    ) -> Result<Vec<PatientSummary>, PatientServiceError> {
        let snapshot = self.snapshot()?;
        let needle = query.filter_text.trim().to_lowercase();
        Ok(snapshot
            .patients
            .iter()
            .filter(|p| {
                needle.is_empty()
                    || p.patient_code.to_lowercase().contains(&needle)
                    || p.display_name.to_lowercase().contains(&needle)
            })
            .filter_map(|p| {
                let exams: Vec<_> = snapshot
                    .examinations
                    .iter()
                    .filter(|e| e.patient_id == p.id)
                    .collect();
                if let Some(eye) = &query.eye_filter {
                    if !exams.iter().any(|e| eye_to_ui(&e.eye) == eye.as_str()) {
                        return None;
                    }
                }
                Some(PatientSummary {
                    id: p.id,
                    patient_code: p.patient_code.clone(),
                    display_name: p.display_name.clone(),
                    examination_count: exams.len(),
                    latest_exam_date: exams.iter().map(|e| e.examined_at.clone()).max(),
                    is_synthetic: false,
                })
            })
            .collect())
    }

    fn get_patient(&self, id: i64) -> Result<PatientDetail, PatientServiceError> {
        let snapshot = self.snapshot()?;
        snapshot
            .patients
            .iter()
            .find(|p| p.id == id)
            .map(|p| PatientDetail {
                id: p.id,
                patient_code: p.patient_code.clone(),
                display_name: p.display_name.clone(),
                created_at: Some(p.created_at.clone()),
                is_synthetic: false,
            })
            .ok_or_else(|| PatientServiceError::NotFound(format!("Patient {id} not found")))
    }

    fn list_examinations(
        &self,
        patient_id: i64,
    ) -> Result<Vec<ExaminationSummary>, PatientServiceError> {
        let snapshot = self.snapshot()?;
        let code = snapshot
            .patients
            .iter()
            .find(|p| p.id == patient_id)
            .map(|p| p.patient_code.clone())
            .ok_or_else(|| {
                PatientServiceError::NotFound(format!("Patient {patient_id} not found"))
            })?;
        // Oldest first, matching the order the examination pages expect.
        let mut exams: Vec<_> = snapshot
            .examinations
            .iter()
            .filter(|e| e.patient_id == patient_id)
            .collect();
        exams.sort_by_key(|e| e.id);
        Ok(exams
            .into_iter()
            .map(|e| ExaminationSummary {
                id: e.id.unwrap_or_default(),
                patient_id,
                patient_code: code.clone(),
                examined_at: e.examined_at.clone(),
                operator: e.operator.clone(),
                notes: e.notes.clone(),
                eye: eye_to_ui(&e.eye).to_string(),
                image_path: e.image_path.clone(),
                has_image: !e.image_path.is_empty(),
                has_analysis: !e.analysis_json.is_empty(),
                analysis_summary: analysis_summary(&e.analysis_json),
                is_synthetic: false,
            })
            .collect())
    }

    fn add_patient(&self, code: &str, name: &str) -> Result<i64, PatientServiceError> {
        let id = self
            .store
            .with(|s| s.add_patient(code.trim(), name.trim()))?;
        self.bump();
        Ok(id)
    }

    fn update_patient(&self, id: i64, code: &str, name: &str) -> Result<(), PatientServiceError> {
        self.store
            .with(|s| s.update_patient(id, code.trim(), name.trim()))?;
        self.bump();
        Ok(())
    }

    fn add_examination(&self, exam: &ExaminationSummary) -> Result<i64, PatientServiceError> {
        let id = self.store.with(|s| {
            s.add_screening_examination(
                exam.patient_id,
                &exam.operator,
                &exam.notes,
                eye_to_db(&exam.eye),
                &exam.image_path,
                "",
            )
        })?;
        self.bump();
        Ok(id)
    }

    fn ensure_unassigned_patient(&self) -> Result<i64, PatientServiceError> {
        let id = self.store.with(|s| s.ensure_unassigned_patient())?;
        self.bump();
        Ok(id)
    }

    fn assign_examination(
        &self,
        examination_id: i64,
        patient_id: i64,
    ) -> Result<(), PatientServiceError> {
        self.store
            .with(|s| s.reassign_examination(examination_id, patient_id))?;
        self.bump();
        // The stored summary names its patient; keep it equal to the new owner.
        if let Some(json) = self.load_analysis(examination_id) {
            if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&json) {
                value["patient_id"] = serde_json::json!(patient_id);
                self.save_examination_analysis(examination_id, &value.to_string())?;
            }
        }
        Ok(())
    }

    fn save_examination_analysis(
        &self,
        examination_id: i64,
        analysis_json: &str,
    ) -> Result<(), PatientServiceError> {
        self.store
            .with(|s| s.set_examination_analysis(examination_id, analysis_json))?;
        self.bump();
        Ok(())
    }

    fn set_examination_image(
        &self,
        examination_id: i64,
        eye: &str,
        image_path: &str,
    ) -> Result<(), PatientServiceError> {
        self.store
            .with(|s| s.set_examination_image(examination_id, eye_to_db(eye), image_path))?;
        self.bump();
        Ok(())
    }

    fn load_analysis(&self, examination_id: i64) -> Option<String> {
        let snapshot = self.snapshot().ok()?;
        snapshot
            .examinations
            .iter()
            .find(|e| e.id == Some(examination_id))
            .map(|e| e.analysis_json.clone())
            .filter(|json| !json.is_empty())
    }

    fn save_examination_masks(
        &self,
        mut run: retinacare_persistence::MaskRunInput,
    ) -> Result<(), PatientServiceError> {
        run.eye = eye_to_db(&run.eye).to_string();
        self.store.with(|s| s.save_mask_run(&run).map(|_| ()))?;
        Ok(())
    }

    fn load_examination_masks(
        &self,
        examination_id: i64,
        patient_id: i64,
        eye: &str,
        source_image_sha256: Option<&str>,
    ) -> Result<Option<retinacare_persistence::StoredMaskRun>, PatientServiceError> {
        let expected = retinacare_persistence::MaskExpectation {
            patient_id,
            eye: eye_to_db(eye).to_string(),
            source_image_sha256: source_image_sha256.map(str::to_string),
        };
        self.store
            .with(|s| s.load_mask_run(examination_id, &expected))
    }

    fn save_attribution(
        &self,
        mut input: retinacare_persistence::AttributionInput,
    ) -> Result<(), PatientServiceError> {
        input.eye = eye_to_db(&input.eye).to_string();
        self.store
            .with(|s| s.save_attribution(&input).map(|_| ()))?;
        Ok(())
    }

    fn list_attributions(
        &self,
        examination_id: i64,
        patient_id: i64,
    ) -> Result<Vec<retinacare_persistence::AttributionSummary>, PatientServiceError> {
        self.store
            .with(|s| s.list_attributions(examination_id, patient_id))
    }

    fn load_attribution(
        &self,
        artifact_id: &str,
        patient_id: i64,
        eye: &str,
        image_sha256: &str,
    ) -> Result<Option<(String, Vec<u8>, String)>, PatientServiceError> {
        let expected = retinacare_persistence::AttributionExpectation {
            patient_id,
            eye: eye_to_db(eye).to_string(),
            image_sha256: image_sha256.to_string(),
        };
        self.store
            .with(|s| s.load_attribution(artifact_id, &expected))
    }

    fn mark_attributions_stale(&self, examination_id: i64) {
        let _ = self
            .store
            .with(|s| s.mark_attributions_stale(examination_id).map(|_| ()));
    }

    fn chat_store(&self) -> Option<super::store::SharedStore> {
        Some(self.store.clone())
    }
}

// ---------------------------------------------------------------------------
// Mock Patient Service (Deterministic Test & Dev Fixtures Only)
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct MockDb {
    patients: Vec<PatientDetail>,
    examinations: Vec<ExaminationSummary>,
    next_patient_id: i64,
    next_exam_id: i64,
}

/// In-memory mock service for automated unit testing and isolated dev mode.
///
/// All records generated or stored through this service are explicitly labeled
/// `is_synthetic = true`.
pub struct MockPatientService {
    db: Arc<Mutex<MockDb>>,
    readiness: PatientServiceReadiness,
}

impl Default for MockPatientService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPatientService {
    pub fn new() -> Self {
        Self {
            db: Arc::new(Mutex::new(MockDb {
                patients: Vec::new(),
                examinations: Vec::new(),
                next_patient_id: 1,
                next_exam_id: 1,
            })),
            readiness: PatientServiceReadiness::Ready,
        }
    }

    /// Creates a mock service pre-seeded with deterministic synthetic fixtures.
    pub fn with_fixtures() -> Self {
        let mut db = MockDb {
            patients: Vec::new(),
            examinations: Vec::new(),
            next_patient_id: 4,
            next_exam_id: 5,
        };

        // Synthetic Patient 1: Has 2 exams (OD and OS)
        db.patients.push(PatientDetail {
            id: 1,
            patient_code: "TEST-P001".to_string(),
            display_name: "Synthetic Patient Alpha".to_string(),
            created_at: Some("2026-10-25T09:00:00Z".to_string()),
            is_synthetic: true,
        });

        db.examinations.push(ExaminationSummary {
            id: 1,
            patient_id: 1,
            patient_code: "TEST-P001".to_string(),
            examined_at: "2026-10-27T10:30:00Z".to_string(),
            operator: "Dr. Tester".to_string(),
            notes: "Routine diabetic retinopathy baseline screening.".to_string(),
            eye: "OD".to_string(),
            image_path: "/home/claude/Desktop/Dird/dist/demo-images/session-1-final/OD-1.png"
                .to_string(),
            has_image: true,
            has_analysis: true,
            analysis_summary: Some("Stage I Screening: Negative Route (Score 0.114)".to_string()),
            is_synthetic: true,
        });

        db.examinations.push(ExaminationSummary {
            id: 2,
            patient_id: 1,
            patient_code: "TEST-P001".to_string(),
            examined_at: "2026-10-27T10:35:00Z".to_string(),
            operator: "Dr. Tester".to_string(),
            notes: "Contralateral eye evaluation.".to_string(),
            eye: "OS".to_string(),
            image_path: "".to_string(),
            has_image: false,
            has_analysis: false,
            analysis_summary: None,
            is_synthetic: true,
        });

        // Synthetic Patient 2: Has 1 exam
        db.patients.push(PatientDetail {
            id: 2,
            patient_code: "TEST-P002".to_string(),
            display_name: "Synthetic Patient Beta".to_string(),
            created_at: Some("2026-10-26T11:15:00Z".to_string()),
            is_synthetic: true,
        });

        db.examinations.push(ExaminationSummary {
            id: 3,
            patient_id: 2,
            patient_code: "TEST-P002".to_string(),
            examined_at: "2026-10-27T14:20:00Z".to_string(),
            operator: "Tech. Smith".to_string(),
            notes: "Mild fundus blur due to media opacity.".to_string(),
            eye: "OD".to_string(),
            image_path: "".to_string(),
            has_image: false,
            has_analysis: false,
            analysis_summary: None,
            is_synthetic: true,
        });

        // Synthetic Patient 3: Has 0 exams (for empty exam list testing)
        db.patients.push(PatientDetail {
            id: 3,
            patient_code: "TEST-P003".to_string(),
            display_name: "Synthetic Patient Gamma".to_string(),
            created_at: Some("2026-10-27T16:00:00Z".to_string()),
            is_synthetic: true,
        });

        Self {
            db: Arc::new(Mutex::new(db)),
            readiness: PatientServiceReadiness::DevFixtureMode,
        }
    }
}

impl PatientService for MockPatientService {
    fn readiness(&self) -> PatientServiceReadiness {
        self.readiness.clone()
    }

    fn list_patients(
        &self,
        query: &PatientQuery,
    ) -> Result<Vec<PatientSummary>, PatientServiceError> {
        let db = self
            .db
            .lock()
            .map_err(|e| PatientServiceError::Storage(e.to_string()))?;
        let filter = query.filter_text.trim().to_lowercase();

        let mut summaries = Vec::new();
        for p in &db.patients {
            if !filter.is_empty()
                && !p.patient_code.to_lowercase().contains(&filter)
                && !p.display_name.to_lowercase().contains(&filter)
            {
                continue;
            }

            let exams: Vec<_> = db
                .examinations
                .iter()
                .filter(|e| e.patient_id == p.id)
                .collect();
            let latest_date = exams.last().map(|e| e.examined_at.clone());

            summaries.push(PatientSummary {
                id: p.id,
                patient_code: p.patient_code.clone(),
                display_name: p.display_name.clone(),
                examination_count: exams.len(),
                latest_exam_date: latest_date,
                is_synthetic: p.is_synthetic,
            });
        }

        Ok(summaries)
    }

    fn get_patient(&self, id: i64) -> Result<PatientDetail, PatientServiceError> {
        let db = self
            .db
            .lock()
            .map_err(|e| PatientServiceError::Storage(e.to_string()))?;
        db.patients
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| PatientServiceError::NotFound(format!("Patient {id} not found")))
    }

    fn list_examinations(
        &self,
        patient_id: i64,
    ) -> Result<Vec<ExaminationSummary>, PatientServiceError> {
        let db = self
            .db
            .lock()
            .map_err(|e| PatientServiceError::Storage(e.to_string()))?;
        let exams: Vec<_> = db
            .examinations
            .iter()
            .filter(|e| e.patient_id == patient_id)
            .cloned()
            .collect();
        Ok(exams)
    }

    fn add_patient(&self, code: &str, name: &str) -> Result<i64, PatientServiceError> {
        if code.trim().is_empty() || code.len() > 64 {
            return Err(PatientServiceError::InvalidInput(
                "Patient code must be 1-64 characters".to_string(),
            ));
        }
        if name.trim().is_empty() || name.len() > 200 {
            return Err(PatientServiceError::InvalidInput(
                "Display name must be 1-200 characters".to_string(),
            ));
        }

        let mut db = self
            .db
            .lock()
            .map_err(|e| PatientServiceError::Storage(e.to_string()))?;
        if db
            .patients
            .iter()
            .any(|p| p.patient_code.eq_ignore_ascii_case(code.trim()))
        {
            return Err(PatientServiceError::InvalidInput(
                "Patient code already exists".to_string(),
            ));
        }

        let id = db.next_patient_id;
        db.next_patient_id += 1;
        db.patients.push(PatientDetail {
            id,
            patient_code: code.trim().to_string(),
            display_name: name.trim().to_string(),
            created_at: Some("2026-10-28T03:30:00Z".to_string()),
            is_synthetic: true,
        });
        Ok(id)
    }

    fn update_patient(&self, id: i64, code: &str, name: &str) -> Result<(), PatientServiceError> {
        if code.trim().is_empty() || code.len() > 64 {
            return Err(PatientServiceError::InvalidInput(
                "Patient code must be 1-64 characters".to_string(),
            ));
        }
        if name.trim().is_empty() || name.len() > 200 {
            return Err(PatientServiceError::InvalidInput(
                "Display name must be 1-200 characters".to_string(),
            ));
        }

        let mut db = self
            .db
            .lock()
            .map_err(|e| PatientServiceError::Storage(e.to_string()))?;
        if let Some(p) = db.patients.iter_mut().find(|p| p.id == id) {
            p.patient_code = code.trim().to_string();
            p.display_name = name.trim().to_string();
            Ok(())
        } else {
            Err(PatientServiceError::NotFound(format!(
                "Patient {id} not found"
            )))
        }
    }

    fn add_examination(&self, exam: &ExaminationSummary) -> Result<i64, PatientServiceError> {
        let mut db = self
            .db
            .lock()
            .map_err(|e| PatientServiceError::Storage(e.to_string()))?;
        let id = db.next_exam_id;
        db.next_exam_id += 1;

        let mut new_exam = exam.clone();
        new_exam.id = id;
        new_exam.is_synthetic = true;
        db.examinations.push(new_exam);
        Ok(id)
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patient_service_query_filtering() {
        let service = MockPatientService::with_fixtures();
        let all = service.list_patients(&PatientQuery::default()).unwrap();
        assert_eq!(all.len(), 3);

        let filtered = service
            .list_patients(&PatientQuery {
                filter_text: "Alpha".to_string(),
                eye_filter: None,
            })
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].patient_code, "TEST-P001");

        let by_code = service
            .list_patients(&PatientQuery {
                filter_text: "P002".to_string(),
                eye_filter: None,
            })
            .unwrap();
        assert_eq!(by_code.len(), 1);
        assert_eq!(by_code[0].display_name, "Synthetic Patient Beta");
    }

    #[test]
    fn test_patient_service_empty_results() {
        let service = MockPatientService::with_fixtures();
        let res = service
            .list_patients(&PatientQuery {
                filter_text: "NonExistentIdentifierXYZ".to_string(),
                eye_filter: None,
            })
            .unwrap();
        assert!(res.is_empty());
    }

    #[test]
    fn test_patient_service_validation() {
        let service = MockPatientService::new();
        assert!(matches!(
            service.add_patient("", "Valid Name"),
            Err(PatientServiceError::InvalidInput(_))
        ));
        assert!(matches!(
            service.add_patient("CODE", ""),
            Err(PatientServiceError::InvalidInput(_))
        ));

        let id = service.add_patient("VALID-01", "Valid Patient").unwrap();
        assert_eq!(id, 1);

        // Duplicate code rejected
        assert!(matches!(
            service.add_patient("VALID-01", "Another Name"),
            Err(PatientServiceError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_mock_patient_service_crud() {
        let service = MockPatientService::new();
        let id = service.add_patient("P-100", "Alice").unwrap();
        let detail = service.get_patient(id).unwrap();
        assert_eq!(detail.patient_code, "P-100");
        assert_eq!(detail.display_name, "Alice");

        service
            .update_patient(id, "P-100-MOD", "Alice Smith")
            .unwrap();
        let updated = service.get_patient(id).unwrap();
        assert_eq!(updated.patient_code, "P-100-MOD");
        assert_eq!(updated.display_name, "Alice Smith");
    }

    #[test]
    fn test_examination_laterality_od_os_association() {
        let service = MockPatientService::with_fixtures();
        let exams = service.list_examinations(1).unwrap();
        assert_eq!(exams.len(), 2);
        assert_eq!(exams[0].eye, "OD");
        assert_eq!(exams[1].eye, "OS");
        assert_ne!(exams[0].id, exams[1].id);
    }

    #[test]
    fn test_live_service_database_locked_handling() {
        let service = LiveCodexPatientService::with_store(super::super::store::SharedStore::new(
            std::env::temp_dir().join("rc-locked-never-created/db.sqlite"),
        ));
        assert_eq!(service.readiness(), PatientServiceReadiness::DatabaseLocked);
        assert!(matches!(
            service.list_patients(&PatientQuery::default()),
            Err(PatientServiceError::DatabaseLocked)
        ));
        assert!(matches!(
            service.get_patient(1),
            Err(PatientServiceError::DatabaseLocked)
        ));
        assert!(matches!(
            service.add_patient("P", "N"),
            Err(PatientServiceError::DatabaseLocked)
        ));
    }

    fn open_service(name: &str) -> (LiveCodexPatientService, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "rc-live-{name}-{}-{}/db.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let service =
            LiveCodexPatientService::with_store(super::super::store::SharedStore::new(&path));
        service.begin_unlock("correct horse".into(), Some("correct horse".into()));
        let start = std::time::Instant::now();
        while service.readiness() != PatientServiceReadiness::Ready {
            assert!(start.elapsed().as_secs() < 60, "unlock timed out");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        (service, path)
    }

    // T32/T35 (service level): a saved explanation is listed, loads with the right identity, is refused for
    // another patient or eye, and becomes stale (refused) once the analysis is re-run.
    #[test]
    fn test_live_attribution_identity_and_staleness() {
        let (service, _path) = open_service("attr");
        let pid = service.add_patient("P-200", "Synthetic").unwrap();
        let exam_id = service
            .add_examination(&ExaminationSummary {
                id: 0,
                patient_id: pid,
                patient_code: String::new(),
                examined_at: String::new(),
                operator: "op".into(),
                notes: String::new(),
                eye: "OD".into(),
                image_path: "/data/img.png".into(),
                has_image: true,
                has_analysis: false,
                analysis_summary: None,
                is_synthetic: true,
            })
            .unwrap();
        let payload: Vec<u8> = (0..64u32).flat_map(|v| (v as f32).to_le_bytes()).collect();
        let digest: String = {
            use sha2::{Digest, Sha256};
            Sha256::digest(&payload)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        };
        let image = "ab".repeat(32);
        service
            .save_attribution(retinacare_persistence::AttributionInput {
                artifact_id: "art-1".into(),
                examination_id: exam_id,
                patient_id: pid,
                eye: "OD".into(),
                run_id: "7".into(),
                image_sha256: image.clone(),
                method: "occlusion_sensitivity".into(),
                target: "{}".into(),
                scope: "{}".into(),
                meta_json: "{}".into(),
                payload: payload.clone(),
                payload_sha256: digest.clone(),
            })
            .unwrap();
        assert_eq!(service.list_attributions(exam_id, pid).unwrap().len(), 1);
        let loaded = service
            .load_attribution("art-1", pid, "OD", &image)
            .unwrap();
        assert_eq!(loaded, Some(("{}".to_string(), payload, digest)));
        assert!(service
            .load_attribution("art-1", pid, "OS", &image)
            .is_err());
        assert!(service
            .load_attribution("art-1", pid + 1, "OD", &image)
            .is_err());
        service.mark_attributions_stale(exam_id);
        assert!(service
            .load_attribution("art-1", pid, "OD", &image)
            .is_err());
        assert_eq!(
            service.list_attributions(exam_id, pid).unwrap()[0].status,
            "stale"
        );
    }

    #[test]
    fn test_live_patient_exam_and_analysis_survive_reopen() {
        let (service, path) = open_service("persist");
        let pid = service.add_patient("P-100", "Alice Example").unwrap();
        let exam_id = service
            .add_examination(&ExaminationSummary {
                id: 0,
                patient_id: pid,
                patient_code: String::new(),
                examined_at: String::new(),
                operator: "op".into(),
                notes: "n".into(),
                eye: "OS".into(),
                image_path: "/data/img.png".into(),
                has_image: true,
                has_analysis: false,
                analysis_summary: None,
                is_synthetic: false,
            })
            .unwrap();
        assert!(service.load_analysis(exam_id).is_none());
        service
            .save_examination_analysis(exam_id, r#"{"not":"an outcome"}"#)
            .unwrap();
        service
            .update_patient(pid, "P-100", "Alice Edited")
            .unwrap();
        drop(service);

        let reopened =
            LiveCodexPatientService::with_store(super::super::store::SharedStore::new(&path));
        reopened.begin_unlock("correct horse".into(), None);
        let start = std::time::Instant::now();
        while reopened.readiness() != PatientServiceReadiness::Ready {
            assert!(start.elapsed().as_secs() < 60);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let patients = reopened.list_patients(&PatientQuery::default()).unwrap();
        assert_eq!(patients.len(), 1);
        assert_eq!(patients[0].display_name, "Alice Edited");
        assert!(!patients[0].is_synthetic);
        assert_eq!(patients[0].examination_count, 1);
        let exams = reopened.list_examinations(pid).unwrap();
        assert_eq!((exams[0].eye.as_str(), exams[0].has_analysis), ("OS", true));
        assert_eq!(exams[0].image_path, "/data/img.png");
        assert!(reopened.load_analysis(exam_id).is_some());
        assert!(matches!(
            reopened.list_examinations(pid + 7),
            Err(PatientServiceError::NotFound(_))
        ));
    }

    #[test]
    fn test_live_patient_filters_isolate_by_text_and_eye() {
        let (service, _path) = open_service("filter");
        let a = service.add_patient("A-1", "Alpha").unwrap();
        let _b = service.add_patient("B-2", "Bravo").unwrap();
        let mut exam = ExaminationSummary {
            id: 0,
            patient_id: a,
            patient_code: String::new(),
            examined_at: String::new(),
            operator: String::new(),
            notes: String::new(),
            eye: "OD".into(),
            image_path: String::new(),
            has_image: false,
            has_analysis: false,
            analysis_summary: None,
            is_synthetic: false,
        };
        service.add_examination(&exam).unwrap();
        let text = PatientQuery {
            filter_text: "brav".into(),
            eye_filter: None,
        };
        assert_eq!(service.list_patients(&text).unwrap().len(), 1);
        let od = PatientQuery {
            filter_text: String::new(),
            eye_filter: Some("OD".into()),
        };
        let hits = service.list_patients(&od).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].patient_code, "A-1");
        exam.eye = "OS".into();
        let os = PatientQuery {
            filter_text: String::new(),
            eye_filter: Some("OS".into()),
        };
        assert!(service.list_patients(&os).unwrap().is_empty());
        assert!(service.add_patient("A-1", "Duplicate").is_err());
    }
}
