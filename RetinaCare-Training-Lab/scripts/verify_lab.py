#!/usr/bin/env python
"""Lab self-check: every CLI answers --help, unit tests pass, .gitignore hides private data, production repos untouched.

Writes reports/verify_lab.json. Exit code 0 only if every check passes.
"""
import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

LAB = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LAB))

PROD = [Path("/home/claude/Desktop/Dird"), Path("/home/claude/Desktop/Dird/GUI/Hackthon dezine")]
MUST_IGNORE = ["data/raw/aptos/a.png", "data/downloads/x.zip", "checkpoints/grading/r/best.ckpt", "models/pretrained/w.safetensors",
               "exports/r/candidate.onnx", ".env", "kaggle.json", ".venv/bin/python", "experiments/baseline/x/predictions.csv"]
MUST_TRACK = ["trainlab/data/datasets.py", "scripts/preflight.py", "configs/grading_aptos.yaml", "README.md"]


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def git_snapshot(repo: Path) -> str:
    return run(["git", "--no-optional-locks", "status", "--porcelain"], cwd=repo).stdout + run(["git", "rev-parse", "HEAD"], cwd=repo).stdout


def check_gitignore() -> dict:
    """Copy .gitignore into a throw-away repo (outside the lab) and ask git what it ignores."""
    with tempfile.TemporaryDirectory() as t:
        subprocess.run(["git", "init", "-q", t], check=True)
        (Path(t) / ".gitignore").write_text((LAB / ".gitignore").read_text())
        ign = {p: run(["git", "check-ignore", "-q", p], cwd=t).returncode == 0 for p in MUST_IGNORE}
        trk = {p: run(["git", "check-ignore", "-q", p], cwd=t).returncode != 0 for p in MUST_TRACK}
    return {"ignored_ok": ign, "tracked_ok": trk, "passed": all(ign.values()) and all(trk.values())}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--skip-tests", action="store_true")
    a = ap.parse_args()
    before = {str(p): git_snapshot(p) for p in PROD}
    res = {"help": {}}
    for s in sorted((LAB / "scripts").glob("*.py")):
        if s.name == "verify_lab.py":
            continue
        r = run([sys.executable, str(s), "--help"])
        res["help"][s.name] = r.returncode == 0
    if not a.skip_tests:
        r = run([sys.executable, "-m", "pytest", "-q", "-x", str(LAB / "tests")], cwd=LAB)
        res["pytest"] = {"passed": r.returncode == 0, "tail": r.stdout.strip().splitlines()[-3:]}
    res["gitignore"] = check_gitignore()
    res["production_repos_unchanged"] = before == {str(p): git_snapshot(p) for p in PROD}
    res["passed"] = all(res["help"].values()) and res.get("pytest", {"passed": True})["passed"] and res["gitignore"]["passed"] and res["production_repos_unchanged"]
    (LAB / "reports").mkdir(exist_ok=True)
    (LAB / "reports/verify_lab.json").write_text(json.dumps(res, indent=1))
    print(json.dumps({k: v for k, v in res.items() if k != "help"}, indent=1), "\nhelp:", res["help"])
    return 0 if res["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
