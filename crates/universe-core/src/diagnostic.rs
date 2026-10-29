//! Diagnostics and the versioned stdout/result envelope.
//!
//! A command reports what it did through one envelope: a schema version, the
//! command name, a success flag, a typed result, the diagnostics it produced, and
//! the artifacts it touched. Diagnostics carry a stable machine-readable code and
//! an optional field path, so a caller can point at the exact input that failed
//! instead of printing a sentence.

use serde::{Deserialize, Serialize};

use crate::artifact::Artifact;
use crate::schema::SchemaVersion;

/// How much a diagnostic matters to the command that produced it.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The command did not complete the requested work.
    Error,
    /// The work completed, but the input is suspect.
    Warning,
    /// Context that does not change the outcome.
    Info,
}

/// One machine-readable finding about one input.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    severity: Severity,
    code: String,
    message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

impl Diagnostic {
    /// Build a diagnostic from its stable code and human message.
    #[must_use]
    pub fn new(severity: Severity, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            path: None,
        }
    }

    /// Attach the field path this diagnostic refers to.
    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Severity of this diagnostic.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Stable machine-readable code.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Human-readable explanation.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Field path this diagnostic refers to, when it is that specific.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Whether this diagnostic is an error.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error)
    }
}

/// The single machine-readable envelope a command writes to stdout.
///
/// The field order is part of the contract: `schema_version`, `command`, `ok`,
/// `result`, `diagnostics`, `artifacts`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope<T> {
    /// Version of this envelope shape.
    pub schema_version: SchemaVersion,
    /// Command that produced the envelope.
    pub command: String,
    /// Whether the command completed the requested work.
    pub ok: bool,
    /// Command-specific result value.
    pub result: T,
    /// Findings about the inputs.
    pub diagnostics: Vec<Diagnostic>,
    /// Artifacts the command read or produced.
    pub artifacts: Vec<Artifact>,
}

impl<T> Envelope<T> {
    /// Build an envelope around a result value.
    #[must_use]
    pub fn new(
        schema_version: SchemaVersion,
        command: impl Into<String>,
        ok: bool,
        result: T,
        diagnostics: Vec<Diagnostic>,
        artifacts: Vec<Artifact>,
    ) -> Self {
        Self {
            schema_version,
            command: command.into(),
            ok,
            result,
            diagnostics,
            artifacts,
        }
    }

    /// Whether any diagnostic is an error.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

impl<T: Serialize> Envelope<T> {
    /// Serialize the envelope to compact JSON.
    ///
    /// # Errors
    ///
    /// Returns the serializer error when `T` cannot be represented as JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, Envelope, Severity};
    use crate::schema::SchemaVersion;

    #[test]
    fn diagnostics_expose_code_path_and_severity() {
        let diagnostic = Diagnostic::new(Severity::Warning, "catalog.entry.duplicate", "twice")
            .with_path("entries[1]");

        assert_eq!(diagnostic.severity(), Severity::Warning);
        assert_eq!(diagnostic.code(), "catalog.entry.duplicate");
        assert_eq!(diagnostic.message(), "twice");
        assert_eq!(diagnostic.path(), Some("entries[1]"));
        assert!(!diagnostic.is_error());
        assert!(!diagnostic.with_path("x").is_error());
        assert!(Diagnostic::new(Severity::Error, "e", "m").is_error());
    }

    #[test]
    fn envelope_reports_errors_and_serializes_compactly() {
        let clean: Envelope<u8> =
            Envelope::new(SchemaVersion::V1, "check", true, 0, vec![], vec![]);
        assert!(!clean.has_errors());
        assert_eq!(
            clean.to_json().expect("serializes"),
            "{\"schema_version\":1,\"command\":\"check\",\"ok\":true,\"result\":0,\
             \"diagnostics\":[],\"artifacts\":[]}"
        );

        let failed: Envelope<u8> = Envelope::new(
            SchemaVersion::V1,
            "check",
            false,
            0,
            vec![Diagnostic::new(Severity::Error, "e", "m")],
            vec![],
        );
        assert!(failed.has_errors());
    }
}
