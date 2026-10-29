"""Tests for the offline-first audit gate."""

from collections.abc import Mapping, Sequence
from pathlib import Path

import pytest

from scripts.audit import (
    EXIT_ENVIRONMENT,
    EXIT_FAILURE,
    EXIT_OK,
    RunOutcome,
    advisory_database,
    audit_argv,
    cargo_home,
    collapse_yanked_notices,
    deny_argv,
    has_index_cache,
    main,
    resolve_cache,
    run,
)


class RecordingRunner:
    """Fake command runner that records argv and replays canned outcomes."""

    def __init__(self, outcomes: list[RunOutcome] | None = None) -> None:
        self.calls: list[tuple[str, ...]] = []
        self.timeouts: list[float] = []
        self.envs: list[Mapping[str, str] | None] = []
        self._outcomes = outcomes or []

    def __call__(
        self,
        argv: Sequence[str],
        timeout: float,
        env: Mapping[str, str] | None = None,
    ) -> RunOutcome:
        self.calls.append(tuple(argv))
        self.timeouts.append(timeout)
        self.envs.append(env)
        if self._outcomes:
            return self._outcomes.pop(0)
        return RunOutcome(code=0, stdout="", stderr="")


def _seed_database(home: Path, *, index: bool = False) -> Path:
    """Create a cargo-deny shaped advisory clone under a Cargo home."""
    database = home / "advisory-dbs" / "advisory-db-abc"
    (database / "crates").mkdir(parents=True, exist_ok=True)
    if index:
        (home / "registry" / "index" / "index.crates.io-1").mkdir(parents=True, exist_ok=True)
    return database


def test_cargo_home_honours_the_environment_override() -> None:
    assert cargo_home({"CARGO_HOME": "/opt/cargo-home"}) == Path("/opt/cargo-home")
    assert cargo_home({}) == Path("~/.cargo").expanduser()


def test_resolves_the_cache_under_cargo_home() -> None:
    cache = resolve_cache(Path("/cargo-home"))

    assert cache.advisory_dbs == Path("/cargo-home/advisory-dbs")
    assert cache.index == Path("/cargo-home/registry/index")


def test_advisory_database_needs_a_seeded_clone(tmp_path: Path) -> None:
    cache = resolve_cache(tmp_path)

    assert advisory_database(cache) is None

    (tmp_path / "advisory-dbs").mkdir()
    (tmp_path / "advisory-dbs" / "db.lock").write_text("", encoding="utf-8")
    assert advisory_database(cache) is None

    database = _seed_database(tmp_path)
    assert advisory_database(cache) == database


def test_index_cache_is_separate_and_optional(tmp_path: Path) -> None:
    cache = resolve_cache(tmp_path)
    _seed_database(tmp_path)

    assert not has_index_cache(cache)
    _seed_database(tmp_path, index=True)
    assert has_index_cache(cache)


def test_deny_runs_offline_with_policy() -> None:
    assert deny_argv(offline=True) == ("cargo", "deny", "--offline", "check")
    assert deny_argv(offline=False) == ("cargo", "deny", "check")


def test_audit_pins_the_shared_database_and_disables_the_fetch() -> None:
    assert audit_argv(Path("/cargo-home/advisory-dbs/advisory-db-abc")) == (
        "cargo",
        "audit",
        "--no-fetch",
        "--db",
        "/cargo-home/advisory-dbs/advisory-db-abc",
        "--deny",
        "warnings",
    )


def test_collapses_repeated_yanked_notices_into_one_count() -> None:
    messages = (
        "    Loaded 1381 security advisories\n"
        "error: couldn't check if the package is yanked: No such crate in crates.io index: serde\n"
        "error: couldn't check if the package is yanked: No such crate in crates.io index: syn\n"
    )

    remaining, count = collapse_yanked_notices(messages)

    assert count == 2
    assert remaining == ["    Loaded 1381 security advisories"]


def test_timeout_is_reported_instead_of_hanging(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def explode(*_args: object, **_kwargs: object) -> None:
        raise __import__("subprocess").TimeoutExpired(cmd="cargo audit", timeout=5.0)

    monkeypatch.setattr("scripts.audit.subprocess.run", explode)

    outcome = run(("cargo", "audit"), timeout=5.0)

    assert outcome.code == 124
    assert "timed out after 5s" in outcome.stderr


def test_check_fails_fast_when_offline_without_a_cache(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    runner = RecordingRunner()

    code = main(
        ["check"],
        environ={"CARGO_HOME": str(tmp_path), "CARGO_NET_OFFLINE": "true"},
        runner=runner,
    )

    assert code == EXIT_ENVIRONMENT
    assert runner.calls == []
    assert "pixi run audit-sync" in capsys.readouterr().err


def test_check_runs_both_tools_offline_when_the_cache_exists(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    database = _seed_database(tmp_path)
    runner = RecordingRunner()

    code = main(["check"], environ={"CARGO_HOME": str(tmp_path)}, runner=runner)

    assert code == EXIT_OK
    assert runner.calls == [deny_argv(offline=True), audit_argv(database)]
    assert [call_env for call_env in runner.envs] == [None, None]
    assert "yanked" in capsys.readouterr().err


def test_check_is_silent_about_degradation_when_the_index_is_cached(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    database = _seed_database(tmp_path, index=True)
    runner = RecordingRunner()

    assert main(["check"], environ={"CARGO_HOME": str(tmp_path)}, runner=runner) == EXIT_OK
    assert runner.calls == [deny_argv(offline=True), audit_argv(database)]
    assert "degraded" not in capsys.readouterr().err


def test_check_bootstraps_once_then_stays_offline(tmp_path: Path) -> None:
    calls: list[tuple[str, ...]] = []

    def runner(
        argv: Sequence[str], timeout: float, env: Mapping[str, str] | None = None
    ) -> RunOutcome:
        calls.append(tuple(argv))
        if tuple(argv) == ("cargo", "deny", "fetch", "db"):
            _seed_database(tmp_path)
        return RunOutcome(code=0, stdout="", stderr="")

    code = main(["check"], environ={"CARGO_HOME": str(tmp_path)}, runner=runner)

    assert code == EXIT_OK
    assert calls[0] == ("cargo", "deny", "fetch", "db")
    assert calls[-2] == deny_argv(offline=True)
    assert calls[-1][:3] == ("cargo", "audit", "--no-fetch")


def test_check_propagates_a_tool_failure(tmp_path: Path) -> None:
    _seed_database(tmp_path)
    runner = RecordingRunner([RunOutcome(code=1, stdout="", stderr="advisory found")])

    code = main(["check"], environ={"CARGO_HOME": str(tmp_path)}, runner=runner)

    assert code == EXIT_FAILURE
    assert len(runner.calls) == 1


def test_sync_seeds_the_database_and_the_index() -> None:
    runner = RecordingRunner()

    code = main(["sync"], environ={"CARGO_HOME": "/tmp/cargo-home"}, runner=runner)

    assert code == EXIT_OK
    assert runner.calls == [
        ("cargo", "deny", "fetch", "db"),
        ("cargo", "deny", "fetch", "index"),
    ]


def test_sync_fails_when_the_database_cannot_be_fetched() -> None:
    runner = RecordingRunner([RunOutcome(code=1, stdout="", stderr="network down")])

    code = main(["sync"], environ={"CARGO_HOME": "/tmp/cargo-home"}, runner=runner)

    assert code == EXIT_ENVIRONMENT
    assert len(runner.calls) == 1


def test_sync_tolerates_an_unreachable_index(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    runner = RecordingRunner(
        [RunOutcome(code=0, stdout="", stderr=""), RunOutcome(code=1, stdout="", stderr="")]
    )

    code = main(["sync"], environ={"CARGO_HOME": str(tmp_path)}, runner=runner)

    assert code == EXIT_OK
    assert "yanked checks stay degraded" in capsys.readouterr().err


def test_rejects_unknown_modes(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["verify"], environ={}, runner=RecordingRunner()) == EXIT_ENVIRONMENT
    assert "unknown mode" in capsys.readouterr().err
