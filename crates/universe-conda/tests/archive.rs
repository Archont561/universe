//! What an audit can learn from a `.conda` archive it was handed.
//!
//! The fixture is built here rather than downloaded: a `.conda` is a zip of two
//! zstd-compressed tars, and spelling that out in the test is a specification of
//! the format the reader has to satisfy.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use tempfile::TempDir;
use universe_conda::open_package;

/// The `info/index.json` the fixture declares, as a Conda index spells it.
const INDEX: &str = r#"{
  "name": "universe-airlock",
  "version": "0.1.0",
  "build": "h1234567_0",
  "build_number": 0,
  "subdir": "linux-64",
  "license": "MIT",
  "depends": ["libgcc >=13"],
  "platform": "linux",
  "arch": "x86_64"
}"#;

/// A `.conda` containing only `info/index.json`, with the members a reader expects.
///
/// The archive is deliberately minimal and deterministic: the zip stores its
/// members uncompressed and every entry carries the same fixed timestamp, so two
/// runs produce the same bytes.
fn write_conda(path: &Path, index: &str) {
    let file = File::create(path).expect("create archive");
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    zip.start_file("metadata.json", options)
        .expect("metadata entry");
    zip.write_all(br#"{"conda_pkg_format_version": 2}"#)
        .expect("metadata bytes");

    zip.start_file("info-fixture.tar.zst", options)
        .expect("info entry");
    let info_tar = tar_bytes(&[("info/index.json", index.as_bytes())]);
    zip.write_all(&zstd::encode_all(info_tar.as_slice(), 0).expect("compress info tar"))
        .expect("info bytes");

    zip.start_file("pkg-fixture.tar.zst", options)
        .expect("pkg entry");
    let pkg_tar = tar_bytes(&[]);
    zip.write_all(&zstd::encode_all(pkg_tar.as_slice(), 0).expect("compress pkg tar"))
        .expect("pkg bytes");

    zip.finish().expect("finish archive");
}

/// A tar in memory, one file per pair, with fixed ownership and timestamps.
fn tar_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, contents) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_cksum();
        builder
            .append_data(&mut header, *name, *contents)
            .expect("append entry");
    }
    builder.into_inner().expect("finish tar")
}

#[test]
fn an_archive_reports_its_identity_size_and_digest() {
    let directory = TempDir::new().expect("temp dir");
    let archive_path = directory
        .path()
        .join("universe-airlock-0.1.0-h1234567_0.conda");
    write_conda(&archive_path, INDEX);

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

    // An independent implementation of the same hash, from the crate the reader
    // does not use for this: a digest the reader computed with its own bug would
    // agree with itself.
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
