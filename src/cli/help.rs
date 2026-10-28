//! help 渲染：把 clap 的结构性标题与结尾提示替换为当前语言。

use crate::domain::lang::Lang;
use crate::i18n::{tr, MsgKey};

/// clap 输出的英文结构片段 → 消息键，按先长后短的顺序替换。
const STRUCTURAL: [(&str, MsgKey); 5] = [
    ("For more information, try '--help'.", MsgKey::HelpMoreInfo),
    ("Arguments:", MsgKey::HelpArguments),
    ("Commands:", MsgKey::HelpCommands),
    ("Options:", MsgKey::HelpOptions),
    ("Usage:", MsgKey::HelpUsage),
];

/// 本地化 clap 生成的 help 或用法错误文本。
pub fn localize(text: &str, lang: Lang) -> String {
    let mut localized = text.to_string();
    for (english, key) in STRUCTURAL {
        localized = localized.replace(english, tr(lang, key));
    }
    localized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_english_text_unchanged() {
        let text = "Usage: lsm <COMMAND>\n\nCommands:\n  add\n\nOptions:\n  -h";
        assert_eq!(localize(text, Lang::En), text);
    }

    #[test]
    fn translates_structural_titles() {
        let text = "Usage: lsm <COMMAND>\n\nCommands:\n  add\n\nOptions:\n  -h\n\nArguments:\n  none\n\nFor more information, try '--help'.";
        let localized = localize(text, Lang::Zh);

        assert!(localized.contains("用法: lsm <COMMAND>"));
        assert!(localized.contains("命令:"));
        assert!(localized.contains("选项:"));
        assert!(localized.contains("参数:"));
        assert!(localized.contains("更多信息请尝试 '--help'。"));
        assert!(!localized.contains("Usage:"));
        assert!(!localized.contains("Commands:"));
    }
}
