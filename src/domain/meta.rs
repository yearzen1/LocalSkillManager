//! `SKILL.md` frontmatter 的元数据与其校验规则。

use super::name::SkillName;

/// `description` 的字符上限（按 Unicode 字符计）。
pub const MAX_DESCRIPTION_LEN: usize = 1024;

/// frontmatter 中的 `name` / `description`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillMeta {
    pub name: SkillName,
    pub description: String,
}

/// 元数据校验失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaError {
    /// `description` trim 后为空。
    DescriptionEmpty,
    /// 超过 [`MAX_DESCRIPTION_LEN`] 个字符。
    DescriptionTooLong { len: usize },
}

impl SkillMeta {
    /// 校验 `description`（`name` 在构造 [`SkillName`] 时已校验）。
    pub fn validate(&self) -> Result<(), MetaError> {
        if self.description.trim().is_empty() {
            return Err(MetaError::DescriptionEmpty);
        }

        let len = self.description.chars().count();
        if len > MAX_DESCRIPTION_LEN {
            return Err(MetaError::DescriptionTooLong { len });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(description: &str) -> SkillMeta {
        SkillMeta {
            name: SkillName::parse("my-skill").unwrap(),
            description: description.to_string(),
        }
    }

    #[test]
    fn accepts_non_empty_description() {
        assert_eq!(meta("Does one thing.").validate(), Ok(()));
        assert_eq!(meta("  padded  ").validate(), Ok(()));
    }

    #[test]
    fn rejects_empty_or_whitespace_only_description() {
        assert_eq!(meta("").validate(), Err(MetaError::DescriptionEmpty));
        assert_eq!(meta("   ").validate(), Err(MetaError::DescriptionEmpty));
        assert_eq!(meta("\n\t").validate(), Err(MetaError::DescriptionEmpty));
    }

    #[test]
    fn accepts_exactly_1024_characters() {
        let description = "d".repeat(MAX_DESCRIPTION_LEN);
        assert_eq!(meta(&description).validate(), Ok(()));
    }

    #[test]
    fn rejects_1025_characters() {
        let description = "d".repeat(MAX_DESCRIPTION_LEN + 1);
        assert_eq!(
            meta(&description).validate(),
            Err(MetaError::DescriptionTooLong {
                len: MAX_DESCRIPTION_LEN + 1
            })
        );
    }

    #[test]
    fn counts_unicode_characters_not_bytes() {
        let description = "说".repeat(MAX_DESCRIPTION_LEN);
        assert_eq!(meta(&description).validate(), Ok(()));
    }
}
