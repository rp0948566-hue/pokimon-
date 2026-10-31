//! Engine layer: Document extraction (PDF), Medical AI Backend, and Model Manifest architecture.

pub mod backend;
pub mod cascade;
pub mod chat;
pub mod cloud;
pub mod explain;
pub mod export;
pub mod gpu;
pub mod keystore;
pub mod manifest;
pub mod ollama;
pub mod papers;
pub mod patient;
pub mod pdf;
pub mod report;
pub mod science;
pub mod store;
pub mod supabase;
pub mod web;

pub use backend::{
    discover_default_manifest, BackendReadiness, InferenceError, InferenceEvent,
    LiveCodexBackendService, MedicalInferenceService, MockMedicalInferenceService, PipelineStage,
    RetinalAnalysisRequest, Stage1GateDecision, Stage1ScreeningResult, STAGE1_GATE_THRESHOLD,
};
pub use chat::{ChatService, ChatServiceError, UnavailableChatService};
pub use export::{
    export_scoped_patient_session, AuditEvent, AuditEventStatus, AuditEventType,
    AuditLedgerReadiness, AuditLedgerService, ExportEvent, ExportState, LiveCodexAuditService,
    LiveCodexPdfService, MockAuditLedgerService, MockPdfExportService, PdfExportService,
    PdfServiceReadiness, ReportExportRequest,
};
pub use manifest::HierarchiRetinaManifest;
pub use patient::{
    ExaminationSummary, LiveCodexPatientService, MockPatientService, PatientDetail, PatientQuery,
    PatientService, PatientServiceError, PatientServiceReadiness, PatientSummary,
};
pub use pdf::extract_pdf_document;
pub use report::{PipelineStageStatus, ReportViewModel, MANDATORY_RESEARCH_CAVEAT};
