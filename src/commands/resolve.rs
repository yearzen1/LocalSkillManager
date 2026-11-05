//! name 解析：把 [`crate::domain::resolve`] 的结果翻成结构化错误（Q50）。

use crate::domain::name::SkillName;
use crate::domain::resolve::{self, Resolution};
use crate::domain::scope::Scope;
use crate::error::AppError;
use crate::i18n::MsgKey;

/// 解析一个 name；零命中时返回调用方给的 not-found 错误，保留各命令自己的 hint。
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
        Resolution::Missing => Err(not_found),
    }
}
