"""Tests for the Python repository checker."""

from pathlib import Path

import pytest

from scripts.repo_check import (
    extract_toml_version,
    extract_workflow_refs,
    is_immutable_sha,
    main,
    validate_repo,
)


def test_recognises_full_sha_workflow_refs_only() -> None:
    sha = "0123456789abcdef0123456789abcdef01234567"
    refs = extract_workflow_refs(
        f"steps:\n  - uses: actions/checkout@{sha} # v1\n  - uses: owner/action@v1\n"
    )

    assert refs == [sha, "v1"]
    assert is_immutable_sha(sha)
    assert not is_immutable_sha("v1")
    assert not is_immutable_sha(sha.upper())


def test_extracts_the_first_toml_version() -> None:
    assert extract_toml_version('name = "x"\nversion = "1.2.3"\nversion = "9.9.9"') == "1.2.3"
    assert extract_toml_version('name = "x"') is None


def test_the_standalone_repository_passes_repo_check() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    assert validate_repo(repo_root) == []


def test_reports_missing_forbidden_and_unreadable_paths(tmp_path: Path) -> None:
    scripts = tmp_path / "scripts"
    scripts.mkdir()
    (tmp_path / "LICENSE").write_text("placeholder\n", encoding="utf-8")
    (tmp_path / ".gitkeep").write_text("", encoding="utf-8")
    (scripts / "legacy.ts").write_text("export {};\n", encoding="utf-8")
    legacy_name = "conda" + "-universe.txt"
    (tmp_path / legacy_name).write_text("legacy path\n", encoding="utf-8")
    (tmp_path / "legacy-content.txt").write_text("air" + "pack\n", encoding="utf-8")
    (tmp_path / "broken-link").symlink_to(tmp_path / "missing-target")

    errors = "\n".join(validate_repo(tmp_path))

    assert "required file is missing: AGENTS.md" in errors
    assert "forbidden standalone-cleanup path exists: LICENSE" in errors
    assert "forbidden license/notice/placeholder file: .gitkeep" in errors
    assert "TypeScript repository script is forbidden: scripts/legacy.ts" in errors
    assert f"legacy repository reference in path: {legacy_name}" in errors
    assert "legacy repository reference in content: legacy-content.txt" in errors
    assert "cannot read broken-link" in errors


def test_reports_version_and_workflow_drift(tmp_path: Path) -> None:
    workflows = tmp_path / ".github" / "workflows"
    workflows.mkdir(parents=True)
    (tmp_path / "Cargo.toml").write_text('version = "1.0.0"\n', encoding="utf-8")
    (tmp_path / "pixi.toml").write_text('version = "2.0.0"\n', encoding="utf-8")
    (workflows / "ci.yml").write_text("steps:\n  - uses: actions/checkout@v4\n", encoding="utf-8")

    errors = "\n".join(validate_repo(tmp_path))

    assert "version drift: Cargo=1.0.0, Pixi=2.0.0" in errors
    assert "action ref is not a full SHA: v4" in errors

    (tmp_path / "pixi.toml").write_text('name = "universe"\n', encoding="utf-8")
    errors = "\n".join(validate_repo(tmp_path))
    assert "Cargo and Pixi must both declare a version" in errors


def test_main_prints_success_and_failure(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    assert main(["--root", str(tmp_path)]) == 1
    captured = capsys.readouterr()
    assert "required file is missing" in captured.err

    repo_root = Path(__file__).resolve().parents[2]
    assert main(["--root", str(repo_root)]) == 0
    captured = capsys.readouterr()
    assert captured.out == "repo-check: repository invariants are valid\n"
