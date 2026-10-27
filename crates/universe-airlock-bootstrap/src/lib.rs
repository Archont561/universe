//! Library surface for the thin `universe-airlock-bootstrap` scaffold.

use std::process::ExitCode;
use universe_core::{CommandOutput, Product};

/// Execute the current scaffold command for the Airlock bootstrapper.
pub fn run<I, S>(args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    universe_core::execute_version(Product::Bootstrap, args)
}

/// Execute, print, and return the process code for the Airlock bootstrapper.
pub fn run_main<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    universe_core::run_main(Product::Bootstrap, args)
}
