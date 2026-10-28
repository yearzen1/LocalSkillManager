//! 唯一的排版出口：成功、警告、信息、错误与列表都在这里成文。

pub mod error;

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

pub(crate) fn arg_refs(msg: &Msg) -> Vec<&str> {
    msg.args.iter().map(String::as_str).collect()
}
