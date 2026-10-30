//! The repository's own manifest is the first consumer of the manifest schema.
//!
//! `decision-132` and `UNI-41` criterion #7 say the same thing from two directions: a
//! facet nobody has written for a real project is a facet that will not fit one, and the
//! owner-lane checkout the rendered workflows run in is this repository. So the manifest
//! at the repository root is read here, by the same parser the CLI uses, and it has to
//! pass the same validation a downstream project's would.

use std::path::PathBuf;

use universe_core::manifest::Manifest;

/// The repository root, from the crate's own manifest directory.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn the_repository_manifest_lints_clean() {
    let path = repository_root().join("universe.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));

    let manifest = Manifest::from_toml(&text).unwrap_or_else(|diagnostics| {
        panic!("the repository manifest does not parse: {diagnostics:?}")
    });

    let findings = manifest.validate();
    assert!(
        findings.is_empty(),
        "the repository manifest has findings: {findings:?}"
    );
}
