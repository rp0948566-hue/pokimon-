# GUI Phase 1 • Task 06: View State Machines

## 1. Primary Navigation Flow

```mermaid
stateDiagram-v2
    [*] --> Directory
    Directory --> PatientDetail: Select Patient
    PatientDetail --> Directory: Return to Directory
    PatientDetail --> ExaminationHistory: View Examinations
    ExaminationHistory --> PatientDetail: Back to Detail
    ExaminationHistory --> ReportOverview: Open in Report [📋]
    ExaminationHistory --> QuadViewer: Open in Quad-Panel [⊞]
    ReportOverview --> ExaminationHistory: Back to Examinations
    QuadViewer --> ExaminationHistory: Back to Examinations
```

---

## 2. Directory View State Machine

```mermaid
stateDiagram-v2
    [*] --> EvaluatingReadiness
    EvaluatingReadiness --> DatabaseLocked: Backend Locked
    EvaluatingReadiness --> Loading: Read Allowed
    Loading --> Empty: 0 Patients Returned
    Loading --> Populated: >= 1 Patients
    Loading --> Error: Service Error
    DatabaseLocked --> Populated: Dev Fixture Mode (Isolated)
```

- **`DatabaseLocked`**: Renders banner explaining SQLCipher is locked; informs user that live patient records require clinician authentication.
- **`Empty`**: Renders zero-state illustration explaining no patients match the filter.
- **`Populated`**: Renders virtualized or scrollable table of patient cards with code, display name, exam count, and action buttons.
- **`Dev Fixture Mode`**: Clearly labeled with amber border tag `[DEV TEST FIXTURES ONLY - NON-CLINICAL]`.
