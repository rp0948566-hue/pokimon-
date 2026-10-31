# GUI Phase 1 • Task 06: Clinical Workflow Research

## 1. Clinical Context & User Journey

RetinaCare serves mobile screening camps, hospital triage stations, and ophthalmology clinics. The clinical workflow requires strict sequence and verification:

1. **Patient Identification**:
   - Clinician searches patient directory using hospital identifier (`patient_code` / MRN).
   - If not found and database is unlocked, a new patient record can be registered.
2. **Context Selection**:
   - Clinician selects the patient, viewing historical examinations.
   - Any previous patient's cached imagery or unsaved inference results are purged from active memory.
3. **Examination Intake**:
   - Clinician initiates or selects an examination session.
   - Specifies Laterality (`OD` right eye, `OS` left eye) and operator ID.
   - Links fundus scan image (validating format, dimensions, and integrity).
4. **Analysis & Report Generation**:
   - Navigates from examination view directly into Quad-Panel Viewer or Clinical Report Overview.
   - Runs Stage I screening inference gate.
   - Preserves exact linkage between Patient ID, Examination ID, and Inference Result.
5. **Session Wrap-Up**:
   - Returns to Patient Directory or switches patient.

---

## 2. Laterality Rules (OD vs OS)

- `OD` (*Oculus Dexter*): Right eye.
- `OS` (*Oculus Sinister*): Left eye.
- **Rule**: Laterality must never be guessed or inferred by AI models from image appearance alone. It must be explicitly recorded by the operator or extracted from calibrated fundus camera metadata.
- **Rule**: A patient may have separate examinations for OD and OS on the same study date. The UI must present distinct visual cards for each eye.
