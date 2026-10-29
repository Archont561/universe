//! Ecosystem-neutral artifact identity.
//!
//! An artifact is the byte payload a catalogue entry publishes. The neutral
//! vocabulary records only what every ecosystem agrees on — a native name, a
//! native version, the payload digest, and its size. Channel, ecosystem, build
//! string, and trust identity belong to the adapter that produced the artifact,
//! not to this crate.

use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Severity};
use crate::digest::Sha256Digest;

/// The native identity of an artifact inside its own ecosystem.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactId {
    /// Native package name, exactly as its ecosystem spells it.
    pub name: String,
    /// Native version string, unparsed and unnormalised.
    pub version: String,
}

impl ArtifactId {
    /// Build an identity, rejecting empty fields.
    ///
    /// # Errors
    ///
    /// Returns the first empty-field diagnostic when `name` or `version` is
    /// empty or whitespace-only.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Result<Self, Diagnostic> {
        let id = Self {
            name: name.into(),
            version: version.into(),
        };
        match id.validate().into_iter().next() {
            Some(diagnostic) => Err(diagnostic),
            None => Ok(id),
        }
    }

    /// Field-level problems with this identity, in field order.
    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for (field, value) in [("name", &self.name), ("version", &self.version)] {
            if value.trim().is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "artifact.identity.empty",
                        format!("artifact {field} must not be empty"),
                    )
                    .with_path(format!("id.{field}")),
                );
            }
        }
        diagnostics
    }
}

/// A published byte payload and the evidence that identifies it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    /// Native identity of the payload.
    pub id: ArtifactId,
    /// SHA-256 digest of the exact original bytes.
    pub digest: Sha256Digest,
    /// Size of the original bytes.
    pub size_bytes: u64,
    /// Optional media type, when the ecosystem publishes one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

impl Artifact {
    /// Field-level problems with this artifact, in field order.
    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.id.validate();
        if let Some(media_type) = &self.media_type
            && media_type.trim().is_empty()
        {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "artifact.media_type.empty",
                    "artifact media_type must not be empty when present",
                )
                .with_path("media_type".to_owned()),
            );
        }
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::ArtifactId;

    #[test]
    fn rejects_empty_identity_fields() {
        let error = ArtifactId::new("  ", "1.0").expect_err("empty name");
        assert_eq!(error.code(), "artifact.identity.empty");
        assert_eq!(error.path(), Some("id.name"));

        let error = ArtifactId::new("name", "").expect_err("empty version");
        assert_eq!(error.path(), Some("id.version"));

        let id = ArtifactId::new("name", "1.0").expect("valid identity");
        assert!(id.validate().is_empty());
    }
}
