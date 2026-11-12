//! Publishing the stable bootstrap ref, and running the entrypoint against it.
//!
//! The rendered entrypoint clones a pinned ref and executes `universe-airlock-bootstrap` from
//! that checkout (`doc-103`, "publish bootstrap content and advance stable bootstrap ref last").
//! A Conda package in the channel is not that ref: a solver can install it, but nothing can
//! `exec` it from a checkout. This file is about the artifact the entrypoint actually needs —
//! a ref whose root holds the executable, published once per binary and not once per build.
//!
//! Everything here runs against a bare repository under a `TempDir`: the verb really pushes
//! with `git`, and the entrypoint really clones what it pushed. No test reaches the network.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// The ref a disconnected host clones, and the one the entrypoint names.
const BRANCH: &str = "bootstrap";

/// The bootstrapper's file name inside a published ref. This is the default the manifest and
/// the entrypoint share, so a mismatch here is a generated script nobody can run.
const BOOTSTRAP: &str = "universe-airlock-bootstrap";

/// The descriptor published beside the binary.
const DESCRIPTOR: &str = "bootstrap.json";

/// A stub that says what it was handed, which is the entrypoint's whole contract.
const STUB: &[u8] =
    b"#!/bin/sh\nprintf 'stub-bootstrap' >&2\nprintf ' %s' \"$@\" >&2\nprintf '\\n' >&2\n";

/// A bare remote, and a project directory to render into.
struct Fixture {
    directory: TempDir,
    remote: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temp dir");
        let remote = directory.path().join("remote.git");
        git(
            &["init", "--bare", "--quiet", &remote.display().to_string()],
            directory.path(),
        );
        Self { directory, remote }
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    /// Publish `bytes` as the bootstrapper through the verb under test.
    fn publish(&self, bytes: &[u8]) -> universe_core::CommandOutput {
        let binary = self.root().join("built-bootstrapper");
        std::fs::write(&binary, bytes).expect("write the built bootstrapper");
        universe_kit::run([
            "airlock",
            "publish-bootstrap",
            "--binary",
            &binary.display().to_string(),
            "--remote",
            &self.remote.display().to_string(),
            "--branch",
            BRANCH,
            "--json",
        ])
    }

    /// Publish with no `--branch` and no manifest, which is the only way to observe the
    /// default the verb would use for a project that declares nothing.
    fn publish_without_branch(&self, bytes: &[u8]) -> universe_core::CommandOutput {
        let binary = self.root().join("unbuilt-bootstrapper");
        std::fs::write(&binary, bytes).expect("write the built bootstrapper");
        universe_kit::run([
            "airlock",
            "publish-bootstrap",
            "--binary",
            &binary.display().to_string(),
            "--remote",
            &self.remote.display().to_string(),
            "--json",
        ])
    }

    /// Every head the bare remote holds.
    fn refs(&self) -> String {
        let output = Command::new("git")
            .args(["ls-remote", "--heads", &self.remote.display().to_string()])
            .output()
            .expect("git is installed");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// A project whose manifest pins `commit` on this remote.
    fn project(&self, commit: &str) -> PathBuf {
        let project = self.root().join("project");
        std::fs::create_dir_all(&project).expect("project");
        std::fs::write(
            project.join("universe.toml"),
            format!(
                r#"
schema = 1

[project]
name = "fixture"
platforms = ["linux-64"]

[airlock.developer]

[airlock.developer.restore]
remote = "{remote}"
branch = "{BRANCH}"
commit = "{commit}"
"#,
                remote = self.remote.display()
            ),
        )
        .expect("manifest");
        project
    }
}

/// Run one git command in `directory`, returning stdout with the trailing newline trimmed.
fn git(arguments: &[&str], directory: &Path) -> String {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_owned()
}

/// The `(mode, path)` pairs one ref holds, from the remote's own object store.
fn tree(remote: &Path, commit: &str) -> Vec<(String, String)> {
    git(&["ls-tree", commit], remote)
        .lines()
        .filter(|line| line.contains('\t'))
        .map(|line| {
            let (head, path) = line.split_once('\t').expect("a tree entry");
            let mode = head.split_whitespace().next().expect("a mode").to_owned();
            (mode, path.to_owned())
        })
        .collect()
}

/// The commit a ref points at on the remote.
fn ref_commit(remote: &Path) -> String {
    git(&["rev-parse", &format!("refs/heads/{BRANCH}")], remote)
}

#[test]
fn the_verb_publishes_an_executable_and_a_descriptor() {
    let fixture = Fixture::new();
    let output = fixture.publish(STUB);

    assert_eq!(
        output.exit_code(),
        0,
        "publish-bootstrap failed\nstdout: {}\nstderr: {}",
        output.stdout(),
        output.stderr()
    );

    let commit = ref_commit(&fixture.remote);
    let entries = tree(&fixture.remote, &commit);
    assert_eq!(
        entries,
        vec![
            ("100644".to_owned(), DESCRIPTOR.to_owned()),
            ("100755".to_owned(), BOOTSTRAP.to_owned())
        ],
        "the ref must hold the descriptor and an executable bootstrapper, in that order"
    );

    // The entrypoint's `[ -x ... ]` test is the read that matters, and the mode above is what
    // makes it true. Reading it back through git is how a clone would see it.
    let descriptor = git(
        &["cat-file", "-p", &format!("{commit}:{DESCRIPTOR}")],
        &fixture.remote,
    );
    let digest = universe_workflow::sha256::sha256_hex(STUB);
    assert!(
        descriptor.contains(&digest),
        "the descriptor does not name the binary's digest: {descriptor}"
    );
    assert!(
        descriptor.contains(&format!("\"size_bytes\":{}", STUB.len())),
        "the descriptor does not name the binary's size: {descriptor}"
    );
    assert!(
        descriptor.contains(universe_core::VERSION),
        "the descriptor does not name the version it was built from: {descriptor}"
    );
}

#[test]
fn republishing_the_same_binary_leaves_the_ref_where_it_was() {
    let fixture = Fixture::new();
    assert_eq!(fixture.publish(STUB).exit_code(), 0);
    let first = ref_commit(&fixture.remote);

    let second = fixture.publish(STUB);
    assert_eq!(second.exit_code(), 0);

    // The pin in a consumer's committed entrypoint is this commit; a republish that moved the
    // ref for identical bytes would rewrite every consumer's script for nothing.
    assert_eq!(
        ref_commit(&fixture.remote),
        first,
        "an unchanged binary produced a new commit"
    );
    // And the report still names the pin, because that is the value the caller pastes into the
    // manifest. An "unchanged" that prints no commit leaves them looking it up by hand.
    assert!(
        second.stdout().contains(&first),
        "the unchanged report does not name the commit it left alone: {}",
        second.stdout()
    );
}

#[test]
fn a_changed_binary_moves_the_ref() {
    let fixture = Fixture::new();
    assert_eq!(fixture.publish(STUB).exit_code(), 0);
    let first = ref_commit(&fixture.remote);

    let changed = b"#!/bin/sh\nprintf 'stub-bootstrap 2\\n' >&2\n";
    assert_eq!(fixture.publish(changed).exit_code(), 0);

    let second = ref_commit(&fixture.remote);
    assert_ne!(second, first, "a changed binary did not move the ref");
}

#[test]
fn the_rendered_entrypoint_runs_what_the_verb_published() {
    // The two halves are written by different commands and pulled together by a manifest, so
    // the risk is that they agree on everything except the file name or the mode. Running the
    // rendered script against the published ref is the only check that sees both.
    let fixture = Fixture::new();
    assert_eq!(fixture.publish(STUB).exit_code(), 0);
    let commit = ref_commit(&fixture.remote);
    let project = fixture.project(&commit);

    let rendered = universe_kit::run([
        "airlock",
        "render",
        "--root",
        &project.display().to_string(),
    ]);
    assert_eq!(rendered.exit_code(), 0, "{}", rendered.stderr());

    let cache = fixture.root().join("cache");
    let output = Command::new("sh")
        .arg(project.join(".universe-airlock/restore.sh"))
        .current_dir(&project)
        .env("UNIVERSE_AIRLOCK_HOME", &cache)
        .env("HOME", cache.join("home"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("sh runs the rendered entrypoint");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("stub-bootstrap restore --commit {commit}")),
        "the entrypoint did not hand over to what the verb published\nstderr: {stderr}"
    );
}

/// The default ref is namespaced, because `bootstrap` at the repository root competes for the
/// same namespace every project uses and says nothing about who owns it.
#[test]
fn publishing_without_a_declared_ref_uses_the_namespaced_default() {
    let fixture = Fixture::new();

    let output = fixture.publish_without_branch(STUB);

    assert_eq!(
        output.exit_code(),
        0,
        "the publish succeeded: {}",
        output.stderr()
    );
    assert!(
        fixture.refs().contains("refs/heads/universe/bootstrap"),
        "the default ref is not the namespaced one: {}",
        fixture.refs()
    );
}
