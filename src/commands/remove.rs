//! `remove`：从指定作用域卸载 skill。

use std::path::Path;

use crate::domain::name::SkillName;
use crate::domain::scope::Scope;
use crate::error::{AppError, Msg};
use crate::i18n::MsgKey;
use crate::infra::fsops;
use crate::infra::{frontmatter, paths};
use crate::output;
use crate::Ctx;

pub fn run(ctx: &Ctx, scope: Scope, name: SkillName, force: bool) -> Result<(), AppError> {
    let target = paths::skill_dir(&ctx.env, scope, &ctx.cwd, &name)?;

    if !fsops::is_present(&target) {
        return Err(AppError::runtime(MsgKey::RemoveNotFound)
            .with_args([name.as_str(), scope.label()])
            .with_hint_args(MsgKey::RemoveNotFoundHint, [scope.label()]));
    }

    if !force {
        ensure_valid_skill(&target, &name)?;
    }

    let report = fsops::remove_entry_atomically(&target)?;

    if let Some(tmp) = report.leftover_temp {
        output::warn(
            ctx.lang,
            &Msg::with_args(MsgKey::RemoveLeftover, [tmp.display().to_string()]),
        );
    }

    output::emit_line(
        ctx.lang,
        &Msg::with_args(
            MsgKey::RemoveRemoved,
            [name.as_str().to_string(), target.display().to_string()],
        ),
    );

    Ok(())
}

/// 无 `--force` 时只认「目录名等于 name、SKILL.md 合法、frontmatter name 一致」的目录。
fn ensure_valid_skill(target: &Path, name: &SkillName) -> Result<(), AppError> {
    let meta = frontmatter::parse_skill_md(&target.join("SKILL.md")).map_err(|err| {
        AppError::runtime(MsgKey::RemoveNotSkill)
            .with_args([name.as_str().to_string(), frontmatter::describe(&err)])
            .with_hint(MsgKey::RemoveNotSkillHint)
    })?;

    if meta.name != *name {
        let detail = format!("name mismatch: {} vs {}", name.as_str(), meta.name.as_str());
        return Err(AppError::runtime(MsgKey::RemoveNotSkill)
            .with_args([name.as_str().to_string(), detail])
            .with_hint(MsgKey::RemoveNotSkillHint));
    }

    Ok(())
}
