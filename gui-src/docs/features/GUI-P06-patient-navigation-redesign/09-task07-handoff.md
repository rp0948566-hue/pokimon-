# Phase 1 • Task 06.1: Phase 2 / Task 07 Handoff Specification

## 1. Context & Architecture Boundary
- **Antigravity (GUI Owner)**: Native Rust `egui`/`eframe` frontend (`rust/`).
- **Codex (Backend Owner)**: Rust medical AI, database persistence, and report generation (`Hackthon Backend/`).
- **Current Integration**: Frontend is completely ready with typed subroutes, breadcrumb state machines, and view dispatchers.

---

## 2. API Contracts Required from Codex

### A. Report Generation Engine (`retinacare-reports`)
Codex must provide a compilation trait or service:
```rust
pub trait ReportGenerationService: Send + Sync {
    fn generate_pdf(
        &self,
        request: &PdfExportRequest,
    ) -> Result<Vec<u8>, ReportGenerationError>;
}

pub struct PdfExportRequest {
    pub patient_id: i64,
    pub examination_id: i64,
    pub include_stage1_gate: bool,
    pub output_path: std::path::PathBuf,
    pub clinician_notes: Option<String>,
}
```

### B. Audit Persistence Table (`retinacare-persistence`)
Codex SQLCipher database must provision and expose:
```sql
CREATE TABLE IF NOT EXISTS audit_events (
    event_id TEXT PRIMARY KEY,
    timestamp_utc TEXT NOT NULL,
    event_type TEXT NOT NULL,
    patient_id INTEGER,
    examination_id INTEGER,
    user_id TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    prev_hash TEXT,
    signature TEXT
);
```

### C. Stage II / III AI Models
When available, models should integrate through the existing `InferenceEngine` trait in `rust/src/engine/model.rs`, providing typed segmentation masks and calibrated multi-class severity arrays.
