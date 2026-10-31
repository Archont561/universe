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
///
/// Under GitHub Actions the same findings are also emitted as workflow-command annotations.
/// That is not decoration: a job's log lives behind a signed blob URL, while its annotations
/// are readable through the Checks API — so an annotation is the difference between a failure
/// someone can diagnose and a failure someone can only see the name of.
pub(crate) fn failure(exit_code: u8, diagnostics: &[Diagnostic]) -> CommandOutput {
    let mut stderr = diagnostics
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
    if running_in_actions() {
        stderr.push_str(&annotations(diagnostics));
    }
    CommandOutput::new(String::new(), stderr, exit_code)
}

/// Whether this process is a step in a GitHub Actions job.
fn running_in_actions() -> bool {
    std::env::var("GITHUB_ACTIONS").is_ok_and(|value| value == "true")
}

/// The diagnostics as `::error` workflow commands.
///
/// The escaping mirrors `scripts/print_sandbox_logs.py`: `%` first, then the characters the
/// protocol reserves, so a message cannot inject a second command.
pub(crate) fn annotations(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let mut properties = format!("title={}", escape(diagnostic.code()));
            if let Some(path) = diagnostic.path() {
                properties = format!("file={},{}", escape(path), properties);
            }
            format!("::error {properties}::{}", escape(diagnostic.message()))
        })
        .map(|annotation| format!("{annotation}\n"))
        .collect()
}

fn escape(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace(':', "%3A")
        .replace(',', "%2C")
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

#[cfg(test)]
mod tests {
    use super::{annotations, escape};
    use universe_core::diagnostic::{Diagnostic, Severity};

    #[test]
    fn reserved_characters_are_escaped_before_the_message_is_joined() {
        // `%` first: escaping it later would corrupt the codes it introduces.
        assert_eq!(escape("100%: a,b\n"), "100%25%3A a%2Cb%0A");
    }

    #[test]
    fn a_diagnostic_becomes_one_error_annotation_naming_its_file_and_code() {
        let rendered = annotations(&[Diagnostic::new(
            Severity::Error,
            "build.command.failed",
            "`sh` exited with 3",
        )
        .with_path("packages[example].command")]);
        assert_eq!(
            rendered,
            "::error file=packages[example].command,title=build.command.failed::`sh` exited with 3\n"
        );
    }

    #[test]
    fn a_diagnostic_without_a_path_still_carries_its_code() {
        let rendered = annotations(&[Diagnostic::new(
            Severity::Error,
            "plan.cycle",
            "a -> b -> a",
        )]);
        assert_eq!(rendered, "::error title=plan.cycle::a -> b -> a\n");
    }
}
