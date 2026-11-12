//! The repository's own manifest is the first consumer of the manifest schema.
//!
//! `decision-132` and `UNI-41` criterion #7 say the same thing from two directions: a
//! facet nobody has written for a real project is a facet that will not fit one, and the
//! owner-lane checkout the rendered workflows run in is this repository. So the manifest
//! at the repository root is read here, by the same parser the CLI uses, and it has to
//! pass the same validation a downstream project's would.

use std::path::PathBuf;

use universe_core::manifest::{Manifest, PackageKind};

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

#[test]
fn the_repository_manifest_declares_every_shipped_package() {
    // The owner widened the pre-release scope to the producer CLI and the umbrella
    // facade alongside Airlock, and `UNI-55` added the bootstrapper, which no ref
    // carried before. A manifest that quietly drops one of them would still lint clean;
    // this is the assertion that the release actually publishes all four.
    let path = repository_root().join("universe.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let manifest = Manifest::from_toml(&text).unwrap_or_else(|diagnostics| {
        panic!("the repository manifest does not parse: {diagnostics:?}")
    });

    let names: Vec<&str> = manifest
        .packages
        .iter()
        .map(|package| package.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "universe-airlock",
            "universe-airlock-bootstrap",
            "universe-kit",
            "universe"
        ],
        "the release manifest does not publish every shipped package"
    );
    for package in &manifest.packages {
        assert_eq!(package.kind, PackageKind::Conda);
        assert!(package.publish.iter().any(|store| store == "prefix-dev"));
        assert!(package.publish.iter().any(|store| store == "branch"));
        assert!(package.release);
        assert_eq!(package.command.first().map(String::as_str), Some("pixi"));
        assert_eq!(package.command.get(1).map(String::as_str), Some("run"));
    }
    let bootstrap = manifest
        .packages
        .iter()
        .find(|package| package.name == "universe-airlock-bootstrap")
        .expect("the bootstrapper");
    // It runs before anything is restored, so it may not depend on what it restores, and
    // nothing may depend on it: a facade that pulled it in would install a binary that has
    // to work before the environment installing it exists (`decision-115`).
    assert!(
        bootstrap.depends_on.is_empty(),
        "the bootstrapper declared a build order: {:?}",
        bootstrap.depends_on
    );

    let facade = manifest
        .packages
        .iter()
        .find(|package| package.name == "universe")
        .expect("the umbrella facade");
    assert_eq!(
        facade.depends_on,
        vec!["universe-kit".to_owned(), "universe-airlock".to_owned()]
    );
    assert!(
        !facade
            .depends_on
            .iter()
            .any(|name| name == "universe-airlock-bootstrap"),
        "the facade must not pull in the bootstrapper"
    );
}
