use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

pub mod cli;
pub mod commands;
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

/// 组合根组装好的运行上下文。
pub struct Ctx {
    /// 进程环境快照。
    pub env: Env,
    /// 本次调用的渲染语言。
    pub lang: Lang,
    /// 本次调用的工作目录，只读一次。
    pub cwd: PathBuf,
    /// 语言是否来自 `--lang` / `LSM_LANG`（而非配置文件或默认值）。
    pub lang_explicit: bool,
}

/// 组合根：读环境 → 定语言 → 解析 → 编排命令 → 输出。
pub fn run(args: impl Iterator<Item = OsString>) -> ExitCode {
    let argv: Vec<OsString> = args.collect();
    let env = Env::read();
    let cli_lang = cli::lang::prescan(&argv);
    let lang_explicit = cli_lang.is_some() || env.lang.is_some();

    let lang = match cli::lang::decide(cli_lang.as_deref(), env.lang.as_deref()) {
        Ok(Decision::Cli(lang) | Decision::Env(lang)) => lang,
        Ok(Decision::NeedFile) => match infra::config::load(&env) {
            Ok(Some(lang)) => lang,
            Ok(None) => Lang::En,
            Err(err) => {
                let app: AppError = err.into();
                return report(&app, Lang::En);
            }
        },
        Err(err) => {
            let app: AppError = err.into();
            return report(&app, Lang::En);
        }
    };

    let ctx = Ctx {
        env,
        lang,
        cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        lang_explicit,
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
        Ok(Outcome::Action(action)) => match commands::dispatch(&ctx, action) {
            Ok(()) => ExitCode::SUCCESS,
            Err(app) => report(&app, ctx.lang),
        },
        Err(app) => report(&app, ctx.lang),
    }
}

fn report(err: &AppError, lang: Lang) -> ExitCode {
    emit(&error_output::render(err, lang), Stream::Stderr);
    ExitCode::from(err.exit_code())
}

fn emit(content: &str, stream: Stream) {
    match stream {
        Stream::Stdout => output::emit_stdout(content),
        Stream::Stderr => output::emit_stderr(content),
    }
}
