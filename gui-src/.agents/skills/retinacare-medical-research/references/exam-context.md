# Grounded selected-examination context

Only provide facts from the currently selected, authorized examination. Do not silently switch to another patient or use the first stored exam. Read-lock the encrypted store and require access authorization; no leakage when locked or patient changed.

Prefer structured typed Rust data with provenance: exam ID, OD/OS, original image identifier, Stage I score/threshold/route, Stage II masks and stats, Stage III fold results if present, explainability method/target, execution devices/timings, saved report metadata and explicit `not_run`/`missing` states. Screen-negative exams must **not** be narrated as if segmentation and grading ran.

`llama3.2:3b` is text-only: provide derived trusted results, not the raw image as if the model could see it. Treat all generated explanations separately from clinician notes and existing signed/verified data.

Recommended read-only tool interfaces: `get_current_exam_summary`, `get_saved_stage_results`, `get_report_sections` and `search_uploaded_papers`. Validate tool arguments server-side and never grant arbitrary filesystem access.
