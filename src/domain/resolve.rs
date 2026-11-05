//! name 前缀解析：`pull` / `push` / `remove` 共用的唯一命中规则（Q50）。

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

/// 精确优先，其次唯一前缀；只做前缀，不做子串。
pub fn resolve(input: &SkillName, available: &[String]) -> Resolution {
    if available.iter().any(|name| name == input.as_str()) {
        return Resolution::Exact(input.clone());
    }

    let mut matches: Vec<String> = available
        .iter()
        .filter(|name| name.starts_with(input.as_str()))
        .cloned()
        .collect();
    matches.sort();

    match matches.len() {
        0 => Resolution::Missing,
        1 => match SkillName::parse(&matches[0]) {
            Ok(name) => Resolution::Prefix(name),
            Err(_) => Resolution::Missing,
        },
        _ => Resolution::Ambiguous(matches),
    }
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
}
