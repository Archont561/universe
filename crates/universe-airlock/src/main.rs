use std::process::ExitCode;

fn main() -> ExitCode {
    universe_airlock::run_main(std::env::args().skip(1))
}
