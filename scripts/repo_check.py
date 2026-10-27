"""Validate standalone-repository invariants."""

import argparse
import os
import re
import sys
from pathlib import Path

from scripts.lib.files import list_files, path_exists

REQUIRED_FILES = (
    ".github/workflows/ci.yml",
    "AGENTS.md",
    "Cargo.toml",
    "CHANGELOG.md",
    "CONTRIBUTING.md",
    "README.md",
    "backlog/config.yml",
    "pixi.toml",
    "pyproject.toml",
    "skills-lock.json",
)

FORBIDDEN_PATHS = (
    "LICENSE",
    "NOTICE.md",
    ".agents/NOTICE.md",
    ".agents/licenses",
    "backlog/MERGE-MAP.md",
    "biome.json",
    "bun.lock",
    "package.json",
    "tsconfig.json",
    "website",
)

FORBIDDEN_BASENAMES = {
    ".gitkeep",
    "copying",
    "license",
    "license.md",
    "license.txt",
    "notice",
    "notice.md",
    "notice.txt",
}

SKIP_DIRS = {
    ".git",
    ".mypy_cache",
    ".pixi",
    ".pytest_cache",
    ".ruff_cache",
    "__pycache__",
    "node_modules",
    "target",
}

# Constructed from fragments so the checker does not itself contain the
# forbidden standalone-repository references it scans for.
LEGACY_TOKENS = (
    "conda" + "-universe",
    "conda" + "_universe",
    "air" + "pack",
)


def extract_workflow_refs(text: str) -> list[str]:
    """Extract the ref after every GitHub Actions ``uses:`` value."""
    pattern = re.compile(r"^\s*(?:-\s*)?uses:\s*[^@\s]+@([^\s#]+)", re.M)
    return pattern.findall(text)


def is_immutable_sha(ref: str) -> bool:
    """Return ``True`` only for a full lowercase 40-character commit SHA."""
    return re.fullmatch(r"[0-9a-f]{40}", ref) is not None


def extract_toml_version(text: str) -> str | None:
    """Extract the first top-level TOML version assignment."""
    match = re.search(r'^version\s*=\s*"([^"]+)"', text, re.M)
    return None if match is None else match.group(1)


def _iter_repo_files(repo_root: Path) -> list[Path]:
    files: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(repo_root):
        dirnames[:] = sorted(name for name in dirnames if name not in SKIP_DIRS)
        for filename in filenames:
            files.append(Path(dirpath) / filename)
    return sorted(files, key=lambda path: path.relative_to(repo_root).as_posix())


def _contains_legacy_token(data: bytes) -> bool:
    lowered = data.lower()
    return any(token.encode() in lowered for token in LEGACY_TOKENS)


def validate_repo(repo_root: Path | None = None) -> list[str]:
    """Validate required files, versions, workflow refs, and clean layout."""
    root = Path.cwd() if repo_root is None else repo_root
    errors: list[str] = []

    for required in REQUIRED_FILES:
        if not path_exists(root / required):
            errors.append(f"required file is missing: {required}")

    for forbidden in FORBIDDEN_PATHS:
        if path_exists(root / forbidden):
            errors.append(f"forbidden standalone-cleanup path exists: {forbidden}")

    for file in _iter_repo_files(root):
        relative_path = file.relative_to(root)
        relative_posix = relative_path.as_posix()
        basename = file.name.lower()

        if basename in FORBIDDEN_BASENAMES or "licenses" in relative_path.parts:
            errors.append(f"forbidden license/notice/placeholder file: {relative_posix}")
        if file.suffix == ".ts" and (
            relative_posix.startswith("scripts/") or relative_posix.startswith("tests/scripts/")
        ):
            errors.append(f"TypeScript repository script is forbidden: {relative_posix}")
        if _contains_legacy_token(relative_posix.encode()):
            errors.append(f"legacy repository reference in path: {relative_posix}")
            continue
        try:
            data = file.read_bytes()
        except OSError as exc:
            errors.append(f"cannot read {relative_posix}: {exc}")
            continue
        if _contains_legacy_token(data):
            errors.append(f"legacy repository reference in content: {relative_posix}")

    cargo_path = root / "Cargo.toml"
    pixi_path = root / "pixi.toml"
    if path_exists(cargo_path) and path_exists(pixi_path):
        cargo_version = extract_toml_version(cargo_path.read_text(encoding="utf-8"))
        pixi_version = extract_toml_version(pixi_path.read_text(encoding="utf-8"))
        if cargo_version is None or pixi_version is None:
            errors.append("Cargo and Pixi must both declare a version")
        elif cargo_version != pixi_version:
            errors.append(f"version drift: Cargo={cargo_version}, Pixi={pixi_version}")

    workflow_root = root / ".github" / "workflows"
    if path_exists(workflow_root):
        workflows = [path for path in list_files(workflow_root) if path.suffix in {".yml", ".yaml"}]
        for workflow in workflows:
            text = workflow.read_text(encoding="utf-8")
            for ref in extract_workflow_refs(text):
                if not is_immutable_sha(ref):
                    errors.append(f"{workflow}: action ref is not a full SHA: {ref}")

    return errors


def main(argv: list[str] | None = None) -> int:
    """Run the repository checker from the command line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)

    errors = validate_repo(args.root)
    if errors:
        for error in errors:
            print(f"repo-check: {error}", file=sys.stderr)
        return 1
    print("repo-check: repository invariants are valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
