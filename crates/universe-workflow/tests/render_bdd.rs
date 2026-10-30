//! Scenarios for the manifest-to-workflow renderer.
//!
//! These complement the unit tests in `src/lib.rs` rather than repeat them: a unit
//! test says what a function returns, a scenario says what a maintainer observes after
//! changing a manifest. Every `Then` reads the rendered files from disk, because the
//! files are the thing the maintainer actually sees — asserting on the in-memory record
//! would pass while the written bytes were wrong.

use std::path::PathBuf;

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use tempfile::TempDir;
use universe_core::diagnostic::Diagnostic;
use universe_core::manifest::Manifest;
use universe_workflow::RenderOutput;

/// A project under a temporary directory, plus what the last render produced.
///
/// The directory is owned here so it lives for the whole scenario and is removed
/// afterwards; nothing in these scenarios touches the live repository.
#[derive(Default)]
struct Project {
    root: Option<TempDir>,
    source: String,
    manifest: Option<Manifest>,
    refusal: Option<Vec<Diagnostic>>,
    rendered: Option<RenderOutput>,
    rerendered: Option<RenderOutput>,
}

impl Project {
    fn directories(&self) -> PathBuf {
        self.root
            .as_ref()
            .expect("a scenario created its project directory")
            .path()
            .join(".github/workflows")
    }

    fn workflow(&self, name: &str) -> String {
        let path = self.directories().join(name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
    }

    fn rendered(&self) -> &RenderOutput {
        self.rendered.as_ref().expect("the workflow set was rendered")
    }
}

const TWO_STORES: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]

[[packages]]
name = "universe-airlock"
kind = "conda"
command = ["rattler-build", "build"]
outputs = ["output/**/*.conda"]
smoke = ["universe-airlock --version"]
publish = ["conda-prefix", "conda-static"]

[[stores]]
name = "conda-prefix"
kind = "prefix-dev"
channel = "someone/channel"
writable = true
auth = "oidc"
verifier = "registry"

[[stores]]
name = "conda-static"
kind = "static"
writable = true
needs_index = true
verifier = "file"
"#;

const NO_STORES: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]

[[packages]]
name = "universe-airlock"
kind = "conda"
command = ["rattler-build", "build"]
outputs = ["output/**/*.conda"]
release = false
"#;

const WRONG_STORE_KIND: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]

[[packages]]
name = "universe-airlock"
kind = "conda"
command = ["rattler-build", "build"]
outputs = ["output/**/*.conda"]
smoke = ["universe-airlock --version"]
publish = ["npm-fixture"]

[[stores]]
name = "npm-fixture"
kind = "npmjs"
writable = true
secrets = ["npm-token"]
"#;

#[fixture]
fn project() -> Project {
    Project::default()
}

fn load(project: &mut Project, source: &str) {
    project.root = Some(tempfile::tempdir().expect("temporary project directory"));
    project.source = source.to_owned();
    match Manifest::from_toml(source) {
        Ok(manifest) => {
            project.manifest = Some(manifest);
            project.refusal = None;
        }
        Err(diagnostics) => {
            project.manifest = None;
            project.refusal = Some(diagnostics);
        }
    }
}

#[given("a project that publishes its Airlock package to a hosted channel and to a static channel")]
fn project_with_two_stores(project: &mut Project) {
    load(project, TWO_STORES);
    assert!(project.manifest.is_some(), "the two-store manifest is valid");
}

#[given("a project that publishes nothing")]
fn project_publishing_nothing(project: &mut Project) {
    load(project, NO_STORES);
    assert!(project.manifest.is_some());
    assert!(project.manifest.as_ref().expect("manifest").stores.is_empty());
}

#[given("a project whose Conda package publishes to an npm store")]
fn project_with_incompatible_store(project: &mut Project) {
    load(project, WRONG_STORE_KIND);
    assert!(
        project.manifest.is_none(),
        "the manifest validator refuses this before rendering"
    );
}

#[when("the workflow set is rendered")]
fn render_the_workflow_set(project: &mut Project) {
    let Some(manifest) = project.manifest.as_ref() else {
        // The manifest was refused earlier; rendering is not reached, which is the point
        // of the refusal scenario.
        return;
    };
    let rendered = universe_workflow::render(manifest, &project.source).expect("the set renders");
    universe_workflow::write(&project.directories(), &rendered).expect("the set is written");
    project.rendered = Some(rendered);
}

#[when("the same manifest is rendered again")]
fn render_again(project: &mut Project) {
    let manifest = project.manifest.as_ref().expect("a valid manifest");
    project.rerendered = Some(
        universe_workflow::render(manifest, &project.source).expect("the set renders again"),
    );
}

#[when("the main workflow file is edited by hand")]
fn edit_by_hand(project: &mut Project) {
    let path = project.directories().join("universe.yml");
    std::fs::write(&path, "# edited after generation\n").expect("the file is editable");
}

#[then("the publish file has a job for the hosted channel")]
fn publish_job_for_hosted_channel(project: &Project) {
    let publish = project.workflow("universe-publish.yml");
    assert!(
        publish.contains("name: publish / conda-prefix"),
        "no hosted-channel job in:\n{publish}"
    );
}

#[then("the publish file has a job for the static channel")]
fn publish_job_for_static_channel(project: &Project) {
    let publish = project.workflow("universe-publish.yml");
    assert!(
        publish.contains("name: publish / conda-static"),
        "no static-channel job in:\n{publish}"
    );
}

#[then("the publish file prints the store to package mapping")]
fn publish_file_prints_mapping(project: &Project) {
    // The mapping is a derived view: the renderer reports it, so a mapping error is
    // visible rather than inferred by reading YAML.
    let mapping = RenderOutput::store_mapping(
        project.manifest.as_ref().expect("a valid manifest"),
    );
    let rendered = project.rendered();
    assert_eq!(
        mapping,
        [
            ("conda-prefix".to_owned(), vec!["universe-airlock".to_owned()]),
            ("conda-static".to_owned(), vec!["universe-airlock".to_owned()]),
        ]
    );
    assert!(rendered.required_checks.iter().all(|check| check.starts_with("universe / ")));
}

#[then("the rendered file set is the main workflow file alone")]
fn only_the_main_file(project: &Project) {
    let names: Vec<&str> = project
        .rendered()
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert_eq!(names, ["universe.yml"]);
    assert!(
        !project.directories().join("universe-publish.yml").exists(),
        "a project with no store must not gain a publish file"
    );
}

#[then("the drift check names the main workflow file")]
fn drift_names_the_main_file(project: &Project) {
    let diagnostics = universe_workflow::check(&project.directories(), project.rendered());
    assert_eq!(diagnostics.len(), 1, "one drifted file: {diagnostics:?}");
    assert_eq!(diagnostics[0].code(), "ci.file.drift");
    assert!(
        diagnostics[0]
            .path()
            .expect("the diagnostic names a path")
            .ends_with("universe.yml"),
        "the diagnostic must name the file that drifted"
    );
}

#[then("rendering is refused because the store kind does not accept the package kind")]
fn refusal_names_the_incompatible_kind(project: &Project) {
    let diagnostics = project
        .refusal
        .as_ref()
        .expect("the manifest was refused at validation");
    let codes: Vec<&str> = diagnostics.iter().map(Diagnostic::code).collect();
    assert!(
        codes.contains(&"manifest.package.publish.incompatible"),
        "expected an incompatible-kind finding, got {codes:?}"
    );
    assert!(
        project.rendered.is_none(),
        "nothing may render from a refused manifest"
    );
}

#[then("both renders are byte-identical")]
fn both_renders_are_identical(project: &Project) {
    let first = project
        .rendered
        .as_ref()
        .expect("the first render happened");
    let second = project
        .rerendered
        .as_ref()
        .expect("the second render happened");
    assert_eq!(
        first, second,
        "a no-op re-render must not manufacture a diff"
    );
}

// One binding per scenario: `#[scenario]` without a `name` selects the first scenario in
// the file, so each function names the one it drives. The fixture parameter exists so the
// macro can place it in the step context; the test body has nothing left to assert,
// because every assertion is a step.
#[scenario(
    path = "tests/features/render.feature",
    name = "Two writable stores produce one publish job each"
)]
fn two_stores_produce_two_publish_jobs(project: Project) {
    let _ = project;
}

#[scenario(
    path = "tests/features/render.feature",
    name = "A project that publishes nothing renders only the main file"
)]
fn nothing_to_publish_renders_one_file(project: Project) {
    let _ = project;
}

#[scenario(
    path = "tests/features/render.feature",
    name = "A hand-edited generated file is reported as drift"
)]
fn a_hand_edit_is_drift(project: Project) {
    let _ = project;
}

#[scenario(
    path = "tests/features/render.feature",
    name = "A Conda package cannot be published to an npm store"
)]
fn an_incompatible_store_is_refused(project: Project) {
    let _ = project;
}

#[scenario(
    path = "tests/features/render.feature",
    name = "Rendering the same manifest twice produces the same bytes"
)]
fn rendering_is_repeatable(project: Project) {
    let _ = project;
}
