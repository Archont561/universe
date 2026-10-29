"""Tests for the ecosystem-neutral core dependency boundary."""

import json
import subprocess
from pathlib import Path
from typing import Any

import pytest

from scripts import crate_check
from scripts.crate_check import ALLOWED_DEPENDENCIES, validate_boundary


def _package(package_id: str, name: str) -> dict[str, Any]:
    return {"id": package_id, "name": name}


def _node(
    node_id: str,
    deps: list[tuple[str, str | None]],
) -> dict[str, Any]:
    return {
        "id": node_id,
        "deps": [{"pkg": pkg, "dep_kinds": [{"kind": kind, "target": None}]} for pkg, kind in deps],
    }


def _metadata(
    packages: list[dict[str, Any]],
    nodes: list[dict[str, Any]],
    workspace_members: list[str] | None = None,
) -> dict[str, Any]:
    return {
        "packages": packages,
        "resolve": {"nodes": nodes},
        "workspace_members": workspace_members or ["core"],
    }


def _neutral_metadata() -> dict[str, Any]:
    packages = [
        _package("core", "universe-core"),
        _package("serde", "serde"),
        _package("derive", "serde_derive"),
        _package("proc", "proc-macro2"),
    ]
    nodes = [
        _node("core", [("serde", None)]),
        _node("serde", [("derive", None)]),
        _node("derive", [("proc", "build")]),
        _node("proc", []),
    ]
    return _metadata(packages, nodes)


def test_accepts_the_serialization_closure() -> None:
    assert validate_boundary(_neutral_metadata()) == []


def test_rejects_a_dependency_outside_the_allowlist() -> None:
    metadata = _neutral_metadata()
    metadata["packages"].append(_package("http", "reqwest"))
    metadata["resolve"]["nodes"][0]["deps"].append(
        {"pkg": "http", "dep_kinds": [{"kind": None, "target": None}]}
    )

    errors = validate_boundary(metadata)

    assert len(errors) == 1
    assert "reqwest" in errors[0]
    assert "outside the neutral allowlist" in errors[0]


def test_rejects_transitive_ecosystem_and_terminal_crates() -> None:
    for forbidden in ("rattler_conda_types", "ratatui", "tokio"):
        metadata = _neutral_metadata()
        metadata["packages"].append(_package("extra", forbidden))
        metadata["resolve"]["nodes"][2]["deps"].append(
            {"pkg": "extra", "dep_kinds": [{"kind": None, "target": None}]}
        )

        errors = validate_boundary(metadata)

        assert any(forbidden in error for error in errors), forbidden


def test_ignores_dev_only_dependencies() -> None:
    metadata = _neutral_metadata()
    metadata["packages"].append(_package("proptest", "proptest"))
    metadata["resolve"]["nodes"][0]["deps"].append(
        {"pkg": "proptest", "dep_kinds": [{"kind": "dev", "target": None}]}
    )

    assert validate_boundary(metadata) == []


def test_rejects_a_workspace_crate_dependency() -> None:
    metadata = _neutral_metadata()
    metadata["packages"].append(_package("assets", "universe-assets"))
    metadata["workspace_members"] = ["core", "assets"]
    metadata["resolve"]["nodes"][0]["deps"].append(
        {"pkg": "assets", "dep_kinds": [{"kind": None, "target": None}]}
    )

    errors = validate_boundary(metadata)

    assert any("must not depend on workspace crate universe-assets" in e for e in errors)


def test_reports_a_package_missing_from_the_graph() -> None:
    errors = validate_boundary(_metadata([], []))

    assert errors == ["universe-core: package is not part of the resolved workspace"]


def test_allowlist_names_no_runtime_machinery() -> None:
    for forbidden in ("reqwest", "tokio", "ratatui", "rattler", "clap"):
        assert forbidden not in ALLOWED_DEPENDENCIES


def test_main_reports_success_and_boundary_failures(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(crate_check, "load_metadata", lambda root: _neutral_metadata())
    assert crate_check.main([]) == 0
    assert "stays ecosystem" in capsys.readouterr().out

    failing = _neutral_metadata()
    failing["packages"].append(_package("http", "reqwest"))
    failing["resolve"]["nodes"][0]["deps"].append(
        {"pkg": "http", "dep_kinds": [{"kind": None, "target": None}]}
    )
    monkeypatch.setattr(crate_check, "load_metadata", lambda root: failing)

    assert crate_check.main([]) == 1
    assert "reqwest" in capsys.readouterr().err


def test_load_metadata_fails_closed_without_cargo(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr("scripts.crate_check.shutil.which", lambda name: None)

    with pytest.raises(SystemExit, match="cargo is not on PATH"):
        crate_check.load_metadata(Path("/repo"))


def test_load_metadata_reports_a_failed_run(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr("scripts.crate_check.shutil.which", lambda name: "/usr/bin/cargo")
    completed = subprocess.CompletedProcess(args=[], returncode=101, stdout="", stderr="boom")
    monkeypatch.setattr("scripts.crate_check.subprocess.run", lambda *args, **kwargs: completed)

    with pytest.raises(SystemExit, match="boom"):
        crate_check.load_metadata(Path("/repo"))


def test_load_metadata_returns_the_parsed_graph(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr("scripts.crate_check.shutil.which", lambda name: "/usr/bin/cargo")
    completed = subprocess.CompletedProcess(
        args=[], returncode=0, stdout=json.dumps(_neutral_metadata()), stderr=""
    )
    monkeypatch.setattr("scripts.crate_check.subprocess.run", lambda *args, **kwargs: completed)

    assert crate_check.load_metadata(Path("/repo"))["packages"]
