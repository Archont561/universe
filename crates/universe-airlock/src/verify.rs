//! Read-only verification of a sealed payload before anything is activated.
//!
//! The integrity rules themselves live in `universe-airlock-bootstrap`'s verification
//! library: one implementation of the checks, shared by the phase that establishes Airlock
//! and the consumer that uses it. This module owns only what the consumer adds — the lane
//! policy, the signature-first order, and the shape of the report a host reads.

use std::path::PathBuf;

use universe_airlock_bootstrap::{
    diagnostics::{self, Finding},
    payload,
    signature::{self, Lane},
};
use universe_core::{
    CommandOutput, STDOUT_SCHEMA_VERSION,
    diagnostic::{Diagnostic, Envelope},
};

/// Exit code for a payload this consumer refuses.
pub const EXIT_REFUSED: u8 = 1;

/// Exit code for a command line that does not describe a lane this build knows.
const EXIT_USAGE: u8 = 2;

const USAGE: &str =
    "usage: universe-airlock verify [--payload <dir>] [--lane developer|sealed] [--json]";

/// One verification report, in the shape scripts read.
#[derive(serde::Serialize)]
struct ResultValue<'a> {
    payload: String,
    version: &'a str,
    commit: &'a str,
    artifacts: usize,
    bytes: u64,
    manifest: &'a str,
    signature: &'a str,
    activated: bool,
}

/// Verify a payload without writing anything.
///
/// Authenticity is assessed **before** the artifacts are read: a payload nobody can
/// authenticate is refused before a single byte is hashed, which is the ordering
/// `decision-131` and `UNI-32` #4 require. `payload::verify` then repeats the assessment
/// as part of its full report; that second look is harmless and keeps the shared verifier
/// the only place the integrity rules are written.
pub fn run(payload: Option<String>, lane: Option<String>, json: bool) -> CommandOutput {
    let lane = match lane {
        Some(text) => match Lane::parse(&text) {
            Some(lane) => lane,
            None => {
                return usage_error(&format!(
                    "unknown lane `{text}`; expected developer or sealed"
                ));
            }
        },
        // No release key is published yet (`UNI-55`), so a sealed payload cannot be
        // authenticated. The developer lane is the usable default and says so loudly.
        None => Lane::Developer,
    };

    let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = payload
        .map(PathBuf::from)
        .unwrap_or_else(|| payload::default_root(&working_directory));

    let (_verdict, notes) = match signature::assess(&root, lane) {
        Ok(assessed) => assessed,
        Err(findings) => return refused(&findings),
    };

    let report = match payload::verify(&root, lane, None) {
        Ok(report) => report,
        Err(findings) => return refused(&findings),
    };

    let result = ResultValue {
        payload: root.display().to_string(),
        version: &report.manifest.version,
        commit: &report.manifest.commit,
        artifacts: report.artifacts,
        bytes: report.bytes,
        manifest: &report.manifest_digest,
        signature: report.verdict.as_str(),
        activated: false,
    };

    let stdout = if json {
        let diagnostics: Vec<Diagnostic> = notes
            .iter()
            .map(|finding| Diagnostic::new(finding.severity(), finding.code(), finding.message()))
            .collect();
        let envelope = Envelope::new(
            STDOUT_SCHEMA_VERSION,
            "verify",
            true,
            result,
            diagnostics,
            Vec::new(),
        );
        format!(
            "{}\n",
            envelope
                .to_json()
                .expect("a verification report serialises")
        )
    } else {
        format!(
            "payload: {}\nversion: {}\ncommit: {}\nartifacts: {} ({} bytes)\nmanifest: {}\nsignature: {}\nverified: nothing was activated by this command\n",
            root.display(),
            report.manifest.version,
            report.manifest.commit,
            report.artifacts,
            report.bytes,
            report.manifest_digest,
            report.verdict.as_str()
        )
    };

    // Warnings are diagnostics, not data: they belong on stderr in both output modes.
    CommandOutput::new(stdout, diagnostics::render_all(&notes), 0)
}

fn refused(findings: &[Finding]) -> CommandOutput {
    // stdout stays empty even in --json mode: a refusal is a diagnostic, not a result.
    CommandOutput::new(
        String::new(),
        diagnostics::render_all(findings),
        EXIT_REFUSED,
    )
}

fn usage_error(message: &str) -> CommandOutput {
    CommandOutput::new(
        String::new(),
        format!("error: {message}\n{USAGE}\n"),
        EXIT_USAGE,
    )
}
