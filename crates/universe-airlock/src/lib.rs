//! Library surface for the `universe-airlock` offline consumer CLI.

use std::process::ExitCode;
use universe_core::{CliError, CommandOutput, Product};

use argp::{EarlyExit, Error, FromArgs};

/// Version of this component, equal to the workspace version the whole artifact set
/// is released under (`decision-122`: one tag, one version train).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The name the reports and usage lines carry.
const BINARY: &str = "universe-airlock";

/// Finite offline consumer for verified sealed Universe snapshots.
///
/// `argp` owns the grammar and `universe-core` owns the report and the diagnostic. Nothing
/// here writes a stream or reads a terminal, so `run` stays a pure function over its
/// arguments and a test can observe it without starting a process. A version request is
/// the default: a consumer with no arguments still reports what it is.
#[derive(FromArgs)]
struct Airlock {
    /// Report the version in the form a human reads.
    #[argp(switch, short = 'V', global)]
    version: bool,

    /// Report the version as a versioned JSON envelope.
    #[argp(switch, global)]
    json: bool,

    #[argp(subcommand)]
    command: Option<Command>,
}

/// The verbs this consumer implements.
#[derive(FromArgs)]
#[argp(subcommand)]
enum Command {
    Version(VersionCommand),
}

/// Report the version this consumer was built from.
#[derive(FromArgs)]
#[argp(subcommand, name = "version")]
struct VersionCommand {}

/// Execute one Airlock command without touching process streams.
pub fn run<I, S>(args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<String> = args
        .into_iter()
        .map(|argument| argument.as_ref().to_owned())
        .collect();

    match Airlock::from_args(&[BINARY], &args) {
        Ok(cli) => {
            // The parsed grammar is forwarded in the core's own vocabulary. That is what
            // keeps the envelope, the one-line report, and the usage error identical to the
            // other editions: the parser decides *what* was asked, the core decides *how* it
            // is written.
            let mut forwarded = Vec::new();
            if cli.version || cli.command.is_some() {
                forwarded.push("version");
            }
            if cli.json {
                forwarded.push("--json");
            }
            universe_core::execute_version(Product::Airlock, forwarded)
        }
        Err(EarlyExit::Help(help)) => {
            CommandOutput::new(help.generate(argp::DEFAULT), String::new(), 0)
        }
        Err(EarlyExit::Err(error)) => {
            let error = match error {
                Error::UnknownArgument(argument) => {
                    CliError::UnknownArgument(argument.to_string_lossy().into_owned())
                }
                other => CliError::UnknownArgument(other.to_string()),
            };
            universe_core::version_usage_output(BINARY, error)
        }
    }
}

/// Execute, print, and return the process code for Universe Airlock.
pub fn run_main<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    universe_core::emit(run(args))
}

#[cfg(test)]
mod tests {
    use super::VERSION;

    // A component that declares its own `version` instead of inheriting the workspace's
    // would split the release train silently, so both sources are asserted here.
    #[test]
    fn version_matches_the_workspace_version() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
        assert_eq!(VERSION, universe_core::VERSION);
    }
}
