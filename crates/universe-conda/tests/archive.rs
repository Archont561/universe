//! What an audit can learn from a `.conda` archive it was handed.

mod support;

use support::{INDEX, write_conda};
use tempfile::TempDir;
use universe_conda::open_package;

#[test]
fn an_archive_reports_its_identity_size_and_digest() {
    let directory = TempDir::new().expect("temp dir");
    let archive_path = directory
        .path()
        .join("universe-airlock-0.1.0-h1234567_0.conda");
    write_conda(&archive_path, &[("info/index.json", INDEX.as_bytes())]);

    let package = open_package(&archive_path).expect("open package");

    let index = package.index();
    assert_eq!(index.name, "universe-airlock");
    assert_eq!(index.version, "0.1.0");
    assert_eq!(index.build, "h1234567_0");
    assert_eq!(index.build_number, 0);
    assert_eq!(index.subdir, "linux-64");
    assert_eq!(index.license.as_deref(), Some("MIT"));
    assert_eq!(index.depends, ["libgcc >=13"]);

    let artifact = package.artifact();
    assert_eq!(artifact.id.name, "universe-airlock");
    assert_eq!(artifact.id.version, "0.1.0");
    assert_eq!(
        artifact.size_bytes,
        std::fs::metadata(&archive_path).expect("metadata").len()
    );

    // An independent implementation of the same hash, so a digest the reader
    // computed with its own bug cannot agree with itself.
    let expected = rattler_digest::compute_file_digest::<rattler_digest::Sha256>(&archive_path)
        .expect("digest");
    assert_eq!(artifact.digest.as_str(), hex_of(&expected));
}

fn hex_of(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}
