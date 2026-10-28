use std::ffi::OsString;
use std::process::ExitCode;

pub mod domain;
pub mod i18n;
pub mod infra;

/// 临时入口：slice 7 会换成 `cli` 本地化解析器。
pub fn run(args: impl Iterator<Item = OsString>) -> ExitCode {
    let args: Vec<OsString> = args.collect();

    if args.iter().any(|arg| arg == "--version") {
        println!("lsm {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    if args.iter().any(|arg| arg == "--help") {
        println!("lsm - install and copy agent skills under .agents/skills");
        println!();
        println!("Usage: lsm <command> [options]");
        return ExitCode::SUCCESS;
    }

    ExitCode::SUCCESS
}
