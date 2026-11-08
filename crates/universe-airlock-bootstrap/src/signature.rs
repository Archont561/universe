//! Whether a payload is signed, and whether this bootstrap may accept that.
//!
//! `decision-131` fixes the policy: a detached signature over `universe-release.json`,
//! verified against a key **compiled into this binary**, before any target state changes.
//! A missing, malformed, unknown-key, or non-matching signature fails closed, and sealed mode
//! has no unsigned path. The developer lane may proceed unsigned, but only while saying so.
//!
//! What is here is the *policy and the order*; the Ed25519 arithmetic is not. `UNI-55` ships
//! the public key inside the bootstrap artifact and owns key rotation, and the verifier lands
//! with it because a key without a verifier is a claim, and a verifier without a trusted key
//! is a suggestion. Until then there is no key in this binary, so **any** signature is a
//! refusal rather than a pass — the safe direction, and the one that cannot be mistaken for
//! verification.

use std::path::Path;

use crate::diagnostics::Finding;

/// Public keys compiled into this binary.
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

/// Assess the payload's signature for `lane`.
///
/// Returns the verdict and any warning that rides with it, or the errors that stop the run.
/// Nothing here writes to the filesystem: a caller assesses before it stages anything.
pub fn assess(payload_root: &Path, lane: Lane) -> Result<(Verdict, Vec<Finding>), Vec<Finding>> {
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

    // The container is read before anything trusts it, because a malformed signature and a
    // forged one deserve different sentences even though both stop the run.
    let text = std::fs::read_to_string(&path).map_err(|error| {
        vec![Finding::error(
            "airlock.signature.unreadable",
            format!("cannot read {}: {error}", path.display()),
        )]
    })?;
    if !looks_like_a_minisign_container(&text) {
        return Err(vec![Finding::error(
            "airlock.signature.malformed",
            format!(
                "{} is not a minisign detached signature: expected an untrusted comment, a \
                 signature line, a trusted comment, and a global signature",
                path.display()
            ),
        )]);
    }

    if RELEASE_KEYS.is_empty() {
        return Err(vec![Finding::error(
            "airlock.signature.no-key",
            "this bootstrap carries no release verification key, so it cannot verify a signed \
             payload and will not activate one it cannot check (the key ships with UNI-55); \
             re-run on the developer lane only if the payload is your own build",
        )]);
    }

    // Unreachable until `UNI-55` adds a key *and* the Ed25519 verifier. Refusing is the only
    // safe placeholder for a branch that cannot be exercised: an unverified `Ok` here would be
    // a pass this build has not earned.
    Err(vec![Finding::error(
        "airlock.signature.unverified",
        "a verification key is compiled in, but this build carries no Ed25519 verifier",
    )])
}

/// Whether `text` has the four-line shape of a minisign detached signature.
///
/// Shape only: this says the file is the right kind of container, never that the signature is
/// good. It runs before any comparison so that "you sent the wrong file" and "the signature
/// does not match" are not reported as the same thing.
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
    use super::{Lane, Verdict, assess, looks_like_a_minisign_container};
    use crate::testing::Scratch;

    const SIGNATURE: &str = "untrusted comment: signature from minisign secret key\n\
                             RWSabc123\n\
                             trusted comment: universe 0.1.0\n\
                             RWTdef456\n";

    fn payload_with(signature: Option<&str>) -> Scratch {
        let scratch = Scratch::new("signature");
        if let Some(text) = signature {
            scratch.write("universe-release.json.minisig", text.as_bytes());
        }
        scratch
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
        let payload = payload_with(None);
        let findings = assess(payload.path(), Lane::Sealed).expect_err("must refuse");
        assert_eq!(findings[0].code(), "airlock.signature.absent");
        assert!(findings[0].message().contains("nothing was activated"));
    }

    #[test]
    fn a_developer_restore_proceeds_unsigned_but_names_the_missing_signature() {
        let payload = payload_with(None);
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
    fn a_signature_this_build_cannot_check_is_a_refusal_not_a_pass() {
        // The whole of `decision-131` in one assertion: with no compiled-in key, a signed
        // payload fails closed rather than being waved through as "probably fine".
        let payload = payload_with(Some(SIGNATURE));
        for lane in [Lane::Developer, Lane::Sealed] {
            let findings = assess(payload.path(), lane).expect_err("must refuse");
            assert_eq!(findings[0].code(), "airlock.signature.no-key", "{lane:?}");
        }
    }

    #[test]
    fn a_file_that_is_not_a_signature_is_named_as_such() {
        let payload = payload_with(Some("hello\n"));
        let findings = assess(payload.path(), Lane::Sealed).expect_err("must refuse");
        assert_eq!(findings[0].code(), "airlock.signature.malformed");

        assert!(looks_like_a_minisign_container(SIGNATURE));
        assert!(!looks_like_a_minisign_container(
            "untrusted comment: only one line"
        ));
        assert!(!looks_like_a_minisign_container(
            "untrusted comment: x\nsig\ntrusted comment: y\n"
        ));
    }
}
