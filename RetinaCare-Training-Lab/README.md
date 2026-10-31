# RetinaCare GPU Training Lab

Isolated research lab. Lives **outside** both production repos. Datasets, checkpoints, exports and
credentials stay here and are git-ignored (`.gitignore`). Nothing here replaces deployed models.
Research use only; no clinical claims.

```bash
cd /home/claude/Desktop/Dird/RetinaCare-Training-Lab && source .venv/bin/activate
```

## Reproduce (in order)

| step | command | needs |
|---|---|---|
| P0 inspect (read-only) | `python scripts/preflight.py [--hash]` | nothing |
| self-check | `python scripts/verify_lab.py` | venv |
| P2 access check | `python scripts/download_datasets.py --dataset all --check-only` | nothing |
| P2 APTOS | `python scripts/download_datasets.py --dataset aptos --download --i-accepted-terms` | Kaggle account, rules accepted, `~/.kaggle/kaggle.json` |
| P2 DDR | `python scripts/download_datasets.py --dataset ddr --download --i-accepted-terms` | CC BY-NC-SA 4.0 non-commercial research use; Drive quota |
| P2 manual sets | place archives in `data/downloads/<idrid\|drive\|fgadr>/`, then `--extract` | login / licence (see `configs/data_sources.yaml`) |
| P3 prepare | `python scripts/prepare_aptos.py` / `prepare_idrid.py` / `prepare_drive.py` | raw data |
| P4 baseline | `python scripts/evaluate_baselines.py` | prepared val manifest |
| P5 smoke (plumbing only) | `python scripts/train_grading.py --smoke` | prepared train/val |
| P5 real run | `python scripts/train_grading.py` / `train_lesions.py --lesion MA` / `train_vessels.py` | prepared data, GPU |
| P6 compare | `python scripts/evaluate_candidate.py --run <run>` then once `--final` | finished run |
| P7 export | `python scripts/export_onnx.py --run <run>` | finished run |

Every script supports `--help`. Training loop: `trainlab/train/engine.py` (`fit`). Task wrappers:
`trainlab/train/grading.py`, `trainlab/train/segmentation.py`, `trainlab/train/segmentation_run.py`.

## Rules baked into the code
- `--smoke` runs are tagged `kind=smoke` in `run_info.json`; `evaluate_candidate.py` and `export_onnx.py` refuse them.
- Locked test (`data/manifests/test/aptos_locked_test.csv`, read-only) is evaluated once (`--final`, lock file `experiments/locked_test_used.json`).
- A missing lesion mask is `mask_status=missing`, never an empty mask. DRIVE test references are never fabricated.
- Baseline outputs are chmod read-only. Export refuses any path inside `/home/claude/Desktop/Dird`.
- Pretrained weights download into `models/pretrained/hf` (HF_HOME is set by `trainlab/models/registry.py`).

## Failure modes
- `BLOCKED` from `download_datasets.py`: account / rules / licence step required; the message gives the exact step.
- CUDA OOM: batch auto-probe picks a size that fits measured free VRAM; repeated OOM aborts with advice. Other GPU processes (Ollama) are never killed.
- RTX 50-series (sm_120) needs a CUDA 12.8+/13 PyTorch build; `scripts/preflight.py` prints the torch arch list.
