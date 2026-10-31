//! Fixtures for the producer verbs.
//!
//! A `.conda` is a zip of two zstd-compressed tars. Building one here rather than committing
//! a binary spells out the format the auditor has to satisfy, and keeps the fixture
//! deterministic: stored members, a fixed timestamp, no compression surprise.

#![allow(dead_code)]

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// An `info/index.json` for `name`, with the fields a real one carries.
#[must_use]
pub fn index(name: &str, version: &str, licence: Option<&str>) -> String {
    let licence = licence.map_or_else(String::new, |value| {
        format!("  \"license\": \"{value}\",\n")
    });
    format!(
        "{{\n  \"name\": \"{name}\",\n  \"version\": \"{version}\",\n  \"build\": \"h1234567_0\",\n  \"build_number\": 0,\n  \"subdir\": \"linux-64\",\n{licence}  \"depends\": [\"libgcc >=13\"],\n  \"platform\": \"linux\",\n  \"arch\": \"x86_64\"\n}}"
    )
}

/// The `info/about.json` a well-formed package carries.
#[must_use]
pub fn about(licence: &str, license_file: &str) -> String {
    format!("{{\n  \"license\": \"{licence}\",\n  \"license_file\": \"{license_file}\"\n}}")
}

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
