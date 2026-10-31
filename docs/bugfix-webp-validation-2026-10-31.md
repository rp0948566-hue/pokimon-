# Bug report: WEBP upload previews but analysis fails (2026-10-31)

No commits, pushes, merges or resets. Words: PASS / FAIL / BLOCKED / NOT RUN.

## Root cause
- GUI `load_fundus_from_path` decodes with the GUI `image` crate (default features, WebP included), so the preview worked. It sends the original path to the backend.
- Backend `preprocess_stage1_bytes` and `decode_stage2_source` accepted only `ImageFormat::Jpeg | Png` (guessed from bytes) and the bridge `image` crate had only `jpeg`/`png` features. WebP returned `UnsupportedFormat`.
- `cascade.rs` mapped every preprocessing error to the fixed text "the source image could not be validated", hiding the reason.
- Eye laterality ("eye not set") is not involved. The file is a real WebP (RIFF/WEBP, 600x393).

## Change
- `native/Cargo.toml`: `image` features add `webp`, `bmp`.
- `preprocessing.rs`: `decodable_bytes()` detects format from content; JPEG/PNG pass through; WebP/BMP are decoded once, dimension limits checked first, converted to 8-bit RGB (alpha dropped) as in-memory PNG. Original file untouched, not renamed. SHA-256 still over the original bytes (stage identity unchanged).
- `stage2_preprocessing.rs`: same helper (also feeds Stage III input).
- `cascade.rs`: new kinds with path-free text: unsupported format, corrupt/undecodable, size outside limits. Classifiers for stages 1-3 use them.
- Quality gates, thresholds, models, CUDA settings unchanged.

## Validation
| Check | Result |
|---|---|
| bridge lib tests (111) | PASS |
| app `cascade` tests (20) | PASS |
| new tests: WebP/BMP decode, original hash kept, GIF/garbage = unsupported, truncated WebP != unsupported | PASS |
| old test "BMP rejected" | updated, BMP now supported |
| valid JPEG / PNG | PASS (existing tests) |
| missing file, inadequate/tiny image, missing eye | NOT RUN (no new code touched; existing tests only) |
| `nice -n 19 ionice -c3 cargo build --release -j2` | PASS (exit 0, 5 min 18 s) |

## Real GUI run (Xvfb 1600x900, new release binary, the supplied WEBP, temp data dir)
- Attachment preview: PASS. Backend ready, GPU (CUDA 0).
- Validation: PASS. Stage I score 0.9976 and Stage II (MA, HE, EX, CWS, vessels) ran. The "could not be validated" failure is gone.
- First attempt (your app still open): **FAIL, GPU memory contention.** The user's installed RetinaCare (pid 65468, ~2.3 GiB VRAM) was running. Stage III `cublasCreate` failed (resource allocation), the app fell back to CPU per policy and said so in the sidebar. Folds then ran on mixed providers and `combine_five_folds` rejected them ("the analysis request is invalid"). Not caused by WebP.
- Rerun after closing the other app: **PASS.** All stages ran, five-fold technical grade 4, "Total run time 24.5 s on CUDA device 0", saved as EXAM-021, no CPU fallback. Same WEBP, same binary.

## Install
PASS. Backup `retinacare.bak-20261031-webp` (old sha256 5ef3fb43...). Installed sha256 96bb2e1f64093467... equals the release build. Installed app launched via `retinacare-launch` (pid running, CUDA process visible). The old app was stopped with SIGTERM at your request.

## Limits
- Menu text still says "Add image (PNG, JPG)" (ui2/composer.rs:473, main.rs:839); file filters already include webp/bmp.
- Mixed-provider ensemble failure shows a vague message; worth a clear text.
- Animated WebP: first frame only. 16-bit/gray/alpha inputs flattened to RGB.
