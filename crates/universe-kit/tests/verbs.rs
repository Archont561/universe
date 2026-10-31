//! The producer verbs, exercised through their command surface.
//!
//! Each test builds a throwaway project in a temporary directory and talks to `run`, which
//! is the same entry point the binary uses. Nothing here reaches the network, and nothing
//! writes outside the directory it owns.

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;
use universe_core::CommandOutput;

const MANIFEST: &str = r#"
schema = 1

[project]
name = "example"
repository = "https://github.com/Archont561/example"
platforms = ["linux-64"]

[[packages]]
name = "example"
kind = "conda"
command = ["sh", "-c", "mkdir -p out && printf payload > out/example-0.1.0-0.conda"]
outputs = ["out/*.conda"]
smoke = ["example --version"]
"#;

fn project(manifest: &str) -> TempDir {
    let directory = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(directory.path().join("universe.toml"), manifest).expect("writes the manifest");
    std::fs::write(
        directory.path().join("Cargo.toml"),
        "[workspace]\n\n[workspace.package]\nversion = \"0.1.0\"\n",
    )
    .expect("writes the workspace manifest");
    directory
}

fn run(root: &Path, arguments: &[&str]) -> CommandOutput {
    let mut args = arguments
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    args.push("--root".to_owned());
    args.push(root.display().to_string());
    universe_kit::run(args)
}

fn json(output: &CommandOutput) -> serde_json::Value {
    serde_json::from_str(output.stdout()).expect("the verb emits one JSON envelope")
}

#[test]
fn plan_reports_the_levels_and_the_runner_matrix() {
    let directory = project(MANIFEST);
    let output = run(directory.path(), &["plan", "--json"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let envelope = json(&output);
    assert_eq!(envelope["command"], "plan");
    assert_eq!(envelope["result"]["has_work"], true);
    assert_eq!(
        envelope["result"]["levels"][0]["jobs"][0]["package"],
        "example"
    );
    assert_eq!(
        envelope["result"]["levels"][0]["jobs"][0]["platforms"][0],
        "linux-64"
    );
}

#[test]
fn plan_refuses_an_invalid_manifest_rather_than_planning_it() {
    // A released package that cannot prove it installs is refused before anything is planned.
    let manifest = MANIFEST.replace("smoke = [\"example --version\"]\n", "");
    let directory = project(&manifest);
    let output = run(directory.path(), &["plan", "--json"]);

    assert_eq!(output.exit_code(), 2);
    assert!(output.stdout().is_empty(), "an error never lands on stdout");
    assert!(output.stderr().contains("smoke"));
}

#[test]
fn a_platform_the_project_does_not_build_leaves_nothing_to_plan() {
    let directory = project(MANIFEST);
    let output = run(
        directory.path(),
        &["plan", "--json", "--platform", "osx-arm64"],
    );

    assert_eq!(output.exit_code(), 0);
    assert_eq!(json(&output)["result"]["has_work"], false);
}

#[test]
fn lint_accepts_a_manifest_whose_version_matches_the_recipe() {
    let directory = project(MANIFEST);
    let recipe = directory.path().join("crates/example/recipe");
    std::fs::create_dir_all(&recipe).expect("creates the recipe directory");
    std::fs::write(
        recipe.join("recipe.yaml"),
        "context:\n  version: \"0.1.0\"\n\npackage:\n  version: ${{ version }}\n",
    )
    .expect("writes the recipe");

    let output = run(directory.path(), &["lint", "--json"]);
    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert_eq!(json(&output)["command"], "lint");
}

#[test]
fn lint_refuses_a_recipe_that_disagrees_with_the_workspace_version() {
    let directory = project(MANIFEST);
    let recipe = directory.path().join("crates/example/recipe");
    std::fs::create_dir_all(&recipe).expect("creates the recipe directory");
    std::fs::write(
        recipe.join("recipe.yaml"),
        "context:\n  version: \"0.0.9\"\n",
    )
    .expect("writes the recipe");

    let output = run(directory.path(), &["lint", "--json"]);
    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("lint.version.recipe"),
        "{}",
        output.stderr()
    );
}

#[test]
fn lint_reports_warnings_without_failing() {
    // A writable store no package names is a warning, which is information, not a refusal.
    let manifest =
        format!("{MANIFEST}\n[[stores]]\nname = \"spare\"\nkind = \"static\"\nwritable = true\n");
    let directory = project(&manifest);
    let output = run(directory.path(), &["lint", "--json"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let warnings = json(&output)["result"]["warnings"]
        .as_array()
        .expect("warnings are a list")
        .iter()
        .filter_map(|warning| warning["code"].as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    assert!(
        warnings.iter().any(|code| code == "manifest.store.dead"),
        "the unused store must be reported, got {warnings:?}"
    );
}

#[test]
fn build_runs_the_declared_command_and_collects_its_outputs() {
    let directory = project(MANIFEST);
    let output = run(directory.path(), &["build", "--json"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let built = &json(&output)["result"]["built"];
    assert_eq!(built[0]["package"], "example");
    assert_eq!(built[0]["outputs"][0], "out/example-0.1.0-0.conda");
    assert!(directory.path().join("out/example-0.1.0-0.conda").is_file());
}

#[test]
fn a_failing_command_is_reported_with_its_own_exit_status() {
    let manifest = MANIFEST.replace(
        r#"command = ["sh", "-c", "mkdir -p out && printf payload > out/example-0.1.0-0.conda"]"#,
        r#"command = ["sh", "-c", "echo broken >&2; exit 3"]"#,
    );
    let directory = project(&manifest);
    let output = run(directory.path(), &["build", "--json"]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("build.command.failed"),
        "{}",
        output.stderr()
    );
    assert!(output.stderr().contains("broken"), "{}", output.stderr());
    assert!(output.stdout().is_empty());
}

#[test]
fn a_command_that_produces_nothing_it_promised_is_refused() {
    let manifest = MANIFEST.replace(
        r#"command = ["sh", "-c", "mkdir -p out && printf payload > out/example-0.1.0-0.conda"]"#,
        r#"command = ["sh", "-c", "true"]"#,
    );
    let directory = project(&manifest);
    let output = run(directory.path(), &["build", "--json"]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("build.output.missing"),
        "{}",
        output.stderr()
    );
}

#[test]
fn build_passes_only_the_declared_environment() {
    // The child writes down what it can see. `HOME` is set in every environment this test
    // runs in, so seeing `absent` there is the assertion that the parent's environment was
    // cleared rather than inherited.
    let manifest = MANIFEST
        .replace(
            r#"command = ["sh", "-c", "mkdir -p out && printf payload > out/example-0.1.0-0.conda"]"#,
            r#"command = ["sh", "-c", "mkdir -p out && printf '%s:%s' \"$GREETING\" \"${HOME:-absent}\" > out/example-0.1.0-0.conda"]"#,
        )
        .replace(
            r#"outputs = ["out/*.conda"]"#,
            "env = { GREETING = \"hello\" }\noutputs = [\"out/*.conda\"]",
        );
    let directory = project(&manifest);

    let output = run(directory.path(), &["build", "--json"]);
    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert_eq!(
        std::fs::read_to_string(directory.path().join("out/example-0.1.0-0.conda"))
            .expect("the artifact exists"),
        "hello:absent"
    );
}

#[test]
fn audit_measures_the_artifact_and_names_its_digest() {
    let directory = project(MANIFEST);
    assert_eq!(run(directory.path(), &["build", "--json"]).exit_code(), 0);

    let output = run(directory.path(), &["audit", "--json"]);
    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let envelope = json(&output);
    assert_eq!(envelope["result"]["artifacts"][0]["size_bytes"], 7);
    assert_eq!(envelope["artifacts"][0]["id"]["name"], "example");
    assert_eq!(envelope["artifacts"][0]["id"]["version"], "0.1.0");
    assert_eq!(
        envelope["artifacts"][0]["digest"],
        "239f59ed55e737c77147cf55ad0c1b030b6d7ee748a7426952f9b852d5a935e5"
    );
}

#[test]
fn audit_refuses_a_released_package_that_built_nothing() {
    let directory = project(MANIFEST);
    let output = run(directory.path(), &["audit", "--json"]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("audit.output.missing"),
        "{}",
        output.stderr()
    );
}

#[test]
fn an_unknown_argument_lists_the_verbs_options() {
    let directory = project(MANIFEST);
    let output = run(directory.path(), &["plan", "--verbose"]);

    assert_eq!(output.exit_code(), 2);
    assert!(output.stderr().contains("usage: universe plan"));
}

#[test]
fn the_human_forms_say_something_a_person_can_read() {
    let directory = project(MANIFEST);
    let planned = run(directory.path(), &["plan"]);
    assert_eq!(planned.exit_code(), 0);
    assert_eq!(planned.stdout(), "level 0: example (linux-64)\n");

    let linted = run(directory.path(), &["lint"]);
    assert_eq!(linted.exit_code(), 0, "stderr: {}", linted.stderr());
    assert!(linted.stdout().ends_with("ok\n"));
}

#[test]
fn a_missing_manifest_is_a_usage_failure_naming_the_path() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let output = run(directory.path(), &["plan", "--json"]);

    assert_eq!(output.exit_code(), 2);
    assert!(output.stderr().contains("universe.toml"));
}

#[test]
fn an_artifact_named_after_a_different_package_is_refused() {
    let manifest = MANIFEST.replace(
        r#"printf payload > out/example-0.1.0-0.conda"#,
        r#"printf payload > out/other-0.1.0-0.conda"#,
    );
    let directory = project(&manifest);
    assert_eq!(run(directory.path(), &["build", "--json"]).exit_code(), 0);

    let output = run(directory.path(), &["audit", "--json"]);
    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("audit.artifact.identity"),
        "{}",
        output.stderr()
    );
}

#[test]
fn the_verbs_do_not_run_a_shell_of_their_own() {
    // A command is argv, so a package that names a program that does not exist is reported
    // as unrunnable rather than being handed to `sh`.
    let manifest = MANIFEST.replace(
        r#"command = ["sh", "-c", "mkdir -p out && printf payload > out/example-0.1.0-0.conda"]"#,
        r#"command = ["definitely-not-a-program"]"#,
    );
    let directory = project(&manifest);
    let output = Command::new("true")
        .status()
        .expect("the harness can run a process");
    assert!(output.success());

    let built = run(directory.path(), &["build", "--json"]);
    assert_eq!(built.exit_code(), 1);
    assert!(
        built.stderr().contains("build.command.unrunnable"),
        "{}",
        built.stderr()
    );
}
