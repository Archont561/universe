//! Scenarios for the disconnected host's Airlock consumer.
//!
//! The unit tests beside `run` say which strings come back. These say what a host observes:
//! a script gets a report it can parse, a human gets one line, and a verb the consumer does
//! not implement is refused with the code a caller branches on.

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use universe_airlock::run;

mod support;
use support::Payload;

/// One command line, and what running it produced.
#[derive(Default)]
struct Invocation {
    arguments: Vec<String>,
    stdout: Option<String>,
    stderr: Option<String>,
    exit_code: Option<u8>,
    payload: Option<Payload>,
}

impl Invocation {
    fn payload(&mut self) -> &mut Payload {
        self.payload.get_or_insert_with(|| Payload::new("bdd"))
    }

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

#[then("the help names the version command")]
fn help_names_the_version_command(invocation: &Invocation) {
    let stdout = invocation.stdout();
    assert!(stdout.contains("Usage:"), "{stdout}");
    assert!(stdout.contains("version"), "{stdout}");
}

#[then("the refusal names the argument it does not implement")]
fn names_the_argument(invocation: &Invocation) {
    assert!(
        invocation.stderr().contains("unknown argument: publish"),
        "{}",
        invocation.stderr()
    );
}

#[given("a published payload")]
fn a_published_payload(invocation: &mut Invocation) {
    invocation.payload().publish();
}

#[given("a payload with a missing artifact")]
fn a_payload_missing_an_artifact(invocation: &mut Invocation) {
    invocation.payload().publish();
    invocation.payload().remove_artifact();
}

#[given("a payload with a corrupt artifact")]
fn a_payload_with_a_corrupt_artifact(invocation: &mut Invocation) {
    invocation.payload().publish();
    invocation.payload().corrupt_artifact();
}

#[given("a payload with an extra object")]
fn a_payload_with_an_extra_object(invocation: &mut Invocation) {
    invocation.payload().publish();
    invocation.payload().add_extra_object();
}

#[given("a payload naming a wrong-platform object")]
fn a_payload_naming_a_wrong_platform_object(invocation: &mut Invocation) {
    invocation.payload().publish();
    invocation.payload().name_a_wrong_platform_object();
}

#[given("a signed payload with correct digests")]
fn a_signed_payload_with_correct_digests(invocation: &mut Invocation) {
    invocation.payload().publish();
    invocation.payload().add_signature();
}

#[given("a payload that is signed and missing its artifact")]
fn a_signed_payload_missing_its_artifact(invocation: &mut Invocation) {
    invocation.payload().publish();
    invocation.payload().add_signature();
    invocation.payload().remove_artifact();
}

fn verify_payload(invocation: &mut Invocation, lane: Option<&str>) {
    let root = invocation.payload().root.to_string_lossy().into_owned();
    let mut arguments = vec!["verify".to_owned(), "--payload".to_owned(), root];
    if let Some(lane) = lane {
        arguments.push("--lane".to_owned());
        arguments.push(lane.to_owned());
    }
    let output = run(arguments.iter().map(String::as_str));
    invocation.stdout = Some(output.stdout().to_owned());
    invocation.stderr = Some(output.stderr().to_owned());
    invocation.exit_code = Some(output.exit_code());
}

#[when("it is verified")]
fn it_is_verified(invocation: &mut Invocation) {
    verify_payload(invocation, None);
}

#[when("it is verified on the sealed lane")]
fn it_is_verified_on_the_sealed_lane(invocation: &mut Invocation) {
    verify_payload(invocation, Some("sealed"));
}

#[then("the report says the payload was verified")]
fn the_report_says_verified(invocation: &Invocation) {
    assert_eq!(invocation.exit_code(), 0, "{}", invocation.stderr());
    assert!(
        invocation.stdout().contains("verified:"),
        "{}",
        invocation.stdout()
    );
}

#[then("the report counts one artifact")]
fn the_report_counts_one_artifact(invocation: &Invocation) {
    assert!(
        invocation.stdout().contains("artifacts: 1"),
        "{}",
        invocation.stdout()
    );
}

#[then("the diagnostic names {code}")]
fn the_refusal_names(invocation: &Invocation, code: String) {
    assert_eq!(invocation.exit_code(), 1, "{}", invocation.stdout());
    assert_eq!(invocation.stdout(), "");
    assert!(
        invocation.stderr().contains(&code),
        "{}",
        invocation.stderr()
    );
}

#[then("the diagnostic does not name {code}")]
fn the_refusal_does_not_name(invocation: &Invocation, code: String) {
    assert!(
        !invocation.stderr().contains(&code),
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
    name = "The consumer lists what it can do"
)]
fn the_consumer_lists_what_it_can_do(invocation: Invocation) {
    let _ = invocation;
}

#[scenario(
    path = "features/airlock.feature",
    name = "A producer verb is refused by name"
)]
fn a_producer_verb_is_refused(invocation: Invocation) {
    let _ = invocation;
}

#[scenario(
    path = "features/airlock.feature",
    name = "A sealed snapshot verifies completely"
)]
fn a_sealed_snapshot_verifies_completely(invocation: Invocation) {
    let _ = invocation;
}

#[scenario(
    path = "features/airlock.feature",
    name = "A missing object is refused by name"
)]
fn a_missing_object_is_refused(invocation: Invocation) {
    let _ = invocation;
}

#[scenario(
    path = "features/airlock.feature",
    name = "A corrupt object is refused by name"
)]
fn a_corrupt_object_is_refused(invocation: Invocation) {
    let _ = invocation;
}

#[scenario(
    path = "features/airlock.feature",
    name = "An extra object is refused by name"
)]
fn an_extra_object_is_refused(invocation: Invocation) {
    let _ = invocation;
}

#[scenario(
    path = "features/airlock.feature",
    name = "An object addressed under another platform is refused"
)]
fn a_wrong_platform_object_is_refused(invocation: Invocation) {
    let _ = invocation;
}

#[scenario(
    path = "features/airlock.feature",
    name = "The sealed lane refuses an unsigned payload"
)]
fn the_sealed_lane_refuses_an_unsigned_payload(invocation: Invocation) {
    let _ = invocation;
}

#[scenario(
    path = "features/airlock.feature",
    name = "A signed payload with no trusted key is refused before artifacts are checked"
)]
fn a_signed_payload_is_refused_before_artifacts(invocation: Invocation) {
    let _ = invocation;
}

#[scenario(
    path = "features/airlock.feature",
    name = "A structurally valid signed payload is still refused without a trusted key"
)]
fn a_valid_signed_payload_is_still_refused(invocation: Invocation) {
    let _ = invocation;
}
