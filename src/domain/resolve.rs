//! name 解析：`pull` / `push` / `remove` 与分组共用的唯一命中规则（Q50 / Q58）。

use crate::domain::name::SkillName;

/// 输入 name 在某个作用域里的解析结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// 目录名与输入完全一致。
    Exact(SkillName),
    /// 恰好一个目录以输入为前缀，解析为该全名。
    Prefix(SkillName),
    /// 多个目录以输入为前缀；按字典序返回候选。
    Ambiguous(Vec<String>),
    /// 没有任何目录以输入为前缀。
    Missing,
}

/// 字符串层面的解析结果；组名等非 [`SkillName`] 输入复用同一规则。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Match {
    /// 精确命中，或恰好一个名字以输入为前缀。
    Found(String),
    /// 多个名字以输入为前缀；按字典序返回候选。
    Ambiguous(Vec<String>),
    /// 没有任何名字以输入为前缀。
    Missing,
}

/// 精确优先，其次唯一前缀；只做前缀，不做子串。
pub fn match_name(input: &str, available: &[String]) -> Match {
    if available.iter().any(|name| name == input) {
        return Match::Found(input.to_string());
    }

    let mut matches: Vec<String> = available
        .iter()
        .filter(|name| name.starts_with(input))
        .cloned()
        .collect();
    matches.sort();

    match matches.len() {
        0 => Match::Missing,
        1 => Match::Found(matches.remove(0)),
        _ => Match::Ambiguous(matches),
    }
}

/// [`SkillName`] 版本的解析；规则与 [`match_name`] 相同。
pub fn resolve(input: &SkillName, available: &[String]) -> Resolution {
    match match_name(input.as_str(), available) {
        Match::Found(name) => match SkillName::parse(&name) {
            Ok(found) if found == *input => Resolution::Exact(found),
            Ok(found) => Resolution::Prefix(found),
            Err(_) => Resolution::Missing,
        },
        Match::Ambiguous(candidates) => Resolution::Ambiguous(candidates),
        Match::Missing => Resolution::Missing,
    }
}

/// 最多 `limit` 条相近名字：子串命中优先，其次编辑距离；结果按相似度与字典序排序。
///
/// 只用于错误提示，不参与自动解析。
pub fn suggestions(input: &str, available: &[String], limit: usize) -> Vec<String> {
    if input.is_empty() || limit == 0 {
        return Vec::new();
    }

    let needle = input.to_lowercase();
    let max_distance = std::cmp::max(2, needle.chars().count() / 3);

    let mut scored: Vec<(usize, &String)> = available
        .iter()
        .filter_map(|name| {
            let lower = name.to_lowercase();
            if lower == needle {
                return Some((0, name));
            }
            if lower.contains(&needle) {
                return Some((1, name));
            }
            let distance = levenshtein(&needle, &lower);
            (distance <= max_distance).then_some((2 + distance, name))
        })
        .collect();

    scored.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1)));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, name)| name.clone())
        .collect()
}

/// 两个字符串的编辑距离；按字符计，避免把多字节字符拆开。
fn levenshtein(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0usize; right.len() + 1];

    for (i, left_char) in left.chars().enumerate() {
        current[0] = i + 1;
        for (j, right_char) in right.iter().enumerate() {
            let cost = usize::from(left_char != *right_char);
            current[j + 1] = (previous[j] + cost)
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(input: &str) -> SkillName {
        SkillName::parse(input).expect("test name should be valid")
    }

    fn available(inputs: &[&str]) -> Vec<String> {
        inputs.iter().map(|input| input.to_string()).collect()
    }

    #[test]
    fn exact_match_wins_over_prefix() {
        let names = available(&["a", "ab"]);

        assert_eq!(resolve(&name("a"), &names), Resolution::Exact(name("a")));
    }

    #[test]
    fn unique_prefix_resolves_to_full_name() {
        let names = available(&["code-review", "docs"]);

        assert_eq!(
            resolve(&name("code"), &names),
            Resolution::Prefix(name("code-review"))
        );
    }

    #[test]
    fn multiple_matches_are_sorted_as_candidates() {
        let names = available(&["code-review", "code-lint"]);

        assert_eq!(
            resolve(&name("code"), &names),
            Resolution::Ambiguous(vec!["code-lint".to_string(), "code-review".to_string()])
        );
    }

    #[test]
    fn no_match_is_missing() {
        assert_eq!(
            resolve(&name("nope"), &available(&["code-review"])),
            Resolution::Missing
        );
    }

    #[test]
    fn does_not_match_suffix_or_substring() {
        assert_eq!(
            resolve(&name("review"), &available(&["code-review"])),
            Resolution::Missing
        );
    }

    #[test]
    fn string_inputs_use_the_same_rule() {
        let names = available(&["predevelop", "predev-tools"]);

        assert_eq!(
            match_name("predevelop", &names),
            Match::Found("predevelop".to_string())
        );
        assert_eq!(
            match_name("predev", &names),
            Match::Ambiguous(vec!["predev-tools".to_string(), "predevelop".to_string()])
        );
        assert_eq!(match_name("nope", &names), Match::Missing);
    }

    #[test]
    fn suggestions_prefer_substring_then_edit_distance() {
        let names = available(&["grill", "code-review", "code-lint"]);

        assert_eq!(suggestions("gril", &names, 3), vec!["grill"]);
        assert_eq!(suggestions("review", &names, 3), vec!["code-review"]);
        assert_eq!(suggestions("groll", &names, 3), vec!["grill"]);
    }

    #[test]
    fn suggestions_respect_the_limit_and_empty_input() {
        let names = available(&["grill", "grill-2", "grill-3"]);

        assert_eq!(suggestions("gril", &names, 2).len(), 2);
        assert!(suggestions("", &names, 3).is_empty());
        assert!(suggestions("gril", &names, 0).is_empty());
    }

    #[test]
    fn levenshtein_counts_character_edits() {
        assert_eq!(levenshtein("gril", "grill"), 1);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }
}
