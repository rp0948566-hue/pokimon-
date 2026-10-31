# 05. Typed API Contracts

## 1. Core Integration Types (`rust/src/engine/backend.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendReadiness {
    NotConfigured,
    ModelMissing { path: PathBuf, reason: String },
    Initializing { stage: String },
    Ready {
        model_id: String,
        version: String,
        provider: String,
        threshold: f32,
    },
    Processing { request_id: String },
    Cancelling { request_id: String },
    Unavailable { reason: String },
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetinalAnalysisRequest {
    pub request_id: String,
    pub examination_id: String,
    pub patient_id: Option<String>,
    pub image_path: PathBuf,
    pub eye: Option<EyeLaterality>,
    pub requested_stage: PipelineStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    Stage1ScreeningGate,
    Stage2LesionSegmentation,   // Unavailable
    Stage3VascularSegmentation, // Unavailable
    Stage4ClinicalGrading,      // Unavailable
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum InferenceEvent {
    ReadinessChanged(BackendReadiness),
    Progress { request_id: String, step: String, percent: f32 },
    Stage1Completed(Stage1ScreeningResult),
    InferenceFailed { request_id: String, error: String },
    Cancelled { request_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferenceError {
    BackendNotReady,
    ImageNotFound(PathBuf),
    QueueFull,
    ServiceStopped,
    InvalidRequest(String),
}
```

## 2. Service Trait Contract

```rust
pub trait MedicalInferenceService: Send + Sync {
    fn readiness(&self) -> BackendReadiness;
    fn submit(&self, request: RetinalAnalysisRequest) -> Result<(), InferenceError>;
    fn cancel(&self, request_id: &str);
    fn try_recv(&self) -> Option<InferenceEvent>;
}
```
