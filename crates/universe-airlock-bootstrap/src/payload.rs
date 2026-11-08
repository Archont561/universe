//! Reading and verifying a local payload, before anything is activated.
//!
//! A payload is a directory a producer published and a target received:
//!
//! ```text
//! universe-release.json          the release manifest, which names every artifact
//! <filename>                     one file per artifact the manifest lists
//! universe-release.json.minisig  optional detached signature over the manifest
//! ```
//!
//! Verification answers four separate questions, deliberately in this order, because the
//! answers have different remedies:
//!
//! 1. is the manifest readable and structurally valid (`airlock.payload.manifest`)?
//! 2. does it describe *this* pinned commit (`airlock.release.commit`)?
//! 3. is every artifact it names present, the right size, and the right digest
//!    (`airlock.payload.missing` / `.size` / `.digest`)?
//! 4. is the payload authentic (`signature::assess`)?
//!
//! Every check is read-only. A caller may run this as often as it likes and nothing on the
//! target has changed; `state::activate` is the only thing that writes, and it verifies again
//! as it copies.

use std::path::{Path, PathBuf};

use universe_core::diagnostic::Severity;
use universe_core::manifest::ReleaseManifest;

use crate::diagnostics::Finding;
use crate::sha256::{hash_file, sha256_hex};
use crate::signature::{self, Lane, Verdict};

/// The manifest file a payload is described by.
pub const MANIFEST_FILE: &str = "universe-release.json";

/// Everything verification established, for a caller that is about to act on it.
#[derive(Clone, Debug)]
pub struct Report {
    /// The manifest, already parsed and validated.
    pub manifest: ReleaseManifest,
    /// SHA-256 of the manifest file's bytes, which is the payload's identity.
    pub manifest_digest: String,
    /// Artifacts the manifest names.
    pub artifacts: usize,
    /// Bytes those artifacts occupy.
    pub bytes: u64,
    /// What the payload's authenticity amounted to.
    pub verdict: Verdict,
    /// Warnings that do not stop the run — an unsigned developer payload is one.
    pub notes: Vec<Finding>,
}

impl Report {
    /// The payload's stable identity, from the manifest's own bytes.
    ///
    /// Derived from content rather than from a clock or a run id, so the same payload restored
    /// twice lands in the same place and a second restore is a no-op instead of a second copy.
    #[must_use]
    pub fn identity(&self) -> String {
        format!(
            "{}-{}",
            self.manifest.version,
            &self.manifest_digest[..16.min(self.manifest_digest.len())]
        )
    }
}

/// One artifact as the manifest describes it, with the path it must live at.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Artifact {
    /// File name as published.
    pub filename: String,
    /// Expected digest, lowercase hexadecimal.
    pub sha256: String,
    /// Expected size in bytes.
    pub size: u64,
}

impl Report {
    /// Every artifact the manifest names, flattened across packages.
    ///
    /// # Errors
    ///
    /// Returns a finding rather than a list when one artifact's name could escape the payload
    /// directory or two artifacts collide on one name, because either makes the set
    /// unaddressable and a restore would write somewhere nobody reviewed.
    pub fn artifacts(&self) -> Result<Vec<Artifact>, Vec<Finding>> {
        let mut artifacts: Vec<Artifact> = Vec::new();
        let mut errors: Vec<Finding> = Vec::new();
        for package in &self.manifest.packages {
            for artifact in &package.artifacts {
                if !is_plain_file_name(&artifact.filename) {
                    errors.push(Finding::error(
                        "airlock.payload.name",
                        format!(
                            "package `{}` names the artifact `{}`, which is not a plain file \
                             name in the payload directory",
                            package.name, artifact.filename
                        ),
                    ));
                    continue;
                }
                let entry = Artifact {
                    filename: artifact.filename.clone(),
                    sha256: artifact.sha256.to_string(),
                    size: artifact.size,
                };
                if let Some(existing) = artifacts
                    .iter()
                    .find(|other| other.filename == entry.filename)
                {
                    if *existing != entry {
                        errors.push(Finding::error(
                            "airlock.payload.duplicate",
                            format!(
                                "two artifacts are published as `{}` with different contents; \
                                 a payload directory cannot hold both",
                                entry.filename
                            ),
                        ));
                    }
                    continue;
                }
                artifacts.push(entry);
            }
        }
        if errors.is_empty() {
            Ok(artifacts)
        } else {
            Err(errors)
        }
    }
}

/// Whether a name is one path segment that stays inside the directory it is joined to.
///
/// A manifest is input from outside the machine, so a name is checked before it is used to
/// build a path rather than after: `../../etc/passwd` is refused as a name, not as a write.
#[must_use]
pub fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
}

/// Verify the payload at `root` for `lane`, optionally against the commit it must describe.
///
/// # Errors
///
/// Returns every problem found, not the first: a payload with three absent artifacts should
/// take one pass to diagnose, not three.
pub fn verify(root: &Path, lane: Lane, pin: Option<&str>) -> Result<Report, Vec<Finding>> {
    let manifest_path = root.join(MANIFEST_FILE);
    let manifest_bytes = match std::fs::read(&manifest_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(vec![Finding::error(
                "airlock.payload.manifest",
                format!(
                    "cannot read the release manifest {}: {error}",
                    manifest_path.display()
                ),
            )]);
        }
    };

    let text = match std::str::from_utf8(&manifest_bytes) {
        Ok(text) => text,
        Err(error) => {
            return Err(vec![Finding::error(
                "airlock.payload.manifest",
                format!("{} is not UTF-8: {error}", manifest_path.display()),
            )]);
        }
    };
    let manifest = match ReleaseManifest::from_json(text) {
        Ok(manifest) => manifest,
        Err(error) => {
            return Err(vec![Finding::error(
                "airlock.payload.manifest",
                format!("{} does not parse: {error}", manifest_path.display()),
            )]);
        }
    };

    let mut errors: Vec<Finding> = manifest
        .validate()
        .into_iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .map(|diagnostic| {
            Finding::error(
                "airlock.release.invalid",
                format!("{}: {}", diagnostic.code(), diagnostic.message()),
            )
        })
        .collect();

    if let Some(pin) = pin
        && manifest.commit != pin
    {
        errors.push(Finding::error(
            "airlock.release.commit",
            format!(
                "the payload was built from commit {} but this restore pins {pin}; the payload \
                 is not the one this checkout was pinned to",
                manifest.commit
            ),
        ));
    }

    let digest = sha256_hex(&manifest_bytes);
    let mut report = Report {
        manifest,
        manifest_digest: digest,
        artifacts: 0,
        bytes: 0,
        verdict: Verdict::Unsigned,
        notes: Vec::new(),
    };

    let artifacts = match report.artifacts() {
        Ok(artifacts) => artifacts,
        Err(name_errors) => {
            errors.extend(name_errors);
            Vec::new()
        }
    };

    for artifact in &artifacts {
        match verify_one(root, artifact) {
            Ok(size) => {
                report.artifacts += 1;
                report.bytes += size;
            }
            Err(finding) => errors.push(finding),
        }
    }

    errors.extend(unexpected_files(root, &artifacts));

    match signature::assess(root, lane) {
        Ok((verdict, notes)) => {
            report.verdict = verdict;
            report.notes = notes;
        }
        Err(signature_errors) => errors.extend(signature_errors),
    }

    if errors.is_empty() {
        Ok(report)
    } else {
        Err(errors)
    }
}

/// Check one artifact's presence, size, and digest.
fn verify_one(root: &Path, artifact: &Artifact) -> Result<u64, Finding> {
    let path = root.join(&artifact.filename);
    let (digest, size) = match hash_file(&path) {
        Ok(measured) => measured,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(Finding::error(
                "airlock.payload.missing",
                format!(
                    "the payload is missing {} ({} bytes, sha256 {}), which the manifest names",
                    artifact.filename, artifact.size, artifact.sha256
                ),
            ));
        }
        Err(error) => {
            return Err(Finding::error(
                "airlock.payload.unreadable",
                format!("cannot read {}: {error}", path.display()),
            ));
        }
    };

    // Size first: it is the cheaper fact to explain, and a truncated download reports it
    // without the reader having to compare two 64-character strings.
    if size != artifact.size {
        return Err(Finding::error(
            "airlock.payload.size",
            format!(
                "{} is {size} bytes but the manifest names {}",
                artifact.filename, artifact.size
            ),
        ));
    }
    if digest != artifact.sha256 {
        return Err(Finding::error(
            "airlock.payload.digest",
            format!(
                "{} hashes to {digest} but the manifest names {}",
                artifact.filename, artifact.sha256
            ),
        ));
    }
    Ok(size)
}

/// Files in the payload that nothing accounts for.
///
/// An extra object is refused rather than ignored: a payload is a closed set, and bytes a
/// manifest does not describe are bytes no verification covered.
fn unexpected_files(root: &Path, artifacts: &[Artifact]) -> Vec<Finding> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        // The directory was readable a moment ago; a failure here is reported by the caller's
        // manifest read rather than guessed at twice.
        Err(_) => return Vec::new(),
    };
    let mut errors = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let accounted = name == MANIFEST_FILE
            || name == signature::SIGNATURE_FILE
            || artifacts.iter().any(|artifact| artifact.filename == name);
        if !accounted {
            errors.push(Finding::error(
                "airlock.payload.extra",
                format!(
                    "the payload holds `{name}`, which the manifest does not describe; a payload \
                     is a closed set"
                ),
            ));
        }
    }
    errors
}

/// Where a payload lives when the caller does not say: the project's own airlock directory.
#[must_use]
pub fn default_root(working_directory: &Path) -> PathBuf {
    working_directory.join(".universe-airlock").join("payload")
}

#[cfg(test)]
mod tests {
    use super::{Report, default_root, is_plain_file_name, verify};
    use crate::sha256::sha256_hex;
    use crate::signature::Lane;
    use crate::testing::Scratch;

    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
    const OTHER_COMMIT: &str = "89abcdef0123456789abcdef0123456789abcdef";

    /// A payload whose single artifact really hashes to what the manifest claims.
    pub(crate) fn payload(label: &str, commit: &str) -> (Scratch, String) {
        let scratch = Scratch::new(label);
        let bytes = b"the only artifact\n";
        let digest = sha256_hex(bytes);
        scratch.write("airlock-0.1.0.conda", bytes);
        let manifest = format!(
            r#"{{
  "schema": 1,
  "version": "0.1.0",
  "commit": "{commit}",
  "packages": [
    {{
      "name": "universe-airlock",
      "kind": "conda",
      "version": "0.1.0",
      "artifacts": [
        {{
          "filename": "airlock-0.1.0.conda",
          "sha256": "{digest}",
          "size": {},
          "stores": ["conda-static"]
        }}
      ]
    }}
  ]
}}
"#,
            bytes.len()
        );
        scratch.write("universe-release.json", manifest.as_bytes());
        (scratch, digest)
    }

    #[test]
    fn a_complete_payload_verifies_and_reports_what_it_covered() {
        let (scratch, digest) = payload("complete", COMMIT);
        let report: Report =
            verify(scratch.path(), Lane::Developer, Some(COMMIT)).expect("verifies");
        assert_eq!(report.artifacts, 1);
        assert_eq!(report.bytes, 18);
        assert_eq!(report.manifest_digest.len(), 64);
        assert_eq!(
            report.identity(),
            format!("0.1.0-{}", &report.manifest_digest[..16])
        );
        assert_eq!(
            report.artifacts(),
            Ok(vec![super::Artifact {
                filename: "airlock-0.1.0.conda".to_owned(),
                sha256: digest,
                size: 18,
            }])
        );
    }

    #[test]
    fn the_identity_follows_the_manifest_bytes_not_the_clock() {
        let (scratch, _) = payload("identity", COMMIT);
        let first = verify(scratch.path(), Lane::Developer, None).expect("verifies");
        let second = verify(scratch.path(), Lane::Developer, None).expect("verifies");
        assert_eq!(first.identity(), second.identity());
    }

    #[test]
    fn a_payload_for_another_commit_is_refused() {
        let (scratch, _) = payload("commit", OTHER_COMMIT);
        let findings = verify(scratch.path(), Lane::Developer, Some(COMMIT)).expect_err("refuses");
        assert_eq!(findings[0].code(), "airlock.release.commit");
        assert!(findings[0].message().contains(OTHER_COMMIT));
    }

    #[test]
    fn a_missing_artifact_is_named_rather_than_summarized() {
        let (scratch, _) = payload("missing", COMMIT);
        std::fs::remove_file(scratch.path().join("airlock-0.1.0.conda")).expect("remove");
        let findings = verify(scratch.path(), Lane::Developer, None).expect_err("refuses");
        assert_eq!(findings[0].code(), "airlock.payload.missing");
        assert!(findings[0].message().contains("airlock-0.1.0.conda"));
    }

    #[test]
    fn a_truncated_artifact_reports_its_size_before_its_digest() {
        // The cheaper fact explains a truncated download; reporting a digest mismatch first
        // would send the reader looking for corruption that is not there.
        let (scratch, _) = payload("truncated", COMMIT);
        scratch.write("airlock-0.1.0.conda", b"short");
        let findings = verify(scratch.path(), Lane::Developer, None).expect_err("refuses");
        assert_eq!(findings[0].code(), "airlock.payload.size");
    }

    #[test]
    fn a_tampered_artifact_of_the_right_size_fails_on_its_digest() {
        let (scratch, _) = payload("tampered", COMMIT);
        scratch.write("airlock-0.1.0.conda", b"the only artifacT\n");
        let findings = verify(scratch.path(), Lane::Developer, None).expect_err("refuses");
        assert_eq!(findings[0].code(), "airlock.payload.digest");
    }

    #[test]
    fn an_object_nothing_describes_is_refused() {
        let (scratch, _) = payload("extra", COMMIT);
        scratch.write("smuggled.conda", b"not in the manifest");
        let findings = verify(scratch.path(), Lane::Developer, None).expect_err("refuses");
        assert_eq!(findings[0].code(), "airlock.payload.extra");
        assert!(findings[0].message().contains("smuggled.conda"));
    }

    #[test]
    fn an_artifact_name_that_could_escape_the_payload_is_refused_before_use() {
        for name in [
            "../../etc/passwd",
            "/etc/passwd",
            "nested/file.conda",
            "..",
            "",
        ] {
            assert!(!is_plain_file_name(name), "{name}");
        }
        for name in [
            "airlock-0.1.0.conda",
            "a.tar.bz2",
            "wheel-1.0-py3-none-any.whl",
        ] {
            assert!(is_plain_file_name(name), "{name}");
        }

        let scratch = Scratch::new("escape");
        let manifest = format!(
            r#"{{"schema":1,"version":"0.1.0","commit":"{COMMIT}","packages":[
                 {{"name":"p","kind":"conda","version":"0.1.0","artifacts":[
                   {{"filename":"../outside.conda","sha256":"{}","size":1,"stores":["conda-static"]}}]}}]}}"#,
            "0".repeat(64)
        );
        scratch.write("universe-release.json", manifest.as_bytes());
        let findings = verify(scratch.path(), Lane::Developer, None).expect_err("refuses");
        assert!(
            findings
                .iter()
                .any(|finding| finding.code() == "airlock.payload.name"),
            "{findings:?}"
        );
    }

    #[test]
    fn an_unreadable_manifest_is_reported_as_the_manifest() {
        let scratch = Scratch::new("no-manifest");
        let findings = verify(scratch.path(), Lane::Developer, None).expect_err("refuses");
        assert_eq!(findings[0].code(), "airlock.payload.manifest");

        scratch.write("universe-release.json", b"not json");
        let findings = verify(scratch.path(), Lane::Developer, None).expect_err("refuses");
        assert_eq!(findings[0].code(), "airlock.payload.manifest");
    }

    #[test]
    fn the_default_location_is_the_projects_airlock_directory() {
        assert_eq!(
            default_root(std::path::Path::new("/project")),
            std::path::Path::new("/project/.universe-airlock/payload")
        );
    }
}
