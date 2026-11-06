//! Trusted publishing: a CI job's identity, exchanged for a token that expires.
//!
//! Every test here stands a `wiremock` server in front of the blocking HTTP client and asserts
//! on the requests that actually crossed the socket: which audience the job asked its CI
//! provider for, what it posted to the mint endpoint, and what it did with the answer. The
//! error tests matter as much as the happy path — an exchange that fails silently would show
//! up as a publish failing much later, on an upload, for a reason that names nothing.

mod store_support;

use store_support::Harness;
use universe_channel::store::{OidcExchange, StoreError, TrustedPublisher};

use wiremock::matchers::{body_bytes, header, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

/// Where a test's fake CI provider lives, as the environment would spell it.
const REQUEST_URL: &str = "ACTIONS_ID_TOKEN_REQUEST_URL";
/// The credential that proves the job may ask.
const REQUEST_TOKEN: &str = "ACTIONS_ID_TOKEN_REQUEST_TOKEN";

/// An exchange configured as though the job were running in CI, pointed at `url`.
fn exchange(url: &str) -> OidcExchange {
    OidcExchange::from_lookup(|name| match name {
        REQUEST_URL => Some(format!("{url}/oidc/token?api-version=2.0")),
        REQUEST_TOKEN => Some("ambient-request-token".to_owned()),
        _ => None,
    })
    .expect("the environment names a provider")
}

#[test]
fn a_job_identity_is_exchanged_for_the_token_the_destination_returns() {
    let harness = Harness::start();
    let endpoint = harness.endpoint();
    let host = endpoint
        .split_once("://")
        .expect("a scheme")
        .1
        .split(':')
        .next()
        .expect("a host")
        .to_owned();

    // The provider is asked for a token scoped to the destination that will redeem it.
    harness.mount(
        Mock::given(method("GET"))
            .and(path("/oidc/token"))
            .and(query_param("api-version", "2.0"))
            .and(query_param("audience", host.as_str()))
            .and(header("authorization", "Bearer ambient-request-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "value": "the-signed-jwt",
            })))
            .expect(1),
    );
    // The destination is then given that jwt, and answers with the token itself rather than a
    // JSON envelope.
    harness.mount(
        Mock::given(method("POST"))
            .and(path("/api/oidc/mint_token"))
            .and(header("content-type", "application/json"))
            .and(body_bytes(b"{\"token\":\"the-signed-jwt\"}".to_vec()))
            .respond_with(ResponseTemplate::new(200).set_body_string("minted-bearer-token\n"))
            .expect(1),
    );

    let token = exchange(&endpoint)
        .mint(&endpoint)
        .expect("the exchange succeeds");

    assert_eq!(
        token, "minted-bearer-token",
        "trailing whitespace is trimmed"
    );
    harness.verify();
}

#[test]
fn a_prefix_dev_destination_is_asked_for_the_audience_prefix_dev_ships() {
    // The audience rule is what makes a token usable by the server that issued it and nothing
    // else; it is asserted here rather than at the call site because it is a protocol fact.
    assert_eq!(
        universe_channel::store::audience_for("https://prefix.dev"),
        "prefix.dev"
    );
}

#[test]
fn a_provider_that_refuses_names_the_permission_the_job_forgot() {
    let harness = Harness::start();
    harness.mount(
        Mock::given(method("GET"))
            .and(path("/oidc/token"))
            .respond_with(ResponseTemplate::new(403).set_body_string("no id token for you")),
    );

    let error = exchange(&harness.endpoint())
        .mint(&harness.endpoint())
        .expect_err("the provider refused");

    let StoreError::Response { message, .. } = &error else {
        panic!("expected the answer to be reported, got {error:?}");
    };
    assert!(message.contains("403"), "{message}");
    assert!(message.contains("id-token: write"), "{message}");
    assert!(message.contains("no id token for you"), "{message}");
}

#[test]
fn a_provider_answer_without_a_value_is_refused_rather_than_used() {
    let harness = Harness::start();
    harness.mount(
        Mock::given(method("GET"))
            .and(path("/oidc/token"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "value": " " })),
            ),
    );

    let error = exchange(&harness.endpoint())
        .mint(&harness.endpoint())
        .expect_err("an empty identity is not an identity");

    assert!(
        error.to_string().contains("empty identity token"),
        "{error}"
    );
}

#[test]
fn a_mint_endpoint_that_refuses_is_reported_with_its_own_answer() {
    let harness = Harness::start();
    harness.mount(
        Mock::given(method("GET"))
            .and(path("/oidc/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "value": "the-signed-jwt" })),
            ),
    );
    harness.mount(
        Mock::given(method("POST"))
            .and(path("/api/oidc/mint_token"))
            .respond_with(ResponseTemplate::new(401).set_body_string("audience mismatch")),
    );

    let error = exchange(&harness.endpoint())
        .mint(&harness.endpoint())
        .expect_err("the destination refused the jwt");

    let message = error.to_string();
    assert!(message.contains("401"), "{message}");
    assert!(message.contains("audience mismatch"), "{message}");
}

#[test]
fn an_empty_minted_body_is_refused() {
    let harness = Harness::start();
    harness.mount(
        Mock::given(method("GET"))
            .and(path("/oidc/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "value": "the-signed-jwt" })),
            ),
    );
    harness.mount(
        Mock::given(method("POST"))
            .and(path("/api/oidc/mint_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string("")),
    );

    let error = exchange(&harness.endpoint())
        .mint(&harness.endpoint())
        .expect_err("a blank token is no token");

    assert!(error.to_string().contains("empty body"), "{error}");
}

#[test]
fn a_job_without_the_permission_is_refused_before_any_request() {
    let error = OidcExchange::from_lookup(|_| None).expect_err("no ambient identity");

    // No server is started, which is the point: the refusal happens without a network.
    assert!(error.to_string().contains("id-token: write"), "{error}");
}
