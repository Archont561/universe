"""Tests for the Gherkin feature-file linter."""

from pathlib import Path

import pytest

from scripts.gherkin_lint import discover, lint, main

CLEAN = """Feature: Publishing a channel

  Scenario: A published channel is restorable
    Given a channel published to a branch
    When a consumer clones that branch
    Then the channel metadata is present
"""

EMPTY_SCENARIO = """Feature: Publishing a channel

  Scenario: An empty scenario
"""


def _write(root: Path, relative: str, text: str) -> Path:
    """Write one feature file inside a directory named `features`."""
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path


def test_discovers_every_features_directory_and_ignores_everything_else(tmp_path: Path) -> None:
    nested = _write(tmp_path, "crates/one/tests/features/a.feature", CLEAN)
    sibling = _write(tmp_path, "crates/two/tests/features/b.feature", CLEAN)
    # Not inside a directory named `features`, not a `.feature` file, or not ours.
    _write(tmp_path, "crates/one/tests/notes.feature", CLEAN)
    _write(tmp_path, "crates/one/tests/features/readme.md", "not a feature file")
    _write(tmp_path, "node_modules/pkg/features/vendored.feature", CLEAN)
    _write(tmp_path, "target/features/generated.feature", CLEAN)

    discovered = discover(tmp_path)

    assert discovered == [nested, sibling]


def test_a_clean_feature_file_produces_no_findings(tmp_path: Path) -> None:
    _write(tmp_path, "features/clean.feature", CLEAN)
    files, findings = lint(tmp_path)
    assert len(files) == 1
    assert findings == []


def test_a_scenario_without_steps_is_reported_at_the_scenario(tmp_path: Path) -> None:
    path = _write(tmp_path, "features/empty.feature", EMPTY_SCENARIO)
    _, findings = lint(tmp_path)

    assert [finding.name for finding in findings] == ["empty-scenario"]
    assert findings[0].file == str(path)
    assert findings[0].line == 3
    assert "does not contain any steps" in findings[0].text


def test_an_unparseable_file_is_a_finding_rather_than_a_crash(tmp_path: Path) -> None:
    # A broken file must be a failed gate with a message, not a traceback the reader has
    # to translate.
    _write(tmp_path, "features/broken.feature", "Scenario: no feature keyword\n  Given a step\n")
    _, findings = lint(tmp_path)

    assert [finding.name for finding in findings] == ["unparseable-file"]


def test_main_fails_closed_when_there_is_no_feature_file_to_lint(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    assert main(["--root", str(tmp_path)]) == 1
    captured = capsys.readouterr()
    assert "the gate would lint nothing" in captured.err


def test_main_reports_findings_on_stderr_and_exits_non_zero(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    _write(tmp_path, "features/empty.feature", EMPTY_SCENARIO)

    assert main(["--root", str(tmp_path)]) == 1
    captured = capsys.readouterr()
    assert captured.out == ""
    assert "empty-scenario" in captured.err
    assert "1 finding(s) in 1 feature file(s)" in captured.err


def test_main_prints_the_count_it_linted(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    _write(tmp_path, "features/clean.feature", CLEAN)

    assert main(["--root", str(tmp_path)]) == 0
    captured = capsys.readouterr()
    assert captured.out == "gherkin-lint: 1 feature file(s) clean\n"


def test_the_repository_features_are_linted_and_clean() -> None:
    # The gate's own regression test: if the feature set moves or breaks, this fails here
    # rather than in CI, and it fails with the count so an empty discovery is visible.
    root = Path(__file__).resolve().parents[2]
    files, findings = lint(root)

    assert files, "the repository must have at least one features/*.feature file"
    assert findings == [], "\n".join(str(finding) for finding in findings)
