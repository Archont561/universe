"""Filesystem helpers shared by the repository checks."""

import os
from pathlib import Path


def list_files(root: Path) -> list[Path]:
    """Recursively list files below *root* in stable relative-path order."""
    files: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames.sort()
        for filename in filenames:
            files.append(Path(dirpath) / filename)
    return sorted(files, key=lambda path: path.relative_to(root).as_posix())


def path_exists(path: Path) -> bool:
    """Return ``True`` when *path* exists and can be stat'ed."""
    try:
        path.stat()
    except OSError:
        return False
    return True
