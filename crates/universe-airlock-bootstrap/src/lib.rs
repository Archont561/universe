//! The thin `universe-airlock-bootstrap` executable.
//!
//! This binary is the only thing a disconnected host runs to establish Universe Airlock: the
//! generated restore entrypoint clones the pinned commit, verifies it, and hands over here with
//! `restore --commit <pin>`. Everything it needs must already be present — no language
//! ecosystem, no registry, no network (`doc-103`).
//!
//! Five verbs, and nothing else:
//!
//! - `version` — the release train this binary belongs to.
//! - `verify`  — read-only: manifest, artifacts, and authenticity, before anything is written.
//! - `restore` — verify, then materialise the payload into a new environment and make it
//!   current in one step. A failure changes nothing (`state`).
//! - `replay`  — install what a mirrored lock pinned into one prefix, in dependency order. This
//!   is the lock path rather than the payload path: no manifest, no commit, no activation.
//! - `doctor`  — read-only: describe what the state root holds and what is inconsistent.
//!
//! What is deliberately absent: adapters, prompts, a TUI, an HTTP server, and anything that
//! publishes. The payload is *read*; producing one is the connected host's job.

pub mod diagnostics;
pub mod hash;
pub mod payload;
pub mod prefix;
pub mod signature;
pub mod state;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use universe_core::{CommandOutput, Product};

use crate::diagnostics::{Finding, render_all};
use crate::signature::Lane;

/// Version of this component, equal to the workspace version the whole artifact set
/// is released under (`decision-122`: one tag, one version train).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// This executable's name, which is not the edition's executable name.
///
/// The bootstrapper is a phase of the Airlock edition (`decision-122`), so it reports that
/// edition while naming itself here.
pub const BINARY_NAME: &str = "universe-airlock-bootstrap";

/// Exit code for a run that could not be completed.
pub const EXIT_REFUSED: u8 = 1;

/// Exit code for a command line this binary does not understand.
pub const EXIT_USAGE: u8 = 2;

const USAGE: &str = "\
usage: universe-airlock-bootstrap <command> [options]

commands:
  version [--json]
  verify  [--payload <dir>] [--lane developer|sealed] [--commit <sha>]
  restore --commit <sha> [--payload <dir>] [--state <dir>] [--lane developer|sealed]
  replay  [--mirror <dir>] [--prefix <dir>]
  doctor  [--payload <dir>] [--state <dir>]

The payload defaults to .universe-airlock/payload and the state root to
.universe-airlock/state, both under the working directory; the restore
entrypoint does not change it, so put the payload in the project you run the
script from. `replay` reads a mirror from .universe-airlock/mirror and installs
into .universe-airlock/prefix when it is not told otherwise.
UNIVERSE_AIRLOCK_STATE overrides the state root.

--lane developer (the default until a signed release exists) allows an unsigned
payload and says so; --lane sealed refuses anything this binary cannot verify.";

/// Execute one command without touching the process' streams.
pub fn run<I, S>(args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let arguments: Vec<String> = args
        .into_iter()
        .map(|argument| argument.as_ref().to_owned())
        .collect();

    let Some(command) = arguments.first() else {
        return usage_error("no command was given");
    };

    match command.as_str() {
        "version" | "--version" | "-V" => {
            universe_core::execute_version_as(Product::Airlock, BINARY_NAME, &arguments)
        }
        "--help" | "-h" | "help" => CommandOutput::new(format!("{USAGE}\n"), String::new(), 0),
        "verify" => run_verify(&arguments[1..]),
        "restore" => run_restore(&arguments[1..]),
        "replay" => run_replay(&arguments[1..]),
        "doctor" => run_doctor(&arguments[1..]),
        other => usage_error(&format!("unknown command: {other}")),
    }
}

/// Execute, print, and return the process code.
pub fn run_main<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let output = run(args);
    print!("{}", output.stdout());
    eprint!("{}", output.stderr());
    ExitCode::from(output.exit_code())
}

fn usage_error(message: &str) -> CommandOutput {
    CommandOutput::new(
        String::new(),
        format!("error: {message}\n{USAGE}\n"),
        EXIT_USAGE,
    )
}

/// The options every verb accepts.
#[derive(Debug)]
struct Options {
    payload: PathBuf,
    state: PathBuf,
    lane: Lane,
    commit: Option<String>,
}

impl Options {
    fn parse(arguments: &[String]) -> Result<Self, CommandOutput> {
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut payload: Option<PathBuf> = None;
        let mut state: Option<PathBuf> = None;
        let mut lane: Option<Lane> = None;
        let mut commit: Option<String> = None;

        let mut rest = arguments.iter();
        while let Some(argument) = rest.next() {
            let mut value = |name: &str| -> Result<String, CommandOutput> {
                rest.next()
                    .cloned()
                    .ok_or_else(|| usage_error(&format!("{name} needs a value")))
            };
            match argument.as_str() {
                "--payload" => payload = Some(PathBuf::from(value("--payload")?)),
                "--state" => state = Some(PathBuf::from(value("--state")?)),
                "--commit" => commit = Some(value("--commit")?),
                "--lane" => {
                    let name = value("--lane")?;
                    lane = Some(Lane::parse(&name).ok_or_else(|| {
                        usage_error(&format!(
                            "unknown lane `{name}`; expected developer or sealed"
                        ))
                    })?);
                }
                other => return Err(usage_error(&format!("unknown argument: {other}"))),
            }
        }

        let environment = std::env::var("UNIVERSE_AIRLOCK_STATE").ok();
        Ok(Self {
            payload: payload.unwrap_or_else(|| payload::default_root(&working_directory)),
            state: state
                .unwrap_or_else(|| state::default_root(&working_directory, environment.as_deref())),
            // The developer lane is the default because no signed release exists yet
            // (`UNI-54`/`UNI-55`); a consumer restore asks for `sealed` and is refused until
            // then, which is the safe direction.
            lane: lane.unwrap_or(Lane::Developer),
            commit,
        })
    }

    /// The state layout these options name.
    fn layout(&self) -> state::Layout {
        state::Layout::new(&self.state)
    }
}

fn run_verify(arguments: &[String]) -> CommandOutput {
    let options = match Options::parse(arguments) {
        Ok(options) => options,
        Err(output) => return output,
    };
    let report = match payload::verify(&options.payload, options.lane, options.commit.as_deref()) {
        Ok(report) => report,
        Err(errors) => return refused(&errors),
    };

    let mut text = String::new();
    text.push_str(&format!("payload: {}\n", options.payload.display()));
    text.push_str(&format!("version: {}\n", report.manifest.version));
    text.push_str(&format!("commit: {}\n", report.manifest.commit));
    text.push_str(&format!(
        "artifacts: {} ({} bytes)\n",
        report.artifacts, report.bytes
    ));
    text.push_str(&format!("manifest: {}\n", report.manifest_digest));
    text.push_str(&format!("signature: {}\n", report.verdict.as_str()));
    text.push_str("verified: nothing was activated by this command\n");
    CommandOutput::new(text, render_all(&report.notes), 0)
}

fn run_restore(arguments: &[String]) -> CommandOutput {
    let options = match Options::parse(arguments) {
        Ok(options) => options,
        Err(output) => return output,
    };
    let Some(commit) = options.commit.as_deref() else {
        return usage_error("restore needs --commit <40-hex>, the pin the entrypoint verified");
    };
    if !universe_core::manifest::is_commit(commit) {
        return usage_error(&format!(
            "`{commit}` is not a full lowercase 40-character commit id"
        ));
    }

    // Verification is separate from activation on purpose: every check that can refuse runs
    // before the first write, so a refusal cannot leave a partially activated target.
    let report = match payload::verify(&options.payload, options.lane, Some(commit)) {
        Ok(report) => report,
        Err(errors) => return refused(&errors),
    };
    let layout = options.layout();
    let outcome = match state::activate(&layout, &options.payload, &report) {
        Ok(outcome) => outcome,
        Err(finding) => return refused(&[finding]),
    };

    let mut text = String::new();
    text.push_str(&format!("restored: {}\n", report.manifest.version));
    text.push_str(&format!("commit: {}\n", report.manifest.commit));
    text.push_str(&format!("environment: {}\n", outcome.env.display()));
    text.push_str(&format!(
        "artifacts: {} ({} bytes){}\n",
        report.artifacts,
        report.bytes,
        if outcome.reused {
            ", already present"
        } else {
            ""
        }
    ));
    text.push_str(&format!("prefix: {}\n", outcome.prefix.display()));
    text.push_str(&format!(
        "installed: {} entries ({} bytes)\n",
        outcome.entries, outcome.bytes
    ));
    text.push_str(&format!("signature: {}\n", report.verdict.as_str()));
    match &outcome.previous {
        Some(previous) => text.push_str(&format!("previous: {}\n", previous.display())),
        None => text.push_str("previous: none\n"),
    }
    CommandOutput::new(text, render_all(&report.notes), 0)
}

/// `replay`: install a mirrored lock into one prefix, with no manifest and no activation.
///
/// The payload path answers "is this release authentic"; this one answers "does this lock's
/// environment exist on this host", which is why it reads a mirror index and writes a prefix
/// rather than a state root. Both go through the same installer, so both get the same digest
/// checks and the same dependency order.
fn run_replay(arguments: &[String]) -> CommandOutput {
    let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (mirror, prefix) = match replay_options(arguments) {
        Ok((mirror, prefix)) => (
            mirror.unwrap_or_else(|| working_directory.join(".universe-airlock/mirror")),
            prefix.unwrap_or_else(|| working_directory.join(".universe-airlock/prefix")),
        ),
        Err(output) => return output,
    };

    let replayed = match prefix::replay_mirror(&mirror, &prefix) {
        Ok(replayed) => replayed,
        Err(finding) => return refused(&[finding]),
    };

    let mut text = String::new();
    text.push_str(&format!("mirror: {}\n", mirror.display()));
    text.push_str(&format!(
        "artifacts: {}{}\n",
        replayed.artifacts,
        if replayed.skipped == 0 {
            String::new()
        } else {
            format!(" ({} left out)", replayed.skipped)
        }
    ));
    text.push_str(&format!("order: {}\n", replayed.order.join(", ")));
    text.push_str(&format!(
        "installed: {} entries ({} bytes)\n",
        replayed.entries, replayed.bytes
    ));
    text.push_str(&format!("prefix: {}\n", prefix.display()));
    CommandOutput::new(text, render_all(&replayed.notes), 0)
}

/// The two paths `replay` accepts, each defaulting to the directory it is conventionally in.
fn replay_options(
    arguments: &[String],
) -> Result<(Option<PathBuf>, Option<PathBuf>), CommandOutput> {
    let mut mirror: Option<PathBuf> = None;
    let mut prefix: Option<PathBuf> = None;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        let mut value = |name: &str| -> Result<String, CommandOutput> {
            rest.next()
                .cloned()
                .ok_or_else(|| usage_error(&format!("{name} needs a value")))
        };
        match argument.as_str() {
            "--mirror" => mirror = Some(PathBuf::from(value("--mirror")?)),
            "--prefix" => prefix = Some(PathBuf::from(value("--prefix")?)),
            other => return Err(usage_error(&format!("unknown argument: {other}"))),
        }
    }
    Ok((mirror, prefix))
}

fn run_doctor(arguments: &[String]) -> CommandOutput {
    let options = match Options::parse(arguments) {
        Ok(options) => options,
        Err(output) => return output,
    };
    let layout = options.layout();
    let status = state::inspect(&layout);

    let mut text = String::new();
    text.push_str(&format!("state: {}\n", layout.root().display()));
    text.push_str(&format!(
        "state root: {}\n",
        if status.root_exists {
            "present"
        } else {
            "absent"
        }
    ));
    match &status.current {
        Some(current) => text.push_str(&format!(
            "current: {} ({})\n",
            current.display(),
            if status.current_exists {
                "present"
            } else {
                "missing"
            }
        )),
        None => text.push_str("current: none\n"),
    }
    if let Some(receipt) = &status.receipt {
        text.push_str(&format!(
            "receipt: version={} commit={} signature={} artifacts={}\n",
            receipt.version,
            receipt.commit,
            receipt.signature,
            receipt.artifacts.len()
        ));
    }

    let mut notes: Vec<Finding> = Vec::new();
    if options.payload.is_dir() {
        match payload::verify(&options.payload, options.lane, None) {
            Ok(report) => text.push_str(&format!(
                "payload: {} verified (version {}, signature {})\n",
                options.payload.display(),
                report.manifest.version,
                report.verdict.as_str()
            )),
            Err(errors) => text.push_str(&format!(
                "payload: {} does not verify ({} problem(s))\n",
                options.payload.display(),
                errors.len()
            )),
        }
    } else {
        text.push_str(&format!("payload: none at {}\n", options.payload.display()));
    }

    let errors = status
        .problems
        .iter()
        .any(|finding| finding.severity() == universe_core::diagnostic::Severity::Error);
    notes.extend(status.problems);
    CommandOutput::new(
        text,
        render_all(&notes),
        if errors { EXIT_REFUSED } else { 0 },
    )
}

/// A refusal: every problem on stderr, no stdout, and the code that means "not done".
fn refused(findings: &[Finding]) -> CommandOutput {
    CommandOutput::new(String::new(), render_all(findings), EXIT_REFUSED)
}

/// Where a payload lives when the caller does not say, re-exported for tests.
#[must_use]
pub fn default_payload_root(working_directory: &Path) -> PathBuf {
    payload::default_root(working_directory)
}

#[cfg(test)]
mod tests {
    use super::{EXIT_REFUSED, EXIT_USAGE, VERSION, run};

    #[test]
    fn version_matches_the_workspace_version() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
        assert_eq!(VERSION, universe_core::VERSION);
    }

    #[test]
    fn the_version_verb_still_speaks_the_core_envelope() {
        let output = run(["version", "--json"]);
        assert_eq!(output.exit_code(), 0);
        assert!(
            output
                .stdout()
                .contains("\"binary\":\"universe-airlock-bootstrap\"")
        );
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn an_unknown_command_is_a_usage_error_naming_it() {
        let output = run(["publish"]);
        assert_eq!(output.exit_code(), EXIT_USAGE);
        assert!(output.stderr().contains("unknown command: publish"));
        assert!(output.stderr().contains("usage:"));
        assert!(output.stdout().is_empty());
    }

    #[test]
    fn restore_without_a_pin_refuses_before_touching_anything() {
        // The entrypoint always passes the pin. A bare `restore` is a caller mistake, and it
        // must not be read as "restore whatever is here".
        let output = run(["restore"]);
        assert_eq!(output.exit_code(), EXIT_USAGE);
        assert!(output.stderr().contains("restore needs --commit"));
    }

    #[test]
    fn restore_refuses_a_pin_that_is_not_a_full_commit() {
        for pin in [
            "HEAD",
            "0123456",
            "0123456789ABCDEF0123456789ABCDEF01234567",
        ] {
            let output = run(["restore", "--commit", pin]);
            assert_eq!(output.exit_code(), EXIT_USAGE, "{pin}");
            assert!(
                output
                    .stderr()
                    .contains("not a full lowercase 40-character commit id"),
                "{pin}: {}",
                output.stderr()
            );
        }
    }

    #[test]
    fn an_unknown_lane_is_refused_rather_than_defaulted() {
        let output = run(["verify", "--lane", "production"]);
        assert_eq!(output.exit_code(), EXIT_USAGE);
        assert!(output.stderr().contains("unknown lane `production`"));
    }

    #[test]
    fn a_missing_payload_is_a_refusal_with_a_code_not_a_panic() {
        let output = run(["verify", "--payload", "/nonexistent/universe-payload"]);
        assert_eq!(output.exit_code(), EXIT_REFUSED);
        assert!(output.stderr().contains("[airlock.payload.manifest]"));
    }

    #[test]
    fn doctor_answers_for_a_state_root_that_does_not_exist() {
        let output = run(["doctor", "--state", "/nonexistent/airlock-state"]);
        assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
        assert!(output.stdout().contains("state root: absent"));
        assert!(output.stdout().contains("current: none"));
    }
}
