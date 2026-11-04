"""Tests for the Python Backlog checker."""

from pathlib import Path

import pytest

from scripts.backlog_check import (
    extract_frontmatter_list,
    main,
    parse_frontmatter,
    validate_backlog,
)


def test_parses_scalar_frontmatter_and_lists() -> None:
    text = (
        "---\nid: UNI-1\ntitle: Example\nstatus: To Do\ndependencies: [UNI-2, UNI-3]\n---\nBody\n"
    )

    frontmatter = parse_frontmatter(text)

    assert frontmatter == {
        "id": "UNI-1",
        "title": "Example",
        "status": "To Do",
        "dependencies": "[UNI-2, UNI-3]",
    }
    assert parse_frontmatter("no frontmatter\n") is None
    assert extract_frontmatter_list(text, "dependencies") == ["UNI-2", "UNI-3"]
    assert extract_frontmatter_list(text, "milestone") == []


def test_validates_a_minimal_backlog(tmp_path: Path) -> None:
    decisions = tmp_path / "backlog" / "decisions"
    tasks = tmp_path / "backlog" / "tasks"
    decisions.mkdir(parents=True)
    tasks.mkdir(parents=True)
    (decisions / "decision-1 - test.md").write_text(
        "---\n"
        "id: decision-1\n"
        "title: Test decision\n"
        "status: accepted\n"
        "---\n"
        "[task](../tasks/uni-1%20-%20test.md)\n",
        encoding="utf-8",
    )
    (tasks / "uni-1 - test.md").write_text(
        "---\nid: UNI-1\ntitle: Test task\nstatus: To Do\n---\nDo it.\n",
        encoding="utf-8",
    )

    assert validate_backlog(tmp_path) == []


def test_validates_milestone_and_task_dependency_references(tmp_path: Path) -> None:
    milestones = tmp_path / "backlog" / "milestones"
    tasks = tmp_path / "backlog" / "tasks"
    milestones.mkdir(parents=True)
    tasks.mkdir(parents=True)
    (milestones / "m-1 - foundation.md").write_text(
        "---\nid: m-1\ntitle: Foundation\n---\n",
        encoding="utf-8",
    )
    (tasks / "uni-1 - parent.md").write_text(
        "---\nid: UNI-1\ntitle: Parent\nstatus: To Do\nmilestone: m-1\n"
        "dependencies:\n  - UNI-2\n---\n",
        encoding="utf-8",
    )
    (tasks / "uni-2 - prerequisite.md").write_text(
        "---\nid: UNI-2\ntitle: Prerequisite\nstatus: To Do\n---\n",
        encoding="utf-8",
    )

    assert validate_backlog(tmp_path) == []

    task = tasks / "uni-1 - parent.md"
    task.write_text(
        task.read_text(encoding="utf-8").replace("m-1", "m-404").replace("UNI-2", "UNI-404"),
        encoding="utf-8",
    )
    errors = "\n".join(validate_backlog(tmp_path))
    assert "unknown milestone reference m-404" in errors
    assert "unknown task dependency UNI-404" in errors


def test_reports_duplicate_ids_invalid_status_and_broken_links(tmp_path: Path) -> None:
    decisions = tmp_path / "backlog" / "decisions"
    decisions.mkdir(parents=True)
    (decisions / "decision-1 - first.md").write_text(
        "---\nid: decision-1\ntitle: First\nstatus: pending\n---\n[missing](missing.md)\n",
        encoding="utf-8",
    )
    (decisions / "decision-1 - second.md").write_text(
        "---\nid: decision-1\ntitle: Second\nstatus: accepted\n---\nBody\n",
        encoding="utf-8",
    )

    errors = "\n".join(validate_backlog(tmp_path))

    assert "duplicate id decision-1" in errors
    assert "invalid decision status pending" in errors
    # Relative-link resolution is alint's rule (backlog-relative-links-resolve),
    # which also covers links that leave backlog/.
    assert "broken Backlog link" not in errors


def test_reports_missing_backlog_and_frontmatter_problems(tmp_path: Path) -> None:
    assert validate_backlog(tmp_path) == ["backlog/ directory is missing"]

    tasks = tmp_path / "backlog" / "tasks"
    docs = tmp_path / "backlog" / "docs"
    tasks.mkdir(parents=True)
    docs.mkdir(parents=True)
    (tasks / "uni-1 - missing-frontmatter.md").write_text("Body only\n", encoding="utf-8")
    (tasks / "uni-2 - missing-id.md").write_text(
        "---\ntitle: Missing ID\nstatus: To Do\n---\nBody\n",
        encoding="utf-8",
    )
    (tasks / "wrong-name.md").write_text(
        "---\nid: UNI-3\nstatus: Blocked\n---\n"
        "[web](https://example.test) [anchor](#section) [root](/tmp/x.md) "
        "[outside](../../outside.md) [directory](../tasks/) [query](?view=1)\n",
        encoding="utf-8",
    )
    (docs / "doc-1 - note.md").write_text(
        "---\nid: doc-1\ntitle: Note\n---\nBody\n",
        encoding="utf-8",
    )

    errors = "\n".join(validate_backlog(tmp_path))

    assert "missing frontmatter" in errors
    assert "missing frontmatter id" in errors
    assert "missing frontmatter title" in errors
    assert 'filename must start with "UNI-3 -"' in errors
    assert "invalid task status Blocked" in errors
    assert "broken Backlog link" not in errors


def test_main_prints_success_and_failure(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    assert main(["--root", str(tmp_path)]) == 1
    captured = capsys.readouterr()
    assert "backlog/ directory is missing" in captured.err

    backlog = tmp_path / "backlog"
    backlog.mkdir()
    (backlog / "README.md").write_text("# Backlog\n", encoding="utf-8")

    assert main(["--root", str(tmp_path)]) == 0
    captured = capsys.readouterr()
    assert captured.out == "backlog-check: Backlog is valid\n"
