# Bilateral Assessment: final report (2026-10-31)

No Git commit, push, merge or reset. Existing patient data untouched. Words: PASS / FAIL / BLOCKED / NOT RUN.

## What it is
Optional page reached from Retinal Imaging (header tab "Bilateral", in the slot of the former disabled "Comparison" tab). Default mode Single Eye points to the existing single-eye workflow. Dual Eye has two independent slots (OS, OD). No model, threshold, checkpoint or backend stage was changed. Each eye is handed, one at a time, to the existing `start_analysis` cascade (CUDA policy unchanged); the verified outcome is filed under that eye.

## Files
New, all in `src/ui2/bilateral_assessment/`: `mod.rs`, `intake.rs` (content sniff, limits, hashes, quality heuristics, duplicate/similarity), `state.rs` (per-eye state machine), `view.rs` (egui page), `tests.rs` (26 tests), this report.
Seams (small): `src/ui2/mod.rs` (+1 line); `src/main.rs` (`AppPage::Bilateral`, `bilateral` field, page arm, sidebar highlight maps to Retinal Imaging, `render_bilateral`/`bilateral_pick`/`bilateral_request`/`bilateral_pump`/`bilateral_on_finished`, one hook line in `finish_cascade`, one pump line per frame); `src/ui2/retinal_imaging/{mod.rs,components.rs,tests.rs}` (Comparison placeholder replaced by a "Bilateral" tab, action `OpenBilateral`, one test updated).
Earlier today (separate task, already installed): WebP/BMP backend support, `native/` files listed in `docs/bugfix-webp-validation-2026-10-31.md`.

## Behaviour
- Per eye: file sniffed by content (JPEG/PNG/WebP/BMP), 64 MiB / 40 Mpx / 64 px limits, SHA-256 of file and of decoded pixels, 16x16 difference-hash for similarity, experimental quality flags.
- Laterality is always "provided by user, not AI verified" (checkbox). There is no trusted metadata source or validated model, so none is claimed.
- Exact duplicate (same bytes, or same decoded pixels in another format): all runs blocked with the message "Both slots contain the same image".
- Similar or mirrored photo: warning plus "I checked: these are different eyes" before Analyse both. Not proof of eye identity.
- Quality flags (blur, dark, bright, low contrast, glare, small field) are experimental, unvalidated thresholds, set from two example photos. Flagged photo = "Needs review / retake", requires explicit acceptance. They never produce a grade or a pass certificate.
- Run: nothing starts on open, toggle, upload or navigation. Analyse OS only / OD only / both (OS then OD, one cascade at a time). A second click while queued is ignored. First-eye failure does not auto-continue. Cancel keeps finished eyes.
- Result filing needs matching request id, patient, image hash, eye label, slot token. A failed or cancelled eye is "Not assessed", never Grade 0. No combined grade or confidence.
- Mode switch while running, replacing the image of a running eye, and patient change are guarded (refuse / wipe session and cancel the job).
- Persistence: each eye's run is saved by the existing single-eye path as its own examination (shown "Examination 23 / 24"). The OS+OD pairing itself is session-only: lost on restart, not saved.

## Tests
| Check | Result |
|---|---|
| 26 new unit tests (`cargo test bilateral`) | PASS |
| Full GUI suite `cargo test -j2` (318 passed, 9 ignored) incl. retinal_imaging and single-eye cascade-flow tests | PASS |
| Release build `nice -n 19 ionice -c3 cargo build --release -j2`, twice (second after a layout fix) | PASS, exit 0 |
| Native GUI, Xvfb 1600x900: open from Imaging tab, Single/Dual, uploads (PNG + WebP), laterality and review checkboxes, Analyse both | PASS |
| Real CUDA run, OS = demo PNG, OD = supplied WebP, double click on Analyse both | PASS: OS then OD, one task each, "Provider: CUDA device 0", 25.0 s and 20.1 s, exams 23 and 24, grades 3 and 4 with the same Stage I scores as the earlier single-eye runs (0.9983 and 0.9976) |
| Per-eye vessel overlay switch (OS and OD each show their own mask) | PASS |
| Exact duplicate in both slots blocks Analyse (GUI) | PASS |
| Light 1600x900, dark 1280x820 empty dual page | PASS; at 1280 the right card edge is clipped by about 3 px (minor, not fixed) |
| 1448x1086, sidebar collapsed, keyboard-only navigation, screen reader | NOT RUN |
| Failure, cancel, patient switch during a real run | unit-tested only; NOT RUN in the live window |
| GPU out of memory / CPU fallback during a bilateral run | NOT RUN (earlier WebP run showed fallback and a mixed-device ensemble rejection; the page would show that eye as Failed) |
| EXIF orientation, CMYK/16-bit, non-retinal photo detection | NOT IMPLEMENTED (no validated classifier; orientation not normalised) |
| Per-eye export, bilateral summary export, per-eye Grad-CAM/occlusion | NOT IMPLEMENTED (page says use the single-eye pages) |
| Near-duplicate detection accuracy on real photo pairs | NOT MEASURED (heuristic; synthetic tests only, false positives and misses possible) |

## Matrix status (64 cases)
PASS by unit and/or GUI test: UPLOAD-01..11, 13, 14, 16, 18; FORMAT-01..05; QUALITY-01..03, 05; RUN-01, 02, 04..09; STATE-02, 03, 07; UI-01, 02; REG-01 (suite).
Design-only (no live test): STATE-04, 06, UPLOAD-17 live, RUN-03, RUN-10 (heatmaps not produced here), UI-03, UI-06.
NOT RUN / NOT IMPLEMENTED: UPLOAD-12 (two photos of one eye cannot be detected without trusted evidence; user verification only), UPLOAD-15, FORMAT-06, FORMAT-07, QUALITY-04, 06, 07 (resolution recorded, not validated), RUN-11, RUN-12, STATE-01, STATE-05, UI-04 (partly), UI-05, EXPORT-01..03, REG-02 (other pages not clicked), REG-03 (page idle repaint not measured).

## Known limits
- Running an eye replaces the image and laterality in the single-eye workspace (it is the existing pipeline). The page that shows the other single-eye image has to reload it.
- Cannot guarantee OD/OS: user assertion only. A same-eye pair of different photos will not be caught.
- Quality thresholds are unvalidated.
- Test eye labels in the live run were arbitrary (demo PNG as OS, WebP as OD), not true laterality.

## GPU
Single process peak VRAM 4718 MiB of 8151 MiB during the run (nvidia-smi sample). One cascade at a time.

## Install
Backup `~/.local/share/retinacare/bin/retinacare.bak-20261031-bilateral` (sha256 96bb2e1f64093467…, verified). Installed sha256 0cca9298ecb32b70… equals `target/release/retinacare`. Installed app launched via `retinacare-launch`. Installed Bilateral page itself not re-opened after the install (the same binary was tested from `target/release`).

## Screenshots
Scratchpad `/tmp/claude-1000/-home-claude-Desktop-Dird/133996a1-6e50-4249-9feb-216efd67039e/scratchpad/`: `bil_a_1page.png`, `bil_b_2uploaded.png`, `bil_b_4running.png`, `bil_b_5done.png`, `bil_b_6results.png`, `bil_b_7vessel_crop.png`, `bil_c_dup.png`, `bil_d1600x900_dualempty.png`, `bil_e_1.png`.

## Figma restyle pass (2026-10-31, later)
No commit, no push. Figma file `zV5EBNbVGXyo8kRJTN9F3d`, page "Retinal Imaging", frames A 83:55, B 83:321, C 83:616, D 84:104, E 84:428, F 84:762, G 84:1096, H 84:1532, I 84:1953.
Rust changed: `view.rs` only (flat status text, larger buttons, taller viewers, short result card with collapsed details, 12 px side margin for the 1280 clip). State, intake, cascade hooks untouched.

| Check | Result |
|---|---|
| `cargo check --tests` | PASS |
| `cargo test bilateral` | PASS, 26 passed |
| `cargo build --release -j2` (nice/ionice) | PASS, BUILD_EXIT=0 |
| Full GUI suite (318 passed, 9 ignored) | NOT RUN after restyle |
| Figma D visual check; B, C, F individual checks | NOT RUN (screenshot call failed / skipped) |
| Live GUI screenshots 1448x1086, 1600x900, 1280x820 | NOT RUN (shell denied access to the new release binary) |
| 1280 clip fix | NOT VERIFIED visually |
| Backup + install + SHA-256 | NOT RUN. Installed sha256 still 0cca9298ecb32b70 (previous Bilateral build). App process 112998 running from the installed path; not force-closed |
| OS/OD/Both validation + duplicate not blocking single-eye runs | unit tests only (PASS); live NOT RUN |

## Installed-binary verification (2026-10-31, 21:33 install)
No commit, no push. Installed `~/.local/share/retinacare/bin/retinacare` sha256 2421bf5a6d531d16…, backup `retinacare.bak-20261031-213304` (made by the user's install). Release-vs-installed hash comparison: NOT RUN (reading `target/release` was permission-denied; user installed it and ran `cmp`). The user's own app (pid 150656) was left open. Checks below used an isolated Xvfb :99 copy of the installed binary with a throwaway data dir.
Dir: `/tmp/claude-1000/-home-claude-Desktop-Dird/133996a1-6e50-4249-9feb-216efd67039e/scratchpad/`

| Check | Result |
|---|---|
| Installed binary starts, Retinal Imaging, header tab "Bilateral" opens page | PASS (`bil_i1_0imaging.png`, `bil_i1_1bil.png`) |
| Single Eye default (frame A) | PASS; page still shows a helper paragraph ("This is the existing workflow...") that Figma A hides. FAIL vs Figma |
| Dual empty (frame B) 1600x900, 1280x820 | PASS (`bil_i1_2dual.png`, `bil_t1280_dual.png`): two cards, Upload/Remove/Analyse buttons, Not assessed results. Analyse buttons disabled with no image |
| Helper lines "JPEG, PNG, WebP or BMP" and the eye-side notice | still shown; Figma hides them. FAIL vs Figma |
| 1280x820 right-card clip | PARTIAL: whole card visible with rounded border, but its right edge sits flush against the scrollbar (x 1241). No text cut. Not a clean margin |
| 1600x900 right card | right edge also flush with scrollbar (x 1561) |
| 1448x1086 | NOT RUN: the Bilateral page opens only after clicking the tab; the harness shot caught Imaging (`bil_t1448_0.png`) |
| Frames C, D, E, F, G, H live (uploads, duplicate, run, results) | NOT RUN: `RETINACARE_BILATERAL_OS/OD` preload did nothing in this build (cards stayed "not uploaded"); file picker not scripted |
| OS-only / OD-only / Both gating, duplicate not blocking single-eye | unit tests PASS (26); live NOT RUN |
| CUDA pipeline | untouched; no analysis run; header shows "GPU (CUDA 0)" |
| Idle CPU | NOT RUN |

## Duplicate rule fix + full re-test (2026-10-31, night)
No commit, no push.
Found live (installed binary, same image in OS and OD, OS confirmed): "Analyse OS only" stayed disabled. Cause: `Session::blockers(eye)` added the exact-duplicate message, so it blocked single-eye runs. Spec: duplicate blocks only Analyse both.
Fix: `state.rs` moves the exact-duplicate blocker from `blockers(eye)` into `blockers_both()`. Test `upload_09_...` renamed `..._block_both_but_not_one_eye` and now asserts that OS alone has no "same image" blocker. No model, threshold, cascade or CUDA code touched.

| Check | Result |
|---|---|
| `cargo test bilateral` | PASS, 26 passed |
| Full `cargo test -j2` | PASS, 318 passed, 0 failed, 9 ignored (baseline met) |
| `cargo build --release -j2` after fix | PASS, BUILD_EXIT=0 |
| Live: both slots same image blocks Analyse both, red message shown | PASS (`bil_dup2.png`, installed pre-fix binary) |
| Live: OS laterality tick shows "Eye side: User confirmed" | PASS (`bil_os_done.png`) |
| Live: Analyse OS only with duplicate present | FAIL on installed pre-fix binary (the bug above); fixed in source, unit-tested; live re-check NOT RUN until new build is installed |
| Live: OS-only, OD-only, Both real CUDA runs on new build | NOT RUN (needs install) |
| New build installed, hash match | NOT RUN: user must close the app and install (access to `target/release` is denied to me) |

## Final delivery pass (2026-10-31, 23:00)
Status labels: PASS / FAIL / BLOCKED / NOT RUN. NOT RUN is never counted as PASS.

### Changed source
- `view.rs`: `single_panel` side-by-side with top-down layout (the first attempt used a horizontal layout and pushed the "Need both eyes?" card off-screen); new `need_both_card`, `swap_row`; scrollbar gutter (`available_width() - 16`); format helper line removed.
- `state.rs`: `Session::swap_eyes`, `swap_pending`.
- `tests.rs`: `swap_01`, `swap_02`.
- `src/explain_app.rs`: Grad-CAM -> occlusion sensitivity fallback (`explain_fallback`, once per request, real method label).
- `src/main.rs`: chat "Which eye are you uploading?" window (`EyeChoice`, `eye_choice_window`, `eye_thumb`), up to two chat images, cancel on patient change.

### Results
| Item | Status | Evidence |
|---|---|---|
| Installed binary (22:25) has duplicate-rule fix; duplicate blocks only Analyse Both | PASS | live GUI, earlier section |
| New build, Single Eye: Figma A layout, right card visible (1448x1086) | PASS | scratchpad `m1_1.png` (build before swap/chat edits) |
| New build, Dual Eye empty layout (1448x1086) | PASS | `p1_2.png` |
| 26 Bilateral tests | PASS (before swap edits) | 26 passed, 0 failed |
| `swap_01`, `swap_02` and full suite (318 baseline) | NOT RUN | run crashed the laptop at 100% CPU; stopped by user |
| Chat "Which eye?" window in the GUI | NOT RUN | compiled only |
| Swap Eyes in the GUI | NOT RUN | compiled only |
| Grad-CAM -> occlusion fallback in the GUI | NOT RUN | compiled only |
| Screenshots 1600x900 and 1280x820 | NOT RUN | |
| Real CUDA OS+OD run, provider per stage | NOT RUN | header "GPU (CUDA 0)" is not evidence |
| Install of the final build | BLOCKED | `cp` into `~/.local/share/retinacare/bin/` was denied; not bypassed. Backup `retinacare.backup-20261031-225527` (75668896 bytes) is intact by size |
| Byte / SHA-256 compare release vs installed | BLOCKED | reading `target/release/retinacare` denied earlier |

### Known limits
- The full 318-test suite was not re-run after the swap and chat edits.
- Eye side is never verified by the app; the user states it.

### Install check (23:06, done by the user in their terminal)
- Install of the 22:54 release build (layout fix, Grad-CAM fallback, first chat prompt): PASS. SHA-256 of release and installed binary match: `7dbaec05b63261a6ae8a01d24f07cc0a13efd849ad05e6b06a52bf5465ab584d`.
- That build does not contain Swap Eyes or the chat thumbnails. Install of the final build: NOT RUN (CPU-capped build still running).

### Real GUI test of the installed 22:54 build (23:08, isolated Xvfb, throwaway data, CPU-capped)
Images: `OD-1.png`, `OI-1.png` from `dist/demo-images/session-1-final` (existing anonymized fixtures). Screenshots in `screenshots/`.
| Test | Status | Evidence |
|---|---|---|
| Two images attached in chat, Send opens "Which eye?" window; nothing runs | PASS | `chat_which_eye_2images_1448x1086.png` |
| Cancel restores both attachments to the composer | PASS | `chat_cancel_restores_2files_1448x1086.png` |
| "Both eyes" opens Bilateral, Dual Eye, images in OS then OD slots, eye-side boxes unchecked, all Analyse buttons disabled, status "Needs input" | PASS | `chat_both_eyes_to_bilateral_1448x1086.png` |
| Attachment order is not treated as proof of laterality | PASS | slots show "Eye side: Not verified"; user must tick each box |
| Thumbnails and per-image OS/OD assignment in the chat window | NOT RUN | in source only; this installed build predates it |
| Swap Eyes | NOT RUN | in source only |
