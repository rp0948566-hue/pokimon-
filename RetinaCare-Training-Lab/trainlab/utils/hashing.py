"""SHA-256 helpers."""
import hashlib
from pathlib import Path


def sha256_file(path, chunk: int = 1 << 20) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while block := f.read(chunk):
            h.update(block)
    return h.hexdigest()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_dir_listing(paths) -> str:
    """Stable fingerprint of a list of file names (split checksums)."""
    return sha256_bytes("\n".join(sorted(str(Path(p).name) for p in paths)).encode())
