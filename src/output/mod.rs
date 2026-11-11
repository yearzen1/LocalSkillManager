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

/// 在批量错误块与收尾的总错误之间留一个空行。
pub fn emit_gap() {
    emit_stderr("\n");
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

/// 进度条固定宽度（列）；不探测终端宽度（Q59）。
pub const BAR_WIDTH: usize = 20;

/// 阶段提示：一行裸文本写 stderr，不加 `info:`/`warning:` 前缀。
pub fn status(lang: Lang, msg: &Msg) {
    emit_stderr(&format!("{}\n", line(lang, msg)));
}

/// 进度行：回到行首重绘同一行。
pub fn progress(text: &str) {
    emit_stderr(&format!("\r{text}"));
}

/// 进度行收尾：补一个换行，让后续输出另起一行。
pub fn progress_end() {
    emit_stderr("\n");
}

/// 字节数的人类可读形式：`512 B`、`1.0 KB`、`11.7 MB`、`3.0 GB`。
pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];

    if n < 1024 {
        return format!("{n} B");
    }

    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    format!("{value:.1} {}", UNITS[unit])
}

/// 固定宽度的 ASCII 进度条，含方括号。
pub fn bar(received: u64, total: u64) -> String {
    let filled = (percent(received, total) as usize * BAR_WIDTH) / 100;
    let empty = BAR_WIDTH - filled;

    format!("[{}{}]", "#".repeat(filled), "-".repeat(empty))
}

/// 整数百分比：向下取整并封顶 100；总长为 0 时视为完成。
pub fn percent(received: u64, total: u64) -> u64 {
    if total == 0 {
        return 100;
    }

    received.saturating_mul(100).saturating_div(total).min(100)
}

pub(crate) fn arg_refs(msg: &Msg) -> Vec<&str> {
    msg.args.iter().map(String::as_str).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_byte_counts_with_one_decimal() {
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(999), "999 B");
        assert_eq!(bytes(1024), "1.0 KB");
        assert_eq!(bytes(1024 * 1024), "1.0 MB");
        assert_eq!(bytes(12_300_000), "11.7 MB");
        assert_eq!(bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
    }

    #[test]
    fn draws_a_fixed_width_bar() {
        assert_eq!(bar(0, 10), "[--------------------]");
        assert_eq!(bar(5, 10), "[##########----------]");
        assert_eq!(bar(10, 10), "[####################]");
    }

    #[test]
    fn floors_the_percentage_and_clamps_at_one_hundred() {
        assert_eq!(percent(0, 10), 0);
        assert_eq!(percent(5, 10), 50);
        assert_eq!(percent(11, 10), 100);
        assert_eq!(percent(0, 0), 100);
    }
}
