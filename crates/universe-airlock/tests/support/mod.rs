//! A payload fixture builder for the consumer's verification scenarios.
//!
//! The digest is **not** computed by the code under test: it is the SHA-256 of a fixed byte
//! string, taken from an independent source and written down here. A verifier that agrees
//! with its own hasher proves nothing.
//!
//! Each integration test crate compiles this module separately and uses a different subset of
//! the builder, so a complete fixture library is deliberate rather than dead code.
#![allow(dead_code)]

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
