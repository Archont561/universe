"""Validate committed agent skills and their content-hash lock."""

import argparse
import hashlib
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import TypedDict, cast

from scripts.lib.files import list_files, path_exists

SKILL_SOURCE = "Archont561/universe"
SKILL_SOURCE_TYPE = "local"


class SkillLockEntry(TypedDict):
    """One entry in ``skills-lock.json``."""

    computedHash: str
    skillPath: str
    source: str
    sourceType: str


class SkillLock(TypedDict):
    """Top-level shape of ``skills-lock.json``."""

    skills: dict[str, SkillLockEntry]
    version: int


@dataclass(frozen=True)
class SkillFrontmatter:
    """Parsed subset of a ``SKILL.md`` frontmatter block."""

    has_description: bool
    name: str | None


def parse_skill_frontmatter(text: str) -> SkillFrontmatter:
    """Parse the name and description presence from skill frontmatter."""
    if not text.startswith("---\n"):
        return SkillFrontmatter(has_description=False, name=None)
    end = text.find("\n---", 4)
    if end == -1:
        return SkillFrontmatter(has_description=False, name=None)

    frontmatter = text[4:end]
    name_match = re.search(r"^name:\s*['\"]?([A-Za-z0-9_-]+)['\"]?\s*$", frontmatter, re.M)
    has_description = re.search(r"^description:\s*(?:\S.*|)$", frontmatter, re.M) is not None
    return SkillFrontmatter(
        has_description=has_description,
        name=None if name_match is None else name_match.group(1),
    )


def hash_skill_directory(directory: Path) -> str:
    """Hash sorted relative POSIX paths and file bytes with SHA-256."""
    digest = hashlib.sha256()
    for file in list_files(directory):
        relative_path = file.relative_to(directory).as_posix()
        digest.update(relative_path.encode())
        digest.update(file.read_bytes())
    return digest.hexdigest()


def _skill_directories(repo_root: Path) -> list[str]:
    skills_root = repo_root / ".agents" / "skills"
    if not path_exists(skills_root):
        return []
    return sorted(entry.name for entry in skills_root.iterdir() if entry.is_dir())


def _load_lock(lock_path: Path, *, write: bool) -> SkillLock:
    if not path_exists(lock_path):
        if write:
            return {"skills": {}, "version": 1}
        return {"skills": {}, "version": 0}
    return cast(SkillLock, json.loads(lock_path.read_text(encoding="utf-8")))


def validate_skills(repo_root: Path | None = None, *, write: bool = False) -> list[str]:
    """Validate skill frontmatter, forbid symlinks, and verify hashes."""
    root = Path.cwd() if repo_root is None else repo_root
    errors: list[str] = []
    lock_path = root / "skills-lock.json"
    lock = _load_lock(lock_path, write=write)
    directories = _skill_directories(root)
    expected_entries: dict[str, SkillLockEntry] = {}

    if lock.get("version") != 1:
        errors.append(f"skills-lock.json version must be 1, found {lock.get('version')}")
    if not path_exists(root / ".agents" / "skills"):
        errors.append(".agents/skills directory is missing")

    locked_skills = lock.get("skills", {})
    for name in directories:
        directory = root / ".agents" / "skills" / name
        skill_path = f".agents/skills/{name}/SKILL.md"
        skill_file = directory / "SKILL.md"
        if not path_exists(skill_file):
            errors.append(f"{skill_path}: SKILL.md is missing")
            continue

        frontmatter = parse_skill_frontmatter(skill_file.read_text(encoding="utf-8"))
        if frontmatter.name != name:
            errors.append(f"{skill_path}: frontmatter name must be {name}")
        if not frontmatter.has_description:
            errors.append(f"{skill_path}: frontmatter description is missing")

        for file in list_files(directory):
            if file.is_symlink():
                errors.append(
                    f"{file.relative_to(root).as_posix()}: symlinks are not allowed in skills"
                )

        computed_hash = hash_skill_directory(directory)
        expected_entries[name] = {
            "computedHash": computed_hash,
            "skillPath": skill_path,
            "source": SKILL_SOURCE,
            "sourceType": SKILL_SOURCE_TYPE,
        }

        if not write:
            previous = locked_skills.get(name)
            if previous is None:
                errors.append(f"{skill_path}: missing skills-lock.json entry")
                continue
            if previous.get("skillPath") != skill_path:
                errors.append(f"{skill_path}: lock skillPath is {previous.get('skillPath')}")
            if previous.get("computedHash") != computed_hash:
                errors.append(f"{skill_path}: content hash drift; run pixi run skills-check-write")
            if previous.get("source") != SKILL_SOURCE:
                errors.append(f"{skill_path}: lock source must be {SKILL_SOURCE}")
            if previous.get("sourceType") != SKILL_SOURCE_TYPE:
                errors.append(f"{skill_path}: lock sourceType must be {SKILL_SOURCE_TYPE}")

    if not write:
        for locked_name in locked_skills:
            if locked_name not in directories:
                errors.append(f"skills-lock.json has stale entry for missing skill {locked_name}")

    if write and not errors:
        updated: SkillLock = {"skills": expected_entries, "version": 1}
        lock_path.write_text(json.dumps(updated, indent=2) + "\n", encoding="utf-8")

    return errors


def main(argv: list[str] | None = None) -> int:
    """Run the skills checker from the command line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--write", action="store_true", help="refresh skills-lock.json")
    args = parser.parse_args(argv)

    errors = validate_skills(args.root, write=args.write)
    if errors:
        for error in errors:
            print(f"skills-check: {error}", file=sys.stderr)
        return 1
    print("skills-check: lock refreshed" if args.write else "skills-check: skills are valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
