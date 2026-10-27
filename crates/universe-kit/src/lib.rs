//! Library surface for the `universe-kit` producer CLI scaffold.

use std::process::ExitCode;
use universe_core::{CommandOutput, Product};

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
