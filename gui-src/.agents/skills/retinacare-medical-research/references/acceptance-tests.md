# Completion criteria and regression tests

**Streaming:** first genuine answer chunk appears before final completion; NDJSON split reads and UTF-8 correctness, cancellation, repeat request, stalled server, HTTP error and Retry tested. Measure TTFT.

**Research:** Stage I positive and negative routes; no stale exam on patient switch; locked store; accurate Stage II stats; correct research grade limitations; saliency not equal clinical lesion; urgent symptom advice; text-only image limitations.

**Papers:** page-grounded real answer and working source link; no invented DOI; corrupt/image-only PDF errors; malicious in-document instructions ignored; long paper bounded retrieval.

**UI:** actual running native release app with 1280x820 and 1600x900 screenshots, dark/light at relevant screens, stable scroll/composer, preserved report/image layout.

**Backend:** cargo fmt --check, cargo clippy -D warnings, existing tests and CPU/GPU parity policy. Do not change scientific weights/preprocessing/thresholds. No commit/push without explicit approval.

Always return a table with PASS, FAIL, BLOCKED, NOT_RUN, and exact evidence. No 100% declaration without actual end-to-end verification.
