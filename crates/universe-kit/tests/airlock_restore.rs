//! The rendered entrypoint, run.
//!
//! `universe airlock render` can be checked for shape by reading its output; whether the
//! script *restores* is only answerable by running it. This test builds the smallest honest
//! version of the loop: a real Git remote holding a pinned ref, a real cache directory, real
//! `git`, and the rendered script executed by `sh`. The bootstrapper in the ref is a stub —
//! the real one is `UNI-33`, and pretending otherwise would make this test measure a binary
//! that does not exist — but everything the entrypoint is responsible for is real: where it
//! fetches from, that it fetches exactly the pinned commit, that a rewritten ref fails, and
//! that a checkout without a bootstrapper fails rather than silently succeeding.
//!
//! Nothing here reaches the network: the remote is a bare repository under a `TempDir`, and
//! no test touches this repository or the user's `HOME`.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use universe_core::CommandOutput;
use universe_core::manifest::Manifest;

/// The branch the fixture publishes to, and the ref the entrypoint names.
const BRANCH: &str = "mirror/pixi";

/// The bootstrapper's file name inside a published ref.
const BOOTSTRAP: &str = "universe-airlock-bootstrap";

/// A project, a remote, and the commit the manifest pins.
struct Fixture {
    directory: TempDir,
    remote: PathBuf,
    commit: String,
}

impl Fixture {
    /// A project whose manifest pins the one commit on the fixture's remote.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temp dir");
        let remote = directory.path().join("remote.git");
        git(
            &["init", "--bare", "--quiet", &remote.display().to_string()],
            directory.path(),
        );
        let commit = publish(&directory, &remote, true);
        let fixture = Self {
            directory,
            remote,
            commit,
        };
        std::fs::write(fixture.root().join("universe.toml"), fixture.manifest()).expect("manifest");
        fixture
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn manifest(&self) -> String {
        format!(
            r#"
schema = 1

[project]
name = "fixture-restore"
platforms = ["linux-64"]

[[packages]]
name = "fixture-restore"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/*.conda"]
release = false

[airlock.developer]
adapters = ["pixi"]
mirror = "mirror"

[airlock.developer.restore]
remote = "{}"
branch = "{BRANCH}"
commit = "{}"

[workflow.checks]
format = "cargo fmt --check"
"#,
            self.remote.display(),
            self.commit
        )
    }

    /// Render the entrypoint into the project.
    fn render(&self) -> CommandOutput {
        universe_kit::run([
            "airlock",
            "render",
            "--root",
            &self.root().display().to_string(),
        ])
    }

    /// Run the rendered POSIX entrypoint, with a cache of its own and the caller's `PATH`.
    fn run_entrypoint(&self, cache: &Path) -> std::process::Output {
        Command::new("sh")
            .arg(self.root().join(".universe-airlock/restore.sh"))
            .env("UNIVERSE_AIRLOCK_HOME", cache)
            .env("HOME", cache.join("home"))
            // The machine's own git configuration is not part of the contract.
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .expect("sh runs the rendered entrypoint")
    }
}

/// Run one git command in `directory`, with an identity the fixture controls.
fn git(arguments: &[&str], directory: &Path) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@invalid",
        ])
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("git is installed");
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Build the pinned ref on a bare remote, and answer the commit it points at.
///
/// `with_bootstrapper` is what distinguishes the two fixtures this file needs: a ref that was
/// published properly, and one that carries the tree but not the executable the entrypoint
/// hands over to.
fn publish(directory: &TempDir, remote: &Path, with_bootstrapper: bool) -> String {
    let work = directory.path().join("work");
    std::fs::create_dir_all(&work).expect("work tree");
    git(&["init", "--quiet", "--initial-branch", BRANCH], &work);
    std::fs::write(
        work.join("airlock.toml"),
        "payload = \"the fixture's pinned content\"\n",
    )
    .expect("payload");
    if with_bootstrapper {
        let script = work.join(BOOTSTRAP);
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf 'stub-bootstrap' >&2\nprintf ' %s' \"$@\" >&2\nprintf '\\n' >&2\n",
        )
        .expect("bootstrapper");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
                .expect("executable");
        }
    }
    git(&["add", "--all"], &work);
    git(&["commit", "--quiet", "-m", "publish"], &work);
    // Pushed by ref name, so the bare remote does not need a HEAD.
    git(
        &[
            "push",
            "--quiet",
            &remote.display().to_string(),
            &format!("HEAD:refs/heads/{BRANCH}"),
        ],
        &work,
    );
    git(&["rev-parse", "HEAD"], &work)
}

/// A manifest pinning `commit` on the fixture's remote.
fn manifest_for(remote: &Path, commit: &str) -> String {
    format!(
        r#"
schema = 1

[project]
name = "fixture-restore"
platforms = ["linux-64"]

[[packages]]
name = "fixture-restore"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/*.conda"]
release = false

[airlock.developer]
adapters = ["pixi"]

[airlock.developer.restore]
remote = "{}"
branch = "{BRANCH}"
commit = "{commit}"
"#,
        remote.display()
    )
}

/// A `pixi`-style cache directory for one test.
fn cache(directory: &TempDir) -> PathBuf {
    let path = directory.path().join("airlock-home");
    std::fs::create_dir_all(&path).expect("cache");
    path
}

#[test]
fn the_entrypoint_clones_the_pin_and_hands_over_to_the_bootstrapper() {
    let fixture = Fixture::new();
    assert_eq!(fixture.render().exit_code(), 0);
    let cache = cache(&fixture.directory);

    let output = fixture.run_entrypoint(&cache);

    assert!(
        output.status.success(),
        "the entrypoint failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("restore --commit {}", fixture.commit)),
        "the bootstrapper was not handed the pinned commit: {stderr}"
    );
    assert!(stderr.contains("stub-bootstrap"), "{stderr}");

    // The checkout is where the script said it would be, at the commit it pinned.
    let checkout = cache.join(&fixture.commit);
    assert!(
        checkout.join(".git").is_dir(),
        "no checkout at {}",
        checkout.display()
    );
    assert_eq!(
        git(&["rev-parse", "HEAD"], &checkout),
        fixture.commit,
        "the checkout is not at the pinned commit"
    );
}

#[test]
fn a_second_run_reuses_the_cached_checkout_instead_of_cloning_again() {
    let fixture = Fixture::new();
    assert_eq!(fixture.render().exit_code(), 0);
    let cache = cache(&fixture.directory);
    assert!(fixture.run_entrypoint(&cache).status.success());

    // With the remote gone, a second clone would fail: success proves the cache was reused,
    // which is what makes the entrypoint usable on a host that only reaches the remote once.
    std::fs::rename(&fixture.remote, fixture.root().join("remote-moved.git")).expect("move");

    let output = fixture.run_entrypoint(&cache);

    assert!(
        output.status.success(),
        "the cached checkout was not reused: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_ref_that_no_longer_holds_the_pinned_commit_fails_rather_than_running_something_else() {
    // The ref exists and the tree is fine; the commit the entrypoint names is not in it. That
    // is what a force-pushed mirror branch looks like to a consumer who kept an old script.
    let directory = tempfile::tempdir().expect("temp dir");
    let remote = directory.path().join("remote.git");
    git(
        &["init", "--bare", "--quiet", &remote.display().to_string()],
        directory.path(),
    );
    publish(&directory, &remote, true);
    let missing = "0".repeat(40);

    let project = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        project.path().join("universe.toml"),
        manifest_for(&remote, &missing),
    )
    .expect("manifest");
    assert_eq!(
        universe_kit::run([
            "airlock",
            "render",
            "--root",
            &project.path().display().to_string()
        ])
        .exit_code(),
        0
    );

    let output = Command::new("sh")
        .arg(project.path().join(".universe-airlock/restore.sh"))
        .env("UNIVERSE_AIRLOCK_HOME", project.path().join("cache"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("sh runs the entrypoint");

    assert!(!output.status.success(), "a replaced pin must not succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not hold commit"), "{stderr}");
    assert!(
        stderr.contains(&missing),
        "the failure does not name the pin: {stderr}"
    );
    // Nothing ran: the stub bootstrapper prints its name, and it was never reached.
    assert!(!stderr.contains("stub-bootstrap"), "{stderr}");
}

#[test]
fn a_ref_published_without_the_bootstrapper_is_named_rather_than_run() {
    let directory = tempfile::tempdir().expect("temp dir");
    let remote = directory.path().join("remote.git");
    git(
        &["init", "--bare", "--quiet", &remote.display().to_string()],
        directory.path(),
    );
    let commit = publish(&directory, &remote, false);

    let project = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        project.path().join("universe.toml"),
        manifest_for(&remote, &commit),
    )
    .expect("manifest");
    assert_eq!(
        universe_kit::run([
            "airlock",
            "render",
            "--root",
            &project.path().display().to_string()
        ])
        .exit_code(),
        0
    );

    let output = Command::new("sh")
        .arg(project.path().join(".universe-airlock/restore.sh"))
        .env("UNIVERSE_AIRLOCK_HOME", project.path().join("cache"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("sh runs the entrypoint");

    assert!(!output.status.success(), "a missing bootstrapper must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(BOOTSTRAP), "{stderr}");
    assert!(stderr.contains("not an executable"), "{stderr}");
}

#[test]
fn the_entrypoint_passes_the_repository_shell_lint() {
    let fixture = Fixture::new();
    assert_eq!(fixture.render().exit_code(), 0);
    let shellcheck = shellcheck();

    let output = Command::new(shellcheck)
        .arg(fixture.root().join(".universe-airlock/restore.sh"))
        .output()
        .expect("shellcheck runs");

    assert!(
        output.status.success(),
        "shellcheck rejected the rendered entrypoint:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// `shellcheck`, from `PATH` or from the environment this repository pins it in.
///
/// The gates run under `pixi run`, which puts it on `PATH`; a test can also be run from a
/// shell that has not been activated, and the pinned tool is still there.
fn shellcheck() -> PathBuf {
    if Command::new("shellcheck")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        return PathBuf::from("shellcheck");
    }
    let pinned =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.pixi/envs/default/bin/shellcheck");
    assert!(
        pinned.exists(),
        "no shellcheck on PATH and none at {}: the lint the manifest claims cannot be skipped",
        pinned.display()
    );
    pinned
}

#[test]
fn the_entrypoint_and_the_rendered_workflow_come_from_one_manifest() {
    let fixture = Fixture::new();
    let text = std::fs::read_to_string(fixture.root().join("universe.toml")).expect("manifest");
    let parsed = Manifest::from_toml(&text).expect("the manifest parses");

    // The workflow that regenerates the entrypoint is the one the renderer writes...
    let rendered = universe_workflow::render(&parsed, &text).expect("the workflow set renders");
    let airlock = rendered
        .files
        .iter()
        .find(|file| file.path.contains("universe-airlock"))
        .expect("an airlock workflow file")
        .content
        .clone();
    assert!(
        airlock.contains("airlock render"),
        "nothing regenerates the entrypoint in CI:\n{airlock}"
    );

    // ...and the commit the entrypoint names is the manifest's, so the two cannot disagree
    // about the pin without one of them failing its own drift check.
    assert_eq!(fixture.render().exit_code(), 0);
    let script = std::fs::read_to_string(fixture.root().join(".universe-airlock/restore.sh"))
        .expect("script");
    let pin = parsed
        .airlock
        .as_ref()
        .and_then(|airlock| airlock.developer.as_ref())
        .and_then(|developer| developer.restore.as_ref())
        .expect("the fixture declares a restore scope");
    assert!(script.contains(&format!("PIN_COMMIT='{}'", pin.commit)));
    assert!(script.contains(&format!("PIN_BRANCH='{}'", pin.branch)));

    // Changing the pin in the manifest makes the committed entrypoint drift, which is the
    // check that keeps a hand edit from surviving review.
    let moved = text.replace(&pin.commit, &"1".repeat(40));
    std::fs::write(fixture.root().join("universe.toml"), moved).expect("manifest");
    let checked = universe_kit::run([
        "airlock",
        "render",
        "--check",
        "--root",
        &fixture.root().display().to_string(),
    ]);
    assert_eq!(checked.exit_code(), 1, "stderr: {}", checked.stderr());
    assert!(
        checked.stdout().contains("PIN_COMMIT"),
        "{}",
        checked.stdout()
    );
}
