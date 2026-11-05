//! `remove`：从指定作用域卸载 skill。

use std::path::Path;

use crate::domain::name::SkillName;
use crate::domain::scope::Scope;
use crate::error::{AppError, Msg};
use crate::i18n::MsgKey;
use crate::infra::fsops;
use crate::infra::{frontmatter, paths, scan};
use crate::output;
use crate::Ctx;

use super::resolve::resolve_name;

pub fn run(ctx: &Ctx, scope: Scope, names: Vec<SkillName>, force: bool) -> Result<(), AppError> {
    let directory = paths::skills_dir(&ctx.env, scope, &ctx.cwd)?;
    let available = scan::dir_names(&directory)?;

    if let [input] = names.as_slice() {
        let name = resolve_name(input, &available, scope, remove_not_found(input, scope))?;
        return remove_one(ctx, scope, &name, force);
    }

    let mut resolved: Vec<SkillName> = Vec::new();
    let mut failed = 0usize;

    // 先解析全部 name；歧义或缺失按该条失败记账，其余照常执行。
    for input in &names {
        match resolve_name(input, &available, scope, remove_not_found(input, scope)) {
            Ok(name) if !resolved.contains(&name) => resolved.push(name),
            Ok(_) => {}
            Err(err) => {
                failed += 1;
                output::emit_stderr(&crate::output::error::render_block(&err, ctx.lang));
            }
        }
    }

    let mut removed = 0usize;

    for name in &resolved {
        match remove_one(ctx, scope, name, force) {
            Ok(()) => removed += 1,
            Err(err) => {
                failed += 1;
                output::emit_stderr(&crate::output::error::render_block(&err, ctx.lang));
            }
        }
    }

    output::emit_line(
        ctx.lang,
        &Msg::with_args(
            MsgKey::RemoveBatchSummary,
            [removed.to_string(), failed.to_string()],
        ),
    );

    if failed > 0 {
        output::emit_gap();
        return Err(AppError::runtime(MsgKey::RemoveBatchFailed)
            .with_args([failed.to_string()])
            .with_hint(MsgKey::RemoveBatchFailedHint));
    }

    Ok(())
}

/// `remove` 的 not-found 错误；hint 指向 `search`（Q47）。
fn remove_not_found(name: &SkillName, scope: Scope) -> AppError {
    AppError::runtime(MsgKey::RemoveNotFound)
        .with_args([name.as_str(), scope.label()])
        .with_hint_args(MsgKey::RemoveNotFoundHint, [name.as_str(), scope.label()])
}

/// 删除单个 skill；多 name 时由 [`run`] 累计成功/失败。
fn remove_one(ctx: &Ctx, scope: Scope, name: &SkillName, force: bool) -> Result<(), AppError> {
    let target = paths::skill_dir(&ctx.env, scope, &ctx.cwd, name)?;

    if !fsops::is_present(&target) {
        return Err(remove_not_found(name, scope));
    }

    if !force {
        ensure_valid_skill(&target, name)?;
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
