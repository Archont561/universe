//! The rendered entrypoint, run.
//!
//! `universe airlock render` can be checked for shape by reading its output; whether the
//! script *restores* is only answerable by running it. This test builds the smallest honest
//! version of the loop: a real Git remote holding a pinned ref, a real cache directory, real
//! `git`, and the rendered script executed by `sh`. Most tests publish a stub bootstrapper, on
//! purpose: they are about what the *entrypoint* is responsible for — where it fetches from,
//! that it fetches exactly the pinned commit, that a rewritten ref fails, and that a checkout
//! without a bootstrapper fails rather than silently succeeding. That the stub is handed
//! `restore --commit <pin>` is the whole contract there.
//!
//! One test publishes the **real** `universe-airlock-bootstrap` binary this workspace built and
//! puts a real payload in the project's working directory, so the rendered script and the real
//! binary restore in one process tree. The binary is found next to this crate's own binary in
//! Cargo's build directory — the same directory the bootstrapper's own integration tests get
//! theirs from — rather than through a checked-in copy or a hard-coded `target/debug` path.
//! That is why the workspace tests, not a single-package run, are the supported way to run this
//! file; `pixi run test` and `pixi run coverage` both build every binary.
//!
//! Nothing here reaches the network: the remote is a bare repository under a `TempDir`, and
//! no test touches this repository or the user's `HOME`.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use universe_core::CommandOutput;
use universe_core::manifest::Manifest;
use universe_workflow::sha256::sha256_hex;

mod support;

/// The branch the fixture publishes to, and the ref the entrypoint names.
const BRANCH: &str = "mirror/pixi";

/// The bootstrapper's file name inside a published ref.
const BOOTSTRAP: &str = "universe-airlock-bootstrap";

/// The one artifact the real bootstrapper installs, and the executable it must produce.
const PAYLOAD_ARTIFACT: &str = "airlock-0.1.0.conda";

/// What the artifact's prefix must be able to run once the restore is done.
const PAYLOAD_EXECUTABLE: &str = "bin/airlock";
const PAYLOAD_SCRIPT: &[u8] = b"#!/bin/sh\nprintf 'airlock 0.1.0\\n'\n";

/// A real `.conda` holding one executable, since a restore installs a prefix from it.
///
/// The archive is built through the shared fixture writer rather than checked in, and it is
/// deterministic: stored members, fixed tar timestamps.
fn payload_archive() -> Vec<u8> {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let path = scratch.path().join(PAYLOAD_ARTIFACT);
    let index = br#"{"name":"airlock","version":"0.1.0","build":"h1234567_0","build_number":0,"subdir":"linux-64","depends":[]}"#;
    let paths = format!(
        r#"{{"paths":[{{"_path":"{PAYLOAD_EXECUTABLE}","path_type":"hardlink","sha256":"{}","size_in_bytes":{}}}],"paths_version":1}}"#,
        sha256_hex(PAYLOAD_SCRIPT),
        PAYLOAD_SCRIPT.len()
    );
    support::write_conda_with_payload(
        &path,
        &[
            ("info/index.json", index.as_slice()),
            ("info/paths.json", paths.as_bytes()),
        ],
        &[(PAYLOAD_EXECUTABLE, PAYLOAD_SCRIPT)],
    );
    std::fs::read(&path).expect("read the archive")
}

/// A project, a remote, and the commit the manifest pins.
struct Fixture {
    directory: TempDir,
    remote: PathBuf,
    commit: String,
}

impl Fixture {
    /// A project whose manifest pins the one commit on the fixture's remote.
    fn new() -> Self {
        Self::with(Bootstrapper::Stub)
    }

    /// The same project, but the pinned ref carries the real bootstrapper binary.
    fn with_real_bootstrapper() -> Self {
        Self::with(Bootstrapper::Real)
    }

    fn with(bootstrapper: Bootstrapper) -> Self {
        let directory = tempfile::tempdir().expect("temp dir");
        let remote = directory.path().join("remote.git");
        git(
            &["init", "--bare", "--quiet", &remote.display().to_string()],
            directory.path(),
        );
        let commit = publish(&directory, &remote, bootstrapper);
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
            // A consumer runs the committed script from the project, not from the repository
            // that builds it. The generated entrypoint never changes the working directory.
            .current_dir(self.root())
            .env("UNIVERSE_AIRLOCK_HOME", cache)
            .env("HOME", cache.join("home"))
            .env_remove("UNIVERSE_AIRLOCK_STATE")
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

/// What the published ref carries at the bootstrapper's file name.
enum Bootstrapper {
    /// A shell script that only reports its arguments; enough to prove the handover.
    Stub,
    /// The real binary the workspace built, so one run can restore for real.
    Real,
    /// No file at all: a ref published without the executable it must hand over to.
    Absent,
}

/// The real bootstrap binary, built into the directory Cargo puts this crate's binary in.
///
/// `cargo nextest run --workspace` builds every binary of the workspace, and the bootstrapper's
/// own integration tests ask for theirs through `CARGO_BIN_EXE_...`; deriving the path from our
/// own binary is what makes it hold under `cargo llvm-cov`, which uses a different target
/// directory. A checked-in copy or a fixed `target/debug` path would prove nothing about the
/// binary the gates actually ship.
fn real_bootstrapper() -> PathBuf {
    let path = Path::new(env!("CARGO_BIN_EXE_universe-kit"))
        .parent()
        .expect("Cargo puts binaries in a directory")
        .join(BOOTSTRAP);
    assert!(
        path.is_file(),
        "the real bootstrapper was not built at {}; run the workspace test task, not one package",
        path.display()
    );
    path
}

/// Build the pinned ref on a bare remote, and answer the commit it points at.
///
/// `bootstrapper` distinguishes the three fixtures this file needs: a ref published with a stub,
/// one published with the real binary, and one carrying the tree but not the executable the
/// entrypoint hands over to.
fn publish(directory: &TempDir, remote: &Path, bootstrapper: Bootstrapper) -> String {
    let work = directory.path().join("work");
    std::fs::create_dir_all(&work).expect("work tree");
    git(&["init", "--quiet", "--initial-branch", BRANCH], &work);
    std::fs::write(
        work.join("airlock.toml"),
        "payload = \"the fixture's pinned content\"\n",
    )
    .expect("payload");
    let bytes = match bootstrapper {
        Bootstrapper::Stub => Some(
            b"#!/bin/sh\nprintf 'stub-bootstrap' >&2\nprintf ' %s' \"$@\" >&2\nprintf '\\n' >&2\n"
                .to_vec(),
        ),
        Bootstrapper::Real => {
            Some(std::fs::read(real_bootstrapper()).expect("read the real bootstrapper"))
        }
        Bootstrapper::Absent => None,
    };
    if let Some(bytes) = bytes {
        let script = work.join(BOOTSTRAP);
        std::fs::write(&script, bytes).expect("bootstrapper");
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

/// Put a payload the real bootstrapper can restore in the project's working directory.
///
/// The generated entrypoint execs the binary from the cloned checkout but does not change the
/// working directory, so the payload's default root is the project's `.universe-airlock/payload`,
/// not the checkout's. That is also the only place it can be: a release manifest that named the
/// commit of the very tree containing itself could never be written. Its `commit` is the pin the
/// entrypoint verified, which is what `restore --commit <pin>` checks against.
fn write_payload(fixture: &Fixture) -> Vec<u8> {
    let payload = fixture.root().join(".universe-airlock").join("payload");
    std::fs::create_dir_all(&payload).expect("payload directory");
    let artifact = payload_archive();
    std::fs::write(payload.join(PAYLOAD_ARTIFACT), &artifact).expect("artifact");
    // The digest is computed by the renderer's hasher, not by the bootstrapper's `sha2`, so a
    // mistake in either implementation fails this test instead of agreeing with itself.
    let manifest = format!(
        r#"{{"schema":1,"version":"0.1.0","commit":"{}","packages":[
             {{"name":"universe-airlock","kind":"conda","version":"0.1.0","artifacts":[
               {{"filename":"{PAYLOAD_ARTIFACT}","sha256":"{}","size":{},
                 "stores":["conda-static"]}}]}}]}}"#,
        fixture.commit,
        sha256_hex(&artifact),
        artifact.len()
    );
    std::fs::write(payload.join("universe-release.json"), manifest).expect("release manifest");
    artifact
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
fn the_rendered_entrypoint_restores_a_payload_with_the_real_bootstrapper() {
    // Both halves of the handover in one process tree: the rendered script clones the pin and
    // execs the binary this workspace built, and that binary verifies and materialises a real
    // payload. The stub bootstrapper the other tests use cannot do any of the second half.
    let fixture = Fixture::with_real_bootstrapper();
    assert_eq!(fixture.render().exit_code(), 0);
    let artifact = write_payload(&fixture);
    let cache = cache(&fixture.directory);

    let output = fixture.run_entrypoint(&cache);

    assert!(
        output.status.success(),
        "the entrypoint failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("restored: 0.1.0"), "{stdout}\n{stderr}");
    // The developer lane is the default because no release key is published yet; the real
    // binary says so rather than silently treating an unsigned payload as sealed.
    assert!(stderr.contains("[airlock.signature.absent]"), "{stderr}");

    // What the binary wrote, not just what it printed: one immutable environment that
    // `current` points at, holding the exact bytes the manifest described.
    let state = fixture.root().join(".universe-airlock").join("state");
    let current = std::fs::read_link(state.join("current")).expect("a current link");
    let environment = state.join(current);
    let restored = std::fs::read(environment.join(PAYLOAD_ARTIFACT))
        .expect("the artifact in the active environment");
    assert_eq!(restored, artifact);

    // And the prefix that environment installed: the executable the archive describes, at the
    // mode the archive recorded, which is what makes a restored prefix runnable.
    let installed = environment.join("prefix").join(PAYLOAD_EXECUTABLE);
    assert_eq!(
        std::fs::read(&installed).expect("the installed executable"),
        PAYLOAD_SCRIPT
    );
    let mode = {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(&installed)
            .expect("the installed executable")
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(mode, 0o755, "the installed executable is {mode:o}");

    // The checkout this very run cloned is at the pin the payload names.
    let checkout = cache.join(&fixture.commit);
    assert_eq!(
        git(&["rev-parse", "HEAD"], &checkout),
        fixture.commit,
        "the restore did not come from the pinned checkout"
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
    publish(&directory, &remote, Bootstrapper::Stub);
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
    let commit = publish(&directory, &remote, Bootstrapper::Absent);

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
