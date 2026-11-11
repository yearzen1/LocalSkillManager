//! `pull` / `push` 共用的方向性复制流程。
//!
//! 两个命令只差源/目标作用域与消息键；批量语义、原子安装与错误排版必须保持一致。

use std::collections::HashSet;
use std::path::Path;

use crate::domain::action::TransferTarget;
use crate::domain::entry::SkillEntry;
use crate::domain::group::{expand, GroupName};
use crate::domain::lang::Lang;
use crate::domain::name::SkillName;
use crate::domain::scope::Scope;
use crate::error::{AppError, Msg};
use crate::i18n::MsgKey;
use crate::infra::fsops::{self, FsError};
use crate::infra::{frontmatter, groups, paths, scan};
use crate::output;
use crate::output::error as error_output;
use crate::Ctx;

use super::resolve::{resolve_group, resolve_name};

/// 一个方向的全部可变部分：作用域与消息键。
pub(super) struct TransferSpec {
    pub(super) source: Scope,
    pub(super) destination: Scope,
    pub(super) empty: MsgKey,
    pub(super) skipped: MsgKey,
    pub(super) summary: MsgKey,
    pub(super) many_failed: MsgKey,
    pub(super) many_failed_hint: MsgKey,
    pub(super) all_failed: MsgKey,
    pub(super) all_failed_hint: MsgKey,
    pub(super) done: MsgKey,
    pub(super) not_found: MsgKey,
    pub(super) not_found_hint: MsgKey,
    pub(super) invalid: MsgKey,
    pub(super) invalid_hint: MsgKey,
    pub(super) same_path: MsgKey,
    pub(super) same_path_hint: MsgKey,
}

pub(super) const PULL: TransferSpec = TransferSpec {
    source: Scope::Global,
    destination: Scope::Project,
    empty: MsgKey::PullAllEmpty,
    skipped: MsgKey::PullAllSkipped,
    summary: MsgKey::PullAllSummary,
    many_failed: MsgKey::PullBatchFailed,
    many_failed_hint: MsgKey::PullBatchFailedHint,
    all_failed: MsgKey::PullAllFailed,
    all_failed_hint: MsgKey::PullAllFailedHint,
    done: MsgKey::PullCopied,
    not_found: MsgKey::PullNotFound,
    not_found_hint: MsgKey::PullNotFoundHint,
    invalid: MsgKey::PullSourceInvalid,
    invalid_hint: MsgKey::PullSourceInvalidHint,
    same_path: MsgKey::PullSamePath,
    same_path_hint: MsgKey::PullSamePathHint,
};

pub(super) const PUSH: TransferSpec = TransferSpec {
    source: Scope::Project,
    destination: Scope::Global,
    empty: MsgKey::PushAllEmpty,
    skipped: MsgKey::PushAllSkipped,
    summary: MsgKey::PushAllSummary,
    many_failed: MsgKey::PushAllFailed,
    many_failed_hint: MsgKey::PushAllFailedHint,
    all_failed: MsgKey::PushAllFailed,
    all_failed_hint: MsgKey::PushAllFailedHint,
    done: MsgKey::PushPushed,
    not_found: MsgKey::PushNotFound,
    not_found_hint: MsgKey::PushNotFoundHint,
    invalid: MsgKey::PushSourceInvalid,
    invalid_hint: MsgKey::PushSourceInvalidHint,
    same_path: MsgKey::PushSamePath,
    same_path_hint: MsgKey::PushSamePathHint,
};

/// 按方向执行一次 `pull` / `push`。
pub(super) fn run(
    ctx: &Ctx,
    spec: &TransferSpec,
    target: TransferTarget,
    force: bool,
) -> Result<(), AppError> {
    let source_dir = paths::skills_dir(&ctx.env, spec.source, &ctx.cwd)?;
    let destination_dir = paths::skills_dir(&ctx.env, spec.destination, &ctx.cwd)?;

    match target {
        TransferTarget::Many(names) => {
            transfer_many(ctx, spec, &source_dir, &destination_dir, &names, force)
        }
        TransferTarget::Groups(groups) => {
            transfer_groups(ctx, spec, &source_dir, &destination_dir, &groups, force)
        }
        TransferTarget::All => transfer_all(ctx, spec, &source_dir, &destination_dir, force),
    }
}

/// 展开 `--group` 后复用多 name 的逐条流程（Q55）。
fn transfer_groups(
    ctx: &Ctx,
    spec: &TransferSpec,
    source_dir: &Path,
    destination_dir: &Path,
    groups: &[GroupName],
    force: bool,
) -> Result<(), AppError> {
    let definitions = groups::load_groups(&ctx.env)?;
    let available = scan::dir_names(source_dir)?;
    let group_names: Vec<String> = definitions
        .iter()
        .map(|group| group.name.as_str().to_string())
        .collect();

    let mut names: Vec<SkillName> = Vec::new();
    let mut seen = HashSet::new();
    let mut failed = 0usize;

    for input in groups {
        let resolved = match resolve_group(input, &group_names) {
            Ok(name) => name,
            Err(err) => {
                failed += 1;
                output::emit_stderr(&error_output::render_block(&err, ctx.lang));
                continue;
            }
        };
        let skills = match expand(&definitions, &resolved) {
            Ok(skills) => skills,
            Err(err) => {
                failed += 1;
                output::emit_stderr(&error_output::render_block(&err.into(), ctx.lang));
                continue;
            }
        };

        if skills.is_empty() {
            failed += 1;
            let err = AppError::runtime(MsgKey::GroupEmpty)
                .with_args([resolved.as_str()])
                .with_hint_args(MsgKey::GroupEmptyHint, [resolved.as_str()]);
            output::emit_stderr(&error_output::render_block(&err, ctx.lang));
            continue;
        }

        for skill in &skills {
            match resolve_name(skill, &available, spec.source, not_found(spec, skill)) {
                Ok(name) if seen.insert(name.clone()) => names.push(name),
                Ok(_) => {}
                Err(err) => {
                    failed += 1;
                    output::emit_stderr(&error_output::render_block(&err, ctx.lang));
                }
            }
        }
    }

    let mut done = 0usize;
    let mut skipped = 0usize;

    for name in &names {
        if !force && fsops::is_present(&destination_dir.join(name.as_str())) {
            skipped += 1;
            output::warn(ctx.lang, &Msg::with_args(spec.skipped, [name.as_str()]));
            continue;
        }

        match transfer_one(ctx, spec, source_dir, destination_dir, name, force) {
            Ok(()) => done += 1,
            Err(err) => {
                failed += 1;
                output::emit_stderr(&error_output::render_block(&err, ctx.lang));
            }
        }
    }

    emit_summary(ctx, spec, done, skipped, failed);
    finish_failed(spec.many_failed, spec.many_failed_hint, failed)
}

/// 逐条复制多个 name；单条失败只记账，不中断（Q44）。
fn transfer_many(
    ctx: &Ctx,
    spec: &TransferSpec,
    source_dir: &Path,
    destination_dir: &Path,
    inputs: &[SkillName],
    force: bool,
) -> Result<(), AppError> {
    let available = scan::dir_names(source_dir)?;

    // 单 name 保持既有输出与退出码：目标已存在仍是错误。
    if let [input] = inputs {
        let name = resolve_name(input, &available, spec.source, not_found(spec, input))?;
        return transfer_one(ctx, spec, source_dir, destination_dir, &name, force);
    }

    let mut names: Vec<SkillName> = Vec::new();
    let mut seen = HashSet::new();
    let mut failed = 0usize;

    // 先解析全部 name；歧义或缺失按该条失败记账，其余照常执行。
    for input in inputs {
        match resolve_name(input, &available, spec.source, not_found(spec, input)) {
            Ok(name) if seen.insert(name.clone()) => names.push(name),
            Ok(_) => {}
            Err(err) => {
                failed += 1;
                output::emit_stderr(&error_output::render_block(&err, ctx.lang));
            }
        }
    }

    let mut done = 0usize;
    let mut skipped = 0usize;

    for name in &names {
        if !force && fsops::is_present(&destination_dir.join(name.as_str())) {
            skipped += 1;
            output::warn(ctx.lang, &Msg::with_args(spec.skipped, [name.as_str()]));
            continue;
        }

        match transfer_one(ctx, spec, source_dir, destination_dir, name, force) {
            Ok(()) => done += 1,
            Err(err) => {
                failed += 1;
                output::emit_stderr(&error_output::render_block(&err, ctx.lang));
            }
        }
    }

    emit_summary(ctx, spec, done, skipped, failed);
    finish_failed(spec.many_failed, spec.many_failed_hint, failed)
}

/// 扫描源作用域并逐条复制；单条失败只记账，不中断。
fn transfer_all(
    ctx: &Ctx,
    spec: &TransferSpec,
    source_dir: &Path,
    destination_dir: &Path,
    force: bool,
) -> Result<(), AppError> {
    let entries = scan::scan_skills(source_dir)?;

    if entries.is_empty() {
        output::emit_line(ctx.lang, &Msg::new(spec.empty));
        return Ok(());
    }

    let mut done = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for entry in &entries {
        if !entry.valid {
            failed += 1;
            report_invalid(ctx, spec, entry);
            continue;
        }

        let Ok(name) = SkillName::parse(&entry.name) else {
            // `scan` 已经校验过名字，这里只是兜底，不让单条脏数据中断整批。
            failed += 1;
            report_invalid(ctx, spec, entry);
            continue;
        };

        if !force && fsops::is_present(&destination_dir.join(name.as_str())) {
            skipped += 1;
            output::warn(ctx.lang, &Msg::with_args(spec.skipped, [name.as_str()]));
            continue;
        }

        match transfer_one(ctx, spec, source_dir, destination_dir, &name, force) {
            Ok(()) => done += 1,
            Err(err) => {
                failed += 1;
                output::emit_stderr(&error_output::render_block(&err, ctx.lang));
            }
        }
    }

    emit_summary(ctx, spec, done, skipped, failed);
    finish_failed(spec.all_failed, spec.all_failed_hint, failed)
}

fn emit_summary(ctx: &Ctx, spec: &TransferSpec, done: usize, skipped: usize, failed: usize) {
    output::emit_line(
        ctx.lang,
        &Msg::with_args(
            spec.summary,
            [done.to_string(), skipped.to_string(), failed.to_string()],
        ),
    );
}

/// 有失败时先留一个空行，再把批次总错误交给调用方渲染。
fn finish_failed(key: MsgKey, hint: MsgKey, failed: usize) -> Result<(), AppError> {
    if failed == 0 {
        return Ok(());
    }

    output::emit_gap();
    Err(AppError::runtime(key)
        .with_args([failed.to_string()])
        .with_hint(hint))
}

fn report_invalid(ctx: &Ctx, spec: &TransferSpec, entry: &SkillEntry) {
    let err = AppError::runtime(spec.invalid)
        .with_args([entry.name.clone(), entry.error.clone().unwrap_or_default()])
        .with_hint(spec.invalid_hint);
    output::emit_stderr(&error_output::render_block(&err, ctx.lang));
}

/// 源作用域内的 not-found 错误；hint 由方向决定。
fn not_found(spec: &TransferSpec, name: &SkillName) -> AppError {
    AppError::runtime(spec.not_found)
        .with_args([name.as_str()])
        .with_hint_args(spec.not_found_hint, [name.as_str()])
}

/// 复制单个 skill：`spec.source` → `spec.destination`。
fn transfer_one(
    ctx: &Ctx,
    spec: &TransferSpec,
    source_dir: &Path,
    destination_dir: &Path,
    name: &SkillName,
    force: bool,
) -> Result<(), AppError> {
    let source = source_dir.join(name.as_str());

    if !fsops::is_present(&source) {
        return Err(not_found(spec, name));
    }

    ensure_valid_source(ctx.lang, spec, &source, name)?;

    let destination = destination_dir.join(name.as_str());

    if fsops::is_same_path(&source, &destination)? {
        return Err(AppError::runtime(spec.same_path).with_hint(spec.same_path_hint));
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
            spec.done,
            [name.as_str().to_string(), destination.display().to_string()],
        ),
    );

    Ok(())
}

/// 源必须有合法 `SKILL.md`，且 frontmatter `name` 与目录名一致。
fn ensure_valid_source(
    lang: Lang,
    spec: &TransferSpec,
    source: &Path,
    name: &SkillName,
) -> Result<(), AppError> {
    let meta = frontmatter::parse_skill_md(&source.join("SKILL.md")).map_err(|err| {
        AppError::runtime(spec.invalid)
            .with_args([name.as_str().to_string(), frontmatter::describe(&err)])
            .with_hint(spec.invalid_hint)
    })?;

    if meta.name != *name {
        let detail = output::line(
            lang,
            &Msg::with_args(
                MsgKey::SkillDirMismatch,
                [name.as_str(), meta.name.as_str()],
            ),
        );
        return Err(AppError::runtime(spec.invalid)
            .with_args([name.as_str().to_string(), detail])
            .with_hint(spec.invalid_hint));
    }

    Ok(())
}
