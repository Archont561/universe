"""Tests for the Python Backlog checker."""

from pathlib import Path

import pytest

from scripts.backlog_check import extract_markdown_links, main, parse_frontmatter, validate_backlog


def test_parses_scalar_frontmatter_and_links() -> None:
    parsed = parse_frontmatter(
        "---\nid: decision-1\ntitle: 'A title: with punctuation'\nlabels:\n  - docs\n---\nBody"
    )

    assert parsed == {
        "id": "decision-1",
        "labels": "",
        "title": "A title: with punctuation",
    }
    assert parse_frontmatter("plain text") is None
    assert parse_frontmatter("---\nid: decision-1\n") is None
    assert parse_frontmatter("---\nnot frontmatter\nid: doc-1\n---\nBody") == {"id": "doc-1"}
    assert extract_markdown_links('[a](b.md "title") ![i](x.png) [web](https://example.test)') == [
        "b.md",
        "x.png",
        "https://example.test",
    ]


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
    assert "broken Backlog link missing.md" in errors


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
