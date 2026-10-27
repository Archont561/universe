use std::process::ExitCode;

fn main() -> ExitCode {
    universe_airlock_bootstrap::run_main(std::env::args().skip(1))
}
