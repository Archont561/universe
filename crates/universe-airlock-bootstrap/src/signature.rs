//! Whether a payload is signed, and whether this bootstrap may accept that.
//!
//! `decision-131` fixes the policy: a detached signature over `universe-release.json`,
//! verified against a key **compiled into this binary**, before any target state changes.
//! A missing, malformed, unknown-key, or non-matching signature fails closed, and sealed mode
//! has no unsigned path. The developer lane may proceed unsigned, but only while saying so.
//!
//! The mechanism is minisign-compatible Ed25519 (`minisign-verify`, which carries no
//! dependencies of its own), chosen in `decision-131` precisely because verification must work
//! on a disconnected host: no CA bundle, no transparency log, no clock.
//!
//! **There is one deliberate hole, and it is the key, not the verifier.** `RELEASE_KEYS` is
//! empty until `UNI-55` publishes the real public key and documents rotation. Everything else
//! is implemented and tested against a fixture vector, so that shipping the key is a constant
//! edit rather than a change of behaviour: with no key, a *signed* payload is refused
//! (`airlock.signature.no-key`) rather than accepted unverified, which is the only failure
//! direction that cannot be mistaken for a pass.

use std::path::Path;

use crate::diagnostics::Finding;
use crate::payload::MANIFEST_FILE;

/// Public keys compiled into this binary, as minisign public-key strings.
///
/// Empty until `UNI-55` ships one. Deliberately a compile-time constant and never a file or a
/// URL: a key fetched at verification time proves only that whoever wrote the payload could
/// also write the key.
pub const RELEASE_KEYS: &[&str] = &[];

/// Where the detached signature lives, beside the manifest it covers.
pub const SIGNATURE_FILE: &str = "universe-release.json.minisig";

/// Which lane a restore belongs to.
///
/// They are not two names for the same caution: a consumer's disconnected host has nothing but
/// this payload, while a developer's host is already builder-controlled. Only the second may
/// accept bytes nobody signed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lane {
    /// A builder's machine: unsigned is allowed, with a diagnostic that names it.
    Developer,
    /// A consumer's machine: nothing unsigned is activated.
    Sealed,
}

impl Lane {
    /// Parse the value a caller passed to `--lane`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "developer" => Some(Self::Developer),
            "sealed" => Some(Self::Sealed),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Developer => "developer",
            Self::Sealed => "sealed",
        }
    }
}

/// What the payload's authenticity amounted to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verdict {
    /// A signature was present and checked against a key in [`RELEASE_KEYS`].
    Verified,
    /// No signature was present, and the lane allowed that.
    Unsigned,
}

impl Verdict {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unsigned => "unsigned",
        }
    }
}

/// Assess the payload's signature for `lane`, against the keys this build carries.
///
/// Nothing here writes to the filesystem: a caller assesses before it stages anything.
///
/// # Errors
///
/// Returns the finding that stopped the run, never a bare "invalid": the codes distinguish an
/// absent signature from one that is malformed, unknown-keyed, or wrong.
pub fn assess(payload_root: &Path, lane: Lane) -> Result<(Verdict, Vec<Finding>), Vec<Finding>> {
    assess_with_keys(payload_root, lane, RELEASE_KEYS)
}

/// [`assess`], against an explicit key list.
///
/// Separate so the verifier can be exercised against a fixture vector while the real key is
/// still outstanding; the public entry point has no way to replace the compiled-in set.
fn assess_with_keys(
    payload_root: &Path,
    lane: Lane,
    keys: &[&str],
) -> Result<(Verdict, Vec<Finding>), Vec<Finding>> {
    let path = payload_root.join(SIGNATURE_FILE);
    if !path.exists() {
        return match lane {
            Lane::Sealed => Err(vec![Finding::error(
                "airlock.signature.absent",
                format!(
                    "sealed restore requires a detached signature over universe-release.json \
                     and the payload has none ({}); nothing was activated",
                    path.display()
                ),
            )]),
            Lane::Developer => Ok((
                Verdict::Unsigned,
                vec![Finding::warning(
                    "airlock.signature.absent",
                    "developer lane: this payload carries no signature, so nothing proves who \
                     wrote it; sealed mode refuses this payload",
                )],
            )),
        };
    }

    if keys.is_empty() {
        // Checked before parsing: the useful sentence names the missing key, and a malformed
        // signature would otherwise be reported as the reason a payload this build cannot
        // check was refused.
        return Err(vec![Finding::error(
            "airlock.signature.no-key",
            "this bootstrap carries no release verification key, so it cannot verify a signed \
             payload and will not activate one it cannot check (the key ships with UNI-55); \
             re-run on the developer lane only if the payload is your own build",
        )]);
    }

    let signature = match minisign_verify::Signature::from_file(&path) {
        Ok(signature) => signature,
        Err(error) => {
            return Err(vec![Finding::error(
                "airlock.signature.malformed",
                format!(
                    "{} does not decode as a minisign detached signature: {error:?}",
                    path.display()
                ),
            )]);
        }
    };

    let manifest = payload_root.join(MANIFEST_FILE);
    let bytes = std::fs::read(&manifest).map_err(|error| {
        vec![Finding::error(
            "airlock.payload.manifest",
            format!(
                "cannot read {} for verification: {error}",
                manifest.display()
            ),
        )]
    })?;

    // The signature covers the manifest, which names the digests of everything else, so one
    // check of this file transitively covers the payload (`decision-131` §1).
    let mut refused: Option<Finding> = None;
    for key in keys {
        let public_key = match minisign_verify::PublicKey::from_base64(key) {
            Ok(public_key) => public_key,
            Err(error) => {
                refused = Some(Finding::error(
                    "airlock.signature.key",
                    format!("the compiled-in key does not decode: {error:?}"),
                ));
                continue;
            }
        };
        match public_key.verify(&bytes, &signature, false) {
            Ok(()) => return Ok((Verdict::Verified, Vec::new())),
            Err(minisign_verify::Error::UnexpectedAlgorithm) => {
                // A legacy minisign signature covers the file itself instead of its BLAKE2b
                // prehash. Refused, and said as refusals go: calling it a mismatch would
                // blame tampering for what is only an old signing mode.
                refused = Some(Finding::error(
                    "airlock.signature.algorithm",
                    format!(
                        "{} is a legacy-mode minisign signature (the kind that signs the file \
                         rather than its prehash); this bootstrap accepts only modern, \
                         prehashed signatures — re-sign the payload with a current minisign",
                        path.display()
                    ),
                ));
            }
            Err(minisign_verify::Error::UnexpectedKeyId) => {
                // Not this key's signature. Another key in the set may still match, which is
                // what makes a rotation (old key and new key both compiled in) work.
                refused = Some(Finding::error(
                    "airlock.signature.unknown-key",
                    format!(
                        "{} was signed by a key this bootstrap does not carry; rotation \
                         publishes a bootstrap signed by both keys (decision-131 §4)",
                        path.display()
                    ),
                ));
            }
            Err(error) => {
                refused = Some(Finding::error(
                    "airlock.signature.mismatch",
                    format!(
                        "the signature on {} does not match its contents ({error:?}); the \
                         manifest was altered after it was signed",
                        manifest.display()
                    ),
                ));
            }
        }
    }

    Err(vec![refused.unwrap_or_else(|| {
        Finding::error(
            "airlock.signature.no-key",
            "no compiled-in key could be tried",
        )
    })])
}

/// Whether `text` has the four-line shape of a minisign detached signature.
///
/// Shape only: this says the file is the right kind of container, never that the signature is
/// good. Kept beside the verifier because the two answer different questions and a caller may
/// want to say "that is not even a signature file" without invoking crypto.
#[must_use]
pub fn looks_like_a_minisign_container(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    if lines.len() < 4 {
        return false;
    }
    lines[0].starts_with("untrusted comment:")
        && !lines[1].trim().is_empty()
        && lines[2].starts_with("trusted comment:")
        && !lines[3].trim().is_empty()
}
#[cfg(test)]
mod tests {
    use super::{Lane, Verdict, assess, assess_with_keys, looks_like_a_minisign_container};
    use std::path::Path;
    use tempfile::TempDir;

    /// A minisign key and a prehashed (`ED`) signature over the bytes `test`.
    ///
    /// `tests/fixtures/minisign/README.md` records where these come from: throwaway key
    /// pairs, signed with an implementation independent of this crate, so the vector is
    /// genuine rather than something this repository could agree with by being wrong in the
    /// same way. It exercises the wiring — key decoding, key-id matching, the prehash, and
    /// the global signature over the trusted comment.
    const FIXTURE_KEY: &str = include_str!("../tests/fixtures/minisign/release-key.pub");
    const FIXTURE_SIGNATURE: &str =
        include_str!("../tests/fixtures/minisign/universe-release.json.minisig");
    /// A second, unrelated key: its id matches nothing the fixture signature names, which is
    /// the case a rotation set has to tolerate.
    const OTHER_KEY: &str = include_str!("../tests/fixtures/minisign/rotation-key.pub");
    /// The same key in legacy mode: it signs the file itself rather than its prehash.
    const LEGACY_SIGNATURE: &str =
        include_str!("../tests/fixtures/minisign/legacy-universe-release.json.minisig");

    fn payload_with(manifest: &[u8], signature: Option<&str>) -> TempDir {
        let directory = TempDir::new().expect("a temporary payload");
        std::fs::write(directory.path().join("universe-release.json"), manifest).expect("manifest");
        if let Some(text) = signature {
            std::fs::write(directory.path().join("universe-release.json.minisig"), text)
                .expect("signature");
        }
        directory
    }

    #[test]
    fn the_lane_parses_only_its_two_names() {
        assert_eq!(Lane::parse("developer"), Some(Lane::Developer));
        assert_eq!(Lane::parse("sealed"), Some(Lane::Sealed));
        assert_eq!(Lane::parse("production"), None);
        assert_eq!(Lane::Developer.as_str(), "developer");
    }

    #[test]
    fn a_consumer_restore_refuses_an_unsigned_payload() {
        let payload = payload_with(b"{}", None);
        let findings = assess(payload.path(), Lane::Sealed).expect_err("must refuse");
        assert_eq!(findings[0].code(), "airlock.signature.absent");
        assert!(findings[0].message().contains("nothing was activated"));
    }

    #[test]
    fn a_developer_restore_proceeds_unsigned_but_names_the_missing_signature() {
        let payload = payload_with(b"{}", None);
        let (verdict, warnings) =
            assess(payload.path(), Lane::Developer).expect("allowed on this lane");
        assert_eq!(verdict, Verdict::Unsigned);
        assert_eq!(warnings[0].code(), "airlock.signature.absent");
        assert!(
            warnings[0]
                .message()
                .contains("nothing proves who wrote it")
        );
    }

    #[test]
    fn a_signed_payload_this_build_has_no_key_for_is_refused_on_both_lanes() {
        // The whole of `decision-131` in one assertion, and the state of this build until
        // `UNI-55` ships the key: present-but-unverifiable fails closed, never passes quietly.
        let payload = payload_with(b"test", Some(FIXTURE_SIGNATURE));
        for lane in [Lane::Developer, Lane::Sealed] {
            let findings = assess(payload.path(), lane).expect_err("must refuse");
            assert_eq!(findings[0].code(), "airlock.signature.no-key", "{lane:?}");
        }
    }

    #[test]
    fn a_fixture_signature_verifies_against_its_key() {
        // The verifier itself, exercised against a real signed vector. With no key compiled in
        // this branch is unreachable from the CLI, so it is tested here instead of being
        // shipped untried.
        let payload = payload_with(b"test", Some(FIXTURE_SIGNATURE));
        let (verdict, notes) =
            assess_with_keys(payload.path(), Lane::Sealed, &[FIXTURE_KEY]).expect("verifies");
        assert_eq!(verdict, Verdict::Verified);
        assert!(notes.is_empty());
    }

    #[test]
    fn a_signature_that_does_not_match_its_contents_is_named_as_a_mismatch() {
        // Same key, same signature, different bytes: the case a rewritten manifest produces.
        let payload = payload_with(b"tampered", Some(FIXTURE_SIGNATURE));
        let findings = assess_with_keys(payload.path(), Lane::Sealed, &[FIXTURE_KEY])
            .expect_err("must refuse");
        assert_eq!(findings[0].code(), "airlock.signature.mismatch");
    }

    #[test]
    fn a_legacy_signature_is_named_as_an_old_signing_mode_not_as_tampering() {
        // The key id matches, so this is not a key question: the payload really was signed by
        // a key we carry, just in the mode that predates prehashing. Reporting it as a
        // mismatch would send someone hunting for an alteration that never happened.
        let payload = payload_with(b"test", Some(LEGACY_SIGNATURE));
        let findings = assess_with_keys(payload.path(), Lane::Sealed, &[FIXTURE_KEY])
            .expect_err("must refuse");
        assert_eq!(findings[0].code(), "airlock.signature.algorithm");
        assert!(
            findings[0].message().contains("prehashed"),
            "{}",
            findings[0].message()
        );
    }

    #[test]
    fn a_signature_from_an_unknown_key_is_named_as_an_unknown_key() {
        let payload = payload_with(b"test", Some(FIXTURE_SIGNATURE));
        let findings =
            assess_with_keys(payload.path(), Lane::Sealed, &[OTHER_KEY]).expect_err("must refuse");
        assert_eq!(findings[0].code(), "airlock.signature.unknown-key");
        assert!(
            findings[0].message().contains("rotation"),
            "{}",
            findings[0].message()
        );
    }

    #[test]
    fn a_rotation_set_accepts_a_signature_from_any_of_its_keys() {
        // `decision-131` §4: rotation publishes a bootstrap carrying both the old key and the
        // new one. An unrelated key beside the matching one must not break verification.
        let payload = payload_with(b"test", Some(FIXTURE_SIGNATURE));
        let (verdict, _) =
            assess_with_keys(payload.path(), Lane::Sealed, &[OTHER_KEY, FIXTURE_KEY])
                .expect("one of the keys matches");
        assert_eq!(verdict, Verdict::Verified);
    }

    #[test]
    fn a_file_that_is_not_a_signature_is_named_as_such() {
        let payload = payload_with(b"test", Some("hello\n"));
        let findings = assess_with_keys(payload.path(), Lane::Sealed, &[FIXTURE_KEY])
            .expect_err("must refuse");
        assert_eq!(findings[0].code(), "airlock.signature.malformed");

        assert!(looks_like_a_minisign_container(FIXTURE_SIGNATURE));
        assert!(!looks_like_a_minisign_container(
            "untrusted comment: only one line"
        ));
        assert!(!looks_like_a_minisign_container(
            "untrusted comment: x\nsig\ntrusted comment: y\n"
        ));
    }

    #[test]
    fn the_public_entry_point_carries_no_key_until_uni_55() {
        // A guard on the hole rather than on the behaviour: when the key lands, this test
        // fails and whoever ships it has to say so in the same commit.
        assert!(
            super::RELEASE_KEYS.is_empty(),
            "a release key is compiled in: update this test and record the rotation procedure"
        );
        let payload = payload_with(b"test", Some(FIXTURE_SIGNATURE));
        assert!(assess(payload.path(), Lane::Sealed).is_err());
        assert!(Path::new("universe-release.json.minisig").is_relative());
    }
}
