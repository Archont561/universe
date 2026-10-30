//! Library surface for the `universe-kit` producer CLI scaffold.

use std::process::ExitCode;
use universe_core::{CommandOutput, Product};

/// Version of this component, equal to the workspace version the whole artifact set
/// is released under (`decision-122`: one tag, one version train).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Execute the current scaffold command for Universe Kit.
pub fn run<I, S>(args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    universe_core::execute_version(Product::Kit, args)
}

/// Execute, print, and return the process code for Universe Kit.
pub fn run_main<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    universe_core::run_main(Product::Kit, args)
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
