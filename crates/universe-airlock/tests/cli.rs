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

#[test]
fn help_is_a_success_that_names_the_version_command() {
    let output = universe_airlock::run(["--help"]);
    assert_eq!(output.exit_code(), 0);
    assert!(output.stderr().is_empty(), "{}", output.stderr());
    assert!(output.stdout().contains("Usage:"), "{}", output.stdout());
    assert!(output.stdout().contains("version"), "{}", output.stdout());
}

// A switch is a flag, not a value: asking twice for the same report changes nothing. Both
// the scaffold parser and argp accepted a repeated `--json`, so this pins the contract
// instead of asserting a refusal that never existed.
#[test]
fn a_repeated_switch_still_reports_one_envelope() {
    let repeated = universe_airlock::run(["--json", "--json"]);
    let once = universe_airlock::run(["--json"]);
    assert_eq!(repeated.exit_code(), 0);
    assert_eq!(repeated.stdout(), once.stdout());
    assert_eq!(repeated.stderr(), "");
}

// The consumer may own its grammar, but the envelope it emits is the core's. Comparing
// against the renderer the other editions still use is what stops the wiring from drifting
// into a second version format.
#[test]
fn the_json_version_is_byte_identical_to_the_core_renderer() {
    let wired = universe_airlock::run(["version", "--json"]);
    let reference =
        universe_core::execute_version(universe_core::Product::Airlock, ["version", "--json"]);
    assert_eq!(wired.stdout(), reference.stdout());
    assert_eq!(wired.stderr(), reference.stderr());
    assert_eq!(wired.exit_code(), reference.exit_code());
}

#[test]
fn the_plain_version_is_byte_identical_to_the_core_renderer() {
    let wired = universe_airlock::run(["--version"]);
    let reference = universe_core::execute_version(universe_core::Product::Airlock, ["--version"]);
    assert_eq!(wired.stdout(), reference.stdout());
}

// `run` is the pure seam; this one proves the process entrypoint puts help on stdout and
// leaves stderr empty, which is what a consumer on a disconnected host actually sees.
#[test]
fn the_binary_prints_help_on_stdout_with_no_diagnostic() {
    let binary = env!("CARGO_BIN_EXE_universe-airlock");
    let output = Command::new(binary)
        .arg("--help")
        .output()
        .expect("run universe-airlock --help");
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Usage:"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stderr.is_empty());
}
