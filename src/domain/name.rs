//! skill 名称校验：目录名即 skill 身份。

/// `name` 的字符上限（按 Unicode 字符计）。
pub const MAX_NAME_LEN: usize = 64;

/// 已校验的 skill 名称。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SkillName(String);

/// 名称格式不符、过长或与 Windows 保留设备名冲突。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    /// 格式不符：只允许字母、数字与单个连字符，且首尾必须是字母或数字。
    /// 是否允许大写由调用方决定（skill 名不允许，组名允许）。
    Format { input: String },
    /// 超过 [`MAX_NAME_LEN`] 个字符。
    TooLong { input: String, len: usize },
    /// 与 Windows 保留设备名冲突。
    Reserved { input: String },
}

impl SkillName {
    /// 解析并完成全部校验（格式、长度、Windows 保留名）。
    pub fn parse(input: &str) -> Result<Self, NameError> {
        validate(input, false)?;
        Ok(SkillName(input.to_string()))
    }

    /// 已校验的原始值。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 只校验、不构造 [`SkillName`]；目录扫描过滤时避免为丢弃的结果分配。
    pub fn is_valid(input: &str) -> bool {
        validate(input, false).is_ok()
    }
}

/// 名称的共享校验：长度、字符集与 Windows 保留名。
///
/// `allow_uppercase` 只放宽字符集；保留名检查始终大小写不敏感，
/// 因此组名的 `CON` 与 `con` 同样被拒绝。
pub(crate) fn validate(input: &str, allow_uppercase: bool) -> Result<(), NameError> {
    let len = input.chars().count();
    if len > MAX_NAME_LEN {
        return Err(NameError::TooLong {
            input: input.to_string(),
            len,
        });
    }

    if !has_valid_format(input, allow_uppercase) {
        return Err(NameError::Format {
            input: input.to_string(),
        });
    }

    if is_windows_reserved(input) {
        return Err(NameError::Reserved {
            input: input.to_string(),
        });
    }

    Ok(())
}

/// 字母/数字开头结尾，连字符必须单个且不落在首尾；`allow_uppercase` 决定接受 `A-Z`。
fn has_valid_format(input: &str, allow_uppercase: bool) -> bool {
    if input.is_empty() {
        return false;
    }

    let mut previous_is_hyphen = true;
    for ch in input.chars() {
        match ch {
            '-' if previous_is_hyphen => return false,
            '-' => previous_is_hyphen = true,
            ch if ch.is_ascii_lowercase() || ch.is_ascii_digit() => previous_is_hyphen = false,
            ch if allow_uppercase && ch.is_ascii_uppercase() => previous_is_hyphen = false,
            _ => return false,
        }
    }

    !previous_is_hyphen
}

/// `con`/`prn`/`aux`/`nul` 与 `com1`~`com9`、`lpt1`~`lpt9`；大小写不敏感。
fn is_windows_reserved(input: &str) -> bool {
    let input = input.to_ascii_lowercase();

    if matches!(input.as_str(), "con" | "prn" | "aux" | "nul") {
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

    #[test]
    fn is_valid_matches_parse() {
        let too_long = "a".repeat(MAX_NAME_LEN + 1);
        let cases = [
            "a",
            "my-skill",
            "My-Skill",
            "a--b",
            "",
            "con",
            "console",
            "com10",
            too_long.as_str(),
        ];

        for input in cases {
            assert_eq!(
                SkillName::is_valid(input),
                SkillName::parse(input).is_ok(),
                "{input:?}"
            );
        }
    }
}
