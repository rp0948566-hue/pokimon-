# 08 - Testing Strategy: Task 05

## Test Plan Structure
1. **Report View Model Verification**:
   - Construct `ReportViewModel` from active examination and Stage 1 screening result.
   - Verify that Stage 1 score, logit, threshold, and route are faithfully mirrored.
   - Verify that Stage 2 (lesions) and Stage 3 (grading) are marked `PendingBackendIntegration`.
2. **Export State Machine Testing**:
   - Verify transition from `Idle` -> `Generating` -> `Saved`.
   - Verify cancellation handling when save dialog is aborted.
   - Verify duplicate export clicks are rejected when state is busy.
   - Verify stale request ID rejection.
3. **Session Export Scope Isolation**:
   - Verify that export payloads contain only data associated with the active examination ID.
   - Verify that patient code matches active patient reference.
4. **Audit Ledger Queries & Unavailable States**:
   - Verify `AuditLedgerReadiness::Unavailable` handling when backend table is absent.
   - Verify event filtering by examination ID using mock service.
5. **Full Regression Suite**:
   - Ensure all 82 existing tests continue passing without regression.
