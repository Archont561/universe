//! Scenarios for `publish-artifacts`: what a store holds after one run and after two.
//!
//! The destination is a directory beside the scenario's project, because that is the one
//! store the claim can be checked against by reading it. What the scenarios assert is what a
//! person would check: the artifact is there under its subdir, a second run of the same build
//! adds nothing, a changed build under an occupied name is refused and the old bytes survive,
//! and a dry run creates nothing.
//!
//! Honest provenance: the verb was built before this feature file existed. These are
//! acceptance scenarios written against behaviour that already shipped, so they record what
//! the upload loop does rather than what designing it first produced.

use std::path::{Path, PathBuf};

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use tempfile::TempDir;
use universe_core::CommandOutput;

const PACKAGE: &str = "example-airlock";
const ARTIFACT: &str = "output/example-airlock-1.0.0-0.conda";
const FRAGMENT: &str = "output/fragments/example-airlock-linux-64.json";
const SUBDIR: &str = "linux-64";

/// A project, the directory standing in for its store, and what the last verb did.
#[derive(Default)]
struct Publish {
    root: Option<TempDir>,
    output: Option<CommandOutput>,
}

impl Publish {
    fn root(&self) -> &Path {
        self.root
            .as_ref()
            .expect("a scenario created its project directory")
            .path()
    }

    fn output(&self) -> &CommandOutput {
        self.output.as_ref().expect("a verb ran")
    }

    /// Where the store keeps what it was given.
    fn store(&self) -> PathBuf {
        self.root().join("published")
    }

    fn published_artifact(&self) -> PathBuf {
        self.store()
            .join(SUBDIR)
            .join("example-airlock-1.0.0-0.conda")
    }

    fn run(&mut self, arguments: &[&str]) -> &CommandOutput {
        let mut invocation: Vec<String> = arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect();
        invocation.push("--root".to_owned());
        invocation.push(self.root().display().to_string());
        self.output = Some(universe_kit::run(invocation));
        self.output()
    }

    fn publish(&mut self) -> &CommandOutput {
        self.run(&["publish-artifacts", "--store", "local-store", "--json"])
    }
}

/// What every scenario starts with: an audited package and a directory to publish it to.
fn audited(extra_store: &str) -> Publish {
    let root = tempfile::tempdir().expect("temporary project directory");
    let store = root.path().join("published");
    let manifest = format!(
        r#"
schema = 1

[project]
name = "fixture-publish"
platforms = ["linux-64"]

[[packages]]
name = "{PACKAGE}"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["{PACKAGE} --version"]
publish = ["local-store"]

[[stores]]
name = "local-store"
kind = "filesystem"
url = "{store}"
writable = true
auth = "token"
{extra_store}"#,
        store = store.display()
    );
    std::fs::write(root.path().join("universe.toml"), manifest).expect("manifest");

    let artifact = root.path().join(ARTIFACT);
    std::fs::create_dir_all(artifact.parent().expect("output directory")).expect("output");
    std::fs::write(&artifact, b"conda bytes").expect("artifact");
    let digest = universe_workflow::sha256::sha256_hex(b"conda bytes");
    let fragment = root.path().join(FRAGMENT);
    std::fs::create_dir_all(fragment.parent().expect("fragment directory")).expect("fragments");
    std::fs::write(
        &fragment,
        format!(
            r#"{{
  "package": "{PACKAGE}",
  "version": "1.0.0",
  "build": "0",
  "build_number": 0,
  "subdir": "{SUBDIR}",
  "digest": "{digest}",
  "size_bytes": 11,
  "depends": [],
  "license": "MIT",
  "license_files": [],
  "path": "{ARTIFACT}"
}}
"#
        ),
    )
    .expect("fragment");

    Publish {
        root: Some(root),
        output: None,
    }
}

#[fixture]
fn publish() -> Publish {
    Publish::default()
}

#[given("an audited package with a filesystem store")]
fn an_audited_package(publish: &mut Publish) {
    *publish = audited("");
}

#[given("an audited package with a filesystem store and a branch store")]
fn an_audited_package_and_a_branch_store(publish: &mut Publish) {
    *publish =
        audited("\n[[stores]]\nname = \"channel-branch\"\nkind = \"branch\"\nwritable = true\n");
}

#[given("the published artifact is replaced by a different build")]
fn replace_the_published_artifact(publish: &mut Publish) {
    // A publish first, so there is an occupied name for the different build to collide with.
    publish.publish();
    let artifact = publish.published_artifact();
    std::fs::create_dir_all(artifact.parent().expect("subdir")).expect("subdir");
    std::fs::write(artifact, b"a different build").expect("the store is writable");
}

#[when("the artifacts are published")]
fn publish_artifacts(publish: &mut Publish) {
    publish.publish();
}

#[when("the artifacts are published again")]
fn publish_artifacts_again(publish: &mut Publish) {
    publish.publish();
}

#[when("the artifacts are published as a dry run")]
fn publish_artifacts_dry_run(publish: &mut Publish) {
    publish.run(&["publish-artifacts", "--store", "local-store", "--dry-run"]);
}

#[when("the branch store is named for an upload")]
fn name_the_branch_store(publish: &mut Publish) {
    publish.run(&["publish-artifacts", "--store", "channel-branch"]);
}

#[then("the store holds the artifact under its subdir")]
fn store_holds_the_artifact(publish: &Publish) {
    assert_eq!(
        publish.output().exit_code(),
        0,
        "the publish succeeded: {}",
        publish.output().stderr()
    );
    assert_eq!(
        std::fs::read(publish.published_artifact()).expect("the published artifact"),
        b"conda bytes"
    );
}

#[then("the second publish reports the artifact as already there")]
fn second_publish_reports_unchanged(publish: &Publish) {
    assert_eq!(
        publish.output().exit_code(),
        0,
        "the second publish succeeded: {}",
        publish.output().stderr()
    );
    let envelope: serde_json::Value =
        serde_json::from_str(publish.output().stdout().trim()).expect("a JSON envelope");
    let stores = &envelope["result"]["stores"];
    assert_eq!(
        stores[0]["uploaded"],
        serde_json::json!([]),
        "nothing was uploaded twice: {stores}"
    );
    assert_eq!(
        stores[0]["unchanged"],
        serde_json::json!(["linux-64/example-airlock-1.0.0-0.conda"]),
        "the artifact is reported as already there: {stores}"
    );
}

#[then("the publish refuses the occupied name")]
fn publish_refuses_the_occupied_name(publish: &Publish) {
    assert_eq!(publish.output().exit_code(), 1, "the publish is refused");
    assert!(
        publish.output().stderr().contains("publish.store.refused"),
        "the refusal names its code: {}",
        publish.output().stderr()
    );
}

#[then("the store still holds the bytes it held")]
fn store_keeps_its_bytes(publish: &Publish) {
    assert_eq!(
        std::fs::read(publish.published_artifact()).expect("the untouched artifact"),
        b"a different build",
        "a refusal leaves the destination alone"
    );
}

#[then("the store directory does not exist")]
fn store_directory_is_absent(publish: &Publish) {
    assert_eq!(
        publish.output().exit_code(),
        0,
        "the dry run succeeded: {}",
        publish.output().stderr()
    );
    assert!(
        !publish.store().exists(),
        "a dry run creates no destination directory"
    );
}

#[then("the publish is a usage error naming the store kind")]
fn publish_is_a_usage_error(publish: &Publish) {
    assert_eq!(publish.output().exit_code(), 2, "it is a usage error");
    assert!(
        publish.output().stderr().contains("publish.store.kind"),
        "the error names the store kind: {}",
        publish.output().stderr()
    );
}

// One binding per scenario. `#[scenario]` without a name takes the first scenario it finds,
// so each function names the one it drives.
#[scenario(
    path = "features/publish.feature",
    name = "An artifact is published to the store under its subdir"
)]
fn an_artifact_is_published(publish: Publish) {
    let _ = publish;
}

#[scenario(
    path = "features/publish.feature",
    name = "Publishing the same build twice uploads nothing the second time"
)]
fn a_republish_is_a_no_op(publish: Publish) {
    let _ = publish;
}

#[scenario(
    path = "features/publish.feature",
    name = "A changed build under an occupied name is refused"
)]
fn a_changed_build_is_refused(publish: Publish) {
    let _ = publish;
}

#[scenario(path = "features/publish.feature", name = "A dry run writes nothing")]
fn a_dry_run_writes_nothing(publish: Publish) {
    let _ = publish;
}

#[scenario(
    path = "features/publish.feature",
    name = "A branch store cannot be uploaded to"
)]
fn a_branch_store_is_refused(publish: Publish) {
    let _ = publish;
}
