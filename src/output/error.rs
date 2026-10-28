//! 错误体渲染：`error:`/`hint:` 与 `错误:`/`提示:` 两套前缀。

use crate::domain::lang::Lang;
use crate::error::AppError;
use crate::i18n::{tr, MsgKey};

use super::line;

/// 渲染为可一次写入 stderr 的文本；hint 缺省时不产生多余空行。
pub fn render(err: &AppError, lang: Lang) -> String {
    let mut text = format!(
        "{} {}\n",
        tr(lang, MsgKey::ErrorPrefix),
        line(lang, &err.msg)
    );

    if let Some(hint) = &err.hint {
        text.push_str(&format!(
            "{} {}\n",
            tr(lang, MsgKey::HintPrefix),
            line(lang, hint)
        ));
    }

    text
}
#[cfg(test)]
mod tests {
    use crate::error::AppError;
    use crate::i18n::MsgKey;

    use super::*;

    #[test]
    fn renders_english_prefixes() {
        let err = AppError::runtime(MsgKey::FsSymlink).with_args(["/tmp/link"]);
        assert_eq!(
            render(&err, Lang::En),
            "error: symbolic links are not supported: /tmp/link\n"
        );
    }

    #[test]
    fn renders_chinese_prefixes() {
        let err = AppError::runtime(MsgKey::FsSymlink).with_args(["/tmp/link"]);
        assert_eq!(render(&err, Lang::Zh), "错误: 不支持符号链接：/tmp/link\n");
    }

    #[test]
    fn renders_hint_on_second_line() {
        let err = AppError::usage(MsgKey::SkillNameInvalid)
            .with_args([".."])
            .with_hint(MsgKey::SkillNameInvalidHint);

        assert_eq!(
            render(&err, Lang::En),
            "error: invalid skill name '..'\nhint: skill names use lowercase letters, digits and single hyphens\n"
        );
    }

    #[test]
    fn omits_hint_line_when_absent() {
        let err = AppError::runtime(MsgKey::SkillDescriptionEmpty);
        let text = render(&err, Lang::En);

        assert_eq!(text.lines().count(), 1);
        assert!(!text.contains("hint:"));
    }
}
