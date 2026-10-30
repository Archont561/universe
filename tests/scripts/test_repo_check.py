"""Tests for the Python repository checker."""

from pathlib import Path

import pytest

from scripts.repo_check import (
    ROADMAP_DOCUMENTS,
    extract_recipe_context_version,
    extract_recipe_package_name,
    extract_task_ids,
    main,
    validate_audit_gate,
    validate_offline_cargo_transport,
    validate_repo,
    validate_roadmap_coverage,
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
