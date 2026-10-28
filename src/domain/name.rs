//! skill 名称校验：目录名即 skill 身份。

/// `name` 的字符上限（按 Unicode 字符计）。
pub const MAX_NAME_LEN: usize = 64;

/// 已校验的 skill 名称。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SkillName(String);

/// skill 名称校验失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    /// 格式不符：只允许小写字母、数字与单个连字符，且首尾必须是字母或数字。
    Format { input: String },
    /// 超过 [`MAX_NAME_LEN`] 个字符。
    TooLong { input: String, len: usize },
    /// 与 Windows 保留设备名冲突。
    Reserved { input: String },
}

impl SkillName {
    /// 解析并完成全部校验（格式、长度、Windows 保留名）。
    pub fn parse(input: &str) -> Result<Self, NameError> {
        let len = input.chars().count();
        if len > MAX_NAME_LEN {
            return Err(NameError::TooLong {
                input: input.to_string(),
                len,
            });
        }

        if !has_valid_format(input) {
            return Err(NameError::Format {
                input: input.to_string(),
            });
        }

        if is_windows_reserved(input) {
            return Err(NameError::Reserved {
                input: input.to_string(),
            });
        }

        Ok(SkillName(input.to_string()))
    }

    /// 已校验的原始值。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 小写字母/数字开头结尾，连字符必须单个且不落在首尾。
fn has_valid_format(input: &str) -> bool {
    if input.is_empty() {
        return false;
    }

    let mut previous_is_hyphen = true;
    for ch in input.chars() {
        match ch {
            '-' if previous_is_hyphen => return false,
            '-' => previous_is_hyphen = true,
            _ if ch.is_ascii_lowercase() || ch.is_ascii_digit() => previous_is_hyphen = false,
            _ => return false,
        }
    }

    !previous_is_hyphen
}

/// `con`/`prn`/`aux`/`nul` 与 `com1`~`com9`、`lpt1`~`lpt9`。
fn is_windows_reserved(input: &str) -> bool {
    if matches!(input, "con" | "prn" | "aux" | "nul") {
        return true;
    }

    ["com", "lpt"].iter().any(|prefix| {
        input.strip_prefix(prefix).is_some_and(|suffix| {
            let mut chars = suffix.chars();
            matches!((chars.next(), chars.next()), (Some(digit), None) if digit.is_ascii_digit() && digit != '0')
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_lowercase_alphanumeric_with_single_hyphens() {
        for input in ["a", "a1", "my-skill", "code-review-2"] {
            let name = SkillName::parse(input).expect("should be valid");
            assert_eq!(name.as_str(), input);
        }
    }

    #[test]
    fn rejects_uppercase_and_invalid_characters() {
        for input in [
            "My-Skill", "a_", "a b", "中文", "a.b", "a/b", "a\\b", "..", ".",
        ] {
            assert_eq!(
                SkillName::parse(input),
                Err(NameError::Format {
                    input: input.to_string()
                })
            );
        }
    }

    #[test]
    fn rejects_leading_trailing_or_doubled_hyphens() {
        for input in ["-a", "a-", "a--b", "-"] {
            assert_eq!(
                SkillName::parse(input),
                Err(NameError::Format {
                    input: input.to_string()
                })
            );
        }
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(
            SkillName::parse(""),
            Err(NameError::Format {
                input: String::new()
            })
        );
    }

    #[test]
    fn accepts_exactly_64_characters() {
        let input = "a".repeat(MAX_NAME_LEN);
        assert!(SkillName::parse(&input).is_ok());
    }

    #[test]
    fn rejects_65_characters() {
        let input = "a".repeat(MAX_NAME_LEN + 1);
        assert_eq!(
            SkillName::parse(&input),
            Err(NameError::TooLong {
                input,
                len: MAX_NAME_LEN + 1
            })
        );
    }

    #[test]
    fn rejects_windows_reserved_device_names() {
        for input in ["con", "prn", "aux", "nul", "com1", "com9", "lpt1", "lpt9"] {
            assert_eq!(
                SkillName::parse(input),
                Err(NameError::Reserved {
                    input: input.to_string()
                })
            );
        }
    }

    #[test]
    fn accepts_names_that_merely_start_with_reserved_words() {
        for input in ["console", "nullable", "com10"] {
            assert!(SkillName::parse(input).is_ok());
        }
    }
}
