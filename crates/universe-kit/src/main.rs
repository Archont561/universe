use std::process::ExitCode;

fn main() -> ExitCode {
    universe_kit::run_main(std::env::args().skip(1))
}
