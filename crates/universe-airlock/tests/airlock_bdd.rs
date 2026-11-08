//! Scenarios for the disconnected host's Airlock consumer.
//!
//! The unit tests beside `run` say which strings come back. These say what a host observes:
//! a script gets a report it can parse, a human gets one line, and a verb the consumer does
//! not implement is refused with the code a caller branches on.

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use universe_airlock::run;

/// One command line, and what running it produced.
#[derive(Default)]
struct Invocation {
    arguments: Vec<String>,
    stdout: Option<String>,
    stderr: Option<String>,
    exit_code: Option<u8>,
}

impl Invocation {
    fn stdout(&self) -> &str {
        self.stdout
            .as_deref()
            .expect("the command line was executed")
    }

    fn stderr(&self) -> &str {
        self.stderr
            .as_deref()
            .expect("the command line was executed")
    }

    fn exit_code(&self) -> u8 {
        self.exit_code.expect("the command line was executed")
    }
}

#[fixture]
fn invocation() -> Invocation {
    Invocation::default()
}

#[given("the airlock command line \"{arguments}\"")]
fn a_command_line(invocation: &mut Invocation, arguments: String) {
    invocation.arguments = arguments.split_whitespace().map(str::to_owned).collect();
}

#[when("it is executed")]
fn execute(invocation: &mut Invocation) {
    let output = run(invocation.arguments.iter().map(String::as_str));
    invocation.stdout = Some(output.stdout().to_owned());
    invocation.stderr = Some(output.stderr().to_owned());
    invocation.exit_code = Some(output.exit_code());
}

#[then("it succeeds with nothing on stderr")]
fn succeeded(invocation: &Invocation) {
    assert_eq!(invocation.exit_code(), 0, "{}", invocation.stderr());
    assert_eq!(invocation.stderr(), "");
}

#[then("the report names universe-airlock and the release train version")]
fn json_report(invocation: &Invocation) {
    let stdout = invocation.stdout();
    assert!(
        stdout.contains("\"binary\":\"universe-airlock\""),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!("\"version\":\"{}\"", universe_airlock::VERSION)),
        "{stdout}"
    );
    assert!(stdout.contains("\"ok\":true"), "{stdout}");
}

#[then("the report is the binary name and version on one line")]
fn plain_report(invocation: &Invocation) {
    assert_eq!(
        invocation.stdout().trim_end(),
        format!("universe-airlock {}", universe_airlock::VERSION)
    );
}

#[then("it exits with the usage code")]
fn usage_exit(invocation: &Invocation) {
    assert_eq!(invocation.exit_code(), 2, "{}", invocation.stdout());
    assert_eq!(invocation.stdout(), "");
}

#[then("the refusal names the argument it does not implement")]
fn names_the_argument(invocation: &Invocation) {
    assert!(
        invocation.stderr().contains("unknown argument: publish"),
        "{}",
        invocation.stderr()
    );
}

#[scenario(
    path = "features/airlock.feature",
    name = "The version is reported in the form a script reads"
)]
fn the_version_is_json(invocation: Invocation) {
    let _ = invocation;
}

#[scenario(
    path = "features/airlock.feature",
    name = "The version is reported in the form a human reads"
)]
fn the_version_is_one_line(invocation: Invocation) {
    let _ = invocation;
}

#[scenario(
    path = "features/airlock.feature",
    name = "A producer verb is refused by name"
)]
fn a_producer_verb_is_refused(invocation: Invocation) {
    let _ = invocation;
}
