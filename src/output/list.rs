//! `list` 的文本表格与 JSON 输出。JSON 字段固定英文，不参与 i18n。

use std::path::Path;

use crate::domain::entry::SkillEntry;
use crate::domain::lang::Lang;
use crate::i18n::{tr, MsgKey};

/// description 列在终端里占的显示列数（宽字符按 2 列计）。
const DESCRIPTION_WIDTH: usize = 60;

/// description 换行后的最大行数，超出在第 3 行末尾以省略号收尾。
const MAX_LINES: usize = 3;

/// 截断标记。
const ELLIPSIS: &str = "...";

/// 非法项的固定标记。
const INVALID_MARKER: &str = "[invalid]";

/// 渲染文本表格。description 压成单行后按 60 显示列换行，续行悬挂缩进到描述列。
pub fn render_text(lang: Lang, entries: &[SkillEntry]) -> String {
    let name_width = entries
        .iter()
        .map(|entry| display_width(&entry.name))
        .max()
        .unwrap_or(0)
        .max(10);

    let mut text = String::new();
    text.push_str(&format!(
        "{}  {}\n",
        pad_to(tr(lang, MsgKey::ListHeaderName), name_width),
        tr(lang, MsgKey::ListHeaderDescription),
    ));
    text.push_str(&format!("{}  {}\n", "-".repeat(name_width), "-".repeat(42)));

    let indent = " ".repeat(name_width + 2);
    for entry in entries {
        let description = match &entry.error {
            Some(error) => format!("{INVALID_MARKER} {error}"),
            None => entry.description.clone(),
        };

        for (index, line) in wrap(&description).into_iter().enumerate() {
            if index == 0 {
                text.push_str(&format!("{}  {line}\n", pad_to(&entry.name, name_width)));
            } else {
                text.push_str(&format!("{indent}{line}\n"));
            }
        }
    }

    text
}

/// 右侧补空格到指定显示宽度（CJK 字符按 2 列计）。
fn pad_to(text: &str, width: usize) -> String {
    let mut padded = text.to_string();
    for _ in display_width(text)..width {
        padded.push(' ');
    }
    padded
}

/// 终端显示宽度：宽字符（CJK、全角标点等）占两列。
fn display_width(text: &str) -> usize {
    text.chars()
        .map(|character| if is_wide(character) { 2 } else { 1 })
        .sum()
}

fn is_wide(character: char) -> bool {
    matches!(
        character as u32,
        0x1100..=0x115F
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1F64F
            | 0x20000..=0x3FFFD
    )
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

/// 把 description 排成不超过 [`DESCRIPTION_WIDTH`] 显示列的若干行，最多 [`MAX_LINES`] 行。
///
/// 先压成单空格分隔的 token 流；英文单词整体换行，单个 token 本身超宽时按列硬切，
/// 保证宽字符不会被劈成半格。
fn wrap(description: &str) -> Vec<String> {
    let collapsed = description.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return vec![String::new()];
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for token in collapsed.split(' ') {
        for chunk in split_by_width(token) {
            if current.is_empty() {
                current.push_str(&chunk);
            } else if display_width(&current) + 1 + display_width(&chunk) <= DESCRIPTION_WIDTH {
                current.push(' ');
                current.push_str(&chunk);
            } else {
                lines.push(std::mem::take(&mut current));
                current.push_str(&chunk);
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }

    if lines.len() > MAX_LINES {
        lines.truncate(MAX_LINES);
        let last = lines.pop().expect("MAX_LINES is greater than zero");
        let kept = truncate_to_width(&last, DESCRIPTION_WIDTH - ELLIPSIS.len());
        lines.push(format!("{kept}{ELLIPSIS}"));
    }

    lines
}

/// 把超宽 token 按显示列切成若干块；宽字符独占整列，不跨行拆开。
fn split_by_width(token: &str) -> Vec<String> {
    if display_width(token) <= DESCRIPTION_WIDTH {
        return vec![token.to_string()];
    }

    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut width = 0;

    for character in token.chars() {
        let character_width = if is_wide(character) { 2 } else { 1 };
        if width + character_width > DESCRIPTION_WIDTH {
            chunks.push(std::mem::take(&mut current));
            width = 0;
        }
        current.push(character);
        width += character_width;
    }
    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

/// 按显示宽度截断，保证不把一个字符留在半格上。
fn truncate_to_width(text: &str, width: usize) -> String {
    let mut kept = String::new();
    let mut used = 0;

    for character in text.chars() {
        let character_width = if is_wide(character) { 2 } else { 1 };
        if used + character_width > width {
            break;
        }
        kept.push(character);
        used += character_width;
    }

    kept
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
    fn aligns_columns_with_wide_characters() {
        let text = render_text(Lang::Zh, &[entry("my-skill", "一句话说明。")]);
        let header = text.lines().next().unwrap();
        let row = text.lines().nth(2).unwrap();

        let header_prefix = &header[..header.find('描').unwrap()];
        let row_prefix = &row[..row.find('一').unwrap()];

        assert_eq!(display_width(header_prefix), display_width(row_prefix));
    }

    #[test]
    fn collapses_whitespace_in_description() {
        let text = render_text(Lang::En, &[entry("my-skill", "two\r\nlines\tand   spaces")]);
        assert!(text.contains("two lines and spaces"));
    }

    #[test]
    fn wraps_wide_characters_at_sixty_display_columns() {
        let text = render_text(Lang::En, &[entry("my-skill", &"干".repeat(45))]);
        let lines: Vec<&str> = text.lines().collect();

        // 45 个宽字符 = 90 列 → 60 列 + 30 列，共两行
        assert_eq!(lines.len(), 4);
        assert!(lines[2].ends_with(&"干".repeat(30)));
        assert_eq!(lines[3].trim_start(), "干".repeat(15));
    }

    #[test]
    fn continuation_lines_align_with_description_column() {
        let text = render_text(Lang::Zh, &[entry("my-skill", &"干".repeat(60))]);
        let lines: Vec<&str> = text.lines().collect();

        let indent = " ".repeat(12);
        assert_eq!(lines.len(), 4);
        assert!(lines[3].starts_with(&indent));
        assert_eq!(&lines[3][indent.len()..], "干".repeat(30));
    }

    #[test]
    fn never_exceeds_sixty_display_columns() {
        let description = format!("{} {}", "word ".repeat(40), "干".repeat(40));
        let text = render_text(Lang::En, &[entry("my-skill", &description)]);

        for row in description_rows(&text) {
            assert!(display_width(&row) <= 60, "row over 60 columns: {row:?}");
        }
    }

    #[test]
    fn caps_long_description_at_three_lines() {
        let text = render_text(Lang::En, &[entry("my-skill", &"a".repeat(1024))]);
        let lines: Vec<&str> = text.lines().collect();

        assert_eq!(lines.len(), 5);
        assert!(lines[4].trim_start().ends_with("..."));
        assert_eq!(display_width(lines[4].trim_start()), 60);
    }

    #[test]
    fn keeps_exactly_sixty_columns_without_ellipsis() {
        let exact = "b".repeat(60);
        let text = render_text(Lang::En, &[entry("my-skill", &exact)]);

        assert_eq!(text.lines().count(), 3);
        assert!(text.contains(&exact));
        assert!(!text.contains("..."));
    }

    #[test]
    fn wraps_ascii_at_word_boundaries() {
        let words: Vec<String> = (0..20).map(|index| format!("word{index:02}")).collect();
        let text = render_text(Lang::En, &[entry("my-skill", &words.join(" "))]);

        let rows = description_rows(&text);
        let wrapped: Vec<&str> = rows.iter().flat_map(|row| row.split_whitespace()).collect();
        assert_eq!(wrapped, words);
    }

    /// 取出每一行描述列的内容：name 列宽固定 `max(10, 8)`，描述列从第 12 列开始。
    fn description_rows(text: &str) -> Vec<String> {
        text.lines()
            .skip(2)
            .map(|line| line.chars().skip(12).collect::<String>())
            .collect()
    }

    #[test]
    fn wraps_long_invalid_error_instead_of_overflowing() {
        let text = render_text(Lang::En, &[invalid("broken", &"x".repeat(80))]);
        let rows = description_rows(&text);

        assert_eq!(
            rows,
            vec!["[invalid]".to_string(), "x".repeat(60), "x".repeat(20)]
        );
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
        let long = "a".repeat(DESCRIPTION_WIDTH + 10);
        let json = render_json(&[entry("my-skill", &long)]);

        assert!(json.contains(&long));
        assert!(!json.contains("..."));
    }
}
