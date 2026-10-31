# GPU-first execution report (2026-10-31)

No commits, pushes, merges or resets. Result words: PASS / FAIL / NOT RUN / BLOCKED.

## Setup checked
| Item | Result |
|---|---|
| GUI `Cargo.toml` enables `retinacare-ai-bridge` feature `cuda` | PASS |
| Installed launcher (`~/.local/share/retinacare/bin/retinacare-launch`) default `RETINACARE_PROVIDER=auto` | PASS (backup `.bak-20261031`) |
| `launcher.env`: `CUDA_LIB_DIRS` (cudart 13, cublas 13, cufft 12, cudnn 9 present), `RETINACARE_MODELS_DIR`, `export RETINACARE_GRADCAM_PYTHON` (Training-Lab venv, torch 2.15.0+cu132, CUDA available, timm present) | PASS |
| Installed binary `~/.local/share/retinacare/bin/retinacare` | **Old build (Oct 31 00:54). NOT replaced.** The new release build is in the GUI `target/release/`. |

## Code changes this pass
- `device.rs`: `record_gpu_fallback` / `gpu_fallbacks` (stage, reason, actual device).
- `model_runtime.rs`: occlusion explainability follows its model's GPU-verified task; a CPU fallback is recorded.
- `main.rs`: default provider `auto` when unset; sidebar status shows `CPU fallback` + the message when one was recorded.
- `scripts/stage3_gradcam.py`: strict `cuda` refuses CPU Grad-CAM.
- Fallback banner: written, built, **never triggered, so NOT RUN**.

## Build
`nice -n 19 ionice -c3 cargo build --release -j2`: PASS, 5 min 15 s, single build. Backend bridge `model_runtime` unit tests: 5 passed. Provider-mode test under strict cuda: stage1=cuda, stage3=cuda, explain=cuda (103 s).

## One native GUI analysis (Xvfb 1600x900, built binary, `OD-1.png`, temporary data dir, no patient data touched)
- Status footer: `Backend ready - GPU (CUDA 0)`. No fallback banner, no fallback line in `app.log`: PASS.
- Cascade finished: Stage I score 0.9983 (threshold 0.2392), MA 58 px, HE 1574 px, EX 8997 px, CWS 400 px, vessels 47520 px, Stage III five-fold technical grade 3. Card text: "Total run time 29.3 s on CUDA device 0".
- Stage times shown in the card: Stage I 2.6 s, MA 1.0 s, HE/EX/CWS 0.6 s each, vessels 0.3 s. Stage III per-stage provider is not printed separately; only the run total says CUDA device 0.
- GPU, 2 s samples: process VRAM 1228 MiB at start, 2346 MiB steady, peak 4540 MiB of 8151 MiB; GPU utilisation spikes 94 % and 74 % at single samples, 0 % between (short kernels, 2 s sampling is coarse).
- Per-process CPU during inference: **not cleanly measured** (only a lifetime average, 35 % to 86 %).

## Finding: idle CPU burn on the chat result page
- Measured as app CPU over 5 s, idle: chat page with the analysis card **205-223 % of a core**; empty Home 19 %; Patients 23 %.
- Cause not located: `perf` and `gdb` are blocked on this machine (`perf_event_paranoid=4`, ptrace). Candidates read in code: nothing obvious in `chat_cards.rs`; the `stick_to_bottom(true)` scroll area at `main.rs:3130` is unverified.
- Caveat: Xvfb has no hardware GL, so rendering ran in Mesa `llvmpip` on the CPU, which inflates the numbers. A repaint loop is still indicated because the other pages stay low. On the real desktop the same loop would cost less, but may still be the "CPU 100 %" symptom.

## Not verified
Ollama actual device (user instance not running; system instance had no model loaded), Grad-CAM and occlusion run inside the GUI, a CPU-fallback banner, Stage II/III per-model provider labels, windowed 1280x820 run, GPU recovery during inference (no such mechanism exists: fallback happens at model load only).

## Remaining CPU paths
GUI rendering/repaint, image decode, preprocessing, database, compilation, and CPU fallback only if CUDA fails to load a model.

## Next steps
1. Replace the installed binary with the new build (not done).
2. Find the repaint loop on the chat result page.
3. Run Grad-CAM, occlusion and Ollama checks with the launcher.
