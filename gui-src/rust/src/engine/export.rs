//! Secure Local Report Export & Audit Ledger Services
//!
//! Provides nonblocking asynchronous export channels, PDF generation adapters,
//! patient session dossier export, and authentic audit ledger presentation.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Configuration payload for an explicit user-initiated report export
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportExportRequest {
    pub request_id: String,
    pub examination_id: String,
    pub patient_code: String,
    pub destination_path: PathBuf,
    pub include_original_image: bool,
    pub include_findings_json: bool,
    pub include_provenance_meta: bool,
    /// The report content, built from verified results only. `None` means there is nothing to write.
    pub document: Option<retinacare_reports::PdfDocument>,
}

/// Asynchronous state machine tracking an active export operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportState {
    /// No export active
    Idle,
    /// Preparing request and launching destination selector
    Preparing,
    /// Background worker generating and writing export
    Generating { request_id: String },
    /// Save destination selected; confirming write
    SaveRequested {
        request_id: String,
        default_filename: String,
    },
    /// Successfully written and validated on disk
    Saved {
        request_id: String,
        path: PathBuf,
        bytes: usize,
    },
    /// Export encountered an I/O, rendering, or validation error
    Failed { request_id: String, error: String },
    /// Operation cancelled by clinician
    Cancelled { request_id: String },
    /// Backend service is not present in local installation
    BackendUnavailable { reason: String },
}

/// Events emitted during background report export
#[derive(Debug, Clone, PartialEq)]
pub enum ExportEvent {
    Started {
        request_id: String,
    },
    Progress {
        request_id: String,
        progress: f32,
        stage: String,
    },
    Completed {
        request_id: String,
        path: PathBuf,
        bytes: usize,
    },
    Failed {
        request_id: String,
        error: String,
    },
    Cancelled {
        request_id: String,
    },
}

/// Readiness status of the PDF export service
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfServiceReadiness {
    Ready { provider: String },
    BackendUnavailable { reason: String },
}

/// Trait defining the clinical PDF export integration boundary
pub trait PdfExportService: Send + Sync {
    fn readiness(&self) -> PdfServiceReadiness;
    fn submit_export(&self, req: ReportExportRequest) -> Result<Receiver<ExportEvent>, String>;
    fn cancel(&self, request_id: &str) -> Result<(), String>;
}

/// Live PDF export on the `retinacare-reports` writer: background thread, atomic write, validated
/// before it appears at the destination. A failure never leaves a partial file.
#[derive(Default)]
pub struct LiveCodexPdfService {
    cancelled: Arc<Mutex<Vec<String>>>,
}

impl PdfExportService for LiveCodexPdfService {
    fn readiness(&self) -> PdfServiceReadiness {
        PdfServiceReadiness::Ready {
            provider: "retinacare-reports PDF writer (text report, atomic write)".to_string(),
        }
    }

    fn submit_export(&self, req: ReportExportRequest) -> Result<Receiver<ExportEvent>, String> {
        let document = req
            .document
            .clone()
            .ok_or("There is no report content to export.")?;
        let (tx, rx) = mpsc::channel();
        let cancelled = Arc::clone(&self.cancelled);
        thread::spawn(move || {
            let id = req.request_id.clone();
            let _ = tx.send(ExportEvent::Started {
                request_id: id.clone(),
            });
            let _ = tx.send(ExportEvent::Progress {
                request_id: id.clone(),
                progress: 0.5,
                stage: "Rendering and validating PDF".to_string(),
            });
            let was_cancelled = cancelled.lock().map(|c| c.contains(&id)).unwrap_or(false);
            if was_cancelled {
                let _ = tx.send(ExportEvent::Cancelled { request_id: id });
                return;
            }
            match retinacare_reports::write_pdf_atomic(&req.destination_path, &document) {
                Ok(bytes) => {
                    let _ = tx.send(ExportEvent::Completed {
                        request_id: id,
                        path: req.destination_path,
                        bytes,
                    });
                }
                Err(error) => {
                    let _ = tx.send(ExportEvent::Failed {
                        request_id: id,
                        error: error.to_string(),
                    });
                }
            }
        });
        Ok(rx)
    }

    fn cancel(&self, request_id: &str) -> Result<(), String> {
        if let Ok(mut cancelled) = self.cancelled.lock() {
            cancelled.push(request_id.to_string());
        }
        Ok(())
    }
}

/// Mock PDF export service for deterministic testing of export lifecycle
pub struct MockPdfExportService {
    pub simulate_failure: bool,
    pub latency: Duration,
    cancelled_ids: Arc<Mutex<Vec<String>>>,
}

impl MockPdfExportService {
    pub fn new(simulate_failure: bool, latency: Duration) -> Self {
        Self {
            simulate_failure,
            latency,
            cancelled_ids: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn new_success() -> Self {
        Self::new(false, Duration::from_millis(5))
    }

    pub fn new_failure(error_msg: &str) -> Self {
        let _ = error_msg;
        Self::new(true, Duration::from_millis(5))
    }
}

impl Default for MockPdfExportService {
    fn default() -> Self {
        Self::new(false, Duration::from_millis(5))
    }
}

impl PdfExportService for MockPdfExportService {
    fn readiness(&self) -> PdfServiceReadiness {
        PdfServiceReadiness::Ready {
            provider: "Mock Native PDF Generator (Test)".to_string(),
        }
    }

    fn submit_export(&self, req: ReportExportRequest) -> Result<Receiver<ExportEvent>, String> {
        let (tx, rx) = mpsc::channel();
        let fail = self.simulate_failure;
        let latency = self.latency;
        let cancelled_arc = Arc::clone(&self.cancelled_ids);

        thread::spawn(move || {
            let _ = tx.send(ExportEvent::Started {
                request_id: req.request_id.clone(),
            });

            if latency.as_millis() > 0 {
                thread::sleep(latency);
            }

            // Check if cancelled
            if let Ok(cancelled) = cancelled_arc.lock() {
                if cancelled.contains(&req.request_id) {
                    let _ = tx.send(ExportEvent::Cancelled {
                        request_id: req.request_id,
                    });
                    return;
                }
            }

            let _ = tx.send(ExportEvent::Progress {
                request_id: req.request_id.clone(),
                progress: 0.5,
                stage: "Rendering clinical charts...".to_string(),
            });

            if fail {
                let _ = tx.send(ExportEvent::Failed {
                    request_id: req.request_id,
                    error: "Simulated I/O disk write error".to_string(),
                });
            } else {
                // Write a mock valid PDF file with signature to destination
                let mock_bytes =
                    b"%PDF-1.7\n1 0 obj\n<< /Title (RetinaCare Report) >>\nendobj\n%%EOF\n";
                let bytes_written = mock_bytes.len();
                if let Some(parent) = req.destination_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&req.destination_path, mock_bytes);

                let _ = tx.send(ExportEvent::Completed {
                    request_id: req.request_id,
                    path: req.destination_path,
                    bytes: bytes_written,
                });
            }
        });

        Ok(rx)
    }

    fn cancel(&self, request_id: &str) -> Result<(), String> {
        if let Ok(mut cancelled) = self.cancelled_ids.lock() {
            cancelled.push(request_id.to_string());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Plaintext Sensitive Patient Session Export (Unencrypted JSON)
// ---------------------------------------------------------------------------

/// Packages and exports an isolated patient examination session as plaintext JSON files.
///
/// # Security & Privacy Notice (Instruction 12)
/// This export produces UNENCRYPTED, PLAINTEXT JSON files. It must NEVER be called
/// or documented as an "encrypted dossier". It is intended only for localized,
/// clinician-initiated data porting to an already-encrypted storage volume.
pub fn export_scoped_patient_session(
    dest_dir: &std::path::Path,
    exam: &crate::domain::RetinalExaminationView,
    stage1_result: Option<&crate::engine::backend::Stage1ScreeningResult>,
) -> Result<PathBuf, String> {
    if !dest_dir.exists() {
        std::fs::create_dir_all(dest_dir)
            .map_err(|e| format!("Failed to create destination dir: {e}"))?;
    }
    let export_folder_name = format!("{}_{}", exam.patient_code, exam.exam_id);
    let session_dir = dest_dir.join(&export_folder_name);
    std::fs::create_dir_all(&session_dir)
        .map_err(|e| format!("Failed to create session dir: {e}"))?;

    // 1. Manifest with explicit plaintext unencrypted disclosure
    let manifest_json = format!(
        "{{\n  \"examination_id\": \"{}\",\n  \"patient_code\": \"{}\",\n  \"laterality\": \"{:?}\",\n  \"export_type\": \"PLAINTEXT_SENSITIVE_UNENCRYPTED_JSON\",\n  \"security_notice\": \"CONFIDENTIAL MEDICAL DATA - NOT ENCRYPTED AT REST. Store only on an encrypted filesystem.\",\n  \"exported_at\": \"{}\",\n  \"non_diagnostic_caveat\": \"{}\"\n}}",
        exam.exam_id,
        exam.patient_code,
        exam.laterality,
        crate::engine::report::utc_now(),
        crate::engine::report::MANDATORY_RESEARCH_CAVEAT
    );
    std::fs::write(session_dir.join("manifest.json"), manifest_json)
        .map_err(|e| format!("Failed to write manifest.json: {e}"))?;

    // 2. Screening findings
    let findings_json = if let Some(res) = stage1_result {
        format!(
            "{{\n  \"stage1\": {{\n    \"decision\": \"{:?}\",\n    \"screening_score\": {},\n    \"raw_logit\": {},\n    \"threshold\": {},\n    \"elapsed_ms\": {},\n    \"model_id\": \"{}\",\n    \"model_sha256\": \"{}\"\n  }},\n  \"stage2\": \"Unavailable (Weights not packaged)\",\n  \"stage3\": \"Unavailable (Weights not packaged)\"\n}}",
            res.decision,
            res.screening_score,
            res.raw_logit,
            res.threshold,
            res.elapsed_ms,
            res.model_id,
            res.model_sha256
        )
    } else {
        "{\n  \"stage1\": \"Pending Execution\",\n  \"stage2\": \"Unavailable\",\n  \"stage3\": \"Unavailable\"\n}".to_string()
    };
    std::fs::write(session_dir.join("screening_findings.json"), findings_json)
        .map_err(|e| format!("Failed to write screening_findings.json: {e}"))?;

    // 3. Original image if present
    if let Some(src_path) = &exam.image_path {
        if src_path.exists() {
            let ext = src_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("png");
            let target_img = session_dir.join(format!("fundus_scan.{}", ext));
            let _ = std::fs::copy(src_path, target_img);
        }
    }

    Ok(session_dir)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub event_id: String,
    pub timestamp: String,
    pub event_type: AuditEventType,
    pub examination_id: String,
    pub request_id: String,
    pub operator: String,
    pub status: AuditEventStatus,
    pub non_sensitive_summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEventType {
    ExaminationLoaded,
    Stage1InferenceRun,
    ReportPreviewed,
    PdfExportRequested,
    PdfExportCompleted,
    SessionExportCompleted,
    ExportFailed,
    AnalysisCompleted,
    AnalysisFailed,
    AnalysisCancelled,
    AttributionSaved,
}

impl AuditEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExaminationLoaded => "examination_loaded",
            Self::Stage1InferenceRun => "stage1_inference_run",
            Self::ReportPreviewed => "report_previewed",
            Self::PdfExportRequested => "pdf_export_requested",
            Self::PdfExportCompleted => "pdf_export_completed",
            Self::SessionExportCompleted => "session_export_completed",
            Self::ExportFailed => "export_failed",
            Self::AnalysisCompleted => "analysis_completed",
            Self::AnalysisFailed => "analysis_failed",
            Self::AnalysisCancelled => "analysis_cancelled",
            Self::AttributionSaved => "attribution_saved",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        [
            Self::ExaminationLoaded,
            Self::Stage1InferenceRun,
            Self::ReportPreviewed,
            Self::PdfExportRequested,
            Self::PdfExportCompleted,
            Self::SessionExportCompleted,
            Self::ExportFailed,
            Self::AnalysisCompleted,
            Self::AnalysisFailed,
            Self::AnalysisCancelled,
            Self::AttributionSaved,
        ]
        .into_iter()
        .find(|kind| kind.as_str() == text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEventStatus {
    Success,
    Failed,
    Cancelled,
    Informational,
}

impl AuditEventStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Informational => "informational",
        }
    }

    pub fn parse(text: &str) -> Self {
        match text {
            "success" => Self::Success,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => Self::Informational,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditLedgerReadiness {
    Ready { record_count: usize },
    Unavailable { reason: String },
}

pub trait AuditLedgerService: Send + Sync {
    fn readiness(&self) -> AuditLedgerReadiness;
    fn query_events(&self, examination_id: Option<&str>) -> Result<Vec<AuditEvent>, String>;
    fn record_event(&self, event: AuditEvent) -> Result<(), String>;
}

/// Live audit ledger on the encrypted database: append-only, one row per event, identifiers and
/// statuses only. Unavailable (never faked) while the database is locked.
pub struct LiveCodexAuditService {
    store: super::store::SharedStore,
    count: std::sync::atomic::AtomicUsize,
    counted: std::sync::atomic::AtomicBool,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AuditDetail {
    request: String,
    operator: String,
    summary: String,
    exam_label: String,
}

impl LiveCodexAuditService {
    pub fn with_store(store: super::store::SharedStore) -> Self {
        Self {
            store,
            count: std::sync::atomic::AtomicUsize::new(0),
            counted: std::sync::atomic::AtomicBool::new(false),
        }
    }

    fn exam_number(label: &str) -> Option<i64> {
        label.strip_prefix("EXAM-").and_then(|n| n.parse().ok())
    }
}

impl AuditLedgerService for LiveCodexAuditService {
    fn readiness(&self) -> AuditLedgerReadiness {
        use std::sync::atomic::Ordering;
        if !self.store.is_open() {
            return AuditLedgerReadiness::Unavailable {
                reason:
                    "The encrypted database is locked; unlock it to read or write the audit ledger."
                        .to_string(),
            };
        }
        if !self.counted.load(Ordering::Relaxed) {
            if let Ok(all) = self.store.with(|s| s.list_audit_events(None)) {
                self.count.store(all.len(), Ordering::Relaxed);
                self.counted.store(true, Ordering::Relaxed);
            }
        }
        AuditLedgerReadiness::Ready {
            record_count: self.count.load(Ordering::Relaxed),
        }
    }

    fn query_events(&self, examination_id: Option<&str>) -> Result<Vec<AuditEvent>, String> {
        let rows = self
            .store
            .with(|s| s.list_audit_events(None))
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let detail: AuditDetail = serde_json::from_str(&row.detail).ok()?;
                if let Some(wanted) = examination_id {
                    if detail.exam_label != wanted {
                        return None;
                    }
                }
                Some(AuditEvent {
                    event_id: format!("evt_{}", row.id),
                    timestamp: row.recorded_at,
                    event_type: AuditEventType::parse(&row.event_type)?,
                    examination_id: detail.exam_label,
                    request_id: detail.request,
                    operator: detail.operator,
                    status: AuditEventStatus::parse(&row.status),
                    non_sensitive_summary: detail.summary,
                })
            })
            .collect())
    }

    fn record_event(&self, event: AuditEvent) -> Result<(), String> {
        use std::sync::atomic::Ordering;
        let detail = serde_json::to_string(&AuditDetail {
            request: event.request_id,
            operator: event.operator,
            summary: event.non_sensitive_summary,
            exam_label: event.examination_id.clone(),
        })
        .map_err(|e| e.to_string())?;
        let exam = Self::exam_number(&event.examination_id);
        self.store
            .with(|s| {
                s.append_audit_event(
                    event.event_type.as_str(),
                    None,
                    exam,
                    event.status.as_str(),
                    &detail,
                )
            })
            .map_err(|e| e.to_string())?;
        if self.counted.load(Ordering::Relaxed) {
            self.count.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }
}

/// Mock in-memory audit ledger service for deterministic testing
pub struct MockAuditLedgerService {
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl MockAuditLedgerService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_sample_events() -> Self {
        let service = Self::default();
        let ev1 = AuditEvent {
            event_id: "evt_001".to_string(),
            timestamp: "2026-10-28 01:28:15 UTC".to_string(),
            event_type: AuditEventType::ExaminationLoaded,
            examination_id: "exam_current".to_string(),
            request_id: "req_load_01".to_string(),
            operator: "Clinician (Local Desk)".to_string(),
            status: AuditEventStatus::Success,
            non_sensitive_summary: "Fundus scan loaded into examination workspace".to_string(),
        };
        let ev2 = AuditEvent {
            event_id: "evt_002".to_string(),
            timestamp: "2026-10-28 01:29:40 UTC".to_string(),
            event_type: AuditEventType::Stage1InferenceRun,
            examination_id: "exam_current".to_string(),
            request_id: "req_inf_01".to_string(),
            operator: "Clinician (Local Desk)".to_string(),
            status: AuditEventStatus::Success,
            non_sensitive_summary:
                "ConvNeXt-V2 Stage 1 screening inference executed (score: 0.7773, ContinueCascade)"
                    .to_string(),
        };
        let ev3 = AuditEvent {
            event_id: "evt_003".to_string(),
            timestamp: "2026-10-28 01:30:05 UTC".to_string(),
            event_type: AuditEventType::ReportPreviewed,
            examination_id: "exam_current".to_string(),
            request_id: "req_view_01".to_string(),
            operator: "Clinician (Local Desk)".to_string(),
            status: AuditEventStatus::Informational,
            non_sensitive_summary: "Clinical report summary preview viewed in GUI".to_string(),
        };
        let _ = service.record_event(ev1);
        let _ = service.record_event(ev2);
        let _ = service.record_event(ev3);
        service
    }
}

impl Default for MockAuditLedgerService {
    fn default() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl AuditLedgerService for MockAuditLedgerService {
    fn readiness(&self) -> AuditLedgerReadiness {
        let count = self.events.lock().map(|e| e.len()).unwrap_or(0);
        AuditLedgerReadiness::Ready {
            record_count: count,
        }
    }

    fn query_events(&self, examination_id: Option<&str>) -> Result<Vec<AuditEvent>, String> {
        let guard = self.events.lock().map_err(|e| e.to_string())?;
        let filtered: Vec<AuditEvent> = match examination_id {
            Some(id) => guard
                .iter()
                .filter(|e| e.examination_id == id)
                .cloned()
                .collect(),
            None => guard.clone(),
        };
        Ok(filtered)
    }

    fn record_event(&self, event: AuditEvent) -> Result<(), String> {
        let mut guard = self.events.lock().map_err(|e| e.to_string())?;
        guard.push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_pdf_service_requires_content_and_writes_a_valid_pdf() {
        let service = LiveCodexPdfService::default();
        assert!(matches!(
            service.readiness(),
            PdfServiceReadiness::Ready { .. }
        ));
        let dest = std::env::temp_dir().join(format!("rc-live-pdf-{}/r.pdf", std::process::id()));
        let mut req = ReportExportRequest {
            request_id: "req_01".to_string(),
            examination_id: "exam_01".to_string(),
            patient_code: "PAT_01".to_string(),
            destination_path: dest.clone(),
            include_original_image: true,
            include_findings_json: true,
            include_provenance_meta: true,
            document: None,
        };
        assert!(
            service.submit_export(req.clone()).is_err(),
            "no content, no PDF"
        );
        req.document = Some(retinacare_reports::PdfDocument {
            title: "t".into(),
            lines: vec![retinacare_reports::PdfLine::Text("hello".into())],
        });
        let rx = service.submit_export(req).unwrap();
        let mut completed = false;
        for event in rx.iter() {
            if let ExportEvent::Completed { path, bytes, .. } = event {
                retinacare_reports::validate_pdf(&std::fs::read(&path).unwrap()).unwrap();
                assert_eq!(std::fs::metadata(&path).unwrap().len() as usize, bytes);
                completed = true;
            }
        }
        assert!(completed);
    }

    #[test]
    fn test_live_pdf_failure_is_reported_and_leaves_no_file() {
        let service = LiveCodexPdfService::default();
        let dir = std::env::temp_dir().join(format!("rc-live-pdf-fail-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("occupied.pdf")).unwrap();
        let req = ReportExportRequest {
            request_id: "req_02".to_string(),
            examination_id: "e".to_string(),
            patient_code: "p".to_string(),
            destination_path: dir.join("occupied.pdf"),
            include_original_image: false,
            include_findings_json: false,
            include_provenance_meta: false,
            document: Some(retinacare_reports::PdfDocument {
                title: "t".into(),
                lines: vec![retinacare_reports::PdfLine::Text("x".into())],
            }),
        };
        let events: Vec<_> = service.submit_export(req).unwrap().iter().collect();
        assert!(events
            .iter()
            .any(|e| matches!(e, ExportEvent::Failed { .. })));
        assert!(!events
            .iter()
            .any(|e| matches!(e, ExportEvent::Completed { .. })));
        assert!(dir.join("occupied.pdf").is_dir());
    }

    #[test]
    fn test_live_audit_service_is_unavailable_until_unlocked_then_append_only() {
        let path =
            std::env::temp_dir().join(format!("rc-live-audit-{}/db.sqlite", std::process::id()));
        let store = super::super::store::SharedStore::new(&path);
        let service = LiveCodexAuditService::with_store(store.clone());
        assert!(matches!(
            service.readiness(),
            AuditLedgerReadiness::Unavailable { .. }
        ));
        assert!(service.query_events(None).is_err());
        store.begin_unlock("correct horse".into(), Some("correct horse".into()));
        let start = std::time::Instant::now();
        while !store.is_open() {
            assert!(start.elapsed().as_secs() < 60);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let make = |exam: &str, kind| AuditEvent {
            event_id: String::new(),
            timestamp: String::new(),
            event_type: kind,
            examination_id: exam.to_string(),
            request_id: "req_1".to_string(),
            operator: "tester".to_string(),
            status: AuditEventStatus::Success,
            non_sensitive_summary: "ok".to_string(),
        };
        service
            .record_event(make("EXAM-001", AuditEventType::AnalysisCompleted))
            .unwrap();
        service
            .record_event(make("EXAM-002", AuditEventType::PdfExportCompleted))
            .unwrap();
        assert!(matches!(
            service.readiness(),
            AuditLedgerReadiness::Ready { record_count: 2 }
        ));
        let one = service.query_events(Some("EXAM-001")).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].event_type, AuditEventType::AnalysisCompleted);
        assert!(
            !one[0].timestamp.is_empty(),
            "timestamp comes from the database"
        );
        assert_eq!(service.query_events(None).unwrap().len(), 2);
    }

    #[test]
    fn test_mock_pdf_service_successful_export_and_file_write() {
        let service = MockPdfExportService::default();
        let dest = std::env::temp_dir().join("retinacare_test_export.pdf");
        let req = ReportExportRequest {
            request_id: "mock_req_100".to_string(),
            examination_id: "exam_100".to_string(),
            patient_code: "PAT_100".to_string(),
            destination_path: dest.clone(),
            include_original_image: true,
            include_findings_json: true,
            include_provenance_meta: true,
            document: None,
        };

        let rx = service.submit_export(req).expect("Submit should succeed");
        let mut completed = false;

        while let Ok(event) = rx.recv() {
            match event {
                ExportEvent::Completed { path, bytes, .. } => {
                    assert_eq!(path, dest);
                    assert!(bytes > 0);
                    completed = true;
                }
                ExportEvent::Failed { error, .. } => panic!("Unexpected failure: {}", error),
                _ => {}
            }
        }
        assert!(completed);
        assert!(dest.exists());
        let _ = std::fs::remove_file(dest);
    }

    #[test]
    fn test_mock_audit_ledger_recording_and_filtering() {
        let service = MockAuditLedgerService::default();
        let ev1 = AuditEvent {
            event_id: "ev_01".to_string(),
            timestamp: "2026-10-28 01:00:00".to_string(),
            event_type: AuditEventType::Stage1InferenceRun,
            examination_id: "exam_A".to_string(),
            request_id: "req_A".to_string(),
            operator: "Clinician".to_string(),
            status: AuditEventStatus::Success,
            non_sensitive_summary: "Stage 1 ConvNeXt screening passed threshold".to_string(),
        };
        let ev2 = AuditEvent {
            event_id: "ev_02".to_string(),
            timestamp: "2026-10-28 01:05:00".to_string(),
            event_type: AuditEventType::PdfExportRequested,
            examination_id: "exam_B".to_string(),
            request_id: "req_B".to_string(),
            operator: "Clinician".to_string(),
            status: AuditEventStatus::Informational,
            non_sensitive_summary: "Export initiated".to_string(),
        };

        service.record_event(ev1).expect("Record should succeed");
        service.record_event(ev2).expect("Record should succeed");

        let all_events = service.query_events(None).expect("Query should succeed");
        assert_eq!(all_events.len(), 2);

        let filtered_a = service
            .query_events(Some("exam_A"))
            .expect("Filter should succeed");
        assert_eq!(filtered_a.len(), 1);
        assert_eq!(filtered_a[0].event_id, "ev_01");

        let filtered_b = service
            .query_events(Some("exam_B"))
            .expect("Filter should succeed");
        assert_eq!(filtered_b.len(), 1);
        assert_eq!(filtered_b[0].event_id, "ev_02");
    }

    #[test]
    fn test_export_scoped_patient_session_creates_isolated_plaintext_export() {
        let tmp = std::env::temp_dir().join("retinacare_test_session_export");
        let exam = crate::domain::RetinalExaminationView {
            exam_id: "exam_scope_test".to_string(),
            patient_code: "PATIENT_ISOLATED".to_string(),
            ..Default::default()
        };

        let res = export_scoped_patient_session(&tmp, &exam, None);
        assert!(res.is_ok());
        let export_dir = res.unwrap();
        assert!(export_dir.exists());
        assert!(export_dir.join("manifest.json").exists());
        assert!(export_dir.join("screening_findings.json").exists());

        let manifest = std::fs::read_to_string(export_dir.join("manifest.json")).unwrap();
        assert!(manifest.contains("PATIENT_ISOLATED"));
        assert!(manifest.contains("exam_scope_test"));
        assert!(manifest.contains("PLAINTEXT_SENSITIVE_UNENCRYPTED_JSON"));

        let _ = std::fs::remove_dir_all(export_dir);
    }
}
