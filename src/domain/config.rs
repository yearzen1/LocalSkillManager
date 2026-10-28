//! `config` 命令可操作的配置键。

/// 支持的配置键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigKey {
    Lang,
}

impl ConfigKey {
    /// `config.toml` 中的键名，同时用于 CLI 入参。
    pub fn label(self) -> &'static str {
        match self {
            ConfigKey::Lang => "lang",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_matches_config_key() {
        assert_eq!(ConfigKey::Lang.label(), "lang");
    }
}
