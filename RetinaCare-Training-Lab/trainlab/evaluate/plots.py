"""Figures drawn only from logged / computed numbers."""
from __future__ import annotations

from pathlib import Path

import numpy as np
import pandas as pd


def _plt():
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    return plt


def training_curves(run_dir) -> Path | None:
    """training_curves.png from run_dir/train.csv + val.csv; None if no epoch was logged."""
    run_dir = Path(run_dir)
    tr, va = run_dir / "train.csv", run_dir / "val.csv"
    if not (tr.exists() and va.exists()):
        return None
    t, v = pd.read_csv(tr), pd.read_csv(va)
    plt = _plt()
    metrics = [c for c in v.columns if c not in ("epoch", "val_loss", "val_n")]
    fig, ax = plt.subplots(1, 2, figsize=(11, 4))
    ax[0].plot(t["epoch"], t["train_loss"], marker="o", label="train loss")
    if "val_loss" in v:
        ax[0].plot(v["epoch"], v["val_loss"], marker="o", label="val loss")
    ax[0].set_xlabel("epoch"); ax[0].set_ylabel("loss"); ax[0].legend(); ax[0].grid(alpha=.3)
    for m in metrics:
        ax[1].plot(v["epoch"], v[m], marker="o", label=m)
    ax[1].set_xlabel("epoch"); ax[1].legend(); ax[1].grid(alpha=.3)
    fig.tight_layout()
    out = run_dir / "training_curves.png"
    fig.savefig(out, dpi=110); plt.close(fig)
    return out


def confusion_matrix_png(cm: np.ndarray, labels, out, title: str = "") -> Path:
    plt = _plt()
    fig, ax = plt.subplots(figsize=(5, 4.5))
    ax.imshow(cm, cmap="Blues")
    ax.set_xticks(range(len(labels)), labels, rotation=45, ha="right"); ax.set_yticks(range(len(labels)), labels)
    for i in range(cm.shape[0]):
        for j in range(cm.shape[1]):
            ax.text(j, i, int(cm[i, j]), ha="center", va="center", color="black" if cm[i, j] < cm.max() / 2 else "white")
    ax.set_xlabel("predicted"); ax.set_ylabel("reference"); ax.set_title(title)
    fig.tight_layout(); fig.savefig(out, dpi=110); plt.close(fig)
    return Path(out)
