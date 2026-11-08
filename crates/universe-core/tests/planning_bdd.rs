//! Scenarios for reading a manifest and planning its build order.
//!
//! The unit tests in `src/planner.rs` say what a function returns. These say what a
//! maintainer observes after declaring a dependency — which package is scheduled first, and
//! which manifest mistakes are refused rather than silently dropped.

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use universe_core::diagnostic::Diagnostic;
use universe_core::manifest::Manifest;
use universe_core::planner::{BuildPlan, plan};

/// The manifest text a scenario declares, and what reading it produced.
#[derive(Default)]
struct Project {
    declaration: Option<String>,
    manifest: Option<Manifest>,
    /// Why a manifest could not be read: the validator refuses an edge that could never
    /// build before the planner ever sees it.
    rejection: Option<Vec<Diagnostic>>,
    plan: Option<BuildPlan>,
    again: Option<BuildPlan>,
}

impl Project {
    /// Read the declared manifest, keeping either the manifest or the diagnostics.
    fn read(&mut self) {
        let text = self
            .declaration
            .clone()
            .expect("a scenario declared the manifest text");
        match Manifest::from_toml(&text) {
            Ok(manifest) => self.manifest = Some(manifest),
            Err(diagnostics) => self.rejection = Some(diagnostics),
        }
    }

    fn manifest(&self) -> &Manifest {
        self.manifest.as_ref().expect("the manifest was read")
    }

    fn plan(&self) -> &BuildPlan {
        self.plan.as_ref().expect("the plan was made")
    }

    /// The diagnostics that refused the manifest, ignoring advisory warnings that are
    /// only reported alongside an error.
    fn refusal_codes(&self) -> Vec<&str> {
        self.rejection
            .as_ref()
            .expect("the manifest was refused")
            .iter()
            .filter(|diagnostic| diagnostic.is_error())
            .map(Diagnostic::code)
            .collect()
    }

    fn refusal_messages(&self) -> Vec<&str> {
        self.rejection
            .as_ref()
            .expect("the manifest was refused")
            .iter()
            .filter(|diagnostic| diagnostic.is_error())
            .map(Diagnostic::message)
            .collect()
    }
}

/// A manifest with two packages and one store, with `edges` spliced in after the last
/// package so a scenario can append a `depends_on` line without restating the rest.
fn declaration(edges: &str) -> String {
    format!(
        r#"
schema = 1

[project]
name = "example-universe"
platforms = ["linux-64", "osx-arm64"]

[[packages]]
name = "universe-kit"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["universe-kit --version"]
publish = ["conda-static"]

[[packages]]
name = "universe-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["universe-airlock --version"]
publish = ["conda-static"]
{edges}

[[stores]]
name = "conda-static"
kind = "static"
url = "https://example.invalid/channel"
writable = true
"#
    )
}

/// Two packages that depend on each other, which no build order can satisfy.
fn cyclic_declaration() -> String {
    String::from(
        r#"
schema = 1

[project]
name = "example-universe"
platforms = ["linux-64"]

[[packages]]
name = "universe-kit"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["universe-kit --version"]
publish = ["conda-static"]
depends_on = ["universe-airlock"]

[[packages]]
name = "universe-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["universe-airlock --version"]
publish = ["conda-static"]
depends_on = ["universe-kit"]

[[stores]]
name = "conda-static"
kind = "static"
url = "https://example.invalid/channel"
writable = true
"#,
    )
}

#[fixture]
fn project() -> Project {
    Project::default()
}

#[given("a project whose two packages are built for linux and osx")]
fn two_platforms(project: &mut Project) {
    project.declaration = Some(declaration(""));
}

#[given("a project whose consumer package depends on a library package")]
fn a_dependency(project: &mut Project) {
    // `universe-airlock` is the consumer, `universe-kit` stands in for the library.
    project.declaration = Some(declaration("depends_on = [\"universe-kit\"]"));
}

#[given("a project whose consumer package depends on a package that does not exist")]
fn an_unknown_dependency(project: &mut Project) {
    project.declaration = Some(declaration("depends_on = [\"universe-portal\"]"));
}

#[given("a project whose two packages depend on each other")]
fn a_cycle(project: &mut Project) {
    project.declaration = Some(cyclic_declaration());
}

#[when("the manifest is read")]
fn read_the_manifest(project: &mut Project) {
    project.read();
}

#[when("the build order is planned")]
fn plan_it(project: &mut Project) {
    project.read();
    let manifest = project.manifest();
    project.plan = Some(plan(manifest).expect("the fixture manifest is plannable"));
}

#[when("the build order is planned again")]
fn plan_again(project: &mut Project) {
    project.again = Some(plan(project.manifest()).expect("the plan is repeatable"));
}

#[then("each package is scheduled with both platforms")]
fn both_platforms(project: &Project) {
    let plan = project.plan();
    assert_eq!(plan.jobs.len(), 2, "{plan:?}");
    for job in &plan.jobs {
        assert_eq!(job.platforms, ["linux-64", "osx-arm64"], "{}", job.package);
    }
}

#[then("the plan is a single level because nothing depends on anything")]
fn one_level(project: &Project) {
    assert_eq!(
        project.plan().levels,
        [vec![
            "universe-airlock".to_owned(),
            "universe-kit".to_owned()
        ]]
    );
}

#[then("the library is in the first level")]
fn library_first(project: &Project) {
    assert_eq!(
        project.plan().levels.first(),
        Some(&vec!["universe-kit".to_owned()])
    );
}

#[then("the consumer is in the second level")]
fn consumer_second(project: &Project) {
    assert_eq!(
        project.plan().levels.get(1),
        Some(&vec!["universe-airlock".to_owned()])
    );
}

#[then("the manifest is refused because the dependency is unknown")]
fn unknown_dependency(project: &Project) {
    // The validator refuses the edge before the planner can schedule around it, which is the
    // earlier and louder of the two refusals.
    let codes = project.refusal_codes();
    assert_eq!(codes, ["manifest.package.dependency.unknown"], "{codes:?}");
    assert!(
        project
            .refusal_messages()
            .iter()
            .any(|line| line.contains("universe-portal")),
        "{:?}",
        project.refusal_messages()
    );
}

#[then("the manifest is refused because the edges form a cycle")]
fn a_cycle_is_refused(project: &Project) {
    let codes = project.refusal_codes();
    assert_eq!(
        codes,
        [
            "manifest.package.dependency.cycle",
            "manifest.package.dependency.cycle"
        ],
        "{codes:?}"
    );
    // Both packages on the cycle are named, because either one is where the author would
    // start looking.
    let messages = project.refusal_messages();
    assert!(messages.iter().any(|line| line.contains("universe-kit")));
    assert!(
        messages
            .iter()
            .any(|line| line.contains("universe-airlock"))
    );
}

#[then("both plans are identical")]
fn identical(project: &Project) {
    assert_eq!(project.plan, project.again);
}

#[scenario(
    path = "features/planning.feature",
    name = "Two platforms are built for every package"
)]
fn every_package_is_built_for_every_platform(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/planning.feature",
    name = "A package that must be built first comes in an earlier level"
)]
fn a_dependency_orders_the_levels(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/planning.feature",
    name = "A dependency on a package the manifest does not declare is refused"
)]
fn an_unknown_dependency_is_refused(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/planning.feature",
    name = "A dependency cycle is refused and the package on it is named"
)]
fn a_cycle_is_named(project: Project) {
    let _ = project;
}

#[scenario(
    path = "features/planning.feature",
    name = "Planning the same manifest twice produces the same order"
)]
fn planning_is_repeatable(project: Project) {
    let _ = project;
}
