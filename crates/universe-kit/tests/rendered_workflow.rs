//! The commands the renderer writes have to be commands the binary accepts.
//!
//! `ci-check` proves the committed workflow matches the renderer, which is a statement about
//! two files agreeing — not about the job working. This test closes that gap for the one thing
//! a manifest edit can silently break: the *shape* of the publish invocation. It renders the
//! workflow from a manifest, takes the command line out of each publish job verbatim, resolves
//! the one GitHub expression in it to a local bare repository, and runs it.
//!
//! Nothing here reaches the network. A registry store is a directory, and a branch store is a
//! bare repository this test creates, so the assertion is about the argv the rendered job
//! issues rather than about anybody's hosting.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use universe_core::CommandOutput;
use universe_core::manifest::Manifest;
use universe_workflow::PUBLISH_FILE;

/// A project whose single package publishes to a directory and to a branch.
fn manifest(published: &Path) -> String {
    format!(
        r#"
schema = 1

[project]
name = "fixture-rendered"
platforms = ["linux-64"]

[[packages]]
name = "example-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-airlock --version"]
publish = ["local-store", "channel-branch"]

[[stores]]
name = "local-store"
kind = "filesystem"
url = "{published}"
writable = true
auth = "token"

[[stores]]
name = "channel-branch"
kind = "branch"
writable = true
auth = "token"
"#,
        published = published.display()
    )
}

struct Fixture {
    directory: TempDir,
    manifest_text: String,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temp dir");
        let published = directory.path().join("published");
        let manifest_text = manifest(&published);
        std::fs::write(directory.path().join("universe.toml"), &manifest_text).expect("manifest");
        let fixture = Self {
            directory,
            manifest_text,
        };
        fixture.add_package();
        fixture
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn published(&self) -> PathBuf {
        self.root().join("published")
    }

    /// A bare repository standing in for the remote a branch job pushes to.
    fn remote(&self) -> PathBuf {
        let path = self.root().join("remote.git");
        let status = Command::new("git")
            .args(["init", "--bare", "--quiet"])
            .arg(&path)
            .status()
            .expect("git is installed");
        assert!(status.success(), "the bare repository is created");
        path
    }

    fn add_package(&self) {
        let relative = "output/example-airlock-1.0.0-0.conda";
        let path = self.root().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("output dir");
        std::fs::write(&path, b"conda bytes").expect("artifact");
        let digest = universe_workflow::sha256::sha256_hex(b"conda bytes");
        let fragments = self.root().join("output/fragments");
        std::fs::create_dir_all(&fragments).expect("fragment dir");
        std::fs::write(
            fragments.join("example-airlock-linux-64.json"),
            format!(
                r#"{{
  "package": "example-airlock",
  "version": "1.0.0",
  "build": "0",
  "build_number": 0,
  "subdir": "linux-64",
  "digest": "{digest}",
  "size_bytes": 11,
  "depends": [],
  "license": "MIT",
  "license_files": [],
  "path": "{relative}"
}}
"#
            ),
        )
        .expect("fragment");
    }

    /// The publish command lines the renderer writes, in file order.
    fn rendered_commands(&self) -> Vec<String> {
        let parsed = Manifest::from_toml(&self.manifest_text).expect("the manifest parses");
        let output = universe_workflow::render(&parsed, &self.manifest_text)
            .expect("the workflow set renders");
        let publish = output
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("the publish file");
        publish
            .content
            .lines()
            .filter_map(|line| {
                line.split_once("universe -- ")
                    .map(|(_, command)| command.trim())
            })
            .filter(|command| command.starts_with("publish"))
            .map(str::to_owned)
            .collect()
    }

    /// Run one rendered command, with the hosting expression resolved to `remote`.
    fn run_rendered(&self, command: &str, remote: &Path) -> CommandOutput {
        let resolved = command.replace(
            "${{ github.server_url }}/${{ github.repository }}.git",
            &remote.display().to_string(),
        );
        let mut invocation: Vec<String> = resolved.split_whitespace().map(str::to_owned).collect();
        invocation.push("--root".to_owned());
        invocation.push(self.root().display().to_string());
        universe_kit::run(invocation)
    }
}

#[test]
fn the_rendered_publish_jobs_are_commands_the_binary_accepts() {
    let fixture = Fixture::new();
    let remote = fixture.remote();

    let commands = fixture.rendered_commands();
    assert_eq!(
        commands.len(),
        2,
        "one command per publishing store: {commands:?}"
    );
    assert!(
        commands[0].starts_with("publish-artifacts --store local-store"),
        "a filesystem store is uploaded to one file at a time: {}",
        commands[0]
    );
    assert!(
        commands[1].starts_with("publish --store channel-branch --remote"),
        "a branch store is pushed to, so the job names a remote: {}",
        commands[1]
    );

    for command in &commands {
        let output = fixture.run_rendered(command, &remote);
        assert_eq!(
            output.exit_code(),
            0,
            "the rendered command failed: {command}\nstderr: {}",
            output.stderr()
        );
    }
}

#[test]
fn the_rendered_jobs_leave_the_artifact_in_both_destinations() {
    let fixture = Fixture::new();
    let remote = fixture.remote();
    for command in fixture.rendered_commands() {
        let output = fixture.run_rendered(&command, &remote);
        assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    }

    assert_eq!(
        std::fs::read(
            fixture
                .published()
                .join("linux-64/example-airlock-1.0.0-0.conda")
        )
        .expect("the uploaded artifact"),
        b"conda bytes"
    );
    let branch = Command::new("git")
        .arg("-C")
        .arg(&remote)
        .args(["rev-parse", "--quiet", "--verify", "refs/heads/channel"])
        .output()
        .expect("git is installed");
    assert!(
        branch.status.success(),
        "the branch job published nothing: {}",
        String::from_utf8_lossy(&branch.stderr)
    );
}

#[test]
fn a_second_run_of_the_rendered_jobs_changes_nothing() {
    let fixture = Fixture::new();
    let remote = fixture.remote();
    for command in fixture.rendered_commands() {
        fixture.run_rendered(&command, &remote).expect_success();
    }
    let commits_before = commit_count(&remote);

    for command in fixture.rendered_commands() {
        let output = fixture.run_rendered(&command, &remote);
        assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
        if command.starts_with("publish-artifacts") {
            let json: serde_json::Value =
                serde_json::from_str(output.stdout().trim()).expect("a JSON envelope");
            assert_eq!(
                json["result"]["stores"][0]["uploaded"],
                serde_json::json!([]),
                "a republish uploads nothing: {json}"
            );
        }
    }
    assert_eq!(
        commits_before,
        commit_count(&remote),
        "a republish pushes no second commit"
    );
}

fn commit_count(remote: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(remote)
        .args(["rev-list", "--count", "channel"])
        .output()
        .expect("git is installed");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

trait ExpectSuccess {
    fn expect_success(self);
}

impl ExpectSuccess for CommandOutput {
    fn expect_success(self) {
        assert_eq!(self.exit_code(), 0, "stderr: {}", self.stderr());
    }
}
