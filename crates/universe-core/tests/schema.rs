//! Behavioural tests for the ecosystem-neutral record families.
//!
//! Fixtures are named by the behaviour they exercise, not by an incident date
//! (doc-204), and every one of them is compiled into the test binary so the suite
//! needs no filesystem or network access.

use universe_core::{
    Artifact, Catalog, Diagnostic, EntryKind, Envelope, ModelError, STDOUT_SCHEMA_VERSION,
    SchemaVersion, Severity, Sha256Digest, Snapshot,
};

const MIXED_KINDS: &str = include_str!("fixtures/catalog-mixed-kinds.json");
const UNKNOWN_FIELD: &str = include_str!("fixtures/catalog-unknown-field.json");
const UNSUPPORTED_VERSION: &str = include_str!("fixtures/catalog-unsupported-version.json");
const INVALID_ENTRIES: &str = include_str!("fixtures/catalog-invalid-entries.json");
const INVALID_DIGEST: &str = include_str!("fixtures/catalog-invalid-digest.json");
const PINNED_SNAPSHOT: &str = include_str!("fixtures/snapshot-pinned.json");
const EMPTY_SNAPSHOT: &str = include_str!("fixtures/snapshot-empty-catalog.json");
const FUTURE_SNAPSHOT: &str = include_str!("fixtures/snapshot-unsupported-version.json");

fn error_codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics.iter().map(Diagnostic::code).collect()
}

fn error_paths(diagnostics: &[Diagnostic]) -> Vec<Option<&str>> {
    diagnostics.iter().map(Diagnostic::path).collect()
}

#[test]
fn catalogue_separates_the_four_kinds_without_ecosystem_vocabulary() {
    let catalog = Catalog::from_json(MIXED_KINDS).expect("the mixed-kind fixture is valid");

    let counts: Vec<(EntryKind, usize)> = EntryKind::ALL
        .into_iter()
        .map(|kind| (kind, catalog.entries_of_kind(kind).count()))
        .collect();

    assert_eq!(
        counts,
        vec![
            (EntryKind::Software, 1),
            (EntryKind::Data, 1),
            (EntryKind::Model, 1),
            (EntryKind::Skill, 1),
        ]
    );

    // Kind selection is a partition: every entry belongs to exactly one kind.
    let selected: usize = counts.iter().map(|(_, count)| count).sum();
    assert_eq!(selected, catalog.entries.len());

    // The wire format carries no ecosystem, channel, or build vocabulary: an
    // adapter adds those, not the neutral catalogue.
    let wire = catalog.to_json().expect("serializes");
    for ecosystem_key in ["\"ecosystem\"", "\"channel\"", "\"build\"", "\"registry\""] {
        assert!(
            !wire.contains(ecosystem_key),
            "{ecosystem_key} must not appear in a neutral catalogue"
        );
    }
    assert!(catalog.validate().is_empty(), "the fixture is clean");
}

#[test]
fn catalogue_round_trips_through_json_unchanged() {
    let first = Catalog::from_json(MIXED_KINDS).expect("valid");
    let wire = first.to_json().expect("serializes");
    let second = Catalog::from_json(&wire).expect("round-trips");

    assert_eq!(first, second);
    assert_eq!(wire, second.to_json().expect("stable"));
}

#[test]
fn unknown_fields_fail_closed() {
    let error = Catalog::from_json(UNKNOWN_FIELD).expect_err("unknown field is rejected");

    match error {
        ModelError::Json(source) => {
            let message = source.to_string();
            assert!(message.contains("channel"), "unexpected message: {message}");
            assert!(
                message.contains("unknown field"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected a JSON error, got {other}"),
    }
}

#[test]
fn unsupported_record_versions_fail_closed() {
    match Catalog::from_json(UNSUPPORTED_VERSION) {
        Err(ModelError::UnsupportedSchemaVersion {
            family,
            found,
            supported,
        }) => {
            assert_eq!(family, "catalog");
            assert_eq!(found.get(), 3);
            assert_eq!(supported, SchemaVersion::V1);
        }
        other => panic!("expected an unsupported version error, got {other:?}"),
    }

    match Snapshot::from_json(FUTURE_SNAPSHOT) {
        Err(ModelError::UnsupportedSchemaVersion { family, found, .. }) => {
            assert_eq!(family, "snapshot");
            assert_eq!(found.get(), 2);
        }
        other => panic!("expected an unsupported version error, got {other:?}"),
    }
}

#[test]
fn digests_must_be_lowercase_hexadecimal() {
    match Catalog::from_json(INVALID_DIGEST) {
        Err(ModelError::Json(source)) => {
            let message = source.to_string();
            assert!(
                message.contains("invalid sha256 digest"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected a rejected digest, got {other:?}"),
    }

    let digest =
        Sha256Digest::parse("39d87dec0138df1d96638f60fa89e66b30afba6a2c5ee29d7a87233b2c45bda7")
            .expect("valid digest");
    assert_eq!(digest.to_string(), digest.as_str());
}

#[test]
fn validation_reports_duplicates_empties_and_missing_artifacts_by_path() {
    let catalog = Catalog::from_json(INVALID_ENTRIES).expect("the fixture parses");
    let diagnostics = catalog.validate();

    assert_eq!(
        error_codes(&diagnostics),
        vec![
            "catalog.entry.duplicate",
            "catalog.entry.empty",
            "catalog.entry.empty",
            "catalog.entry.no_artifacts",
        ]
    );
    assert_eq!(
        error_paths(&diagnostics),
        vec![
            Some("entries[1]"),
            Some("entries[2].name"),
            Some("entries[2].version"),
            Some("entries[2]"),
        ]
    );
    assert!(!diagnostics[0].is_error() || diagnostics[0].severity() == Severity::Error);
    assert_eq!(diagnostics[3].severity(), Severity::Warning);
}

#[test]
fn validation_is_invariant_under_entry_permutation() {
    let catalog = Catalog::from_json(INVALID_ENTRIES).expect("parses");

    let mut rotations: Vec<Vec<String>> = Vec::new();
    for rotation in 0..catalog.entries.len() {
        let mut entries = catalog.entries.clone();
        entries.rotate_left(rotation);
        let rotated = Catalog {
            schema_version: catalog.schema_version,
            entries,
        };
        let diagnostics = rotated.validate();
        let mut codes: Vec<String> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code().to_owned())
            .collect();
        codes.sort_unstable();
        rotations.push(codes);
    }

    assert!(
        rotations.windows(2).all(|pair| pair[0] == pair[1]),
        "validation must not depend on entry order: {rotations:?}"
    );
}

#[test]
fn snapshot_pins_the_catalogue_it_embeds() {
    let snapshot = Snapshot::from_json(PINNED_SNAPSHOT).expect("valid snapshot");
    let catalog = Catalog::from_json(MIXED_KINDS).expect("valid catalog");

    assert_eq!(snapshot.id, "universe-v1.0.0-linux-64");
    assert_eq!(snapshot.catalog, catalog);
    assert!(snapshot.validate().is_empty());
    assert_eq!(
        Snapshot::from_json(&snapshot.to_json().expect("serializes")).expect("round-trips"),
        snapshot
    );
}

#[test]
fn snapshot_without_content_is_rejected() {
    let snapshot = Snapshot::from_json(EMPTY_SNAPSHOT).expect("parses");
    let diagnostics = snapshot.validate();

    assert_eq!(
        error_codes(&diagnostics),
        vec!["snapshot.id.empty", "snapshot.catalog.empty"]
    );
    assert_eq!(
        error_paths(&diagnostics),
        vec![Some("id"), Some("catalog.entries")]
    );
}

#[test]
fn result_envelope_carries_typed_diagnostics_and_artifacts() {
    let artifact = Artifact {
        id: universe_core::ArtifactId::new("cloudcompare", "2.13.2").expect("valid identity"),
        digest: Sha256Digest::parse(
            "39d87dec0138df1d96638f60fa89e66b30afba6a2c5ee29d7a87233b2c45bda7",
        )
        .expect("valid digest"),
        size_bytes: 1048576,
        media_type: None,
    };
    let envelope = Envelope::new(
        STDOUT_SCHEMA_VERSION,
        "check",
        false,
        "result-value".to_owned(),
        vec![
            Diagnostic::new(Severity::Error, "catalog.entry.duplicate", "twice")
                .with_path("entries[1]"),
        ],
        vec![artifact],
    );

    let wire = envelope.to_json().expect("serializes");
    let parsed: Envelope<String> = serde_json::from_str(&wire).expect("round-trips");

    assert_eq!(parsed, envelope);
    assert!(parsed.has_errors());
    assert_eq!(parsed.diagnostics[0].path(), Some("entries[1]"));
    assert_eq!(parsed.artifacts[0].id.name, "cloudcompare");
    assert!(wire.starts_with("{\"schema_version\":2,\"command\":\"check\",\"ok\":false"));
}
