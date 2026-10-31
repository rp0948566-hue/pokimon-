# 02 - Medical Report UX and Regulatory Research

## 1. Clinical Truthfulness & Non-Diagnostic Framing
- **Statutory Caveat**: RetinaCare is an investigational hackathon research tool, not a cleared SaMD (Software as a Medical Device).
- **Mandatory Disclaimers**: Every report view, export dialog, and preview must prominently display:
  > *"For investigational screening only - not a clinical diagnosis. Requires clinical examination by a licensed ophthalmologist."*
- **Binary Gate vs Clinical DR Grade**:
  - Stage I ConvNeXt-V2 produces a binary screening route (`NegativeRoute` vs `ContinueCascade`) based on threshold `0.2391716`.
  - Under no circumstances may this be presented as an ICDR 5-class Diabetic Retinopathy grade (0 = None, 1 = Mild, 2 = Moderate, 3 = Severe, 4 = Proliferative).
  - Raw sigmoid score \(\sigma(z)\) must never be termed "clinically calibrated risk" without external multi-center clinical validation evidence.

## 2. Report Overview UX Standards
- **Aspect-Ratio Preserved Thumbnail**: Fundus images must preserve exact optical proportions without stretching or warping.
- **Eye Laterality Clarity**: Prominently tag `OD` (Right Eye) vs `OS` (Left Eye) to prevent surgical or diagnostic laterality errors.
- **Model Provenance**: Record the model family, architecture, SHA-256 weight hash, and execution provider.

## 3. Local Export Security
- **No Unencrypted New Database**: Clinical exports must remain user-initiated, local-only files.
- **Native File Dialog**: Use OS native file save dialogs (`rfd::FileDialog`) ensuring the user explicitly chooses the destination folder and filename.
- **Sanitized Filenames**: Auto-generate safe default filenames (e.g., `RetinaCare_Report_OD_20261028.pdf`) stripping unsafe directory traversal characters.
