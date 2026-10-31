---
title: Bilateral assessment, heat-map smoothing, launch and push (2026-11-01)
type: status
status: PARTIAL
verified_on: 2026-11-01
---
# 2026-11-01 state

Link: [[GUI/retinal-imaging-figma-2026-10-31]], [[Handoffs/next-actions]], [[active-phase]].

## Done and built (installed exe sha256 `5a6272eb…b4288b`)
- Bilateral Assessment (Single/Dual Eye, OS/OD, Swap Eyes); chat "Which eye?" popup; per-card `View OS/OD results` button.
- Sidebar status card: detail text clipped to 24 chars, full text on hover (`ui2/shell.rs`).
- Retinal Imaging side panel: FA/ICG/OCT overlays, Add to Study, Color Map hidden (not deleted) until data exists.
- Occlusion heat map: display-only box blur `smooth_for_display` in `attribution.rs`; stored cells and labels unchanged. 17/17 attribution tests pass.
- `~/retinacare-fast.sh`: build (all cores) + install + launch with live progress; no-op build 1 s, full release 76-81 s.

## Verified in the real GUI (isolated Xvfb, CUDA device 0, 2026-11-01)
- OS analysis: Stage I 0.9968, grade 2, exam 36. Explainability occlusion: 325 forward calls, ~32 s, label "Occlusion sensitivity", smooth heat map, no 8x8 blocks, aligned with the fundus. VRAM 3.5 -> 4.7 GB of 8.1 GB. 0 errors in log.

## NOT verified
- Grad-CAM in the GUI (menu click missed; occlusion re-ran). Occlusion fallback was not forced.
- Per-page OS/OD toggle (Lesion, Vessel, Explainability, Retinal Review, Clinical Assessment, Research Findings, Report): not started.
- Swap Eyes and the View OS/OD click path with real saved results; 1280x820 and 1600x900 shots; full 318-test run.
- Window and navigation of the final launched app (PID check only).

## Git
- Pushed to `https://github.com/rp0948566-hue/pokimon-.git` main: `83ba999` -> `c6cebc2` (merge keeps remote initial commit). Code under `gui-src/` is a copy of `GUI/Hackthon dezine`; that repo has its own history and is unchanged. Local branch `push-main`.

## Gotchas
- GPU VRAM exhaustion makes the model load fail with "a required model is missing or could not be loaded"; stop other GPU users first.
- `run.sh` kills the pid in `app.pid`: never leave a real app pid there.
- A nested `.git` (GUI/Hackthon dezine) blocks `git add` of its files from the parent repo.
