# 04 - Implementation Plan: Task 05

## Step-by-Step Execution Sequence

1. **Architecture & Module Design (`rust/src/engine/report.rs` & `rust/src/engine/export.rs`)**:
   - Define `ReportViewModel` and data contracts.
   - Define `PdfExportService` trait, `PdfServiceReadiness`, `ExportState`, and event channels.
   - Define `AuditLedgerService` trait, `AuditLedgerReadiness`, and `AuditEvent`.
   - Implement `LiveCodexPdfService` (reporting truthful backend absence).
   - Implement `LiveCodexAuditService` (reporting truthful audit log absence).
   - Implement mock services for deterministic unit testing.

2. **UI Integration (`rust/src/ui/report_view.rs` / `rust/src/ui/viewer.rs`)**:
   - Integrate Report Overview Panel in the Report workspace.
   - Add Export Action toolbar: `[📄 Export Clinical PDF]`, `[📦 Export Session Dossier]`, `[📋 Audit Ledger]`.
   - Implement Export Scope Dialog with opt-in checkboxes.
   - Implement Audit Ledger drawer / modal.
   - Maintain truthful unavailable states when backend services are not packaged.

3. **Application State & Nonblocking Polling (`rust/src/main.rs`)**:
   - Add report view state, export state machine, and audit ledger state to `RetinaCareApp`.
   - Drain async export events nonblockingly on every frame in `update()`.
   - Prevent duplicate export clicks while state is `Generating` or `SaveRequested`.
   - Wire examination reset and session isolation.

4. **Testing Suite (`rust/src/main.rs` & `rust/src/engine/`)**:
   - Write at least 10 new unit/integration tests covering:
     - Report view model generation from Stage 1 result.
     - Accurate missing-stage representation.
     - Session export scope isolation.
     - Export state transitions (Idle, Generating, Saved, Failed, Cancelled).
     - Save dialog cancel handling.
     - Duplicate request suppression.
     - Audit ledger query and filtering.
     - Truthful backend unavailable state handling.

5. **Desktop Verification & Real Capture**:
   - Compile release binary.
   - Launch application on Ubuntu desktop.
   - Capture real screenshots of Report Overview, Export Dialog, Backend Unavailable notice, and Audit Ledger view.
   - Verify zero compiler warnings under `clippy -D warnings` and clean `fmt`.

6. **Documentation & Memory**:
   - Complete remaining docs (`05` through `13`).
   - Update Obsidian memory vault.
