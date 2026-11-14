//! The `ci render` / `ci check` verbs, exercised through the CLI's public entry point.
//!
//! Every test builds its own project in a temporary directory, because a renderer tested
//! against this repository's own manifest can pass while being unable to render anything
//! else.

use std::path::Path;

use tempfile::TempDir;
use universe_kit::run;
use universe_workflow::{LEGACY_PUBLISH_FILE, PUBLISH_FILE};

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

/// A manifest with no package lane and no publication: an Airlock-only consumer.
const AIRLOCK_ONLY: &str = r#"
schema = 1

[project]
name = "fixture-project"
platforms = ["linux-64"]

[airlock.developer]
adapters = ["pixi"]

[airlock.developer.restore]
remote = "https://example.invalid/universe.git"
branch = "universe/bootstrap"
commit = "0123456789abcdef0123456789abcdef01234567"
"#;

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
    assert!(workflows.join(PUBLISH_FILE).is_file());
    // A rename must not leave the old tag-triggered file behind.
    assert!(!workflows.join(LEGACY_PUBLISH_FILE).exists());
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

/// The reproducer from GitHub issue 120, at the seam a consumer meets it.
///
/// `release.yml` is a name this renderer uses and so does every npm trusted-publishing lane.
/// A consumer that declares no publish lane keeps its own file, is told that it was kept, and —
/// because nothing about that file is this project's business — `ci check` stays quiet.
#[test]
fn a_foreign_release_workflow_survives_a_render_and_is_reported() {
    let project = project(AIRLOCK_ONLY);
    let workflows = project.path().join(".github/workflows");
    std::fs::create_dir_all(&workflows).expect("workflows dir");
    let theirs = "name: npm publish\non:\n  push:\n    tags: [\"v*\"]\n";
    std::fs::write(workflows.join(PUBLISH_FILE), theirs).expect("their file");

    let render = output_of(project.path(), &["render"]);

    assert_eq!(render.exit_code(), 0, "stderr: {}", render.stderr());
    assert_eq!(
        std::fs::read_to_string(workflows.join(PUBLISH_FILE)).expect("still there"),
        theirs,
        "ci render deleted a workflow it did not write"
    );
    // Reported, not silent: the file is named, and so is the reason it was left alone.
    assert!(
        render.stderr().contains(PUBLISH_FILE),
        "{}",
        render.stderr()
    );
    assert!(render.stderr().contains("stamp"), "{}", render.stderr());

    let check = output_of(project.path(), &["check"]);
    assert_eq!(check.exit_code(), 0, "stderr: {}", check.stderr());
}

/// The other half: when the manifest does render that name, the run refuses rather than
/// replacing somebody else's workflow, and says which file is in the way.
#[test]
fn a_foreign_release_workflow_is_not_overwritten_and_the_collision_is_named() {
    let project = project(MANIFEST);
    let workflows = project.path().join(".github/workflows");
    std::fs::create_dir_all(&workflows).expect("workflows dir");
    let theirs = "name: npm publish\n";
    std::fs::write(workflows.join(PUBLISH_FILE), theirs).expect("their file");

    let render = output_of(project.path(), &["render"]);

    assert_eq!(render.exit_code(), 2, "stdout: {}", render.stdout());
    assert!(
        render.stderr().contains(PUBLISH_FILE),
        "{}",
        render.stderr()
    );
    assert_eq!(
        std::fs::read_to_string(workflows.join(PUBLISH_FILE)).expect("still there"),
        theirs,
        "ci render overwrote a workflow it did not write"
    );
    // Nothing else was written either: a refusal leaves the tree as it found it.
    assert!(
        !workflows.join("universe.yml").exists(),
        "a refused render left a partial tree behind"
    );
}

/// Each document in a `--stdout` render is separated, so a script can split the stream.
///
/// Without a separator the two YAML documents run into one another and the only way to find
/// the boundary is to search for the generator's stamp — guessing at a format rather than
/// reading one (GitHub issue 124).
#[test]
fn stdout_separates_the_documents_it_renders() {
    let project = project(MANIFEST);
    let output = output_of(project.path(), &["render", "--stdout"]);
    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());

    let documents: Vec<&str> = output.stdout().split("---\n").collect();
    // The separator goes *between* documents, so N files split into N pieces — no empty
    // leading piece to discard and no trailing separator to strip.
    assert_eq!(documents.len(), 2, "stdout: {}", output.stdout());
    for (index, document) in documents.iter().enumerate() {
        assert!(
            document.starts_with("# @generated by universe"),
            "document {index} is not a whole rendered file: {document}"
        );
    }
    // The pieces are the files, byte for byte: rendering them separately produces the same.
    let written = output_of(project.path(), &["render"]);
    assert_eq!(written.exit_code(), 0);
    let universe = std::fs::read_to_string(project.path().join(".github/workflows/universe.yml"))
        .expect("file");
    let publish = std::fs::read_to_string(project.path().join(".github/workflows/release.yml"))
        .expect("file");
    assert_eq!(documents[0], universe);
    assert_eq!(documents[1], publish);
}

/// One document needs no separator: there is nothing to separate it from.
#[test]
fn a_single_document_is_rendered_without_a_separator() {
    let project = project(AIRLOCK_ONLY);
    let output = output_of(project.path(), &["render", "--stdout"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert!(
        output.stdout().starts_with("# @generated by universe"),
        "stdout: {}",
        output.stdout()
    );
    assert!(
        !output.stdout().contains("---\n"),
        "a lone document was given a separator: {}",
        output.stdout()
    );
}
