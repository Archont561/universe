//! Fixtures for the bootstrap's own tests.
//!
//! A `.conda` is a zip of two zstd-compressed tars, and building one here states the format the
//! installer has to satisfy rather than hiding it behind a binary blob. Members are stored
//! uncompressed, which is what the format requires and what the reader assumes.

#![allow(dead_code)]

use std::io::Write;

/// Build a `.conda` holding one executable, `bin/fixture`, whose output is `marker`.
///
/// The archive's `info/paths.json` records the digest and size of what it installs, so a restore
/// exercises the same per-file check a real package does.
#[must_use]
pub fn conda(marker: &str) -> Vec<u8> {
    let executable = format!("#!/bin/sh\nprintf '{marker}\\n'\n").into_bytes();
    let index = br#"{"name":"fixture","version":"0.1.0","build":"h1234567_0","build_number":0,"subdir":"linux-64","depends":[]}"#;
    let paths = format!(
        r#"{{"paths":[{{"_path":"bin/fixture","path_type":"hardlink","sha256":"{}","size_in_bytes":{}}}],"paths_version":1}}"#,
        digest(&executable),
        executable.len()
    );

    let mut info = tar::Builder::new(Vec::new());
    for (name, bytes) in [
        ("info/index.json", index.as_slice()),
        ("info/paths.json", paths.as_bytes()),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_cksum();
        info.append_data(&mut header, name, bytes)
            .expect("info entry");
    }

    let mut payload = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(executable.len() as u64);
    header.set_mode(0o755);
    header.set_mtime(0);
    header.set_cksum();
    payload
        .append_data(&mut header, "bin/fixture", executable.as_slice())
        .expect("payload entry");

    let mut buffer = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut buffer);
    // One fixed timestamp, because a `.conda` here stands for a published artifact: two calls
    // with the same marker have to be the same bytes, or a test that compares a restored file
    // with the fixture it was published from fails whenever the two land in different seconds.
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(
            zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).expect("a DOS timestamp"),
        );
    zip.start_file("metadata.json", options).expect("metadata");
    zip.write_all(br#"{"conda_pkg_format_version": 2}"#)
        .expect("metadata bytes");
    zip.start_file("info-fixture.tar.zst", options)
        .expect("info");
    zip.write_all(
        &zstd::encode_all(info.into_inner().expect("info tar").as_slice(), 0).expect("compress"),
    )
    .expect("info bytes");
    zip.start_file("pkg-fixture.tar.zst", options)
        .expect("payload");
    zip.write_all(
        &zstd::encode_all(payload.into_inner().expect("payload tar").as_slice(), 0)
            .expect("compress"),
    )
    .expect("payload bytes");
    zip.finish().expect("finish archive");
    buffer.into_inner()
}

/// Lowercase SHA-256, computed here rather than through the installer's hasher.
fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let mut text = String::new();
    for byte in hasher.finalize() {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}
