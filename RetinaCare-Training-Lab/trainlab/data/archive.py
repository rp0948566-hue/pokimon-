"""Safe archive extraction (zip-slip / symlink guard) and file inventory."""
from __future__ import annotations

import tarfile
import zipfile
from pathlib import Path

from trainlab.utils.hashing import sha256_file


def _inside(base: Path, target: Path) -> bool:
    try:
        target.resolve().relative_to(base.resolve())
        return True
    except ValueError:
        return False


def safe_extract(archive, dest) -> list[str]:
    """Extract zip/tar into dest; raise ValueError if any member escapes dest. Returns member names."""
    archive, dest = Path(archive), Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    if zipfile.is_zipfile(archive):
        with zipfile.ZipFile(archive) as z:
            names = z.namelist()
            for n in names:
                if not _inside(dest, dest / n):
                    raise ValueError(f"unsafe path in archive {archive.name}: {n}")
            z.extractall(dest)
        return names
    if tarfile.is_tarfile(archive):
        with tarfile.open(archive) as t:
            members = t.getmembers()
            for m in members:
                if not _inside(dest, dest / m.name) or m.issym() or m.islnk():
                    raise ValueError(f"unsafe member in archive {archive.name}: {m.name}")
            t.extractall(dest)
        return [m.name for m in members]
    raise ValueError(f"not a zip/tar archive: {archive}")


def inventory(root, with_hash: bool = False) -> dict:
    root = Path(root)
    files = [p for p in root.rglob("*") if p.is_file()] if root.exists() else []
    inv = {"files": len(files), "bytes": sum(p.stat().st_size for p in files)}
    if with_hash:
        inv["sha256"] = {str(p.relative_to(root)): sha256_file(p) for p in files}
    return inv
