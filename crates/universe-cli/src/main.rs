use std::ffi::OsString;
use std::process::{self, Command};

fn main() {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let (component, forwarded_args) = universe_cli::route(&args);
    // The path is resolved here rather than handed to `Command::new` as a bare name, so a
    // prefix install works when its bin directory is not on `PATH` (GitHub issue 125) and a
    // stray binary of the same name earlier on `PATH` cannot be what runs instead.
    let executable = match universe_cli::resolve(component, &universe_cli::Search::ambient()) {
        Ok(executable) => executable,
        Err(error) => {
            eprintln!("universe: {error}");
            process::exit(127);
        }
    };
    let status = match Command::new(&executable).args(forwarded_args).status() {
        Ok(status) => status,
        Err(error) => {
            eprintln!("universe: cannot run {}: {error}", executable.display());
            process::exit(127);
        }
    };

    process::exit(status.code().unwrap_or(1));
}
