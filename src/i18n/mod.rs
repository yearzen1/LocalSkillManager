//! 用户可见文案的唯一来源。纯函数：不读环境、不读文件、不依赖 `error`。

use crate::domain::lang::Lang;

mod en;
mod zh;

/// 声明消息键，同时生成 [`MsgKey`] 与完整的键清单。
macro_rules! keys {
    ($($key:ident),* $(,)?) => {
        /// 一条用户可见文案的稳定标识。
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum MsgKey {
            $($key),*
        }

        /// 全部键，供覆盖性测试使用；由宏生成，不会漏项。
        #[cfg(test)]
        const ALL: &[MsgKey] = &[$(MsgKey::$key),*];
    };
}

keys!(
    // 前缀与结构标题
    ErrorPrefix,
    HintPrefix,
    WarningPrefix,
    HelpAbout,
    HelpUsage,
    HelpCommands,
    HelpOptions,
    HelpArguments,
    HelpMoreInfo,
    HelpFlagHelp,
    VersionFlagHelp,
    // 通用参数
    LangHelp,
    // 用法错误
    UsageInvalidLanguage,
    UsageInvalidLanguageHint,
    UsageInvalidScope,
    UsageInvalidScopeHint,
    UsageMissingSubcommand,
    UsageMultipleActions,
    UsageMissingActionValue,
    UsageMultipleScopes,
    UsageMissingScope,
    UsageCopyWithScope,
    UsageMixedForms,
    UsageHintForms,
    UsageMissingArgument,
    UsageInvalidValue,
    UsageUnknownArgument,
    UsageUnexpectedArgument,
    UsageInvalidSubcommand,
    UsageInvalidArguments,
    UsageHelpHint,
    // add
    AddAbout,
    AddSourceArg,
    AddScopeArg,
    AddForceFlag,
    AddInstalled,
    AddNestedTarget,
    AddNestedTargetHint,
    AddGitWarning,
    AddUnsupportedSource,
    // list
    ListAbout,
    ListScopeArg,
    ListJsonFlag,
    ListEmpty,
    ListHeaderName,
    ListHeaderDescription,
    // copy
    CopyAbout,
    CopyNameArg,
    CopyForceFlag,
    CopyCopied,
    CopyNotFound,
    CopyNotFoundHint,
    CopySourceInvalid,
    CopySourceInvalidHint,
    CopySamePath,
    CopySamePathHint,
    // config
    ConfigAbout,
    ConfigGetAbout,
    ConfigSetAbout,
    ConfigKeyLangHelp,
    ConfigValueArg,
    ConfigSaved,
    ConfigInvalidValue,
    ConfigParseFailed,
    ConfigWriteFailed,
    ConfigUnknownKey,
    ConfigFixHint,
    // skill 元数据
    SkillAlreadyExists,
    SkillAlreadyExistsHint,
    SkillNameInvalid,
    SkillNameInvalidHint,
    SkillNameTooLong,
    SkillNameReserved,
    SkillDescriptionEmpty,
    SkillDescriptionTooLong,
    SkillFrontmatterMissing,
    SkillFrontmatterMissingHint,
    SkillFrontmatterUtf8,
    SkillFrontmatterYaml,
    SkillFieldMissing,
    SkillDirMismatch,
    SkillDirMismatchHint,
    // 文件系统
    FsIo,
    FsSymlink,
    FsSourceMissing,
    FsSourceType,
    FsBackupLeftover,
    // 路径
    PathHomeUnavailable,
    PathHomeUnavailableHint,
    PathConfigDirUnavailable,
    PathConfigDirUnavailableHint,
);

/// 取一条文案。
pub fn tr(lang: Lang, key: MsgKey) -> &'static str {
    match lang {
        Lang::En => en::text(key),
        Lang::Zh => zh::text(key),
    }
}

/// 取一条文案并替换 `{0}`、`{1}`… 占位符；缺少的参数原样保留占位符。
pub fn tr_args(lang: Lang, key: MsgKey, args: &[&str]) -> String {
    let template = tr(lang, key);
    let mut rendered = String::with_capacity(template.len());

    let mut rest = template;
    while let Some(open) = rest.find('{') {
        rendered.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            rendered.push_str(&rest[open..]);
            return rendered;
        };

        let index = &after_open[..close];
        match index
            .parse::<usize>()
            .ok()
            .and_then(|index| args.get(index))
        {
            Some(value) => rendered.push_str(value),
            None => {
                rendered.push_str(&rest[open..open + close + 2]);
            }
        }
        rest = &after_open[close + 1..];
    }

    rendered.push_str(rest);
    rendered
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn placeholders(template: &str) -> BTreeSet<&str> {
        let mut found = BTreeSet::new();
        let mut rest = template;
        while let Some(open) = rest.find('{') {
            let after_open = &rest[open + 1..];
            let Some(close) = after_open.find('}') else {
                break;
            };
            found.insert(&after_open[..close]);
            rest = &after_open[close + 1..];
        }
        found
    }

    #[test]
    fn every_key_has_text_in_both_languages() {
        for key in ALL {
            assert!(!tr(Lang::En, *key).is_empty(), "missing en text: {key:?}");
            assert!(!tr(Lang::Zh, *key).is_empty(), "missing zh text: {key:?}");
        }
    }

    #[test]
    fn both_languages_use_the_same_placeholders() {
        for key in ALL {
            assert_eq!(
                placeholders(tr(Lang::En, *key)),
                placeholders(tr(Lang::Zh, *key)),
                "placeholder mismatch: {key:?}"
            );
        }
    }

    #[test]
    fn substitutes_positional_arguments() {
        assert_eq!(
            tr_args(Lang::En, MsgKey::AddInstalled, &["my-skill", "/tmp/skills"]),
            "installed my-skill -> /tmp/skills"
        );
        assert_eq!(
            tr_args(Lang::Zh, MsgKey::AddInstalled, &["my-skill", "/tmp/skills"]),
            "已安装 my-skill -> /tmp/skills"
        );
    }

    #[test]
    fn keeps_placeholders_when_arguments_are_missing() {
        assert_eq!(
            tr_args(Lang::En, MsgKey::AddInstalled, &["my-skill"]),
            "installed my-skill -> {1}"
        );
    }

    #[test]
    fn leaves_plain_text_untouched() {
        assert_eq!(tr_args(Lang::En, MsgKey::HelpUsage, &["ignored"]), "Usage:");
    }

    #[test]
    fn language_switch_changes_text() {
        assert_ne!(
            tr(Lang::En, MsgKey::ErrorPrefix),
            tr(Lang::Zh, MsgKey::ErrorPrefix)
        );
        assert_eq!(tr(Lang::En, MsgKey::ErrorPrefix), "error:");
        assert_eq!(tr(Lang::Zh, MsgKey::ErrorPrefix), "错误:");
    }
}
