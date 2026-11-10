"""Tests for the Python repository checker."""

import re
from pathlib import Path

import pytest

from scripts.repo_check import (
    ROADMAP_DOCUMENTS,
    V1_SURFACE_KEYS,
    extract_recipe_context_version,
    extract_recipe_package_name,
    extract_task_ids,
    main,
    tag_annotation,
    validate_audit_gate,
    validate_documented_pixi_tasks,
    validate_gate_order,
    validate_offline_cargo_transport,
    validate_release_notes,
    validate_release_version,
    validate_repo,
    validate_required_gate_name,
    validate_roadmap_coverage,
    validate_scheduled_audit,
    validate_session_bootstrap,
    validate_workflow_push_paths,
)


def test_extracts_recipe_version_and_package_name() -> None:
    recipe = """context:
  version: "2.3.4"

package:
  name: universe-kit
  version: ${{ version }}
"""

    assert extract_recipe_context_version(recipe) == "2.3.4"
    assert extract_recipe_package_name(recipe) == "universe-kit"
    assert extract_recipe_context_version("package:\n  name: x\n") is None


def test_offline_cargo_gates_require_vendored_transport() -> None:
    lockfile = '[[package]]\nname = "serde"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n'
    sandbox_config = (
        "cargo_vendor = true\n"
        "\n[[bundle]]\n"
        "cargo_vendor = true\n"
        "\n[workflow]\n"
        "push_paths = [\n"
        '  ".pixi-sandbox.toml",\n'
        '  "Cargo.lock",\n'
        '  "Cargo.toml",\n'
        '  "crates/**/Cargo.toml",\n'
        '  "pixi.lock",\n'
        '  "pixi.toml",\n'
        "]\n"
    )
    publisher = "args: pixi-sandbox pipeline\n  --cargo-vendor true\n"
    lefthook = (
        "pre-push:\n  commands:\n    gates:\n"
        "      run: CARGO_NET_OFFLINE=true pixi run --offline --as-is offline-gates\n"
    )
    pixi_config = (
        '[tasks.offline-gates]\ndepends-on = ["check", "coverage", "lint-actions", "build"]\n'
    )

    assert (
        validate_offline_cargo_transport(lockfile, sandbox_config, publisher, lefthook, pixi_config)
        == []
    )

    errors = validate_offline_cargo_transport(
        lockfile,
        sandbox_config,
        publisher.replace("--cargo-vendor true", "--cargo-vendor false"),
        lefthook.replace("CARGO_NET_OFFLINE=true ", ""),
        pixi_config,
    )
    assert any("publisher must include Cargo vendor data" in error for error in errors)
    assert any("pre-push must run offline-gates" in error for error in errors)

    pixi_with_online_dependency = (
        '[tasks.offline-gates]\ndepends-on = ["check", "coverage", "lint-actions", "build"]\n'
        '[tasks.check]\ndepends-on = ["audit"]\n'
        '[tasks.audit]\ncmd = "cargo audit"\n'
    )
    online_errors = validate_offline_cargo_transport(
        lockfile, sandbox_config, publisher, lefthook, pixi_with_online_dependency
    )
    assert any("transitively includes online task: audit" in error for error in online_errors)


def test_the_standalone_repository_passes_repo_check() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    assert validate_repo(repo_root) == []


def test_reports_legacy_references_and_unreadable_paths(tmp_path: Path) -> None:
    """Shape checks moved to alint; the token scan and read failures stay here."""
    scripts = tmp_path / "scripts"
    scripts.mkdir()
    legacy_name = "conda" + "-universe.txt"
    (tmp_path / legacy_name).write_text("legacy path\n", encoding="utf-8")
    (tmp_path / "legacy-content.txt").write_text("air" + "pack\n", encoding="utf-8")
    (tmp_path / "broken-link").symlink_to(tmp_path / "missing-target")

    errors = "\n".join(validate_repo(tmp_path))

    assert f"legacy repository reference in path: {legacy_name}" in errors
    assert "legacy repository reference in content: legacy-content.txt" in errors
    assert "cannot read broken-link" in errors
    # Required files, forbidden paths, placeholder basenames, and TypeScript
    # automation are `.alint.yml` rules now, so a bare tree is not their concern.
    assert "required file is missing" not in errors
    assert "forbidden license/notice/placeholder file" not in errors
    assert "TypeScript repository script is forbidden" not in errors


def test_main_prints_success_and_failure(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    (tmp_path / "legacy.txt").write_text("conda" + "-universe\n", encoding="utf-8")

    assert main(["--root", str(tmp_path)]) == 1
    captured = capsys.readouterr()
    assert "legacy repository reference" in captured.err

    repo_root = Path(__file__).resolve().parents[2]
    assert main(["--root", str(repo_root)]) == 0
    captured = capsys.readouterr()
    assert captured.out == "repo-check: repository invariants are valid\n"


#: A minimal valid body for the release notes the fixture now has to provide. The first line
#: is written separately because `UNI-59` AC#2 reuses it as the annotated tag message.
_NOTES_BODY = (
    "\n\n## Not a v1 release\n\n"
    "Conda, Cargo, Python, npm, Airlock and portal: none of the six v1 surfaces is complete.\n\n"
    "The three verbs are `version`, `verify` and `restore`.\n"
)


def _notes_first_line(version: str = "0.1.0") -> str:
    return f"Universe {version} is a linux-64-only pre-release checkpoint, not v1."


def _write_notes(root: Path, first: str, body: str = _NOTES_BODY) -> None:
    path = root / "docs" / "releases" / "0.1.0.md"
    path.write_text(first + body, encoding="utf-8")


def _release_fixture(root: Path, changelog: str, workspace_version: str = "0.1.0") -> None:
    """The files release parity reads, and nothing else it needs."""
    (root / "Cargo.toml").write_text(
        f'[workspace.package]\nversion = "{workspace_version}"\n', encoding="utf-8"
    )
    (root / "CHANGELOG.md").write_text(changelog, encoding="utf-8")
    notes = root / "docs" / "releases" / f"{workspace_version}.md"
    notes.parent.mkdir(parents=True, exist_ok=True)
    notes.write_text(_notes_first_line(workspace_version) + _NOTES_BODY, encoding="utf-8")


def test_a_changelog_whose_first_version_disagrees_with_the_workspace_is_reported(
    tmp_path: Path,
) -> None:
    # `## Unreleased` is not a version; the first versioned heading is the release the
    # workspace claims. A 0.2.0 section beside a 0.1.0 workspace is drift the checker has to
    # name, not something a reader is left to notice at the tag.
    _release_fixture(
        tmp_path,
        "# Changelog\n\n## Unreleased\n\n## 0.2.0\n\n- A later release.\n",
    )
    errors = "\n".join(validate_repo(tmp_path))
    assert "changelog/workspace version drift: CHANGELOG.md=0.2.0, Cargo.toml=0.1.0" in errors, (
        errors
    )


def test_a_suffixed_version_heading_is_not_the_version(tmp_path: Path) -> None:
    # `universe lint` requires the released heading to equal the workspace version exactly;
    # `## 0.1.0 (pre-release)` is a heading with a suffix, not the version 0.1.0. Local
    # gates do not run that verb, so `repo-check` has to reach the same verdict.
    _release_fixture(
        tmp_path,
        "# Changelog\n\n## Unreleased\n\n## 0.1.0 (pre-release)\n\n- First.\n",
    )
    assert (
        "changelog/workspace version drift: "
        "CHANGELOG.md=0.1.0 (pre-release), Cargo.toml=0.1.0" in validate_release_version(tmp_path)
    )


def test_a_tag_that_disagrees_with_the_changelog_is_reported(tmp_path: Path) -> None:
    # A checkout before the first tag has nothing to disagree with. Once `v0.1.0` is cut, a
    # newer `v*` tag beside an older changelog is the drift AC#6 guards against. The highest
    # version wins, not the last tag in whatever order git happened to print them.
    _release_fixture(
        tmp_path,
        "# Changelog\n\n## Unreleased\n\n## 0.1.0\n\n- First release.\n",
    )
    assert (
        "changelog/tag version drift: CHANGELOG.md=0.1.0, newest v* tag=0.2.0"
        in validate_release_version(tmp_path, tags=["v0.1.0", "v0.2.0"])
    )
    assert "changelog/tag version drift: CHANGELOG.md=0.1.0, newest v* tag=0.2.0" in "\n".join(
        validate_repo(tmp_path, tags=["v0.2.0"])
    )
    assert validate_release_version(tmp_path, tags=[]) == []
    assert validate_release_version(tmp_path, tags=["v0.1.0"]) == []


def test_a_recipe_name_must_agree_with_its_crate_manifest(tmp_path: Path) -> None:
    # The directory name is not the Cargo workspace's identity. A recipe can match
    # `crates/universe-airlock/` and still name a package the crate manifest does not declare.
    _release_fixture(
        tmp_path,
        "# Changelog\n\n## Unreleased\n\n## 0.1.0\n\n- First release.\n",
    )
    crate = tmp_path / "crates" / "universe-airlock"
    (crate / "recipe").mkdir(parents=True)
    (crate / "Cargo.toml").write_text(
        '[package]\nname = "renamed-airlock"\nversion.workspace = true\n', encoding="utf-8"
    )
    (crate / "recipe" / "recipe.yaml").write_text(
        'context:\n  version: "0.1.0"\n\npackage:\n  name: universe-airlock\n',
        encoding="utf-8",
    )
    errors = "\n".join(validate_repo(tmp_path))
    assert (
        "recipe identity drift in crates/universe-airlock/recipe/recipe.yaml: "
        "recipe=universe-airlock, Cargo.toml=renamed-airlock" in errors
    ), errors


def test_versioned_packages_must_match_their_versions_files(tmp_path: Path) -> None:
    (tmp_path / "Cargo.toml").write_text(
        '[workspace.package]\nversion = "0.1.0"\n', encoding="utf-8"
    )
    for package in ("packages/config", "packages/portal", "packages/schema"):
        directory = tmp_path / package
        directory.mkdir(parents=True)
        (directory / "VERSIONS").write_text("0.1.0\n", encoding="utf-8")
        (directory / "package.json").write_text('{"version":"0.1.0"}\n', encoding="utf-8")
    assets = tmp_path / "crates/universe-assets"
    assets.mkdir(parents=True)
    (assets / "VERSIONS").write_text("0.2.0\n", encoding="utf-8")
    (assets / "Cargo.toml").write_text("[package]\nversion.workspace = true\n", encoding="utf-8")
    facade = tmp_path / "crates/universe-cli"
    facade.mkdir(parents=True)
    (facade / "VERSIONS").write_text("0.2.0\n", encoding="utf-8")
    (facade / "Cargo.toml").write_text("[package]\nversion.workspace = true\n", encoding="utf-8")

    errors = "\n".join(validate_repo(tmp_path))

    assert "version drift in crates/universe-assets: VERSIONS=0.2.0, manifest=0.1.0" in errors
    assert "version drift in crates/universe-cli: VERSIONS=0.2.0, manifest=0.1.0" in errors


def test_audit_gate_requires_the_offline_first_runner() -> None:
    configured = (
        "[tasks]\n"
        'audit = { cmd = "python -m scripts.audit check" }\n'
        'audit-sync = { cmd = "python -m scripts.audit sync" }\n'
    )
    legacy = (
        "[tasks]\n"
        'audit = { cmd = "cargo deny check && cargo audit --deny warnings" }\n'
        'audit-sync = { cmd = "python -m scripts.audit sync" }\n'
    )
    without_sync = '[tasks]\naudit = { cmd = "python -m scripts.audit check" }\n'

    assert validate_audit_gate(configured) == []
    assert "offline-first" in validate_audit_gate(legacy)[0]
    assert "audit-sync" in validate_audit_gate(without_sync)[0]


def test_audit_gate_may_not_return_to_the_per_commit_gates() -> None:
    in_gates = (
        "[tasks]\n"
        'audit = { cmd = "python -m scripts.audit check" }\n'
        'audit-sync = { cmd = "python -m scripts.audit sync" }\n'
        'gates = { depends-on = ["check", "audit"] }\n'
    )
    per_diff = (
        "[tasks]\n"
        'audit = { cmd = "python -m scripts.audit check" }\n'
        'audit-sync = { cmd = "python -m scripts.audit sync" }\n'
        'gates = { depends-on = ["check", "build"] }\n'
    )

    assert validate_audit_gate(per_diff) == []
    errors = validate_audit_gate(in_gates)
    assert any("must not depend on `audit`" in error for error in errors), errors


def test_a_scheduled_workflow_has_to_run_the_audit(tmp_path: Path) -> None:
    workflows = tmp_path / ".github" / "workflows"
    workflows.mkdir(parents=True)
    daily = """name: security
on:
  schedule:
    - cron: '17 5 * * *'
jobs:
  audit:
    steps:
      - run: pixi run --frozen audit
"""
    (workflows / "security.yml").write_text(daily, encoding="utf-8")

    assert validate_scheduled_audit(tmp_path) == []

    # The scheduled sweep is the only thing running the audit now, so its removal has to be
    # an error rather than a silent green.
    (workflows / "security.yml").unlink()
    (workflows / "ci.yml").write_text(
        "name: ci\njobs:\n  gates:\n    steps:\n      - run: pixi run gates\n",
        encoding="utf-8",
    )
    errors = validate_scheduled_audit(tmp_path)
    assert any("never run" in error for error in errors), errors


def test_an_audit_that_only_runs_on_push_is_not_a_schedule(tmp_path: Path) -> None:
    workflows = tmp_path / ".github" / "workflows"
    workflows.mkdir(parents=True)
    (workflows / "ci.yml").write_text(
        "name: ci\non:\n  push:\n    branches: [main]\n"
        "jobs:\n  audit:\n    steps:\n      - run: pixi run audit\n",
        encoding="utf-8",
    )

    errors = validate_scheduled_audit(tmp_path)
    assert any("never on a schedule" in error for error in errors), errors
    assert any("never run" in error for error in errors), errors


def test_the_cheap_checks_run_before_anything_compiles() -> None:
    ordered = (
        "[tasks]\n"
        'check-fast = { depends-on = ["fmt-check", "backlog-check", "repo-check", '
        '"skills-check", "alint-check", "crate-check", "lint-gherkin", "lint-prose"] }\n'
        'check = { depends-on = ["check-fast", "lint", "typecheck"] }\n'
    )
    compile_first = (
        "[tasks]\n"
        'check-fast = { depends-on = ["fmt-check", "backlog-check", "repo-check", '
        '"skills-check", "alint-check", "crate-check", "lint-gherkin", "lint-prose"] }\n'
        'check = { depends-on = ["lint", "check-fast"] }\n'
    )
    incomplete = (
        "[tasks]\n"
        'check-fast = { depends-on = ["fmt-check", "alint-check"] }\n'
        'check = { depends-on = ["check-fast", "lint"] }\n'
    )

    assert validate_gate_order(ordered) == []
    assert any("first dependency" in error for error in validate_gate_order(compile_first))
    assert any("missing the checks" in error for error in validate_gate_order(incomplete))


def test_the_session_start_installs_the_hooks() -> None:
    wired = (
        "[tasks]\n"
        'setup = { depends-on = ["hooks-install"] }\n'
        'hooks-install = { cmd = "lefthook install" }\n'
    )
    # A `setup` that still exists but reaches nothing is the failure worth catching: the
    # command succeeds, the session thinks the hooks are on, and the first bad commit is the
    # one that finds out otherwise.
    no_hooks = (
        '[tasks]\nsetup = { depends-on = ["fmt"] }\nhooks-install = { cmd = "lefthook install" }\n'
    )
    renamed = (
        '[tasks]\nsetup = { depends-on = ["hooks-install"] }\nhooks-install = { cmd = "true" }\n'
    )

    assert validate_session_bootstrap(wired) == []
    assert any(
        "must depend on `hooks-install`" in error for error in validate_session_bootstrap(no_hooks)
    )
    assert any(
        "must run `lefthook install`" in error for error in validate_session_bootstrap(renamed)
    )


def test_ci_keeps_a_job_named_gates() -> None:
    split = "jobs:\n  fast:\n    runs-on: ubuntu-latest\n  gates:\n    needs: [fast]\n"
    renamed = "jobs:\n  fast:\n    runs-on: ubuntu-latest\n  verdict:\n    needs: [fast]\n"

    assert validate_required_gate_name(split) == []
    # A required check that never reports blocks the pull request on `Expected`, which is a
    # worse failure than a slow one.
    assert "gates" in validate_required_gate_name(renamed)[0]


def test_offline_transport_requires_every_republish_trigger() -> None:
    lockfile = '[[package]]\nname = "serde"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n'
    sandbox_config = (
        "cargo_vendor = true\n"
        "\n[[bundle]]\n"
        'name = "developer"\n'
        'platforms = ["linux-64"]\n'
        "cargo_vendor = true\n"
        "\n[workflow]\n"
        'push_paths = ["pixi.toml", "pixi.lock"]\n'
    )
    publisher = "args: pixi-sandbox pipeline\n  --cargo-vendor true\n"
    lefthook = (
        "pre-push:\n  commands:\n    gates:\n"
        "      run: CARGO_NET_OFFLINE=true pixi run --offline --as-is offline-gates\n"
    )
    pixi_config = (
        '[tasks.offline-gates]\ndepends-on = ["check", "coverage", "lint-actions", "build"]\n'
    )

    errors = validate_offline_cargo_transport(
        lockfile, sandbox_config, publisher, lefthook, pixi_config
    )

    assert any("Cargo.lock" in error and "push_paths" in error for error in errors)


def test_the_publish_workflow_lists_exactly_the_declared_push_paths() -> None:
    sandbox_config = (
        '[workflow]\npush_paths = [\n  ".pixi-sandbox.toml",\n  "Cargo.toml",\n  "bun.lock",\n]\n'
    )
    workflow = (
        "on:\n"
        "  push:\n"
        "    branches: [main]\n"
        "    paths:\n"
        '      - ".pixi-sandbox.toml"\n'
        '      - "Cargo.toml"\n'
        '      - "bun.lock"\n'
        "\n"
        "permissions:\n"
        "  contents: read\n"
    )

    assert validate_workflow_push_paths(sandbox_config, workflow) == []


def test_a_publish_workflow_that_forgot_a_declared_trigger_is_reported() -> None:
    # The transport rots silently when a trigger exists in the config and not in the
    # workflow: a change to `bun.lock` repacks the environment but never republishes it.
    sandbox_config = '[workflow]\npush_paths = [".pixi-sandbox.toml", "Cargo.toml", "bun.lock"]\n'
    workflow = (
        "on:\n"
        "  push:\n"
        "    paths:\n"
        '      - ".pixi-sandbox.toml"\n'
        '      - "Cargo.toml"\n'
        "\n"
        "permissions:\n"
        "  contents: read\n"
    )

    errors = validate_workflow_push_paths(sandbox_config, workflow)

    assert len(errors) == 1
    assert "bun.lock" in errors[0]
    assert "workflow" in errors[0]


def test_a_publish_workflow_trigger_the_config_does_not_declare_is_reported() -> None:
    sandbox_config = '[workflow]\npush_paths = ["Cargo.toml"]\n'
    workflow = 'on:\n  push:\n    paths:\n      - "Cargo.toml"\n      - "docs/**"\n'

    errors = validate_workflow_push_paths(sandbox_config, workflow)

    assert any("docs/**" in error for error in errors)


def test_a_publish_workflow_with_no_push_paths_block_is_reported() -> None:
    sandbox_config = '[workflow]\npush_paths = ["Cargo.toml"]\n'

    errors = validate_workflow_push_paths(sandbox_config, "on:\n  workflow_dispatch:\n")

    assert any("push" in error and "paths" in error for error in errors)


def _write_task(root: Path, number: int) -> None:
    tasks = root / "backlog" / "tasks"
    tasks.mkdir(parents=True, exist_ok=True)
    (tasks / f"uni-{number} - A-task.md").write_text(
        f"---\nid: UNI-{number}\ntitle: A task\n---\n", encoding="utf-8"
    )


def _write_roadmap(root: Path, document: str, body: str) -> None:
    path = root / document
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body, encoding="utf-8")


def test_roadmap_coverage_accepts_a_document_that_names_every_task(tmp_path: Path) -> None:
    _write_task(tmp_path, 1)
    _write_task(tmp_path, 2)
    for document in ROADMAP_DOCUMENTS:
        _write_roadmap(tmp_path, document, "| M | `UNI-1`, `UNI-2` |\n")

    assert extract_task_ids(tmp_path) == {"UNI-1", "UNI-2"}
    assert validate_roadmap_coverage(tmp_path) == []


def test_roadmap_coverage_reports_a_task_the_roadmap_forgot(tmp_path: Path) -> None:
    _write_task(tmp_path, 1)
    _write_task(tmp_path, 7)
    _write_roadmap(tmp_path, ROADMAP_DOCUMENTS[0], "| M | `UNI-1` |\n")

    errors = validate_roadmap_coverage(tmp_path)

    assert len(errors) == 1
    assert "UNI-7 is missing from the roadmap" in errors[0]


def test_roadmap_coverage_reports_an_identifier_without_a_task(tmp_path: Path) -> None:
    _write_task(tmp_path, 1)
    _write_roadmap(tmp_path, ROADMAP_DOCUMENTS[1], "`UNI-1` then `UNI-9`\n")

    errors = validate_roadmap_coverage(tmp_path)

    assert len(errors) == 1
    assert "UNI-9 is referenced but has no task record" in errors[0]


def _documented_repo(root: Path, readme: str) -> Path:
    """Write a minimal repository whose Pixi table declares the tasks under test."""
    (root / "pixi.toml").write_text(
        '[tasks]\nfmt = { cmd = "cargo fmt" }\ngates = { depends-on = ["fmt"] }\n'
        'package-kit = { cmd = "rattler-build build" }\n',
        encoding="utf-8",
    )
    (root / "README.md").write_text(readme, encoding="utf-8")
    return root


def test_documented_task_names_must_be_declared(tmp_path: Path) -> None:
    root = _documented_repo(
        tmp_path,
        "```sh\npixi run fmt\npixi run universe -- version\n```\n",
    )

    errors = validate_documented_pixi_tasks(root)

    assert errors == [
        "README.md: `pixi run universe` names no declared Pixi task and no known executable"
    ]


def test_documented_executables_and_placeholders_are_not_tasks(tmp_path: Path) -> None:
    root = _documented_repo(
        tmp_path,
        "```sh\n"
        "pixi run --locked cargo test -p universe-core\n"
        "pixi run --environment packaging package-kit\n"
        "pixi run <verb>\n"
        "pixi run --frozen\n"
        "```\n",
    )

    assert validate_documented_pixi_tasks(root) == []


def test_documented_tasks_ignore_backlog_promises(tmp_path: Path) -> None:
    root = _documented_repo(tmp_path, "pixi run fmt\n")
    task = root / "backlog" / "tasks" / "uni-1 - Planned.md"
    task.parent.mkdir(parents=True)
    task.write_text('Add `ci-render = { cmd = "pixi run ci-render" }`.', encoding="utf-8")

    assert validate_documented_pixi_tasks(root) == []


def test_documented_tasks_are_checked_in_the_agent_and_contributor_guides(
    tmp_path: Path,
) -> None:
    root = _documented_repo(tmp_path, "pixi run fmt\n")
    (root / "AGENTS.md").write_text("`pixi run lint`\n", encoding="utf-8")

    assert validate_documented_pixi_tasks(root) == [
        "AGENTS.md: `pixi run lint` names no declared Pixi task and no known executable"
    ]


def test_the_workspace_version_selects_which_release_notes_must_exist(tmp_path: Path) -> None:
    # `UNI-59` AC#3 needs a notes file for the version the tag will carry. Deriving the name
    # from `Cargo.toml` means a bump cannot leave the previous release's notes standing in for
    # the new one's.
    _release_fixture(tmp_path, "# Changelog\n\n## Unreleased\n\n## 0.1.0\n\n- First.\n")
    (tmp_path / "docs" / "releases" / "0.1.0.md").unlink()
    assert "release notes are missing: docs/releases/0.1.0.md" in validate_release_notes(tmp_path)

    later = tmp_path / "later"
    later.mkdir()
    _release_fixture(later, "# Changelog\n\n## 0.2.0\n\n- Later.\n", workspace_version="0.2.0")
    (later / "docs" / "releases" / "0.2.0.md").unlink()
    assert "release notes are missing: docs/releases/0.2.0.md" in validate_release_notes(later)


def test_the_tag_annotation_is_the_first_non_empty_line() -> None:
    assert tag_annotation("\n\nUniverse 0.1.0 is a checkpoint.\n\nBody.\n") == (
        "Universe 0.1.0 is a checkpoint."
    )
    assert tag_annotation("\n\n") is None


def test_the_notes_open_with_a_plain_annotation_naming_the_version(tmp_path: Path) -> None:
    # AC#2 annotates the tag with the message the notes open with. A heading, a frontmatter
    # marker or a wrapped first line would put Markdown or YAML into `git tag -a -m`.
    _release_fixture(tmp_path, "# Changelog\n\n## Unreleased\n\n## 0.1.0\n\n- First.\n")
    _write_notes(tmp_path, "# Universe 0.1.0")
    assert "plain tag annotation" in "\n".join(validate_release_notes(tmp_path))

    _write_notes(tmp_path, "A pre-release checkpoint.")
    assert "tag annotation does not name 0.1.0" in "\n".join(validate_release_notes(tmp_path))

    _write_notes(tmp_path, "Universe 0.1.0 is a pre-release checkpoint")
    assert "tag annotation must end with a period" in "\n".join(validate_release_notes(tmp_path))

    _write_notes(tmp_path, _notes_first_line())
    assert validate_release_notes(tmp_path) == []


def test_the_notes_state_the_pre_release_scope_and_the_three_verbs(tmp_path: Path) -> None:
    _release_fixture(tmp_path, "# Changelog\n\n## Unreleased\n\n## 0.1.0\n\n- First.\n")
    _write_notes(
        tmp_path,
        _notes_first_line(),
        "\n\n## Not a v1 release\n\nConda Cargo Python npm Airlock portal.\n\n"
        "Only `version` and `verify`.\n",
    )
    assert "release notes do not state restore" in validate_release_notes(tmp_path)

    _write_notes(
        tmp_path,
        "Universe 0.1.0 is a checkpoint, not v1.",
        "\n\n## Not a v1 release\n\nConda Cargo Python npm Airlock portal.\n\n"
        "`version`, `verify` and `restore`.\n",
    )
    errors = validate_release_notes(tmp_path)
    assert "release notes do not state pre-release" in errors
    assert "release notes do not state linux-64" in errors


def test_the_notes_disclaim_every_v1_surface_named_by_decision_119(tmp_path: Path) -> None:
    _release_fixture(tmp_path, "# Changelog\n\n## Unreleased\n\n## 0.1.0\n\n- First.\n")
    _write_notes(
        tmp_path,
        _notes_first_line(),
        "\n\nThe three verbs are `version`, `verify` and `restore`.\n",
    )
    assert "no 'Not a v1 release' section" in "\n".join(validate_release_notes(tmp_path))

    _write_notes(
        tmp_path,
        _notes_first_line(),
        "\n\n## Not a v1 release\n\nCargo, Python and npm are absent.\n\n"
        "`version`, `verify` and `restore`.\n",
    )
    errors = "\n".join(validate_release_notes(tmp_path))
    for key in ("conda", "airlock", "portal"):
        assert key in errors, errors


def test_the_v1_surface_keys_are_transcribed_from_decision_119() -> None:
    # The six keys are not an invented list: they name the six numbered surfaces of the
    # decision AC#3 says the notes must not claim. If that decision gains or reorders a
    # surface, this test fails and the check has to be revisited rather than quietly
    # disclaiming a v1 the repository no longer describes.
    root = Path(__file__).resolve().parents[2]
    name = "decision-119 - Full-multi-ecosystem-v1-and-static-catalog-distribution.md"
    decision = (root / "backlog" / "decisions" / name).read_text(encoding="utf-8")
    section = decision.split("## Decision", 1)[1].split("## Consequences", 1)[0]
    items = [line for line in section.splitlines() if re.match(r"^\d+\. ", line)]
    assert len(items) == len(V1_SURFACE_KEYS)
    for item, key in zip(items, V1_SURFACE_KEYS, strict=True):
        assert key in item.lower(), (key, item)


def test_a_tag_annotation_that_differs_from_the_notes_is_reported(tmp_path: Path) -> None:
    _release_fixture(tmp_path, "# Changelog\n\n## Unreleased\n\n## 0.1.0\n\n- First.\n")
    assert validate_release_notes(tmp_path, tag_subject=_notes_first_line()) == []

    errors = "\n".join(
        validate_release_notes(tmp_path, tag_subject="Universe 0.1.0 is a v1 release.")
    )
    assert "tag annotation drift" in errors
    assert "Universe 0.1.0 is a v1 release." in errors
