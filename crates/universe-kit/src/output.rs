//! Shared plumbing for the producer verbs.
//!
//! Every verb answers to the same two conventions, and they are kept here so they cannot
//! drift apart: **stdout is data** (an error never lands there, so a redirect cannot capture
//! one), and **exit codes are a contract** — `0` the work completed, `1` the repository
//! disagrees with what was asked, `2` the invocation or the manifest is wrong.

use serde_json::Value;
use universe_core::diagnostic::{Diagnostic, Envelope};
use universe_core::{CommandOutput, STDOUT_SCHEMA_VERSION};

/// Exit code for a repository that does not match what the command asked of it.
pub(crate) const FINDINGS: u8 = 1;

/// Exit code for a bad invocation or an invalid manifest.
pub(crate) const USAGE: u8 = 2;

/// Put every diagnostic on stderr, one line each, and report the failure as an exit code.
pub(crate) fn failure(exit_code: u8, diagnostics: &[Diagnostic]) -> CommandOutput {
    let stderr = diagnostics
        .iter()
        .map(|diagnostic| {
            let path = diagnostic
                .path()
                .map(|path| format!(" [{path}]"))
                .unwrap_or_default();
            format!(
                "error: {}{path}: {}\n",
                diagnostic.code(),
                diagnostic.message()
            )
        })
        .collect::<String>();
    CommandOutput::new(String::new(), stderr, exit_code)
}

/// A successful command whose result is a JSON document.
///
/// The envelope is the workspace's, not this crate's: a consumer reads `schema_version` and
/// `diagnostics` the same way whatever verb produced them.
pub(crate) fn json_success(
    command: &str,
    result: Value,
    diagnostics: Vec<Diagnostic>,
    artifacts: Vec<universe_core::Artifact>,
) -> CommandOutput {
    let envelope = Envelope::new(
        STDOUT_SCHEMA_VERSION,
        command,
        true,
        result,
        diagnostics,
        artifacts,
    );
    CommandOutput::new(
        format!(
            "{}\n",
            envelope.to_json().expect("a JSON result always serializes")
        ),
        String::new(),
        0,
    )
}

/// A successful command whose output is for a person.
pub(crate) fn human(body: String) -> CommandOutput {
    CommandOutput::new(body, String::new(), 0)
}
