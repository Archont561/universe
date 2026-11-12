//! Scenarios for the facade's one rule: which executable a command line names.
//!
//! The facade never runs anything itself, so every scenario ends at the pair (component,
//! arguments). That pair is the whole contract between the facade and its components.

use std::ffi::OsString;

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use universe_cli::route;

/// A typed command line and where the facade sent it.
#[derive(Default)]
struct Facade {
    typed: Vec<String>,
    component: Option<&'static str>,
    forwarded: Vec<String>,
}

#[fixture]
fn facade() -> Facade {
    Facade::default()
}

#[given("the command line \"{command}\"")]
fn a_command_line(facade: &mut Facade, command: String) {
    // The first word is the facade's own name, which the process never sees in `args()`.
    facade.typed = command
        .split_whitespace()
        .skip(1)
        .map(str::to_owned)
        .collect();
}

#[when("it is routed")]
fn routed(facade: &mut Facade) {
    let arguments: Vec<OsString> = facade.typed.iter().map(OsString::from).collect();
    let (component, forwarded) = route(&arguments);
    facade.forwarded = forwarded
        .iter()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    facade.component = Some(component);
}

#[then("the producer runs it")]
fn the_producer_runs_it(facade: &Facade) {
    assert_eq!(facade.component, Some("universe-kit"));
}

#[then("the consumer runs it")]
fn the_consumer_runs_it(facade: &Facade) {
    assert_eq!(facade.component, Some("universe-airlock"));
}

#[then("the arguments reach it unchanged")]
fn unchanged(facade: &Facade) {
    assert_eq!(facade.forwarded, facade.typed);
}

#[then("the arguments reach it without the namespace")]
fn without_the_namespace(facade: &Facade) {
    assert!(
        facade.forwarded.len() < facade.typed.len(),
        "nothing was dropped: {:?}",
        facade.forwarded
    );
    assert_eq!(facade.forwarded, facade.typed[1..]);
}

#[scenario(
    path = "features/routing.feature",
    name = "A command with no namespace belongs to the producer"
)]
fn a_bare_command_is_producer_work(facade: Facade) {
    let _ = facade;
}

#[scenario(
    path = "features/routing.feature",
    name = "The explicit producer namespace is not forwarded"
)]
fn the_producer_namespace_is_dropped(facade: Facade) {
    let _ = facade;
}

#[scenario(
    path = "features/routing.feature",
    name = "The airlock namespace belongs to the consumer"
)]
fn the_airlock_namespace_is_consumer_work(facade: Facade) {
    let _ = facade;
}

#[scenario(
    path = "features/routing.feature",
    name = "Rendering an entrypoint is producer work in the consumer's namespace"
)]
fn rendering_is_producer_work(facade: Facade) {
    let _ = facade;
}

#[scenario(
    path = "features/routing.feature",
    name = "Publishing the ref the entrypoint clones is producer work"
)]
fn publishing_the_bootstrap_ref_is_producer_work(facade: Facade) {
    let _ = facade;
}
#[scenario(
    path = "features/routing.feature",
    name = "Packing the payload a restore consumes is producer work"
)]
fn packing_the_payload_is_producer_work(facade: Facade) {
    let _ = facade;
}
