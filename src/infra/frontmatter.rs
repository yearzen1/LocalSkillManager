//! 读取并解析 `SKILL.md` 的 frontmatter。只取 `name` / `description`，其它字段与正文不参与、不改写。

use std::path::{Path, PathBuf};

use crate::domain::meta::{MetaError, SkillMeta};
use crate::domain::name::{NameError, SkillName};

/// frontmatter 解析失败。
#[derive(Debug)]
pub enum FrontmatterError {
    /// 读取文件失败。
    Io {
        path: PathBuf,
        source: anyhow::Error,
    },
    /// 文件不是合法 UTF-8。
    InvalidUtf8 { path: PathBuf },
    /// 文件开头没有 `---` 包裹的 frontmatter 块。
    MissingFrontmatter { path: PathBuf },
    /// 缺少 `name` / `description`，或对应值不是字符串。
    MissingField { path: PathBuf, field: &'static str },
    /// frontmatter 不是合法 YAML。
    InvalidYaml {
        path: PathBuf,
        source: anyhow::Error,
    },
    /// `name` 不符合 [`SkillName`] 规则。
    InvalidName { path: PathBuf, source: NameError },
    /// `description` 不符合 [`SkillMeta`] 规则。
    InvalidMeta { path: PathBuf, source: MetaError },
}

/// 解析 `SKILL.md`，返回已通过全部校验的元数据。
pub fn parse_skill_md(path: &Path) -> Result<SkillMeta, FrontmatterError> {
    let with_path = || path.to_path_buf();

    let bytes = std::fs::read(path).map_err(|err| FrontmatterError::Io {
        path: with_path(),
        source: anyhow::Error::new(err),
    })?;

    let text = String::from_utf8(bytes)
        .map_err(|_| FrontmatterError::InvalidUtf8 { path: with_path() })?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);

    let block = extract_frontmatter(text)
        .ok_or_else(|| FrontmatterError::MissingFrontmatter { path: with_path() })?;

    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&block).map_err(|err| FrontmatterError::InvalidYaml {
            path: with_path(),
            source: anyhow::Error::new(err),
        })?;

    let missing = |field: &'static str| FrontmatterError::MissingField {
        path: with_path(),
        field,
    };

    let raw_name = string_field(&value, "name").ok_or_else(|| missing("name"))?;
    let description = string_field(&value, "description").ok_or_else(|| missing("description"))?;

    let name = SkillName::parse(raw_name).map_err(|source| FrontmatterError::InvalidName {
        path: with_path(),
        source,
    })?;

    let meta = SkillMeta {
        name,
        description: description.to_string(),
    };
    meta.validate()
        .map_err(|source| FrontmatterError::InvalidMeta {
            path: with_path(),
            source,
        })?;

    Ok(meta)
}

/// 取出首个 `---` 行到下一个 `---` 行之间的正文（不含分隔行）。
fn extract_frontmatter(text: &str) -> Option<String> {
    let mut lines = text.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }

    let mut body = Vec::new();
    for line in lines {
        if line.trim_end() == "---" {
            return Some(body.join("\n"));
        }
        body.push(line);
    }

    None
}

fn string_field<'a>(value: &'a serde_yaml_ng::Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(serde_yaml_ng::Value::as_str)
}

/// 把解析错误压成一句话，供 `list --json` 与 `copy` 的源校验复用。固定英文，不参与 i18n。
pub fn describe(err: &FrontmatterError) -> String {
    match err {
        FrontmatterError::Io { source, .. } => match source.downcast_ref::<std::io::Error>() {
            Some(io) if io.kind() == std::io::ErrorKind::NotFound => "missing SKILL.md".to_string(),
            _ => "unreadable SKILL.md".to_string(),
        },
        FrontmatterError::InvalidUtf8 { .. } => "SKILL.md is not valid UTF-8".to_string(),
        FrontmatterError::MissingFrontmatter { .. } => "missing frontmatter".to_string(),
        FrontmatterError::MissingField { field, .. } => format!("missing {field}"),
        FrontmatterError::InvalidYaml { .. } => "invalid YAML frontmatter".to_string(),
        FrontmatterError::InvalidName { .. } => "invalid name".to_string(),
        FrontmatterError::InvalidMeta { .. } => "invalid description".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn write_skill(content: &[u8]) -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("SKILL.md");
        fs::write(&path, content).unwrap();
        (dir, path)
    }

    fn parse(content: &str) -> Result<SkillMeta, FrontmatterError> {
        let (_dir, path) = write_skill(content.as_bytes());
        parse_skill_md(&path)
    }

    #[test]
    fn parses_name_and_description() {
        let meta = parse("---\nname: my-skill\ndescription: Does one thing.\n---\n\n# Body\n")
            .expect("should parse");
        assert_eq!(meta.name.as_str(), "my-skill");
        assert_eq!(meta.description, "Does one thing.");
    }

    #[test]
    fn ignores_extra_frontmatter_fields() {
        let meta = parse("---\nname: my-skill\ndescription: One line.\nversion: 2\n---\nbody")
            .expect("should parse");
        assert_eq!(meta.name.as_str(), "my-skill");
    }

    #[test]
    fn rejects_file_without_frontmatter() {
        let err = parse("# My Skill\n\nbody").unwrap_err();
        assert!(matches!(err, FrontmatterError::MissingFrontmatter { .. }));
    }

    #[test]
    fn rejects_unclosed_frontmatter() {
        let err = parse("---\nname: my-skill\ndescription: One line.\n").unwrap_err();
        assert!(matches!(err, FrontmatterError::MissingFrontmatter { .. }));
    }

    #[test]
    fn rejects_missing_name() {
        let err = parse("---\ndescription: One line.\n---\n").unwrap_err();
        assert!(matches!(
            err,
            FrontmatterError::MissingField { field: "name", .. }
        ));
    }

    #[test]
    fn rejects_missing_description() {
        let err = parse("---\nname: my-skill\n---\n").unwrap_err();
        assert!(matches!(
            err,
            FrontmatterError::MissingField {
                field: "description",
                ..
            }
        ));
    }

    #[test]
    fn rejects_invalid_yaml() {
        let err = parse("---\nname: [unclosed\n---\n").unwrap_err();
        assert!(matches!(err, FrontmatterError::InvalidYaml { .. }));
    }

    #[test]
    fn strips_optional_bom() {
        let meta = parse("\u{feff}---\nname: my-skill\ndescription: One line.\n---\n")
            .expect("should parse");
        assert_eq!(meta.name.as_str(), "my-skill");
    }

    #[test]
    fn accepts_crlf_line_endings() {
        let meta = parse("---\r\nname: my-skill\r\ndescription: One line.\r\n---\r\nbody")
            .expect("should parse");
        assert_eq!(meta.name.as_str(), "my-skill");
    }

    #[test]
    fn rejects_non_utf8_content() {
        let (_dir, path) = write_skill(b"---\nname: my-skill\ndescription: \xff\xfe\n---\n");
        let err = parse_skill_md(&path).unwrap_err();
        assert!(matches!(err, FrontmatterError::InvalidUtf8 { .. }));
    }

    #[test]
    fn rejects_missing_file() {
        let dir = TempDir::new().unwrap();
        let err = parse_skill_md(&dir.path().join("SKILL.md")).unwrap_err();
        assert!(matches!(err, FrontmatterError::Io { .. }));
    }

    #[test]
    fn rejects_invalid_name() {
        let err = parse("---\nname: My-Skill\ndescription: One line.\n---\n").unwrap_err();
        assert!(matches!(err, FrontmatterError::InvalidName { .. }));
    }

    #[test]
    fn rejects_too_long_description() {
        let long = "d".repeat(1025);
        let err = parse(&format!("---\nname: my-skill\ndescription: {long}\n---\n")).unwrap_err();
        assert!(matches!(err, FrontmatterError::InvalidMeta { .. }));
    }

    #[test]
    fn leaves_file_content_untouched() {
        let content = "---\nname: my-skill\ndescription: One line.\nextra: keep me\n---\nbody";
        let (_dir, path) = write_skill(content.as_bytes());
        parse_skill_md(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
    }
}
