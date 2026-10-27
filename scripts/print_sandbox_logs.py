"""Print bounded pixi-sandbox diagnostics as GitHub Actions annotations."""

from __future__ import annotations

import argparse
from pathlib import Path


def _escape_workflow_command(value: str) -> str:
    """Escape untrusted text for the GitHub Actions workflow-command protocol."""
    return value.replace("%", "%25").replace("\r", "%0D").replace(":", "%3A").replace(",", "%2C")


def render_annotations(log_dir: Path, *, tail_lines: int = 80) -> list[str]:
    """Return annotations for the last ``tail_lines`` from each regular log file."""
    if not log_dir.is_dir():
        return ["::notice::No pixi-sandbox diagnostic directory was created."]

    logs = sorted(path for path in log_dir.iterdir() if path.is_file())
    if not logs:
        return ["::notice::No pixi-sandbox diagnostic files were created."]

    annotations: list[str] = []
    for log in logs:
        try:
            text = log.read_bytes().decode("utf-8", errors="replace")
            lines = text.split("\n")
            if lines and lines[-1] == "":
                lines.pop()
        except OSError as exc:
            message = _escape_workflow_command(f"{log}: {exc}")
            annotations.append(f"::error title=Pixi sandbox diagnostics::{message}")
            continue
        for line in lines[-tail_lines:]:
            annotations.append(
                f"::error title=Pixi sandbox diagnostics::{_escape_workflow_command(line)}"
            )
    return annotations


def main(argv: list[str] | None = None) -> int:
    """Run the bounded diagnostic renderer."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log_dir", type=Path, help="pixi-sandbox diagnostic directory")
    parser.add_argument("--tail-lines", type=int, default=80)
    args = parser.parse_args(argv)
    if args.tail_lines < 1:
        parser.error("--tail-lines must be positive")

    for annotation in render_annotations(args.log_dir, tail_lines=args.tail_lines):
        print(annotation)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
