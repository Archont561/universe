//! What an archive has to prove about its licence before it may publish.

mod support;

use support::{INDEX, write_conda};
use tempfile::TempDir;
use universe_conda::{LicenceRules, audit_licence, open_package};

/// An `info/about.json` naming its licence and the file that carries it.
const ABOUT: &str = r#"{
  "home": "https://github.com/Archont561/universe",
  "license": "MIT",
  "license_family": "MIT",
  "license_file": "info/licenses/LICENSE",
  "summary": "Sealed offline snapshots"
}"#;

const LICENCE_TEXT: &[u8] = b"MIT License\n\nCopyright (c) 2026 Archont561\n";

fn fixture(files: &[(&str, &[u8])]) -> (TempDir, std::path::PathBuf) {
    let directory = TempDir::new().expect("temp dir");
    let path = directory
        .path()
        .join("universe-airlock-0.1.0-h1234567_0.conda");
    write_conda(&path, files);
    (directory, path)
}

#[test]
fn a_declared_licence_and_its_bundled_text_are_accepted() {
    let (_directory, path) = fixture(&[
        ("info/index.json", INDEX.as_bytes()),
        ("info/about.json", ABOUT.as_bytes()),
        ("info/licenses/LICENSE", LICENCE_TEXT),
    ]);

    let package = open_package(&path).expect("open package");
    let audit = audit_licence(&package, LicenceRules::default()).expect("audit licence");

    assert!(audit.violations().is_empty(), "{:?}", audit.violations());
    assert_eq!(audit.expression(), Some("MIT"));
    assert_eq!(audit.files(), ["info/licenses/LICENSE"]);
}

#[test]
fn a_licence_file_the_archive_does_not_carry_is_refused() {
    let (_directory, path) = fixture(&[
        ("info/index.json", INDEX.as_bytes()),
        ("info/about.json", ABOUT.as_bytes()),
    ]);

    let package = open_package(&path).expect("open package");
    let audit = audit_licence(&package, LicenceRules::default()).expect("audit licence");

    let codes: Vec<&str> = audit
        .violations()
        .iter()
        .map(|violation| violation.code())
        .collect();
    assert_eq!(codes, ["conda.licence.file.missing"]);
    assert_eq!(
        audit.violations()[0].message(),
        "info/about.json names info/licenses/LICENSE, which the archive does not carry"
    );
}

#[test]
fn a_licence_expression_that_is_not_spdx_is_refused() {
    let about = ABOUT.replace("\"MIT\"", "\"MIT OR\"");
    let (_directory, path) = fixture(&[
        ("info/index.json", INDEX.as_bytes()),
        ("info/about.json", about.as_bytes()),
        ("info/licenses/LICENSE", LICENCE_TEXT),
    ]);

    let package = open_package(&path).expect("open package");
    let audit = audit_licence(&package, LicenceRules::default()).expect("audit licence");

    let codes: Vec<&str> = audit
        .violations()
        .iter()
        .map(|violation| violation.code())
        .collect();
    assert_eq!(codes, ["conda.licence.expression.invalid"]);
}
