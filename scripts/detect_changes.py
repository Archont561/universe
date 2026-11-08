"""Decide which CI lanes a change actually needs.

A pull request that touches only `backlog/` or a design document cannot break a Rust
gate, yet it used to pay the full compile. This module answers two questions from a diff
and nothing else, so the answer is a property of the paths rather than of a runner:

- ``code`` — does anything that the compile-based lanes can fail on change?
- ``web`` — does the Bun workspace change?

The classifier is a pure function of the path list; the command-line wrapper is the only
part that reads Git. Every decision is conservative in the same direction: an unknown path
is *code*, so a new top-level directory is checked rather than skipped. A lane that
silently skips is worse than a lane that runs too often.

Exit status is always zero for a usable answer. An unavailable comparison revision is not
a reason to guess — it exits non-zero, and the caller's jobs then run in full.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from collections.abc import Iterable, Sequence

#: Paths under these prefixes cannot affect a Rust, Python, or workflow gate.
PROSE_PREFIXES = ("backlog/", "docs/", "web/")

#: Prose at the repository root, by exact name.
PROSE_FILES = (
    "README.md",
    "CHANGELOG.md",
    "LICENSE",
    "AGENTS.md",
)

#: Paths that only the Bun workspace can fail on.
WEB_PREFIXES = ("web/",)


def is_prose(path: str) -> bool:
    """Return whether `path` is documentation that no compiled gate reads.

    `web/` is prose for the Rust lanes but code for the Bun lane, so the two questions are
    asked separately rather than folding "not Rust" into "needs nothing".
    """
    normalized = path.replace("\\", "/")
    if normalized in PROSE_FILES:
        return True
    return any(normalized.startswith(prefix) for prefix in PROSE_PREFIXES)


def code_changed(paths: Iterable[str]) -> bool:
    """Whether any changed path can affect a compile-based lane."""
    return any(not is_prose(path) for path in paths)


def web_changed(paths: Iterable[str]) -> bool:
    """Whether any changed path can affect the Bun workspace."""
    normalized = {path.replace("\\", "/") for path in paths}
    return any(
        path.startswith(WEB_PREFIXES) or path in {"bun.lock", "package.json", "bun.lockb"}
        for path in normalized
    )


def changed_paths(base: str, head: str, event: str, cwd: str | None = None) -> list[str] | None:
    """List the files a push or pull request changed, or `None` if that cannot be known.

    `cwd` exists so the decision can be asked of a fixture repository; the workflow always
    asks it of the checkout it is already in.
    """
    if not base or set(base) <= {"0"}:
        return None
    if event == "push":
        # The pushed range; `merge-base` would hide commits already on the branch.
        revision = f"{base}..{head}"
    else:
        # A pull request's files, measured from where the branch left the base.
        anchor = _merge_base(base, head, cwd)
        if anchor is None:
            return None
        revision = f"{anchor}..{head}"
    completed = subprocess.run(
        ["git", "diff", "--name-only", revision],
        capture_output=True,
        text=True,
        check=False,
        cwd=cwd,
    )
    if completed.returncode != 0:
        return None
    return [line for line in completed.stdout.splitlines() if line.strip()]


def _merge_base(base: str, head: str, cwd: str | None = None) -> str | None:
    """The commit where `head` left `base`, or `None` when they share no history yet."""
    completed = subprocess.run(
        ["git", "merge-base", base, head],
        capture_output=True,
        text=True,
        check=False,
        cwd=cwd,
    )
    if completed.returncode != 0:
        return None
    return completed.stdout.strip() or None


def main(argv: Sequence[str] | None = None) -> int:
    """Print the lane decisions and, in Actions, export them as step outputs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="")
    parser.add_argument("--head", default="")
    parser.add_argument("--event", default="")
    parser.add_argument(
        "--github-output",
        default=None,
        help="append `key=value` lines here (Actions passes the step's output file)",
    )
    args = parser.parse_args(argv)

    paths = changed_paths(args.base, args.head, args.event)
    if paths is None:
        # Fail closed for the *decision*, not for the run: the caller runs every lane. An
        # empty list would mean "nothing changed" and skip them all, which is the one answer
        # this module must never give when it does not know.
        print(
            "detect-changes: no usable comparison revision; every lane will run",
            file=sys.stderr,
        )
        code, web = "true", "true"
    else:
        code = "true" if code_changed(paths) else "false"
        web = "true" if web_changed(paths) else "false"
    described = "unknown" if paths is None else f"{len(paths)} file(s)"
    print(f"detect-changes: {described} changed; code={code} web={web}")
    for path in paths or []:
        print(f"  {path}")

    if args.github_output:
        with open(args.github_output, "a", encoding="utf-8") as handle:
            handle.write(f"code={code}\n")
            handle.write(f"web={web}\n")
    return 0


if __name__ == "__main__":  # pragma: no cover - exercised through `main`
    raise SystemExit(main())
