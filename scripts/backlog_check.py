"""Validate the Markdown Backlog store.

File naming shape and relative-link resolution are owned by `.alint.yml`
(decision-121): alint also resolves links that leave `backlog/`, which this
checker skipped. What stays here is the identity and cross-record work a
declarative rule cannot express — filename-to-frontmatter-id equality, duplicate
ids, status enums (RE2 has no look-ahead), and milestone/dependency resolution.
"""

import argparse
import re
import sys
from pathlib import Path

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


def extract_frontmatter_list(text: str, key: str) -> list[str]:
    """Extract a simple YAML scalar list from the frontmatter block."""
    match = re.search(rf"(?m)^{re.escape(key)}:[ \t]*(.*)$", text)
    if match is None:
        return []

    value = match.group(1).strip()
    if value.startswith("[") and value.endswith("]"):
        return [item.strip().strip("\"'") for item in value[1:-1].split(",") if item.strip()]
    if value:
        return [value.strip("\"'")]

    items: list[str] = []
    for line in text[match.end() :].splitlines():
        if not line.strip():
            continue
        if not line[:1].isspace():
            break
        item = re.match(r"^\s+-\s*(.+?)\s*$", line)
        if item is not None:
            items.append(item.group(1).strip("\"'"))
    return items


def _category_for(relative_path: Path) -> str:
    parts = set(relative_path.parts)
    for category in ("decisions", "tasks", "docs", "milestones"):
        if category in parts:
            return category
    return "other"


def validate_backlog(repo_root: Path | None = None) -> list[str]:
    """Validate Backlog identity, statuses, and cross-record references."""
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

    categories_by_id: dict[str, str] = {}
    for file in files:
        frontmatter = parse_frontmatter(file.read_text(encoding="utf-8"))
        if frontmatter is not None and frontmatter.get("id"):
            categories_by_id[frontmatter["id"].lower()] = _category_for(
                file.relative_to(backlog_root)
            )

    for file in files:
        relative_posix = file.relative_to(backlog_root).as_posix()
        if _category_for(file.relative_to(backlog_root)) != "tasks":
            continue
        text = file.read_text(encoding="utf-8")
        frontmatter = parse_frontmatter(text)
        if frontmatter is None:
            continue

        milestone = frontmatter.get("milestone")
        if milestone and categories_by_id.get(milestone.lower()) != "milestones":
            errors.append(f"{relative_posix}: unknown milestone reference {milestone}")
        for dependency in extract_frontmatter_list(text, "dependencies"):
            if categories_by_id.get(dependency.lower()) != "tasks":
                errors.append(f"{relative_posix}: unknown task dependency {dependency}")

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
