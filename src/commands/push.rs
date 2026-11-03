//! `push`：把当前项目的 skill 推送到全局目录（`copy` 的反向操作）。

use std::path::Path;

use crate::domain::action::CopyTarget;
use crate::domain::entry::SkillEntry;
use crate::domain::name::SkillName;
use crate::domain::scope::Scope;
use crate::error::{AppError, Msg};
use crate::i18n::MsgKey;
use crate::infra::fsops::{self, FsError};
use crate::infra::{frontmatter, paths, scan};
use crate::output;
use crate::output::error as error_output;
use crate::Ctx;

pub fn run(ctx: &Ctx, target: CopyTarget, force: bool) -> Result<(), AppError> {
    match target {
        CopyTarget::Many(names) => push_many(ctx, &names, force),
        CopyTarget::All => push_all(ctx, force),
    }
}

/// 逐条推送多个 name；单条失败只记账，不中断（Q44 / Q46）。
fn push_many(ctx: &Ctx, names: &[SkillName], force: bool) -> Result<(), AppError> {
    if let [name] = names {
        return push_one(ctx, name, force);
    }

    let mut pushed = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for name in names {
        let destination = paths::skill_dir(&ctx.env, Scope::Global, &ctx.cwd, name)?;
        if !force && fsops::is_present(&destination) {
            skipped += 1;
            output::warn(
                ctx.lang,
                &Msg::with_args(MsgKey::PushAllSkipped, [name.as_str()]),
            );
            continue;
        }

        match push_one(ctx, name, force) {
            Ok(()) => pushed += 1,
            Err(err) => {
                failed += 1;
                output::emit_stderr(&error_output::render(&err, ctx.lang));
            }
        }
    }

    output::emit_line(
        ctx.lang,
        &Msg::with_args(
            MsgKey::PushAllSummary,
            [pushed.to_string(), skipped.to_string(), failed.to_string()],
        ),
    );

    if failed > 0 {
        return Err(AppError::runtime(MsgKey::PushAllFailed)
            .with_args([failed.to_string()])
            .with_hint(MsgKey::PushAllFailedHint));
    }

    Ok(())
}

/// 扫描项目目录，逐条推送到 global；单条失败只记账，不中断。
fn push_all(ctx: &Ctx, force: bool) -> Result<(), AppError> {
    let project_dir = paths::skills_dir(&ctx.env, Scope::Project, &ctx.cwd)?;
    let entries = scan::scan_skills(&project_dir)?;

    if entries.is_empty() {
        output::emit_line(ctx.lang, &Msg::new(MsgKey::PushAllEmpty));
        return Ok(());
    }

    let mut pushed = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for entry in &entries {
        if !entry.valid {
            failed += 1;
            report_invalid(ctx, entry);
            continue;
        }

        let Ok(name) = SkillName::parse(&entry.name) else {
            failed += 1;
            report_invalid(ctx, entry);
            continue;
        };

        let destination = paths::skill_dir(&ctx.env, Scope::Global, &ctx.cwd, &name)?;
        if !force && fsops::is_present(&destination) {
            skipped += 1;
            output::warn(
                ctx.lang,
                &Msg::with_args(MsgKey::PushAllSkipped, [name.as_str()]),
            );
            continue;
        }

        match push_one(ctx, &name, force) {
            Ok(()) => pushed += 1,
            Err(err) => {
                failed += 1;
                output::emit_stderr(&error_output::render(&err, ctx.lang));
            }
        }
    }

    output::emit_line(
        ctx.lang,
        &Msg::with_args(
            MsgKey::PushAllSummary,
            [pushed.to_string(), skipped.to_string(), failed.to_string()],
        ),
    );

    if failed > 0 {
        return Err(AppError::runtime(MsgKey::PushAllFailed)
            .with_args([failed.to_string()])
            .with_hint(MsgKey::PushAllFailedHint));
    }

    Ok(())
}

fn report_invalid(ctx: &Ctx, entry: &SkillEntry) {
    let err = AppError::runtime(MsgKey::PushSourceInvalid)
        .with_args([entry.name.clone(), entry.error.clone().unwrap_or_default()])
        .with_hint(MsgKey::PushSourceInvalidHint);
    output::emit_stderr(&error_output::render(&err, ctx.lang));
}

/// 推送单个 skill：project → global。
fn push_one(ctx: &Ctx, name: &SkillName, force: bool) -> Result<(), AppError> {
    let project_dir = paths::skills_dir(&ctx.env, Scope::Project, &ctx.cwd)?;
    let source = project_dir.join(name.as_str());

    if !fsops::is_present(&source) {
        return Err(AppError::runtime(MsgKey::PushNotFound)
            .with_args([name.as_str()])
            .with_hint(MsgKey::PushNotFoundHint));
    }

    ensure_valid_source(&source, name)?;

    let destination = paths::skill_dir(&ctx.env, Scope::Global, &ctx.cwd, name)?;

    if fsops::is_same_path(&source, &destination)? {
        return Err(AppError::runtime(MsgKey::PushSamePath).with_hint(MsgKey::PushSamePathHint));
    }

    if fsops::contains_path(&source, &destination)? {
        return Err(AppError::runtime(MsgKey::AddNestedTarget)
            .with_hint_args(MsgKey::AddNestedTargetHint, [source.display().to_string()]));
    }

    if !force && fsops::is_present(&destination) {
        return Err(FsError::AlreadyExists {
            path: destination.clone(),
        }
        .into());
    }

    if fsops::contains_git_dir(&source) {
        output::warn(ctx.lang, &Msg::new(MsgKey::AddGitWarning));
    }

    let report = fsops::install_skill_dir(&source, &destination, force)?;

    if let Some(backup) = report.leftover_backup {
        output::warn(
            ctx.lang,
            &Msg::with_args(MsgKey::FsBackupLeftover, [backup.display().to_string()]),
        );
    }

    output::emit_line(
        ctx.lang,
        &Msg::with_args(
            MsgKey::PushPushed,
            [name.as_str().to_string(), destination.display().to_string()],
        ),
    );

    Ok(())
}

/// 源必须有合法 `SKILL.md`，且 frontmatter `name` 与目录名一致。
fn ensure_valid_source(source: &Path, name: &SkillName) -> Result<(), AppError> {
    let meta = frontmatter::parse_skill_md(&source.join("SKILL.md")).map_err(|err| {
        AppError::runtime(MsgKey::PushSourceInvalid)
            .with_args([name.as_str().to_string(), frontmatter::describe(&err)])
            .with_hint(MsgKey::PushSourceInvalidHint)
    })?;

    if meta.name != *name {
        let detail = format!("name mismatch: {} vs {}", name.as_str(), meta.name.as_str());
        return Err(AppError::runtime(MsgKey::PushSourceInvalid)
            .with_args([name.as_str().to_string(), detail])
            .with_hint(MsgKey::PushSourceInvalidHint));
    }

    Ok(())
}
