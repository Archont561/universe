//! Library surface for the thin `universe-airlock-bootstrap` scaffold.

use std::process::ExitCode;
use universe_core::{CommandOutput, Product};

/// Version of this component, equal to the workspace version the whole artifact set
/// is released under (`decision-122`: one tag, one version train).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// This executable's name, which is not the edition's executable name.
///
/// The bootstrapper is a phase of the Airlock edition (`decision-122`), so it reports that
/// edition while naming itself here.
pub const BINARY_NAME: &str = "universe-airlock-bootstrap";

/// Execute the current scaffold command for the Airlock bootstrapper.
pub fn run<I, S>(args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    universe_core::execute_version_as(Product::Airlock, BINARY_NAME, args)
}

/// Execute, print, and return the process code for the Airlock bootstrapper.
pub fn run_main<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    universe_core::run_main_as(Product::Airlock, BINARY_NAME, args)
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
