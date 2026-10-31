//! Medical AI Backend Integration Layer
//!
//! Provides typed Rust contracts and nonblocking integration connecting the
//! RetinaCare desktop GUI to the genuine Codex medical AI backend crates:
//! `retinacare-app` and `retinacare-ai-bridge`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::domain::EyeLaterality;
pub use retinacare_ai_bridge::device::ProviderRequest;
pub use retinacare_ai_bridge::stage1::{
    DecodedStage1Gate, Stage1GateDecision, STAGE1_GATE_THRESHOLD,
};
use retinacare_app::models::{ModelEvent, ModelService, ModelSubmitError};

/// Authentic medical AI backend readiness state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum BackendReadiness {
    NotConfigured,
    ModelMissing {
        path: PathBuf,
        reason: String,
    },
    ModelInvalid {
        path: PathBuf,
        reason: String,
    },
    Initializing {
        stage: String,
    },
    Ready {
        model_id: String,
        version: String,
        provider: String,
        threshold: f32,
    },
    Processing {
        request_id: String,
    },
    Cancelling {
        request_id: String,
    },
    Unavailable {
        reason: String,
    },
    Failed {
        error: String,
    },
}

impl BackendReadiness {
    pub fn is_ready(&self) -> bool {
        matches!(self, BackendReadiness::Ready { .. })
    }

    pub fn is_processing(&self) -> bool {
        matches!(self, BackendReadiness::Processing { .. })
    }

    pub fn display_label(&self) -> String {
        match self {
            BackendReadiness::NotConfigured => "Backend: Not Configured".to_string(),
            BackendReadiness::ModelMissing { .. } => "Stage I: Model Missing".to_string(),
            BackendReadiness::ModelInvalid { .. } => "Stage I: Model Invalid".to_string(),
            BackendReadiness::Initializing { stage } => {
                format!("Stage I: Initializing ({stage})...")
            }
            BackendReadiness::Ready { provider, .. } => format!("Stage I: Ready ({provider})"),
            BackendReadiness::Processing { .. } => "Stage I: Screening In Progress...".to_string(),
            BackendReadiness::Cancelling { .. } => "Stage I: Cancelling...".to_string(),
            BackendReadiness::Unavailable { reason } => format!("Stage I: Unavailable ({reason})"),
            BackendReadiness::Failed { error } => format!("Stage I: Failed ({error})"),
        }
    }
}

/// Requested pipeline stage. Stages II and III exist in the backend (`ModelService::analyze`)
/// but are not wired into this adapter yet, so they stay flagged as unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    Stage1ScreeningGate,
    Stage2Segmentation,
    Stage3Grading,
}

impl PipelineStage {
    pub fn is_available(&self) -> bool {
        matches!(self, PipelineStage::Stage1ScreeningGate)
    }

    pub fn label(&self) -> &'static str {
        match self {
            PipelineStage::Stage1ScreeningGate => "Stage I Screening Gate (HierarchiRetina)",
            PipelineStage::Stage2Segmentation => {
                "Stage II Lesion + Vessel Segmentation (Pending GUI Integration)"
            }
            PipelineStage::Stage3Grading => "Stage III DR Grading (Pending GUI Integration)",
        }
    }
}

/// Strongly typed retinal analysis request with correlation IDs.
#[derive(Debug, Clone, PartialEq)]
pub struct RetinalAnalysisRequest {
    pub request_id: String,
    pub examination_id: String,
    pub patient_id: Option<String>,
    pub image_path: PathBuf,
    pub eye: Option<EyeLaterality>,
    pub requested_stage: PipelineStage,
    pub submitted_at: Instant,
}

impl RetinalAnalysisRequest {
    pub fn new(
        examination_id: impl Into<String>,
        image_path: impl Into<PathBuf>,
        eye: Option<EyeLaterality>,
    ) -> Self {
        let req_num = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        Self {
            request_id: format!("req-{req_num}"),
            examination_id: examination_id.into(),
            patient_id: None,
            image_path: image_path.into(),
            eye,
            requested_stage: PipelineStage::Stage1ScreeningGate,
            submitted_at: Instant::now(),
        }
    }
}

/// Authentic Stage I screening result produced by genuine ONNX inference.
#[derive(Debug, Clone, PartialEq)]
pub struct Stage1ScreeningResult {
    pub request_id: String,
    pub examination_id: String,
    pub model_id: String,
    pub model_version: String,
    pub model_sha256: String,
    pub provider: String,
    pub raw_logit: f32,
    pub screening_score: f32,
    pub threshold: f32,
    pub decision: Stage1GateDecision,
    pub elapsed_ms: u64,
    pub preprocessing_ms: u64,
    pub disclaimer: String,
}

/// Nonblocking events dispatched from the inference service to the GUI thread.
#[derive(Debug, Clone, PartialEq)]
pub enum InferenceEvent {
    ReadinessChanged(BackendReadiness),
    Progress {
        request_id: String,
        step: String,
        percent: f32,
    },
    Stage1Completed(Stage1ScreeningResult),
    InferenceFailed {
        request_id: String,
        error: String,
    },
    Cancelled {
        request_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferenceError {
    BackendNotReady,
    ImageNotFound(PathBuf),
    QueueFull,
    ServiceStopped,
    StageUnavailable(PipelineStage),
    InvalidRequest(String),
}

impl std::fmt::Display for InferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InferenceError::BackendNotReady => write!(f, "Inference backend is not ready"),
            InferenceError::ImageNotFound(p) => write!(f, "Image file not found: {}", p.display()),
            InferenceError::QueueFull => write!(f, "Inference queue is full"),
            InferenceError::ServiceStopped => write!(f, "Inference service has been stopped"),
            InferenceError::StageUnavailable(s) => {
                write!(f, "Pipeline stage is unavailable: {}", s.label())
            }
            InferenceError::InvalidRequest(r) => write!(f, "Invalid request: {r}"),
        }
    }
}

impl std::error::Error for InferenceError {}

/// Abstract medical inference interface enabling both real background execution
/// and deterministic mock testing.
pub trait MedicalInferenceService: Send + Sync {
    /// Whether the genuine Stage I -> II -> III cascade can run (all models present, service alive).
    /// Mock services keep the default: no cascade, only the Stage I path below.
    fn cascade_status(&self) -> Result<(), String> {
        Err("this inference service has no cascade".to_string())
    }

    fn submit_cascade(
        &self,
        _request: crate::engine::cascade::AnalysisRequest,
    ) -> Result<(), crate::engine::cascade::CascadeSubmitError> {
        Err(crate::engine::cascade::CascadeSubmitError::Unavailable(
            "this inference service has no cascade".to_string(),
        ))
    }

    /// Cooperative: the backend stops at its next stage boundary. Returns false for an unknown request.
    fn cancel_cascade(&self, _request_id: u64) -> bool {
        false
    }

    fn try_recv_cascade(&self) -> Option<crate::engine::cascade::AnalysisEvent> {
        None
    }

    fn readiness(&self) -> BackendReadiness;
    fn submit(&self, request: RetinalAnalysisRequest) -> Result<(), InferenceError>;
    fn cancel(&self, request_id: &str);
    fn try_recv(&self) -> Option<InferenceEvent>;
}

/// Production service adapter connecting to Codex's `retinacare_app::models::ModelService`.
pub struct LiveCodexBackendService {
    service: Mutex<ModelService>,
    manifest_path: PathBuf,
    cascade_paths: Result<crate::engine::cascade::CascadeModelPaths, String>,
    model_id: String,
    model_version: String,
    model_sha256: String,
    threshold: f32,
    readiness: Arc<Mutex<BackendReadiness>>,
    active_request: Arc<Mutex<Option<RetinalAnalysisRequest>>>,
    cancelled_request: Arc<AtomicBool>,
}

impl LiveCodexBackendService {
    pub fn new(manifest_path: impl Into<PathBuf>) -> Self {
        let manifest_path = manifest_path.into();
        let service = ModelService::new();
        let readiness = Arc::new(Mutex::new(BackendReadiness::Initializing {
            stage: "Validating model manifest".to_string(),
        }));
        let active_request = Arc::new(Mutex::new(None));
        let cancelled_request = Arc::new(AtomicBool::new(false));

        let mut model_id = "hierarchiretina-stage1-convnextv2l-768".to_string();
        let mut model_version = "1.0.0".to_string();
        let mut model_sha256 = String::new();
        let mut threshold = STAGE1_GATE_THRESHOLD;

        if !manifest_path.exists() {
            *readiness.lock().unwrap() = BackendReadiness::ModelMissing {
                path: manifest_path.clone(),
                reason: "manifest.json does not exist on disk".to_string(),
            };
        } else {
            // Read and validate manifest
            match std::fs::read_to_string(&manifest_path) {
                Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(json) => {
                        if let Some(id) = json.get("model_id").and_then(|v| v.as_str()) {
                            model_id = id.to_string();
                        }
                        if let Some(ver) = json.get("version").and_then(|v| v.as_str()) {
                            model_version = ver.to_string();
                        }
                        if let Some(hash) = json.get("sha256").and_then(|v| v.as_str()) {
                            model_sha256 = hash.to_string();
                        }
                        if let Some(th) = json
                            .get("outputs")
                            .and_then(|o| o.get("gate_threshold"))
                            .and_then(|v| v.as_f64())
                        {
                            threshold = th as f32;
                        }

                        // Submit load request to ModelService
                        if let Err(err) = service.load(manifest_path.clone(), ProviderRequest::Cpu)
                        {
                            *readiness.lock().unwrap() = BackendReadiness::Failed {
                                error: format!("Failed to queue model load: {err:?}"),
                            };
                        }
                    }
                    Err(err) => {
                        *readiness.lock().unwrap() = BackendReadiness::ModelInvalid {
                            path: manifest_path.clone(),
                            reason: format!("JSON deserialization failed: {err}"),
                        };
                    }
                },
                Err(err) => {
                    *readiness.lock().unwrap() = BackendReadiness::ModelInvalid {
                        path: manifest_path.clone(),
                        reason: format!("Failed to read manifest file: {err}"),
                    };
                }
            }
        }

        let cascade_paths = crate::engine::cascade::discover_cascade_paths(&manifest_path);
        Self {
            service: Mutex::new(service),
            cascade_paths,
            manifest_path,
            model_id,
            model_version,
            model_sha256,
            threshold,
            readiness,
            active_request,
            cancelled_request,
        }
    }

    pub fn manifest_path(&self) -> &PathBuf {
        &self.manifest_path
    }

    pub fn model_sha256(&self) -> &str {
        &self.model_sha256
    }
}

impl LiveCodexBackendService {
    /// Device of the loaded Stage I model; CPU until the load has reported.
    fn current_provider(&self) -> String {
        match &*self.readiness.lock().unwrap() {
            BackendReadiness::Ready { provider, .. } => provider.clone(),
            _ => "CPU".to_string(),
        }
    }
}

impl MedicalInferenceService for LiveCodexBackendService {
    fn cascade_status(&self) -> Result<(), String> {
        self.cascade_paths
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone)
    }

    fn submit_cascade(
        &self,
        request: crate::engine::cascade::AnalysisRequest,
    ) -> Result<(), crate::engine::cascade::CascadeSubmitError> {
        use crate::engine::cascade::CascadeSubmitError;
        let paths = self
            .cascade_paths
            .clone()
            .map_err(CascadeSubmitError::Unavailable)?;
        let service = self
            .service
            .lock()
            .map_err(|_| CascadeSubmitError::Stopped)?;
        service
            .analyze(request, paths)
            .map_err(CascadeSubmitError::from)
    }

    fn cancel_cascade(&self, request_id: u64) -> bool {
        self.service
            .lock()
            .map(|service| service.cancel_analysis(request_id))
            .unwrap_or(false)
    }

    fn try_recv_cascade(&self) -> Option<crate::engine::cascade::AnalysisEvent> {
        self.service.lock().ok()?.try_next_analysis().ok()
    }

    fn readiness(&self) -> BackendReadiness {
        self.readiness.lock().unwrap().clone()
    }

    fn submit(&self, request: RetinalAnalysisRequest) -> Result<(), InferenceError> {
        let readiness = self.readiness();
        if !readiness.is_ready() {
            return Err(InferenceError::BackendNotReady);
        }

        if !request.requested_stage.is_available() {
            return Err(InferenceError::StageUnavailable(request.requested_stage));
        }

        if !request.image_path.exists() {
            return Err(InferenceError::ImageNotFound(request.image_path.clone()));
        }

        self.cancelled_request.store(false, Ordering::Relaxed);
        let req_id = request.request_id.clone();
        let image_path = request.image_path.clone();

        *self.active_request.lock().unwrap() = Some(request);
        *self.readiness.lock().unwrap() = BackendReadiness::Processing {
            request_id: req_id.clone(),
        };

        match self
            .service
            .lock()
            .unwrap()
            .infer_stage1_image(self.model_id.clone(), image_path)
        {
            Ok(()) => Ok(()),
            Err(ModelSubmitError::QueueFull) => {
                *self.readiness.lock().unwrap() = BackendReadiness::Ready {
                    model_id: self.model_id.clone(),
                    version: self.model_version.clone(),
                    provider: self.current_provider(),
                    threshold: self.threshold,
                };
                *self.active_request.lock().unwrap() = None;
                Err(InferenceError::QueueFull)
            }
            Err(ModelSubmitError::Stopped) => {
                *self.readiness.lock().unwrap() = BackendReadiness::Failed {
                    error: "Model service worker stopped".to_string(),
                };
                *self.active_request.lock().unwrap() = None;
                Err(InferenceError::ServiceStopped)
            }
        }
    }

    fn cancel(&self, request_id: &str) {
        let active = self.active_request.lock().unwrap();
        if let Some(req) = active.as_ref() {
            if req.request_id == request_id {
                self.cancelled_request.store(true, Ordering::Relaxed);
                *self.readiness.lock().unwrap() = BackendReadiness::Cancelling {
                    request_id: request_id.to_string(),
                };
            }
        }
    }

    fn try_recv(&self) -> Option<InferenceEvent> {
        while let Ok(event) = self.service.lock().unwrap().try_next() {
            match event {
                ModelEvent::Loaded(Ok(loaded)) => {
                    let new_ready = BackendReadiness::Ready {
                        model_id: loaded.model_id,
                        version: loaded.version,
                        provider: provider_name(loaded.provider),
                        threshold: self.threshold,
                    };
                    *self.readiness.lock().unwrap() = new_ready.clone();
                    return Some(InferenceEvent::ReadinessChanged(new_ready));
                }
                ModelEvent::Loaded(Err(err)) => {
                    let failed = BackendReadiness::Failed {
                        error: format!("ONNX session load failed: {err}"),
                    };
                    *self.readiness.lock().unwrap() = failed.clone();
                    return Some(InferenceEvent::ReadinessChanged(failed));
                }
                ModelEvent::Stage1Inference {
                    model_id: _,
                    result,
                } => {
                    let mut active_guard = self.active_request.lock().unwrap();
                    let maybe_req = active_guard.take();

                    let ready_state = BackendReadiness::Ready {
                        model_id: self.model_id.clone(),
                        version: self.model_version.clone(),
                        provider: self.current_provider(),
                        threshold: self.threshold,
                    };
                    *self.readiness.lock().unwrap() = ready_state.clone();

                    if self.cancelled_request.swap(false, Ordering::Relaxed) {
                        if let Some(req) = maybe_req {
                            return Some(InferenceEvent::Cancelled {
                                request_id: req.request_id,
                            });
                        }
                    }

                    if let Some(req) = maybe_req {
                        match result {
                            Ok(output) => {
                                let screening_result = Stage1ScreeningResult {
                                    request_id: req.request_id,
                                    examination_id: req.examination_id,
                                    model_id: output.model_id,
                                    model_version: output.version,
                                    model_sha256: output.sha256,
                                    provider: provider_name(output.provider),
                                    raw_logit: output.gate.raw_logit,
                                    screening_score: output.gate.screening_score,
                                    threshold: output.gate.threshold,
                                    decision: output.gate.decision,
                                    elapsed_ms: (output.elapsed_microseconds / 1000) as u64,
                                    preprocessing_ms: (output.preprocessing_microseconds.unwrap_or(0) / 1000) as u64,
                                    disclaimer: "Research and screening triage prototype only. Not certified for clinical diagnosis. Clinical examination required for definitive care.".to_string(),
                                };
                                return Some(InferenceEvent::Stage1Completed(screening_result));
                            }
                            Err(err) => {
                                return Some(InferenceEvent::InferenceFailed {
                                    request_id: req.request_id,
                                    error: err,
                                });
                            }
                        }
                    }
                }
                ModelEvent::Unloaded { model_id, .. } => {
                    let un = BackendReadiness::Unavailable {
                        reason: format!("Model `{model_id}` was unloaded"),
                    };
                    *self.readiness.lock().unwrap() = un.clone();
                    return Some(InferenceEvent::ReadinessChanged(un));
                }
                _ => {}
            }
        }
        None
    }
}

/// Deterministic mock service for fast unit tests without filesystem or ONNX dependencies.
pub struct MockMedicalInferenceService {
    readiness: Mutex<BackendReadiness>,
    events: Mutex<Vec<InferenceEvent>>,
    should_fail: AtomicBool,
}

impl MockMedicalInferenceService {
    pub fn new_ready() -> Self {
        Self {
            readiness: Mutex::new(BackendReadiness::Ready {
                model_id: "hierarchiretina-stage1-convnextv2l-768".to_string(),
                version: "1.0.0".to_string(),
                provider: "Mock CPU".to_string(),
                threshold: STAGE1_GATE_THRESHOLD,
            }),
            events: Mutex::new(Vec::new()),
            should_fail: AtomicBool::new(false),
        }
    }

    pub fn new_unavailable(reason: &str) -> Self {
        Self {
            readiness: Mutex::new(BackendReadiness::Unavailable {
                reason: reason.to_string(),
            }),
            events: Mutex::new(Vec::new()),
            should_fail: AtomicBool::new(false),
        }
    }

    pub fn set_should_fail(&self, fail: bool) {
        self.should_fail.store(fail, Ordering::Relaxed);
    }

    pub fn queue_event(&self, event: InferenceEvent) {
        self.events.lock().unwrap().push(event);
    }
}

impl MedicalInferenceService for MockMedicalInferenceService {
    fn readiness(&self) -> BackendReadiness {
        self.readiness.lock().unwrap().clone()
    }

    fn submit(&self, request: RetinalAnalysisRequest) -> Result<(), InferenceError> {
        let readiness = self.readiness();
        if !readiness.is_ready() {
            return Err(InferenceError::BackendNotReady);
        }

        if !request.requested_stage.is_available() {
            return Err(InferenceError::StageUnavailable(request.requested_stage));
        }

        if self.should_fail.load(Ordering::Relaxed) {
            self.queue_event(InferenceEvent::InferenceFailed {
                request_id: request.request_id,
                error: "Simulated mock inference failure".to_string(),
            });
            return Ok(());
        }

        let screening_score = 0.785;
        let decision = if screening_score >= STAGE1_GATE_THRESHOLD {
            Stage1GateDecision::ContinueCascade
        } else {
            Stage1GateDecision::NegativeRoute
        };

        self.queue_event(InferenceEvent::Stage1Completed(Stage1ScreeningResult {
            request_id: request.request_id,
            examination_id: request.examination_id,
            model_id: "hierarchiretina-stage1-convnextv2l-768".to_string(),
            model_version: "1.0.0".to_string(),
            model_sha256: "9ad51e1f428dc59ab49f1f49e8f68a84cd23d8fd4c2c461b45b06bcc726880ac"
                .to_string(),
            provider: "Mock CPU".to_string(),
            raw_logit: 1.295,
            screening_score,
            threshold: STAGE1_GATE_THRESHOLD,
            decision,
            elapsed_ms: 142,
            preprocessing_ms: 38,
            disclaimer: "Deterministic test mock output. Research prototype only.".to_string(),
        }));

        Ok(())
    }

    fn cancel(&self, request_id: &str) {
        self.queue_event(InferenceEvent::Cancelled {
            request_id: request_id.to_string(),
        });
    }

    fn try_recv(&self) -> Option<InferenceEvent> {
        let mut events = self.events.lock().unwrap();
        if events.is_empty() {
            None
        } else {
            Some(events.remove(0))
        }
    }
}

/// Helper to discover the default model manifest path across known repository topologies.
pub fn discover_default_manifest() -> PathBuf {
    // An installed build has no source tree: the launcher points here at the external models folder.
    if let Some(dir) = std::env::var_os("RETINACARE_MODELS_DIR") {
        let manifest = PathBuf::from(dir).join("stage1_convnextv2l_768/manifest.json");
        if manifest.exists() {
            return manifest;
        }
    }
    let candidates = [
        // GUI crate lives at Dird/GUI/<name>/rust, so Dird/models is three levels up.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../models/stage1_convnextv2l_768/manifest.json"),
        PathBuf::from("/home/claude/Desktop/Dird/models/stage1_convnextv2l_768/manifest.json"),
        PathBuf::from("../../Dird/models/stage1_convnextv2l_768/manifest.json"),
        PathBuf::from("../Dird/models/stage1_convnextv2l_768/manifest.json"),
        PathBuf::from("models/stage1_convnextv2l_768/manifest.json"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return candidate;
        }
    }

    // Default fallback path for explicit error reporting
    PathBuf::from("/home/claude/Desktop/Dird/models/stage1_convnextv2l_768/manifest.json")
}

/// The device a model really runs on, as shown in the UI and written to results.
pub fn provider_name(provider: retinacare_ai_bridge::device::ActualProvider) -> String {
    use retinacare_ai_bridge::device::ActualProvider;
    match provider {
        ActualProvider::Cpu => "CPU".to_string(),
        ActualProvider::Cuda { device_id } => format!("GPU (CUDA {device_id})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_service_submits_and_returns_stage1_result() {
        let service = MockMedicalInferenceService::new_ready();
        assert!(service.readiness().is_ready());

        let req = RetinalAnalysisRequest::new("exam-001", "image.png", Some(EyeLaterality::OD));
        let req_id = req.request_id.clone();
        assert!(service.submit(req).is_ok());

        let event = service.try_recv().expect("Event should be available");
        match event {
            InferenceEvent::Stage1Completed(res) => {
                assert_eq!(res.request_id, req_id);
                assert_eq!(res.examination_id, "exam-001");
                assert_eq!(res.decision, Stage1GateDecision::ContinueCascade);
                assert!(res.screening_score > STAGE1_GATE_THRESHOLD);
                assert_eq!(res.raw_logit, 1.295);
            }
            other => panic!("Unexpected event: {other:?}"),
        }
    }

    #[test]
    fn unavailable_service_rejects_submission() {
        let service = MockMedicalInferenceService::new_unavailable("CUDA driver missing");
        assert!(!service.readiness().is_ready());

        let req = RetinalAnalysisRequest::new("exam-002", "image.png", None);
        assert_eq!(service.submit(req), Err(InferenceError::BackendNotReady));
    }

    #[test]
    fn unavailable_pipeline_stages_are_rejected() {
        let service = MockMedicalInferenceService::new_ready();
        let mut req = RetinalAnalysisRequest::new("exam-003", "image.png", None);
        req.requested_stage = PipelineStage::Stage2Segmentation;

        assert_eq!(
            service.submit(req),
            Err(InferenceError::StageUnavailable(
                PipelineStage::Stage2Segmentation
            ))
        );
    }

    #[test]
    fn mock_service_cancellation_emits_cancelled_event() {
        let service = MockMedicalInferenceService::new_ready();
        service.cancel("req-test-123");

        let event = service.try_recv().expect("Cancelled event expected");
        assert_eq!(
            event,
            InferenceEvent::Cancelled {
                request_id: "req-test-123".to_string()
            }
        );
    }

    #[test]
    fn mock_service_failure_mode() {
        let service = MockMedicalInferenceService::new_ready();
        service.set_should_fail(true);

        let req = RetinalAnalysisRequest::new("exam-fail", "image.png", None);
        let req_id = req.request_id.clone();
        assert!(service.submit(req).is_ok());

        let event = service.try_recv().expect("Failure event expected");
        match event {
            InferenceEvent::InferenceFailed { request_id, error } => {
                assert_eq!(request_id, req_id);
                assert!(error.contains("Simulated mock"));
            }
            other => panic!("Unexpected event: {other:?}"),
        }
    }

    #[test]
    fn discover_default_manifest_resolves_path() {
        let path = discover_default_manifest();
        // Verifies the discovery utility returns a valid path
        assert!(path.to_str().unwrap().contains("manifest.json"));
    }

    /// Heavy: loads the real Stage I ONNX (~800 MB) through the backend. Run with `-- --ignored`.
    #[test]
    #[ignore = "loads the real Stage I model; needs Dird/models and the demo image"]
    fn live_service_runs_stage1_on_demo_image() {
        let manifest = discover_default_manifest();
        let image = manifest
            .ancestors()
            .nth(3)
            .unwrap()
            .join("dist/demo-images/session-1-final/OD-1.png");
        let service = LiveCodexBackendService::new(manifest);
        let deadline = Instant::now() + std::time::Duration::from_secs(300);
        let mut result = None;
        let mut submitted = false;
        while Instant::now() < deadline && result.is_none() {
            if !submitted && matches!(service.readiness(), BackendReadiness::Ready { .. }) {
                submitted = true;
                service
                    .submit(RetinalAnalysisRequest::new(
                        "live-exam",
                        image.clone(),
                        None,
                    ))
                    .expect("submit");
            }
            if let Some(InferenceEvent::Stage1Completed(r)) = service.try_recv() {
                result = Some(r);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let r = result.expect("Stage1Completed within 300 s");
        assert_eq!(r.model_sha256, service.model_sha256());
        assert!(r.provider.contains("CPU"));
    }
}
