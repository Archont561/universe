"""Tests for the pixi-sandbox diagnostic annotation renderer."""

from pathlib import Path

import pytest

from scripts.print_sandbox_logs import main, render_annotations


def test_missing_and_empty_log_directories_emit_notices(tmp_path: Path) -> None:
    missing = tmp_path / "missing"
    empty = tmp_path / "empty"
    empty.mkdir()

    assert render_annotations(missing) == [
        "::notice::No pixi-sandbox diagnostic directory was created."
    ]
    assert render_annotations(empty) == ["::notice::No pixi-sandbox diagnostic files were created."]


def test_annotations_escape_github_workflow_command_metacharacters(tmp_path: Path) -> None:
    (tmp_path / "failure.log").write_text("safe%bad:comma,carriage\rreturn\n", encoding="utf-8")

    assert render_annotations(tmp_path) == [
        "::error title=Pixi sandbox diagnostics::safe%25bad%3Acomma%2Ccarriage%0Dreturn"
    ]


def test_only_the_last_requested_lines_are_emitted(tmp_path: Path) -> None:
    (tmp_path / "failure.log").write_text("one\ntwo\nthree\n", encoding="utf-8")

    assert render_annotations(tmp_path, tail_lines=2) == [
        "::error title=Pixi sandbox diagnostics::two",
        "::error title=Pixi sandbox diagnostics::three",
    ]


def test_cli_prints_notices_and_rejects_invalid_tail_size(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    assert main([str(tmp_path / "missing")]) == 0
    assert (
        capsys.readouterr().out == "::notice::No pixi-sandbox diagnostic directory was created.\n"
    )

    with pytest.raises(SystemExit) as error:
        main([str(tmp_path), "--tail-lines", "0"])
    assert error.value.code == 2
