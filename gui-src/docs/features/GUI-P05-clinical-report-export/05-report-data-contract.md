# 05 - Report Data Contracts and Type Safety

## Typed Rust Data Contracts

```rust
use std::path::PathBuf;
use crate::domain::EyeLaterality;
use crate::engine::backend::{Stage1Decision, Stage1ScreeningResult};

/// View model representing a complete clinical screening report
#[derive(Debug, Clone, PartialEq)]
pub struct ReportViewModel {
    pub examination_id: String,
    pub patient_code: String,
    pub examined_at: String,
    pub operator: String,
    pub notes: String,
    pub laterality: EyeLaterality,
    pub image_path: Option<PathBuf>,
    pub stage1_result: Option<Stage1ScreeningResult>,
    pub stage2_status: PipelineStageStatus,
    pub stage3_status: PipelineStageStatus,
    pub model_id: String,
    pub model_checkpoint_hash: String,
    pub execution_provider: String,
    pub non_diagnostic_caveat: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineStageStatus {
    Available,
    PendingBackendIntegration(String),
    NotRequested,
}

/// Request configuration for exporting a clinical examination report
#[derive(Debug, Clone)]
pub struct ReportExportRequest {
    pub request_id: String,
    pub examination_id: String,
    pub patient_code: String,
    pub destination_path: PathBuf,
    pub include_original_image: bool,
    pub include_findings_json: bool,
    pub include_provenance_meta: bool,
}
```
