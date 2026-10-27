use std::process::Command;

#[test]
fn library_reports_universe_kit_version() {
    let output = universe_kit::run(["version", "--json"]);
    assert_eq!(output.exit_code(), 0);
    assert!(output.stdout().contains("\"binary\":\"universe-kit\""));
    assert!(output.stderr().is_empty());
}

#[test]
fn library_rejects_unknown_command() {
    let output = universe_kit::run(["build"]);
    assert_eq!(output.exit_code(), 2);
    assert!(output.stderr().contains("unknown argument: build"));
}

#[test]
fn library_main_returns_process_codes() {
    assert_eq!(
        format!("{:?}", universe_kit::run_main(["version"])),
        format!("{:?}", std::process::ExitCode::SUCCESS)
    );
    assert_eq!(
        format!("{:?}", universe_kit::run_main(["build"])),
        format!("{:?}", std::process::ExitCode::from(2))
    );
}

#[test]
fn binary_reports_version_and_usage_error() {
    let binary = env!("CARGO_BIN_EXE_universe-kit");

    let ok = Command::new(binary)
        .arg("--version")
        .output()
        .expect("run universe-kit --version");
    assert!(ok.status.success());
    assert!(String::from_utf8_lossy(&ok.stdout).contains("universe-kit 0.1.0"));
    assert!(ok.stderr.is_empty());

    let error = Command::new(binary)
        .arg("build")
        .output()
        .expect("run universe-kit build");
    assert_eq!(error.status.code(), Some(2));
    assert!(error.stdout.is_empty());
    assert!(String::from_utf8_lossy(&error.stderr).contains("unknown argument: build"));
}
