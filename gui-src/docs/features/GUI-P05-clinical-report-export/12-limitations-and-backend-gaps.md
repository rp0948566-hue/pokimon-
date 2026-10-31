# 12 - Limitations and Backend Gaps

## 1. Missing Backend PDF Generation Engine
- **Current State**: `retinacare-reports` defines only a basic `ReportData` struct in `/home/claude/Desktop/Dird/native/crates/retinacare-reports/src/lib.rs`.
- **Gap**: There is no function `generate_pdf(&ReportData, &Path) -> Result<(), ...>` or any PDF writer (such as `printpdf`, `genpdf`, or `lopdf`).
- **Mitigation in GUI**: Truthful display of `BackendUnavailable` state with detailed technical instructions for Codex.

## 2. Missing Backend Audit Event Persistence
- **Current State**: `retinacare-persistence` (`SqlCipherStore`) manages patient and examination records.
- **Gap**: There is no `audit_log` table, nor methods `record_audit_event()` and `query_audit_events()`.
- **Mitigation in GUI**: Truthful display of `AuditHistoryUnavailable` state.

## 3. Path Dependency Layout
- **Current State**: `rust/Cargo.toml` links `retinacare-app` and `retinacare-ai-bridge` via `../../Dird/native/crates/...`.
- **Risk**: Hardcoded relative path fails if repos are moved independently.
