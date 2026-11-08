//! The bootstrap's one diagnostic shape.
//!
//! Every refusal names a stable code as well as a sentence, because the codes are what a
//! consumer's automation can branch on and what a bug report can be searched by. The wording
//! may change; the codes are the contract.

use universe_core::diagnostic::Severity;

/// One thing the bootstrap has to say about the work.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    severity: Severity,
    code: &'static str,
    message: String,
}

impl Finding {
    /// Something that stopped the work.
    #[must_use]
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
        }
    }

    /// Something the work survived, that the operator should still read.
    #[must_use]
    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn severity(&self) -> Severity {
        self.severity
    }

    #[must_use]
    pub fn code(&self) -> &'static str {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// `error: [code] message`, which is how a caller sees it on stderr.
    #[must_use]
    pub fn render(&self) -> String {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "note",
        };
        format!("{level}: [{}] {}\n", self.code, self.message)
    }
}

/// Render a whole set, errors first so the reason a run stopped is the first line read.
#[must_use]
pub fn render_all(findings: &[Finding]) -> String {
    let mut ordered: Vec<&Finding> = findings
        .iter()
        .filter(|finding| finding.severity() == Severity::Error)
        .collect();
    ordered.extend(
        findings
            .iter()
            .filter(|finding| finding.severity() != Severity::Error),
    );
    ordered.iter().map(|finding| finding.render()).collect()
}

#[cfg(test)]
mod tests {
    use super::{Finding, render_all};

    #[test]
    fn errors_are_rendered_before_warnings() {
        let rendered = render_all(&[
            Finding::warning("airlock.thing.warn", "context"),
            Finding::error("airlock.thing.stopped", "it failed"),
        ]);
        assert!(
            rendered.starts_with("error: [airlock.thing.stopped]"),
            "{rendered}"
        );
        assert!(rendered.contains("warning: [airlock.thing.warn] context"));
    }

    #[test]
    fn a_finding_carries_its_code_and_message_separately() {
        let finding = Finding::error("airlock.payload.missing", "an artifact is absent");
        assert_eq!(finding.code(), "airlock.payload.missing");
        assert_eq!(finding.message(), "an artifact is absent");
    }
}
