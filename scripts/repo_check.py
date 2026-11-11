"""Validate the invariants a declarative config cannot express.

Repository shape — required files, forbidden paths and basenames, naming, and
immutable CI refs — is owned by `.alint.yml` (decision-121, `pixi run
alint-check`). What stays here is the rest: a legacy-token scan that must not
match its own configuration, version inheritance through a Cargo workspace,
recipe and release-version parity, and set/graph reasoning over the offline
transport and the audit gate.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tomllib
from collections.abc import Sequence
from pathlib import Path
from typing import Any

from scripts.lib.files import path_exists

SKIP_DIRS = {
    ".git",
    ".mypy_cache",
    ".pixi",
    ".pixi-sandbox",
    ".pytest_cache",
    ".ruff_cache",
    "__pycache__",
    "node_modules",
    "target",
}

# Constructed from fragments so the checker does not itself contain the
# forbidden standalone-repository references it scans for.
LEGACY_TOKENS = (
    "conda" + "-universe",
    "conda" + "_universe",
    "air" + "pack",
)


def validate_offline_cargo_transport(
    cargo_lock: str,
    sandbox_config: str,
    publisher_workflow: str,
    lefthook_config: str,
    pixi_config: str,
) -> list[str]:
    """Ensure registry dependencies are vendored and the push gate is offline."""
    registry_source = re.search(r'(?m)^source\s*=\s*"registry\+', cargo_lock)
    if registry_source is None:
        return []

    errors: list[str] = []
    try:
        config = tomllib.loads(sandbox_config)
    except tomllib.TOMLDecodeError as exc:
        return [f"cannot parse sandbox config for Cargo vendoring: {exc}"]

    bundles = config.get("bundle", [])
    vendor_enabled = (
        all(
            bundle.get("cargo_vendor", config.get("cargo_vendor", False)) is True
            for bundle in bundles
        )
        if bundles
        else config.get("cargo_vendor") is True
    )
    if not vendor_enabled:
        errors.append("Cargo registry dependencies require cargo_vendor=true in the sandbox config")

    vendor_arg = (
        r'(?m)^\s*--cargo-vendor\s+(?:true|"' + re.escape("${{ matrix.cargo_vendor }}") + r'")\s*$'
    )
    if not re.search(vendor_arg, publisher_workflow):
        errors.append("sandbox publisher must include Cargo vendor data")
    if not re.search(
        r"(?m)^\s*run:\s*CARGO_NET_OFFLINE=true\s+pixi run --offline --as-is offline-gates\s*$",
        lefthook_config,
    ):
        errors.append("pre-push must run offline-gates with CARGO_NET_OFFLINE=true")

    try:
        pixi_tasks = tomllib.loads(pixi_config).get("tasks", {})
    except tomllib.TOMLDecodeError as exc:
        return [*errors, f"cannot parse Pixi tasks for offline gate: {exc}"]
    offline_gate = pixi_tasks.get("offline-gates", {})
    required_dependencies = {"check", "coverage", "lint-actions", "build"}
    dependencies = set(offline_gate.get("depends-on", []))
    if dependencies != required_dependencies:
        errors.append(
            "Pixi offline-gates dependencies must be check, coverage, lint-actions, and build"
        )

    # A transport that is not republished when its inputs change silently rots:
    # a stale branch keeps restoring an environment that no longer matches the
    # lockfiles. Require every input that changes the packed payload.
    required_push_paths = {
        ".pixi-sandbox.toml",
        "Cargo.lock",
        "Cargo.toml",
        "crates/**/Cargo.toml",
        "pixi.lock",
        "pixi.toml",
    }
    workflow_table = config.get("workflow", {})
    declared_push_paths = (
        set(workflow_table.get("push_paths", [])) if isinstance(workflow_table, dict) else set()
    )
    missing_push_paths = sorted(required_push_paths - declared_push_paths)
    if missing_push_paths:
        errors.append(
            "sandbox config push_paths must include every republish trigger; missing: "
            + ", ".join(missing_push_paths)
        )

    network_tasks = {"audit", "ci", "gates", "web-build", "web-check", "web-install"}
    pending = list(dependencies)
    visited: set[str] = set()
    while pending:
        task_name = pending.pop()
        if task_name in visited:
            continue
        visited.add(task_name)
        if task_name in network_tasks:
            errors.append(f"Pixi offline-gates transitively includes online task: {task_name}")
        task = pixi_tasks.get(task_name, {})
        for dependency in task.get("depends-on", []):
            if isinstance(dependency, str):
                pending.append(dependency)
            elif isinstance(dependency, dict) and isinstance(dependency.get("task_name"), str):
                pending.append(dependency["task_name"])
    return errors


PUSH_ENTRY = re.compile(r'^\s+- "(?P<path>[^"]+)"\s*$')


def declared_workflow_push_paths(workflow: str) -> list[str] | None:
    """Extract ``on.push.paths`` from a workflow file without a YAML parser.

    The repository has no YAML dependency and this is one block of one file, so
    the reader is deliberately narrow: it looks for a two-space ``push:`` key, then
    a four-space ``paths:`` beneath it, then the ``- "..."`` entries at deeper
    indentation. A quoting style other than plain double quotes is out of scope and
    shows up as a missing entry rather than as a silent pass.

    Returns ``None`` when the workflow has no push trigger at all, and an empty list
    when it has one without a ``paths`` filter — those are different facts, and only
    the first is a stale transport.
    """
    lines = workflow.splitlines()
    for index, line in enumerate(lines):
        if line.rstrip() != "  push:":
            continue
        for inner, candidate in enumerate(lines[index + 1 :], start=index + 1):
            stripped = candidate.strip()
            if candidate and not candidate.startswith("    "):
                break
            if stripped == "paths:":
                return _push_entries(lines[inner + 1 :])
        return []
    return None


def _push_entries(lines: list[str]) -> list[str]:
    """The quoted entries of a ``paths:`` block, stopping at the next key."""
    entries: list[str] = []
    for line in lines:
        match = PUSH_ENTRY.match(line)
        if match is not None:
            entries.append(match.group("path"))
            continue
        if line.strip() and not line.strip().startswith("-"):
            break
    return entries


def validate_workflow_push_paths(sandbox_config: str, workflow: str) -> list[str]:
    """Require the publish workflow to trigger on exactly the declared republish inputs.

    ``[workflow] push_paths`` in the sandbox config is the policy; the workflow's
    ``on.push.paths`` is a copy of it. When the copy is short, a change to a trigger
    that really does change the packed payload repacks the environment and never
    publishes it, and nothing fails — the branch simply serves an older tree. That is
    the staleness this check exists to make loud, in both directions: an entry the
    config declares and the workflow omits, and an entry the workflow carries that the
    config does not own.
    """
    try:
        workflow_table = tomllib.loads(sandbox_config).get("workflow", {})
    except tomllib.TOMLDecodeError as exc:
        return [f"cannot parse the sandbox config's workflow table: {exc}"]
    declared = (
        list(workflow_table.get("push_paths", [])) if isinstance(workflow_table, dict) else []
    )
    if not declared:
        return []

    present = declared_workflow_push_paths(workflow)
    if present is None:
        return [
            "the publish workflow has no push trigger, so the declared push_paths "
            "never fire; add an on.push.paths block"
        ]

    errors = [
        f"publish workflow is missing declared republish trigger {path!r}; "
        "the transport will not republish when it changes"
        for path in declared
        if path not in present
    ]
    errors.extend(
        f"publish workflow triggers on {path!r}, which the sandbox config does not declare"
        for path in present
        if path not in declared
    )
    return errors


AUDIT_TASK_COMMAND = "python -m scripts.audit check"

#: A step that runs the audit task, in any of the forms this repository writes it in.
SCHEDULED_AUDIT_COMMAND = re.compile(r"pixi\s+run\s+(?:--frozen\s+|--locked\s+)?audit\b")


def validate_audit_gate(pixi_config: str) -> list[str]:
    """Ensure the audit task runs the offline-first audit runner.

    The task is also required to stay *out* of `gates`. Its input changes without a commit,
    so as a per-diff gate it reds a pull request for an advisory its diff did not introduce;
    it runs on a schedule instead (decision-134), and this is the invariant that keeps a
    later "put it back in gates, it is only a second" from silently reintroducing that.
    """
    try:
        tasks = tomllib.loads(pixi_config).get("tasks", {})
    except tomllib.TOMLDecodeError as exc:
        return [f"cannot parse Pixi tasks for the audit gate: {exc}"]

    errors: list[str] = []
    audit = tasks.get("audit", {})
    command = audit.get("cmd") if isinstance(audit, dict) else None
    if not isinstance(command, str) or AUDIT_TASK_COMMAND not in command:
        errors.append(
            f"the audit task must run '{AUDIT_TASK_COMMAND}' so the gate stays offline-first"
        )
    if "audit-sync" not in tasks:
        errors.append("the audit gate needs an audit-sync task to seed the advisory cache")
    gates = tasks.get("gates", {})
    depends_on = gates.get("depends-on") if isinstance(gates, dict) else None
    if isinstance(depends_on, list) and "audit" in depends_on:
        errors.append(
            "`gates` must not depend on `audit`: advisories arrive without a commit, so the "
            "audit runs on a schedule (.github/workflows/security.yml) rather than per diff"
        )
    return errors


#: Tasks that must run before anything that compiles, because they report in seconds.
CHEAP_TASKS = (
    "fmt-check",
    "backlog-check",
    "repo-check",
    "skills-check",
    "alint-check",
    "crate-check",
    "lint-gherkin",
    "lint-prose",
)


def validate_gate_order(pixi_config: str) -> list[str]:
    """Ensure the cheap checks run before the ones that need a compile.

    `check` is a serial chain, so its order *is* its feedback loop: measured on 2026-11-08,
    `lint` cost 108s and `test` 100s, while the five repository checks together cost under
    1.5s. With them last, a typo in a Backlog record took four minutes to report. This keeps
    the cheap set first so a later addition cannot quietly push it behind a build.
    """
    try:
        tasks = tomllib.loads(pixi_config).get("tasks", {})
    except tomllib.TOMLDecodeError as exc:
        return [f"cannot parse Pixi tasks for gate order: {exc}"]

    errors: list[str] = []
    check = tasks.get("check", {})
    depends_on = check.get("depends-on") if isinstance(check, dict) else None
    if not isinstance(depends_on, list) or "check-fast" not in depends_on:
        return ["`check` must depend on `check-fast`, which holds the checks that need no compile"]
    if depends_on[0] != "check-fast":
        errors.append(
            "`check-fast` must be the first dependency of `check`: the tasks that compile "
            "would otherwise report before a formatting error does"
        )

    fast = tasks.get("check-fast", {})
    fast_dependencies = fast.get("depends-on") if isinstance(fast, dict) else None
    missing = sorted(set(CHEAP_TASKS) - set(fast_dependencies or []))
    if missing:
        errors.append(f"`check-fast` is missing the checks that need no compile: {missing}")
    return errors


def validate_session_bootstrap(pixi_config: str) -> list[str]:
    """Ensure the documented start of a session really installs the hooks.

    `AGENTS.md` rule 14 tells an agent to run `pixi run setup` before it reads anything else,
    and the whole value of that instruction is the hooks it wires. So `setup` has to reach
    `hooks-install`, and `hooks-install` has to be `lefthook install`: a rename on either side
    leaves a command that exits zero and installs nothing, which is the failure mode that
    looks like success.
    """
    try:
        tasks = tomllib.loads(pixi_config).get("tasks", {})
    except tomllib.TOMLDecodeError as exc:
        return [f"cannot parse Pixi tasks for the session bootstrap: {exc}"]

    errors: list[str] = []
    hooks_install = tasks.get("hooks-install", {})
    command = hooks_install.get("cmd") if isinstance(hooks_install, dict) else None
    if not isinstance(command, str) or "lefthook install" not in command:
        errors.append(
            "`hooks-install` must run `lefthook install`; otherwise `pixi run setup` reports "
            "success and installs no hooks"
        )

    setup = tasks.get("setup", {})
    dependencies = setup.get("depends-on") if isinstance(setup, dict) else None
    if not isinstance(dependencies, list) or "hooks-install" not in dependencies:
        errors.append(
            "`setup` must depend on `hooks-install`: the first command of a session is where "
            "the hooks have to be installed"
        )
    return errors


def validate_required_gate_name(ci_workflow: str) -> list[str]:
    """Ensure CI still reports a job called `gates`.

    Branch protection requires a check by that name. Splitting the job for a faster loop is
    allowed; leaving nothing behind that answers to the name is not, because the pull request
    then waits forever on a check that will never report.
    """
    # A job block starts at two-space indentation under `jobs:`; the name is the key.
    if not any(
        line.startswith("  gates:") or line.startswith("  gates: ")
        for line in ci_workflow.splitlines()
    ):
        return [
            "ci.yml declares no `gates` job; the required check must keep its name even when "
            "the work is split across lanes"
        ]
    return []


def validate_scheduled_audit(repo_root: Path) -> list[str]:
    """Ensure some scheduled workflow runs the audit task.

    With the audit out of `gates`, nothing per-commit would notice if its workflow were
    deleted or its schedule removed: the repository would be green forever while never
    auditing. A scheduled workflow that names no audit step, or an audit step in a workflow
    that never runs on a clock, is the same silence, so both are refused.
    """
    workflow_dir = repo_root / ".github" / "workflows"
    if not path_exists(workflow_dir):
        return ["no .github/workflows directory, so the scheduled audit cannot exist"]

    errors: list[str] = []
    scheduled = 0
    for workflow in sorted(workflow_dir.glob("*.yml")):
        try:
            text = workflow.read_text(encoding="utf-8")
        except OSError as exc:
            errors.append(f"cannot read {workflow.name}: {exc}")
            continue
        has_cron = "cron:" in text and "schedule:" in text
        runs_audit = SCHEDULED_AUDIT_COMMAND.search(text) is not None
        if runs_audit and not has_cron:
            errors.append(
                f"{workflow.name} runs the audit but never on a schedule; the audit is not "
                "part of `gates`, so a workflow that only runs on push audits nothing new"
            )
        if has_cron and runs_audit:
            scheduled += 1

    if scheduled == 0:
        errors.append(
            "no workflow runs the audit on a schedule: the audit is not in `gates`, so it "
            "would never run (expected a `schedule:` trigger with a `pixi run audit` step)"
        )
    return errors


#: Where the release notes for each version live. `UNI-59` AC#3 needs one file per release.
RELEASE_NOTES_DIR = "docs/releases"

#: Facts `UNI-59` AC#3 requires the notes to state, and the verbs the tag actually ships.
REQUIRED_RELEASE_FACTS = ("pre-release", "linux-64", "version", "verify", "restore")

#: The six numbered v1 surfaces of `decision-119`, as short keys. The test that reads them
#: back out of that decision is what keeps this transcription honest.
V1_SURFACE_KEYS = ("conda", "cargo", "python", "npm", "airlock", "portal")


def tag_annotation(text: str) -> str | None:
    """The first prose line of the release notes, which AC#2 reuses to annotate the tag.

    A heading, a frontmatter fence or a blank line is not a message `git tag -a -m` can
    carry. The line is returned verbatim; `validate_release_notes` decides whether its shape
    is one a tag may hold.
    """
    for line in text.splitlines():
        if line.strip():
            return line.strip()
    return None


def release_tag_subject(root: Path, version: str) -> str | None:
    """The subject of an annotated `v<version>` tag, or `None` before one exists."""
    if not path_exists(root / ".git"):
        return None
    completed = subprocess.run(
        ["git", "for-each-ref", "--format=%(contents:subject)", f"refs/tags/v{version}"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(completed.stderr.strip() or "git for-each-ref failed")
    return completed.stdout.strip() or None


def _markdown_section(text: str, heading: str) -> str | None:
    """The body of one `##` section, stopping before the next heading."""
    marker = f"## {heading}"
    start = text.find(marker)
    if start < 0:
        return None
    rest = text[start + len(marker) :]
    boundary = rest.find("\n## ")
    return rest if boundary < 0 else rest[:boundary]


def _workspace_version(root: Path) -> str | None:
    """The one version literal `decision-126` puts in the root manifest."""
    try:
        manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
        version: object = manifest["workspace"]["package"]["version"]
    except (OSError, KeyError, TypeError, tomllib.TOMLDecodeError):
        return None
    return version if isinstance(version, str) else None


def validate_release_notes(root: Path, tag_subject: str | None = None) -> list[str]:
    """`UNI-59` AC#3, checked the way AC#1 checks version parity.

    The notes for the workspace version must exist, open with a line that can be handed to
    `git tag -a -m` unchanged, state the pre-release scope and the three verbs the tag ships,
    and disclaim all six v1 surfaces of `decision-119` in a section of their own. A checkout
    without a changelog or a workspace manifest is not a release-lane repository. The tag
    subject is injectable so the AC#2 comparison needs no Git repository in a test.
    """
    changelog = root / "CHANGELOG.md"
    manifest = root / "Cargo.toml"
    if not path_exists(changelog) or not path_exists(manifest):
        return []
    version = _workspace_version(root)
    if version is None:
        return ["cannot read workspace version for the release notes"]

    notes = root / RELEASE_NOTES_DIR / f"{version}.md"
    relative = notes.relative_to(root).as_posix()
    if not path_exists(notes):
        return [f"release notes are missing: {relative}"]
    try:
        text = notes.read_text(encoding="utf-8")
    except OSError as exc:
        return [f"cannot read release notes {relative}: {exc}"]

    errors: list[str] = []
    first = tag_annotation(text)
    if first is None:
        return [f"release notes {relative} are empty; AC#2 needs a line to tag with"]
    if first.startswith(("#", "---", ">", "-", "`")) or "`" in first:
        errors.append(
            f"release notes {relative} must open with a plain tag annotation, not Markdown"
        )
    if len(first) > 140:
        errors.append(f"release notes {relative} open with more than 140 characters")
    if version not in first:
        errors.append(f"tag annotation does not name {version}: {first}")
    if not first.endswith("."):
        errors.append("tag annotation must end with a period")

    body = text.lower()
    for fact in REQUIRED_RELEASE_FACTS:
        if fact not in body:
            errors.append(f"release notes do not state {fact}")

    section = _markdown_section(text, "Not a v1 release")
    if section is None:
        errors.append("release notes have no 'Not a v1 release' section disclaiming decision-119")
    else:
        missing = [key for key in V1_SURFACE_KEYS if key not in section.lower()]
        if missing:
            errors.append("release notes do not disclaim the v1 surfaces: " + ", ".join(missing))

    if tag_subject is not None and first != tag_subject.strip():
        errors.append(
            f'tag annotation drift: v{version} tag subject is "{tag_subject.strip()}" '
            f'but the notes open with "{first}"'
        )
    return errors


def changelog_version(text: str) -> str | None:
    """The first released `##` heading, exactly as the release must spell its version.

    `## Unreleased` starts with a letter and is skipped. The heading itself is returned rather
    than a version parsed out of it: `universe lint` requires the heading to equal the
    workspace version, so `## 0.1.0 (pre-release)` is a drift finding, not the version 0.1.0.
    Local gates do not run that verb, and this keeps `repo-check`'s verdict the same as the
    rendered validate job's.
    """
    for line in text.splitlines():
        if not line.startswith("## "):
            continue
        heading = line[3:].strip().removeprefix("[").removesuffix("]")
        if heading and heading[0].isascii() and heading[0].isdigit():
            return heading
    return None


def newest_release_tag(tags: Sequence[str]) -> str | None:
    """The highest `vX.Y.Z` tag, or `None` before the first release is tagged.

    Git prints tags in its own order, so "the tag" means the highest version rather than the
    last line. Tags outside the release pattern are ignored: an unrelated `v*` ref must not
    decide what the changelog is compared against.
    """
    versions = [
        match.group(1)
        for tag in tags
        if (match := re.fullmatch(r"v(\d+\.\d+\.\d+)", tag.strip())) is not None
    ]
    if not versions:
        return None
    return max(versions, key=lambda version: tuple(int(part) for part in version.split(".")))


def release_tags(root: Path) -> list[str]:
    """The `v*` tags of a checkout. A tree that is not a Git checkout has no tags."""
    if not path_exists(root / ".git"):
        return []
    completed = subprocess.run(
        ["git", "tag", "--list", "v*"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        # Failing closed matters here: a checker that silently saw no tags would let a tag
        # disagree with the changelog and still report a clean repository.
        raise RuntimeError(completed.stderr.strip() or "git tag --list failed")
    return [line.strip() for line in completed.stdout.splitlines() if line.strip()]


def validate_release_version(root: Path, tags: Sequence[str] = ()) -> list[str]:
    """The changelog, workspace version and recipe version agree on one release.

    `decision-126` puts version parity in a repository check rather than a release-time
    computation, and `UNI-59` #1 names the three sources. A checkout without a changelog or
    a workspace manifest is not a release-lane repository, so there is nothing to compare.
    """
    errors: list[str] = []
    changelog = root / "CHANGELOG.md"
    manifest = root / "Cargo.toml"
    if not path_exists(changelog) or not path_exists(manifest):
        return errors
    try:
        workspace = tomllib.loads(manifest.read_text(encoding="utf-8"))["workspace"]["package"][
            "version"
        ]
        text = changelog.read_text(encoding="utf-8")
    except (OSError, KeyError, tomllib.TOMLDecodeError) as exc:
        return [f"cannot read release version inputs: {exc}"]

    top = changelog_version(text)
    if top is None:
        return [f"CHANGELOG.md has no versioned section; the workspace declares {workspace}"]
    if top != workspace:
        errors.append(
            f"changelog/workspace version drift: CHANGELOG.md={top}, Cargo.toml={workspace}"
        )

    recipe = root / "crates" / "universe-airlock" / "recipe" / "recipe.yaml"
    if path_exists(recipe):
        try:
            recipe_version = extract_recipe_context_version(recipe.read_text(encoding="utf-8"))
        except OSError as exc:
            return [f"cannot read the Airlock recipe version: {exc}"]
        if recipe_version != top:
            errors.append(
                f"changelog/recipe version drift: CHANGELOG.md={top}, "
                f"recipe={recipe_version or '<missing>'}"
            )

    latest = newest_release_tag(tags)
    if latest is not None and latest != top:
        errors.append(f"changelog/tag version drift: CHANGELOG.md={top}, newest v* tag={latest}")
    return errors


def extract_recipe_context_version(text: str) -> str | None:
    """Extract a Rattler recipe's ``context.version`` literal."""
    match = re.search(r"(?ms)^context:\s*\n\s+version:\s*['\"]?([^'\"\s]+)", text)
    return None if match is None else match.group(1)


def extract_recipe_package_name(text: str) -> str | None:
    """Extract a Rattler recipe package name."""
    match = re.search(r"(?ms)^package:\s*\n\s+name:\s*([^\s#]+)", text)
    return None if match is None else match.group(1)


ROADMAP_DOCUMENTS = (
    "backlog/docs/roadmap/doc-4 - Milestone-roadmap-and-skills-sh-rollout.md",
    "backlog/docs/roadmap/doc-112 - Ordered-milestone-DAG-and-sprint-train.md",
)

# Documentation a reader is meant to follow today. Backlog records are excluded on
# purpose: a task may name the Pixi task it intends to add, and that is a promise
# rather than a command someone will run this afternoon.
DOCUMENTED_TASK_DOCUMENTS = (
    "README.md",
    "CONTRIBUTING.md",
    "AGENTS.md",
)
DOCUMENTED_TASK_GLOBS = ("docs/*.md",)

# `pixi run` accepts either a declared task or an executable, and the two are
# indistinguishable by shape. The second form is enumerated so that a renamed or
# never-written task is an error instead of silently reading as a binary name.
PIXI_RUN_EXECUTABLES = frozenset(
    {
        "actionlint",
        "alint",
        "bash",
        "bun",
        "cargo",
        "convco",
        "deno",
        "gh",
        "git",
        "mypy",
        "node",
        "npm",
        "pixi",
        "pytest",
        "python",
        "python3",
        "rattler-build",
        "ruff",
        "sh",
        "taplo",
        "uv",
    }
)

# Flags whose value is a separate token. Every other flag stands alone.
PIXI_VALUE_FLAGS = frozenset({"-e", "--color", "--environment", "--manifest-path"})
PIXI_TASK_NAME = re.compile(r"^[a-z][a-z0-9-]*$")


def extract_task_ids(repo_root: Path) -> set[str]:
    """Collect the task identifiers declared by the Backlog task records."""
    identifiers: set[str] = set()
    for path in sorted((repo_root / "backlog" / "tasks").glob("uni-*.md")):
        try:
            text = path.read_text(encoding="utf-8")
        except OSError:
            continue
        match = re.search(r"(?m)^id:\s*(UNI-\d+)\s*$", text)
        if match is not None:
            identifiers.add(match.group(1))
    return identifiers


def validate_roadmap_coverage(repo_root: Path) -> list[str]:
    """Ensure the roadmap documents describe exactly the tasks that exist.

    A roadmap that quietly stops mentioning a task is worse than no roadmap: it is the
    document a reader trusts when asking what is planned, and its DAG is where blocker edges
    are explained. Both directions matter, so a phantom identifier is also an error.
    """
    errors: list[str] = []
    tasks = extract_task_ids(repo_root)
    if not tasks:
        return errors

    for document in ROADMAP_DOCUMENTS:
        path = repo_root / document
        if not path_exists(path):
            continue
        referenced = set(re.findall(r"UNI-\d+", path.read_text(encoding="utf-8")))
        errors.extend(
            f"{path.name}: task {task} is missing from the roadmap"
            for task in sorted(tasks - referenced, key=_task_number)
        )
        errors.extend(
            f"{path.name}: {phantom} is referenced but has no task record"
            for phantom in sorted(referenced - tasks, key=_task_number)
        )
    return errors


def _documented_pixi_commands(text: str) -> list[str]:
    """Return the command each `pixi run` on a line invokes.

    Only the same line is inspected: a wrapped sentence or a fenced block ends the
    command, and reading past it would invent invocations that were never written.
    """
    commands: list[str] = []
    for match in re.finditer(r"pixi run\b", text):
        tokens = text[match.end() :].split("\n", 1)[0].split()
        index = 0
        while index < len(tokens):
            token = tokens[index].strip("`\"'(),;:")
            if not token or token.startswith("#"):
                break
            if token.startswith("-"):
                index += 1 + (token in PIXI_VALUE_FLAGS)
                continue
            commands.append(token)
            break
    return commands


def validate_documented_pixi_tasks(repo_root: Path) -> list[str]:
    """Ensure every `pixi run` a reader is told to use names something that exists.

    A documented command is a promise about this checkout. When it names a task that
    was renamed or never written, the failure surfaces in a terminal minutes after a
    clone rather than in review, so the documentation is checked like code.
    """
    config_path = repo_root / "pixi.toml"
    if not path_exists(config_path):
        return []
    try:
        tasks = tomllib.loads(config_path.read_text(encoding="utf-8")).get("tasks", {})
    except (OSError, tomllib.TOMLDecodeError) as exc:
        return [f"cannot read the Pixi task table: {exc}"]
    declared = set(tasks)

    documents = [repo_root / name for name in DOCUMENTED_TASK_DOCUMENTS]
    for pattern in DOCUMENTED_TASK_GLOBS:
        documents.extend(sorted(repo_root.glob(pattern)))

    errors: list[str] = []
    for document in documents:
        if not path_exists(document):
            continue
        try:
            text = document.read_text(encoding="utf-8")
        except OSError as exc:
            errors.append(f"cannot read {document.name}: {exc}")
            continue
        relative = document.relative_to(repo_root).as_posix()
        for command in _documented_pixi_commands(text):
            if not PIXI_TASK_NAME.match(command):
                continue
            if command in declared or command in PIXI_RUN_EXECUTABLES:
                continue
            errors.append(
                f"{relative}: `pixi run {command}` names no declared Pixi task "
                "and no known executable"
            )
    return errors


def validate_manifest_build_tasks(repo_root: Path) -> list[str]:
    """Ensure every package build command names a declared Pixi task.

    A `[[packages]].command` is not documentation: the rendered build job runs it
    verbatim. A task that was renamed or never written would otherwise fail on the
    release runner, after the tag, instead of in `repo-check`.
    """
    manifest_path = repo_root / "universe.toml"
    pixi_path = repo_root / "pixi.toml"
    if not path_exists(manifest_path) or not path_exists(pixi_path):
        return []
    try:
        manifest: dict[str, Any] = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        pixi: dict[str, Any] = tomllib.loads(pixi_path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        return [f"cannot read the manifest build tasks: {exc}"]

    tasks_value = pixi.get("tasks", {})
    if not isinstance(tasks_value, dict):
        return ["pixi.toml has no [tasks] table"]
    tasks: dict[str, Any] = tasks_value

    packages_value = manifest.get("packages", [])
    if not isinstance(packages_value, list):
        return ["universe.toml has no [[packages]] list"]

    errors: list[str] = []
    for package_value in packages_value:
        if not isinstance(package_value, dict):
            continue
        name = str(package_value.get("name", "<unnamed>"))
        command_value = package_value.get("command", [])
        if not isinstance(command_value, list):
            errors.append(f"universe.toml: package '{name}' has no command list")
            continue
        command: list[str] = [token for token in command_value if isinstance(token, str)]
        if not command:
            continue
        if len(command) < 2 or command[0] != "pixi" or command[1] != "run":
            errors.append(f"universe.toml: package '{name}' build command is not `pixi run`")
            continue
        task_name: str | None = None
        index = 2
        while index < len(command):
            token = command[index]
            if token.startswith("-"):
                index += 1 + (token in PIXI_VALUE_FLAGS)
                continue
            task_name = token
            break
        if task_name is None:
            errors.append(f"universe.toml: package '{name}' names no Pixi task")
        elif task_name not in tasks:
            errors.append(
                f"universe.toml: package '{name}' names no declared Pixi task '{task_name}'"
            )
    return errors


def _task_number(identifier: str) -> int:
    """Sort task identifiers numerically rather than lexicographically."""
    return int(identifier.split("-")[1])


def _iter_repo_files(repo_root: Path) -> list[Path]:
    files: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(repo_root):
        dirnames[:] = sorted(name for name in dirnames if name not in SKIP_DIRS)
        for filename in filenames:
            files.append(Path(dirpath) / filename)
    return sorted(files, key=lambda path: path.relative_to(repo_root).as_posix())


def _contains_legacy_token(data: bytes) -> bool:
    lowered = data.lower()
    return any(token.encode() in lowered for token in LEGACY_TOKENS)


def validate_repo(repo_root: Path | None = None, tags: Sequence[str] | None = None) -> list[str]:
    """Validate the non-declarative repository invariants."""
    root = Path.cwd() if repo_root is None else repo_root
    errors: list[str] = []

    # The token scan reads every file, so it also surfaces unreadable paths
    # (dangling symlinks) instead of silently skipping them.
    for file in _iter_repo_files(root):
        relative_path = file.relative_to(root)
        relative_posix = relative_path.as_posix()
        if _contains_legacy_token(relative_posix.encode()):
            errors.append(f"legacy repository reference in path: {relative_posix}")
            continue
        try:
            data = file.read_bytes()
        except OSError as exc:
            errors.append(f"cannot read {relative_posix}: {exc}")
            continue
        if _contains_legacy_token(data):
            errors.append(f"legacy repository reference in content: {relative_posix}")

    versioned_packages = (
        "packages/config",
        "packages/portal",
        "packages/schema",
        "crates/universe-assets",
        "crates/universe-cli",
    )
    for package in versioned_packages:
        directory = root / package
        versions_file = directory / "VERSIONS"
        manifest = directory / ("package.json" if package.startswith("packages/") else "Cargo.toml")
        if not path_exists(versions_file) or not path_exists(manifest):
            errors.append(f"versioned package is missing VERSIONS or manifest: {package}")
            continue
        try:
            declared = versions_file.read_text(encoding="utf-8").strip()
            if manifest.suffix == ".json":
                manifest_version = json.loads(manifest.read_text(encoding="utf-8")).get("version")
            else:
                cargo_manifest = tomllib.loads(manifest.read_text(encoding="utf-8"))
                manifest_version = cargo_manifest["package"].get("version")
                if isinstance(manifest_version, dict) and manifest_version.get("workspace") is True:
                    workspace = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
                    manifest_version = workspace["workspace"]["package"]["version"]
            if not declared or declared != manifest_version:
                errors.append(
                    f"version drift in {package}: VERSIONS={declared or '<empty>'}, "
                    f"manifest={manifest_version or '<missing>'}"
                )
        except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
            errors.append(f"cannot validate package version in {package}: {exc}")

    workspace_manifest_path = root / "Cargo.toml"
    if path_exists(workspace_manifest_path):
        try:
            workspace_package = tomllib.loads(workspace_manifest_path.read_text(encoding="utf-8"))
            canonical_version = workspace_package["workspace"]["package"]["version"]
        except (OSError, KeyError, tomllib.TOMLDecodeError) as exc:
            errors.append(f"cannot read workspace package version for recipes: {exc}")
        else:
            for crate_name in ("universe-kit", "universe-airlock", "universe-airlock-bootstrap"):
                recipe = root / "crates" / crate_name / "recipe" / "recipe.yaml"
                if not path_exists(recipe):
                    continue
                text = recipe.read_text(encoding="utf-8")
                recipe_name = extract_recipe_package_name(text)
                recipe_version = extract_recipe_context_version(text)
                if recipe_name != crate_name:
                    errors.append(
                        f"recipe package name drift in {recipe.relative_to(root)}: "
                        f"found {recipe_name or '<missing>'}, expected {crate_name}"
                    )
                # The directory name is not the Cargo workspace's identity: the crate's own
                # manifest is the source of truth for what its recipe is allowed to name.
                crate_manifest = root / "crates" / crate_name / "Cargo.toml"
                if path_exists(crate_manifest):
                    try:
                        crate_identity = tomllib.loads(crate_manifest.read_text(encoding="utf-8"))[
                            "package"
                        ]["name"]
                    except (OSError, KeyError, tomllib.TOMLDecodeError) as exc:
                        errors.append(f"cannot read crate identity for {crate_name}: {exc}")
                    else:
                        if recipe_name != crate_identity:
                            errors.append(
                                f"recipe identity drift in {recipe.relative_to(root)}: "
                                f"recipe={recipe_name or '<missing>'}, "
                                f"Cargo.toml={crate_identity}"
                            )
                if recipe_version != canonical_version:
                    errors.append(
                        f"recipe version drift in {recipe.relative_to(root)}: "
                        f"found {recipe_version or '<missing>'}, expected {canonical_version}"
                    )

            facade_recipe = root / "crates" / "universe-cli" / "recipe" / "recipe.yaml"
            if path_exists(facade_recipe):
                text = facade_recipe.read_text(encoding="utf-8")
                recipe_name = extract_recipe_package_name(text)
                recipe_version = extract_recipe_context_version(text)
                expected_version_file = root / "crates" / "universe-cli" / "VERSIONS"
                expected_version = (
                    expected_version_file.read_text(encoding="utf-8").strip()
                    if path_exists(expected_version_file)
                    else ""
                )
                if recipe_name != "universe":
                    errors.append(
                        f"recipe package name drift in {facade_recipe.relative_to(root)}: "
                        f"found {recipe_name or '<missing>'}, expected universe"
                    )
                if recipe_version != expected_version:
                    errors.append(
                        f"recipe version drift in {facade_recipe.relative_to(root)}: "
                        f"found {recipe_version or '<missing>'}, "
                        f"expected {expected_version or '<missing>'}"
                    )

    errors.extend(validate_roadmap_coverage(root))
    errors.extend(validate_documented_pixi_tasks(root))
    errors.extend(validate_manifest_build_tasks(root))
    errors.extend(validate_scheduled_audit(root))
    ci_workflow_path = root / ".github" / "workflows" / "ci.yml"
    if path_exists(ci_workflow_path):
        errors.extend(validate_required_gate_name(ci_workflow_path.read_text(encoding="utf-8")))

    offline_transport_paths = (
        root / "Cargo.lock",
        root / ".pixi-sandbox.toml",
        root / ".github" / "workflows" / "publish-sandbox.yml",
        root / "lefthook.yml",
        root / "pixi.toml",
    )
    if all(path_exists(path) for path in offline_transport_paths):
        cargo_lock, sandbox_config, publisher_workflow, lefthook_config, pixi_config = (
            offline_transport_paths
        )
        errors.extend(
            validate_offline_cargo_transport(
                cargo_lock.read_text(encoding="utf-8"),
                sandbox_config.read_text(encoding="utf-8"),
                publisher_workflow.read_text(encoding="utf-8"),
                lefthook_config.read_text(encoding="utf-8"),
                pixi_config.read_text(encoding="utf-8"),
            )
        )
        errors.extend(validate_audit_gate(pixi_config.read_text(encoding="utf-8")))
        errors.extend(validate_gate_order(pixi_config.read_text(encoding="utf-8")))
        errors.extend(validate_session_bootstrap(pixi_config.read_text(encoding="utf-8")))
        errors.extend(
            validate_workflow_push_paths(
                sandbox_config.read_text(encoding="utf-8"),
                publisher_workflow.read_text(encoding="utf-8"),
            )
        )

    if tags is None:
        try:
            tags = release_tags(root)
        except (OSError, RuntimeError) as exc:
            errors.append(f"cannot read release tags: {exc}")
            tags = []
    errors.extend(validate_release_version(root, tags))
    tag_subject = None
    version = _workspace_version(root)
    if version is not None and f"v{version}" in tags:
        try:
            tag_subject = release_tag_subject(root, version)
        except (OSError, RuntimeError) as exc:
            errors.append(f"cannot read the v{version} tag annotation: {exc}")
    errors.extend(validate_release_notes(root, tag_subject=tag_subject))
    return errors


def main(argv: list[str] | None = None) -> int:
    """Run the repository checker from the command line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)

    errors = validate_repo(args.root)
    if errors:
        for error in errors:
            print(f"repo-check: {error}", file=sys.stderr)
        return 1
    print("repo-check: repository invariants are valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
