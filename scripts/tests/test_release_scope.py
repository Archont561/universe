"""The v0.1.0 manifest must publish the three CLIs the owner put in scope."""

import tomllib
from pathlib import Path
from typing import Any, cast

ROOT = Path(__file__).resolve().parents[2]
EXPECTED_PACKAGES = ("universe-airlock", "universe-kit", "universe")
TASKS_BY_PACKAGE = {
    "universe-airlock": "package-airlock",
    "universe-kit": "package-kit",
    "universe": "package-cli",
}
BINARIES_BY_PACKAGE = {
    "universe-airlock": "universe-airlock",
    "universe-kit": "universe-kit",
    "universe": "universe",
}


def _manifest() -> dict[str, Any]:
    return tomllib.loads((ROOT / "universe.toml").read_text(encoding="utf-8"))


def _tasks() -> dict[str, Any]:
    tasks = tomllib.loads((ROOT / "pixi.toml").read_text(encoding="utf-8")).get("tasks", {})
    return cast(dict[str, Any], tasks)


def _publish_lines() -> list[str]:
    workflow = (ROOT / ".github" / "workflows" / "release.yml").read_text(encoding="utf-8")
    return [
        line.strip()
        for line in workflow.splitlines()
        if "pixi run --frozen universe -- publish" in line
    ]


def test_the_manifest_declares_all_three_cli_packages() -> None:
    packages = {package["name"]: package for package in _manifest()["packages"]}
    assert set(packages) == set(EXPECTED_PACKAGES)

    tasks = _tasks()
    for name in EXPECTED_PACKAGES:
        package = packages[name]
        assert package["kind"] == "conda"
        assert package["publish"] == ["prefix-dev", "branch"]
        assert package["release"] is True
        assert package["command"][0:2] == ["pixi", "run"]
        assert package["command"][-1] == TASKS_BY_PACKAGE[name]
        assert TASKS_BY_PACKAGE[name] in tasks
        assert package["smoke"] == [f"{BINARIES_BY_PACKAGE[name]} --version"]

    assert packages["universe"]["depends_on"] == [
        "universe-kit",
        "universe-airlock",
    ]


def test_the_facade_recipe_and_pixi_task_exist() -> None:
    tasks = _tasks()
    assert "package-cli" in tasks
    command = tasks["package-cli"]["cmd"]
    assert "crates/universe-cli/recipe/recipe.yaml" in command
    assert "crates/universe-cli/recipe/variants.yaml" in command
    assert "--test=skip" in command
    assert (ROOT / "crates" / "universe-cli" / "recipe" / "variants.yaml").is_file()

    alint = (ROOT / ".alint.yml").read_text(encoding="utf-8")
    assert "universe-required-cli-recipe" in alint
    assert "universe-required-cli-variants" in alint


def test_the_rendered_workflow_publishes_every_package_to_every_store() -> None:
    lines = _publish_lines()
    for store in ("prefix-dev", "releases"):
        for name in EXPECTED_PACKAGES:
            assert any(
                f"publish-artifacts --store {store} --force --package {name} --json" in line
                for line in lines
            ), f"{store} does not publish {name}: {lines}"

    branch_lines = [line for line in lines if "publish --store branch" in line]
    assert len(branch_lines) == len(EXPECTED_PACKAGES)
    for name in EXPECTED_PACKAGES:
        assert any(f"--package {name} --json" in line for line in branch_lines), (
            f"branch does not publish {name}: {branch_lines}"
        )
    assert all("--force" not in line for line in branch_lines)


def test_the_release_records_name_all_three_packages() -> None:
    notes = (ROOT / "docs" / "releases" / "0.1.0.md").read_text(encoding="utf-8")
    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    decision = (
        ROOT
        / "backlog"
        / "decisions"
        / "decision-132 - v0.1.0-is-a-pre-release-packaging-milestone,-not-a-v1-cut.md"
    ).read_text(encoding="utf-8")

    for document in (notes, changelog):
        assert "universe-airlock" in document
        assert "universe-kit" in document
        assert "umbrella `universe` facade" in document

    assert "## Amendment: the main CLI packages are in scope" in decision
    assert "recorded as a reversal" in decision
