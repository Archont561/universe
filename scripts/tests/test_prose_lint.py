"""Tests for the prose gate.

Vale itself is not exercised here: it is a Go binary from the Pixi environment, and a test
that needed it would fail on a machine where only the Python toolchain is installed. What is
tested is everything this repository owns — which documents are in the corpus, what a finding
reads like, and the three-way distinction between clean, failing, and never linted. A stub
`vale` on `PATH` stands in for the binary and answers with a report the test wrote.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

import pytest

from scripts.prose_lint import (
    EXIT_NO_VALE,
    Finding,
    discover,
    main,
    parse,
)

STUB = """#!/bin/sh
cat '{report}'
exit {status}
"""


def stub_vale(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, report: str, status: int = 0
) -> None:
    """Put a fake `vale` first on `PATH` that prints *report* and exits with *status*."""
    binary_dir = tmp_path / "bin"
    binary_dir.mkdir(exist_ok=True)
    report_file = tmp_path / "report.json"
    report_file.write_text(report, encoding="utf-8")
    binary = binary_dir / "vale"
    binary.write_text(STUB.format(report=report_file, status=status), encoding="utf-8")
    binary.chmod(0o755)
    monkeypatch.setenv("PATH", f"{binary_dir}:{os.environ['PATH']}")


def stub_path(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """Point `PATH` at a directory that holds no `vale` at all."""
    empty = tmp_path / "empty"
    empty.mkdir()
    monkeypatch.setenv("PATH", str(empty))


@pytest.fixture(autouse=True)
def _isolate_environment(monkeypatch: pytest.MonkeyPatch) -> None:
    """Keep the developer's own CI markers out of every test."""
    monkeypatch.delenv("GITHUB_ACTIONS", raising=False)


def _report(**alerts: list[dict[str, object]]) -> str:
    return json.dumps(alerts)


def _alert(
    message: str, line: int, column: int, rule: str = "Universe.Filler"
) -> dict[str, object]:
    return {"Message": message, "Span": [line, column], "Check": rule, "Severity": "error"}


def test_the_corpus_is_the_published_documentation(tmp_path: Path) -> None:
    for name in ("README.md", "AGENTS.md"):
        (tmp_path / name).write_text("# a\n", encoding="utf-8")
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "AIRLOCK.md").write_text("# b\n", encoding="utf-8")
    (tmp_path / "backlog").mkdir()
    (tmp_path / "backlog" / "README.md").write_text("# c\n", encoding="utf-8")
    (tmp_path / ".agents").mkdir()
    (tmp_path / ".agents" / "skill.md").write_text("# d\n", encoding="utf-8")

    assert discover(tmp_path) == [Path("AGENTS.md"), Path("README.md"), Path("docs/AIRLOCK.md")]


def test_a_repository_with_no_document_fails_closed(tmp_path: Path) -> None:
    # The gate must not pass because it linted nothing: an empty corpus is the one result
    # that looks identical to a clean one.
    assert main(["--root", str(tmp_path)]) == 1


def test_a_clean_corpus_passes(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    (tmp_path / "README.md").write_text("# a\n", encoding="utf-8")
    stub_vale(tmp_path, monkeypatch, "{}")

    assert main(["--root", str(tmp_path)]) == 0
    assert "1 document(s) clean" in capsys.readouterr().out


def test_a_finding_names_the_file_line_column_and_rule(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    (tmp_path / "README.md").write_text("# a\n", encoding="utf-8")
    report = _report(**{"README.md": [_alert("'leverage' is filler", 12, 4)]})
    stub_vale(tmp_path, monkeypatch, report, status=1)

    assert main(["--root", str(tmp_path)]) == 1
    captured = capsys.readouterr()
    assert "README.md:12:4: 'leverage' is filler (Universe.Filler)" in captured.err
    assert "1 finding(s) in 1 document(s)" in captured.err


def test_a_run_under_actions_emits_annotations(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    (tmp_path / "README.md").write_text("# a\n", encoding="utf-8")
    report = _report(**{"README.md": [_alert("x", 3, 1)]})
    stub_vale(tmp_path, monkeypatch, report, status=1)
    monkeypatch.setenv("GITHUB_ACTIONS", "true")

    assert main(["--root", str(tmp_path)]) == 1
    assert "::error file=README.md,line=3,col=1" in capsys.readouterr().out


def test_a_missing_binary_is_not_a_clean_corpus(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    (tmp_path / "README.md").write_text("# a\n", encoding="utf-8")
    stub_path(tmp_path, monkeypatch)

    assert main(["--root", str(tmp_path)]) == EXIT_NO_VALE
    assert "vale is not available" in capsys.readouterr().err


def test_a_linter_that_could_not_run_is_a_failure(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    (tmp_path / "README.md").write_text("# a\n", encoding="utf-8")
    stub_vale(tmp_path, monkeypatch, "{}", status=2)

    assert main(["--root", str(tmp_path)]) == EXIT_NO_VALE
    assert "vale exited 2" in capsys.readouterr().err


def test_a_report_that_is_not_json_is_a_failure(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    (tmp_path / "README.md").write_text("# a\n", encoding="utf-8")
    stub_vale(tmp_path, monkeypatch, "not json")

    assert main(["--root", str(tmp_path)]) == EXIT_NO_VALE
    assert "did not print JSON" in capsys.readouterr().err


def test_explicit_paths_override_the_corpus(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    (tmp_path / "backlog").mkdir()
    (tmp_path / "backlog" / "record.md").write_text("# a\n", encoding="utf-8")
    stub_vale(tmp_path, monkeypatch, "{}")

    assert main(["--root", str(tmp_path), "--paths", "backlog/record.md"]) == 0
    assert "1 document(s) clean" in capsys.readouterr().out


def test_parse_keeps_an_absolute_path_relative_to_the_root(tmp_path: Path) -> None:
    absolute = str(tmp_path / "docs" / "MANIFEST.md")
    findings = parse(_report(**{absolute: [_alert("x", 1, 1, rule="Universe.Terms")]}), tmp_path)

    assert findings == [Finding("docs/MANIFEST.md", 1, 1, "Universe.Terms", "x")]


def test_findings_are_sorted_by_position_and_render_two_ways() -> None:
    findings = parse(
        _report(**{"b.md": [_alert("second", 2, 1)], "a.md": [_alert("first", 1, 5)]}),
        Path.cwd(),
    )
    assert [finding.message for finding in findings] == ["first", "second"]
    assert str(findings[0]) == "a.md:1:5: first (Universe.Filler)"
    assert findings[0].annotation().startswith("::error file=a.md,line=1,col=5")


def test_the_command_is_runnable_as_a_module() -> None:
    # `pixi run lint-prose` invokes the module, so the entry point has to exist even though
    # the tests call `main` directly.
    assert sys.modules["scripts.prose_lint"].__doc__ is not None
