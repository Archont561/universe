//! Scenarios for the bootstrap a disconnected host actually runs.
//!
//! Everything is real: a payload directory, a state root, and the library entrypoint the
//! binary calls. A restore is judged by what is on disk and by the exit code, because that is
//! all the generated entrypoint can observe.

use std::path::{Path, PathBuf};

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use tempfile::TempDir;
use universe_airlock_bootstrap::hash::sha256_hex;
use universe_airlock_bootstrap::run;

mod support;

/// The commit a payload was published for, and one it was not.
const PIN: &str = "0123456789abcdef0123456789abcdef01234567";
const OTHER: &str = "89abcdef0123456789abcdef0123456789abcdef";

/// What a release actually ships, as the audit would have written it: a real archive, because a
/// restore installs a prefix from it now.
const ARTIFACT_NAME: &str = "universe-airlock-0.1.0.conda";

/// The archive one scenario publishes.
fn artifact() -> Vec<u8> {
    support::conda("the only artifact")
}

/// One host: its payload, its state root, and the last command it ran.
#[derive(Default)]
struct Host {
    root: Option<TempDir>,
    code: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
}

impl Host {
    fn root(&self) -> &Path {
        self.root
            .as_ref()
            .expect("a scenario published a payload")
            .path()
    }

    fn payload(&self) -> PathBuf {
        self.root().join("payload")
    }

    fn state(&self) -> PathBuf {
        self.root().join("state")
    }

    fn stdout(&self) -> &str {
        self.stdout.as_deref().expect("a command ran")
    }

    fn stderr(&self) -> &str {
        self.stderr.as_deref().expect("a command ran")
    }

    fn code(&self) -> i32 {
        self.code.expect("a command ran")
    }

    /// Run one command the way the entrypoint does, with the payload and state root named.
    fn run(&mut self, arguments: &[&str]) {
        let mut complete: Vec<String> = arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect();
        complete.push("--payload".to_owned());
        complete.push(self.payload().display().to_string());
        complete.push("--state".to_owned());
        complete.push(self.state().display().to_string());

        let output = run(complete);
        self.code = Some(i32::from(output.exit_code()));
        self.stdout = Some(output.stdout().to_owned());
        self.stderr = Some(output.stderr().to_owned());
    }

    fn restore(&mut self, commit: &str) {
        self.run(&["restore", "--commit", commit]);
    }

    fn current(&self) -> PathBuf {
        self.state().join("current")
    }
}

#[fixture]
fn host() -> Host {
    Host::default()
}

/// Publish a payload that names `pin` and describes the artifact it ships.
fn publish(host: &mut Host, pin: &str) {
    let root = TempDir::new().expect("a scratch host directory");
    let payload = root.path().join("payload");
    std::fs::create_dir_all(&payload).expect("create the payload directory");
    let artifact = artifact();
    std::fs::write(payload.join(ARTIFACT_NAME), &artifact).expect("write the artifact");
    let manifest = format!(
        r#"{{"schema":1,"version":"0.1.0","commit":"{pin}","packages":[
             {{"name":"universe-airlock","kind":"conda","version":"0.1.0","artifacts":[
               {{"filename":"{ARTIFACT_NAME}","sha256":"{}","size":{},
                 "stores":["conda-static"]}}]}}]}}"#,
        sha256_hex(&artifact),
        artifact.len()
    );
    std::fs::write(payload.join("universe-release.json"), manifest).expect("write the manifest");
    host.root = Some(root);
}

#[given("a payload published for the pinned commit")]
fn published(host: &mut Host) {
    publish(host, PIN);
}

#[given("a payload whose artifact matches its manifest")]
fn published_with_a_matching_artifact(host: &mut Host) {
    publish(host, PIN);
}

// The manifest names the pinned commit and nothing else, so a restore of any other commit has
// no release to verify against.
#[given("a payload that names only the pinned commit")]
fn published_for_the_pin_only(host: &mut Host) {
    publish(host, PIN);
}

#[given("an unsigned payload published for the pinned commit")]
fn unsigned_payload(host: &mut Host) {
    publish(host, PIN);
}

#[given("a payload that has already been restored once")]
fn already_restored(host: &mut Host) {
    publish(host, PIN);
    host.restore(PIN);
    assert_eq!(
        host.code(),
        0,
        "the first restore failed: {}",
        host.stderr()
    );
}

#[when("the payload is verified")]
fn verified(host: &mut Host) {
    host.run(&["verify"]);
}

#[when("the pinned commit is restored")]
fn restored(host: &mut Host) {
    host.restore(PIN);
}

#[when("the pinned commit is restored again")]
fn restored_again(host: &mut Host) {
    host.restore(PIN);
}

#[when("a different commit is restored")]
fn restored_at_another_commit(host: &mut Host) {
    host.restore(OTHER);
}

#[when("the payload is restored in the sealed lane")]
fn restored_sealed(host: &mut Host) {
    host.run(&["restore", "--commit", PIN, "--lane", "sealed"]);
}

#[then("verification succeeds and names the release version")]
fn verification_succeeds(host: &Host) {
    assert_eq!(host.code(), 0, "stderr: {}", host.stderr());
    assert!(
        host.stdout().contains("version: 0.1.0"),
        "{}",
        host.stdout()
    );
    assert!(
        host.stdout().contains("signature: unsigned"),
        "{}",
        host.stdout()
    );
    // The developer lane says why it proceeded, rather than proceeding silently.
    assert!(
        host.stderr().contains("[airlock.signature.absent]"),
        "{}",
        host.stderr()
    );
}

#[then("it says nothing was activated")]
fn nothing_activated(host: &Host) {
    assert!(
        host.stdout().contains("verified: nothing was activated"),
        "{}",
        host.stdout()
    );
}

#[then("the state root is still empty")]
fn state_untouched(host: &Host) {
    assert!(
        !host.state().exists(),
        "a read-only verb created the state root"
    );
}

#[then("the restore succeeds and names the release version")]
fn restore_succeeds(host: &Host) {
    assert_eq!(host.code(), 0, "stderr: {}", host.stderr());
    assert!(
        host.stdout().contains("restored: 0.1.0"),
        "{}",
        host.stdout()
    );
}

#[then("the artifact sits under the environment current names")]
fn artifact_under_current(host: &Host) {
    let link = std::fs::read_link(host.current()).expect("a current link");
    let environment = host.state().join(link);
    assert_eq!(
        std::fs::read(environment.join(ARTIFACT_NAME)).expect("the artifact"),
        artifact()
    );
}

#[then("the restore is refused because the commit does not match")]
fn commit_mismatch(host: &Host) {
    assert_eq!(host.code(), 1, "{}", host.stdout());
    assert!(
        host.stderr().contains("[airlock.release.commit]"),
        "{}",
        host.stderr()
    );
}

#[then("the restore is refused because the signature is absent")]
fn signature_absent(host: &Host) {
    assert_eq!(host.code(), 1, "{}", host.stdout());
    assert!(
        host.stderr().contains("[airlock.signature.absent]"),
        "{}",
        host.stderr()
    );
    assert_eq!(host.stdout(), "");
}

#[then("nothing became current")]
fn nothing_current(host: &Host) {
    assert!(
        !host.current().exists(),
        "a refused restore pointed current somewhere"
    );
}

#[then("the second restore succeeds without copying again")]
fn second_restore_reuses(host: &Host) {
    assert_eq!(host.code(), 0, "stderr: {}", host.stderr());
    assert!(
        host.stdout().contains("already present"),
        "{}",
        host.stdout()
    );
}

#[scenario(
    path = "features/bootstrap.feature",
    name = "Verification reads a payload and installs nothing"
)]
fn verification_installs_nothing(host: Host) {
    let _ = host;
}

#[scenario(
    path = "features/bootstrap.feature",
    name = "A restore materialises the payload and makes it current"
)]
fn a_restore_makes_the_payload_current(host: Host) {
    let _ = host;
}

#[scenario(
    path = "features/bootstrap.feature",
    name = "A restore of a commit the payload does not name is refused"
)]
fn a_restore_of_another_commit_is_refused(host: Host) {
    let _ = host;
}

#[scenario(
    path = "features/bootstrap.feature",
    name = "A sealed restore refuses an unsigned payload"
)]
fn a_sealed_restore_refuses_an_unsigned_payload(host: Host) {
    let _ = host;
}

#[scenario(
    path = "features/bootstrap.feature",
    name = "A second restore reuses the environment it already made"
)]
fn a_second_restore_reuses_the_environment(host: Host) {
    let _ = host;
}
