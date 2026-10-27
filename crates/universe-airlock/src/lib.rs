//! Library surface for the `universe-airlock` offline consumer scaffold.

use std::process::ExitCode;
use universe_core::{CommandOutput, Product};

/// Execute the current scaffold command for Universe Airlock.
pub fn run<I, S>(args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    universe_core::execute_version(Product::Airlock, args)
}

/// Execute, print, and return the process code for Universe Airlock.
pub fn run_main<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    universe_core::run_main(Product::Airlock, args)
}
