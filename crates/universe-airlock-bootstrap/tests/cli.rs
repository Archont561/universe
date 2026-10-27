use std::process::Command;

#[test]
fn library_reports_bootstrap_version() {
    let output = universe_airlock_bootstrap::run(["version", "--json"]);
    assert_eq!(output.exit_code(), 0);
    assert!(
        output
            .stdout()
            .contains("\"binary\":\"universe-airlock-bootstrap\"")
    );
    assert!(output.stderr().is_empty());
}

#[test]
fn library_rejects_unimplemented_restore_command() {
    let output = universe_airlock_bootstrap::run(["restore"]);
    assert_eq!(output.exit_code(), 2);
    assert!(output.stderr().contains("unknown argument: restore"));
}

#[test]
fn library_main_returns_process_codes() {
    assert_eq!(
        format!("{:?}", universe_airlock_bootstrap::run_main(["version"])),
        format!("{:?}", std::process::ExitCode::SUCCESS)
    );
    assert_eq!(
        format!("{:?}", universe_airlock_bootstrap::run_main(["restore"])),
        format!("{:?}", std::process::ExitCode::from(2))
    );
}

#[test]
fn binary_reports_version_and_usage_error() {
    let binary = env!("CARGO_BIN_EXE_universe-airlock-bootstrap");

    let ok = Command::new(binary)
        .arg("--version")
        .output()
        .expect("run universe-airlock-bootstrap --version");
    assert!(ok.status.success());
    assert!(String::from_utf8_lossy(&ok.stdout).contains("universe-airlock-bootstrap 0.1.0"));
    assert!(ok.stderr.is_empty());

    let error = Command::new(binary)
        .arg("restore")
        .output()
        .expect("run universe-airlock-bootstrap restore");
    assert_eq!(error.status.code(), Some(2));
    assert!(error.stdout.is_empty());
    assert!(String::from_utf8_lossy(&error.stderr).contains("unknown argument: restore"));
}
