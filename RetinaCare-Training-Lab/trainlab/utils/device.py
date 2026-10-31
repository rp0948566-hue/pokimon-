"""Device selection and measured VRAM, never assumed."""
from __future__ import annotations

import subprocess


def pick_device(prefer: str = "cuda"):
    import torch

    if prefer == "cuda" and torch.cuda.is_available():
        return torch.device("cuda:0")
    return torch.device("cpu")


def nvidia_smi() -> dict:
    """Free/total VRAM in MiB from the driver (includes other processes such as Ollama)."""
    try:
        out = subprocess.run(
            ["nvidia-smi", "--query-gpu=name,memory.total,memory.used,memory.free,driver_version",
             "--format=csv,noheader,nounits"], capture_output=True, text=True, timeout=15, check=True,
        ).stdout.strip().splitlines()[0]
    except (OSError, subprocess.SubprocessError, IndexError):
        return {"available": False}
    name, total, used, free, driver = [s.strip() for s in out.split(",")]
    return {"available": True, "name": name, "total_mib": int(total), "used_mib": int(used),
            "free_mib": int(free), "driver": driver}


def gpu_snapshot() -> dict:
    import torch

    if not torch.cuda.is_available():
        return {"cuda": False}
    return {"cuda": True, "device": torch.cuda.get_device_name(0),
            "capability": list(torch.cuda.get_device_capability(0)),
            "peak_allocated_mib": round(torch.cuda.max_memory_allocated() / 2**20, 1),
            "peak_reserved_mib": round(torch.cuda.max_memory_reserved() / 2**20, 1)}


def probe_batch_size(make_model, make_batch, candidates=(8, 4, 2, 1), headroom_mib: int = 600,
                     device=None, amp: bool = True) -> dict:
    """Largest candidate whose measured fwd+bwd peak fits in free VRAM minus headroom.

    make_model() -> nn.Module; make_batch(bs) -> (x, y) on CPU. Returns {"batch_size", "peak_mib", "tried"}.
    """
    import torch

    device = device or pick_device()
    if device.type != "cuda":
        return {"batch_size": 2, "peak_mib": None, "tried": [], "note": "cpu: fixed small batch"}
    tried = []
    for bs in candidates:
        model = make_model().to(device).train()
        try:
            torch.cuda.empty_cache()
            torch.cuda.reset_peak_memory_stats()
            x, y = make_batch(bs)
            with torch.autocast("cuda", dtype=torch.float16, enabled=amp):
                loss = model(x.to(device)).float().mean()
            loss.backward()
            peak = torch.cuda.max_memory_allocated() / 2**20
            free = nvidia_smi().get("free_mib", 0)
            tried.append({"batch_size": bs, "peak_mib": round(peak, 1), "ok": True})
            if free >= headroom_mib:  # driver free VRAM measured while this batch's memory is still resident
                return {"batch_size": bs, "peak_mib": round(peak, 1), "tried": tried}
        except torch.cuda.OutOfMemoryError:
            tried.append({"batch_size": bs, "ok": False, "error": "OOM"})
        finally:
            del model
            torch.cuda.empty_cache()
    return {"batch_size": 1, "peak_mib": None, "tried": tried, "note": "all candidates failed or tight; using 1"}
