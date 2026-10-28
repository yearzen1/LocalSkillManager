//! 唯一的排版出口：成功、警告、信息、错误与列表都在这里成文。

pub mod error;
pub mod list;

use std::io::Write;

use crate::domain::lang::Lang;
use crate::error::Msg;
use crate::i18n::{tr, tr_args, MsgKey};

/// 渲染一条独立消息（无前缀）。
pub fn line(lang: Lang, msg: &Msg) -> String {
    tr_args(lang, msg.key, &arg_refs(msg))
}

/// 渲染带前缀的消息，如 `warning: ...`。
pub fn prefixed_line(lang: Lang, prefix: MsgKey, msg: &Msg) -> String {
    format!("{} {}", tr(lang, prefix), line(lang, msg))
}

/// 把一条消息连同换行写到 stdout。
pub fn emit_line(lang: Lang, msg: &Msg) {
    emit_stdout(&format!("{}\n", line(lang, msg)));
}

/// 警告一律走 stderr，不污染 `--json` 的 stdout。
pub fn warn(lang: Lang, msg: &Msg) {
    emit_stderr(&format!(
        "{}\n",
        prefixed_line(lang, MsgKey::WarningPrefix, msg)
    ));
}

/// 原样写 stdout（用于 `list --json` 与裸值）。
pub fn emit_stdout(text: &str) {
    print!("{text}");
    let _ = std::io::stdout().flush();
}

pub fn emit_stderr(text: &str) {
    eprint!("{text}");
    let _ = std::io::stderr().flush();
}

pub(crate) fn arg_refs(msg: &Msg) -> Vec<&str> {
    msg.args.iter().map(String::as_str).collect()
}
