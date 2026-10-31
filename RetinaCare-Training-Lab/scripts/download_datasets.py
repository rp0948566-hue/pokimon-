#!/usr/bin/env python
"""P2: dataset access check / authorised download / safe extraction.

  --check-only   report disk, credentials (never echoed), rules acceptance, existing files
  --download     APTOS (Kaggle CLI) or DDR (author's public Drive folder via gdown), only with --i-accepted-terms
  --extract      safely unpack archives that YOU placed in data/downloads/<dataset>/

No access control is bypassed: sources that need a login or a licence are reported BLOCKED with the
exact step for the user. FGADR is never downloaded automatically.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import yaml

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

from trainlab.data.archive import inventory, safe_extract  # noqa: E402

STATUS = LAB / "reports/data_audit/download_status.json"


def kaggle_cli() -> str | None:
    return shutil.which("kaggle") or (str(LAB / ".venv/bin/kaggle") if (LAB / ".venv/bin/kaggle").exists() else None)


def kaggle_credentials() -> dict:
    """Presence only. Values are never read into the report."""
    f = Path.home() / ".kaggle/kaggle.json"
    env = bool(os.environ.get("KAGGLE_USERNAME") and os.environ.get("KAGGLE_KEY")) or bool(os.environ.get("KAGGLE_API_TOKEN"))
    return {"file": f.exists(), "file_mode_ok": (f.stat().st_mode & 0o077 == 0) if f.exists() else None, "env": env,
            "present": f.exists() or env}


def check(name: str, src: dict) -> dict:
    raw, dl = LAB / "data/raw" / name, LAB / "data/downloads" / name
    free_gib = shutil.disk_usage(LAB).free / 2**30
    res = {"dataset": name, "url": src["url"], "access": src["access"], "est_gib": src["est_gib"],
           "disk_free_gib": round(free_gib, 1), "disk_ok": free_gib > src["est_gib"] * 2.5,
           "raw": inventory(raw), "downloads": inventory(dl), "status": None, "blocker": None}
    if res["raw"]["files"]:
        res["status"] = "PRESENT"
    elif src["method"] == "auto_kaggle":
        cred = kaggle_credentials()
        res["kaggle_cli"] = bool(kaggle_cli()); res["kaggle_credentials"] = cred
        if not (res["kaggle_cli"] and cred["present"]):
            res["status"] = "BLOCKED"
            res["blocker"] = ("No Kaggle credentials. User step: sign in at kaggle.com, open "
                              f"{src['url'].replace('/data', '')} and click 'Join Competition' / accept the rules, then create an API token "
                              "(Settings > API) and save it to ~/.kaggle/kaggle.json (chmod 600) or export KAGGLE_USERNAME/KAGGLE_KEY.")
        else:
            r = subprocess.run([kaggle_cli(), "competitions", "files", "-c", src["kaggle_competition"]], capture_output=True, text=True, timeout=60)
            if r.returncode == 0:
                res["status"] = "READY_TO_DOWNLOAD"
            else:
                res["status"] = "BLOCKED"
                res["blocker"] = f"Kaggle refused the listing (rules probably not accepted): {r.stderr.strip().splitlines()[-1][:200] if r.stderr.strip() else r.returncode}"
    elif src["method"] == "auto_gdrive":
        try:
            import gdown  # noqa: F401
            res["status"] = "READY_TO_DOWNLOAD"
        except ImportError:
            res["status"] = "BLOCKED"
            res["blocker"] = "gdown not installed in the lab venv (uv pip install gdown)"
    elif src["method"] == "manual_restricted":
        res["status"] = "BLOCKED"; res["blocker"] = f"Restricted access: {src['access']} Not auto-downloaded."
    else:
        archives = [p for p in dl.glob("*") if p.is_file()] if dl.exists() else []
        if archives:
            res["status"] = "ARCHIVE_PLACED"
        else:
            res["status"] = "BLOCKED"
            res["blocker"] = f"{src['access']} Download from {src['url']} (or {src.get('mirror', 'n/a')}) into {dl}, then run --extract."
    return res


def download_aptos(src: dict, accepted: bool) -> dict:
    if not accepted:
        return {"status": "BLOCKED", "blocker": "Refusing without --i-accepted-terms (Kaggle competition rules must be accepted by you first)."}
    res = check("aptos", src)
    if res["status"] != "READY_TO_DOWNLOAD":
        return res
    dest = LAB / "data/downloads/aptos"
    dest.mkdir(parents=True, exist_ok=True)
    r = subprocess.run([kaggle_cli(), "competitions", "download", "-c", src["kaggle_competition"], "-p", str(dest)], text=True)
    res["status"] = "DOWNLOADED" if r.returncode == 0 else "FAILED"
    res["downloads"] = inventory(dest)
    return res


def download_gdrive(name: str, src: dict, accepted: bool) -> dict:
    """Author-hosted public Drive folder (DDR). Requires --i-accepted-terms: licence is CC BY-NC-SA 4.0."""
    if not accepted:
        return {"status": "BLOCKED", "blocker": f"Refusing without --i-accepted-terms (licence: {src['access']})"}
    res = check(name, src)
    if res["status"] != "READY_TO_DOWNLOAD":
        return res
    import gdown
    dest = LAB / "data/downloads" / name
    dest.mkdir(parents=True, exist_ok=True)
    try:
        files = gdown.download_folder(id=src["gdrive_folder_id"], output=str(dest), quiet=False, resume=True, retries=3, timeout=60)
        res["status"] = "DOWNLOADED" if files else "FAILED"
    except Exception as e:  # Drive quota / listing errors come back as many exception types
        res["status"], res["blocker"] = "FAILED", f"gdown error: {str(e)[:300]}"
    res["downloads"] = inventory(dest)
    return res


def extract(name: str) -> dict:
    dl, raw = LAB / "data/downloads" / name, LAB / "data/raw" / name
    arch = sorted(p for p in dl.glob("*") if p.is_file() and p.suffix in (".zip", ".tar", ".gz", ".tgz")) if dl.exists() else []
    if not arch and not (dl.exists() and list(dl.glob("*.zip.0*"))):
        return {"dataset": name, "status": "BLOCKED", "blocker": f"no archive in {dl}"}
    parts = sorted(p for p in dl.glob("*.zip.0*") if p.is_file())
    if parts:  # multipart zip (DDR): concatenate in order into one archive, then extract safely
        whole = dl / parts[0].name.split(".zip.")[0]
        whole = whole.with_suffix(".zip")
        if not whole.exists():
            with open(whole, "wb") as out:
                for part in parts:
                    with open(part, "rb") as f:
                        shutil.copyfileobj(f, out, 1 << 22)
        arch = [whole]
    n = 0
    for a in arch:
        n += len(safe_extract(a, raw))
        # Kaggle zips can nest image zips; one extra level only, same safety checks.
        for inner in raw.glob("*.zip"):
            n += len(safe_extract(inner, raw / inner.stem))
    return {"dataset": name, "status": "EXTRACTED", "members": n, "raw": inventory(raw)}


def save_status(res: dict) -> None:
    STATUS.parent.mkdir(parents=True, exist_ok=True)
    cur = json.loads(STATUS.read_text()) if STATUS.exists() else {}
    cur[res["dataset"]] = res
    STATUS.write_text(json.dumps(cur, indent=1))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dataset", required=True, choices=["aptos", "idrid", "drive", "fgadr", "ddr", "all"])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--check-only", action="store_true")
    g.add_argument("--download", action="store_true")
    g.add_argument("--extract", action="store_true")
    ap.add_argument("--i-accepted-terms", action="store_true", help="confirm YOU accepted the dataset terms (needed for --download)")
    a = ap.parse_args()
    srcs = yaml.safe_load((LAB / "configs/data_sources.yaml").read_text())
    names = list(srcs) if a.dataset == "all" else [a.dataset]
    if a.download and len(names) > 1:
        ap.error("download one dataset at a time")
    rc = 0
    for n in names:
        if a.check_only:
            res = check(n, srcs[n])
        elif a.extract:
            res = extract(n)
        else:
            m = srcs[n]["method"]
            if m == "auto_kaggle":
                res = download_aptos(srcs[n], a.i_accepted_terms)
            elif m == "auto_gdrive":
                res = download_gdrive(n, srcs[n], a.i_accepted_terms)
            else:
                res = {"status": "BLOCKED", "blocker": f"{n} has no scripted download: {srcs[n]['access']}"}
        res["dataset"] = n
        save_status(res)
        print(f"[{n}] {res['status']}" + (f" -- {res['blocker']}" if res.get("blocker") else ""))
        rc |= res["status"] in ("BLOCKED", "FAILED")
    return int(rc)


if __name__ == "__main__":
    sys.exit(main())
