//! `list` 的文本表格与 JSON 输出。JSON 字段固定英文，不参与 i18n。

use std::path::Path;

use crate::domain::entry::SkillEntry;
use crate::domain::lang::Lang;
use crate::i18n::{tr, MsgKey};

/// description 在文本表格里的字符上限。
const DESCRIPTION_LIMIT: usize = 60;

/// 非法项的固定标记。
const INVALID_MARKER: &str = "[invalid]";

/// 渲染文本表格。description 压成单行并截断，非法项标 `[invalid]`。
pub fn render_text(lang: Lang, entries: &[SkillEntry]) -> String {
    let name_width = entries
        .iter()
        .map(|entry| entry.name.chars().count())
        .max()
        .unwrap_or(0)
        .max(10);

    let mut text = String::new();
    text.push_str(&format!(
        "{:<name_width$}  {}\n",
        tr(lang, MsgKey::ListHeaderName),
        tr(lang, MsgKey::ListHeaderDescription),
    ));
    text.push_str(&format!("{}  {}\n", "-".repeat(name_width), "-".repeat(42)));

    for entry in entries {
        let description = match &entry.error {
            Some(error) => format!("{INVALID_MARKER} {error}"),
            None => summarize(&entry.description),
        };
        text.push_str(&format!("{:<name_width$}  {}\n", entry.name, description));
    }

    text
}

/// 渲染 JSON 数组；空列表输出 `[]`。
pub fn render_json(entries: &[SkillEntry]) -> String {
    if entries.is_empty() {
        return "[]".to_string();
    }

    let rows: Vec<JsonEntry> = entries
        .iter()
        .map(|entry| JsonEntry {
            name: &entry.name,
            description: &entry.description,
            path: entry.path.display().to_string(),
            valid: entry.valid,
            error: entry.error.as_deref(),
        })
        .collect();

    serde_json::to_string_pretty(&rows).expect("serializing plain skill entries cannot fail")
}

/// 文本列：压成单行、按字符截断到 [`DESCRIPTION_LIMIT`]，超出加 `...`。
fn summarize(description: &str) -> String {
    let collapsed = description.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut characters = collapsed.chars();

    let truncated: String = characters.by_ref().take(DESCRIPTION_LIMIT).collect();
    if characters.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

#[derive(serde::Serialize)]
struct JsonEntry<'a> {
    name: &'a str,
    description: &'a str,
    path: String,
    valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}

/// 空目录提示。
pub fn empty_message(lang: Lang, dir: &Path) -> String {
    crate::i18n::tr_args(lang, MsgKey::ListEmpty, &[&dir.display().to_string()])
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn entry(name: &str, description: &str) -> SkillEntry {
        SkillEntry {
            name: name.to_string(),
            description: description.to_string(),
            path: PathBuf::from(format!("/skills/{name}")),
            valid: true,
            error: None,
        }
    }

    fn invalid(name: &str, error: &str) -> SkillEntry {
        SkillEntry {
            name: name.to_string(),
            description: String::new(),
            path: PathBuf::from(format!("/skills/{name}")),
            valid: false,
            error: Some(error.to_string()),
        }
    }

    #[test]
    fn renders_header_and_rows() {
        let text = render_text(Lang::En, &[entry("my-skill", "One-line description.")]);
        let lines: Vec<&str> = text.lines().collect();

        assert!(lines[0].starts_with("NAME"));
        assert!(lines[0].contains("DESCRIPTION"));
        assert!(lines[1].starts_with("----------"));
        assert!(lines[2].starts_with("my-skill"));
        assert!(lines[2].contains("One-line description."));
    }

    #[test]
    fn localizes_header() {
        let text = render_text(Lang::Zh, &[entry("my-skill", "一句话说明。")]);
        assert!(text.contains("名称"));
        assert!(text.contains("描述"));
    }

    #[test]
    fn collapses_whitespace_in_description() {
        let text = render_text(Lang::En, &[entry("my-skill", "two\r\nlines\tand   spaces")]);
        assert!(text.contains("two lines and spaces"));
    }

    #[test]
    fn truncates_long_description_to_sixty_characters() {
        let long = "a".repeat(DESCRIPTION_LIMIT + 10);
        let text = render_text(Lang::En, &[entry("my-skill", &long)]);

        assert!(text.contains(&format!("{}...", "a".repeat(DESCRIPTION_LIMIT))));
        assert!(!text.contains(&"a".repeat(DESCRIPTION_LIMIT + 1)));
    }

    #[test]
    fn keeps_exactly_sixty_characters_without_ellipsis() {
        let exact = "b".repeat(DESCRIPTION_LIMIT);
        let text = render_text(Lang::En, &[entry("my-skill", &exact)]);

        assert!(text.contains(&exact));
        assert!(!text.contains("..."));
    }

    #[test]
    fn marks_invalid_entries_with_error() {
        let text = render_text(Lang::En, &[invalid("broken", "missing SKILL.md")]);
        assert!(text.contains("[invalid] missing SKILL.md"));
    }

    #[test]
    fn json_empty_list_is_brackets() {
        assert_eq!(render_json(&[]), "[]");
    }

    #[test]
    fn json_contains_all_fields_in_order() {
        let json = render_json(&[entry("my-skill", "One line.")]);

        assert!(json.starts_with('['));
        assert!(json.contains("\"name\": \"my-skill\""));
        assert!(json.contains("\"description\": \"One line.\""));
        assert!(json.contains("\"path\""));
        assert!(json.contains("\"valid\": true"));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn json_adds_error_only_for_invalid_entries() {
        let json = render_json(&[entry("ok", "fine"), invalid("broken", "missing SKILL.md")]);

        assert!(json.contains("\"valid\": false"));
        assert!(json.contains("\"error\": \"missing SKILL.md\""));
    }

    #[test]
    fn json_keeps_full_description() {
        let long = "a".repeat(DESCRIPTION_LIMIT + 10);
        let json = render_json(&[entry("my-skill", &long)]);

        assert!(json.contains(&long));
        assert!(!json.contains("..."));
    }
}
