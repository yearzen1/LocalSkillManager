//! `--lang` 预扫描与三态语言决策表。纯函数，不读环境、不读文件。

use std::ffi::OsString;

use crate::domain::lang::{Lang, LangError};

/// 取最后一个 `--lang` 的原始值，支持 `--lang zh` 与 `--lang=zh`。
pub fn prescan(argv: &[OsString]) -> Option<String> {
    let mut found = None;
    let mut tokens = argv.iter();

    while let Some(token) = tokens.next() {
        let Some(text) = token.to_str() else {
            continue;
        };

        if text == "--lang" {
            if let Some(value) = tokens.next().and_then(|value| value.to_str()) {
                found = Some(value.to_string());
            }
        } else if let Some(value) = text.strip_prefix("--lang=") {
            found = Some(value.to_string());
        }
    }

    found
}

/// 语言来源的三态判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// 命令行给出了合法值。
    Cli(Lang),
    /// 环境变量给出了合法值。
    Env(Lang),
    /// 前两档都没有，需要读配置文件。
    NeedFile,
}

/// `--lang` 优先于 `LSM_LANG`；前两档命中时永不返回 [`Decision::NeedFile`]。
pub fn decide(cli: Option<&str>, env: Option<&str>) -> Result<Decision, LangError> {
    if let Some(raw) = cli {
        return Ok(Decision::Cli(Lang::parse(raw)?));
    }

    if let Some(raw) = env {
        return Ok(Decision::Env(Lang::parse(raw)?));
    }

    Ok(Decision::NeedFile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn prescan_reads_separated_form() {
        assert_eq!(
            prescan(&args(&["list", "--lang", "zh", "global"])),
            Some("zh".to_string())
        );
    }

    #[test]
    fn prescan_reads_inline_form() {
        assert_eq!(
            prescan(&args(&["--lang=zh-CN", "list", "global"])),
            Some("zh-CN".to_string())
        );
    }

    #[test]
    fn prescan_keeps_the_last_occurrence() {
        assert_eq!(
            prescan(&args(&["--lang", "en", "--lang", "zh"])),
            Some("zh".to_string())
        );
    }

    #[test]
    fn prescan_ignores_absent_flag() {
        assert_eq!(prescan(&args(&["list", "global"])), None);
    }

    #[test]
    fn cli_wins_over_env() {
        assert_eq!(decide(Some("zh"), Some("en")), Ok(Decision::Cli(Lang::Zh)));
    }

    #[test]
    fn env_is_used_when_cli_is_absent() {
        assert_eq!(decide(None, Some("zh-CN")), Ok(Decision::Env(Lang::Zh)));
    }

    #[test]
    fn falls_back_to_file_only_when_both_are_absent() {
        assert_eq!(decide(None, None), Ok(Decision::NeedFile));
    }

    #[test]
    fn invalid_cli_value_is_rejected() {
        assert_eq!(
            decide(Some("fr"), Some("zh")),
            Err(LangError::Unsupported("fr".to_string()))
        );
    }

    #[test]
    fn invalid_env_value_is_rejected() {
        assert_eq!(
            decide(None, Some("EN")),
            Err(LangError::Unsupported("EN".to_string()))
        );
    }

    #[test]
    fn never_asks_for_file_when_a_higher_priority_source_hit() {
        for (cli, env) in [
            (Some("en"), None),
            (None, Some("en")),
            (Some("zh"), Some("zh")),
        ] {
            assert_ne!(decide(cli, env).unwrap(), Decision::NeedFile);
        }
    }
}
