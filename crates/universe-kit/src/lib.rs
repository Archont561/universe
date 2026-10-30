//! Library surface for the `universe-kit` producer CLI scaffold.

use std::process::ExitCode;
use universe_core::{CommandOutput, Product};

mod ci;

/// Version of this component, equal to the workspace version the whole artifact set
/// is released under (`decision-122`: one tag, one version train).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Execute one Universe Kit command.
///
/// Dispatch is by first token and deliberately tiny: `ci` is the only verb family the
/// producer has so far, and everything else is still the scaffold's version command. When
/// a second family lands this becomes a match on the same token rather than a new parser.
pub fn run<I, S>(args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect();
    if args.first().map(String::as_str) == Some("ci") {
        return ci::run(&args[1..]);
    }
    universe_core::execute_version(Product::Kit, args)
}

/// Execute, print, and return the process code for Universe Kit.
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

    // The `ci` family must not swallow the scaffold command it sits in front of.
    #[test]
    fn a_version_request_still_reaches_the_version_command() {
        let output = super::run(["version"]);
        assert_eq!(output.exit_code(), 0);
        assert!(output.stdout().contains("universe-kit"));
    }
}
