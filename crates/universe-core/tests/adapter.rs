//! Behavioural tests for the adapter boundary and its conformance fixtures.
//!
//! Fixtures are named by the behaviour they exercise, not by an incident date
//! (doc-204), and every one of them is compiled into the test binary so the suite
//! needs no filesystem or network access.

use serde_json::Value;
use universe_core::adapter::{
    Adapter, AuditRequest, BuildRequest, Capabilities, Operation, Outcome, OutcomeKind,
    PlanRequest, RequestContext,
};
use universe_core::{Artifact, ArtifactId, Diagnostic, Sha256Digest};

const SUCCESS: &str = include_str!("fixtures/adapter-success.json");
const USER_FAILURE: &str = include_str!("fixtures/adapter-user-failure.json");
const USAGE_FAILURE: &str = include_str!("fixtures/adapter-usage-failure.json");
const INFRASTRUCTURE_FAILURE: &str = include_str!("fixtures/adapter-infrastructure-failure.json");
const DIGEST_MISMATCH: &str = include_str!("fixtures/adapter-digest-mismatch.json");
const UNKNOWN_EVIDENCE: &str = include_str!("fixtures/adapter-unknown-evidence.json");

const FIXTURES: [(&str, &str); 6] = [
    ("success", SUCCESS),
    ("user_failure", USER_FAILURE),
    ("usage_failure", USAGE_FAILURE),
    ("infrastructure_failure", INFRASTRUCTURE_FAILURE),
    ("digest_mismatch", DIGEST_MISMATCH),
    ("unknown_evidence", UNKNOWN_EVIDENCE),
];

fn parse(fixture: &str) -> Outcome {
    serde_json::from_str(fixture).expect("fixture parses as an outcome")
}

fn codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics.iter().map(Diagnostic::code).collect()
}

fn context() -> RequestContext {
    RequestContext {
        adapter: "fixture".to_owned(),
        package: ArtifactId::new("pkg", "0.1.0").expect("identity"),
        platform: "linux-64".to_owned(),
        workdir: "packages/pkg".to_owned(),
    }
}

fn artifact() -> Artifact {
    Artifact {
        id: ArtifactId::new("pkg", "0.1.0").expect("identity"),
        digest: Sha256Digest::parse(&"a".repeat(64)).expect("digest"),
        size_bytes: 4096,
        media_type: None,
    }
}

#[test]
fn every_fixture_satisfies_the_conformance_rules() {
    for (name, fixture) in FIXTURES {
        let outcome = parse(fixture);
        let violations = outcome.validate();
        assert!(
            violations.is_empty(),
            "{name} violates the conformance rules: {:?}",
            codes(&violations)
        );
    }
}

#[test]
fn every_fixture_round_trips_through_json() {
    for (name, fixture) in FIXTURES {
        let original: Value = serde_json::from_str(fixture).expect("fixture is JSON");
        let reserialized = serde_json::to_value(parse(fixture)).expect("outcome serializes");
        assert_eq!(
            original, reserialized,
            "{name} does not round-trip unchanged"
        );
    }
}

#[test]
fn exactly_one_fixture_is_retryable() {
    let retryable: Vec<&str> = FIXTURES
        .into_iter()
        .filter(|(_, fixture)| parse(fixture).is_retryable())
        .map(|(name, _)| name)
        .collect();
    assert_eq!(retryable, vec!["infrastructure_failure"]);
}

#[test]
fn only_success_carries_artifacts() {
    for (name, fixture) in FIXTURES {
        let outcome = parse(fixture);
        if name == "success" {
            assert_eq!(
                outcome.artifacts().len(),
                1,
                "{name} must name its artifact"
            );
            assert!(outcome.is_success());
        } else {
            assert!(
                outcome.artifacts().is_empty(),
                "{name} must not carry artifacts"
            );
            assert!(!outcome.is_success());
        }
    }
}

#[test]
fn every_outcome_kind_has_a_fixture() {
    let covered: Vec<OutcomeKind> = FIXTURES
        .into_iter()
        .map(|(_, fixture)| parse(fixture).kind())
        .collect();
    assert_eq!(covered, OutcomeKind::ALL.to_vec());
}

#[test]
fn an_unknown_field_in_an_outcome_fails_closed() {
    let fixture = r#"{"outcome":"user_failure","diagnostics":[],"retry_after":30}"#;
    assert!(
        serde_json::from_str::<Outcome>(fixture).is_err(),
        "an outcome with an undeclared field must not parse"
    );
}

#[test]
fn a_request_round_trips_and_rejects_an_absolute_workdir() {
    let request = PlanRequest {
        context: context(),
        command: "make package".to_owned(),
    };
    let encoded = serde_json::to_string(&request).expect("request serializes");
    let decoded: PlanRequest = serde_json::from_str(&encoded).expect("request parses");
    assert_eq!(request, decoded);
    assert!(request.context.validate().is_empty());

    let mut absolute = request;
    absolute.context.workdir = "/home/someone/build".to_owned();
    assert_eq!(
        codes(&absolute.context.validate()),
        vec!["adapter.request.workdir.invalid"]
    );
}

struct FakeAdapter;

impl Adapter for FakeAdapter {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            adapter: "fixture".to_owned(),
            operations: vec![Operation::Plan, Operation::Build, Operation::Audit],
            platforms: Vec::new(),
        }
    }

    fn plan(&self, request: &PlanRequest) -> Outcome {
        assert!(request.context.validate().is_empty());
        parse(USER_FAILURE)
    }

    fn build(&self, request: &BuildRequest) -> Outcome {
        assert!(request.context.validate().is_empty());
        parse(SUCCESS)
    }

    fn audit(&self, request: &AuditRequest) -> Outcome {
        assert_eq!(request.artifacts.len(), 1);
        parse(DIGEST_MISMATCH)
    }
}

#[test]
fn an_adapter_drives_every_fixture_through_the_trait() {
    let adapter = FakeAdapter;
    let capabilities = adapter.capabilities();
    assert!(capabilities.validate().is_empty());
    assert!(capabilities.supports(Operation::Audit));

    let plan = adapter.plan(&PlanRequest {
        context: context(),
        command: "make package".to_owned(),
    });
    assert!(plan.is_failure());
    assert!(!plan.is_retryable());

    let build = adapter.build(&BuildRequest {
        context: context(),
        command: "make package".to_owned(),
    });
    assert!(build.is_success());
    assert_eq!(build.artifacts().len(), 1);

    let audit = adapter.audit(&AuditRequest {
        context: context(),
        artifacts: vec![artifact()],
    });
    assert_eq!(
        codes(audit.diagnostics()),
        vec!["adapter.audit.digest_mismatch"]
    );
    assert!(audit.validate().is_empty());
}
