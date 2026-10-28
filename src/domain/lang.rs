//! 语言枚举与归一化，是 `--lang` / `LSM_LANG` / 配置文件的唯一归一处。

/// 支持的语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    Zh,
}

/// 语言标签无法识别。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LangError {
    /// 保留原值，供上层拼提示。
    Unsupported(String),
}

impl Lang {
    /// 严格解析：只接受 `en`、`zh`、`en-US`、`zh-CN`，其余（含大小写变体）一律拒绝。
    pub fn parse(input: &str) -> Result<Self, LangError> {
        match input {
            "en" | "en-US" => Ok(Lang::En),
            "zh" | "zh-CN" => Ok(Lang::Zh),
            other => Err(LangError::Unsupported(other.to_string())),
        }
    }

    /// 落盘与脚本消费用的稳定代码。
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Zh => "zh",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_english_and_chinese_tags() {
        assert_eq!(Lang::parse("en"), Ok(Lang::En));
        assert_eq!(Lang::parse("zh"), Ok(Lang::Zh));
        assert_eq!(Lang::parse("en-US"), Ok(Lang::En));
        assert_eq!(Lang::parse("zh-CN"), Ok(Lang::Zh));
    }

    #[test]
    fn rejects_unsupported_tags() {
        assert_eq!(
            Lang::parse("fr"),
            Err(LangError::Unsupported("fr".to_string()))
        );
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(Lang::parse(""), Err(LangError::Unsupported(String::new())));
    }

    #[test]
    fn rejects_case_variants() {
        assert_eq!(
            Lang::parse("EN"),
            Err(LangError::Unsupported("EN".to_string()))
        );
        assert_eq!(
            Lang::parse("zh-cn"),
            Err(LangError::Unsupported("zh-cn".to_string()))
        );
    }

    #[test]
    fn code_matches_config_value() {
        assert_eq!(Lang::En.code(), "en");
        assert_eq!(Lang::Zh.code(), "zh");
    }
}
