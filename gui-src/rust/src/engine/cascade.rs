//! Real Stage I -> Stage II -> Stage III cascade adapter for the GUI.
//!
//! The backend (`retinacare_app::models::ModelService::analyze`) runs the models on its own worker
//! thread and reports through a separate `AnalysisEvent` channel. This module owns everything the GUI
//! needs around that: request identity, stale/out-of-order event rejection, per-stage progress,
//! identity verification of the finished report, and the plain-data outcome that the pages, the PDF
//! report and the saved examination all read. Nothing here runs inference or blocks the UI thread.

use crate::domain::EyeLaterality;
use retinacare_ai_bridge::{
    device::ActualProvider,
    stage1::Stage1GateDecision,
    stage2_contract::{
        EyeLaterality as BackendEye, MaskPixelData, Stage2MaskResult, Stage2ModelKind,
    },
    stage3_grading::Stage3Scope,
};
pub use retinacare_app::cascade::{
    AnalysisEvent, AnalysisPolicy, AnalysisReport, AnalysisRequest, AnalysisStage, AnalysisState,
    CascadeGrade, CascadeModelPaths,
};
use retinacare_app::{cascade::StageSummary, models::AnalysisSubmitError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Largest encoded image the backend accepts (`stage2_preprocessing::MAX_ENCODED_BYTES`).
pub const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;
/// Ids at or above this value are unsaved scratch examinations that are never written to the database.
pub const SCRATCH_EXAM_BASE: i64 = 1_000_000_000;

pub const RESEARCH_BANNER: &str = "SINGLE-FOLD TECHNICAL TEST (LG-DRG fold 0) - RESEARCH ONLY. \
Not a clinical grade; five-fold ensemble, licensed-image acceptance and clinical validation are pending.";
/// Shown only when all five official folds ran and were combined.
pub const RESEARCH_BANNER_ENSEMBLE: &str =
    "FIVE-FOLD ENSEMBLE (LG-DRG, official release v1.0) - RESEARCH ONLY. \
Not a clinical grade; licensed-image acceptance and clinical validation are pending.";
const ENSEMBLE_SCOPE_LABEL: &str = "five-fold ensemble";

// ---------------------------------------------------------------------------------------------
// Plain data shown by the pages and stored with the examination
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageKey {
    Screening,
    Ma,
    He,
    Ex,
    Cws,
    Vessel,
    Grading,
}

impl StageKey {
    pub const ALL: [StageKey; 7] = [
        StageKey::Screening,
        StageKey::Ma,
        StageKey::He,
        StageKey::Ex,
        StageKey::Cws,
        StageKey::Vessel,
        StageKey::Grading,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Screening => "Stage I screening gate",
            Self::Ma => "Stage II microaneurysms (MA)",
            Self::He => "Stage II hemorrhages (HE)",
            Self::Ex => "Stage II exudates (EX)",
            Self::Cws => "Stage II cotton-wool spots (CWS)",
            Self::Vessel => "Stage II vessels (SwinHRUNet++)",
            Self::Grading => "Stage III LG-DRG grading",
        }
    }

    pub fn from_stage(stage: AnalysisStage) -> Self {
        match stage {
            AnalysisStage::Screening => Self::Screening,
            AnalysisStage::Segmentation(Stage2ModelKind::Ma) => Self::Ma,
            AnalysisStage::Segmentation(Stage2ModelKind::He) => Self::He,
            AnalysisStage::Segmentation(Stage2ModelKind::Ex) => Self::Ex,
            AnalysisStage::Segmentation(Stage2ModelKind::Cws) => Self::Cws,
            AnalysisStage::Segmentation(Stage2ModelKind::Vessel) => Self::Vessel,
            AnalysisStage::Grading => Self::Grading,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageRunState {
    Pending,
    Running,
    Done,
    Skipped,
    Failed,
    Cancelled,
}

impl StageRunState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "waiting",
            Self::Running => "running",
            Self::Done => "done",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
            Self::Cancelled => "not run (cancelled)",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StageRow {
    pub key: StageKey,
    pub state: StageRunState,
    pub wall_ms: Option<u64>,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CascadePhase {
    #[default]
    Idle,
    /// Accepted by the backend, no stage has started yet.
    Queued,
    Running,
    /// Cancel was requested; stops at the next stage boundary (a running model call is not interrupted).
    Cancelling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomeState {
    Graded,
    Ungradable,
    NegativeRoute,
    Cancelled,
    Failed,
}

impl OutcomeState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Graded => "Completed - research-only grade",
            Self::Ungradable => "Completed - image ungradable",
            Self::NegativeRoute => "Completed - Stage I negative route",
            Self::Cancelled => "Cancelled - no grade",
            Self::Failed => "Failed - no grade",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreeningRow {
    #[serde(default)]
    pub model_version: String,
    #[serde(default)]
    pub elapsed_ms: u64,
    pub continue_cascade: bool,
    pub score: f32,
    pub threshold: f32,
    pub raw_logit: f32,
    pub model_id: String,
    pub model_sha256: String,
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaskRow {
    pub key: StageKey,
    pub width: u32,
    pub height: u32,
    pub foreground_pixels: u64,
    pub coverage_percent: f32,
    pub model_sha256: String,
    pub provider: String,
    pub wall_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradingRow {
    pub grade: u8,
    pub ungradable: bool,
    pub scope: String,
    pub gradable_probability: f64,
    pub ordinal_probabilities: [f64; 3],
    pub severity_logits: Option<[f32; 3]>,
    pub model_sha256: Vec<String>,
    pub provider: String,
    pub wall_ms: u64,
    pub research_only: bool,
}

/// The one typed result snapshot. Contains no pixel data, so it is small enough to store with the
/// examination and to print in the PDF report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CascadeOutcome {
    pub request_id: u64,
    pub examination_id: i64,
    pub patient_id: Option<i64>,
    pub eye: Option<String>,
    pub state: OutcomeState,
    pub headline: String,
    pub grade: Option<u8>,
    pub screening: Option<ScreeningRow>,
    pub masks: Vec<MaskRow>,
    pub grading: Option<GradingRow>,
    pub failed_stage: Option<String>,
    pub failure: Option<String>,
    /// Stage output kept after a failed or cancelled run: evidence for diagnosis, never a result.
    pub diagnostic_only: bool,
    pub stage_timings_ms: Vec<(String, u64)>,
    pub total_seconds: f64,
    pub peak_rss_mib: Option<u64>,
    /// Sampled by the app during the run; None when no NVIDIA GPU could be read.
    #[serde(default)]
    pub gpu: Option<crate::engine::gpu::GpuSample>,
    pub image_sha256: Option<String>,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
    pub identity_verified: bool,
    pub banner: String,
}

impl CascadeOutcome {
    /// True when the stored grade came from the full five-fold ensemble (not a single-fold test).
    pub fn is_ensemble(&self) -> bool {
        self.grading
            .as_ref()
            .is_some_and(|g| g.scope == ENSEMBLE_SCOPE_LABEL)
    }
}

/// Live state for the Report / Quad Viewer pages.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CascadeView {
    pub phase: CascadePhase,
    /// Backend can run the real cascade (all seven manifests present, service alive).
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub request_id: Option<u64>,
    pub stages: Vec<StageRow>,
    pub started: Option<Instant>,
    pub outcome: Option<CascadeOutcome>,
    /// A cancelled/abandoned run is still finishing in the backend; new runs wait for it.
    pub finishing_previous: bool,
    pub error: Option<String>,
    /// Whether the verified result was written to the encrypted database (or why it was not).
    pub save_note: Option<String>,
    /// The outcome was loaded from the saved examination, not computed in this session. Masks are
    /// stored only when `masks_restored` is true; otherwise the panels stay empty (see `mask_note`).
    pub restored: bool,
    /// The five masks of a restored examination were loaded and verified from the encrypted database.
    pub masks_restored: bool,
    /// Why masks could not be restored or saved (legacy examination, failed verification, I/O).
    pub mask_note: Option<String>,
    /// Stage III folds finished so far and the total (1 or 5), from real backend events.
    pub folds_done: Option<(u8, u8)>,
}

impl CascadeView {
    pub fn is_active(&self) -> bool {
        self.phase != CascadePhase::Idle
    }

    pub fn clear_run(&mut self) {
        self.phase = CascadePhase::Idle;
        self.request_id = None;
        self.stages.clear();
        self.started = None;
        self.outcome = None;
        self.error = None;
        self.save_note = None;
        self.restored = false;
        self.masks_restored = false;
        self.mask_note = None;
        self.folds_done = None;
    }

    pub fn begin(&mut self, request_id: u64) {
        self.clear_run();
        self.phase = CascadePhase::Queued;
        self.request_id = Some(request_id);
        self.started = Some(Instant::now());
        self.stages = StageKey::ALL
            .iter()
            .map(|key| StageRow {
                key: *key,
                state: StageRunState::Pending,
                wall_ms: None,
                detail: String::new(),
            })
            .collect();
    }

    fn row(&mut self, key: StageKey) -> Option<&mut StageRow> {
        self.stages.iter_mut().find(|row| row.key == key)
    }

    /// Apply a progress event that already passed identity routing.
    pub fn apply_progress(&mut self, event: &AnalysisEvent) {
        match event {
            AnalysisEvent::Started { .. } => {
                if self.phase == CascadePhase::Queued {
                    self.phase = CascadePhase::Running;
                }
            }
            AnalysisEvent::StageStarted { stage, .. } => {
                if self.phase == CascadePhase::Queued {
                    self.phase = CascadePhase::Running;
                }
                if let Some(row) = self.row(StageKey::from_stage(*stage)) {
                    row.state = StageRunState::Running;
                }
            }
            AnalysisEvent::StageCompleted { stage, summary, .. } => {
                let (wall_ms, detail) = summarise_stage(summary);
                if let Some(row) = self.row(StageKey::from_stage(*stage)) {
                    row.state = StageRunState::Done;
                    row.wall_ms = Some(wall_ms);
                    row.detail = detail;
                }
            }
            AnalysisEvent::StageSkipped { stage, .. } => {
                if let Some(row) = self.row(StageKey::from_stage(*stage)) {
                    row.state = StageRunState::Skipped;
                    row.detail = "skipped: Stage I negative route".to_string();
                }
            }
            AnalysisEvent::FoldCompleted { fold, folds, .. } => {
                self.folds_done = Some((fold + 1, *folds));
                if let Some(row) = self.row(StageKey::Grading) {
                    row.detail = format!("fold {} of {} done", fold + 1, folds);
                }
            }
            AnalysisEvent::Finished(_) => {}
        }
    }

    /// Terminal state: every row that never finished is marked from the outcome.
    pub fn finish(&mut self, outcome: CascadeOutcome) {
        let failed = outcome.failed_stage.clone();
        for row in &mut self.stages {
            match row.state {
                StageRunState::Running | StageRunState::Pending => {
                    row.state = match outcome.state {
                        OutcomeState::Cancelled => StageRunState::Cancelled,
                        OutcomeState::Failed if failed.as_deref() == Some(row.key.label()) => {
                            StageRunState::Failed
                        }
                        OutcomeState::Failed => StageRunState::Cancelled,
                        _ => StageRunState::Skipped,
                    };
                }
                _ => {}
            }
        }
        self.phase = CascadePhase::Idle;
        self.outcome = Some(outcome);
    }
}

fn summarise_stage(summary: &StageSummary) -> (u64, String) {
    match summary {
        StageSummary::Screening(s) => (
            (s.elapsed_microseconds / 1000) as u64,
            format!(
                "score {:.4} vs threshold {:.4} -> {}",
                s.screening_score,
                s.threshold,
                match s.decision {
                    Stage1GateDecision::NegativeRoute => "negative route (grade 0)",
                    Stage1GateDecision::ContinueCascade => "continue cascade",
                }
            ),
        ),
        StageSummary::Segmentation(m) => (
            (m.elapsed_microseconds / 1000) as u64,
            format!(
                "{} foreground pixels ({}x{})",
                m.foreground_pixels, m.width, m.height
            ),
        ),
        StageSummary::Grading(g) => (
            (g.elapsed_microseconds / 1000) as u64,
            if g.ungradable {
                "ungradable".to_string()
            } else {
                format!("technical grade {}", g.grade)
            },
        ),
    }
}

// ---------------------------------------------------------------------------------------------
// Request identity and event routing
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct JobIdentity {
    pub request_id: u64,
    pub examination_id: i64,
    pub patient_id: Option<i64>,
    pub eye: Option<BackendEye>,
    pub image_path: PathBuf,
    pub image_sha256: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// One heavy cascade at a time: a run (or a cancelled run still finishing) is in flight.
    Busy,
    NoImage,
    ImageUnreadable(String),
    ImageTooLarge,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Busy => write!(f, "An analysis is already running or still finishing."),
            Self::NoImage => write!(f, "Select a retinal image first."),
            Self::ImageUnreadable(why) => write!(f, "The image file cannot be read: {why}"),
            Self::ImageTooLarge => write!(f, "The image file exceeds the 64 MiB limit."),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Routed {
    /// Not for the active request (cancelled, abandoned, or unknown): dropped.
    Stale,
    /// Progress for the active request.
    Progress,
    /// Terminal event for the active request; carries its identity for verification.
    FinishedActive(Box<JobIdentity>),
    /// Terminal event of an abandoned request: the backend is free again.
    FinishedAbandoned,
}

#[derive(Debug, Default)]
pub struct CascadeController {
    next_id: u64,
    active: Option<JobIdentity>,
    draining: HashSet<u64>,
}

impl CascadeController {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            ..Self::default()
        }
    }

    pub fn is_busy(&self) -> bool {
        self.active.is_some() || !self.draining.is_empty()
    }

    pub fn finishing_previous(&self) -> bool {
        !self.draining.is_empty()
    }

    pub fn active(&self) -> Option<&JobIdentity> {
        self.active.as_ref()
    }

    /// Validate the image and create the request. The job becomes active only here; call
    /// `rollback` if the backend then refuses it.
    pub fn prepare(
        &mut self,
        patient_id: Option<i64>,
        examination_id: i64,
        eye: EyeLaterality,
        image_path: Option<&Path>,
        dimensions: Option<[usize; 2]>,
    ) -> Result<AnalysisRequest, Refusal> {
        if self.is_busy() {
            return Err(Refusal::Busy);
        }
        let path = image_path.ok_or(Refusal::NoImage)?;
        let size = std::fs::metadata(path)
            .map_err(|e| Refusal::ImageUnreadable(e.kind().to_string()))?
            .len();
        if size > MAX_IMAGE_BYTES {
            return Err(Refusal::ImageTooLarge);
        }
        let bytes =
            std::fs::read(path).map_err(|e| Refusal::ImageUnreadable(e.kind().to_string()))?;
        let [w, h] = dimensions.ok_or(Refusal::NoImage)?;
        let request_id = self.next_id;
        self.next_id += 1;
        let eye = backend_eye(eye);
        self.active = Some(JobIdentity {
            request_id,
            examination_id,
            patient_id,
            eye,
            image_path: path.to_path_buf(),
            image_sha256: hex(&Sha256::digest(&bytes)),
            width: w as u32,
            height: h as u32,
        });
        Ok(AnalysisRequest {
            request_id,
            examination_id,
            patient_id,
            eye,
            image_path: path.to_path_buf(),
            policy: AnalysisPolicy::TechnicalFold0,
        })
    }

    pub fn rollback(&mut self, request_id: u64) {
        if self.active.as_ref().map(|j| j.request_id) == Some(request_id) {
            self.active = None;
        }
    }

    /// The user switched patient, examination or image: stop showing and attaching this job. The backend
    /// is asked to cancel it; new runs wait until its terminal event arrives.
    pub fn abandon(&mut self) -> Option<u64> {
        let job = self.active.take()?;
        self.draining.insert(job.request_id);
        Some(job.request_id)
    }

    pub fn route(&mut self, event: &AnalysisEvent) -> Routed {
        let (id, terminal) = match event {
            AnalysisEvent::Started { request_id }
            | AnalysisEvent::StageStarted { request_id, .. }
            | AnalysisEvent::StageCompleted { request_id, .. }
            | AnalysisEvent::StageSkipped { request_id, .. }
            | AnalysisEvent::FoldCompleted { request_id, .. } => (*request_id, false),
            AnalysisEvent::Finished(report) => (report.request_id, true),
        };
        if self.active.as_ref().map(|j| j.request_id) == Some(id) {
            if terminal {
                return match self.active.take() {
                    Some(job) => Routed::FinishedActive(Box::new(job)),
                    None => Routed::Stale,
                };
            }
            return Routed::Progress;
        }
        if terminal && self.draining.remove(&id) {
            return Routed::FinishedAbandoned;
        }
        Routed::Stale
    }
}

fn backend_eye(eye: EyeLaterality) -> Option<BackendEye> {
    match eye {
        EyeLaterality::OD => Some(BackendEye::Right),
        EyeLaterality::OS => Some(BackendEye::Left),
        EyeLaterality::Unspecified => None,
    }
}

fn eye_label(eye: Option<BackendEye>) -> Option<String> {
    eye.map(|e| match e {
        BackendEye::Right => "OD".to_string(),
        BackendEye::Left => "OS".to_string(),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Check that a finished report (and each mask in it) belongs to the job the GUI submitted.
pub fn verify_identity(job: &JobIdentity, report: &AnalysisReport) -> Result<(), String> {
    if report.request_id != job.request_id {
        return Err("result belongs to a different request".into());
    }
    if report.examination_id != job.examination_id || report.patient_id != job.patient_id {
        return Err("result belongs to a different examination or patient".into());
    }
    if report.eye != job.eye {
        return Err("result belongs to the other eye".into());
    }
    let produced_output = matches!(
        report.state,
        AnalysisState::Graded | AnalysisState::Ungradable | AnalysisState::NegativeRoute
    );
    if produced_output {
        match &report.source_image_sha256 {
            Some(sha) if *sha == job.image_sha256 => {}
            _ => return Err("result was computed from a different image file".into()),
        }
        if report.image_width != Some(job.width) || report.image_height != Some(job.height) {
            return Err("result image size differs from the loaded image".into());
        }
    }
    for mask in &report.masks {
        if mask.examination_id != job.examination_id
            || mask.laterality != job.eye
            || mask.source_image_sha256 != job.image_sha256
        {
            return Err("a segmentation mask does not belong to this image or eye".into());
        }
        if mask.original_width != job.width
            || mask.original_height != job.height
            || mask.mask_width != job.width
            || mask.mask_height != job.height
        {
            return Err("a segmentation mask has the wrong frame size".into());
        }
    }
    Ok(())
}

fn provider_label(provider: ActualProvider) -> String {
    match provider {
        ActualProvider::Cpu => "CPU".to_string(),
        ActualProvider::Cuda { device_id } => format!("CUDA device {device_id}"),
    }
}

fn scope_label(scope: Stage3Scope) -> String {
    match scope {
        Stage3Scope::SingleFoldTechnicalTest { fold } => {
            format!("single-fold technical test (fold {fold})")
        }
        Stage3Scope::FiveFoldEnsemble => ENSEMBLE_SCOPE_LABEL.to_string(),
    }
}

/// Convert a finished backend report into the plain outcome. A failed identity check turns the whole
/// run into a failure without a grade, and the verified-only fields are dropped.
pub fn outcome_from_report(
    report: &AnalysisReport,
    identity: Result<(), String>,
) -> CascadeOutcome {
    let ensemble = matches!(
        report.grading.as_ref().map(|g| g.scope),
        Some(Stage3Scope::FiveFoldEnsemble)
    );
    let mut outcome = CascadeOutcome {
        request_id: report.request_id,
        examination_id: report.examination_id,
        patient_id: report.patient_id,
        eye: eye_label(report.eye),
        state: match report.state {
            AnalysisState::Graded => OutcomeState::Graded,
            AnalysisState::Ungradable => OutcomeState::Ungradable,
            AnalysisState::NegativeRoute => OutcomeState::NegativeRoute,
            AnalysisState::Cancelled => OutcomeState::Cancelled,
            AnalysisState::Failed => OutcomeState::Failed,
        },
        headline: String::new(),
        grade: None,
        screening: report.screening.as_ref().map(|s| ScreeningRow {
            model_version: s.version.clone(),
            elapsed_ms: (s.elapsed_microseconds / 1000) as u64,
            continue_cascade: s.gate.decision == Stage1GateDecision::ContinueCascade,
            score: s.gate.screening_score,
            threshold: s.gate.threshold,
            raw_logit: s.gate.raw_logit,
            model_id: s.model_id.clone(),
            model_sha256: s.sha256.clone(),
            provider: provider_label(s.provider),
        }),
        masks: report
            .masks
            .iter()
            .map(|m| {
                let pixels = match &m.data {
                    MaskPixelData::Binary(p) => p.iter().filter(|v| **v == 255).count() as u64,
                    MaskPixelData::Probability(_) => 0,
                };
                let total = (m.mask_width as u64 * m.mask_height as u64).max(1);
                MaskRow {
                    key: StageKey::from_stage(AnalysisStage::Segmentation(m.model_kind)),
                    width: m.mask_width,
                    height: m.mask_height,
                    foreground_pixels: pixels,
                    coverage_percent: pixels as f32 * 100.0 / total as f32,
                    model_sha256: m.model_sha256.clone(),
                    provider: provider_label(m.provider),
                    wall_ms: (m.elapsed_microseconds / 1000) as u64,
                }
            })
            .collect(),
        grading: report.grading.as_ref().map(|g| GradingRow {
            grade: g.grade,
            ungradable: g.ungradable,
            scope: scope_label(g.scope),
            gradable_probability: g.gradable_probability,
            ordinal_probabilities: g.ordinal_probabilities,
            severity_logits: g.severity_logits,
            model_sha256: g.model_sha256.clone(),
            provider: provider_label(g.provider),
            wall_ms: (g.elapsed_microseconds / 1000) as u64,
            research_only: g.research_only,
        }),
        failed_stage: report
            .failure
            .as_ref()
            .map(|f| StageKey::from_stage(f.stage).label().to_string()),
        failure: report.failure.as_ref().map(|f| f.message.to_string()),
        diagnostic_only: report.evidence_is_diagnostic_only,
        stage_timings_ms: report
            .timings
            .iter()
            .map(|t| {
                (
                    StageKey::from_stage(t.stage).label().to_string(),
                    (t.wall_microseconds / 1000) as u64,
                )
            })
            .collect(),
        total_seconds: report.total_microseconds as f64 / 1_000_000.0,
        peak_rss_mib: report.peak_rss_kib.map(|k| k / 1024),
        gpu: None,
        image_sha256: report.source_image_sha256.clone(),
        image_width: report.image_width,
        image_height: report.image_height,
        identity_verified: identity.is_ok(),
        banner: if ensemble {
            RESEARCH_BANNER_ENSEMBLE.to_string()
        } else {
            RESEARCH_BANNER.to_string()
        },
    };
    if let Err(reason) = identity {
        outcome.state = OutcomeState::Failed;
        outcome.failure = Some(format!("identity check failed: {reason}"));
        outcome.failed_stage = None;
        outcome.diagnostic_only = true;
        outcome.headline = "Failed - result rejected, no grade".to_string();
        return outcome;
    }
    match report.cascade_grade {
        Some(CascadeGrade::Grade0FromStage1Gate)
            if outcome.state == OutcomeState::NegativeRoute =>
        {
            outcome.grade = Some(0);
            outcome.headline =
                "Stage I negative route: grade 0 from the screening gate (Stage II/III skipped)"
                    .to_string();
        }
        Some(CascadeGrade::Severity(grade)) if outcome.state == OutcomeState::Graded => {
            outcome.grade = Some(grade);
            outcome.headline = if ensemble {
                format!("Five-fold ensemble severity grade {grade} (research only)")
            } else {
                format!("Single-fold technical severity grade {grade} (research only)")
            };
        }
        Some(CascadeGrade::Ungradable) if outcome.state == OutcomeState::Ungradable => {
            outcome.headline = "Ungradable image - no severity grade".to_string();
        }
        _ => {
            outcome.grade = None;
            // Failed / cancelled (or an inconsistent report): never carry a grade.
            if matches!(
                outcome.state,
                OutcomeState::Graded | OutcomeState::NegativeRoute | OutcomeState::Ungradable
            ) {
                outcome.state = OutcomeState::Failed;
                outcome.failure = Some("report state and cascade grade disagree".to_string());
                outcome.diagnostic_only = true;
            }
            outcome.headline = outcome.state.label().to_string();
        }
    }
    outcome
}

// ---------------------------------------------------------------------------------------------
// Mask pixels -> egui images
// ---------------------------------------------------------------------------------------------

pub fn mask_color(kind: Stage2ModelKind) -> [u8; 3] {
    match kind {
        Stage2ModelKind::Ma => [255, 70, 70],
        Stage2ModelKind::He => [255, 150, 30],
        Stage2ModelKind::Ex => [255, 235, 60],
        Stage2ModelKind::Cws => [130, 220, 255],
        Stage2ModelKind::Vessel => [70, 255, 140],
    }
}

/// RGBA image where only foreground pixels are painted. Returns None for a malformed mask.
pub fn mask_image(mask: &Stage2MaskResult) -> Option<eframe::egui::ColorImage> {
    let MaskPixelData::Binary(pixels) = &mask.data else {
        return None;
    };
    mask_image_from_pixels(
        mask.model_kind,
        mask.mask_width as usize,
        mask.mask_height as usize,
        pixels,
    )
}

/// Same painting for raw 0/255 pixels (used for masks restored from the encrypted database).
pub fn mask_image_from_pixels(
    kind: Stage2ModelKind,
    w: usize,
    h: usize,
    pixels: &[u8],
) -> Option<eframe::egui::ColorImage> {
    if pixels.len() != w.checked_mul(h)? || w == 0 || h == 0 {
        return None;
    }
    let [r, g, b] = mask_color(kind);
    let rgba: Vec<u8> = pixels
        .iter()
        .flat_map(|p| {
            if *p == 255 {
                [r, g, b, 255]
            } else {
                [0, 0, 0, 0]
            }
        })
        .collect();
    Some(eframe::egui::ColorImage::from_rgba_unmultiplied(
        [w, h],
        &rgba,
    ))
}

fn kind_label(kind: Stage2ModelKind) -> &'static str {
    match kind {
        Stage2ModelKind::Ma => "ma",
        Stage2ModelKind::He => "he",
        Stage2ModelKind::Ex => "ex",
        Stage2ModelKind::Cws => "cws",
        Stage2ModelKind::Vessel => "vessel",
    }
}

pub fn kind_from_label(label: &str) -> Option<Stage2ModelKind> {
    Some(match label {
        "ma" => Stage2ModelKind::Ma,
        "he" => Stage2ModelKind::He,
        "ex" => Stage2ModelKind::Ex,
        "cws" => Stage2ModelKind::Cws,
        "vessel" => Stage2ModelKind::Vessel,
        _ => return None,
    })
}

/// The five verified masks of a completed run as a storable set. The eye keeps the UI spelling
/// (`OD`/`OS`); the patient service converts it. Probability masks and incomplete sets are refused.
pub fn mask_run_from_report(
    report: &AnalysisReport,
) -> Result<retinacare_persistence::MaskRunInput, String> {
    let patient_id = report.patient_id.ok_or("the run has no patient")?;
    let source_image_sha256 = report
        .source_image_sha256
        .clone()
        .ok_or("the run has no source image hash")?;
    let (Some(image_width), Some(image_height)) = (report.image_width, report.image_height) else {
        return Err("the run has no image size".to_string());
    };
    let mut masks = Vec::new();
    for m in &report.masks {
        let MaskPixelData::Binary(pixels) = &m.data else {
            return Err("only binary masks can be stored".to_string());
        };
        masks.push(retinacare_persistence::StoredMask {
            kind: kind_label(m.model_kind).to_string(),
            width: m.mask_width,
            height: m.mask_height,
            model_version: m.model_version.clone(),
            model_sha256: m.model_sha256.clone(),
            checkpoint_sha256: m.checkpoint_sha256.clone(),
            preprocessing_version: m.preprocessing_version.clone(),
            threshold: m.threshold.value,
            provider: provider_label(m.provider),
            elapsed_microseconds: u64::try_from(m.elapsed_microseconds).unwrap_or(u64::MAX),
            transform_json: serde_json::to_string(&m.transform).map_err(|e| e.to_string())?,
            pixels: pixels.clone(),
        });
    }
    Ok(retinacare_persistence::MaskRunInput {
        examination_id: report.examination_id,
        patient_id,
        eye: eye_label(report.eye).unwrap_or_default(),
        request_id: report.request_id,
        source_image_sha256,
        image_width,
        image_height,
        masks,
    })
}

// ---------------------------------------------------------------------------------------------
// Backend adapter
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CascadeSubmitError {
    QueueFull,
    Stopped,
    Duplicate,
    Unavailable(String),
}

impl From<AnalysisSubmitError> for CascadeSubmitError {
    fn from(error: AnalysisSubmitError) -> Self {
        match error {
            AnalysisSubmitError::QueueFull => Self::QueueFull,
            AnalysisSubmitError::Stopped => Self::Stopped,
            AnalysisSubmitError::DuplicateRequest => Self::Duplicate,
        }
    }
}

impl std::fmt::Display for CascadeSubmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull => write!(f, "The analysis queue is full. Wait for the current run."),
            Self::Stopped => write!(f, "The analysis service has stopped."),
            Self::Duplicate => write!(f, "This request is already queued."),
            Self::Unavailable(why) => write!(f, "Full analysis unavailable: {why}"),
        }
    }
}

/// The seven manifests the cascade loads, derived from the Stage I manifest location
/// (`<models>/stage1_convnextv2l_768/manifest.json`). Reports every missing file.
pub fn discover_cascade_paths(stage1_manifest: &Path) -> Result<CascadeModelPaths, String> {
    let models = stage1_manifest
        .parent()
        .and_then(Path::parent)
        .ok_or("cannot derive the models directory")?;
    let paths = CascadeModelPaths {
        stage1: stage1_manifest.to_path_buf(),
        ma: models.join("stage2_hsmoe_aunet/ma/manifest.json"),
        he: models.join("stage2_hsmoe_aunet/he/manifest.json"),
        ex: models.join("stage2_hsmoe_aunet/ex/manifest.json"),
        cws: models.join("stage2_hsmoe_aunet/cws/manifest.json"),
        vessel: models.join("stage2_swinhrunetpp/manifest.json"),
        stage3_fold0: models.join("stage3_lgdrg/manifest_fold0.json"),
        // All four extra folds, or none: a partial set never turns into a partial ensemble.
        stage3_other_folds: {
            let extra: Vec<PathBuf> = (1..=4)
                .map(|k| models.join(format!("stage3_lgdrg/manifest_fold{k}.json")))
                .collect();
            if extra.iter().all(|p| p.is_file()) {
                extra
            } else {
                Vec::new()
            }
        },
    };
    let missing: Vec<String> = [
        &paths.stage1,
        &paths.ma,
        &paths.he,
        &paths.ex,
        &paths.cws,
        &paths.vessel,
        &paths.stage3_fold0,
    ]
    .iter()
    .filter(|p| !p.is_file())
    .map(|p| p.strip_prefix(models).unwrap_or(p).display().to_string())
    .collect();
    if missing.is_empty() {
        Ok(paths)
    } else {
        Err(format!("missing model manifests: {}", missing.join(", ")))
    }
}

/// Seconds elapsed since the run began, shown as a measured value (never an estimate).
pub fn elapsed_label(started: Option<Instant>) -> String {
    let elapsed = started.map(|s| s.elapsed()).unwrap_or(Duration::ZERO);
    format!("{}:{:02}", elapsed.as_secs() / 60, elapsed.as_secs() % 60)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use retinacare_ai_bridge::{
        device::ActualProvider,
        model_runtime::Stage1GateOutput,
        stage1::DecodedStage1Gate,
        stage2_contract::{
            ImageRect, Stage2ProcessingStatus, Stage2SpatialTransform, ThresholdMetadata,
        },
    };

    pub(crate) fn temp_image(name: &str) -> (PathBuf, String) {
        let path =
            std::env::temp_dir().join(format!("rc-cascade-{name}-{}.png", std::process::id()));
        std::fs::write(&path, b"fake image bytes").unwrap();
        let sha = hex(&Sha256::digest(b"fake image bytes"));
        (path, sha)
    }

    pub(crate) fn prepared(c: &mut CascadeController, name: &str) -> (AnalysisRequest, String) {
        let (path, sha) = temp_image(name);
        let req = c
            .prepare(Some(3), 7, EyeLaterality::OD, Some(&path), Some([64, 48]))
            .unwrap();
        (req, sha)
    }

    pub(crate) fn report(
        job: &JobIdentity,
        state: AnalysisState,
        grade: Option<CascadeGrade>,
    ) -> AnalysisReport {
        AnalysisReport {
            request_id: job.request_id,
            examination_id: job.examination_id,
            patient_id: job.patient_id,
            eye: job.eye,
            state,
            cascade_grade: grade,
            source_image_sha256: Some(job.image_sha256.clone()),
            image_width: Some(job.width),
            image_height: Some(job.height),
            screening: None,
            masks: vec![],
            grading: None,
            failure: None,
            cancelled_before: None,
            evidence_is_diagnostic_only: false,
            timings: vec![],
            total_microseconds: 1_500_000,
            peak_rss_kib: Some(2_048 * 1024),
            policy: AnalysisPolicy::TechnicalFold0,
            research_only: true,
            origin: "test",
            validation_status: "test",
        }
    }

    pub(crate) fn mask(job: &JobIdentity, kind: Stage2ModelKind) -> Stage2MaskResult {
        let pixels = vec![0_u8; (job.width * job.height) as usize];
        Stage2MaskResult {
            processing_status: Stage2ProcessingStatus::Complete,
            error: None,
            examination_id: job.examination_id,
            source_image_sha256: job.image_sha256.clone(),
            laterality: job.eye,
            model_kind: kind,
            model_version: "v".into(),
            checkpoint_sha256: "0".repeat(64),
            model_sha256: "0".repeat(64),
            original_width: job.width,
            original_height: job.height,
            mask_width: job.width,
            mask_height: job.height,
            transform: Stage2SpatialTransform::LesionCircleStretch {
                crop: ImageRect {
                    x: 0,
                    y: 0,
                    width: job.width,
                    height: job.height,
                },
                model_size: 1024,
            },
            preprocessing_version: "p".into(),
            threshold: ThresholdMetadata {
                value: kind.threshold(),
                operator: kind.threshold_operator(),
            },
            provider: ActualProvider::Cpu,
            elapsed_microseconds: 1000,
            data: MaskPixelData::Binary(pixels),
        }
    }

    #[test]
    fn one_heavy_job_at_a_time_and_no_duplicates() {
        let mut c = CascadeController::new();
        let (req, _) = prepared(&mut c, "busy");
        assert_eq!(req.request_id, 1);
        assert!(c.is_busy());
        let (path, _) = temp_image("busy2");
        assert_eq!(
            c.prepare(Some(3), 7, EyeLaterality::OD, Some(&path), Some([64, 48]))
                .unwrap_err(),
            Refusal::Busy
        );
    }

    #[test]
    fn prepare_rejects_missing_unreadable_and_oversized_input() {
        let mut c = CascadeController::new();
        assert_eq!(
            c.prepare(None, 1, EyeLaterality::OD, None, Some([1, 1]))
                .unwrap_err(),
            Refusal::NoImage
        );
        assert!(matches!(
            c.prepare(
                None,
                1,
                EyeLaterality::OD,
                Some(Path::new("/nonexistent/x.png")),
                Some([1, 1])
            ),
            Err(Refusal::ImageUnreadable(_))
        ));
        let (path, _) = temp_image("nodims");
        assert_eq!(
            c.prepare(None, 1, EyeLaterality::OD, Some(&path), None)
                .unwrap_err(),
            Refusal::NoImage
        );
        assert!(
            !c.is_busy(),
            "a refused request must not leave an active job"
        );
    }

    #[test]
    fn events_route_only_to_the_active_request_with_one_terminal() {
        let mut c = CascadeController::new();
        let (req, _) = prepared(&mut c, "route");
        let started = AnalysisEvent::Started {
            request_id: req.request_id,
        };
        assert_eq!(c.route(&started), Routed::Progress);
        assert_eq!(
            c.route(&AnalysisEvent::Started { request_id: 999 }),
            Routed::Stale
        );
        let job = c.active().unwrap().clone();
        let fin = AnalysisEvent::Finished(Box::new(report(
            &job,
            AnalysisState::NegativeRoute,
            Some(CascadeGrade::Grade0FromStage1Gate),
        )));
        assert!(matches!(c.route(&fin), Routed::FinishedActive(_)));
        assert_eq!(
            c.route(&fin),
            Routed::Stale,
            "a second terminal event is ignored"
        );
        assert!(!c.is_busy());
    }

    #[test]
    fn abandoned_job_blocks_new_runs_until_its_terminal_event() {
        let mut c = CascadeController::new();
        let (req, _) = prepared(&mut c, "abandon");
        let job = c.active().unwrap().clone();
        assert_eq!(c.abandon(), Some(req.request_id));
        assert!(c.active().is_none());
        assert!(c.finishing_previous() && c.is_busy());
        assert_eq!(
            c.route(&AnalysisEvent::StageStarted {
                request_id: req.request_id,
                stage: AnalysisStage::Screening
            }),
            Routed::Stale
        );
        let fin = AnalysisEvent::Finished(Box::new(report(&job, AnalysisState::Cancelled, None)));
        assert_eq!(c.route(&fin), Routed::FinishedAbandoned);
        assert!(!c.is_busy());
        let (path, _) = temp_image("after");
        assert!(c
            .prepare(Some(4), 8, EyeLaterality::OS, Some(&path), Some([64, 48]))
            .is_ok());
    }

    #[test]
    fn identity_mismatches_are_rejected_before_any_result_is_attached() {
        let mut c = CascadeController::new();
        let _ = prepared(&mut c, "ident");
        let job = c.active().unwrap().clone();
        let good = report(
            &job,
            AnalysisState::NegativeRoute,
            Some(CascadeGrade::Grade0FromStage1Gate),
        );
        assert!(verify_identity(&job, &good).is_ok());

        let mut wrong_exam = good.clone();
        wrong_exam.examination_id += 1;
        assert!(verify_identity(&job, &wrong_exam).is_err());

        let mut wrong_eye = good.clone();
        wrong_eye.eye = Some(BackendEye::Left);
        assert!(verify_identity(&job, &wrong_eye).is_err());

        let mut wrong_sha = good.clone();
        wrong_sha.source_image_sha256 = Some("f".repeat(64));
        assert!(verify_identity(&job, &wrong_sha).is_err());

        let mut wrong_dims = good.clone();
        wrong_dims.image_width = Some(job.width + 1);
        assert!(verify_identity(&job, &wrong_dims).is_err());

        let mut other_mask = good.clone();
        let mut m = mask(&job, Stage2ModelKind::Ma);
        m.laterality = Some(BackendEye::Left);
        other_mask.masks = vec![m];
        assert!(verify_identity(&job, &other_mask).is_err());

        let mut small_mask = good.clone();
        let mut m = mask(&job, Stage2ModelKind::Ma);
        m.mask_width -= 1;
        small_mask.masks = vec![m];
        assert!(verify_identity(&job, &small_mask).is_err());

        let outcome = outcome_from_report(&wrong_exam, verify_identity(&job, &wrong_exam));
        assert_eq!(outcome.state, OutcomeState::Failed);
        assert_eq!(outcome.grade, None);
        assert!(!outcome.identity_verified);
    }

    #[test]
    fn outcomes_never_carry_a_grade_unless_the_cascade_produced_one() {
        let mut c = CascadeController::new();
        let _ = prepared(&mut c, "outcome");
        let job = c.active().unwrap().clone();

        let neg = outcome_from_report(
            &report(
                &job,
                AnalysisState::NegativeRoute,
                Some(CascadeGrade::Grade0FromStage1Gate),
            ),
            Ok(()),
        );
        assert_eq!(
            (neg.state, neg.grade),
            (OutcomeState::NegativeRoute, Some(0))
        );
        assert!(neg.headline.contains("skipped"));

        let sev = outcome_from_report(
            &report(&job, AnalysisState::Graded, Some(CascadeGrade::Severity(3))),
            Ok(()),
        );
        assert_eq!((sev.state, sev.grade), (OutcomeState::Graded, Some(3)));
        assert!(sev.banner.contains("RESEARCH ONLY"));

        let ung = outcome_from_report(
            &report(
                &job,
                AnalysisState::Ungradable,
                Some(CascadeGrade::Ungradable),
            ),
            Ok(()),
        );
        assert_eq!((ung.state, ung.grade), (OutcomeState::Ungradable, None));
        assert!(ung.headline.starts_with("Ungradable"));

        let cancelled = outcome_from_report(
            &report(
                &job,
                AnalysisState::Cancelled,
                Some(CascadeGrade::Severity(4)),
            ),
            Ok(()),
        );
        assert_eq!(
            cancelled.grade, None,
            "a cancelled run must not publish a grade"
        );

        let mut failed = report(&job, AnalysisState::Failed, None);
        failed.failure = Some(retinacare_app::cascade::StageFailure {
            stage: AnalysisStage::Grading,
            kind: retinacare_app::cascade::StageFailureKind::Inference,
            message: retinacare_app::cascade::StageFailureKind::Inference.message(),
        });
        let failed = outcome_from_report(&failed, Ok(()));
        assert_eq!((failed.state, failed.grade), (OutcomeState::Failed, None));
        assert_eq!(
            failed.failed_stage.as_deref(),
            Some("Stage III LG-DRG grading")
        );

        let inconsistent = outcome_from_report(&report(&job, AnalysisState::Graded, None), Ok(()));
        assert_eq!(
            (inconsistent.state, inconsistent.grade),
            (OutcomeState::Failed, None)
        );
    }

    #[test]
    fn outcome_round_trips_through_json_without_pixel_data() {
        let mut c = CascadeController::new();
        let _ = prepared(&mut c, "json");
        let job = c.active().unwrap().clone();
        let mut r = report(
            &job,
            AnalysisState::NegativeRoute,
            Some(CascadeGrade::Grade0FromStage1Gate),
        );
        r.masks = vec![mask(&job, Stage2ModelKind::Ma)];
        let outcome = outcome_from_report(&r, Ok(()));
        let json = serde_json::to_string(&outcome).unwrap();
        assert!(
            json.len() < 4096,
            "summary must stay small, got {}",
            json.len()
        );
        assert_eq!(
            serde_json::from_str::<CascadeOutcome>(&json).unwrap(),
            outcome
        );
    }

    #[test]
    fn progress_view_tracks_stages_and_marks_unfinished_rows_on_cancel() {
        let mut c = CascadeController::new();
        let (req, _) = prepared(&mut c, "view");
        let job = c.active().unwrap().clone();
        let mut view = CascadeView::default();
        view.begin(req.request_id);
        assert_eq!(view.phase, CascadePhase::Queued);
        view.apply_progress(&AnalysisEvent::Started {
            request_id: req.request_id,
        });
        view.apply_progress(&AnalysisEvent::StageStarted {
            request_id: req.request_id,
            stage: AnalysisStage::Screening,
        });
        assert_eq!(view.phase, CascadePhase::Running);
        assert_eq!(view.stages[0].state, StageRunState::Running);
        let cancelled = outcome_from_report(&report(&job, AnalysisState::Cancelled, None), Ok(()));
        view.finish(cancelled);
        assert_eq!(view.phase, CascadePhase::Idle);
        assert!(view
            .stages
            .iter()
            .all(|r| r.state == StageRunState::Cancelled));
    }

    #[test]
    fn mask_image_paints_only_foreground_and_rejects_malformed_masks() {
        let mut c = CascadeController::new();
        let _ = prepared(&mut c, "img");
        let job = c.active().unwrap().clone();
        let mut m = mask(&job, Stage2ModelKind::Vessel);
        if let MaskPixelData::Binary(p) = &mut m.data {
            p[0] = 255;
        }
        let img = mask_image(&m).unwrap();
        assert_eq!(img.pixels[0].a(), 255);
        assert_eq!(img.pixels[1].a(), 0);
        m.mask_width += 1;
        assert!(mask_image(&m).is_none());
    }

    #[test]
    fn cascade_paths_report_every_missing_manifest() {
        let dir = std::env::temp_dir().join(format!("rc-models-{}", std::process::id()));
        let stage1 = dir.join("stage1_convnextv2l_768/manifest.json");
        std::fs::create_dir_all(stage1.parent().unwrap()).unwrap();
        std::fs::write(&stage1, "{}").unwrap();
        let err = discover_cascade_paths(&stage1).unwrap_err();
        assert!(err.contains("stage2_hsmoe_aunet/ma/manifest.json"));
        assert!(err.contains("stage3_lgdrg/manifest_fold0.json"));
        assert!(!err.contains("stage1_convnextv2l_768"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Heavy end-to-end: GUI adapter + real `ModelService` + all seven real models on CPU.
    /// Run: cargo test live_cascade -- --ignored --nocapture
    #[test]
    #[ignore = "loads seven real ONNX models (~2.2 GB RSS, several minutes on CPU)"]
    fn live_cascade_runs_through_the_gui_adapter() {
        use crate::engine::{
            discover_default_manifest, LiveCodexBackendService, MedicalInferenceService,
        };
        let manifest = discover_default_manifest();
        let image = manifest
            .ancestors()
            .nth(3)
            .unwrap()
            .join("dist/demo-images/session-1-final/OD-1.png");
        let service = LiveCodexBackendService::new(manifest);
        service
            .cascade_status()
            .expect("all seven manifests present");
        let mut ctl = CascadeController::new();
        let request = ctl
            .prepare(
                Some(1),
                42,
                EyeLaterality::OD,
                Some(&image),
                Some([640, 640]),
            )
            .unwrap();
        let id = request.request_id;
        service.submit_cascade(request).unwrap();
        // A second submit while one is active must be refused before reaching the backend.
        assert_eq!(
            ctl.prepare(
                Some(1),
                42,
                EyeLaterality::OD,
                Some(&image),
                Some([640, 640])
            )
            .unwrap_err(),
            Refusal::Busy
        );
        let mut view = CascadeView::default();
        view.begin(id);
        let started = Instant::now();
        let mut trace = Vec::new();
        let mut finished = None;
        while started.elapsed() < Duration::from_secs(900) && finished.is_none() {
            while let Some(event) = service.try_recv_cascade() {
                trace.push(format!(
                    "{:>6.1}s {}",
                    started.elapsed().as_secs_f32(),
                    match &event {
                        AnalysisEvent::Started { .. } => "Started".to_string(),
                        AnalysisEvent::StageStarted { stage, .. } =>
                            format!("StageStarted {stage:?}"),
                        AnalysisEvent::StageCompleted { stage, .. } =>
                            format!("StageCompleted {stage:?}"),
                        AnalysisEvent::StageSkipped { stage, .. } =>
                            format!("StageSkipped {stage:?}"),
                        AnalysisEvent::FoldCompleted { fold, folds, .. } =>
                            format!("FoldCompleted {fold}/{folds}"),
                        AnalysisEvent::Finished(r) => format!("Finished {:?}", r.state),
                    }
                ));
                match ctl.route(&event) {
                    Routed::Progress => view.apply_progress(&event),
                    Routed::FinishedActive(job) => {
                        if let AnalysisEvent::Finished(report) = event {
                            finished = Some((*job, report));
                        }
                    }
                    other => panic!("unexpected routing {other:?}"),
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        println!("--- event trace ---\n{}", trace.join("\n"));
        let (job, report) = finished.expect("a Finished event within 15 minutes");
        let identity = verify_identity(&job, &report);
        let outcome = outcome_from_report(&report, identity);
        println!(
            "outcome: {} | total {:.1}s | rss {:?} MiB",
            outcome.headline, outcome.total_seconds, outcome.peak_rss_mib
        );
        assert!(outcome.identity_verified);
        assert!(matches!(
            outcome.state,
            OutcomeState::Graded | OutcomeState::Ungradable | OutcomeState::NegativeRoute
        ));
        if outcome.state != OutcomeState::NegativeRoute {
            assert_eq!(outcome.masks.len(), 5);
            for m in &report.masks {
                assert!(mask_image(m).is_some());
            }
            assert!(outcome.grading.as_ref().unwrap().research_only);
            persist_and_reload_real_masks(&report);
        }
        view.finish(outcome);
        assert!(!ctl.is_busy());
    }

    /// Real five-mask roundtrip through the encrypted database: save, close, reopen, compare pixels.
    fn persist_and_reload_real_masks(report: &AnalysisReport) {
        use retinacare_ai_bridge::stage2_contract::MaskPixelData;
        use retinacare_persistence::{MaskExpectation, SqlCipherStore};
        let dir = std::env::temp_dir().join(format!("rc-real-masks-{}", std::process::id()));
        let path = dir.join("db.sqlite");
        let mut run = mask_run_from_report(report).expect("real masks convert");
        let (pid, eid) = {
            let store = SqlCipherStore::open(&path, "real-mask-test").unwrap();
            let pid = store.add_patient("P-REAL", "Synthetic").unwrap();
            let eid = store.add_examination(pid, "op", "").unwrap();
            store.set_examination_image(eid, "right", "demo").unwrap();
            run.patient_id = pid;
            run.examination_id = eid;
            run.eye = "right".to_string();
            let started = Instant::now();
            store.save_mask_run(&run).unwrap();
            println!("REAL-MASKS save {:?}", started.elapsed());
            (pid, eid)
        };
        let store = SqlCipherStore::open(&path, "real-mask-test").unwrap();
        let started = Instant::now();
        let loaded = store
            .load_mask_run(
                eid,
                &MaskExpectation {
                    patient_id: pid,
                    eye: "right".to_string(),
                    source_image_sha256: report.source_image_sha256.clone(),
                },
            )
            .unwrap()
            .expect("stored run");
        println!("REAL-MASKS load+verify {:?}", started.elapsed());
        assert_eq!(loaded.input.masks.len(), 5);
        for original in &report.masks {
            let MaskPixelData::Binary(expected) = &original.data else {
                panic!("binary mask expected");
            };
            let stored = loaded
                .input
                .masks
                .iter()
                .find(|m| m.kind == kind_label(original.model_kind))
                .expect("stored kind");
            assert_eq!(&stored.pixels, expected, "{:?}", original.model_kind);
            println!(
                "REAL-MASKS {:?} {}x{} foreground {}",
                original.model_kind,
                stored.width,
                stored.height,
                stored.pixels.iter().filter(|p| **p == 255).count()
            );
        }
        let size = std::fs::metadata(&path).unwrap().len();
        println!("REAL-MASKS database file {size} bytes for 5 masks");
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[allow(dead_code)]
    fn _gate_output_type_is_stable(_: Stage1GateOutput, _: DecodedStage1Gate) {}

    fn grading_result(
        scope: Stage3Scope,
        hashes: usize,
    ) -> retinacare_ai_bridge::stage3_grading::Stage3GradeResult {
        use retinacare_ai_bridge::stage3_grading::{
            Stage3GradeResult, CORN_THRESHOLDS, FOLD_CHECKPOINT_SHA256, GRADABILITY_THRESHOLD,
        };
        Stage3GradeResult {
            examination_id: 7,
            source_image_sha256: "a".repeat(64),
            scope,
            model_version: "1.0.0".into(),
            checkpoint_sha256: FOLD_CHECKPOINT_SHA256[..hashes]
                .iter()
                .map(|h| h.to_string())
                .collect(),
            model_sha256: (0..hashes).map(|i| format!("{i:064x}")).collect(),
            severity_logits: None,
            gradability_logit: None,
            full_probabilities: [0.1, 0.2, 0.5, 0.1, 0.1],
            ordinal_probabilities: [0.7, 0.8, 0.1],
            gradable_probability: 0.9,
            corn_thresholds: CORN_THRESHOLDS,
            gradability_threshold: GRADABILITY_THRESHOLD,
            grade: 3,
            ungradable: false,
            provider: ActualProvider::Cpu,
            elapsed_microseconds: 5_000_000,
            research_only: true,
        }
    }

    fn graded_outcome(scope: Stage3Scope, hashes: usize) -> CascadeOutcome {
        let job = JobIdentity {
            request_id: 1,
            examination_id: 7,
            patient_id: Some(3),
            eye: Some(retinacare_ai_bridge::stage2_contract::EyeLaterality::Right),
            image_path: PathBuf::from("/unused.png"),
            image_sha256: "a".repeat(64),
            width: 64,
            height: 48,
        };
        let mut r = report(&job, AnalysisState::Graded, Some(CascadeGrade::Severity(3)));
        r.grading = Some(grading_result(scope, hashes));
        outcome_from_report(&r, Ok(()))
    }

    #[test]
    fn five_fold_result_gets_the_ensemble_banner_headline_and_scope_and_is_not_called_single_fold()
    {
        let outcome = graded_outcome(Stage3Scope::FiveFoldEnsemble, 5);
        assert!(outcome.is_ensemble());
        assert_eq!(outcome.banner, RESEARCH_BANNER_ENSEMBLE);
        assert!(outcome
            .headline
            .starts_with("Five-fold ensemble severity grade 3"));
        assert!(!outcome.banner.contains("SINGLE-FOLD"));
        let grading = outcome.grading.as_ref().unwrap();
        assert_eq!(grading.scope, "five-fold ensemble");
        assert_eq!(
            grading.model_sha256.len(),
            5,
            "all five model hashes are kept"
        );
        assert!(grading.research_only);
    }

    #[test]
    fn a_historic_single_fold_result_keeps_its_single_fold_research_label() {
        let outcome = graded_outcome(Stage3Scope::SingleFoldTechnicalTest { fold: 0 }, 1);
        assert!(!outcome.is_ensemble());
        assert_eq!(outcome.banner, RESEARCH_BANNER);
        assert!(outcome
            .headline
            .starts_with("Single-fold technical severity grade 3"));
        assert_eq!(
            outcome.grading.as_ref().unwrap().scope,
            "single-fold technical test (fold 0)"
        );
        // A stored summary written before the ensemble existed still decodes and stays single-fold.
        let json = serde_json::to_string(&outcome).unwrap();
        let restored: CascadeOutcome = serde_json::from_str(&json).unwrap();
        assert!(!restored.is_ensemble() && restored.banner == RESEARCH_BANNER);
    }
}
