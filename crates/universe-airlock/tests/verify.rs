//! The consumer verb as a disconnected host runs it: one payload directory, no network.

use std::process::Command;

mod support;
use support::Payload;

const BINARY: &str = env!("CARGO_BIN_EXE_universe-airlock");

fn run(arguments: &[&str]) -> (i32, String, String) {
    let output = Command::new(BINARY)
        .args(arguments)
        .output()
        .expect("run universe-airlock");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// A verification that succeeds with every proxy pointed at a dead address still proves
/// something useful: any later network fetch would fail here rather than silently working.
#[test]
fn the_binary_verifies_a_payload_with_no_reachable_network() {
    let payload = Payload::new("offline");
    payload.publish();
    let root = payload.root.to_string_lossy().into_owned();
    let output = Command::new(BINARY)
        .args(["verify", "--payload", &root])
        .env("http_proxy", "http://127.0.0.1:1")
        .env("https_proxy", "http://127.0.0.1:1")
        .env("ALL_PROXY", "http://127.0.0.1:1")
        .env("NO_PROXY", "")
        .output()
        .expect("run universe-airlock verify");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("verified:"));
}

#[test]
fn verify_json_is_one_versioned_envelope_and_errors_stay_off_stdout() {
    let payload = Payload::new("json");
    payload.publish();
    let root = payload.root.to_string_lossy().into_owned();
    let (code, stdout, stderr) = run(&["verify", "--payload", &root, "--json"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.trim_start().starts_with('{'), "{stdout}");
    assert!(stdout.contains("\"command\":\"verify\""), "{stdout}");
    assert!(stdout.contains("\"ok\":true"), "{stdout}");
    assert!(stdout.contains("\"schema_version\""), "{stdout}");
    assert!(stderr.contains("developer lane"), "{stderr}");

    payload.remove_artifact();
    let (code, stdout, stderr) = run(&["verify", "--payload", &root, "--json"]);
    assert_eq!(code, 1);
    assert_eq!(stdout, "");
    assert!(stderr.contains("airlock.payload.missing"), "{stderr}");
}

/// The Conda package test ships this directory. A fixture that drifted from the manifest
/// would make the installed package fail its own test, so the repository holds it honest.
#[test]
fn the_recipe_fixture_verifies_from_the_installed_layout() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("recipe/payload");
    let root = fixture.to_string_lossy().into_owned();
    let (code, stdout, stderr) = run(&["verify", "--payload", &root]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("artifacts: 1"), "{stdout}");
}
