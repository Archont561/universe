"""Tests for the Python skills checker."""

import hashlib
import json
from pathlib import Path
from typing import cast

import pytest

from scripts.skills_check import (
    SkillLock,
    SkillLockEntry,
    hash_skill_directory,
    main,
    parse_skill_frontmatter,
    validate_skills,
)


def test_parses_skill_frontmatter() -> None:
    parsed = parse_skill_frontmatter("---\nname: tdd\ndescription: Test first\n---\nBody")
    assert parsed.name == "tdd"
    assert parsed.has_description

    missing = parse_skill_frontmatter("no frontmatter")
    assert missing.name is None
    assert not missing.has_description

    unclosed = parse_skill_frontmatter("---\nname: tdd\ndescription: Test first\n")
    assert unclosed.name is None
    assert not unclosed.has_description


def test_hashes_sorted_relative_paths_and_file_bytes(tmp_path: Path) -> None:
    references = tmp_path / "references"
    references.mkdir()
    (tmp_path / "SKILL.md").write_text("skill", encoding="utf-8")
    (references / "guide.md").write_text("guide", encoding="utf-8")

    expected = hashlib.sha256()
    expected.update(b"SKILL.md")
    expected.update(b"skill")
    expected.update(b"references/guide.md")
    expected.update(b"guide")

    assert hash_skill_directory(tmp_path) == expected.hexdigest()


def test_write_mode_creates_a_valid_local_lock(tmp_path: Path) -> None:
    skill_root = tmp_path / ".agents" / "skills" / "demo"
    skill_root.mkdir(parents=True)
    (skill_root / "SKILL.md").write_text(
        "---\nname: demo\ndescription: Demo skill\n---\nUse it well.\n",
        encoding="utf-8",
    )
    (tmp_path / "skills-lock.json").write_text('{"version":1,"skills":{}}\n', encoding="utf-8")

    assert validate_skills(tmp_path, write=True) == []
    assert validate_skills(tmp_path) == []

    lock = cast(SkillLock, json.loads((tmp_path / "skills-lock.json").read_text(encoding="utf-8")))
    entry = lock["skills"]["demo"]
    assert len(entry["computedHash"]) == 64
    assert entry["source"] == "Archont561/universe"
    assert entry["sourceType"] == "local"


def test_reports_missing_skills_and_lock_problems(tmp_path: Path) -> None:
    errors = "\n".join(validate_skills(tmp_path))
    assert "skills-lock.json version must be 1, found 0" in errors
    assert ".agents/skills directory is missing" in errors

    errors = "\n".join(validate_skills(tmp_path, write=True))
    assert ".agents/skills directory is missing" in errors
    assert not (tmp_path / "skills-lock.json").exists()

    skills_root = tmp_path / ".agents" / "skills"
    missing = skills_root / "missing"
    bad = skills_root / "bad"
    missing.mkdir(parents=True)
    bad.mkdir()
    (bad / "SKILL.md").write_text("---\nname: other\n---\nBody\n", encoding="utf-8")
    target = tmp_path / "target.txt"
    target.write_text("target\n", encoding="utf-8")
    (bad / "linked.txt").symlink_to(target)
    (tmp_path / "skills-lock.json").write_text(
        json.dumps(
            {
                "version": 1,
                "skills": {
                    "bad": {
                        "computedHash": "0" * 64,
                        "skillPath": ".agents/skills/bad/WRONG.md",
                        "source": "elsewhere/example",
                        "sourceType": "remote",
                    },
                    "stale": {
                        "computedHash": "1" * 64,
                        "skillPath": ".agents/skills/stale/SKILL.md",
                        "source": "Archont561/universe",
                        "sourceType": "local",
                    },
                },
            }
        ),
        encoding="utf-8",
    )

    errors = "\n".join(validate_skills(tmp_path))

    assert "SKILL.md is missing" in errors
    assert "frontmatter name must be bad" in errors
    assert "frontmatter description is missing" in errors
    assert "symlinks are not allowed in skills" in errors
    assert "lock skillPath is .agents/skills/bad/WRONG.md" in errors
    assert "content hash drift" in errors
    assert "lock source must be Archont561/universe" in errors
    assert "lock sourceType must be local" in errors
    assert "stale entry for missing skill stale" in errors


def test_reports_a_missing_lock_entry_and_main_modes(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    skill_root = tmp_path / ".agents" / "skills" / "demo"
    skill_root.mkdir(parents=True)
    (skill_root / "SKILL.md").write_text(
        "---\nname: demo\ndescription: Demo skill\n---\nUse it well.\n",
        encoding="utf-8",
    )
    (tmp_path / "skills-lock.json").write_text('{"version":1,"skills":{}}\n', encoding="utf-8")

    errors = "\n".join(validate_skills(tmp_path))
    assert "missing skills-lock.json entry" in errors

    assert main(["--root", str(tmp_path)]) == 1
    captured = capsys.readouterr()
    assert "missing skills-lock.json entry" in captured.err

    assert main(["--root", str(tmp_path), "--write"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "skills-check: lock refreshed\n"

    assert main(["--root", str(tmp_path)]) == 0
    captured = capsys.readouterr()
    assert captured.out == "skills-check: skills are valid\n"


def _write_skill(root: Path, name: str) -> None:
    """Create one skill directory with valid frontmatter."""
    directory = root / ".agents" / "skills" / name
    directory.mkdir(parents=True)
    (directory / "SKILL.md").write_text(
        f"---\nname: {name}\ndescription: {name} skill\n---\nUse it well.\n",
        encoding="utf-8",
    )


def _write_lock(root: Path, entries: dict[str, SkillLockEntry]) -> None:
    """Write a lock file holding exactly `entries`."""
    (root / "skills-lock.json").write_text(
        json.dumps({"version": 1, "skills": entries}), encoding="utf-8"
    )


def test_accepts_a_vendored_skill_locked_by_its_upstream_path(tmp_path: Path) -> None:
    # A skill installed with `npx skills add <repo> --skill <name>` is locked by where it
    # comes from, not by where it now lives: a refresh has to be able to find it upstream.
    _write_skill(tmp_path, "vendored")
    _write_lock(
        tmp_path,
        {
            "vendored": {
                "computedHash": hash_skill_directory(tmp_path / ".agents" / "skills" / "vendored"),
                "skillPath": "plugins/patterns/bdd/skills/vendored/SKILL.md",
                "source": "thebushidocollective/han",
                "sourceType": "github",
            }
        },
    )

    assert validate_skills(tmp_path) == []


def test_a_vendored_skill_must_name_an_owner_and_repository(tmp_path: Path) -> None:
    _write_skill(tmp_path, "vendored")
    _write_lock(
        tmp_path,
        {
            "vendored": {
                "computedHash": hash_skill_directory(tmp_path / ".agents" / "skills" / "vendored"),
                "skillPath": "SKILL.md",
                "source": "han",
                "sourceType": "github",
            }
        },
    )

    errors = "\n".join(validate_skills(tmp_path))
    assert "lock source must be an owner/repository slug" in errors


def test_write_mode_keeps_the_provenance_of_a_vendored_skill(tmp_path: Path) -> None:
    # Refreshing the hashes must not silently re-author someone else's skill.
    _write_skill(tmp_path, "vendored")
    _write_skill(tmp_path, "local")
    _write_lock(
        tmp_path,
        {
            "vendored": {
                "computedHash": "0" * 64,
                "skillPath": "plugins/patterns/bdd/skills/vendored/SKILL.md",
                "source": "thebushidocollective/han",
                "sourceType": "github",
            },
            "local": {
                "computedHash": "0" * 64,
                "skillPath": ".agents/skills/local/SKILL.md",
                "source": "Archont561/universe",
                "sourceType": "local",
            },
        },
    )

    assert validate_skills(tmp_path, write=True) == []
    assert validate_skills(tmp_path) == []

    lock = cast(SkillLock, json.loads((tmp_path / "skills-lock.json").read_text(encoding="utf-8")))
    vendored = lock["skills"]["vendored"]
    assert vendored["source"] == "thebushidocollective/han"
    assert vendored["sourceType"] == "github"
    assert vendored["skillPath"] == "plugins/patterns/bdd/skills/vendored/SKILL.md"
    assert vendored["computedHash"] == hash_skill_directory(
        tmp_path / ".agents" / "skills" / "vendored"
    )
    assert lock["skills"]["local"]["sourceType"] == "local"
