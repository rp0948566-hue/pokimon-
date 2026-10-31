"""Per-epoch CSV logging and a timestamped run directory."""
from __future__ import annotations

import csv
import time
from pathlib import Path


def make_run_dir(root, name: str) -> Path:
    stamp = time.strftime("%Y%m%d-%H%M%S")
    d = Path(root) / f"{name}-{stamp}"
    d.mkdir(parents=True, exist_ok=False)
    return d


class CsvLogger:
    """Append-only CSV; header fixed by the first row so resumed runs keep columns."""

    def __init__(self, path):
        self.path = Path(path)

    def write(self, row: dict) -> None:
        new = not self.path.exists()
        with open(self.path, "a", newline="") as f:
            w = csv.DictWriter(f, fieldnames=list(row))
            if new:
                w.writeheader()
            w.writerow(row)
