//! The producer verbs, exercised through their command surface.
//!
//! Each test builds a throwaway project in a temporary directory and talks to `run`, which is
//! the same entry point the binary uses. Nothing here reaches the network, and nothing writes
//! outside the directory it owns.
//!
//! Two fixtures rather than one: a generic package, for the command-runner behaviour that is
//! true of every ecosystem, and a Conda package carrying a real `.conda`, for the archive
//! pass. A test that wants the second says so in its name.

mod support;

use std::path::Path;

use tempfile::TempDir;
use universe_core::CommandOutput;

/// A package whose artifact is whatever its command writes. `cargo` keeps the auditor on the
/// generic path, so these tests stay about the runner.
const GENERIC: &str = r#"
schema = 1

[project]
name = "example"
repository = "https://github.com/Archont561/example"
platforms = ["linux-64"]

[[packages]]
name = "example"
kind = "cargo"
command = ["sh", "-c", "mkdir -p out && printf payload > out/example-0.1.0.crate"]
outputs = ["out/*.crate"]
smoke = ["example --version"]
"#;

/// A Conda package whose archive is placed before the run, so the archive pass has something
/// real to open. The command does nothing: this fixture is about reading, not building.
const CONDA: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]

[[packages]]
name = "example"
kind = "conda"
command = ["true"]
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

/// A project holding a well-formed `out/example-0.1.0-h1234567_0.conda`.
fn conda_project() -> TempDir {
    let directory = project(CONDA);
    place_archive(directory.path(), "example", "0.1.0", Some("MIT"), "MIT");
    directory
}

fn place_archive(root: &Path, name: &str, version: &str, index_licence: Option<&str>, about: &str) {
    let directory = root.join("out");
    std::fs::create_dir_all(&directory).expect("creates the output directory");
    support::write_conda(
        &directory.join(format!("{name}-{version}-h1234567_0.conda")),
        &[
            (
                "info/index.json",
                support::index(name, version, index_licence).as_bytes(),
            ),
            (
                "info/about.json",
                support::about(about, "info/licenses/LICENSE").as_bytes(),
            ),
            ("info/licenses/LICENSE", b"MIT License\n"),
        ],
    );
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
    let directory = project(GENERIC);
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
    let manifest = GENERIC.replace("smoke = [\"example --version\"]\n", "");
    let directory = project(&manifest);
    let output = run(directory.path(), &["plan", "--json"]);

    assert_eq!(output.exit_code(), 2);
    assert!(output.stdout().is_empty(), "an error never lands on stdout");
    assert!(output.stderr().contains("smoke"));
}

#[test]
fn a_platform_the_project_does_not_build_leaves_nothing_to_plan() {
    let directory = project(GENERIC);
    let output = run(
        directory.path(),
        &["plan", "--json", "--platform", "osx-arm64"],
    );

    assert_eq!(output.exit_code(), 0);
    assert_eq!(json(&output)["result"]["has_work"], false);
}

fn with_recipe(root: &Path, version: &str) {
    let recipe = root.join("crates/example/recipe");
    std::fs::create_dir_all(&recipe).expect("creates the recipe directory");
    std::fs::write(
        recipe.join("recipe.yaml"),
        format!(
            "context:\n  version: \"{version}\"\n\npackage:\n  name: example\n  version: ${{{{ version }}}}\n\nbuild:\n  number: 0\n\nabout:\n  license: MIT\n  license_file: LICENSE\n"
        ),
    )
    .expect("writes the recipe");
    std::fs::write(recipe.join("LICENSE"), "MIT License\n").expect("writes the licence");
}

#[test]
fn lint_accepts_a_manifest_whose_versions_agree() {
    let directory = project(GENERIC);
    with_recipe(directory.path(), "0.1.0");

    let output = run(directory.path(), &["lint", "--json"]);
    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert_eq!(json(&output)["command"], "lint");
}

#[test]
fn lint_refuses_a_recipe_that_disagrees_with_the_workspace_version() {
    let directory = project(GENERIC);
    with_recipe(directory.path(), "0.0.9");

    let output = run(directory.path(), &["lint", "--json"]);
    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("lint.version.recipe"),
        "{}",
        output.stderr()
    );
}

#[test]
fn lint_refuses_a_recipe_that_pins_nothing() {
    // The recipe holds a url source with no digest: the bytes are not pinned, so a rebuild
    // is a different artifact with the same name.
    let directory = project(GENERIC);
    with_recipe(directory.path(), "0.1.0");
    std::fs::write(
        directory.path().join("crates/example/recipe/recipe.yaml"),
        "context:\n  version: \"0.1.0\"\n\npackage:\n  name: example\n  version: ${{ version }}\n\nsource:\n  - url: https://example.invalid/example-0.1.0.tar.gz\n\nbuild:\n  number: 0\n\nabout:\n  license: MIT\n  license_file: LICENSE\n",
    )
    .expect("writes the recipe");

    let output = run(directory.path(), &["lint", "--json"]);
    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("recipe.source.sha256.missing"),
        "{}",
        output.stderr()
    );
    assert!(
        output
            .stderr()
            .contains("crates/example/recipe/recipe.yaml"),
        "the finding names the recipe: {}",
        output.stderr()
    );
}

#[test]
fn lint_reports_warnings_without_failing() {
    // A writable store no package names is a warning, which is information, not a refusal.
    let manifest =
        format!("{GENERIC}\n[[stores]]\nname = \"spare\"\nkind = \"static\"\nwritable = true\n");
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
    let directory = project(GENERIC);
    let output = run(directory.path(), &["build", "--json"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let built = &json(&output)["result"]["built"];
    assert_eq!(built[0]["package"], "example");
    assert_eq!(built[0]["outputs"][0], "out/example-0.1.0.crate");
    assert!(directory.path().join("out/example-0.1.0.crate").is_file());
}

#[test]
fn a_failing_command_is_reported_with_its_own_exit_status() {
    let manifest = GENERIC.replace(
        r#"command = ["sh", "-c", "mkdir -p out && printf payload > out/example-0.1.0.crate"]"#,
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
    assert!(
        output.stderr().contains("exit status: 3"),
        "{}",
        output.stderr()
    );
    assert!(output.stdout().is_empty());
}

#[test]
fn a_command_that_produces_nothing_it_promised_is_refused() {
    let manifest = GENERIC.replace(
        r#"command = ["sh", "-c", "mkdir -p out && printf payload > out/example-0.1.0.crate"]"#,
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
    // The child writes down what it can see: the declared value, and whether it has a home
    // directory to work in. The allowlist's *exclusions* are asserted by the unit tests on
    // `ambient`, because mutating this process's environment is `unsafe` in Rust 2024 and a
    // test that does it can race every other test in the binary.
    let manifest = GENERIC
        .replace(
            r#"command = ["sh", "-c", "mkdir -p out && printf payload > out/example-0.1.0.crate"]"#,
            r#"command = ["sh", "-c", "mkdir -p out && printf '%s:%s' \"$GREETING\" \"${HOME:+present}\" > out/example-0.1.0.crate"]"#,
        )
        .replace(
            r#"outputs = ["out/*.crate"]"#,
            "env = { GREETING = \"hello\" }\noutputs = [\"out/*.crate\"]",
        );
    let directory = project(&manifest);

    let output = run(directory.path(), &["build", "--json"]);
    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert_eq!(
        std::fs::read_to_string(directory.path().join("out/example-0.1.0.crate"))
            .expect("the artifact exists"),
        "hello:present"
    );
}

#[test]
fn audit_measures_a_generic_artifact_and_names_its_digest() {
    let directory = project(GENERIC);
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
fn audit_opens_the_archive_and_reports_what_it_says_about_itself() {
    let directory = conda_project();
    let output = run(directory.path(), &["audit", "--json"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let receipt = &json(&output)["result"]["artifacts"][0];
    assert_eq!(receipt["build"], "h1234567_0");
    assert_eq!(receipt["build_number"], 0);
    assert_eq!(receipt["subdir"], "linux-64");
    assert_eq!(receipt["license"], "MIT");
    assert_eq!(receipt["depends"][0], "libgcc >=13");
    assert_eq!(json(&output)["artifacts"][0]["id"]["version"], "0.1.0");
}

#[test]
fn audit_refuses_an_archive_whose_index_names_another_package() {
    let directory = project(CONDA);
    place_archive(directory.path(), "imposter", "0.1.0", Some("MIT"), "MIT");
    // Rename so the file name matches the manifest but the archive does not.
    std::fs::rename(
        directory.path().join("out/imposter-0.1.0-h1234567_0.conda"),
        directory.path().join("out/example-0.1.0-h1234567_0.conda"),
    )
    .expect("renames the archive");

    let output = run(directory.path(), &["audit", "--json"]);
    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("audit.package.identity"),
        "{}",
        output.stderr()
    );
}

#[test]
fn audit_refuses_a_licence_that_is_not_spdx() {
    let directory = project(CONDA);
    place_archive(
        directory.path(),
        "example",
        "0.1.0",
        Some("MIT"),
        "MIT AND possibly",
    );

    let output = run(directory.path(), &["audit", "--json"]);
    assert_eq!(output.exit_code(), 1);
    assert!(output.stderr().contains("licence"), "{}", output.stderr());
}

#[test]
fn audit_emits_one_fragment_per_artifact_under_its_subdir() {
    let directory = conda_project();
    let fragments = directory.path().join("fragments");

    let output = run(
        directory.path(),
        &[
            "audit",
            "--json",
            "--emit-fragment",
            &fragments.display().to_string(),
        ],
    );

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let fragment = fragments.join("linux-64/example-0.1.0-h1234567_0.conda.json");
    let document: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&fragment).expect("the fragment was written"),
    )
    .expect("the fragment is JSON");
    assert_eq!(document["package"], "example");
    assert_eq!(document["subdir"], "linux-64");
    assert_eq!(document["license"], "MIT");
}

#[test]
fn audit_refuses_a_released_package_that_built_nothing() {
    let directory = project(GENERIC);
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
    let directory = project(GENERIC);
    let output = run(directory.path(), &["plan", "--verbose"]);

    assert_eq!(output.exit_code(), 2);
    assert!(output.stderr().contains("usage: universe plan"));
}

#[test]
fn only_audit_accepts_emit_fragment() {
    let directory = conda_project();
    let output = run(directory.path(), &["build", "--emit-fragment", "x"]);

    assert_eq!(output.exit_code(), 2);
    assert!(
        output
            .stderr()
            .contains("unknown argument: --emit-fragment")
    );
}

#[test]
fn the_human_forms_say_something_a_person_can_read() {
    let directory = project(GENERIC);
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
    let manifest = GENERIC.replace(
        r#"mkdir -p out && printf payload > out/example-0.1.0.crate"#,
        r#"mkdir -p out && printf payload > out/other-0.1.0.crate"#,
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
fn a_build_whose_archive_names_another_package_is_refused_where_it_happened() {
    let directory = project(CONDA);
    place_archive(directory.path(), "imposter", "0.1.0", Some("MIT"), "MIT");
    std::fs::rename(
        directory.path().join("out/imposter-0.1.0-h1234567_0.conda"),
        directory.path().join("out/example-0.1.0-h1234567_0.conda"),
    )
    .expect("renames the archive");

    let output = run(directory.path(), &["build", "--json"]);
    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("build.output.identity"),
        "{}",
        output.stderr()
    );
}

#[test]
fn an_unrunnable_program_is_reported_rather_than_handed_to_a_shell() {
    let manifest = GENERIC.replace(
        r#"command = ["sh", "-c", "mkdir -p out && printf payload > out/example-0.1.0.crate"]"#,
        r#"command = ["definitely-not-a-program"]"#,
    );
    let directory = project(&manifest);

    let built = run(directory.path(), &["build", "--json"]);
    assert_eq!(built.exit_code(), 1);
    assert!(
        built.stderr().contains("build.command.unrunnable"),
        "{}",
        built.stderr()
    );
}

#[test]
fn build_writes_a_receipt_naming_the_bytes_it_produced() {
    let directory = project(GENERIC);
    let receipts = directory.path().join("receipts");

    let output = run(
        directory.path(),
        &[
            "build",
            "--json",
            "--receipts",
            &receipts.display().to_string(),
        ],
    );

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let receipt: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(receipts.join("example.json")).expect("the receipt was written"),
    )
    .expect("the receipt is JSON");
    assert_eq!(receipt["command"][0], "sh");
    assert_eq!(receipt["cwd"], ".");
    assert_eq!(receipt["outputs"][0]["path"], "out/example-0.1.0.crate");
    assert_eq!(receipt["outputs"][0]["size_bytes"], 7);
}

#[test]
fn a_command_that_hangs_is_killed_at_the_timeout() {
    let manifest = GENERIC.replace(
        r#"command = ["sh", "-c", "mkdir -p out && printf payload > out/example-0.1.0.crate"]"#,
        r#"command = ["sh", "-c", "sleep 30"]"#,
    );
    let directory = project(&manifest);

    let started = std::time::Instant::now();
    let output = run(directory.path(), &["build", "--json", "--timeout", "1"]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("timed out after 1s"),
        "{}",
        output.stderr()
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(20),
        "the run was not bounded: {:?}",
        started.elapsed()
    );
}

#[test]
fn changed_only_keeps_the_packages_a_commit_touched_and_their_dependents() {
    // A real repository, because the filter reads git rather than a manifest facet.
    let directory = project(&format!(
        "{GENERIC}\n[[packages]]\nname = \"downstream\"\nkind = \"cargo\"\ncommand = [\"true\"]\noutputs = [\"out/*.downstream\"]\nsmoke = [\"downstream --version\"]\ndepends_on = [\"example\"]\n"
    ));
    let root = directory.path();
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(
        root,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "commit",
            "-qm",
            "first",
        ],
    );
    let base = git_stdout(root, &["rev-parse", "HEAD"]);

    std::fs::create_dir_all(root.join("crates/example")).expect("creates the package directory");
    std::fs::write(root.join("crates/example/README.md"), "changed\n").expect("writes a file");
    git(root, &["add", "-A"]);
    git(
        root,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "commit",
            "-qm",
            "second",
        ],
    );

    let output = run(
        root,
        &["plan", "--json", "--changed-only", "--base-ref", &base],
    );
    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let planned = &json(&output)["result"];
    let names: Vec<&str> = planned["jobs"]
        .as_array()
        .expect("jobs is a list")
        .iter()
        .filter_map(|job| job["package"].as_str())
        .collect();
    // Execution order, not alphabetical: the dependency comes first.
    assert_eq!(names, ["example", "downstream"]);

    // A change outside every package's directory selects nothing.
    std::fs::write(root.join("README.md"), "unrelated\n").expect("writes a file");
    git(root, &["add", "-A"]);
    git(
        root,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "commit",
            "-qm",
            "third",
        ],
    );
    let after = git_stdout(root, &["rev-parse", "HEAD~1"]);
    let narrowed = run(
        root,
        &["plan", "--json", "--changed-only", "--base-ref", &after],
    );
    assert_eq!(narrowed.exit_code(), 0, "stderr: {}", narrowed.stderr());
    assert_eq!(json(&narrowed)["result"]["has_work"], false);
}

#[test]
fn changed_only_fails_closed_when_git_cannot_answer() {
    let directory = project(GENERIC);
    let output = run(
        directory.path(),
        &[
            "plan",
            "--json",
            "--changed-only",
            "--base-ref",
            "not-a-ref",
        ],
    );

    assert_eq!(output.exit_code(), 2, "{}", output.stderr());
    assert!(
        output.stderr().contains("plan.git.failed"),
        "{}",
        output.stderr()
    );
}

fn git(root: &Path, arguments: &[&str]) {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .status()
        .expect("runs git");
    assert!(status.success(), "git {arguments:?} failed");
}

fn git_stdout(root: &Path, arguments: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .expect("runs git");
    assert!(output.status.success(), "git {arguments:?} failed");
    String::from_utf8(output.stdout)
        .expect("git stdout is UTF-8")
        .trim()
        .to_owned()
}
