# Retinal Imaging: QA report (2026-10-31)

Base: branch `test`, HEAD 010d906, clean tree before the work. Nothing committed, pushed, merged or reset.
Result words: PASS / FAIL / NOT RUN / BLOCKED. Evidence is real output from this machine.

## Figma (Phase 1)
- File: https://www.figma.com/design/zV5EBNbVGXyo8kRJTN9F3d, page "Retinal Imaging".
- Frames 1600x900 (68:228), 1448x1086 (69:10), 1280x820 (69:241); opened-preview frames 72:23, 72:263, 72:503, 72:743.
- Built earlier in this same session from the app's own tokens, before this brief arrived. Not rebuilt from the new reference image, and not compared against `APPROVED_LIGHT_THEME.png` (file not supplied; only the PDF copy). **F04 NOT RUN.**
- No Figma tool was used after the Rust work began.

## Baseline vs after
| Check | Baseline | After | Result |
|---|---|---|---|
| `cargo fmt --all -- --check` | exit 0 | exit 0 | PASS |
| `cargo test` (GUI) | 277 passed, 0 failed, 9 ignored | 289 passed, 0 failed, 9 ignored (+12 new) | PASS |
| `cargo clippy --all-targets` | not run | exit 0, 2 warnings, both in this folder's tests (fixed afterwards, not re-run) | PASS with caveat |
| Backend `cargo test --workspace --manifest-path native/Cargo.toml` | not run | run interrupted, no output | **NOT RUN (F20)** |
| Release build | not run | not run (debug build used) | **NOT RUN (F21)** |
After the clippy fix and the last edits no test was re-run, on the user's instruction (no more tests).

## Screenshots (real X11 window grabs, `qa/`)
| File | Viewport | Theme | State |
|---|---|---|---|
| 1600x900_light_no_image.png | 1600x900 | light | no image: empty state, Open Retinal Review |
| 1600x900_light_one_image.png | 1600x900 | light | one image, empty comparison, 3 cards empty |
| 1280x820_light_one_image.png | 1280x820 | light | same |
| 1280x820_dark_one_image.png | 1280x820 | dark | same |
| 1600x900_light_existing_home / _patients / _retinal_review.png | 1600x900 | light | existing pages, unchanged, new sidebar item visible |
The image used is `tests/fixtures/synthetic_fundus_od.png`, a synthetic fixture, not patient data.

## Acceptance matrix
| ID | Result | Evidence |
|---|---|---|
| F01 one nav item opens page | PASS | screenshots: one "Retinal Imaging" entry, active style |
| F02 sidebar collapsed/expanded | NOT RUN | synthetic clicks do not reach the XWayland window |
| F03 existing navigation | PARTIAL | Home and Retinal Review inspected unchanged via `RETINACARE_PAGE`; Patients captured but not inspected; Report doc not inspected; sidebar clicks NOT RUN |
| F05 / F06 1600x900 / 1280x820 | PASS | screenshots, no clipping; right rail scrolls (last card cut at bottom) |
| F07 empty exam | PASS | no-image screenshot |
| F08 one image, no comparison | PASS | screenshot |
| F09 two authorized images | NOT RUN | app has no second-exam source; comparison is same-exam only |
| F10 cross-patient comparison | PASS by design | no cross-exam image path exists; state resets on key change (unit test) |
| F11 exam switch | PASS | `switching_exam_or_eye_clears_every_selection` |
| F12 source missing | PASS | no-image screenshot, no substitution |
| F13 exactly four previews | PASS | screenshots + `four_previews_...` |
| F14 no FA/ICG/OCT as real | PASS | radios disabled with reason; card labels tested |
| F15 overlays only from saved artifacts | PARTIAL | availability logic tested with no artifacts; overlay drawing with real masks NOT RUN (no analysis run) |
| F16 Figma lifecycle | PASS | see above |
| F17 isolation | PASS | all page code in `rust/src/ui2/retinal_imaging/` |
| F18 minimal integration | PASS | see list below |
| F19 GUI tests | PASS | 289/0/9 |
| F20 backend tests | NOT RUN | |
| F21 release build | NOT RUN | |
| F22 no real DB writes | PASS | page is read-only; app launched with no DB action; no write code in folder |
| F23 no git mutation | PASS | `git status` shows only the files below; no commit, push or reset |
| F24 light/dark | PASS | screenshots |
| F25 enhancement + reset | PARTIAL | brightness bounds/reset tested; Contrast/Saturation not implemented (disabled) |
| F26 grid/annotations | PARTIAL | grid code present, visual NOT RUN; annotations disabled (none stored) |
| F27 select each preview | PARTIAL | Original opens/Back tested with real pointer events; Vessel/Lesion/Explainability need saved artifacts, NOT RUN |
| F28 export | PARTIAL | Export routes to existing report page only when a saved result exists; not exercised |
| F29 keyboard focus | NOT RUN | focus ring coded, not checked |
| F30 1448x1086 | NOT RUN | |
| F31-F33 Retinal Review, PDF, chats unchanged | PARTIAL | Retinal Review and Home render unchanged; PDF export, chats NOT RUN |
| F34 restart | NOT RUN | |
| F35 CUDA/CPU | PASS | page does no compute and sets no provider; no GPU or driver setting touched |
| F36 disabled controls labelled | PASS | hover reasons in code; tooltip screenshot NOT captured |

## New tests (12, all passing in the last run)
availability without image / with only original; exam or eye switch clears selection; brightness bounds and reset; four labels with no FA/ICG/OCT/Red-Free; missing-artifact preview auto-closes; empty exam and small window draw; real pointer clicks: card opens, Back closes (and is absent before opening), unavailable card does not open, header tabs route only to existing pages (Comparison inert), rail clicks never switch on overlays without artifacts.

## Existing files changed (all minimal)
- `ui2/mod.rs`: `pub mod retinal_imaging;`
- `ui2/shell.rs`, `ui2/icon_paths.rs`: new `Icon::Imaging` glyph (the 12th icon).
- `main.rs`: `AppPage::RetinalImaging`; `PAGES` 11 to 12 and one `nav` entry after Retinal Review; one state field and its default; one dispatch arm; one no-op arm in the legacy sidebar match (not reachable there); `RETINACARE_PAGE=imaging` dev value.
Retinal Review (`AppPage::Report`) is untouched.

## Not built (stated, not hidden)
Contrast and Saturation, Measurement, Save Image, Add to Study, Annotations, Color Map, Comparison tab. Red-Free, FA, ICG and OCT have no data in this app, so those controls stay disabled. The approved image's FA/OCT preview tiles are replaced by Original Fundus, Vessel Mask, Lesion Overlay, Explainability.

## Known issues
- Right rail needs scrolling at 1600x900 and 1280x820 (last card partly hidden).
- Left photo shows the fixture's own dark square background; real photos may differ.
- Preview cards for masks show "No saved ..." until an analysis has run for the open exam.

## Rollback
Remove the `PAGES` entry and the `nav` entry in `RetinaCareApp::chrome_v2` (main.rs): the page is unreachable and nothing else changes. Full removal: also delete this folder and the lines listed above. No `git reset`.

## Update 2026-10-31 (later): zoom/pan and GPU policy
- Added display-only wheel zoom (max 8x) and drag pan on the primary viewer and the opened view; `Reset` clears them. Test extended: `brightness_is_bounded_and_reset_is_neutral`. `cargo clippy --all-targets` exit 0; `cargo test retinal_imaging` 12 passed, 0 failed. PASS.
- GPU: see FINAL_IMPLEMENTATION_REPORT.md. Full GUI suite, screenshots, real GPU run: NOT RUN in this pass.

## Update 2026-10-31 (controls pass)
Code written: `adjust.rs` (contrast, saturation, red-free, export), markers and pixel ruler in `view.rs`, wired in `components.rs`; 3 new unit tests (pixel maths, red-free, reset).
Results so far: `cargo clippy --all-targets` exit 0 and `cargo test retinal_imaging` 15 passed, 0 failed (run before the last test rename only). Full GUI suite: NOT RUN (stopped on user instruction, CPU load). Native screenshots, Save Image dialog, click-placement of markers in the real window: NOT RUN.
