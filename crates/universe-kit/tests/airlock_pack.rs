//! The producer half of a restore: writing the payload a bootstrapper reads.
//!
//! `universe-airlock-bootstrap` verifies a *closed* payload — a `universe-release.json` naming
//! every artifact with its size and digest, and one file per artifact beside it (`UNI-33`). The
//! rendered entrypoint hands over to that binary, and on this repository the handover used to end
//! at `[airlock.payload.manifest] cannot read the release manifest`, because nothing wrote the
//! file. These tests are about the verb that writes it.
//!
//! The seam each test observes is the one a reader relies on: the descriptor is parsed back as
//! `universe_core::manifest::ReleaseManifest`, and one test runs the **real** bootstrapper
//! binary against the directory this verb produced. That binary is the consumer's reader; a
//! producer test that only inspected its own output would pass while the two disagreed.
//!
//! Nothing here reaches the network, and every fixture lives in a `TempDir`.

use std::path::{Path, PathBuf};

use tempfile::TempDir;
use universe_core::CommandOutput;
use universe_core::manifest::{PackageKind, ReleaseManifest, ReleasedArtifact};
use universe_workflow::sha256::sha256_hex;

mod support;

/// The release version every fixture package is built at.
const VERSION: &str = "0.1.0";

/// The commit the fixture's restore scope pins.
const PIN: &str = "0123456789abcdef0123456789abcdef01234567";

/// The bootstrapper's binary name, published as the package `universe-airlock-bootstrap`.
const BOOTSTRAPPER: &str = "universe-airlock-bootstrap";

/// A project directory with a manifest and whatever outputs a test built.
struct Fixture {
    directory: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            directory: tempfile::tempdir().expect("temp dir"),
        }
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    /// Build one `.conda` under the output path the manifest declares for `package`.
    ///
    /// The archive's own `info/index.json` is what carries the name and version, so this is the
    /// fixture's independent statement of what the artifact *is*, and the verb has to read it
    /// rather than guess from the file name.
    fn build(&self, package: &str, version: &str) -> PathBuf {
        let directory = self.root().join("output").join(package).join("linux-64");
        std::fs::create_dir_all(&directory).expect("output directory");
        let path = directory.join(format!("{package}-{version}-h1234567_0.conda"));
        support::write_conda(
            &path,
            &[(
                "info/index.json",
                support::index(package, version, None).as_bytes(),
            )],
        );
        path
    }

    /// Write the manifest, with `delivery` spliced into place.
    fn manifest(&self, delivery: &str) {
        let text = format!(
            r#"
schema = 1

[project]
name = "pack-fixture"
platforms = ["linux-64"]

[[packages]]
name = "pack-fixture-bootstrap"
kind = "conda"
command = ["true"]
outputs = ["output/pack-fixture-bootstrap/linux-64/*.conda"]
smoke = ["true"]

[airlock.developer]

[airlock.developer.restore]
remote = "https://example.invalid/universe.git"
branch = "bootstrap"
commit = "{PIN}"
{delivery}"#
        );
        std::fs::write(self.root().join("universe.toml"), text).expect("manifest");
    }

    /// A manifest that declares a delivery scope, and nothing else unusual.
    fn declared(&self) {
        self.manifest("\n[airlock.delivery]\n");
    }

    /// The payload directory the bootstrapper reads by default.
    fn payload(&self) -> PathBuf {
        self.root().join(".universe-airlock").join("payload")
    }

    fn pack(&self, extra: &[&str]) -> CommandOutput {
        let root = self.root().display().to_string();
        let mut arguments = vec!["airlock", "pack", "--root", &root];
        arguments.extend_from_slice(extra);
        universe_kit::run(arguments)
    }

    /// The descriptor this verb wrote, parsed as the record the consumer reads.
    fn descriptor(&self) -> ReleaseManifest {
        let text = std::fs::read_to_string(self.payload().join("universe-release.json"))
            .expect("the payload has no release manifest");
        ReleaseManifest::from_json(&text).expect("the descriptor is not a release manifest")
    }
}

/// What one artifact's bytes say about themselves, computed here rather than by the verb.
#[derive(Debug, Eq, PartialEq)]
struct Artifact {
    filename: String,
    digest: String,
    size: u64,
}

/// Describe `path` from its bytes, independently of anything the verb computes.
fn describe(path: &Path) -> Artifact {
    let bytes = std::fs::read(path).expect("read the artifact");
    Artifact {
        filename: path
            .file_name()
            .expect("an artifact has a name")
            .to_string_lossy()
            .into_owned(),
        digest: sha256_hex(&bytes),
        size: bytes.len() as u64,
    }
}

#[test]
fn the_payload_the_verb_writes_is_the_one_the_bootstrapper_verifies() {
    // The whole point of the change: a producer writes, the consumer reads, and neither one
    // gets to agree with itself. The bootstrapper is the real binary this workspace built.
    let fixture = Fixture::new();
    let built = fixture.build("pack-fixture-bootstrap", VERSION);
    fixture.declared();

    let packed = fixture.pack(&[]);
    assert_eq!(
        packed.exit_code(),
        0,
        "pack failed\nstdout: {}\nstderr: {}",
        packed.stdout(),
        packed.stderr()
    );

    // What the producer claims, checked against the bytes and against the archive's own index.
    let artifact = describe(&built);
    let descriptor = fixture.descriptor();
    assert_eq!(
        descriptor.commit, PIN,
        "the descriptor names another commit"
    );
    assert_eq!(descriptor.version, VERSION);
    assert_eq!(descriptor.packages.len(), 1);
    let package = &descriptor.packages[0];
    assert_eq!(package.name, "pack-fixture-bootstrap");
    assert_eq!(package.version, VERSION);
    assert_eq!(package.kind, PackageKind::Conda);
    assert_eq!(
        package.artifacts,
        vec![ReleasedArtifact {
            filename: artifact.filename.clone(),
            sha256: artifact.digest.parse().expect("a digest"),
            size: artifact.size,
            stores: vec!["releases".to_owned()],
        }],
        "the descriptor does not describe the artifact that was built"
    );
    let copied = fixture.payload().join(&artifact.filename);
    assert_eq!(
        describe(&copied),
        Artifact {
            filename: artifact.filename.clone(),
            digest: artifact.digest.clone(),
            size: artifact.size,
        },
        "the payload holds different bytes than the artifact"
    );

    // A second pack over its own output is byte-identical: the payload is a function of the
    // artifacts, not of when it ran.
    let manifest =
        std::fs::read(fixture.payload().join("universe-release.json")).expect("manifest");
    assert_eq!(fixture.pack(&[]).exit_code(), 0);
    assert_eq!(
        std::fs::read(fixture.payload().join("universe-release.json")).expect("manifest"),
        manifest,
        "a second pack of the same artifacts rewrote the descriptor"
    );

    // And the consumer accepts it.
    let bootstrap = Path::new(env!("CARGO_BIN_EXE_universe-kit"))
        .parent()
        .expect("Cargo puts binaries in a directory")
        .join(BOOTSTRAPPER);
    let verified = std::process::Command::new(&bootstrap)
        .arg("verify")
        .arg("--payload")
        .arg(fixture.payload())
        .arg("--commit")
        .arg(PIN)
        .env("HOME", fixture.root().join("home"))
        .output()
        .expect("the bootstrapper runs");
    let stdout = String::from_utf8_lossy(&verified.stdout);
    assert!(
        verified.status.success(),
        "the bootstrapper refused the payload this verb wrote\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&verified.stderr)
    );
    // The lines the consumer prints are the evidence: it read this directory as one payload,
    // at the version and commit the producer recorded, with one artifact in it.
    for expected in [
        format!("version: {VERSION}"),
        format!("commit: {PIN}"),
        "artifacts: 1".to_owned(),
    ] {
        assert!(
            stdout.contains(&expected),
            "the bootstrapper did not report `{expected}`: {stdout}"
        );
    }
}

#[test]
fn a_manifest_that_declares_no_delivery_scope_is_refused() {
    let fixture = Fixture::new();
    fixture.build("pack-fixture-bootstrap", VERSION);
    fixture.manifest("");

    let packed = fixture.pack(&[]);

    assert_ne!(packed.exit_code(), 0, "{}", packed.stdout());
    assert!(
        packed.stderr().contains("airlock.pack.scope"),
        "the refusal does not name the missing scope: {}",
        packed.stderr()
    );
    assert!(
        !fixture.payload().exists(),
        "a refused pack still created a payload directory"
    );
}

#[test]
fn an_output_the_build_did_not_produce_is_refused() {
    let fixture = Fixture::new();
    fixture.declared();

    let packed = fixture.pack(&[]);

    assert_ne!(packed.exit_code(), 0, "{}", packed.stdout());
    assert!(
        packed.stderr().contains("airlock.pack.output"),
        "the refusal does not name the missing output: {}",
        packed.stderr()
    );
    assert!(
        !fixture.payload().join("universe-release.json").exists(),
        "a refused pack still wrote a descriptor"
    );
}

#[test]
fn a_payload_directory_holding_something_else_is_refused_rather_than_cleaned() {
    // The bootstrapper refuses a payload with files its manifest does not name, and a signature
    // over the previous manifest would be left describing bytes that are no longer there.
    // Deleting a producer's files is the wrong fix for that; refusing to write a half-closed
    // directory is the right one.
    let fixture = Fixture::new();
    fixture.build("pack-fixture-bootstrap", VERSION);
    fixture.declared();
    let foreign = fixture.payload().join("stale-0.0.9-h1234567_0.conda");
    std::fs::create_dir_all(fixture.payload()).expect("payload directory");
    std::fs::write(&foreign, b"an artifact nobody asked for\n").expect("foreign artifact");

    let packed = fixture.pack(&[]);

    assert_ne!(packed.exit_code(), 0, "{}", packed.stdout());
    assert!(
        packed.stderr().contains("airlock.pack.stale"),
        "the refusal does not name the stale file: {}",
        packed.stderr()
    );
    assert!(
        foreign.is_file(),
        "the verb deleted a file it did not write"
    );
    assert!(
        !fixture.payload().join("universe-release.json").exists(),
        "a refused pack still wrote a descriptor"
    );
}

#[test]
fn an_archive_whose_index_disagrees_with_its_file_name_is_refused() {
    // The descriptor names a file name *and* an identity, and the bootstrapper copies by file
    // name. An archive built as one package and published under another name is a payload that
    // restores something nobody can name, so it is caught here rather than on the target.
    let fixture = Fixture::new();
    let directory = fixture
        .root()
        .join("output/pack-fixture-bootstrap/linux-64");
    std::fs::create_dir_all(&directory).expect("output directory");
    support::write_conda(
        &directory.join("pack-fixture-bootstrap-0.1.0-h1234567_0.conda"),
        &[(
            "info/index.json",
            support::index("something-else", "9.9.9", None).as_bytes(),
        )],
    );
    fixture.declared();

    let packed = fixture.pack(&[]);

    assert_ne!(packed.exit_code(), 0, "{}", packed.stdout());
    assert!(
        packed.stderr().contains("airlock.pack.identity"),
        "the refusal does not say the identity disagrees with the name: {}",
        packed.stderr()
    );
    assert!(
        !fixture.payload().join("universe-release.json").exists(),
        "a refused pack still wrote a descriptor"
    );
}
