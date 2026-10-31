#!/usr/bin/env python
"""P0: read-only inspection of hardware, production repos and deployed model artifacts.

Writes reports/preflight.json and reports/model_inventory.md. Never modifies either production repo
(git is called with --no-optional-locks; model files are only opened for reading).
"""
from __future__ import annotations

import argparse
import json
import platform
import shutil
import subprocess
import sys
from pathlib import Path

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

from trainlab.models.adapters import BACKEND, UPSTREAM_SRC  # noqa: E402
from trainlab.utils.device import nvidia_smi  # noqa: E402
from trainlab.utils.hashing import sha256_file  # noqa: E402

GUI = BACKEND / "GUI/Hackthon dezine"
GUI_RUST = GUI / "rust"
# Where the user may have placed authorised dataset downloads (inventory only, never copied).
DATASET_HINTS = {"aptos": ("aptos", "train.csv"), "idrid": ("idrid",), "drive": ("drive",), "fgadr": ("fgadr",), "ddr": ("ddr",)}


def sh(cmd: list[str], cwd=None, timeout=30) -> str:
    try:
        r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout)
        return (r.stdout + r.stderr).strip()
    except (OSError, subprocess.SubprocessError) as e:
        return f"ERROR: {e}"


def git_state(repo: Path) -> dict:
    g = ["git", "--no-optional-locks"]
    status = sh(g + ["status", "--porcelain"], cwd=repo).splitlines()
    return {"path": str(repo), "branch": sh(g + ["branch", "--show-current"], cwd=repo),
            "head": sh(g + ["rev-parse", "--short", "HEAD"], cwd=repo), "dirty_entries": len(status),
            "status_sample": status[:15]}


def hardware() -> dict:
    import os
    smi = nvidia_smi()
    procs = sh(["nvidia-smi", "--query-compute-apps=pid,process_name,used_gpu_memory", "--format=csv,noheader"])
    mem = {k: v for k, v in (l.split(":") for l in Path("/proc/meminfo").read_text().splitlines()[:3])}
    du = shutil.disk_usage(LAB)
    out = {"gpu": smi, "gpu_compute_processes": procs or "none", "cpu_count": os.cpu_count(), "platform": platform.platform(),
           "python": sys.version.split()[0], "mem_total_kib": mem.get("MemTotal"), "mem_available_kib": mem.get("MemAvailable"),
           "disk_free_gib_lab": round(du.free / 2**30, 1), "ollama_running": bool(sh(["pgrep", "-x", "ollama"]))}
    try:
        import torch
        out["torch"] = {"version": torch.__version__, "cuda_build": torch.version.cuda, "cuda_available": torch.cuda.is_available()}
        if torch.cuda.is_available():
            out["torch"]["capability"] = list(torch.cuda.get_device_capability(0))
            out["torch"]["arch_list"] = torch.cuda.get_arch_list()
            free, total = torch.cuda.mem_get_info()
            out["torch"]["free_vram_mib"], out["torch"]["total_vram_mib"] = free // 2**20, total // 2**20
    except ImportError:
        out["torch"] = {"error": "torch not installed in this interpreter"}
    return out


def model_inventory(do_hash: bool) -> list[dict]:
    rows = []
    for man_path in sorted((BACKEND / "models").rglob("manifest*.json")):
        if man_path.name == "manifest.example.json":
            continue
        man = json.loads(man_path.read_text())
        d = man_path.parent
        files = sorted(p for p in d.iterdir() if p.suffix in (".onnx", ".pth", ".pt"))
        # Trainable PyTorch checkpoints live beside the ONNX or one level up (stage1 .pth is in models/).
        pth = [p for p in list(d.glob("*.pth")) + list(d.glob("*.pt")) + list((BACKEND / "models").glob(f"{d.name}*.pth"))]
        if man["task"] == "lg_drg_grading":
            pth = [p for p in d.glob(f"stage3_lgdrg_best_fold{man.get('validation', {}).get('fold')}.pt")]
        row = {"manifest": str(man_path.relative_to(BACKEND)), "model_id": man["model_id"], "task": man["task"],
               "architecture": man["architecture"], "license": man.get("license"), "source_url": man.get("source_url"),
               "onnx": man["checkpoint"], "onnx_sha256_manifest": man["sha256"], "input": man["input"],
               "outputs": man["outputs"], "pytorch_checkpoints": [p.name for p in pth],
               "thresholds": {k: v for k, v in man.get("validation", {}).items() if "threshold" in k or k == "corn_thresholds"},
               "files": [f.name for f in files]}
        if do_hash:
            row["onnx_sha256_actual"] = sha256_file(d / man["checkpoint"])
            row["hash_matches_manifest"] = row["onnx_sha256_actual"] == man["sha256"]
            row["pytorch_sha256"] = {p.name: sha256_file(p) for p in pth}
        rows.append(row)
    return rows


def local_datasets() -> dict:
    found = {}
    for name in DATASET_HINTS:
        raw = LAB / "data/raw" / name
        files = [p for p in raw.rglob("*") if p.is_file()] if raw.exists() else []
        found[name] = {"lab_raw_files": len(files)}
    for base in (Path.home() / "Downloads", Path.home() / "Datasets"):
        if base.exists():
            hits = [p.name for p in base.iterdir() if any(k in p.name.lower() for k in ("aptos", "idrid", "drive", "fgadr", "ddr", "messidor"))]
            found.setdefault("external_candidates", {})[str(base)] = hits[:20]
    return found


COMPAT = [
    ("Stage I gate (ConvNeXtV2-L 768)", "binary screening (grade>=1)", "stage1_convnextv2l_768_best_auc_model.pth + stage1.onnx",
     "baseline: APTOS/IDRiD grade>=1 labels. fine-tune: needs upstream arch (hierarchiretina-src, MIT) + VRAM probe; NOT measured on 8 GB"),
    ("Stage II MA/HE/EX/CWS (HSMoE-AUNet)", "lesion segmentation", "*_best_model.pth + stage2_*.onnx",
     "baseline/fine-tune: IDRiD pixel masks (MA,HE,EX,SE~CWS). CWS vs SE semantic mapping unverified"),
    ("Stage II vessel (SwinHRUNet++)", "vessel segmentation", "stage2_swinhrunetpp_best_dice.pth + stage2_vessel.onnx",
     "baseline/fine-tune: DRIVE training references only (test refs withheld)"),
    ("Stage III LG-DRG x5 folds", "5-grade CORN + gradability, input RGB+5 masks", "stage3_lgdrg_best_fold{0-4}.pt + .onnx",
     "needs Stage II masks for every image; cascade baseline NOT implemented in lab (PARTIAL)"),
]


def render_md(pf: dict) -> str:
    L = ["# Model inventory (read-only, P0)", "", f"Backend `{pf['backend_git']['branch']}` dirty={pf['backend_git']['dirty_entries']}; "
         f"GUI `{pf['gui_git']['branch']}` dirty={pf['gui_git']['dirty_entries']}", "",
         "| model_id | task | arch | pytorch ckpt | onnx sha256 (manifest) | thresholds |", "|---|---|---|---|---|---|"]
    for r in pf["models"]:
        L.append(f"| {r['model_id']} | {r['task']} | {r['architecture']} | {', '.join(r['pytorch_checkpoints']) or 'NONE'} | "
                 f"`{r['onnx_sha256_manifest'][:12]}` | {json.dumps(r['thresholds'])} |")
    L += ["", "## Compatibility matrix", "", "| stage | task | artifacts | data / training compatibility |", "|---|---|---|---|"]
    L += [f"| {a} | {b} | {c} | {d} |" for a, b, c, d in COMPAT]
    L += ["", "Licence of deployed weights: CC-BY-NC-4.0 (non-commercial), code MIT. ONNX files are inference-only; "
          "fine-tuning must start from the `.pth/.pt` PyTorch checkpoints with the upstream architecture.", ""]
    return "\n".join(L)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--hash", action="store_true", help="SHA-256 every deployed model file (reads ~8 GB)")
    ap.add_argument("--out", default=str(LAB / "reports"))
    a = ap.parse_args()
    pf = {"backend_git": git_state(BACKEND), "gui_git": git_state(GUI_RUST if (GUI_RUST / ".git").exists() else GUI),
          "hardware": hardware(), "models": model_inventory(a.hash), "datasets_present": local_datasets(),
          "upstream_arch_source": {"path": str(UPSTREAM_SRC), "exists": UPSTREAM_SRC.exists()},
          "kaggle_cli": shutil.which("kaggle") or str(LAB / ".venv/bin/kaggle"),
          "kaggle_credentials_present": bool((Path.home() / ".kaggle/kaggle.json").exists()
                                             or __import__("os").environ.get("KAGGLE_KEY"))}
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "preflight.json").write_text(json.dumps(pf, indent=1))
    (out / "model_inventory.md").write_text(render_md(pf))
    g = pf["hardware"]["gpu"]
    print(f"gpu={g.get('name')} free={g.get('free_mib')}MiB disk_free={pf['hardware']['disk_free_gib_lab']}GiB "
          f"torch={pf['hardware']['torch']} models={len(pf['models'])} -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
