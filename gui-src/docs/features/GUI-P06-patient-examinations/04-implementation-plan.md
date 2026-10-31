# GUI Phase 1 • Task 06: Implementation Plan

## 1. Work Breakdown & Execution Phases

```mermaid
flowchart TD
    Step1["Step 1: Baseline Audit & Handoff Review"] --> Step2["Step 2: Engine Data Models & PatientService Trait"]
    Step2 --> Step3["Step 3: Service Implementations (Live, Mock, Unavailable)"]
    Step3 --> Step4["Step 4: Patients UI Component (Directory, Detail, Exams)"]
    Step4 --> Step5["Step 5: App State & Navigation Integration (main.rs)"]
    Step5 --> Step6["Step 6: Isolation & Session Sanitization Logic"]
    Step6 --> Step7["Step 7: Task 05 Export Labeling Review"]
    Step7 --> Step8["Step 8: Automated Unit & Integration Testing"]
    Step8 --> Step9["Step 9: Real Desktop Screenshot Verification"]
    Step9 --> Step10["Step 10: Documentation & Obsidian Memory"]
```

## 2. File Targets & Modifications

1. `rust/src/engine/patient.rs` (New): Data contracts, error models, service traits, live adapter, mock service, and unit tests.
2. `rust/src/engine/mod.rs` (Modify): Re-export `patient` module.
3. `rust/src/ui/patients.rs` (New): Patient directory UI, patient detail editor, examination list, image linker, and navigation toolbar.
4. `rust/src/ui/mod.rs` (Modify): Re-export `patients` module.
5. `rust/src/domain.rs` (Modify): Add patient and examination routing references.
6. `rust/src/main.rs` (Modify): Add `AppPage::Patients`, wire patient service, handle navigation actions, enforce isolation on switch, and add integration tests.
7. `rust/src/engine/export.rs` (Modify): Explicitly label `export_scoped_patient_session()` as plaintext sensitive export per instruction 12.
