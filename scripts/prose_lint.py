"""Lint the repository's prose with Vale.

`vale` is the linter; this module is the corpus, the invocation, and the exit code. All three
matter. Vale lints whatever paths it is handed, so the question "which documents are ours to
enforce" has no answer inside `.vale.ini`, and a gate that answers it by accident misses the
files a pull request adds.

The corpus is the documentation this repository publishes: the four documents at the root and
everything under `docs/`. Two trees are deliberately outside it.

- `backlog/` holds records rather than prose. A task file is a checklist that names commands,
  paths and other records in whatever case the command needs, and a terminology gate over 150
  historical records would rewrite history to satisfy a rule aimed at README prose.
  `decision-138` records the choice and what would reopen it.
- `.agents/` is vendored. Those skill files arrive from upstream with a content hash in
  `skills-lock.json`, so a finding there is a finding against somebody else's text, and
  fixing it would break the hash that proves the file is unmodified.

Findings are printed as `file:line:column: message (rule)` and, under GitHub Actions, also as
`::error` workflow annotations, so a failing pull request says what is wrong in the check
summary instead of only in a log nobody reads.
"""

import argparse
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

#: Documents at the repository root that are prose this repository writes.
ROOT_DOCUMENTS = ("README.md", "CONTRIBUTING.md", "AGENTS.md", "CHANGELOG.md")

#: Directories whose Markdown is prose this repository writes.
PROSE_DIRECTORIES = ("docs",)

#: Markdown suffix.
MARKDOWN = ".md"

#: Trees that are never linted, with the reason they are not.
EXCLUDED_PREFIXES = (
    # Records, not prose. See the module docstring and `decision-138`.
    "backlog/",
    # Vendored upstream skills, hashed in `skills-lock.json`.
    ".agents/",
    # Build output and dependency trees.
    "target/",
    "node_modules/",
    ".pixi/",
    "output/",
)

#: Exit code when Vale itself is missing, so the caller can tell "no linter" from "findings".
EXIT_NO_VALE = 2


@dataclass(frozen=True)
class Finding:
    """One Vale alert, flattened to what a reader needs."""

    file: str
    line: int
    column: int
    rule: str
    message: str

    def __str__(self) -> str:
        """Render the finding in `file:line:column: message (rule)` form."""
        return f"{self.file}:{self.line}:{self.column}: {self.message} ({self.rule})"

    def annotation(self) -> str:
        """Render the finding as a GitHub Actions error annotation."""
        return (
            f"::error file={self.file},line={self.line},col={self.column},"
            f"title=prose ({self.rule})::{self.message}"
        )


def discover(root: Path) -> list[Path]:
    """Return the documents the prose gate owns, relative to *root*, in stable order."""
    found: list[Path] = []
    for name in ROOT_DOCUMENTS:
        candidate = root / name
        if candidate.is_file():
            found.append(Path(name))
    for directory in PROSE_DIRECTORIES:
        base = root / directory
        if not base.is_dir():
            continue
        for path in sorted(base.rglob(f"*{MARKDOWN}")):
            relative = path.relative_to(root).as_posix()
            if any(relative.startswith(prefix) for prefix in EXCLUDED_PREFIXES):
                continue
            if path.is_file():
                found.append(Path(relative))
    return sorted(found, key=lambda path: path.as_posix())


def parse(report: str, root: Path) -> list[Finding]:
    """Turn Vale's JSON report into findings sorted by file, line and column."""
    try:
        payload = json.loads(report) if report.strip() else {}
    except json.JSONDecodeError as error:
        raise ValueError(f"vale did not print JSON: {error}") from error

    findings: list[Finding] = []
    for file, alerts in payload.items():
        relative = _relative(file, root)
        for alert in alerts:
            span = alert.get("Span") or [0, 0]
            findings.append(
                Finding(
                    file=relative,
                    line=int(span[0]),
                    column=int(span[1]),
                    rule=str(alert.get("Check", "")),
                    message=str(alert.get("Message", "")).strip(),
                )
            )
    return sorted(findings, key=lambda finding: (finding.file, finding.line, finding.column))


def _relative(file: str, root: Path) -> str:
    """Vale reports the path it was given; the gate prints it relative to the root."""
    path = Path(file)
    if path.is_absolute():
        try:
            return path.relative_to(root).as_posix()
        except ValueError:
            return path.as_posix()
    return path.as_posix()


def lint(root: Path, files: list[Path]) -> list[Finding] | None:
    """Run Vale over *files* and return its findings, or `None` when Vale could not run.

    `None` is distinct from an empty list: an empty list is a clean corpus, and a corpus that
    was never linted is not clean, it is unchecked.
    """
    arguments = ["vale", "--output=JSON", *[path.as_posix() for path in files]]
    try:
        completed = subprocess.run(arguments, cwd=root, capture_output=True, text=True, check=False)
    except FileNotFoundError:
        return None
    # Vale exits 1 when it reports alerts and 2 when it could not run at all; both are read
    # from the report rather than from the status, because the report says why.
    if completed.returncode not in (0, 1):
        print(
            f"prose-lint: vale exited {completed.returncode}: {completed.stderr.strip()}",
            file=sys.stderr,
        )
        return None
    try:
        return parse(completed.stdout, root)
    except ValueError as error:
        print(f"prose-lint: {error}", file=sys.stderr)
        return None


def main(argv: list[str] | None = None) -> int:
    """Run the prose gate from the command line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--paths",
        nargs="*",
        type=Path,
        default=None,
        help="lint exactly these files instead of the gate's corpus",
    )
    args = parser.parse_args(argv)

    files = discover(args.root) if args.paths is None else list(args.paths)
    if not files:
        print(
            "prose-lint: no Markdown document to lint; the gate would check nothing",
            file=sys.stderr,
        )
        return 1

    findings = lint(args.root, files)
    if findings is None:
        print(
            "prose-lint: vale is not available; install it with `pixi install --locked`",
            file=sys.stderr,
        )
        return EXIT_NO_VALE

    annotate = os.environ.get("GITHUB_ACTIONS") == "true"
    for finding in findings:
        print(f"prose-lint: {finding}", file=sys.stderr)
        if annotate:
            print(finding.annotation())
    if findings:
        print(
            f"prose-lint: {len(findings)} finding(s) in {len(files)} document(s)",
            file=sys.stderr,
        )
        return 1
    print(f"prose-lint: {len(files)} document(s) clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
