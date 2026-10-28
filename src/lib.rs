use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

pub mod cli;
pub mod domain;
pub mod error;
pub mod i18n;
pub mod infra;
pub mod output;

use crate::cli::{lang::Decision, Outcome, Stream};
use crate::domain::lang::Lang;
use crate::error::AppError;
use crate::infra::env::Env;
use crate::output::error as error_output;

/// 组合根：读环境 → 定语言 → 解析 → 输出。`main` 只负责接线。
pub fn run(args: impl Iterator<Item = OsString>) -> ExitCode {
    let argv: Vec<OsString> = args.collect();
    let env = Env::read();

    let lang = match cli::lang::decide(cli::lang::prescan(&argv).as_deref(), env.lang.as_deref()) {
        Ok(Decision::Cli(lang) | Decision::Env(lang)) => lang,
        // 配置文件在下一片接入；暂以默认语言继续。
        Ok(Decision::NeedFile) => Lang::En,
        Err(err) => {
            let app: AppError = err.into();
            return report(&app, Lang::En);
        }
    };

    match cli::parse(lang, argv) {
        Ok(Outcome::Text {
            content,
            stream,
            exit_code,
        }) => {
            emit(&content, stream);
            ExitCode::from(exit_code)
        }
        // 命令在后续切片接入。
        Ok(Outcome::Action(_action)) => ExitCode::SUCCESS,
        Err(app) => report(&app, lang),
    }
}

fn report(err: &AppError, lang: Lang) -> ExitCode {
    emit(&error_output::render(err, lang), Stream::Stderr);
    ExitCode::from(err.exit_code())
}

fn emit(content: &str, stream: Stream) {
    match stream {
        Stream::Stdout => {
            print!("{content}");
            let _ = std::io::stdout().flush();
        }
        Stream::Stderr => {
            eprint!("{content}");
            let _ = std::io::stderr().flush();
        }
    }
}
