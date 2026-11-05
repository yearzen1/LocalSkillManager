//! `pull`：把全局 skill 复制到当前项目。

use crate::domain::action::TransferTarget;
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

use super::resolve::resolve_name;

pub fn run(ctx: &Ctx, target: TransferTarget, force: bool) -> Result<(), AppError> {
    match target {
        TransferTarget::Many(names) => pull_many(ctx, &names, force),
        TransferTarget::All => pull_all(ctx, force),
    }
}

/// 逐条复制多个 name；单条失败只记账，不中断（Q44）。
fn pull_many(ctx: &Ctx, inputs: &[SkillName], force: bool) -> Result<(), AppError> {
    let global_dir = paths::skills_dir(&ctx.env, Scope::Global, &ctx.cwd)?;
    let available = scan::dir_names(&global_dir)?;

    // 单 name 保持既有输出与退出码：目标已存在仍是错误。
    if let [input] = inputs {
        let name = resolve_name(input, &available, Scope::Global, pull_not_found(input))?;
        return pull_one(ctx, &name, force);
    }

    let mut names: Vec<SkillName> = Vec::new();
    let mut failed = 0usize;

    // 先解析全部 name；歧义或缺失按该条失败记账，其余照常执行。
    for input in inputs {
        match resolve_name(input, &available, Scope::Global, pull_not_found(input)) {
            Ok(name) if !names.contains(&name) => names.push(name),
            Ok(_) => {}
            Err(err) => {
                failed += 1;
                output::emit_stderr(&error_output::render_block(&err, ctx.lang));
            }
        }
    }

    let mut pulled = 0usize;
    let mut skipped = 0usize;

    for name in &names {
        let destination = paths::skill_dir(&ctx.env, Scope::Project, &ctx.cwd, name)?;
        if !force && fsops::is_present(&destination) {
            skipped += 1;
            output::warn(
                ctx.lang,
                &Msg::with_args(MsgKey::PullAllSkipped, [name.as_str()]),
            );
            continue;
        }

        match pull_one(ctx, name, force) {
            Ok(()) => pulled += 1,
            Err(err) => {
                failed += 1;
                output::emit_stderr(&error_output::render_block(&err, ctx.lang));
            }
        }
    }

    output::emit_line(
        ctx.lang,
        &Msg::with_args(
            MsgKey::PullAllSummary,
            [pulled.to_string(), skipped.to_string(), failed.to_string()],
        ),
    );

    if failed > 0 {
        output::emit_gap();
        return Err(AppError::runtime(MsgKey::PullBatchFailed)
            .with_args([failed.to_string()])
            .with_hint(MsgKey::PullBatchFailedHint));
    }

    Ok(())
}

/// 逐条复制全局目录下的全部 skill；单条失败只记账，不中断。
fn pull_all(ctx: &Ctx, force: bool) -> Result<(), AppError> {
    let global_dir = paths::skills_dir(&ctx.env, Scope::Global, &ctx.cwd)?;
    let entries = scan::scan_skills(&global_dir)?;

    if entries.is_empty() {
        output::emit_line(ctx.lang, &Msg::new(MsgKey::PullAllEmpty));
        return Ok(());
    }

    let mut pulled = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for entry in &entries {
        if !entry.valid {
            failed += 1;
            report_invalid(ctx, entry);
            continue;
        }

        let Ok(name) = SkillName::parse(&entry.name) else {
            // `scan` 已经校验过名字，这里只是兜底，不让单条脏数据中断整批。
            failed += 1;
            report_invalid(ctx, entry);
            continue;
        };

        let destination = paths::skill_dir(&ctx.env, Scope::Project, &ctx.cwd, &name)?;
        if !force && fsops::is_present(&destination) {
            skipped += 1;
            output::warn(
                ctx.lang,
                &Msg::with_args(MsgKey::PullAllSkipped, [name.as_str()]),
            );
            continue;
        }

        match pull_one(ctx, &name, force) {
            Ok(()) => pulled += 1,
            Err(err) => {
                failed += 1;
                output::emit_stderr(&error_output::render(&err, ctx.lang));
            }
        }
    }

    output::emit_line(
        ctx.lang,
        &Msg::with_args(
            MsgKey::PullAllSummary,
            [pulled.to_string(), skipped.to_string(), failed.to_string()],
        ),
    );

    if failed > 0 {
        output::emit_gap();
        return Err(AppError::runtime(MsgKey::PullAllFailed)
            .with_args([failed.to_string()])
            .with_hint(MsgKey::PullAllFailedHint));
    }

    Ok(())
}

fn report_invalid(ctx: &Ctx, entry: &SkillEntry) {
    let err = AppError::runtime(MsgKey::PullSourceInvalid)
        .with_args([entry.name.clone(), entry.error.clone().unwrap_or_default()])
        .with_hint(MsgKey::PullSourceInvalidHint);
    output::emit_stderr(&error_output::render_block(&err, ctx.lang));
}

/// `pull` 的 not-found 错误；hint 指向 `search`（Q47）。
fn pull_not_found(name: &SkillName) -> AppError {
    AppError::runtime(MsgKey::PullNotFound)
        .with_args([name.as_str()])
        .with_hint_args(MsgKey::PullNotFoundHint, [name.as_str()])
}

/// 复制单个 skill。
fn pull_one(ctx: &Ctx, name: &SkillName, force: bool) -> Result<(), AppError> {
    let global_dir = paths::skills_dir(&ctx.env, Scope::Global, &ctx.cwd)?;
    let source = global_dir.join(name.as_str());

    if !fsops::is_present(&source) {
        return Err(pull_not_found(name));
    }

    ensure_valid_source(&source, name)?;

    let destination = paths::skill_dir(&ctx.env, Scope::Project, &ctx.cwd, name)?;

    if fsops::is_same_path(&source, &destination)? {
        return Err(AppError::runtime(MsgKey::PullSamePath).with_hint(MsgKey::PullSamePathHint));
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
            MsgKey::PullCopied,
            [name.as_str().to_string(), destination.display().to_string()],
        ),
    );

    Ok(())
}

/// 源必须有合法 `SKILL.md`，且 frontmatter `name` 与目录名一致。
fn ensure_valid_source(source: &std::path::Path, name: &SkillName) -> Result<(), AppError> {
    let meta = frontmatter::parse_skill_md(&source.join("SKILL.md")).map_err(|err| {
        AppError::runtime(MsgKey::PullSourceInvalid)
            .with_args([name.as_str().to_string(), frontmatter::describe(&err)])
            .with_hint(MsgKey::PullSourceInvalidHint)
    })?;

    if meta.name != *name {
        let detail = format!("name mismatch: {} vs {}", name.as_str(), meta.name.as_str());
        return Err(AppError::runtime(MsgKey::PullSourceInvalid)
            .with_args([name.as_str().to_string(), detail])
            .with_hint(MsgKey::PullSourceInvalidHint));
    }

    Ok(())
}
