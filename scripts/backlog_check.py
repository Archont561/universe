"""Validate the Markdown Backlog store."""

import argparse
import re
import sys
from pathlib import Path
from urllib.parse import unquote

from scripts.lib.files import list_files, path_exists

DECISION_STATUSES = {"accepted", "proposed", "superseded", "deprecated"}
TASK_STATUSES = {"To Do", "In Progress", "Done"}
SPECIAL_FILES = {"README.md"}

type Frontmatter = dict[str, str]


def parse_frontmatter(text: str) -> Frontmatter | None:
    """Parse the scalar key/value subset used by Backlog frontmatter."""
    if not text.startswith("---\n"):
        return None

    end = text.find("\n---", 4)
    if end == -1:
        return None

    values: Frontmatter = {}
    for line in text[4:end].split("\n"):
        if line[:1].isspace():
            continue
        match = re.match(r"^([A-Za-z0-9_-]+):\s*(.*)$", line)
        if match is None:
            continue
        key, value = match.groups()
        values[key] = re.sub(r"^[\"']|[\"']$", "", value)
    return values


def extract_markdown_links(text: str) -> list[str]:
    """Extract inline Markdown link targets, ignoring optional titles."""
    pattern = re.compile(r'!?\[[^\]]*\]\(([^)\s]+)(?:\s+"[^"]*")?\)')
    return pattern.findall(text)


def _category_for(relative_path: Path) -> str:
    parts = set(relative_path.parts)
    for category in ("decisions", "tasks", "docs", "milestones"):
        if category in parts:
            return category
    return "other"


def _is_external_or_anchor(target: str) -> bool:
    return re.match(r"^(?:https?:|mailto:|tel:|#|/)", target) is not None


def validate_backlog(repo_root: Path | None = None) -> list[str]:
    """Validate IDs, statuses, filenames, frontmatter, and Backlog links."""
    root = Path.cwd() if repo_root is None else repo_root
    backlog_root = root / "backlog"
    errors: list[str] = []
    seen_ids: dict[str, str] = {}

    if not path_exists(backlog_root):
        return ["backlog/ directory is missing"]

    files = [path for path in list_files(backlog_root) if path.suffix == ".md"]
    for file in files:
        relative_path = file.relative_to(backlog_root)
        relative_posix = relative_path.as_posix()
        text = file.read_text(encoding="utf-8")

        if relative_posix not in SPECIAL_FILES:
            frontmatter = parse_frontmatter(text)
            if frontmatter is None:
                errors.append(f"{relative_posix}: missing frontmatter")
                continue

            item_id = frontmatter.get("id", "")
            title = frontmatter.get("title", "")
            if not item_id:
                errors.append(f"{relative_posix}: missing frontmatter id")
                continue
            if not title:
                errors.append(f"{relative_posix}: missing frontmatter title")

            normalized_id = item_id.lower()
            previous = seen_ids.get(normalized_id)
            if previous is not None:
                errors.append(f"{relative_posix}: duplicate id {item_id} also used by {previous}")
            else:
                seen_ids[normalized_id] = relative_posix

            if not file.name.lower().startswith(f"{normalized_id} -"):
                errors.append(f'{relative_posix}: filename must start with "{item_id} -"')

            category = _category_for(relative_path)
            status = frontmatter.get("status")
            if category == "decisions" and status not in DECISION_STATUSES:
                errors.append(f"{relative_posix}: invalid decision status {status}")
            if category == "tasks" and status not in TASK_STATUSES:
                errors.append(f"{relative_posix}: invalid task status {status}")

        for target in extract_markdown_links(text):
            if _is_external_or_anchor(target):
                continue
            clean = unquote(target.split("#", 1)[0].split("?", 1)[0])
            if not clean or clean.endswith("/"):
                continue
            resolved = (file.parent / clean).resolve()
            try:
                resolved.relative_to(backlog_root.resolve())
            except ValueError:
                continue
            if not path_exists(resolved):
                errors.append(f"{relative_posix}: broken Backlog link {target}")

    return errors


def main(argv: list[str] | None = None) -> int:
    """Run the Backlog checker from the command line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)

    errors = validate_backlog(args.root)
    if errors:
        for error in errors:
            print(f"backlog-check: {error}", file=sys.stderr)
        return 1
    print("backlog-check: Backlog is valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
