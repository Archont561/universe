use std::process::Command;

#[test]
fn library_reports_universe_airlock_version() {
    let output = universe_airlock::run(["version", "--json"]);
    assert_eq!(output.exit_code(), 0);
    assert!(output.stdout().contains("\"binary\":\"universe-airlock\""));
    assert!(output.stderr().is_empty());
}

#[test]
fn library_rejects_producer_only_command() {
    let output = universe_airlock::run(["publish"]);
    assert_eq!(output.exit_code(), 2);
    assert!(output.stderr().contains("unknown argument: publish"));
}

#[test]
fn library_main_returns_process_codes() {
    assert_eq!(
        format!("{:?}", universe_airlock::run_main(["version"])),
        format!("{:?}", std::process::ExitCode::SUCCESS)
    );
    assert_eq!(
        format!("{:?}", universe_airlock::run_main(["publish"])),
        format!("{:?}", std::process::ExitCode::from(2))
    );
}

#[test]
fn binary_reports_version_and_usage_error() {
    let binary = env!("CARGO_BIN_EXE_universe-airlock");

    let ok = Command::new(binary)
        .arg("--version")
        .output()
        .expect("run universe-airlock --version");
    assert!(ok.status.success());
    assert!(String::from_utf8_lossy(&ok.stdout).contains("universe-airlock 0.1.0"));
    assert!(ok.stderr.is_empty());

    let error = Command::new(binary)
        .arg("publish")
        .output()
        .expect("run universe-airlock publish");
    assert_eq!(error.status.code(), Some(2));
    assert!(error.stdout.is_empty());
    assert!(String::from_utf8_lossy(&error.stderr).contains("unknown argument: publish"));
}
