//! A payload fixture builder for the consumer's verification scenarios.
//!
//! The digest is **not** computed by the code under test: it is the SHA-256 of a fixed byte
//! string, taken from an independent source and written down here. A verifier that agrees
//! with its own hasher proves nothing.
//!
//! Each integration test crate compiles this module separately and uses a different subset of
//! the builder, so a complete fixture library is deliberate rather than dead code.
#![allow(dead_code)]

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub const ARTIFACT: &[u8] = b"the only artifact\n";
pub const ARTIFACT_NAME: &str = "airlock-0.1.0.conda";
pub const ARTIFACT_SHA256: &str =
    "f1c50761eb511f773358beb44bfafdd5a4684123fdb8dbae8b72a5acb10bcd19";
pub const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
pub const MANIFEST_FILE: &str = "universe-release.json";
pub const SIGNATURE_FILE: &str = "universe-release.json.minisig";

/// One payload directory, deleted when the scenario ends.
pub struct Payload {
    pub root: PathBuf,
}

impl Payload {
    pub fn new(label: &str) -> Self {
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "universe-airlock-verify-{}-{label}-{unique}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create a payload directory");
        Self { root }
    }

    pub fn publish(&self) {
        self.write_manifest(ARTIFACT_NAME, ARTIFACT_SHA256, ARTIFACT.len());
        std::fs::write(self.root.join(ARTIFACT_NAME), ARTIFACT).expect("write the artifact");
    }

    pub fn write_manifest(&self, filename: &str, digest: &str, size: usize) {
        let manifest = format!(
            r#"{{"schema":1,"version":"0.1.0","commit":"{COMMIT}","packages":[
                 {{"name":"universe-airlock","kind":"conda","version":"0.1.0","artifacts":[
                   {{"filename":"{filename}","sha256":"{digest}","size":{size},
                     "stores":["conda-static"]}}]}}]}}"#
        );
        std::fs::write(self.root.join(MANIFEST_FILE), manifest).expect("write the manifest");
    }

    pub fn remove_artifact(&self) {
        std::fs::remove_file(self.root.join(ARTIFACT_NAME)).expect("remove the artifact");
    }

    pub fn corrupt_artifact(&self) {
        std::fs::write(self.root.join(ARTIFACT_NAME), b"the wrong artifact")
            .expect("corrupt the artifact");
    }

    pub fn add_extra_object(&self) {
        std::fs::write(self.root.join("surprise.conda"), b"unlisted").expect("add an extra object");
    }

    pub fn name_a_wrong_platform_object(&self) {
        self.write_manifest(
            "win-64/airlock-0.1.0.conda",
            ARTIFACT_SHA256,
            ARTIFACT.len(),
        );
    }

    pub fn add_signature(&self) {
        std::fs::write(self.root.join(SIGNATURE_FILE), b"untrusted signature bytes")
            .expect("add a detached signature");
    }
}

impl Drop for Payload {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

// ---------------------------------------------------------------------------------------------
// A `.conda` a sandbox can be handed.
// ---------------------------------------------------------------------------------------------

/// One file a fixture package ships.
pub struct Shipped {
    pub path: &'static str,
    pub bytes: Vec<u8>,
    pub mode: u32,
}

/// A file a package installs at `0755`, which is how a package ships a command.
pub fn command(path: &'static str, script: &str) -> Shipped {
    Shipped {
        path,
        bytes: script.as_bytes().to_vec(),
        mode: 0o755,
    }
}

/// A file a package installs at `0644`.
pub fn data(path: &'static str, bytes: &[u8]) -> Shipped {
    Shipped {
        path,
        bytes: bytes.to_vec(),
        mode: 0o644,
    }
}

/// Build a `.conda` named `name` that installs `shipped`.
///
/// The format is written out rather than hidden behind a binary blob: a zip of a
/// zstd-compressed info tar and a zstd-compressed payload tar, with `info/paths.json` recording
/// what each installed file's bytes will be. The digests are computed with `sha2` here, not with
/// the installer's hasher, so a mistake in that hasher cannot agree with itself.
pub fn conda_package(name: &str, depends: &[&str], shipped: &[Shipped]) -> Vec<u8> {
    let index = serde_json::json!({
        "name": name,
        "version": "0.1.0",
        "build": "h1234567_0",
        "build_number": 0,
        "subdir": "linux-64",
        "depends": depends,
    });
    let records: Vec<serde_json::Value> = shipped
        .iter()
        .map(|file| {
            serde_json::json!({
                "_path": file.path,
                "path_type": "hardlink",
                "sha256": digest(&file.bytes),
                "size_in_bytes": file.bytes.len(),
            })
        })
        .collect();
    let paths = serde_json::json!({ "paths": records, "paths_version": 1 });

    let mut info = tar::Builder::new(Vec::new());
    for (member, bytes) in [
        ("info/index.json", index.to_string().into_bytes()),
        ("info/paths.json", paths.to_string().into_bytes()),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_cksum();
        info.append_data(&mut header, member, bytes.as_slice())
            .expect("info entry");
    }

    let mut payload = tar::Builder::new(Vec::new());
    for file in shipped {
        let mut header = tar::Header::new_gnu();
        header.set_size(file.bytes.len() as u64);
        header.set_mode(file.mode);
        header.set_mtime(0);
        header.set_cksum();
        payload
            .append_data(&mut header, file.path, file.bytes.as_slice())
            .expect("payload entry");
    }

    let mut buffer = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut buffer);
    // One fixed timestamp: two calls with the same arguments produce the same bytes, so a test
    // that compares what landed with what it published is comparing content, not the clock.
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(
            zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).expect("a DOS timestamp"),
        );
    zip.start_file("metadata.json", options).expect("metadata");
    zip.write_all(br#"{"conda_pkg_format_version": 2}"#)
        .expect("metadata bytes");
    zip.start_file(format!("info-{name}.tar.zst"), options)
        .expect("info");
    zip.write_all(&compress(&info.into_inner().expect("info tar")))
        .expect("info bytes");
    zip.start_file(format!("pkg-{name}.tar.zst"), options)
        .expect("payload");
    zip.write_all(&compress(&payload.into_inner().expect("payload tar")))
        .expect("payload bytes");
    zip.finish().expect("finish archive");
    buffer.into_inner()
}

fn compress(tar: &[u8]) -> Vec<u8> {
    zstd::encode_all(tar, 0).expect("compress a tar")
}

/// Lowercase SHA-256, computed with `sha2` rather than through the installer's hasher.
pub fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let mut text = String::new();
    for byte in hasher.finalize() {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}
