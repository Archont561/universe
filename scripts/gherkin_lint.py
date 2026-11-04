"""Lint every Gherkin feature file the repository owns.

`gherlint` is the linter; this module is the discovery and the exit code around it.
Both halves matter. A linter run over a directory that no longer holds a feature
file passes, and a gate that passes because it linted nothing is worse than no gate
at all, so discovery fails closed when it finds no files.

Feature files are found by the shape this repository gave them: a file named
``*.feature`` inside a directory named ``features``. That is the layout the Rust
crate tests use, and searching for the directory name rather than a fixed path
means a second crate's features are linted on the day they are added, not on the
day someone remembers to wire them in.
"""

import argparse
import os
import sys
from dataclasses import dataclass
from pathlib import Path

from gherlint.linter import GherkinLinter
from gherlint.objectmodel.nodes import Document, Node
from gherlint.reporting import Message, MessageStore, Reporter

FEATURE_DIRECTORY = "features"
FEATURE_SUFFIX = ".feature"

# Directories that never hold repository content: the sandbox payload, package
# caches, and build output. Walking them would make the gate slower than the build
# it guards, and a `.feature` file inside a dependency is not ours to fix.
SKIPPED_DIRECTORIES = frozenset(
    {
        ".git",
        ".pixi",
        ".pixi-sandbox",
        ".venv",
        "node_modules",
        "output",
        "target",
    }
)


@dataclass(frozen=True)
class Finding:
    """One lint message, flattened to what a reader needs."""

    file: str
    line: int
    column: int
    text: str
    name: str

    def __str__(self) -> str:
        """Render the finding in `file:line:column: message (name)` form."""
        return f"{self.file}:{self.line}:{self.column}: {self.text} ({self.name})"


class CollectingReporter(Reporter):
    """Reporter that records messages instead of printing them mid-walk.

    The stock text reporter prints as it goes, which interleaves findings from
    different files and leaves the caller nothing to count. Collecting them keeps
    the output sorted and makes the exit code a decision rather than a guess.
    """

    def __init__(self) -> None:
        super().__init__()
        self.findings: list[Finding] = []

    def emit(self, message: Message, node: Node, **format_args: object) -> None:
        """Record one message at the node it was raised for."""
        root = node.get_root()
        if not isinstance(root, Document):
            raise RuntimeError("a gherlint message was raised outside a document")
        text = message.text.format(**format_args) if format_args else message.text
        self.findings.append(
            Finding(
                file=str(root.filename),
                line=node.line,
                column=node.column,
                text=text,
                name=message.name,
            )
        )


def discover(root: Path) -> list[Path]:
    """Return every ``features/*.feature`` file below *root*, in stable order."""
    found: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(name for name in dirnames if name not in SKIPPED_DIRECTORIES)
        if Path(dirpath).name != FEATURE_DIRECTORY:
            continue
        for filename in sorted(filenames):
            if filename.endswith(FEATURE_SUFFIX):
                found.append(Path(dirpath) / filename)
    return sorted(found, key=lambda path: path.relative_to(root).as_posix())


def lint_file(path: Path) -> list[Finding]:
    """Lint one feature file and return its findings."""
    reporter = CollectingReporter()
    # `gherlint` keeps its message registry in a class-level store and registers every
    # message again when a linter is built, so a second linter in one process raises
    # `DuplicateMessageError`. Its CLI builds exactly one per process; a repository gate
    # over many files does not, so the store is cleared first and repopulated by the
    # constructor. Nothing else lives in that store between files.
    MessageStore.clear()
    linter = GherkinLinter(path)
    # The linter hard-codes the printing text reporter, and prints as it walks. Findings
    # are collected instead, so they can be sorted and counted before anything is printed.
    linter.reporter = reporter
    for checker in linter.checkers:
        checker.reporter = reporter
    linter.lint_file(path)
    # A parse error is reported once for the document and again for the first node it could
    # not read: one broken file, two messages, differing only in which node they blame. The
    # first is the file-level statement, so the rest are dropped rather than counted.
    unique: dict[tuple[str, str], Finding] = {}
    for finding in reporter.findings:
        unique.setdefault((finding.name, finding.text), finding)
    return list(unique.values())


def lint(root: Path) -> tuple[list[Path], list[Finding]]:
    """Lint every feature file below *root*.

    Returns the files that were linted and the findings across all of them.
    """
    files = discover(root)
    findings: list[Finding] = []
    for file in files:
        findings.extend(lint_file(file))
    return files, findings


def main(argv: list[str] | None = None) -> int:
    """Run the feature linter from the command line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)

    files, findings = lint(args.root)
    if not files:
        print(
            f"gherkin-lint: no *{FEATURE_SUFFIX} file under any "
            f"{FEATURE_DIRECTORY}/ directory; the gate would lint nothing",
            file=sys.stderr,
        )
        return 1
    for finding in findings:
        print(f"gherkin-lint: {finding}", file=sys.stderr)
    if findings:
        print(
            f"gherkin-lint: {len(findings)} finding(s) in {len(files)} feature file(s)",
            file=sys.stderr,
        )
        return 1
    print(f"gherkin-lint: {len(files)} feature file(s) clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
