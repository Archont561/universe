"""Run the Rust dependency audit gate offline-first.

Both audit tools need a RustSec advisory database before they can run, and the
yanked-crate check additionally needs a crates.io index cache. Those live in
``CARGO_HOME`` as caches, never in the repository. This module makes the gate
honest about that split:

- one advisory clone seeded by ``cargo deny fetch db`` serves both tools, so
  cargo-audit is always pointed at it with ``--db`` and ``--no-fetch``;
- when the cache exists, the gate never touches the network and finishes in
  well under a second;
- when the cache is missing and network access is allowed, the gate bootstraps
  it once with a bounded timeout instead of hanging on an unreachable index;
- when the cache is missing and network access is denied, the gate fails closed
  with an actionable message instead of waiting out a TCP retry storm;
- when the advisory data is present but the index cache is not, the yanked check
  is reported as one explicit degradation notice naming the skipped count, so a
  degraded run is never mistaken for a complete one.
"""

import os
import subprocess
import sys
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path

from scripts.lib.files import path_exists

EXIT_OK = 0
EXIT_FAILURE = 1
EXIT_ENVIRONMENT = 2

CHECK_TIMEOUT_SECONDS = 120.0
SYNC_TIMEOUT_SECONDS = 600.0
INDEX_TIMEOUT_SECONDS = 120.0
TIMEOUT_EXIT_CODE = 124

MODES = ("check", "sync")
_OFFLINE_TRUE_VALUES = {"1", "true", "yes"}

ADVISORY_DBS_DIRNAME = "advisory-dbs"
INDEX_DIRNAME = "registry/index"
ADVISORY_DB_MARKERS = ("crates", "rust")
YANKED_NOTICE_FRAGMENT = "couldn't check if the package is yanked"


@dataclass(frozen=True)
class AdvisoryCache:
    """Cargo-home locations the audit gate reads."""

    advisory_dbs: Path
    index: Path


@dataclass(frozen=True)
class RunOutcome:
    """Captured result of one audit command."""

    code: int
    stdout: str
    stderr: str


Runner = Callable[[Sequence[str], float, Mapping[str, str] | None], RunOutcome]


def cargo_home(environ: Mapping[str, str]) -> Path:
    """Return the Cargo home directory the audit caches belong to."""
    configured = environ.get("CARGO_HOME")
    return Path(configured) if configured else Path("~/.cargo").expanduser()


def resolve_cache(home: Path) -> AdvisoryCache:
    """Derive the advisory and index cache locations from one Cargo home."""
    return AdvisoryCache(
        advisory_dbs=home / ADVISORY_DBS_DIRNAME,
        index=home / INDEX_DIRNAME,
    )


def advisory_database(cache: AdvisoryCache) -> Path | None:
    """Return the seeded advisory clone, if cargo-deny has one."""
    if not path_exists(cache.advisory_dbs):
        return None
    for candidate in sorted(cache.advisory_dbs.iterdir()):
        if candidate.is_dir() and any(
            path_exists(candidate / marker) for marker in ADVISORY_DB_MARKERS
        ):
            return candidate
    return None


def has_index_cache(cache: AdvisoryCache) -> bool:
    """Report whether a crates.io index cache is available for yanked checks."""
    return path_exists(cache.index) and any(cache.index.iterdir())


def deny_argv(*, offline: bool) -> tuple[str, ...]:
    """Build the cargo-deny invocation for the requested network policy."""
    return ("cargo", "deny", "--offline", "check") if offline else ("cargo", "deny", "check")


def audit_argv(database: Path) -> tuple[str, ...]:
    """Build an offline cargo-audit invocation pinned to the local clone."""
    return ("cargo", "audit", "--no-fetch", "--db", str(database), "--deny", "warnings")


def fetch_argv() -> tuple[str, ...]:
    """Build the required advisory-database bootstrap command (networked)."""
    return ("cargo", "deny", "fetch", "db")


def fetch_index_argv() -> tuple[str, ...]:
    """Build the optional crates.io index bootstrap command (networked)."""
    return ("cargo", "deny", "fetch", "index")


def run(argv: Sequence[str], timeout: float, env: Mapping[str, str] | None = None) -> RunOutcome:
    """Run one command under a hard timeout and capture its output."""
    try:
        completed = subprocess.run(
            list(argv),
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
            env=None if env is None else {**os.environ, **env},
        )
    except subprocess.TimeoutExpired:
        return RunOutcome(
            code=TIMEOUT_EXIT_CODE,
            stdout="",
            stderr=f"{' '.join(argv)} timed out after {timeout:.0f}s",
        )
    return RunOutcome(code=completed.returncode, stdout=completed.stdout, stderr=completed.stderr)


def collapse_yanked_notices(messages: str) -> tuple[list[str], int]:
    """Drop repeated yanked-check errors and count how many were removed."""
    kept: list[str] = []
    collapsed = 0
    for line in messages.splitlines():
        if YANKED_NOTICE_FRAGMENT in line:
            collapsed += 1
        else:
            kept.append(line)
    return kept, collapsed


def _emit(text: str) -> None:
    """Write a diagnostic line to stderr; stdout stays data-only."""
    print(text, file=sys.stderr)


def _report(outcome: RunOutcome, *, collapse_yanked: bool = False) -> int:
    """Echo a command's captured output, returning how many notices were collapsed."""
    if outcome.stdout:
        _emit(outcome.stdout.rstrip())
    if collapse_yanked:
        lines, collapsed = collapse_yanked_notices(outcome.stderr)
    else:
        lines, collapsed = [*outcome.stderr.splitlines()], 0
    for line in lines:
        if line.strip():
            _emit(line)
    return collapsed


def _fail(message: str) -> None:
    """Report an actionable environment failure."""
    _emit(f"audit: {message}")


def _offline_requested(argv: Sequence[str], environ: Mapping[str, str]) -> bool:
    """Decide whether the caller forbade network access."""
    if "--offline" in argv:
        return True
    return environ.get("CARGO_NET_OFFLINE", "").strip().lower() in _OFFLINE_TRUE_VALUES


def _missing_cache_message() -> str:
    """Explain how to obtain the missing advisory database."""
    return (
        "RustSec advisory database is not cached; run `pixi run audit-sync` on a "
        "connected host, or drop --offline to bootstrap it here"
    )


def _bootstrap(runner: Runner, cache: AdvisoryCache) -> int:
    """Seed the advisory clone once, then try the optional index cache."""
    outcome = runner(fetch_argv(), SYNC_TIMEOUT_SECONDS, None)
    _report(outcome)
    if outcome.code != EXIT_OK:
        _fail(f"cache bootstrap failed: {' '.join(fetch_argv())} exited {outcome.code}")
        return EXIT_ENVIRONMENT

    runner(fetch_index_argv(), INDEX_TIMEOUT_SECONDS, None)
    if not has_index_cache(cache):
        _fail("crates.io index cache unavailable; yanked checks stay degraded")
    return EXIT_OK


def _run_check(cache: AdvisoryCache, database: Path, runner: Runner) -> int:
    """Run both audit tools with network access denied."""
    skipped = 0
    for argv, collapse in ((deny_argv(offline=True), False), (audit_argv(database), True)):
        outcome = runner(argv, CHECK_TIMEOUT_SECONDS, None)
        if outcome.code != EXIT_OK:
            _report(outcome, collapse_yanked=collapse)
            _fail(f"{' '.join(argv)} exited {outcome.code}")
            return EXIT_FAILURE
        skipped += _report(outcome, collapse_yanked=collapse)

    if not has_index_cache(cache):
        _fail(
            f"yanked enforcement degraded ({skipped} check(s) skipped) — "
            "no local crates.io index cache; run `pixi run audit-sync` where network is available"
        )
    return EXIT_OK


def check(environ: Mapping[str, str], offline: bool, runner: Runner) -> int:
    """Run the offline-first audit gate."""
    cache = resolve_cache(cargo_home(environ))
    database = advisory_database(cache)

    if database is None:
        if offline:
            _fail(_missing_cache_message())
            return EXIT_ENVIRONMENT
        if _bootstrap(runner, cache) != EXIT_OK:
            return EXIT_ENVIRONMENT
        database = advisory_database(cache)
        if database is None:
            _fail(_missing_cache_message())
            return EXIT_ENVIRONMENT

    return _run_check(cache, database, runner)


def sync(environ: Mapping[str, str], runner: Runner) -> int:
    """Refresh the advisory database and index cache from the network."""
    return _bootstrap(runner, resolve_cache(cargo_home(environ)))


def main(
    argv: Sequence[str] | None = None,
    *,
    environ: Mapping[str, str] | None = None,
    runner: Runner | None = None,
) -> int:
    """Dispatch the audit gate from the command line."""
    arguments = list(sys.argv[1:] if argv is None else argv)
    environment = dict(os.environ) if environ is None else dict(environ)
    selected = run if runner is None else runner

    mode = next((argument for argument in arguments if not argument.startswith("-")), "check")
    if mode not in MODES:
        _fail(f"unknown mode {mode!r}; expected one of: {', '.join(MODES)}")
        return EXIT_ENVIRONMENT

    if mode == "sync":
        return sync(environment, selected)
    return check(environment, _offline_requested(arguments, environment), selected)


if __name__ == "__main__":
    raise SystemExit(main())
