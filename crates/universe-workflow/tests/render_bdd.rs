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
        self.rendered
            .as_ref()
            .expect("the workflow set was rendered")
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

const LOCK_ADAPTERS: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]

[airlock.developer]
adapters = ["pixi", "bun"]
mirror = "mirror"

[[packages]]
name = "universe-airlock"
kind = "conda"
command = ["rattler-build", "build"]
outputs = ["output/**/*.conda"]
smoke = ["universe-airlock --version"]
publish = ["conda-static"]

[[stores]]
name = "conda-static"
kind = "static"
writable = true
needs_index = true
verifier = "file"
"#;

const VERIFY_PUBLISHED: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]

[workflow]
verify_published = true

[[packages]]
name = "universe-airlock"
kind = "conda"
command = ["rattler-build", "build"]
outputs = ["output/**/*.conda"]
smoke = ["universe-airlock --version"]
publish = ["conda-static"]

[[stores]]
name = "conda-static"
kind = "static"
writable = true
needs_index = true
verifier = "file"
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
    assert!(
        project.manifest.is_some(),
        "the two-store manifest is valid"
    );
}

#[given("a project that publishes nothing")]
fn project_publishing_nothing(project: &mut Project) {
    load(project, NO_STORES);
    assert!(project.manifest.is_some());
    assert!(
        project
            .manifest
            .as_ref()
            .expect("manifest")
            .stores
            .is_empty()
    );
}

#[given("a project whose Conda package publishes to an npm store")]
fn project_with_incompatible_store(project: &mut Project) {
    load(project, WRONG_STORE_KIND);
    assert!(
        project.manifest.is_none(),
        "the manifest validator refuses this before rendering"
    );
}

#[given("a project that declares Pixi and Bun lock adapters")]
fn project_with_lock_adapters(project: &mut Project) {
    load(project, LOCK_ADAPTERS);
    assert!(
        project.manifest.is_some(),
        "the lock-adapter manifest is valid"
    );
}

#[given("a project that verifies every published destination")]
fn project_that_verifies_publications(project: &mut Project) {
    load(project, VERIFY_PUBLISHED);
    assert!(
        project
            .manifest
            .as_ref()
            .expect("manifest")
            .workflow
            .verify_published,
        "the manifest asks for the destinations to be verified"
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
    project.rerendered =
        Some(universe_workflow::render(manifest, &project.source).expect("the set renders again"));
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
    // A registry keeps what it was first given, so it is uploaded to one file at a time;
    // `publish` pushes a tree to a branch and would fail here with no remote named.
    assert!(
        publish.contains("publish-artifacts --store conda-prefix"),
        "the hosted job does not use the verb that uploads:\n{publish}"
    );
}

#[then("the publish file has a job for the static channel")]
fn publish_job_for_static_channel(project: &Project) {
    let publish = project.workflow("universe-publish.yml");
    assert!(
        publish.contains("name: publish / conda-static"),
        "no static-channel job in:\n{publish}"
    );
    // A branch is pushed to, and `publish` refuses to run without a remote: naming it here
    // is the difference between a job that works and one that fails on the first tag.
    assert!(
        publish.contains("publish --store conda-static --remote"),
        "the static job names no remote to push to:\n{publish}"
    );
    assert!(
        publish.contains("${{ github.server_url }}/${{ github.repository }}.git"),
        "the static job's remote is not the repository it runs in:\n{publish}"
    );
}

#[then("the hosted job may mint an identity token")]
fn hosted_job_may_mint_an_identity_token(project: &Project) {
    let publish = project.workflow("universe-publish.yml");
    assert!(
        publish.contains("id-token: write"),
        "the hosted job cannot ask for an identity token without the permission:\n{publish}"
    );
    // The permission has to be on the job that publishes, not somewhere in the file.
    let hosted = job_block(&publish, "publish / conda-prefix");
    assert!(
        hosted.contains("id-token: write"),
        "`id-token: write` is not in the hosted job's own permissions:\n{hosted}"
    );
}

#[then("the hosted job exports no long-lived secret")]
fn hosted_job_exports_no_long_lived_secret(project: &Project) {
    let publish = project.workflow("universe-publish.yml");
    let hosted = job_block(&publish, "publish / conda-prefix");
    assert!(
        !hosted.contains("secrets."),
        "a trusted-publishing job must not read a stored secret:\n{hosted}"
    );
    // The branch store still declares one, so this is a claim about the hosted job rather
    // than about a manifest that happened to declare nothing.
    let static_job = job_block(&publish, "publish / conda-static");
    assert!(
        !static_job.contains("id-token: write"),
        "a store that is pushed to needs no identity token:\n{static_job}"
    );
}

/// One job's YAML, from its `name:` line to the next job at the same indentation.
///
/// A substring search over the whole file cannot say which job a permission belongs to, and
/// a permission on the wrong job is exactly the mistake this scenario exists to catch.
fn job_block(publish: &str, name: &str) -> String {
    let start = publish
        .find(&format!("name: {name}"))
        .unwrap_or_else(|| panic!("no job named '{name}' in:\n{publish}"));
    // Jobs sit two spaces in, so the next line at that indentation opens the following job.
    let rest = &publish[start..];
    let end = rest
        .match_indices("\n  ")
        .skip(1)
        .map(|(offset, _)| offset)
        .find(|offset| {
            let candidate = &rest[*offset..];
            candidate.starts_with("\n  ") && !candidate.starts_with("\n    ")
        })
        .unwrap_or(rest.len());
    rest[..end].to_owned()
}

#[then("the publish file prints the store to package mapping")]
fn publish_file_prints_mapping(project: &Project) {
    // The mapping is a derived view: the renderer reports it, so a mapping error is
    // visible rather than inferred by reading YAML.
    let mapping = RenderOutput::store_mapping(project.manifest.as_ref().expect("a valid manifest"));
    let rendered = project.rendered();
    assert_eq!(
        mapping,
        [
            (
                "conda-prefix".to_owned(),
                vec!["universe-airlock".to_owned()]
            ),
            (
                "conda-static".to_owned(),
                vec!["universe-airlock".to_owned()]
            ),
        ]
    );
    assert!(
        rendered
            .required_checks
            .iter()
            .all(|check| check.starts_with("universe / "))
    );
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

/// Every generated file, read back as text — the unit for the two shape assertions below.
fn rendered_files(project: &Project) -> Vec<String> {
    project
        .rendered()
        .files
        .iter()
        .map(|file| project.workflow(&file.path))
        .collect()
}

/// The jobs of one rendered file as `(name, body)`, split on the two-space indent that
/// marks a key directly under `jobs:`.
fn job_blocks(text: &str) -> Vec<(String, String)> {
    let mut blocks: Vec<(String, String)> = Vec::new();
    let mut in_jobs = false;
    for line in text.lines() {
        if line.trim_end() == "jobs:" {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        let starts_job = line.starts_with("  ")
            && !line.starts_with("   ")
            && !line.trim_start().starts_with('-')
            && line.trim_end().ends_with(':');
        if starts_job {
            blocks.push((line.trim().trim_end_matches(':').to_owned(), String::new()));
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Some((_, body)) = blocks.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    blocks
}

#[then("every job names the runner it runs on")]
fn every_job_names_its_runner(project: &Project) {
    for text in rendered_files(project) {
        let blocks = job_blocks(&text);
        assert!(!blocks.is_empty(), "a rendered file has no jobs:\n{text}");
        for (name, body) in blocks {
            let runner = body
                .lines()
                .find_map(|line| line.trim().strip_prefix("runs-on:"))
                .unwrap_or_default()
                .trim()
                .to_owned();
            assert!(
                !runner.is_empty(),
                "job {name} renders without a runner, so it schedules nowhere:\n{text}"
            );
        }
    }
}

#[then("every job that runs a project command checks the project out and sets up Pixi first")]
fn every_job_checks_out_first(project: &Project) {
    let mut checked = 0;
    for text in rendered_files(project) {
        for (name, body) in job_blocks(&text) {
            // Job names read `universe / <id>`, so the search has to skip them: only a
            // command that runs the project's tooling needs the work tree.
            let needs_the_tree = body
                .lines()
                .filter(|line| !line.trim_start().starts_with("name:"))
                .any(|line| line.contains("universe ") || line.contains("pixi run"));
            if !needs_the_tree {
                // A job that only reads its predecessors' results has nothing to check out,
                // and must not gain a failure mode that could outrank their success.
                continue;
            }
            checked += 1;
            // Step boundaries are the six-space `- ` lines inside `steps:`. Reading the
            // whole sequence, rather than just its `uses:` members, is what makes the
            // order assertion mean something: a `run:` step placed first is invisible to a
            // filter that only looks at action references.
            let steps: Vec<&str> = body
                .lines()
                .filter(|line| line.starts_with("      - "))
                .map(str::trim)
                .collect();
            assert!(
                steps.len() >= 2,
                "job {name} runs a project command without checking out:\n{text}"
            );
            assert!(
                steps[0].contains("actions/checkout@"),
                "job {name}: the first step must check the project out, found {}:\n{text}",
                steps[0]
            );
            assert!(
                steps[1].contains("prefix-dev/setup-pixi@"),
                "job {name}: the second step must set Pixi up, found {}:\n{text}",
                steps[1]
            );
        }
    }
    assert!(
        checked > 0,
        "no job ran a project command, so this scenario asserted nothing"
    );
}

#[then("no step runs a bare universe command")]
fn no_bare_universe_command(project: &Project) {
    let mut reached = 0;
    for text in rendered_files(project) {
        for line in text.lines() {
            let command = line.trim();
            assert!(
                !command.starts_with("universe "),
                "a step runs `{command}`, which no runner has on its PATH:\n{text}"
            );
            if command.starts_with("pixi run --frozen universe -- ") {
                reached += 1;
            }
        }
    }
    assert!(
        reached > 0,
        "the rendered set reaches no Universe verb at all, so this scenario asserted nothing"
    );
}

#[then("the build job uploads the artifacts it produced")]
fn build_uploads(project: &Project) {
    let main = project.workflow("universe.yml");
    let build = job_blocks(&main)
        .into_iter()
        .find(|(name, _)| name == "build")
        .expect("the main file has a build job")
        .1;
    assert!(
        build.contains("actions/upload-artifact@"),
        "the build job does not hand its output on:\n{main}"
    );
    assert!(
        build.contains("name: universe-${{ matrix.platform }}"),
        "the artifact is not named per platform, so the downloaders cannot ask for one:\n{main}"
    );
    assert!(
        build.contains("if-no-files-found: error"),
        "a build that produced nothing must fail here rather than at publish time:\n{main}"
    );
}

#[then("every job that audits, verifies, or publishes downloads them")]
fn later_jobs_download(project: &Project) {
    let mut checked = 0;
    for file in ["universe.yml", "universe-publish.yml"] {
        let text = project.workflow(file);
        for (name, body) in job_blocks(&text) {
            let consumes = body.contains("audit --json")
                || body.contains("verify --artifact")
                || body.contains("publish-artifacts --store")
                || body.contains("publish --store");
            if !consumes {
                continue;
            }
            checked += 1;
            assert!(
                body.contains("actions/download-artifact@"),
                "job {name} consumes artifacts it never downloads:\n{text}"
            );
        }
    }
    assert!(
        checked >= 3,
        "only {checked} consuming jobs were found, so this scenario proved little"
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

#[then("the rendered file set includes the Airlock file")]
fn airlock_file_is_rendered(project: &Project) {
    let names: Vec<&str> = project
        .rendered()
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert!(
        names.contains(&"universe-airlock.yml"),
        "declared lock adapters must produce the Airlock file, got {names:?}"
    );
    assert!(
        project.directories().join("universe-airlock.yml").is_file(),
        "the file is in the rendered record but not on disk"
    );
}

#[then("the Airlock file exercises every declared lock format")]
fn airlock_file_covers_each_adapter(project: &Project) {
    let airlock = project.workflow("universe-airlock.yml");
    // The adapters are a matrix, so one job covers both formats rather than one job each;
    // reading the axis is what proves both were declared.
    assert!(
        airlock.contains("adapter: [pixi, bun]"),
        "the mirror job does not fan out over the declared adapters:\n{airlock}"
    );
    assert!(
        airlock.contains("airlock mirror --adapter ${{ matrix.adapter }}"),
        "the mirror job does not pass its own axis to the verb:\n{airlock}"
    );
    let render = job_blocks(&airlock)
        .into_iter()
        .find(|(name, _)| name == "render-airlock")
        .expect("the Airlock file renders the entrypoints")
        .1;
    assert!(
        render.contains("needs: [mirror]"),
        "rendering the entrypoints before the mirror finishes would publish a partial lock:\n{airlock}"
    );
}

#[then("the publish file holds a separate reader for the anonymous destination")]
fn publish_file_has_a_separate_reader(project: &Project) {
    let publish = project.workflow("universe-publish.yml");
    let blocks = job_blocks(&publish);
    let (_, store_job) = blocks
        .iter()
        .find(|(name, _)| name == "conda-static")
        .expect("the publish file has the store's job");
    // A destination anyone can read is checked by a job that presents no credential of
    // its own, so the check cannot pass on the publisher's authority.
    assert!(
        !store_job.contains("verify --published"),
        "the publishing job verified its own anonymous destination:\n{publish}"
    );
    let (_, reader) = blocks
        .iter()
        .find(|(name, _)| name == "verify-index")
        .expect("a verifiable destination needs a reader job");
    assert!(
        reader.contains("verify --index"),
        "the reader job does not re-read the destinations:\n{publish}"
    );
    assert!(
        reader.contains("needs: [conda-static]"),
        "the reader must run after the publish it reads:\n{publish}"
    );
}

// One binding per scenario: `#[scenario]` without a `name` selects the first scenario in
// the file, so each function names the one it drives. The fixture parameter exists so the
// macro can place it in the step context; the test body has nothing left to assert,
// because every assertion is a step.
#[scenario(
    path = "features/render.feature",
    name = "Two writable stores produce one publish job each"
)]
fn two_stores_produce_two_publish_jobs(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/render.feature",
    name = "A project that publishes nothing renders only the main file"
)]
fn nothing_to_publish_renders_one_file(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/render.feature",
    name = "A hand-edited generated file is reported as drift"
)]
fn a_hand_edit_is_drift(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/render.feature",
    name = "A Conda package cannot be published to an npm store"
)]
fn an_incompatible_store_is_refused(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/render.feature",
    name = "Rendering the same manifest twice produces the same bytes"
)]
fn rendering_is_repeatable(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/render.feature",
    name = "Every job gets a runner, and one that runs a project command checks out first"
)]
fn every_job_runs_somewhere_and_checks_out_first(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/render.feature",
    name = "Every Universe command runs through the Pixi task that built it"
)]
fn every_command_goes_through_the_pixi_task(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/render.feature",
    name = "A build's artifacts reach the jobs that audit and publish them"
)]
fn artifacts_reach_the_jobs_that_need_them(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/render.feature",
    name = "Lock adapters add the Airlock file with a job per declared format"
)]
fn lock_adapters_add_the_airlock_file(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/render.feature",
    name = "A project that verifies published destinations gets a separate reader job"
)]
fn a_verifiable_destination_gets_a_reader(project: Project) {
    let _ = project;
}
