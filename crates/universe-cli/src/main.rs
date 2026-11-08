use std::ffi::OsString;
use std::process::{self, Command};

fn main() {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let (component, forwarded_args) = universe_cli::route(&args);
    let status = match Command::new(component).args(forwarded_args).status() {
        Ok(status) => status,
        Err(error) => {
            eprintln!("universe: cannot run {component}: {error}");
            process::exit(127);
        }
    };

    process::exit(status.code().unwrap_or(1));
}
