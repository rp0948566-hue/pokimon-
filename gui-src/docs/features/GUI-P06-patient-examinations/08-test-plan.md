# GUI Phase 1 • Task 06: Test Plan

## 1. Test Matrix

### A. Engine Unit Tests (`rust/src/engine/patient.rs`)
1. `test_patient_service_query_filtering`: Search by code substring and case insensitivity.
2. `test_patient_service_empty_results`: Query yielding 0 records returns clean empty vector.
3. `test_patient_service_validation`: Invalid patient code (empty, > 64 chars) or name rejected with `InvalidInput`.
4. `test_mock_patient_service_crud`: In-memory patient creation and retrieval.
5. `test_examination_laterality_od_os_association`: Examination records maintain explicit `OD` vs `OS` distinctions.
6. `test_live_service_database_locked_handling`: Live service returns `DatabaseLocked` when unauthenticated.

### B. UI Component & Isolation Integration Tests (`rust/src/main.rs`)
7. `test_patient_page_navigation`: Switching between `Home`, `Report`, and `Patients`.
8. `test_patient_switch_clears_active_examination`: Switching patient invalidates previous exam ID and loaded image.
9. `test_patient_switch_drops_stale_inference_event`: Late-arriving inference result from previous patient is ignored.
10. `test_plaintext_export_dialog_security_warning`: Export session modal renders mandatory plaintext warning.
11. `test_directory_state_rendering_compact_window`: Renders without panic or clipping under 680x580 window constraints.

### C. Regression Gate
12. All 94 existing unit & integration tests from Tasks 01–05 must continue to pass without modification.
