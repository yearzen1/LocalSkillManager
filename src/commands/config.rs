//! `config get` / `config set`。

use crate::domain::action::ConfigCommand;
use crate::domain::config::ConfigKey;
use crate::domain::lang::Lang;
use crate::error::{AppError, Msg};
use crate::i18n::MsgKey;
use crate::infra::config;
use crate::output;
use crate::Ctx;

pub fn run(ctx: &Ctx, command: ConfigCommand) -> Result<(), AppError> {
    match command {
        ConfigCommand::Get(key) => get(ctx, key),
        ConfigCommand::Set(key, value) => set(ctx, key, value),
    }
}

/// 输出裸值，方便 `$(lsm config get lang)`。
fn get(ctx: &Ctx, key: ConfigKey) -> Result<(), AppError> {
    match key {
        ConfigKey::Lang => {
            let value = config::load(&ctx.env)?.unwrap_or(Lang::En);
            output::emit_stdout(&format!("{}\n", value.code()));
            Ok(())
        }
    }
}

fn set(ctx: &Ctx, key: ConfigKey, value: Lang) -> Result<(), AppError> {
    match key {
        ConfigKey::Lang => {
            let path = config::set_lang(&ctx.env, value)?;

            // 显式指定语言时沿用该语言；否则用刚写入的值确认。
            let render_lang = if ctx.lang_explicit { ctx.lang } else { value };
            let message = Msg::with_args(
                MsgKey::ConfigSaved,
                [value.code().to_string(), path.display().to_string()],
            );
            output::emit_line(render_lang, &message);
            Ok(())
        }
    }
}
