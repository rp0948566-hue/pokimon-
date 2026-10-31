# Promotion plan (NOT executed)
No candidate exists: no real training run has happened (all datasets BLOCKED). Nothing to promote.
When a candidate passes `evaluate_candidate.py --final` and `export_onnx.py` parity, future work (needs explicit user approval, isolated branch):
1. Add candidate ONNX + manifest under `models/` with sha256; keep Stage I/III files untouched.
2. Wire a separate model id in the Rust bridge; do not change deployed thresholds.
3. Re-run native tests, CPU/GPU parity, GUI cascade, PDF export, disclaimers.
Rollback: delete the new model id/manifest; deployed models never modified.
