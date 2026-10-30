"""Every workflow shape the renderer can emit has to be a valid Actions file.

The renderer has its own unit and scenario tests, but they assert what it *decided*; these
assert what a runner would *do* with the result. A job whose `runs-on:` is empty, or a step
that reads the work tree before the checkout, is a workflow that renders without complaint
and fails on the first run — which is the failure mode this file exists to move left.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[1]

HEADER = """
schema = 1

[project]
name = "example"
repository = "https://github.com/Archont561/example"
platforms = ["linux-64"]
"""


# A package facet ends at the next table header, so everything a package owns — including
# `publish` — has to be written before the first `[[stores]]`.
def _package(publish: list[str]) -> str:
    quoted = ", ".join(f'"{name}"' for name in publish)
    return f"""
[[packages]]
name = "example-airlock"
kind = "conda"
command = ["pixi", "run", "--frozen", "package-airlock"]
platforms = ["linux-64"]
outputs = ["output/example/linux-64/*.conda"]
smoke = ["example-airlock --version"]
publish = [{quoted}]
"""


HOSTED_STORE = """
[[stores]]
name = "conda-prefix"
kind = "prefix-dev"
channel = "someone/channel"
writable = true
auth = "oidc"
verifier = "registry"
"""

BRANCH_STORE = """
[[stores]]
name = "conda-branch"
kind = "branch"
writable = true
needs_index = true
auth = "token"
secrets = ["CHANNEL_TOKEN"]
verifier = "file"
"""

HOSTED = HEADER + _package(["conda-prefix"]) + HOSTED_STORE

BRANCH = HEADER + _package(["conda-branch"]) + BRANCH_STORE

MULTI = HEADER + _package(["conda-prefix", "conda-branch"]) + HOSTED_STORE + BRANCH_STORE

NOTHING_PUBLISHED = HEADER + _package([]).replace("publish = []", "publish = []\nrelease = false")

AIRLOCK = (
    HEADER
    + _package([]).replace("publish = []", "publish = []\nrelease = false")
    + """
[airlock.developer]
adapters = ["pixi"]
mirror = "mirror"

[airlock.delivery]
adapters = ["pixi"]
catalog = "github:someone/repo@0123456"
"""
)

VARIANTS = {
    "hosted-channel-only": (HOSTED, {"universe.yml", "universe-publish.yml"}),
    "branch-channel-only": (BRANCH, {"universe.yml", "universe-publish.yml"}),
    "both-channels": (MULTI, {"universe.yml", "universe-publish.yml"}),
    "nothing-published": (NOTHING_PUBLISHED, {"universe.yml"}),
    "lock-adapters": (AIRLOCK, {"universe.yml", "universe-airlock.yml"}),
}


def _require(program: str) -> str:
    found = shutil.which(program)
    if found is None:
        pytest.fail(f"{program} is not on PATH; these tests run under `pixi run test`")
    return found


def _universe(*arguments: str) -> subprocess.CompletedProcess[str]:
    """Run the component binary. Never the facade: rendering must not depend on it."""
    return subprocess.run(
        [
            _require("cargo"),
            "run",
            "-q",
            "--locked",
            "--package",
            "universe-kit",
            "--bin",
            "universe-kit",
            "--",
            *arguments,
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


def _render(root: Path, manifest: str) -> None:
    root.mkdir(parents=True, exist_ok=True)
    (root / "universe.toml").write_text(manifest, encoding="utf-8")
    rendered = _universe("ci", "render", "--root", str(root))
    assert rendered.returncode == 0, rendered.stderr


@pytest.mark.parametrize("variant", sorted(VARIANTS))
def test_every_rendered_variant_passes_actionlint(variant: str, tmp_path: Path) -> None:
    manifest, expected_files = VARIANTS[variant]
    root = tmp_path / variant
    _render(root, manifest)

    emitted = sorted(path.name for path in (root / ".github" / "workflows").iterdir())
    assert set(emitted) == expected_files

    linted = subprocess.run(
        [_require("actionlint"), *emitted],
        cwd=root / ".github" / "workflows",
        capture_output=True,
        text=True,
        check=False,
    )
    assert linted.returncode == 0, f"{variant} does not lint:\n{linted.stdout}{linted.stderr}"


@pytest.mark.parametrize("variant", sorted(VARIANTS))
def test_a_rendered_variant_is_accepted_by_check(variant: str, tmp_path: Path) -> None:
    manifest, _ = VARIANTS[variant]
    root = tmp_path / variant
    _render(root, manifest)

    checked = _universe("ci", "check", "--root", str(root))

    assert checked.returncode == 0, checked.stderr
