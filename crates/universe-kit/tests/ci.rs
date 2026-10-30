//! The `ci render` / `ci check` verbs, exercised through the CLI's public entry point.
//!
//! Every test builds its own project in a temporary directory, because a renderer tested
//! against this repository's own manifest can pass while being unable to render anything
//! else.

use std::path::Path;

use tempfile::TempDir;
use universe_kit::run;

const MANIFEST: &str = r#"
schema = 1

[project]
name = "fixture-project"
platforms = ["linux-64"]

[[packages]]
name = "fixture-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["fixture-airlock --version"]
publish = ["conda-prefix"]

[[stores]]
name = "conda-prefix"
kind = "prefix-dev"
channel = "someone/channel"
writable = true
auth = "oidc"
verifier = "registry"
"#;

/// A project directory holding one manifest.
fn project(manifest: &str) -> TempDir {
    let directory = tempfile::tempdir().expect("temp dir");
    std::fs::write(directory.path().join("universe.toml"), manifest).expect("manifest");
    directory
}

/// `universe ci <args…> --root <project>`: the verb comes first, like the command line.
fn output_of(root: &Path, args: &[&str]) -> universe_core::CommandOutput {
    let mut invocation = vec!["ci"];
    invocation.extend_from_slice(args);
    invocation.extend_from_slice(&["--root", root.to_str().expect("utf-8 root")]);
    run(invocation)
}

#[test]
fn render_writes_the_file_set_the_manifest_calls_for() {
    let project = project(MANIFEST);
    let output = output_of(project.path(), &["render"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let workflows = project.path().join(".github/workflows");
    assert!(workflows.join("universe.yml").is_file());
    assert!(workflows.join("universe-publish.yml").is_file());
    // Nothing declared a lock adapter, so the mirror file must not appear.
    assert!(!workflows.join("universe-airlock.yml").exists());
}

#[test]
fn render_is_idempotent_so_a_no_op_never_manufactures_a_diff() {
    let project = project(MANIFEST);
    assert_eq!(output_of(project.path(), &["render"]).exit_code(), 0);
    let first = std::fs::read_to_string(project.path().join(".github/workflows/universe.yml"))
        .expect("first render");
    assert_eq!(output_of(project.path(), &["render"]).exit_code(), 0);
    let second = std::fs::read_to_string(project.path().join(".github/workflows/universe.yml"))
        .expect("second render");
    assert_eq!(first, second);
}

#[test]
fn stdout_renders_without_writing_anything() {
    let project = project(MANIFEST);
    let output = output_of(project.path(), &["render", "--stdout"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert!(output.stdout().contains("name: universe"));
    assert!(
        !project.path().join(".github/workflows").exists(),
        "--stdout must not touch the working tree"
    );
}

#[test]
fn render_prints_the_required_checks_in_settings_order() {
    let project = project(MANIFEST);
    let output = output_of(project.path(), &["render", "--print-required-checks"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let lines: Vec<&str> = output.stdout().lines().collect();
    assert_eq!(lines.first(), Some(&"universe / view"));
    assert_eq!(lines.last(), Some(&"universe / verdict"));
    assert!(!lines.iter().any(|line| line.starts_with("publish /")));
}

#[test]
fn check_passes_immediately_after_render() {
    let project = project(MANIFEST);
    assert_eq!(output_of(project.path(), &["render"]).exit_code(), 0);
    let output = output_of(project.path(), &["check"]);
    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
}

#[test]
fn check_fails_on_a_hand_edited_workflow_and_names_the_file() {
    let project = project(MANIFEST);
    assert_eq!(output_of(project.path(), &["render"]).exit_code(), 0);
    std::fs::write(
        project.path().join(".github/workflows/universe.yml"),
        "# somebody edited the generated file\n",
    )
    .expect("edit");

    let output = output_of(project.path(), &["check"]);
    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("universe.yml"),
        "stderr must name the drifted file: {}",
        output.stderr()
    );
}

#[test]
fn check_fails_when_a_declared_file_was_never_rendered() {
    let project = project(MANIFEST);
    let output = output_of(project.path(), &["check"]);
    assert_eq!(output.exit_code(), 1);
    assert!(output.stderr().contains("universe.yml"));
}

#[test]
fn a_manifest_that_does_not_parse_is_a_usage_error_naming_the_problem() {
    let project = project("schema = [oops\n");
    let output = output_of(project.path(), &["render"]);

    assert_eq!(output.exit_code(), 2);
    assert!(output.stdout().is_empty(), "stdout is data, not errors");
    assert!(output.stderr().contains("manifest.toml.invalid"));
}

#[test]
fn a_missing_manifest_is_reported_rather_than_treated_as_empty() {
    let directory = tempfile::tempdir().expect("temp dir");
    let output = output_of(directory.path(), &["render"]);

    assert_eq!(output.exit_code(), 2);
    assert!(output.stderr().contains("universe.toml"));
}

#[test]
fn json_mode_emits_one_envelope_and_keeps_errors_off_stdout() {
    let project = project(MANIFEST);
    let output = output_of(project.path(), &["render", "--json"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert!(output.stdout().trim_start().starts_with('{'));
    assert!(output.stdout().contains("\"command\":\"ci render\""));
    assert!(output.stdout().contains("universe.yml"));
}

#[test]
fn json_mode_reports_the_store_to_package_mapping_it_rendered() {
    let project = project(MANIFEST);
    let output = output_of(project.path(), &["render", "--json"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    // The mapping is a derived view, so it is printed rather than left to be inferred
    // from the YAML a reader has to parse.
    assert!(
        output.stdout().contains(
            "\"destinations\":[{\"packages\":[\"fixture-airlock\"],\"store\":\"conda-prefix\"}]"
        ),
        "stdout: {}",
        output.stdout()
    );
}

#[test]
fn an_unknown_ci_verb_is_refused_with_usage() {
    let project = project(MANIFEST);
    let output = output_of(project.path(), &["frobnicate"]);

    assert_eq!(output.exit_code(), 2);
    assert!(output.stderr().contains("render"));
    assert!(output.stderr().contains("check"));
}
