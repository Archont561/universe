//! The binary as a consumer's host runs it: real payload directory, real state root.
//!
//! Nothing here mocks the filesystem or the process. A restore is judged by what is on disk
//! afterwards and by the exit code, because that is all the generated entrypoint can observe.

use std::path::PathBuf;
use std::process::Command;

mod support;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const OTHER_COMMIT: &str = "89abcdef0123456789abcdef0123456789abcdef";
const BINARY: &str = env!("CARGO_BIN_EXE_universe-airlock-bootstrap");

/// A scenario directory that deletes itself, so no test writes outside the temp tree.
struct Scenario {
    root: PathBuf,
}

impl Scenario {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "universe-airlock-bootstrap-cli-{}-{label}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create a scenario directory");
        Self { root }
    }

    fn payload(&self) -> PathBuf {
        self.root.join("payload")
    }

    fn state(&self) -> PathBuf {
        self.root.join("state")
    }

    /// Publish a one-artifact payload described by a manifest that really matches it.
    fn publish(&self, version: &str, commit: &str, bytes: &[u8]) -> String {
        let payload = self.payload();
        std::fs::create_dir_all(&payload).expect("create a payload directory");
        std::fs::write(payload.join("airlock-0.1.0.conda"), bytes).expect("write an artifact");
        let digest = digest(bytes);
        let manifest = format!(
            r#"{{"schema":1,"version":"{version}","commit":"{commit}","packages":[
                 {{"name":"universe-airlock","kind":"conda","version":"{version}","artifacts":[
                   {{"filename":"airlock-0.1.0.conda","sha256":"{digest}","size":{},
                     "stores":["conda-static"]}}]}}]}}"#,
            bytes.len()
        );
        std::fs::write(payload.join("universe-release.json"), manifest).expect("write a manifest");
        digest
    }

    fn run(&self, arguments: &[&str]) -> Output {
        let output = Command::new(BINARY)
            .args(arguments)
            .current_dir(&self.root)
            .env("UNIVERSE_AIRLOCK_STATE", self.state())
            .output()
            .expect("run the bootstrap");
        Output {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// `restore --commit <pin>` against this scenario's payload and state root.
    fn restore(&self, commit: &str) -> Output {
        self.run(&[
            "restore",
            "--commit",
            commit,
            "--payload",
            &self.payload().display().to_string(),
            "--state",
            &self.state().display().to_string(),
        ])
    }
}

impl Drop for Scenario {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

struct Output {
    code: i32,
    stdout: String,
    stderr: String,
}

/// SHA-256 of the fixture bytes, taken from the crate so the fixture and the check agree.
fn digest(bytes: &[u8]) -> String {
    universe_airlock_bootstrap::hash::sha256_hex(bytes)
}

#[test]
fn the_binary_reports_its_version_and_usage() {
    let ok = Command::new(BINARY)
        .arg("--version")
        .output()
        .expect("run --version");
    assert!(ok.status.success());
    assert!(String::from_utf8_lossy(&ok.stdout).contains("universe-airlock-bootstrap 0.1.0"));
    assert!(ok.stderr.is_empty());

    let scenario = Scenario::new("usage");
    let error = scenario.run(&["publish"]);
    assert_eq!(error.code, 2);
    assert!(error.stdout.is_empty());
    assert!(error.stderr.contains("unknown command: publish"));
    assert!(error.stderr.contains("usage: universe-airlock-bootstrap"));
}

#[test]
fn the_usage_says_where_the_payload_really_lives() {
    // A consumer puts the closed payload where the binary will look for it. The rendered
    // entrypoint execs the binary from the pinned checkout but keeps the caller's working
    // directory; `crates/universe-kit/tests/airlock_restore.rs` restores from that directory
    // with the real binary. Telling the reader it has already changed into the checkout sends
    // them to a place where no payload can be, so the help has to name the real default.
    let output = Command::new(BINARY)
        .arg("--help")
        .output()
        .expect("run --help");
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(
        help.contains("The payload defaults to .universe-airlock/payload"),
        "{help}"
    );
    assert!(
        help.contains("entrypoint does not change it"),
        "the usage must say the working directory is the caller's: {help}"
    );
    assert!(
        !help.contains("already changed into the pinned checkout"),
        "the usage still claims the entrypoint changes the working directory: {help}"
    );
}

#[test]
fn verify_reports_a_payload_without_activating_anything() {
    let scenario = Scenario::new("verify");
    scenario.publish("0.1.0", COMMIT, &support::conda("the only artifact"));

    let output = scenario.run(&[
        "verify",
        "--payload",
        &scenario.payload().display().to_string(),
        "--state",
        &scenario.state().display().to_string(),
    ]);

    assert_eq!(output.code, 0, "stderr: {}", output.stderr);
    assert!(output.stdout.contains("version: 0.1.0"));
    assert!(output.stdout.contains("signature: unsigned"));
    assert!(output.stdout.contains("verified: nothing was activated"));
    // The developer lane says why it proceeded.
    assert!(output.stderr.contains("[airlock.signature.absent]"));
    // And the state root was not created by a read-only verb.
    assert!(!scenario.state().exists(), "verify wrote to the state root");
}

#[test]
fn a_restore_materialises_the_payload_and_doctor_can_see_it() {
    let scenario = Scenario::new("restore");
    scenario.publish("0.1.0", COMMIT, &support::conda("the only artifact"));

    let restored = scenario.restore(COMMIT);

    assert_eq!(restored.code, 0, "stderr: {}", restored.stderr);
    assert!(restored.stdout.contains("restored: 0.1.0"));
    assert!(restored.stdout.contains("commit: 0123456789"));
    assert!(restored.stdout.contains("previous: none"));
    assert!(
        restored
            .stderr
            .contains("warning: [airlock.signature.absent]")
    );

    let doctor = scenario.run(&[
        "doctor",
        "--state",
        &scenario.state().display().to_string(),
        "--payload",
        &scenario.payload().display().to_string(),
    ]);
    assert_eq!(doctor.code, 0, "stderr: {}", doctor.stderr);
    assert!(doctor.stdout.contains("current:"), "{}", doctor.stdout);
    assert!(
        doctor.stdout.contains("receipt: version=0.1.0"),
        "{}",
        doctor.stdout
    );

    // The artifact is really on disk under the environment `current` names.
    let current = std::fs::read_link(scenario.state().join("current")).expect("a current link");
    let environment = scenario.state().join(current);
    assert_eq!(
        std::fs::read(environment.join("airlock-0.1.0.conda")).expect("the artifact"),
        support::conda("the only artifact")
    );
}

#[test]
fn a_second_restore_reuses_the_environment_instead_of_copying_again() {
    let scenario = Scenario::new("idempotent");
    scenario.publish("0.1.0", COMMIT, &support::conda("the only artifact"));

    assert_eq!(scenario.restore(COMMIT).code, 0);
    let second = scenario.restore(COMMIT);

    assert_eq!(second.code, 0, "stderr: {}", second.stderr);
    assert!(
        second.stdout.contains("already present"),
        "{}",
        second.stdout
    );
}

#[test]
fn a_restore_from_another_commit_is_refused_before_any_state_exists() {
    let scenario = Scenario::new("commit");
    scenario.publish("0.1.0", OTHER_COMMIT, b"the only artifact\n");

    let output = scenario.restore(COMMIT);

    assert_eq!(output.code, 1);
    assert!(output.stderr.contains("[airlock.release.commit]"));
    assert!(output.stdout.is_empty());
    assert!(
        !scenario.state().join("current").exists(),
        "a refused restore pointed current somewhere"
    );
}

#[test]
fn a_sealed_restore_refuses_an_unsigned_payload_and_activates_nothing() {
    let scenario = Scenario::new("sealed");
    scenario.publish("0.1.0", COMMIT, &support::conda("the only artifact"));

    let output = scenario.run(&[
        "restore",
        "--commit",
        COMMIT,
        "--lane",
        "sealed",
        "--payload",
        &scenario.payload().display().to_string(),
        "--state",
        &scenario.state().display().to_string(),
    ]);

    assert_eq!(output.code, 1);
    assert!(output.stderr.contains("[airlock.signature.absent]"));
    assert!(output.stdout.is_empty());
    assert!(!scenario.state().join("current").exists());
}

#[test]
fn a_failed_restore_leaves_the_previous_version_active() {
    // The end-to-end form of `UNI-33` #3, through the binary a host actually runs.
    let scenario = Scenario::new("preserve");
    scenario.publish("0.1.0", COMMIT, &support::conda("the only artifact"));
    let first = scenario.restore(COMMIT);
    assert_eq!(first.code, 0, "stderr: {}", first.stderr);
    let before = std::fs::read_link(scenario.state().join("current")).expect("a current link");

    // Republish as a new version, verified, then corrupt it before the restore runs.
    scenario.publish("0.2.0", COMMIT, &support::conda("a different artifact"));
    let payload = scenario.payload();
    std::fs::write(
        payload.join("airlock-0.1.0.conda"),
        b"tampered after publication",
    )
    .expect("corrupt");
    let failed = scenario.restore(COMMIT);

    assert_eq!(failed.code, 1);
    assert!(
        failed.stderr.contains("[airlock.payload.size]"),
        "{}",
        failed.stderr
    );

    let after = std::fs::read_link(scenario.state().join("current")).expect("a current link");
    assert_eq!(before, after, "a failed restore moved current");

    let doctor = scenario.run(&["doctor", "--state", &scenario.state().display().to_string()]);
    assert!(
        doctor.stdout.contains("receipt: version=0.1.0"),
        "{}",
        doctor.stdout
    );
}

#[test]
fn the_handover_contract_restores_from_the_callers_working_directory() {
    // This is the invocation the rendered entrypoint makes after it has cloned and verified
    // the pin: `restore --commit <pin>` and no other flags. The entrypoint execs the binary
    // from the pinned checkout but does not change the working directory, so the payload and
    // state defaults resolve in the project the script was run from. Everything the restore
    // needs must therefore have a default that holds there.
    let project = std::env::temp_dir().join(format!(
        "universe-airlock-bootstrap-handover-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&project);
    let payload = project.join(".universe-airlock").join("payload");
    std::fs::create_dir_all(&payload).expect("create the payload directory");
    let bytes = support::conda("the only artifact");
    std::fs::write(payload.join("airlock-0.1.0.conda"), &bytes).expect("write an artifact");
    let manifest = format!(
        r#"{{"schema":1,"version":"0.1.0","commit":"{COMMIT}","packages":[
             {{"name":"universe-airlock","kind":"conda","version":"0.1.0","artifacts":[
               {{"filename":"airlock-0.1.0.conda","sha256":"{}","size":{},
                 "stores":["conda-static"]}}]}}]}}"#,
        digest(&bytes),
        bytes.len()
    );
    std::fs::write(payload.join("universe-release.json"), manifest).expect("write a manifest");

    let output = Command::new(BINARY)
        .args(["restore", "--commit", COMMIT])
        .current_dir(&project)
        .env_remove("UNIVERSE_AIRLOCK_STATE")
        .output()
        .expect("run the handover invocation");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");
    assert!(stdout.contains("restored: 0.1.0"), "{stdout}");
    assert!(stderr.contains("[airlock.signature.absent]"), "{stderr}");

    // The default state root, and the environment it now names.
    let state = project.join(".universe-airlock").join("state");
    let current = std::fs::read_link(state.join("current")).expect("a current link");
    let environment = state.join(current);
    assert_eq!(
        std::fs::read(environment.join("airlock-0.1.0.conda")).expect("the artifact"),
        bytes
    );
    // The prefix is what a caller puts on `PATH`, and the restore installed it.
    let installed = environment.join("prefix").join("bin/fixture");
    assert!(
        installed.is_file(),
        "no prefix was installed: {installed:?}"
    );
    assert!(stdout.contains("prefix: "), "{stdout}");

    let _ = std::fs::remove_dir_all(&project);
}

#[test]
fn restore_without_a_pin_is_a_usage_error() {
    let scenario = Scenario::new("no-pin");
    scenario.publish("0.1.0", COMMIT, &support::conda("the only artifact"));

    let output = scenario.run(&["restore"]);

    assert_eq!(output.code, 2);
    assert!(output.stderr.contains("restore needs --commit"));
}
