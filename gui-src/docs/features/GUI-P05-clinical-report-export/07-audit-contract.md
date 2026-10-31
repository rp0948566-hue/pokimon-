# 07 - Audit Ledger Contract and Authenticity

## 1. Minimal Audit Event Contract
```rust
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEventStatus {
    Success,
    Failed,
    Cancelled,
    Informational,
}
```

## 2. Integrity and Truthfulness Guarantees
- No synthetic audit events are created in production mode.
- If backend persistence (`SqlCipherStore`) does not have an audit log table, the UI displays `AuditLedgerReadiness::Unavailable`.
- Only events acknowledged as persisted by backend are reported as logged.
- No certification claims (HIPAA, FIPS) are made.
