//! name 解析：把 [`crate::domain::resolve`] 的结果翻成结构化错误（Q50 / Q58）。

use crate::domain::group::GroupName;
use crate::domain::name::SkillName;
use crate::domain::resolve::{self, Match, Resolution};
use crate::domain::scope::Scope;
use crate::error::AppError;
use crate::i18n::MsgKey;

/// 解析一个 name；零命中时返回调用方给的 not-found 错误，有相近名字时把 hint 换成候选。
pub fn resolve_name(
    input: &SkillName,
    available: &[String],
    scope: Scope,
    not_found: AppError,
) -> Result<SkillName, AppError> {
    match resolve::resolve(input, available) {
        Resolution::Exact(name) | Resolution::Prefix(name) => Ok(name),
        Resolution::Ambiguous(candidates) => {
            let listed = candidates.join(", ");
            Err(AppError::runtime(MsgKey::NameAmbiguous)
                .with_args([input.as_str(), scope.label(), listed.as_str()])
                .with_hint_args(MsgKey::NameAmbiguousHint, [scope.label()]))
        }
        Resolution::Missing => Err(with_suggestions(not_found, input.as_str(), available)),
    }
}

/// 解析一个组名：精确优先，其次唯一前缀；零命中时列出相近组名（Q58）。
pub fn resolve_group(input: &GroupName, available: &[String]) -> Result<GroupName, AppError> {
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
