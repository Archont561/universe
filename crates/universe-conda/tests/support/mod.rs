//! Fixtures for the Conda adapter's tests.
//!
//! A `.conda` is a zip of two zstd-compressed tars, and building one here rather
//! than committing a binary spells out the format the reader has to satisfy. The
//! archives are deterministic: stored members, a fixed timestamp, and no
//! compression-level surprises, so two runs produce the same bytes.

#![allow(dead_code)]

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// An `info/index.json` a fixture can start from, with the fields a real one has.
pub const INDEX: &str = r#"{
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

/// Write a `.conda` whose info section holds `files` and whose payload is empty.
pub fn write_conda(path: &Path, files: &[(&str, &[u8])]) {
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
    zip.write_all(&compress(&tar_bytes(files)))
        .expect("info bytes");

    zip.start_file("pkg-fixture.tar.zst", options)
        .expect("pkg entry");
    zip.write_all(&compress(&tar_bytes(&[])))
        .expect("pkg bytes");

    zip.finish().expect("finish archive");
}

fn compress(tar: &[u8]) -> Vec<u8> {
    zstd::encode_all(tar, 0).expect("compress tar")
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
