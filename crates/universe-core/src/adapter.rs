//! The in-process boundary between core and an ecosystem adapter.
//!
//! An adapter speaks one ecosystem. Core does not: it owns identity, digests,
//! diagnostics, indexing, publication authority, and generation assembly, and it
//! learns nothing about Conda, Cargo, PyPI, or npm from an adapter that returns
//! these records. Everything crossing this boundary is serializable, so a
//! conformance fixture states what happened in a file rather than in a test
//! binary, and an adapter can be exercised outside the process that hosts it
//! without shipping an adapter protocol.
//!
//! [`Outcome`] is a flat record with a closed field set rather than a tagged
//! union. Serde cannot enforce `deny_unknown_fields` inside an internally tagged
//! enum — the attribute is silently ignored — so a union would accept a misspelt
//! field and lose the value. A struct rejects the unknown field, and
//! [`Outcome::validate`] then enforces which fields belong to which outcome, which
//! is the same rule stated twice: a record says only what it is entitled to say.
//!
//! The three failures are separated because they route differently, and collapsing
//! them is how a retry storm starts. A *user failure* means the input is wrong and
//! no retry will fix it; a *usage failure* means core called the adapter wrongly,
//! which is a defect in this repository rather than in the project being built; an
//! *infrastructure failure* means the environment failed and the same call may
//! succeed later. `DigestMismatch` and `UnknownEvidence` are separate from the
//! failures because performing an operation correctly and being unable to prove
//! what it produced are different answers, and only the first of those is a
//! failure.

use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, ArtifactId};
use crate::diagnostic::{Diagnostic, Severity};
use crate::digest::Sha256Digest;

/// An operation an adapter may expose.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// Describe what a build would do without performing it.
    Plan,
    /// Produce artifacts.
    Build,
    /// Inspect artifacts that already exist.
    Audit,
}

/// What an adapter declares before it is asked to do anything.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    /// Adapter implementation name, as the manifest spells it.
    pub adapter: String,
    /// Operations this adapter implements.
    pub operations: Vec<Operation>,
    /// Platforms this adapter declares support for; empty means platform-agnostic.
    #[serde(default)]
    pub platforms: Vec<String>,
}

impl Capabilities {
    /// Field-level problems with these capabilities, in field order.
    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if self.adapter.trim().is_empty() {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "adapter.capabilities.adapter.empty",
                    "adapter name must not be empty",
                )
                .with_path("adapter".to_owned()),
            );
        }
        if self.operations.is_empty() {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "adapter.capabilities.operations.empty",
                    "an adapter must implement at least one operation",
                )
                .with_path("operations".to_owned()),
            );
        }
        let mut seen: Vec<Operation> = Vec::new();
        for operation in &self.operations {
            if seen.contains(operation) {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "adapter.capabilities.operations.duplicate",
                        format!("operation {operation:?} is declared more than once"),
                    )
                    .with_path("operations".to_owned()),
                );
            } else {
                seen.push(*operation);
            }
        }
        if self
            .platforms
            .iter()
            .any(|platform| platform.trim().is_empty())
        {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "adapter.capabilities.platforms.empty",
                    "a declared platform must not be empty",
                )
                .with_path("platforms".to_owned()),
            );
        }
        diagnostics
    }

    /// Whether this adapter claims the given operation.
    #[must_use]
    pub fn supports(&self, operation: Operation) -> bool {
        self.operations.contains(&operation)
    }
}

/// What every request carries, independent of the operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestContext {
    /// Adapter the request is addressed to.
    pub adapter: String,
    /// Native identity of the package being worked on.
    pub package: ArtifactId,
    /// Platform the work targets, as the manifest spells it.
    pub platform: String,
    /// Workspace-relative directory the command runs in.
    ///
    /// Relative on purpose: an absolute path would carry this host's layout into
    /// a record that other hosts have to read.
    pub workdir: String,
}

impl RequestContext {
    /// Field-level problems with this context, in field order.
    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if self.adapter.trim().is_empty() {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "adapter.request.adapter.empty",
                    "request adapter must not be empty",
                )
                .with_path("context.adapter".to_owned()),
            );
        }
        diagnostics.extend(self.package.validate());
        if self.platform.trim().is_empty() {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "adapter.request.platform.empty",
                    "request platform must not be empty",
                )
                .with_path("context.platform".to_owned()),
            );
        }
        if self.workdir.trim().is_empty() || self.workdir.starts_with('/') {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "adapter.request.workdir.invalid",
                    "request workdir must be a non-empty workspace-relative path",
                )
                .with_path("context.workdir".to_owned()),
            );
        }
        diagnostics
    }
}

/// A request to describe what a build would produce.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanRequest {
    /// Caller-supplied context.
    pub context: RequestContext,
    /// Command the manifest declared, unparsed and unmodified.
    pub command: String,
}

/// A request to produce artifacts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildRequest {
    /// Caller-supplied context.
    pub context: RequestContext,
    /// Command the manifest declared, unparsed and unmodified.
    pub command: String,
}

/// A request to inspect artifacts that already exist.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuditRequest {
    /// Caller-supplied context.
    pub context: RequestContext,
    /// Artifacts the caller expects to exist, with the digests it recorded.
    pub artifacts: Vec<Artifact>,
}

/// An artifact an adapter produced, and where it left the bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProducedArtifact {
    /// Identity, digest, and size of the payload.
    pub artifact: Artifact,
    /// Path to the produced bytes, relative to the request's workdir.
    pub path: String,
}

impl ProducedArtifact {
    /// Field-level problems with this produced artifact, in field order.
    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.artifact.validate();
        if self.path.trim().is_empty() || self.path.starts_with('/') {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "adapter.artifact.path.invalid",
                    "produced artifact path must be a non-empty path relative to the workdir",
                )
                .with_path("path".to_owned()),
            );
        }
        diagnostics
    }
}

/// Which answer an [`Outcome`] carries.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeKind {
    /// The operation completed and produced artifacts.
    Success,
    /// The input is wrong, and repeating the call cannot fix it.
    UserFailure,
    /// Core called the adapter wrongly: a defect in this repository.
    UsageFailure,
    /// The environment failed, and the same call may succeed later.
    InfrastructureFailure,
    /// The bytes do not match the digest the request recorded.
    DigestMismatch,
    /// A fact could not be established, and reporting a failure would be a guess.
    UnknownEvidence,
}

impl OutcomeKind {
    /// Every outcome kind, in declaration order.
    pub const ALL: [Self; 6] = [
        Self::Success,
        Self::UserFailure,
        Self::UsageFailure,
        Self::InfrastructureFailure,
        Self::DigestMismatch,
        Self::UnknownEvidence,
    ];

    /// Whether this kind reports a failure.
    #[must_use]
    pub const fn is_failure(self) -> bool {
        matches!(
            self,
            Self::UserFailure | Self::UsageFailure | Self::InfrastructureFailure
        )
    }
}

/// The two digests a mismatch compares.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DigestMismatch {
    /// Artifact whose bytes were contradictory.
    pub artifact: ArtifactId,
    /// Digest the request expected.
    pub expected: Sha256Digest,
    /// Digest the adapter observed.
    pub found: Sha256Digest,
}

/// What an adapter returns from any operation.
///
/// A flat record with a closed field set: `outcome` says which answer this is, and
/// [`Outcome::validate`] rejects a field the answer is not entitled to carry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    /// Which answer this is.
    pub outcome: OutcomeKind,
    /// Artifacts produced; present only on success.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ProducedArtifact>,
    /// Findings attached to the answer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
    /// Digest comparison; present only on a mismatch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mismatch: Option<DigestMismatch>,
    /// Record the evidence was missing for; optional on unknown evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Outcome {
    /// The kind of answer this is.
    #[must_use]
    pub const fn kind(&self) -> OutcomeKind {
        self.outcome
    }

    /// Whether the operation completed successfully.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self.outcome, OutcomeKind::Success)
    }

    /// Whether this outcome reports a failure.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        self.outcome.is_failure()
    }

    /// Whether repeating the identical call could succeed.
    ///
    /// Only an infrastructure failure qualifies. A user failure is fixed by the
    /// input changing, a usage failure by this repository changing, and the two
    /// evidence outcomes by obtaining evidence — none of which a retry performs.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self.outcome, OutcomeKind::InfrastructureFailure)
    }

    /// Diagnostics attached to this outcome.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Artifacts produced by this outcome.
    #[must_use]
    pub fn artifacts(&self) -> &[ProducedArtifact] {
        &self.artifacts
    }

    /// Field-level problems with this outcome.
    ///
    /// The rules are the ones a conformance fixture asserts, so an adapter that
    /// reports an empty failure, or a success with no artifacts, or a mismatch
    /// whose digests are equal, is rejected here rather than surfacing later as a
    /// silent success.
    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let kind = self.outcome;

        if kind != OutcomeKind::Success && !self.artifacts.is_empty() {
            diagnostics.push(Diagnostic::new(
                Severity::Error,
                "adapter.outcome.artifacts.not_applicable",
                format!("a {kind:?} outcome must not carry artifacts"),
            ));
        }
        if kind != OutcomeKind::DigestMismatch && self.mismatch.is_some() {
            diagnostics.push(Diagnostic::new(
                Severity::Error,
                "adapter.outcome.mismatch.not_applicable",
                format!("a {kind:?} outcome must not carry a digest comparison"),
            ));
        }
        if !matches!(
            kind,
            OutcomeKind::DigestMismatch | OutcomeKind::UnknownEvidence
        ) && self.path.is_some()
        {
            diagnostics.push(Diagnostic::new(
                Severity::Error,
                "adapter.outcome.path.not_applicable",
                format!("a {kind:?} outcome must not name a missing record"),
            ));
        }

        match kind {
            OutcomeKind::Success => {
                if self.artifacts.is_empty() {
                    diagnostics.push(Diagnostic::new(
                        Severity::Error,
                        "adapter.outcome.success.without_artifacts",
                        "a successful outcome must name at least one artifact",
                    ));
                }
                for artifact in &self.artifacts {
                    diagnostics.extend(artifact.validate());
                }
            }
            OutcomeKind::UserFailure
            | OutcomeKind::UsageFailure
            | OutcomeKind::InfrastructureFailure
            | OutcomeKind::UnknownEvidence => {
                if self.diagnostics.is_empty() {
                    diagnostics.push(Diagnostic::new(
                        Severity::Error,
                        "adapter.outcome.without_diagnostics",
                        format!("a {kind:?} outcome must carry at least one diagnostic"),
                    ));
                }
            }
            OutcomeKind::DigestMismatch => match &self.mismatch {
                None => diagnostics.push(Diagnostic::new(
                    Severity::Error,
                    "adapter.outcome.mismatch.missing",
                    "a digest mismatch must name the two digests it compared",
                )),
                Some(mismatch) => {
                    if mismatch.expected == mismatch.found {
                        diagnostics.push(
                            Diagnostic::new(
                                Severity::Error,
                                "adapter.outcome.mismatch.contradiction",
                                "a digest mismatch must name two different digests",
                            )
                            .with_path("mismatch.found".to_owned()),
                        );
                    }
                    diagnostics.extend(mismatch.artifact.validate());
                }
            },
        }
        diagnostics
    }
}

/// One ecosystem, behind a neutral boundary.
///
/// The trait is deliberately synchronous: an adapter that wants concurrency owns
/// it, because nothing here knows whether the ecosystem tolerates parallel requests
/// against one destination.
pub trait Adapter {
    /// What this adapter declares it can do.
    fn capabilities(&self) -> Capabilities;

    /// Describe what a [`BuildRequest`] would produce.
    fn plan(&self, request: &PlanRequest) -> Outcome;

    /// Produce artifacts for a [`BuildRequest`].
    fn build(&self, request: &BuildRequest) -> Outcome;

    /// Inspect artifacts for an [`AuditRequest`].
    fn audit(&self, request: &AuditRequest) -> Outcome;
}

#[cfg(test)]
mod tests {
    use super::{Capabilities, DigestMismatch, Operation, Outcome, OutcomeKind};
    use crate::artifact::{Artifact, ArtifactId};
    use crate::diagnostic::{Diagnostic, Severity};
    use crate::digest::Sha256Digest;

    fn artifact() -> Artifact {
        Artifact {
            id: ArtifactId::new("pkg", "1.0.0").expect("identity"),
            digest: Sha256Digest::parse(&"a".repeat(64)).expect("digest"),
            size_bytes: 12,
            media_type: None,
        }
    }

    fn outcome(kind: OutcomeKind) -> Outcome {
        Outcome {
            outcome: kind,
            artifacts: Vec::new(),
            diagnostics: vec![Diagnostic::new(Severity::Error, "x", "y")],
            mismatch: None,
            path: None,
        }
    }

    #[test]
    fn capabilities_report_their_operations() {
        let capabilities = Capabilities {
            adapter: "example".to_owned(),
            operations: vec![Operation::Plan, Operation::Build],
            platforms: Vec::new(),
        };
        assert!(capabilities.validate().is_empty());
        assert!(capabilities.supports(Operation::Plan));
        assert!(!capabilities.supports(Operation::Audit));
    }

    #[test]
    fn duplicate_operations_are_rejected() {
        let capabilities = Capabilities {
            adapter: "example".to_owned(),
            operations: vec![Operation::Build, Operation::Build],
            platforms: Vec::new(),
        };
        assert!(
            capabilities
                .validate()
                .iter()
                .any(|diagnostic| diagnostic.code() == "adapter.capabilities.operations.duplicate")
        );
    }

    #[test]
    fn a_failure_without_diagnostics_is_rejected() {
        let mut failure = outcome(OutcomeKind::UserFailure);
        failure.diagnostics.clear();
        assert_eq!(
            failure.validate()[0].code(),
            "adapter.outcome.without_diagnostics"
        );
        assert!(failure.is_failure());
        assert!(!failure.is_retryable());
    }

    #[test]
    fn only_infrastructure_failure_is_retryable() {
        assert!(outcome(OutcomeKind::InfrastructureFailure).is_retryable());
        for kind in [
            OutcomeKind::Success,
            OutcomeKind::UserFailure,
            OutcomeKind::UsageFailure,
            OutcomeKind::DigestMismatch,
            OutcomeKind::UnknownEvidence,
        ] {
            assert!(
                !outcome(kind).is_retryable(),
                "{kind:?} must not be retryable"
            );
        }
    }

    #[test]
    fn success_without_artifacts_is_rejected() {
        let mut success = outcome(OutcomeKind::Success);
        success.artifacts.clear();
        assert_eq!(
            success.validate()[0].code(),
            "adapter.outcome.success.without_artifacts"
        );
    }

    #[test]
    fn fields_outside_the_outcome_are_rejected() {
        let mut success = outcome(OutcomeKind::Success);
        success.artifacts.push(super::ProducedArtifact {
            artifact: artifact(),
            path: "output/pkg.tar.zst".to_owned(),
        });
        success.path = Some("packages[0]".to_owned());
        assert_eq!(
            success.validate()[0].code(),
            "adapter.outcome.path.not_applicable"
        );
    }

    #[test]
    fn a_digest_mismatch_against_itself_is_a_contradiction() {
        let mut mismatch = outcome(OutcomeKind::DigestMismatch);
        let digest = artifact().digest;
        mismatch.mismatch = Some(DigestMismatch {
            artifact: artifact().id,
            expected: digest.clone(),
            found: digest,
        });
        assert_eq!(
            mismatch.validate()[0].code(),
            "adapter.outcome.mismatch.contradiction"
        );
    }

    #[test]
    fn a_digest_mismatch_without_its_comparison_is_rejected() {
        let mismatch = outcome(OutcomeKind::DigestMismatch);
        assert_eq!(
            mismatch.validate()[0].code(),
            "adapter.outcome.mismatch.missing"
        );
    }
}
