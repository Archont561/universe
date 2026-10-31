//! The `ci` verb family: render the workflow set from a manifest, and prove it has not drifted.
//!
//! This module is the command layer and nothing else. Parsing arguments, reading the
//! manifest, and turning failures into exit codes live here; deciding what a job *is* lives
//! in `universe-workflow`, and the YAML shape lives in the template. Keeping the three apart
//! is what lets the renderer be tested without a process and the CLI be tested without a
//! particular job layout.
//!
//! Two conventions hold across the verbs:
//!
//! - **stdout is data.** Rendered YAML or one JSON envelope, never an error. Diagnostics and
//!   usage go to stderr, so `universe ci render --stdout > file` cannot silently capture an
//!   error message.
//! - **exit codes are a contract**: `0` matched, `1` the repository disagrees with the
//!   manifest, `2` the invocation or the manifest is wrong. A check that reported drift as a
//!   usage error would blame the caller for a repository problem.

use std::path::PathBuf;

use serde_json::json;
use universe_core::diagnostic::{Diagnostic, Envelope, Severity};
use universe_core::{CommandOutput, STDOUT_SCHEMA_VERSION};
use universe_workflow::RenderOutput;

/// Default manifest file name, read from the project root.
const MANIFEST_FILE: &str = "universe.toml";

/// Where rendered files go, relative to the project root.
const OUTPUT_DIR: &str = ".github/workflows";

/// Exit code for a repository that does not match its manifest.
const DRIFT: u8 = 1;

/// Exit code for a bad invocation or an invalid manifest.
const USAGE: u8 = 2;

/// What the caller asked for.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Verb {
    /// Write the rendered set.
    Render,
    /// Compare the rendered set with what is on disk.
    Check,
}

/// Parsed options for one `ci` invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Invocation {
    verb: Verb,
    root: PathBuf,
    print_stdout: bool,
    print_required_checks: bool,
    json: bool,
}

/// Why an invocation could not be interpreted.
#[derive(Clone, Debug, Eq, PartialEq)]
struct UsageError(String);

impl Invocation {
    fn parse(args: &[String]) -> Result<Self, UsageError> {
        let mut invocation = Self {
            verb: Verb::Render,
            root: PathBuf::from("."),
            print_stdout: false,
            print_required_checks: false,
            json: false,
        };
        let mut verb_seen = false;
        let mut rest = args.iter();

        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "render" if !verb_seen => {
                    invocation.verb = Verb::Render;
                    verb_seen = true;
                }
                "check" if !verb_seen => {
                    invocation.verb = Verb::Check;
                    verb_seen = true;
                }
                "--stdout" => invocation.print_stdout = true,
                "--print-required-checks" => invocation.print_required_checks = true,
                "--json" => invocation.json = true,
                "--root" => {
                    let value = rest
                        .next()
                        .ok_or_else(|| UsageError("--root needs a directory".to_owned()))?;
                    invocation.root = PathBuf::from(value);
                }
                other => {
                    return Err(UsageError(format!(
                        "unknown argument: {other}\nusage: universe ci <render|check> [--root <dir>] [--stdout] [--print-required-checks] [--json]"
                    )));
                }
            }
        }

        if !verb_seen {
            return Err(UsageError(
                "usage: universe ci <render|check> [--root <dir>] [--stdout] [--print-required-checks] [--json]"
                    .to_owned(),
            ));
        }
        Ok(invocation)
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }

    fn output_dir(&self) -> PathBuf {
        self.root.join(OUTPUT_DIR)
    }
}

/// Run `universe ci <verb>`.
///
/// # Errors
///
/// Never returns an error: a failure is an exit code plus a diagnostic on stderr, because
/// the caller is a CLI whose contract is its streams.
pub(crate) fn run(args: &[String]) -> CommandOutput {
    match Invocation::parse(args) {
        Ok(invocation) => execute(&invocation),
        Err(UsageError(message)) => failure(
            USAGE,
            vec![Diagnostic::new(
                Severity::Error,
                "ci.usage.invalid",
                message,
            )],
        ),
    }
}

fn execute(invocation: &Invocation) -> CommandOutput {
    let manifest_path = invocation.manifest_path();
    let Ok(text) = std::fs::read_to_string(&manifest_path) else {
        return failure(
            USAGE,
            vec![
                Diagnostic::new(
                    Severity::Error,
                    "ci.manifest.missing",
                    format!("no {} at this root", manifest_path.display()),
                )
                .with_path(manifest_path.display().to_string()),
            ],
        );
    };

    let manifest = match universe_core::manifest::Manifest::from_toml(&text) {
        Ok(manifest) => manifest,
        Err(diagnostics) => return failure(USAGE, diagnostics),
    };

    let rendered = match universe_workflow::render(&manifest, &text) {
        Ok(rendered) => rendered,
        Err(error) => {
            return failure(
                USAGE,
                vec![Diagnostic::new(
                    Severity::Error,
                    "ci.render.failed",
                    error.to_string(),
                )],
            );
        }
    };

    let mut destinations = RenderOutput::store_mapping(&manifest);
    destinations.sort_by(|left, right| left.0.cmp(&right.0));

    match invocation.verb {
        Verb::Render => write(invocation, &rendered, &destinations),
        Verb::Check => check(invocation, &rendered, &destinations),
    }
}

fn write(
    invocation: &Invocation,
    rendered: &RenderOutput,
    destinations: &[(String, Vec<String>)],
) -> CommandOutput {
    if invocation.print_stdout {
        let body: String = rendered
            .files
            .iter()
            .map(|file| file.content.as_str())
            .collect();
        return success(
            "ci render",
            &body,
            rendered,
            destinations,
            invocation.json,
            vec![],
        );
    }

    if invocation.print_required_checks {
        let body = required_checks(rendered);
        return success(
            "ci render",
            &body,
            rendered,
            destinations,
            invocation.json,
            vec![],
        );
    }

    if let Err(error) = universe_workflow::write(&invocation.output_dir(), rendered) {
        return failure(
            USAGE,
            vec![Diagnostic::new(
                Severity::Error,
                "ci.render.write_failed",
                error.to_string(),
            )],
        );
    }
    let written = rendered
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    success(
        "ci render",
        &format!("{written}\n"),
        rendered,
        destinations,
        invocation.json,
        vec![],
    )
}

fn check(
    invocation: &Invocation,
    rendered: &RenderOutput,
    destinations: &[(String, Vec<String>)],
) -> CommandOutput {
    let diagnostics = universe_workflow::check(&invocation.output_dir(), rendered);
    if diagnostics.is_empty() {
        return success(
            "ci check",
            "ok\n",
            rendered,
            destinations,
            invocation.json,
            vec![],
        );
    }
    failure(DRIFT, diagnostics)
}

fn success(
    command: &str,
    body: &str,
    rendered: &RenderOutput,
    destinations: &[(String, Vec<String>)],
    json: bool,
    diagnostics: Vec<Diagnostic>,
) -> CommandOutput {
    if json {
        let envelope: Envelope<serde_json::Value> = Envelope::new(
            STDOUT_SCHEMA_VERSION,
            command,
            true,
            json!({
                "files": rendered.files.iter().map(|file| &file.path).collect::<Vec<_>>(),
                "required_checks": rendered.required_checks,
                // Sorted by store name so the derived view is stable across renders.
                "destinations": destinations
                    .iter()
                    .map(|(store, packages)| json!({ "store": store, "packages": packages }))
                    .collect::<Vec<_>>(),
            }),
            diagnostics,
            Vec::new(),
        );
        return CommandOutput::new(
            format!(
                "{}\n",
                envelope
                    .to_json()
                    .expect("the ci envelope always serializes")
            ),
            String::new(),
            0,
        );
    }
    CommandOutput::new(body.to_owned(), String::new(), 0)
}

/// The required-check list, one per line, as the settings page expects it pasted.
fn required_checks(rendered: &RenderOutput) -> String {
    rendered
        .required_checks
        .iter()
        .map(|check| format!("{check}\n"))
        .collect()
}

fn failure(exit_code: u8, diagnostics: Vec<Diagnostic>) -> CommandOutput {
    crate::output::failure(exit_code, &diagnostics)
}

#[cfg(test)]
mod tests {
    use super::{Invocation, PathBuf, Verb, run};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn a_verb_is_required_and_both_verbs_parse() {
        assert!(Invocation::parse(&args(&[])).is_err());
        assert_eq!(
            Invocation::parse(&args(&["render"])).expect("render").verb,
            Verb::Render
        );
        assert_eq!(
            Invocation::parse(&args(&["check"])).expect("check").verb,
            Verb::Check
        );
    }

    #[test]
    fn options_are_read_in_any_order_and_root_is_honoured() {
        let invocation =
            Invocation::parse(&args(&["--json", "check", "--root", "/tmp/x"])).expect("parses");
        assert_eq!(invocation.verb, Verb::Check);
        assert!(invocation.json);
        assert_eq!(
            invocation.manifest_path(),
            PathBuf::from("/tmp/x/universe.toml")
        );
    }

    #[test]
    fn a_second_verb_is_refused_rather_than_ignored() {
        // `render check` would otherwise render and report nothing about drift.
        let error = Invocation::parse(&args(&["render", "check"])).expect_err("refused");
        assert!(error.0.contains("unknown argument: check"));
    }

    #[test]
    fn root_without_a_value_is_a_usage_error() {
        let error = Invocation::parse(&args(&["render", "--root"])).expect_err("refused");
        assert!(error.0.contains("--root needs a directory"));
    }

    #[test]
    fn an_unknown_argument_lists_the_verbs() {
        let output = run(&args(&["publish"]));
        assert_eq!(output.exit_code(), 2);
        assert!(output.stderr().contains("render|check"));
    }

    #[test]
    fn a_missing_manifest_names_the_path_it_looked_for() {
        let output = run(&args(&["render", "--root", "/nonexistent-project"]));
        assert_eq!(output.exit_code(), 2);
        assert!(
            output
                .stderr()
                .contains("/nonexistent-project/universe.toml")
        );
    }
}
