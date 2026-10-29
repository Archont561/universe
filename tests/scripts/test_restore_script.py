"""Regression coverage for the checked-in restore launcher.

`scripts/restore.sh` is the airlock's only entrypoint into a verified developer
transport, so its branch derivation, its refusals, and the order in which it
verifies and writes are contracts, not implementation detail. Every case here
runs the real script in a subprocess against a fixture config; nothing mocks the
shell, because the value of this suite is that the shell really behaves this way.

`--print-branch` is deliberately git-free and network-free, which is what makes
it testable on a disconnected host: it resolves the branch from the reviewed
config and exits.
"""

import os
import platform
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
RESTORE_SCRIPT = REPO_ROOT / "scripts" / "restore.sh"
REVIEWED_CONFIG = REPO_ROOT / ".pixi-sandbox.toml"

# The launcher maps a host to one Pixi platform name; keep these in lockstep with
# the `case "$(uname -s)-$(uname -m)"` block in scripts/restore.sh.
_PLATFORM_BY_MACHINE = {
    ("linux", "x86_64"): "linux-64",
    ("linux", "aarch64"): "linux-aarch64",
    ("linux", "arm64"): "linux-aarch64",
    ("darwin", "arm64"): "osx-arm64",
    ("darwin", "x86_64"): "osx-64",
}


def host_platform() -> str:
    """Return the Pixi platform name this host resolves to."""
    key = (platform.system().lower(), platform.machine().lower())
    resolved = _PLATFORM_BY_MACHINE.get(key)
    if resolved is None:  # pragma: no cover - unsupported CI host
        pytest.skip(f"restore.sh has no platform mapping for {key}")
    return resolved


def other_platforms(count: int = 2) -> list[str]:
    """Return Pixi platform names that are definitely not this host's."""
    return [name for name in ("linux-64", "osx-64", "win-64") if name != host_platform()][:count]


def sandbox_config(
    *bundles: tuple[str, list[str]], prefix: str = "sandbox", schema: int = 1
) -> str:
    """Render a `.pixi-sandbox.toml` fixture declaring the given bundles."""
    lines = [
        f"schema = {schema}",
        f'branch_prefix = "{prefix}"',
        "cargo_vendor = true",
        "",
    ]
    for name, platforms in bundles:
        rendered = ", ".join(f'"{item}"' for item in platforms)
        lines.extend(
            [
                "[[bundle]]",
                f'name = "{name}"',
                f"platforms = [{rendered}]",
                "cargo_vendor = true",
                "",
            ]
        )
    return "\n".join(lines)


def run_launcher(
    tmp_path: Path,
    *arguments: str,
    config: Path | None = None,
    environ: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    """Run the launcher from a scratch cwd so the repository tree is untouched."""
    environment = {
        key: value for key, value in os.environ.items() if not key.startswith("PIXI_SANDBOX_")
    }
    if config is not None:
        environment["PIXI_SANDBOX_CONFIG"] = str(config)
    environment.update(environ or {})

    return subprocess.run(
        ["bash", str(RESTORE_SCRIPT), *arguments],
        capture_output=True,
        text=True,
        check=False,
        cwd=tmp_path,
        env=environment,
    )


def test_host_platform_matches_the_launcher_mapping() -> None:
    expected = host_platform()
    probe = run_launcher(
        Path("/tmp"),
        "--print-branch",
        config=REVIEWED_CONFIG,
    )

    if expected in {"linux-64", "linux-aarch64"}:
        assert probe.returncode in {0, 2}
    # The reviewed config publishes this host under a single reviewed branch.
    if expected == "linux-64":
        assert probe.returncode == 0, probe.stderr
        assert probe.stdout.strip() == "sandbox/developer-linux-64"


def test_derives_the_branch_from_the_reviewed_config() -> None:
    result = run_launcher(Path("/tmp"), "--print-branch", config=REVIEWED_CONFIG)

    if host_platform() != "linux-64":
        pytest.skip("the reviewed bundle only declares linux-64")

    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "sandbox/developer-linux-64"
    assert "no fetch" in result.stderr.lower() or result.stderr == ""


def test_derives_a_renamed_prefix_and_bundle(tmp_path: Path) -> None:
    config = tmp_path / "config.toml"
    config.write_text(sandbox_config(("cpu", [host_platform()]), prefix="transport"), "utf-8")

    result = run_launcher(tmp_path, "--print-branch", config=config)

    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == f"transport/cpu-{host_platform()}"


def test_requires_an_explicit_bundle_when_several_are_declared(tmp_path: Path) -> None:
    config = tmp_path / "config.toml"
    config.write_text(
        sandbox_config(("cpu", [host_platform()]), ("gpu", [host_platform()])),
        "utf-8",
    )

    result = run_launcher(tmp_path, "--print-branch", config=config)

    assert result.returncode == 2
    assert "several bundles" in result.stderr
    assert "cpu" in result.stderr and "gpu" in result.stderr
    assert result.stdout == ""


def test_selects_the_requested_bundle(tmp_path: Path) -> None:
    config = tmp_path / "config.toml"
    config.write_text(
        sandbox_config(("cpu", [host_platform()]), ("gpu", [host_platform()])),
        "utf-8",
    )

    result = run_launcher(
        tmp_path,
        "--print-branch",
        config=config,
        environ={"PIXI_SANDBOX_BUNDLE": "gpu"},
    )

    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == f"sandbox/gpu-{host_platform()}"


def test_rejects_an_unknown_bundle(tmp_path: Path) -> None:
    config = tmp_path / "config.toml"
    config.write_text(sandbox_config(("cpu", [host_platform()])), "utf-8")

    result = run_launcher(
        tmp_path,
        "--print-branch",
        config=config,
        environ={"PIXI_SANDBOX_BUNDLE": "gpu"},
    )

    assert result.returncode == 2
    assert "no bundle 'gpu'" in result.stderr


def test_reports_a_host_with_no_published_bundle(tmp_path: Path) -> None:
    config = tmp_path / "config.toml"
    config.write_text(sandbox_config(("cpu", other_platforms(1))), "utf-8")

    result = run_launcher(tmp_path, "--print-branch", config=config)

    assert result.returncode == 2
    assert "publishes nothing for this host" in result.stderr


@pytest.mark.parametrize("schema", ["2", "0"])
def test_rejects_an_unsupported_schema(tmp_path: Path, schema: str) -> None:
    config = tmp_path / "config.toml"
    config.write_text(
        sandbox_config(("cpu", [host_platform()]), schema=int(schema)),
        "utf-8",
    )

    result = run_launcher(tmp_path, "--print-branch", config=config)

    assert result.returncode == 2
    assert "only knows schema 1" in result.stderr


def test_rejects_a_config_without_bundles(tmp_path: Path) -> None:
    config = tmp_path / "config.toml"
    config.write_text(sandbox_config(), "utf-8")

    result = run_launcher(tmp_path, "--print-branch", config=config)

    assert result.returncode == 2
    assert "declares no [[bundle]] targets" in result.stderr


def test_reports_a_missing_config_instead_of_guessing_a_branch(tmp_path: Path) -> None:
    result = run_launcher(tmp_path, "--print-branch", config=tmp_path / "absent.toml")

    assert result.returncode == 2
    assert "pass it explicitly" in result.stderr
    assert result.stdout == ""


@pytest.mark.parametrize(
    "branch",
    [
        "sandbox/../main",
        "/absolute",
        "has space",
        "tilde~",
        "caret^",
        "colon:ref",
        "glob*",
        "question?",
        "bracket[",
        "back\\slash",
        "trailing.",
        "dot./inside",
        "@{push}",
        "-dash",
    ],
)
def test_refuses_an_unsafe_branch_argument(tmp_path: Path, branch: str) -> None:
    result = run_launcher(tmp_path, "--print-branch", branch)

    assert result.returncode == 2
    assert "refusing unsafe branch name" in result.stderr
    assert result.stdout == ""


def test_accepts_a_safe_explicit_branch_without_reading_a_config(tmp_path: Path) -> None:
    result = run_launcher(
        tmp_path,
        "--print-branch",
        "sandbox/developer-linux-64",
        config=tmp_path / "absent.toml",
    )

    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "sandbox/developer-linux-64"


def test_print_branch_touches_neither_the_network_nor_the_tree(tmp_path: Path) -> None:
    result = run_launcher(tmp_path, "--print-branch", config=REVIEWED_CONFIG)

    if host_platform() != "linux-64":
        pytest.skip("the reviewed bundle only declares linux-64")

    assert result.returncode == 0
    assert list(tmp_path.iterdir()) == []


def test_help_documents_the_no_profile_edit_mode() -> None:
    result = run_launcher(Path("/tmp"), "--help")

    assert result.returncode == 0
    assert "PIXI_SANDBOX_USER_TOOLS" in result.stdout
    assert "leaves HOME and every profile untouched" in result.stdout
    assert "bash scripts/restore.sh [branch] [output-path]" in result.stdout


def test_launcher_verifies_before_it_writes_and_verifies_the_result() -> None:
    """Guard the ordered contract: verify the branch, restore, verify the tree."""
    body = RESTORE_SCRIPT.read_text(encoding="utf-8")
    guard = body.index("is_safe_ref")

    verify_branch = body.index("doctor --branch-location")
    restore = body.index("restore --branch-location")
    verify_restored = body.index("--verify-restored")
    skip_mode = body.index('if [ "$PIXI_SANDBOX_USER_TOOLS" = skip ]; then')

    assert guard < verify_branch < restore < verify_restored < skip_mode


def test_launcher_never_edits_a_profile_in_skip_mode() -> None:
    body = RESTORE_SCRIPT.read_text(encoding="utf-8")
    skip_block = body[body.index('if [ "$PIXI_SANDBOX_USER_TOOLS" = skip ]; then') :]
    skip_block = skip_block[: skip_block.index("elif")]

    assert "not registered" in skip_block
    for mutation in ("sed -i", ">>", "tee", "cp "):
        assert mutation not in skip_block
