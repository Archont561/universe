"""Validate the dependency boundary of the ecosystem-neutral core crate.

`universe-core` is the vocabulary every adapter, binary, and future consumer
shares, so its dependency closure is the thing that decides whether that
vocabulary stays neutral. A comment claiming neutrality is not evidence; this
check reads the resolved graph from ``cargo metadata`` and fails closed.

The policy is an allowlist, not a denylist: a new dependency has to be added here
deliberately, in the same review that adds it to the crate. Denylists age badly —
they catch the crate you thought of and admit the one you did not.
"""

import argparse
import json
import shutil
import subprocess
import sys
from collections import deque
from pathlib import Path
from typing import Any

NEUTRAL_PACKAGE = "universe-core"

#: Crates `universe-core` may reach through normal or build dependencies.
#:
#: Serialization and its proc-macro machinery, plus the error derive. Nothing
#: here performs I/O, networking, terminal rendering, package resolution, or
#: ecosystem-specific parsing: those are the jobs of adapter and binary crates.
ALLOWED_DEPENDENCIES = frozenset(
    {
        # TOML parsing, for `universe.toml`. The manifest contract is core's, and the
        # neutral vocabulary is what every store and adapter is described in; the parser
        # reads text and performs no I/O. `toml_edit` and its parser closure come with
        # `parse` only — no `display`, no `serde` feature — so the dependency stays a
        # reader and never becomes a writer of project files.
        "equivalent",
        "hashbrown",
        "indexmap",
        "itoa",
        "memchr",
        "proc-macro2",
        "quote",
        "serde",
        "serde_core",
        "serde_derive",
        "serde_json",
        "syn",
        "thiserror",
        "thiserror-impl",
        "toml_datetime",
        "toml_edit",
        "toml_parser",
        "unicode-ident",
        "winnow",
        "zmij",
    }
)


def _packages(metadata: dict[str, Any]) -> dict[str, dict[str, Any]]:
    """Index ``cargo metadata`` packages by package id."""
    return {package["id"]: package for package in metadata["packages"]}


def _normal_edges(node: dict[str, Any]) -> list[str]:
    """Package ids this node depends on for normal or build use.

    Dev-only edges are excluded: they are not compiled into the published
    contract, so a test helper cannot widen the boundary.
    """
    edges: list[str] = []
    for dependency in node.get("deps", []):
        kinds = dependency.get("dep_kinds") or [{"kind": None}]
        if any(kind.get("kind") != "dev" for kind in kinds):
            edges.append(dependency["pkg"])
    return edges


def dependency_closure(metadata: dict[str, Any], package: str) -> dict[str, set[str]]:
    """Map every reachable package name to the names it pulls in.

    Returns an empty mapping when ``package`` is not part of the resolved graph.
    """
    packages = _packages(metadata)
    by_name = {details["name"]: package_id for package_id, details in packages.items()}
    root_id = by_name.get(package)
    if root_id is None:
        return {}

    nodes = {node["id"]: node for node in metadata.get("resolve", {}).get("nodes", [])}
    closure: dict[str, set[str]] = {package: set()}
    queue = deque([root_id])
    seen = {root_id}

    while queue:
        node_id = queue.popleft()
        node = nodes.get(node_id)
        if node is None:
            continue
        current = packages[node_id]["name"]
        for dependency_id in _normal_edges(node):
            dependency = packages.get(dependency_id)
            if dependency is None:
                continue
            name = dependency["name"]
            closure.setdefault(current, set()).add(name)
            closure.setdefault(name, set())
            if dependency_id not in seen:
                seen.add(dependency_id)
                queue.append(dependency_id)

    return closure


def validate_boundary(
    metadata: dict[str, Any],
    package: str = NEUTRAL_PACKAGE,
    allowed: frozenset[str] = ALLOWED_DEPENDENCIES,
) -> list[str]:
    """Return one error per dependency outside the neutral allowlist."""
    closure = dependency_closure(metadata, package)
    if not closure:
        return [f"{package}: package is not part of the resolved workspace"]

    workspace_members = set(metadata.get("workspace_members", []))
    packages = _packages(metadata)
    errors: list[str] = []

    for dependency in sorted(set(closure) - {package}):
        if dependency not in allowed:
            errors.append(
                f"{package}: dependency {dependency} is outside the neutral allowlist"
                " (add it here only if it performs no I/O, networking, terminal,"
                " ecosystem, or package-resolution work)"
            )
        for package_id, details in packages.items():
            if details["name"] == dependency and package_id in workspace_members:
                errors.append(f"{package}: must not depend on workspace crate {dependency}")

    return errors


def metadata_argv(cargo: str, root: Path) -> list[str]:
    """Build the ``cargo metadata`` invocation for the workspace at ``root``.

    ``--locked`` is the point: the boundary is checked against the graph the
    repository actually ships, and a stale lock must fail rather than be repaired
    silently by this check.

    ``--offline`` is deliberately absent. It made this task depend on some earlier
    task having warmed the cargo cache — true when ``gates`` ran the tasks in
    order, false once they overlap, which is exactly how it started failing when a
    dependency was added. Offline operation is already guaranteed where it matters:
    the sandbox replaces crates.io with the vendored tree.
    """
    return [
        cargo,
        "metadata",
        "--locked",
        "--format-version",
        "1",
        "--manifest-path",
        str(root / "Cargo.toml"),
    ]


def load_metadata(root: Path) -> dict[str, Any]:
    """Run ``cargo metadata`` for the workspace at ``root`` against the locked graph."""
    cargo = shutil.which("cargo")
    if cargo is None:
        message = "cargo is not on PATH; run this through `pixi run crate-check`"
        raise SystemExit(f"crate-check: {message}")

    result = subprocess.run(  # noqa: S603 - fixed argv, no shell
        metadata_argv(cargo, root),
        capture_output=True,
        check=False,
        text=True,
    )
    if result.returncode != 0:
        raise SystemExit(f"crate-check: cargo metadata failed:\n{result.stderr.strip()}")
    metadata: dict[str, Any] = json.loads(result.stdout)
    return metadata


def main(argv: list[str] | None = None) -> int:
    """Run the dependency-boundary check from the command line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)

    errors = validate_boundary(load_metadata(args.root))
    if errors:
        for error in errors:
            print(f"crate-check: {error}", file=sys.stderr)
        return 1
    print(f"crate-check: {NEUTRAL_PACKAGE} stays ecosystem, UI, and network free")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
