use std::process::ExitCode;

fn main() -> ExitCode {
    lsm::run(std::env::args_os().skip(1))
}
