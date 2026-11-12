"""The v0.1.0 manifest must publish the four artifacts the owner put in scope.

The first three are the CLIs `decision-132` widened to; the fourth is the Airlock
bootstrapper, which `UNI-55` ships as a separately installable package so a disconnected host
has something to run before the consumer it restores exists.
"""

import tomllib
from pathlib import Path
from typing import Any, cast

ROOT = Path(__file__).resolve().parents[2]
EXPECTED_PACKAGES = (
    "universe-airlock",
    "universe-airlock-bootstrap",
    "universe-kit",
    "universe",
)
TASKS_BY_PACKAGE = {
    "universe-airlock": "package-airlock",
    "universe-airlock-bootstrap": "package-airlock-bootstrap",
    "universe-kit": "package-kit",
    "universe": "package-cli",
}
BINARIES_BY_PACKAGE = {
    "universe-airlock": "universe-airlock",
    "universe-airlock-bootstrap": "universe-airlock-bootstrap",
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

    # A branch store is one orphan snapshot tree. Three separate pushes would each
    # replace the whole tree, so run 43045274207 left `channel` holding only the
    # last package. One command has to name every package.
    branch_lines = [line for line in lines if "publish --store branch" in line]
    assert len(branch_lines) == 1, f"a snapshot store pushes one tree: {branch_lines}"
    for name in EXPECTED_PACKAGES:
        assert f"--package {name}" in branch_lines[0], (
            f"the one snapshot omits {name}: {branch_lines[0]}"
        )
    assert "--force" not in branch_lines[0]


def test_the_three_package_re_watch_evidence_is_recorded() -> None:
    """Independent source: run 43045274207's build audit, release assets and journal."""
    notes = (
        ROOT
        / "backlog"
        / "tasks"
        / "uni-59 - Cut-the-v0.1.0-pre-release-from-the-rendered-workflow.md"
    ).read_text(encoding="utf-8")
    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    decision = (
        ROOT
        / "backlog"
        / "decisions"
        / "decision-132 - v0.1.0-is-a-pre-release-packaging-milestone,-not-a-v1-cut.md"
    ).read_text(encoding="utf-8")

    for document in (notes, changelog, decision):
        assert "43045274207" in document, "the green three-package re-watch is missing"
    for digest in (
        "2e98b81d2155f5033bef5001dae2c6a3dbf5037d97e32e182f4af7e050bfab26",
        "763999409faee8b9972e7935d8e4f8d49599c4004e7eea8f80b5541304392c0b",
        "19e00dffc113f7d6d26e139b8d7cca3c72a995c20846f709b8aa4dca790f7217",
    ):
        assert digest in notes, f"the audit digest {digest} is not recorded"
    assert "1d30ef21701497dd196eeaa1599d7ec3ee24edc6" in notes, (
        "the one-package branch outcome is not recorded"
    )
    assert "87457" in notes, "the fresh prefix.dev read-back is not recorded"
    assert "one command" in notes, "the branch snapshot fix is not recorded"


def test_the_force_replacement_re_watch_evidence_is_recorded() -> None:
    """Independent source: run 43061441347's audit, repodata, release assets and branch tree."""
    notes = (
        ROOT
        / "backlog"
        / "tasks"
        / "uni-59 - Cut-the-v0.1.0-pre-release-from-the-rendered-workflow.md"
    ).read_text(encoding="utf-8")
    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    decision = (
        ROOT
        / "backlog"
        / "decisions"
        / "decision-132 - v0.1.0-is-a-pre-release-packaging-milestone,-not-a-v1-cut.md"
    ).read_text(encoding="utf-8")
    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    workflows = (ROOT / "docs" / "WORKFLOWS.md").read_text(encoding="utf-8")

    for document in (notes, changelog, decision, readme, workflows):
        assert "43061441347" in document, "the corrected re-watch is missing"
    for digest in (
        "134f5e7dfd2d249f08195a2678e7ff6200f54b53fe04bc95c3229ff1ca8e651d",
        "63b958b773573e158ff30bdd682a0669311d58058a790c2783ae1c90902e9c12",
        "470f52e41e4dfaae20ce22d893cf1e098b2784c123ae623cdacab07c84667e69",
    ):
        assert digest in notes, f"the audit digest {digest} is not recorded"
    assert "7afdff10e32c28d368caaf07acc18a118d81ea74" in notes, (
        "the three-package branch snapshot is not recorded"
    )
    assert "87896" in notes, "the fresh prefix.dev read-back is not recorded"
    assert "read-write-delete" in notes, "the force-replace outcome is not recorded"
    assert "offline" in notes, "the still-open offline solve is not recorded"


def test_the_tag_run_evidence_is_recorded() -> None:
    """Independent source: run 43073492712's build audit, release assets and branch tree."""
    notes = (
        ROOT
        / "backlog"
        / "tasks"
        / "uni-59 - Cut-the-v0.1.0-pre-release-from-the-rendered-workflow.md"
    ).read_text(encoding="utf-8")
    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    decision = (
        ROOT
        / "backlog"
        / "decisions"
        / "decision-132 - v0.1.0-is-a-pre-release-packaging-milestone,-not-a-v1-cut.md"
    ).read_text(encoding="utf-8")
    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    workflows = (ROOT / "docs" / "WORKFLOWS.md").read_text(encoding="utf-8")

    for document in (notes, changelog, decision, readme, workflows):
        assert "43073492712" in document, "the tag run is missing"
    for digest in (
        "7153aedcc8188a044524f6bc84639188e7d62491caf74d9630a4fcbd5859a3ab",
        "314d6941c8a1776f04954a117ea6d91cca3d97971ca6203e68d1c5a11d6233b2",
        "c4e1fc2792135a803fb3ec9a772d2fd9e2a529d323ccb792d30f7ef1636f9cec",
    ):
        assert digest in notes, f"the tag-run digest {digest} is not recorded"
    assert "eb532021ccc386c875c720e6b67138b4b30c1152" in notes, (
        "the tag-run branch snapshot is not recorded"
    )
    assert "88268" in notes, "the tag-run fresh prefix.dev read is not recorded"
    assert "- [x] #2" in notes, "criterion 2 is not ticked after the tag was cut"
    assert "https://github.com/Archont561/universe/actions/runs/43073492712" in notes
    assert "No `v0.1.0` tag is cut" not in readme, "README still claims no tag"
    notes_doc = (ROOT / "docs" / "releases" / "0.1.0.md").read_text(encoding="utf-8")
    assert "no restore entrypoint is committed at this tag" in notes_doc


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


def test_the_rebuilt_channel_is_published_and_solves_offline() -> None:
    """Independent source: run 43230051708's publish jobs and the prefix.dev read-back."""
    notes = (
        ROOT
        / "backlog"
        / "tasks"
        / "uni-59 - Cut-the-v0.1.0-pre-release-from-the-rendered-workflow.md"
    ).read_text(encoding="utf-8")
    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    decision = (
        ROOT
        / "backlog"
        / "decisions"
        / "decision-132 - v0.1.0-is-a-pre-release-packaging-milestone,-not-a-v1-cut.md"
    ).read_text(encoding="utf-8")
    workflows = (ROOT / "docs" / "WORKFLOWS.md").read_text(encoding="utf-8")
    notes_doc = (ROOT / "docs" / "releases" / "0.1.0.md").read_text(encoding="utf-8")

    for document in (notes, changelog, decision, workflows, notes_doc):
        assert "43230051708" in document, "the republish run is missing"
        assert "93323" in document, "the fresh prefix.dev read after the rebuild is missing"
    for digest in (
        "359c34535c59887a94d064301ab88cdc6c651f752674b11e43d339870116c66b",
        "548e832c34a6cfef92e5ac6fba73af805a9095ca06cea969de3a571074ad2f14",
        "29f258fc57b820fe0db9346f11fa47709b1dcc7750c08fa7fca710b4b6131906",
    ):
        assert digest in notes, f"the rebuilt digest {digest} is not recorded"
    assert "- [x] #4" in notes, "criterion 4 is not ticked after the rebuild published"
    assert "The default environment has been installed." in notes, (
        "the offline solve transcript lines are not recorded"
    )
    assert "transcript" in notes_doc


def test_the_bootstrap_is_a_separately_installable_package() -> None:
    """Independent source: decision-115 separates bootstrap readiness from sealed readiness.

    The bootstrapper runs before anything it restores exists, so it stays its own artifact: it
    is published like the others, and neither it nor the facade may depend on the other.
    """
    packages = {package["name"]: package for package in _manifest()["packages"]}
    bootstrap = packages["universe-airlock-bootstrap"]

    assert bootstrap["kind"] == "conda"
    assert bootstrap["platforms"] == ["linux-64"]
    assert bootstrap["outputs"] == ["output/universe-airlock-bootstrap/linux-64/*.conda"]
    assert bootstrap["publish"] == ["prefix-dev", "branch"]
    assert bootstrap["release"] is True
    assert "depends_on" not in bootstrap, (
        "the bootstrapper restores the consumer, so it cannot depend on it"
    )
    assert "universe-airlock-bootstrap" not in packages["universe"]["depends_on"], (
        "the facade must not pull in a binary that runs before any environment exists"
    )

    lines = _publish_lines()
    for store in ("prefix-dev", "releases"):
        assert any(
            f"publish-artifacts --store {store} --force --package universe-airlock-bootstrap"
            in line
            for line in lines
        ), f"{store} does not publish the bootstrapper: {lines}"
    branch_lines = [line for line in lines if "publish --store branch" in line]
    assert len(branch_lines) == 1
    assert "--package universe-airlock-bootstrap" in branch_lines[0]


def test_the_consumer_docs_and_the_asset_list_agree_on_the_bootstrapper() -> None:
    """Independent source: the recipe, the Pixi task and the manifest entry all exist."""
    recipe = ROOT / "crates" / "universe-airlock-bootstrap" / "recipe" / "recipe.yaml"
    variants = ROOT / "crates" / "universe-airlock-bootstrap" / "recipe" / "variants.yaml"
    assert recipe.is_file() and variants.is_file()

    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    notes = (ROOT / "docs" / "releases" / "0.1.0.md").read_text(encoding="utf-8")

    for document in (readme, notes):
        assert "universe-airlock-bootstrap" in document
        assert "separately installable" in document.lower()
    assert "no runtime dependency" in notes.lower()


def test_the_bootstrap_release_evidence_is_recorded() -> None:
    """Independent source: run 43252390113's build audit and the three destinations."""
    task = (
        ROOT / "backlog" / "tasks" / "uni-55 - Package-and-ship-the-Airlock-bootstrap-artifact.md"
    ).read_text(encoding="utf-8")
    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")

    for document in (task, changelog):
        assert "43252390113" in document, "the bootstrap publish run is missing"

    digest = "1fb284e59d46fd54a97cae2c2e61c72afbb64e24011d06e9436acb5dc1a88861"
    assert len(digest) == 64
    for document in (task, changelog):
        assert digest in document, "the bootstrapper's published digest is not recorded"
    assert "298467" in task, "the bootstrapper's published size is not recorded"
    assert "5424f2a40a5efb5f9067deb90877848a222c6084" in task, (
        "the four-package branch snapshot is not recorded"
    )
    assert "93807" in task, "the fresh prefix.dev read is not recorded"
    assert "- [x] #2" in task, "criterion 2 is not ticked after the clean-environment smoke"
    assert "- [x] #3" in task, "criterion 3 is not ticked after the publish read-back"
    assert "universe-airlock-bootstrap 0.1.0" in task, (
        "the clean-environment smoke output is not recorded"
    )
