"""Validate the invariants a declarative config cannot express.

Repository shape — required files, forbidden paths and basenames, naming, and
immutable CI refs — is owned by `.alint.yml` (decision-121, `pixi run
alint-check`). What stays here is the rest: a legacy-token scan that must not
match its own configuration, version inheritance through a Cargo workspace,
recipe drift, and set/graph reasoning over the offline transport and the audit
gate.
"""

import argparse
import json
import os
import re
import sys
import tomllib
from pathlib import Path

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


def validate_repo(repo_root: Path | None = None) -> list[str]:
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
        errors.extend(
            validate_workflow_push_paths(
                sandbox_config.read_text(encoding="utf-8"),
                publisher_workflow.read_text(encoding="utf-8"),
            )
        )

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
