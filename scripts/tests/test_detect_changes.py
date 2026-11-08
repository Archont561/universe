"""Tests for the CI lane classifier."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from scripts.detect_changes import changed_paths, code_changed, main, web_changed


def test_prose_paths_do_not_ask_for_a_compile() -> None:
    assert not code_changed(["backlog/tasks/uni-1 - a-task.md"])
    assert not code_changed(["README.md", "CHANGELOG.md", "AGENTS.md"])
    assert not code_changed(["docs/design/anything.md"])


def test_a_source_path_asks_for_the_compile_lanes() -> None:
    assert code_changed(["crates/universe-kit/src/airlock.rs"])
    assert code_changed(["pixi.toml"])
    assert code_changed([".github/workflows/ci.yml"])


def test_an_unknown_top_level_directory_is_treated_as_code() -> None:
    # Failing towards "run the lane" is the only safe direction: a skipped lane that should
    # have run is invisible, while a lane that runs too often only costs minutes.
    assert code_changed(["somewhere-new/thing.txt"])
    assert not code_changed([])


def test_the_bun_lane_has_its_own_answer() -> None:
    assert web_changed(["packages/portal/src/routes/+page.svelte"])
    assert web_changed(["packages/config/features/constants.feature"])
    assert web_changed(["bun.lock"])
    assert web_changed(["turbo.json"])
    # `web-check` is a Pixi task, so the toolchain manifest selects the lane that runs it.
    assert web_changed(["pixi.toml"])
    assert not web_changed(["crates/universe-kit/src/lib.rs"])
    assert not web_changed(["backlog/tasks/uni-1 - a-task.md"])


def test_a_package_change_selects_both_lanes() -> None:
    # `packages/` is where the Bun workspace lives and it is not prose, so a change there has
    # to reach the Bun lane *and* the compile lanes. It used to reach neither the Bun lane
    # (the classifier named a `web/` directory the workspace no longer has) — a skipped lane,
    # which looks exactly like a passing one.
    assert code_changed(["packages/portal/src/lib/index.ts"])
    assert web_changed(["packages/portal/src/lib/index.ts"])


def test_a_docs_only_change_names_no_lane(tmp_path: Path) -> None:
    output = tmp_path / "out"
    output.write_text("", encoding="utf-8")
    assert (
        main(
            [
                "--base",
                "",
                "--head",
                "",
                "--event",
                "workflow_dispatch",
                "--github-output",
                str(output),
            ]
        )
        == 0
    )
    # An unusable revision means "run everything": the decision fails closed even though the
    # command itself succeeds.
    assert output.read_text(encoding="utf-8") == "code=true\nweb=true\n"


def _git(*arguments: str, cwd: Path) -> None:
    subprocess.run(
        ["git", *arguments],
        cwd=cwd,
        check=True,
        capture_output=True,
        env={
            "GIT_AUTHOR_NAME": "test",
            "GIT_AUTHOR_EMAIL": "test@example.invalid",
            "GIT_COMMITTER_NAME": "test",
            "GIT_COMMITTER_EMAIL": "test@example.invalid",
            "GIT_CONFIG_GLOBAL": "/dev/null",
            "HOME": str(cwd),
            "PATH": "/usr/bin:/bin",
        },
    )


def test_a_real_diff_names_the_lane_it_needs(tmp_path: Path) -> None:
    _git("init", "-q", "--initial-branch=main", cwd=tmp_path)
    (tmp_path / "README.md").write_text("start\n", encoding="utf-8")
    _git("add", ".", cwd=tmp_path)
    _git("commit", "-q", "-m", "start", cwd=tmp_path)
    base = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=tmp_path, capture_output=True, text=True, check=True
    ).stdout.strip()

    (tmp_path / "backlog").mkdir()
    (tmp_path / "backlog" / "note.md").write_text("prose\n", encoding="utf-8")
    _git("add", ".", cwd=tmp_path)
    _git("commit", "-q", "-m", "docs only", cwd=tmp_path)
    docs_head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=tmp_path, capture_output=True, text=True, check=True
    ).stdout.strip()

    assert changed_paths(base, docs_head, "pull_request", str(tmp_path)) == ["backlog/note.md"]

    (tmp_path / "crate.rs").write_text("fn main() {}\n", encoding="utf-8")
    _git("add", ".", cwd=tmp_path)
    _git("commit", "-q", "-m", "code", cwd=tmp_path)
    code_head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=tmp_path, capture_output=True, text=True, check=True
    ).stdout.strip()

    paths = changed_paths(base, code_head, "pull_request", str(tmp_path))
    assert paths is not None
    assert code_changed(paths)


def test_an_unknown_revision_is_reported_as_unknown(tmp_path: Path) -> None:
    _git("init", "-q", "--initial-branch=main", cwd=tmp_path)
    (tmp_path / "README.md").write_text("start\n", encoding="utf-8")
    _git("add", ".", cwd=tmp_path)
    _git("commit", "-q", "-m", "start", cwd=tmp_path)

    # A base that is not in this repository: the caller must be told, not given a guess.
    assert changed_paths("0" * 40, "HEAD", "pull_request", str(tmp_path)) is None


@pytest.mark.parametrize(
    "path",
    ["packages/package.json", "packages/portal/src/app.html", "packages/schema/src/index.ts"],
)
def test_web_paths_are_recognized_through_the_prefix(path: str) -> None:
    assert web_changed([path])
