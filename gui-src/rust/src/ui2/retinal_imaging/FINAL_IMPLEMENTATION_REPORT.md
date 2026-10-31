# Retinal Imaging: final implementation report (2026-10-31)

Nothing committed, pushed, merged or reset. Result words: PASS / FAIL / NOT RUN / BLOCKED.

## 1. Already complete (earlier work, see QA_REPORT.md)
Page, sidebar entry (one), four real-output preview cards, overlays on a shared fitted rect, brightness, grid, disabled-with-reason controls, 12 page tests, screenshots at 1600x900, 1280x820, light/dark.

## 2. Incomplete found
No zoom/pan. Contrast, Saturation, Measurement, Save Image, Add to Study, Annotations, Color Map, Comparison tab, second-exam comparison: not implemented.
GPU: the page runs no inference (read-only, verified by grep: no infer/spawn/submit in the folder). CPU-saturation cause lives in the app's model stack, not this page.

## 3. Fixed this pass
- Zoom (wheel, max 8x) and pan (drag, bounded) on the primary and opened viewers; Reset clears. Display-only. `view.rs`, `mod.rs`.
- Root cause of "CPU 100 %, GPU 0 %": a bare `retinacare` binary ran with `RETINACARE_PROVIDER` unset, which means `auto`; `auto` silently falls back to CPU when CUDA libs are not on `LD_LIBRARY_PATH` (only an `eprintln!`). The installed launcher already sets `cuda`. Fix: `main()` now defaults `RETINACARE_PROVIDER=cuda` when unset (`main.rs`). Opt out with `auto` or `cpu`.
- Grad-CAM script fell back to CPU when torch had no CUDA. Under `RETINACARE_PROVIDER=cuda` it now exits with a clear error (`scripts/stage3_gradcam.py`); `RETINACARE_GRADCAM_DEVICE=cpu` remains an explicit opt-out.

## 4. Still unavailable
FA, ICG, OCT, Red-Free (no data). Contrast, Saturation, Measurement (no calibration), Save Image, Add to Study, Annotations, Color Map, cross-exam comparison. Occlusion explainability is not GPU-verified and is refused under strict cuda with a named error (existing design). Torch is not installed in the system `python3` here; the Grad-CAM interpreter is set by `RETINACARE_GRADCAM_PYTHON`.

## 5. Changed files
`ui2/retinal_imaging/{mod.rs,view.rs,tests.rs,README.md,QA_REPORT.md,FINAL_IMPLEMENTATION_REPORT.md}`, `src/main.rs` (provider default, 5 lines), `scripts/stage3_gradcam.py` (strict check).

## 6-9. Figma, screenshots, routing, controls
Figma comparison against the approved light screenshot: NOT RUN (file not supplied). Screenshots not retaken this pass; earlier ones in `qa/`. Routing: earlier PASS for one nav entry; sidebar clicks NOT RUN. Control status table: README.md "Controls".

## 11-13. Execution providers, GPU, CPU
Policy (code read, `device.rs`, `model_runtime.rs`): GPU-verified tasks run on CUDA device 0 with CPU fallback disabled (`with_disable_cpu_fallback`): Stage I, five Stage II models, vessel, Stage III five-fold. Strict mode refuses unverified tasks. Live check, Stage I only (backend test `local_stage1_image_runs_through_the_nonblocking_model_service`, `OD-1.png`, `RETINACARE_STAGE1_PROVIDER=cuda`, build with `--features retinacare-ai-bridge/cuda`): PASS, asserts `ActualProvider::Cuda { device_id: 0 }`, 30 s incl. load. 1 s sampling: idle 0 % / 13 MiB; during run GPU 11-12 %, VRAM 1321 then 2345 MiB of 8151; system CPU 18-78 % over the whole run (includes test build and image decode; not attributed per process). Samples are coarse (1 s); peak GPU is a lower bound. Stage II, Stage III, Grad-CAM and the full GUI cascade: NOT RUN. Without the `cuda` feature the same test fails with `CudaNotCompiled` (correct, explicit error). Earlier real run (memory/Testing/cuda-and-packaging.md): cascade 39.9 s on RTX 5070.

## 15. Tests
`cargo fmt` PASS; `cargo clippy --all-targets` PASS (exit 0); `cargo test retinal_imaging` PASS 12/0. Full GUI and backend suites, release build, native screenshots, UI-blocking check: NOT RUN this pass.

## 16. Risks
Strict-cuda default means a machine without CUDA libs now gets errors instead of a slow CPU run (intended). Not verified end to end after the default change.

## 17. Confirmation
No commits or pushes were made.

## Update: controls pass (later 2026-10-31)
Completed in code: Contrast, Saturation, simulated Red-Free, Save Image (original vs adjusted_view naming), temporary Annotations, pixel-only Measurement. Still unavailable: FA/ICG/OCT (no data), Add to Study (no study model), Color Map (fixed palettes), cross-exam comparison.
Changed files added: `adjust.rs`, `components.rs`, `view.rs`, `mod.rs`, `tests.rs`.
Verification: clippy PASS, 15 page tests PASS. Native GUI check of the new controls, screenshots, full suite: NOT RUN. CPU/GPU profiling of Stage II, Stage III, Grad-CAM and the full GUI path: NOT RUN; the CPU problem is NOT claimed solved.
Machine note: heavy builds/tests were stopped because the laptop was crashing under CPU load. Strict CUDA is the default (`RETINACARE_PROVIDER=cuda`); `RETINACARE_ORT_INTRA_THREADS=4` caps ONNX CPU threads if needed.

---
# Final pass: approved-reference fit, idle-CPU fix, rebuild (2026-10-31, evening)
No commit, push, merge or reset was run. Result words: PASS / FAIL / NOT RUN / BLOCKED.

## Reference used
`/home/claude/Downloads/report/download.png` (1448x1086, the approved light design; copy in `qa/approved_reference_1448x1086.png`). The Figma file `zV5EBNbVGXyo8kRJTN9F3d` page "Retinal Imaging" holds the earlier frames (node 68:228 etc.); no newer frame was found there. The brief JSON and annotated PNG are in `~/Downloads/report/`. `~/Downloads/RetinaCare_Light_Imaging_Figma_Handoff/` does not exist.

## Idle CPU: repaint loop
- Cause: `avatar()` in `src/ui/web_cards.rs` called `request_repaint_after(50 ms)` on every frame it was drawn, i.e. a 20 fps full-window redraw whenever an assistant message was on screen. The blink lasts about 0.14 s every 3.6 s.
- Change: repaint every 50 ms only inside the blink window, otherwise sleep until the next blink (`secs_to`). Drawing is unchanged. The `orb()` "searching" animation was not touched (it animates only while searching).
- Measured as app CPU over 6 s, idle, same saved chat, same Xvfb software-GL display (`llvmpipe`), old installed binary vs new release:

| Page | Before | After |
|---|---|---|
| Chat result page | 211 % of one core | 31 % |
| Empty Home | 19 % | 19 % |
| Retinal Imaging (new binary only) | not measured before | 27 % |

- Caveat: software GL inflates all numbers; on the laptop GPU the absolute values will differ. Avatar blink appearance was not visually compared; the draw code is unchanged. Residual 31 % vs 19 % is unexplained (could be the blink cycle or the larger page); not investigated further.

## Four known mismatches
| # | Mismatch | Fix (all in `retinal_imaging/`) | Evidence |
|---|---|---|---|
| 1 | Preview strip too short | strip height 27 % of body, clamp 120-250 (was 20 %, 104-150) `view.rs` | `qa/v2_1448x1086_*` |
| 2 | Right controls too narrow | dropdown width 60 to 110 px, full-width Reset `components.rs` | same |
| 3 | Header too tall | title 32 to 26 pt, body starts 58 px below (was 72), subtitle moved up `view.rs` | same |
| 4 | Selected colour | radio ring/dot, active tab border, selected card border now `accent_warm` (existing token) | `qa/v2_1448x1086_dark.png`, controls sheet |
Not pixel-compared. Visual check only: layout proportions and the five right cards now match the reference at 1448x1086 with no clipping. Known remaining differences: page title and subtitle kept (reference has none), header has no tab icons or settings/help/user icons (not in the app shell), preview cards show real outputs (Original, Vessel Mask, Lesion Overlay, Explainability) instead of the reference's FA/OCT tiles (no such data exists), Red-Free is a labelled simulation. At 1600x900 and 1280x820 the right column scrolls and the last card is cut (reference fits only at 1448x1086).

## Controls tested in the real window (release binary, Xvfb 1448x1086, `OD-1.png`)
| Control | Result |
|---|---|
| Red-Free (simulated) | PASS: grey green-channel view, caption says simulated |
| Annotations | PASS: two numbered markers placed on clicks |
| Measurement (pixels) | PASS: line + "361 px", note says pixels only |
| Contrast dropdown + value | PASS: pixels change; Reset restores |
| Reset | PASS |
| Brightness, Saturation | NOT RUN in window (same code path as Contrast; unit-tested earlier) |
| Zoom / pan | NOT RUN in window this pass (earlier unit test only) |
| Save Image (file dialog) | NOT RUN (native dialog not driven) |
| Export, Add to Study (disabled), Color Map (disabled), Grid | NOT RUN in window |
| Preview-card selection with real masks | NOT RUN (test exam has no saved masks; cards show unavailable states) |
| FA / ICG / OCT | disabled with reason: PASS (seen) |
| Dark theme, no-image state | PASS (screenshots) |
| Sidebar collapsed | NOT RUN |
| Other pages (Patients, Lesion, Vessel, Explainability, Reports) | NOT RUN this pass; only Home and the saved chat page were opened |

## Build
`cd "/home/claude/Desktop/Dird/GUI/Hackthon dezine/rust" && nice -n 19 ionice -c3 cargo build --release -j2`
Run 1 (avatar + layout): exit 0, 5 m 02 s. Run 2 (warm accent): `CARGO_EXIT=0`, 4 m 57 s. One build at a time. `file`: ELF 64-bit x86-64 PIE; `ldd`: 0 missing libraries. Tests (`cargo test`) were NOT run after these edits; the last test run (15 page tests) predates them.

## Binaries
- Release: SHA-256 `4153379ebf26608cbc60d488c4209c78773d422e86eec05c06f94403a5b397eb`, 74,842,912 bytes.
- Installed (unchanged, previous build): `5ef3fb43d4f7f9bf2174040f0103a5bcd3e13c02240e72270f77ea242b7b4012`, 74,835,232 bytes.
- Backup of installed: `~/.local/share/retinacare/bin/retinacare.bak-20261031`, SHA-256 identical to the installed file (verified).
- **Installation: BLOCKED.** A real RetinaCare process (PID 50925, `DISPLAY=:0`, started 18:29) was running from the installed path; I did not close it or replace its binary. Verification above used `target/release/retinacare`, not the installed app. Installed-app launch and GUI checks: NOT RUN.
- Manual install after closing RetinaCare (copies to a temp file, then renames atomically):
`cp "/home/claude/Desktop/Dird/GUI/Hackthon dezine/rust/target/release/retinacare" ~/.local/share/retinacare/bin/retinacare.new && chmod 755 ~/.local/share/retinacare/bin/retinacare.new && mv ~/.local/share/retinacare/bin/retinacare.new ~/.local/share/retinacare/bin/retinacare && sha256sum ~/.local/share/retinacare/bin/retinacare`
(expected hash is the release hash above).

## GPU
Not touched this pass beyond the earlier verified run (cascade 29.3 s on CUDA device 0, provider label `GPU (CUDA 0)` in the sidebar of every launch). No inference was run in this pass.

## Remaining
Install the new binary; click-test Save Image, zoom/pan, card selection with a saved exam, sidebar collapse; run `cargo test retinal_imaging`; compare pixels against the reference numerically; re-measure the idle chat page on the real GPU display; Ollama device and Grad-CAM/occlusion GUI runs (from earlier).
