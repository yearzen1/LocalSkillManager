//! name 解析：把 [`crate::domain::resolve`] 的结果翻成结构化错误（Q50 / Q58）。

use crate::domain::group::{GroupInput, GroupName};
use crate::domain::name::SkillName;
use crate::domain::resolve::{self, Match};
use crate::domain::scope::Scope;
use crate::error::AppError;
use crate::i18n::MsgKey;

/// 解析一个名字输入（可能是前缀）；零命中时返回调用方给的 not-found 错误，有相近名字时把 hint 换成候选。
pub fn resolve_name(
    input: &str,
    available: &[String],
    scope: Scope,
    not_found: AppError,
) -> Result<SkillName, AppError> {
    match resolve::match_name(input, available) {
        Match::Found(name) => Ok(SkillName::parse(&name).expect("scanned names are valid")),
        Match::Ambiguous(candidates) => {
            let listed = candidates.join(", ");
            Err(AppError::runtime(MsgKey::NameAmbiguous)
                .with_args([input, scope.label(), listed.as_str()])
                .with_hint_args(MsgKey::NameAmbiguousHint, [scope.label()]))
        }
        Match::Missing => Err(with_suggestions(not_found, input, available)),
    }
}

/// 解析一个组名输入（可能是前缀）：精确优先，其次唯一前缀；零命中时列出相近组名（Q58）。
pub fn resolve_group(input: &GroupInput, available: &[String]) -> Result<GroupName, AppError> {
    match resolve::match_name(input.as_str(), available) {
        Match::Found(name) => {
            Ok(GroupName::parse(&name).expect("group names from config are already validated"))
        }
        Match::Ambiguous(candidates) => Err(AppError::runtime(MsgKey::GroupAmbiguous)
            .with_args([input.as_str(), candidates.join(", ").as_str()])
            .with_hint(MsgKey::GroupAmbiguousHint)),
        Match::Missing => {
            let err = AppError::runtime(MsgKey::GroupNotFound).with_args([input.as_str()]);
            Err(with_suggestions(err, input.as_str(), available))
        }
    }
}

/// 零命中时若存在相近名字，把 hint 换成候选列表。
fn with_suggestions(err: AppError, input: &str, available: &[String]) -> AppError {
    let candidates = resolve::suggestions(input, available, 3);
    if candidates.is_empty() {
        return err;
    }
    err.with_hint_args(MsgKey::NameSuggestion, [candidates.join(", ")])
}
