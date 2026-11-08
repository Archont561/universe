//! Scenarios for the channel loop: publish to a branch, restore with Git.
//!
//! These complement `tests/channel.rs` rather than repeat it. The integration tests say
//! what a verb returns; a scenario says what a consumer observes — a branch holding a
//! channel, and a clone of it that a solver could read.
//!
//! Every scenario builds its own project *and* its own bare remote under a temporary
//! directory, so nothing here reads or writes a branch of this repository. The restore step
//! is a real `git clone`, because "restores with Git alone" is the claim, and a fake
//! transport would prove only that the fake works.
//!
//! Honest provenance: the publisher was built before this file existed. These are
//! acceptance scenarios written against behaviour that already shipped, so they record
//! what the loop does rather than what designing it first produced.

use std::path::{Path, PathBuf};
use std::process::Command;

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use tempfile::TempDir;
use universe_core::CommandOutput;

const BRANCH: &str = "channel";
const PACKAGE: &str = "example-airlock";
const ARTIFACT: &str = "output/example-airlock-1.0.0-0.conda";
const FRAGMENT: &str = "output/fragments/example-airlock-linux-64.json";

/// A project directory, the bare remote standing in for its channel branch, and what the
/// last verb did to either.
#[derive(Default)]
struct Channel {
    root: Option<TempDir>,
    remote: Option<TempDir>,
    output: Option<CommandOutput>,
    checkout: Option<TempDir>,
}

impl Channel {
    fn root(&self) -> &Path {
        self.root
            .as_ref()
            .expect("a scenario created its project directory")
            .path()
    }

    fn remote(&self) -> &Path {
        self.remote
            .as_ref()
            .expect("a scenario created its remote repository")
            .path()
    }

    fn output(&self) -> &CommandOutput {
        self.output.as_ref().expect("a verb ran")
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }

    /// Run one `universe` verb against this project.
    fn run(&mut self, arguments: &[&str]) -> &CommandOutput {
        let mut invocation: Vec<String> = arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect();
        invocation.push("--root".to_owned());
        invocation.push(self.root().display().to_string());
        let output = universe_kit::run(invocation);
        self.output = Some(output);
        self.output()
    }

    /// Publish to the scenario's remote, naming the store whose budget applies.
    ///
    /// The store is named because the ceiling belongs to the destination, and because this
    /// is the invocation the rendered publish job issues.
    fn publish(&mut self) -> &CommandOutput {
        self.publish_with(&[])
    }

    /// Publish with extra flags, in the order the rendered job would pass them.
    fn publish_with(&mut self, extra: &[&str]) -> &CommandOutput {
        let remote = self.remote().display().to_string();
        let mut arguments = vec![
            "publish",
            "--remote",
            remote.as_str(),
            "--branch",
            BRANCH,
            "--store",
            "channel-branch",
        ];
        arguments.extend_from_slice(extra);
        self.run(&arguments)
    }

    fn branch_exists(&self) -> bool {
        Command::new("git")
            .args(["-C"])
            .arg(self.remote())
            .args(["rev-parse", "--quiet", "--verify"])
            .arg(format!("refs/heads/{BRANCH}"))
            .output()
            .expect("git is installed")
            .status
            .success()
    }

    fn commit_count(&self) -> String {
        let output = Command::new("git")
            .args(["-C"])
            .arg(self.remote())
            .args(["rev-list", "--count", BRANCH])
            .output()
            .expect("git is installed");
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    fn read_checkout(&self, relative: &str) -> Vec<u8> {
        let path = self
            .checkout
            .as_ref()
            .expect("a consumer cloned the branch")
            .path()
            .join(relative);
        std::fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
    }
}

/// What every scenario starts with: an audited package, its fragment, and an empty remote.
fn audited(store_block: &str) -> Channel {
    let root = tempfile::tempdir().expect("temporary project directory");
    let manifest = format!(
        r#"
schema = 1

[project]
name = "fixture-channel"
platforms = ["linux-64"]

[[packages]]
name = "{PACKAGE}"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["{PACKAGE} --version"]
publish = ["channel-branch"]

[[stores]]
name = "channel-branch"
kind = "branch"
url = "https://example.invalid/universe.git"
writable = true
{store_block}"#
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
  "subdir": "linux-64",
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

    let remote = tempfile::tempdir().expect("temporary remote directory");
    let status = Command::new("git")
        .args(["init", "--bare", "--quiet"])
        .arg(remote.path())
        .status()
        .expect("git is installed");
    assert!(
        status.success(),
        "the scenario's remote is a bare repository"
    );

    Channel {
        root: Some(root),
        remote: Some(remote),
        output: None,
        checkout: None,
    }
}

#[fixture]
fn channel() -> Channel {
    Channel::default()
}

#[given("an audited package with a channel branch store")]
fn an_audited_package(channel: &mut Channel) {
    *channel = audited("");
}

#[given("an audited package with a channel branch store and two builds of one version")]
fn an_audited_package_with_two_builds(channel: &mut Channel) {
    *channel = audited("");
    let bytes = b"a second build";
    let relative = "output/example-airlock-1.0.0-1.conda";
    std::fs::write(channel.path(relative), bytes).expect("the second artifact");
    std::fs::write(
        channel.path("output/fragments/example-airlock-1-linux-64.json"),
        format!(
            r#"{{
  "package": "{PACKAGE}",
  "version": "1.0.0",
  "build": "1",
  "build_number": 1,
  "subdir": "linux-64",
  "digest": "{}",
  "size_bytes": {},
  "depends": [],
  "license": "MIT",
  "license_files": [],
  "path": "{relative}"
}}
"#,
            universe_workflow::sha256::sha256_hex(bytes),
            bytes.len()
        ),
    )
    .expect("the second fragment");
}

#[given("a channel whose store refuses snapshots above four bytes")]
fn a_channel_with_a_tiny_budget(channel: &mut Channel) {
    *channel = audited("\n[stores.budgets]\ntransport = 4\n");
}

#[given("the artifact is altered after the audit")]
fn alter_the_artifact(channel: &Channel) {
    // The bytes moved; the fragment beside them still describes what the audit measured.
    std::fs::write(channel.path(ARTIFACT), b"rebuilt bytes").expect("the artifact is writable");
}

#[given("the package is rebuilt and its fragment updated")]
fn rebuild_the_package(channel: &Channel) {
    // A rebuild writes new bytes and a new fragment together, so the fragment still tells
    // the truth about the artifact. The branch, published from the previous build, is now
    // the only thing that disagrees.
    let bytes = b"rebuilt bytes";
    std::fs::write(channel.path(ARTIFACT), bytes).expect("the artifact is writable");
    let fragment =
        std::fs::read_to_string(channel.path(FRAGMENT)).expect("the fragment is readable");
    let digest = universe_workflow::sha256::sha256_hex(bytes);
    let updated = fragment
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("\"digest\"") {
                format!("  \"digest\": \"{digest}\",")
            } else if line.trim_start().starts_with("\"size_bytes\"") {
                format!("  \"size_bytes\": {},", bytes.len())
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<String>>()
        .join("\n");
    std::fs::write(channel.path(FRAGMENT), format!("{updated}\n"))
        .expect("the fragment is writable");
}

// The same action appears as a `Given` in one scenario and as a `When` in another, because
// what it is depends on the story: preparation there, the change under test here. `And`
// inherits the keyword it follows, so the step is registered for both.
#[when("the artifact is altered after the audit")]
fn alter_the_artifact_during_the_scenario(channel: &Channel) {
    alter_the_artifact(channel);
}

#[when("the package is rebuilt and its fragment updated")]
fn rebuild_the_package_during_the_scenario(channel: &Channel) {
    rebuild_the_package(channel);
}

#[when("the channel is published")]
fn publish(channel: &mut Channel) {
    channel.publish();
}

#[when("the channel is published keeping one build per version")]
fn publish_keeping_one(channel: &mut Channel) {
    channel.publish_with(&["--keep", "1"]);
}

#[when("the channel is published again")]
fn publish_again(channel: &mut Channel) {
    channel.publish();
}

#[when("the channel is checked")]
fn check(channel: &mut Channel) {
    let remote = channel.remote().display().to_string();
    channel.run(&[
        "publish", "--check", "--remote", &remote, "--branch", BRANCH,
    ]);
}

#[when("a consumer clones the channel branch")]
fn clone_the_branch(channel: &mut Channel) {
    let checkout = tempfile::tempdir().expect("temporary checkout directory");
    let output = Command::new("git")
        .args(["clone", "--quiet", "--depth", "1", "--branch", BRANCH])
        .arg(channel.remote())
        .arg(checkout.path())
        .output()
        .expect("git is installed");
    assert!(
        output.status.success(),
        "the clone failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    channel.checkout = Some(checkout);
}

#[then("the branch holds the channel metadata and the package at the standard paths")]
fn branch_holds_the_channel(channel: &Channel) {
    assert_eq!(
        channel.output().exit_code(),
        0,
        "the publish failed: {}",
        channel.output().stderr()
    );
    let listing = Command::new("git")
        .args(["-C"])
        .arg(channel.remote())
        .args(["ls-tree", "-r", "--name-only", BRANCH])
        .output()
        .expect("git is installed");
    let paths = String::from_utf8_lossy(&listing.stdout);
    for path in [
        "linux-64/repodata.json",
        "noarch/repodata.json",
        "channeldata.json",
        "README.md",
        "linux-64/example-airlock-1.0.0-0.conda",
    ] {
        assert!(
            paths.contains(path),
            "the branch does not hold {path}:\n{paths}"
        );
    }
}

#[then("the clone's package matches the digest the fragment recorded")]
fn clone_matches_the_digest(channel: &Channel) {
    let bytes = channel.read_checkout("linux-64/example-airlock-1.0.0-0.conda");
    assert_eq!(
        universe_workflow::sha256::sha256_hex(&bytes),
        universe_workflow::sha256::sha256_hex(b"conda bytes"),
        "the cloned package is not the one the audit measured"
    );
    let metadata = String::from_utf8(channel.read_checkout("linux-64/repodata.json"))
        .expect("repodata is UTF-8");
    assert!(
        metadata.contains(&universe_workflow::sha256::sha256_hex(b"conda bytes")),
        "the cloned metadata does not name the digest of the package beside it"
    );
}

#[then("the branch holds the receipt under the metadata path")]
fn branch_holds_the_receipt(channel: &Channel) {
    assert_eq!(
        channel.output().exit_code(),
        0,
        "the publish failed: {}",
        channel.output().stderr()
    );
    let listing = Command::new("git")
        .args(["-C"])
        .arg(channel.remote())
        .args(["ls-tree", "-r", "--name-only", BRANCH])
        .output()
        .expect("git is installed");
    let paths = String::from_utf8_lossy(&listing.stdout);
    let receipt = "meta/linux-64/example-airlock-1.0.0-0.conda.json";
    assert!(
        paths.contains(receipt),
        "the branch does not hold {receipt}:\n{paths}"
    );
    // The receipt is the auditor's text, so the digests a client reads without downloading
    // the archive and the digests the metadata advertises have to be the same numbers.
    assert!(
        paths.contains("linux-64/example-airlock-1.0.0-0.conda"),
        "a receipt with no artifact beside it is worse than none:\n{paths}"
    );
}

#[then("the clone's receipt names the digest of the bytes beside it")]
fn clone_receipt_matches_the_bytes(channel: &Channel) {
    let receipt = String::from_utf8(
        channel.read_checkout("meta/linux-64/example-airlock-1.0.0-0.conda.json"),
    )
    .expect("the receipt is UTF-8");
    let receipt: serde_json::Value = serde_json::from_str(&receipt).expect("the receipt is JSON");
    let bytes = channel.read_checkout("linux-64/example-airlock-1.0.0-0.conda");

    assert_eq!(
        receipt["digest"],
        universe_workflow::sha256::sha256_hex(&bytes).as_str(),
        "the cloned receipt does not name the digest of the archive in the same tree"
    );
    assert_eq!(receipt["size_bytes"], bytes.len());
}

#[then("the branch holds only the newest build")]
fn only_the_newest_build(channel: &Channel) {
    assert_eq!(
        channel.output().exit_code(),
        0,
        "the publish failed: {}",
        channel.output().stderr()
    );
    let listing = Command::new("git")
        .args(["-C"])
        .arg(channel.remote())
        .args(["ls-tree", "-r", "--name-only", BRANCH])
        .output()
        .expect("git is installed");
    let paths = String::from_utf8_lossy(&listing.stdout);
    assert!(
        paths.contains("linux-64/example-airlock-1.0.0-1.conda"),
        "the newest build is missing:\n{paths}"
    );
    assert!(
        !paths.contains("linux-64/example-airlock-1.0.0-0.conda"),
        "a build retention left out is in the tree:\n{paths}"
    );
}

#[then("the publish says the branch bounds what it serves rather than what it stores")]
fn retention_is_stated_honestly(channel: &Channel) {
    let stdout = channel.output().stdout();
    assert!(
        stdout.contains("bounds what it serves"),
        "the report does not say what a force-push bounds: {stdout}"
    );
    assert!(
        stdout.contains("does not delete"),
        "the report implies it reclaimed storage: {stdout}"
    );
}

#[then("the branch holds exactly one commit")]
fn one_commit(channel: &Channel) {
    assert_eq!(
        channel.output().exit_code(),
        0,
        "a republish over identical artifacts must not fail: {}",
        channel.output().stderr()
    );
    assert_eq!(
        channel.commit_count(),
        "1",
        "a republish added history to a branch that is meant to be a snapshot"
    );
}

#[then("the publish names the artifact it cannot vouch for")]
fn publish_names_the_artifact(channel: &Channel) {
    assert_eq!(channel.output().exit_code(), 1, "the publish must fail");
    assert!(
        channel.output().stderr().contains(ARTIFACT),
        "the failure does not name the artifact: {}",
        channel.output().stderr()
    );
}

#[then("the branch does not exist")]
fn nothing_was_published(channel: &Channel) {
    assert!(
        !channel.branch_exists(),
        "a refused publish still created the branch"
    );
}

#[then("the publish is refused before anything is pushed")]
fn publish_is_refused(channel: &Channel) {
    assert_eq!(channel.output().exit_code(), 1, "the publish must fail");
    assert!(
        channel
            .output()
            .stderr()
            .contains("publish.budget.exceeded"),
        "the refusal is not a budget finding: {}",
        channel.output().stderr()
    );
}

#[then("the check reports drift against the fragments")]
fn check_reports_drift(channel: &Channel) {
    assert_eq!(
        channel.output().exit_code(),
        1,
        "a drifted branch must fail the check"
    );
    assert!(
        channel.output().stderr().contains("publish.check.drift"),
        "the check did not report drift: {}",
        channel.output().stderr()
    );
}

// One binding per scenario. `#[scenario]` without a name takes the first scenario it finds,
// so each function names the one it drives.
#[scenario(
    path = "tests/features/channel.feature",
    name = "An audited package is published to the branch and restored with Git alone"
)]
fn a_published_channel_is_restorable(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "tests/features/channel.feature",
    name = "The branch carries each artifact's audit receipt"
)]
fn the_receipt_travels_with_the_artifact(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "tests/features/channel.feature",
    name = "Publishing the same artifacts twice pushes no second commit"
)]
fn a_republish_is_a_no_op(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "tests/features/channel.feature",
    name = "An artifact that no longer matches its fragment stops the publish"
)]
fn a_tampered_artifact_refuses_publication(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "tests/features/channel.feature",
    name = "A store that refuses snapshots above four bytes publishes nothing"
)]
fn a_budget_refuses_a_snapshot(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "tests/features/channel.feature",
    name = "Retention keeps the newest builds and says what that bounded"
)]
fn retention_is_declared(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "tests/features/channel.feature",
    name = "A branch that no longer matches its fragments is reported as drift"
)]
fn drift_is_reported(channel: Channel) {
    let _ = channel;
}
