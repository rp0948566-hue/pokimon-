# GUI Phase 1 • Task 05: Test Results

## 1. Test Suite Execution Summary

- **Toolchain**: `rustc 1.84.1`, `cargo 1.84.1` on Ubuntu Linux x86_64
- **Manifest**: `rust/Cargo.toml`
- **Total Tests Executed**: 94 unit & integration tests
- **Passed**: 94
- **Failed**: 0
- **Ignored**: 0
- **Clippy Warnings**: 0 (`cargo clippy --all-targets -- -D warnings` passed cleanly)
- **Formatting**: Checked (`cargo fmt --check` passed cleanly)
- **Release Build**: Succeeded (`cargo build --release` produces binary `rust/target/release/retinacare`)

---

## 2. Test Breakdown by Domain Module

### Engine: Report Domain (`rust/src/engine/report.rs`)
1. `test_stage1_result_faithful_mapping_negative`: Verifies that a real Stage I score below threshold maps strictly to `Stage1GateDecision::NegativeRoute` with exact calibrated score and threshold.
2. `test_stage1_result_faithful_mapping_positive`: Verifies that a real Stage I score above threshold maps strictly to `Stage1GateDecision::ContinueCascade`.
3. `test_stages_2_and_3_marked_unavailable`: Proves that Stage II (lesion/vessel segmentations) and Stage III (DR Grades 0-4) are permanently marked `PipelineStageStatus::Unavailable` and never fabricated.
4. `test_mandatory_research_caveat_present`: Asserts that `MANDATORY_RESEARCH_CAVEAT` is strictly present on all constructed `ReportViewModel` instances.
5. `test_cross_patient_isolation_guard`: Ensures report view model validation blocks mismatched patient or exam IDs.

### Engine: Export & Audit Services (`rust/src/engine/export.rs`)
6. `test_live_codex_pdf_service_returns_unavailable`: Verifies that the production PDF service faithfully returns `PdfServiceReadiness::BackendUnavailable` because Codex has not implemented the generation engine.
7. `test_mock_pdf_service_lifecycle`: Verifies the asynchronous generation lifecycle (`Preparing` -> `Generating` -> `Saved`) and channel events.
8. `test_live_codex_audit_service_returns_unavailable`: Verifies that the production audit service faithfully returns `AuditLedgerReadiness::Unavailable` because Codex has not implemented the audit events table.
9. `test_mock_audit_ledger_service_events`: Verifies querying, filtering by exam ID, and appending audit records.
10. `test_scoped_patient_session_export`: Verifies JSON export contains only the active patient and exam data with zero leakage of other patients.
11. `test_duplicate_export_prevention`: Ensures a second export request while an active export is in progress is strictly rejected or guarded.

### Application Integration Tests (`rust/src/main.rs`)
12. `test_report_view_mode_toggle`: Asserts toggling between Quad-Panel viewer and Clinical Report Overview updates `RetinalExaminationView.report_view_mode`.
13. `test_export_pdf_modal_toggle`: Asserts opening and closing the PDF export modal.
14. `test_audit_ledger_modal_toggle`: Asserts opening and closing the audit ledger drawer.
15. `test_scoped_session_export_execution`: Asserts scoped patient session JSON export execution.

---

## 3. Full Test Command Output

```
running 94 tests
test domain::tests::test_create_image_analysis_request ... ok
test domain::tests::test_doctor_note_roundtrip ... ok
test domain::tests::test_retinal_view_default ... ok
test engine::backend::tests::test_backend_availability_check ... ok
test engine::backend::tests::test_mock_stage1_screening_run ... ok
test engine::backend::tests::test_payload_validation_missing_file ... ok
test engine::export::tests::test_duplicate_export_prevention ... ok
test engine::export::tests::test_live_codex_audit_service_returns_unavailable ... ok
test engine::export::tests::test_live_codex_pdf_service_returns_unavailable ... ok
test engine::export::tests::test_mock_audit_ledger_service_events ... ok
test engine::export::tests::test_mock_pdf_service_lifecycle ... ok
test engine::export::tests::test_scoped_patient_session_export ... ok
test engine::report::tests::test_cross_patient_isolation_guard ... ok
test engine::report::tests::test_mandatory_research_caveat_present ... ok
test engine::report::tests::test_stage1_result_faithful_mapping_negative ... ok
test engine::report::tests::test_stage1_result_faithful_mapping_positive ... ok
test engine::report::tests::test_stages_2_and_3_marked_unavailable ... ok
...
test tests::test_audit_ledger_modal_toggle ... ok
test tests::test_export_pdf_modal_toggle ... ok
test tests::test_report_view_mode_toggle ... ok
test tests::test_scoped_session_export_execution ... ok

test result: ok. 94 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
