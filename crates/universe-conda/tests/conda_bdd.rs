//! Scenarios for what an audit learns from a `.conda` archive and from a recipe.
//!
//! Every fixture here is built, not committed: an archive is a zip of two zstd tars, so
//! writing one in a scenario spells out the format the reader has to satisfy. Nothing is
//! mocked — the digest is taken from real bytes on a real filesystem, and the licence file is
//! looked for in the real directory the recipe lives in.

mod support;

use std::path::PathBuf;

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use support::{INDEX, write_conda};
use tempfile::TempDir;
use universe_conda::{
    CondaPackage, LicenceRules, PackageError, RecipeRules, audit_licence, lint_recipe, open_package,
};
use universe_core::diagnostic::Diagnostic;

/// One archive, the identity it reported, and the refusals a scenario collected.
#[derive(Default)]
struct Archive {
    directory: Option<TempDir>,
    path: Option<PathBuf>,
    readings: Vec<Reading>,
    refusal: Option<PackageError>,
    violations: Vec<Diagnostic>,
    recipe_directory: Option<TempDir>,
    findings: Vec<Diagnostic>,
}

/// What one open of an archive reported.
struct Reading {
    name: String,
    version: String,
    build: String,
    subdir: String,
    digest: String,
    size_bytes: u64,
}

impl Archive {
    /// The scratch directory the archive lives in, created on first use.
    fn directory(&mut self) -> &TempDir {
        self.directory.get_or_insert_with(|| {
            TempDir::new().expect("a scratch directory for the archive fixture")
        })
    }

    /// The archive path a scenario declared, created on first use.
    fn archive_path(&mut self) -> PathBuf {
        let path = self
            .directory()
            .path()
            .join("universe-airlock-0.1.0-h1234567_0.conda");
        self.path = Some(path.clone());
        path
    }

    fn open(&mut self) {
        let path = self.path.clone().expect("a scenario built an archive");
        match open_package(&path) {
            Ok(package) => self.readings.push(reading_of(&package)),
            Err(error) => self.refusal = Some(error),
        }
    }

    fn last_reading(&self) -> &Reading {
        self.readings.last().expect("the archive was opened")
    }

    fn refusal(&self) -> &PackageError {
        self.refusal
            .as_ref()
            .expect("opening the archive was refused")
    }
}

fn reading_of(package: &CondaPackage) -> Reading {
    let index = package.index();
    let artifact = package.artifact();
    Reading {
        name: index.name.clone(),
        version: index.version.clone(),
        build: index.build.clone(),
        subdir: index.subdir.clone(),
        digest: artifact.digest.as_str().to_owned(),
        size_bytes: artifact.size_bytes,
    }
}

#[fixture]
fn archive() -> Archive {
    Archive::default()
}

#[given("an archive carrying its index record")]
fn an_indexed_archive(archive: &mut Archive) {
    let path = archive.archive_path();
    write_conda(&path, &[("info/index.json", INDEX.as_bytes())]);
}

#[given("an archive with no index record")]
fn an_archive_without_an_index(archive: &mut Archive) {
    let path = archive.archive_path();
    write_conda(&path, &[("info/about.json", b"{}")]);
}

#[given("an archive whose about record names a licence file it does not carry")]
fn a_licence_file_that_is_absent(archive: &mut Archive) {
    const ABOUT: &str = r#"{"license":"MIT","license_file":"info/licenses/LICENSE"}"#;
    let path = archive.archive_path();
    write_conda(
        &path,
        &[
            ("info/index.json", INDEX.as_bytes()),
            ("info/about.json", ABOUT.as_bytes()),
        ],
    );
}

#[given("a recipe whose url source names no digest")]
fn an_unpinned_recipe(archive: &mut Archive) {
    const RECIPE: &str = r#"
schema_version: 1
context:
  version: "1.2.3"
package:
  name: example
  version: ${{ version }}
source:
  - url: https://example.invalid/example-1.2.3.tar.gz
build:
  number: 0
about:
  license: MIT
  license_file: LICENSE
"#;
    let directory = TempDir::new().expect("a scratch directory for the recipe fixture");
    // The licence file is present, so the only finding is the one the scenario is about.
    std::fs::write(directory.path().join("LICENSE"), b"MIT License\n").expect("write the licence");
    std::fs::write(directory.path().join("recipe.yaml"), RECIPE).expect("write the recipe");
    archive.recipe_directory = Some(directory);
}

#[when("the archive is opened")]
fn opened(archive: &mut Archive) {
    archive.open();
}

#[when("the archive is opened twice")]
fn opened_twice(archive: &mut Archive) {
    archive.open();
    archive.open();
}

#[when("a byte of the archive changes")]
fn the_bytes_change(archive: &mut Archive) {
    use std::io::Write;

    let path = archive.path.clone().expect("a scenario built an archive");
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("the archive is writable");
    file.write_all(b"\n").expect("append a byte");
}

#[when("the archive is opened again")]
fn opened_again(archive: &mut Archive) {
    archive.open();
}

#[when("the licence is audited")]
fn audited(archive: &mut Archive) {
    let path = archive.path.clone().expect("a scenario built an archive");
    let package = open_package(&path).expect("open the archive");
    let audit = audit_licence(&package, LicenceRules::default()).expect("audit the licence");
    archive.violations = audit.violations().to_vec();
}

#[when("the recipe is linted")]
fn linted(archive: &mut Archive) {
    let directory = archive.recipe_directory.as_ref().expect("a recipe fixture");
    let text =
        std::fs::read_to_string(directory.path().join("recipe.yaml")).expect("read the recipe");
    archive.findings = lint_recipe(directory.path(), &text, RecipeRules::default());
}

#[then("the reading names the package, version, build and subdir the index declares")]
fn the_identity(archive: &Archive) {
    let reading = archive.last_reading();
    assert_eq!(reading.name, "universe-airlock");
    assert_eq!(reading.version, "0.1.0");
    assert_eq!(reading.build, "h1234567_0");
    assert_eq!(reading.subdir, "linux-64");
}

#[then("the reading is as large as the file on disk")]
fn the_size(archive: &Archive) {
    let path = archive.path.as_ref().expect("an archive");
    let on_disk = std::fs::metadata(path)
        .expect("the archive's metadata")
        .len();
    assert_eq!(archive.last_reading().size_bytes, on_disk);
}

#[then("both readings name the same digest")]
fn the_same_digest(archive: &Archive) {
    assert_eq!(
        archive.readings.len(),
        2,
        "the archive was not opened twice"
    );
    assert_eq!(archive.readings[0].digest, archive.readings[1].digest);
}

#[then("the reading names a different digest")]
fn a_different_digest(archive: &Archive) {
    assert_eq!(
        archive.readings.len(),
        3,
        "the archive was not opened a third time"
    );
    assert_ne!(
        archive.readings[2].digest, archive.readings[1].digest,
        "the digest did not follow the bytes"
    );
}

#[then("the reading is refused because the index is missing")]
fn the_index_is_missing(archive: &Archive) {
    assert!(
        matches!(archive.refusal(), PackageError::MissingIndex),
        "expected a missing index, got {:?}",
        archive.refusal()
    );
}

#[then("the audit reports one violation")]
fn one_violation(archive: &Archive) {
    let codes: Vec<&str> = archive.violations.iter().map(Diagnostic::code).collect();
    assert_eq!(codes, ["conda.licence.file.missing"], "{codes:?}");
}

#[then("the violation names the licence file that is missing")]
fn names_the_licence_file(archive: &Archive) {
    let message = archive.violations[0].message();
    assert!(
        message.contains("info/licenses/LICENSE"),
        "the violation does not say which file is missing: {message}"
    );
}

#[then("the lint reports one finding for the missing digest")]
fn one_finding(archive: &Archive) {
    let codes: Vec<&str> = archive.findings.iter().map(Diagnostic::code).collect();
    assert_eq!(codes, ["recipe.source.sha256.missing"], "{codes:?}");
    assert_eq!(archive.findings[0].path(), Some("source[0].sha256"));
}

#[scenario(
    path = "features/conda.feature",
    name = "An archive reports the identity its index declares"
)]
fn an_archive_reports_its_identity(archive: Archive) {
    let _ = archive;
}

#[scenario(
    path = "features/conda.feature",
    name = "The digest is of the bytes, not of the file name"
)]
fn the_digest_follows_the_bytes(archive: Archive) {
    let _ = archive;
}

#[scenario(
    path = "features/conda.feature",
    name = "An archive with no index record is refused"
)]
fn an_archive_without_an_index_is_refused(archive: Archive) {
    let _ = archive;
}

#[scenario(
    path = "features/conda.feature",
    name = "A licence file named by the archive must be carried by it"
)]
fn a_missing_licence_file_is_refused(archive: Archive) {
    let _ = archive;
}

#[scenario(
    path = "features/conda.feature",
    name = "A recipe whose source is not pinned is refused"
)]
fn an_unpinned_recipe_is_refused(archive: Archive) {
    let _ = archive;
}

#[given("a recipe that takes the C stdlib without suppressing its run exports")]
fn a_recipe_that_leaks_c_run_exports(archive: &mut Archive) {
    const RECIPE: &str = r#"
schema_version: 1
context:
  version: "1.2.3"
package:
  name: example
  version: ${{ version }}
source:
  - path: .
build:
  number: 0
requirements:
  build:
    - ${{ stdlib('c') }}
about:
  license: MIT
  license_file: LICENSE
"#;
    let directory = TempDir::new().expect("a scratch directory for the recipe fixture");
    std::fs::write(directory.path().join("LICENSE"), b"MIT License\n").expect("write the licence");
    std::fs::write(directory.path().join("recipe.yaml"), RECIPE).expect("write the recipe");
    archive.recipe_directory = Some(directory);
}

#[then("the lint reports the leaked run export")]
fn lint_reports_the_leak(archive: &Archive) {
    let codes: Vec<&str> = archive.findings.iter().map(Diagnostic::code).collect();
    assert_eq!(codes, ["recipe.run_exports.leaked"]);
    assert_eq!(
        archive.findings[0].path(),
        Some("requirements.ignore_run_exports.by_name")
    );
}

#[scenario(
    path = "features/conda.feature",
    name = "A recipe that leaks the C stdlib run exports is refused"
)]
fn a_leaked_c_run_export_is_refused(archive: Archive) {
    let _ = archive;
}
