//! `copy`：把全局 skill 复制到当前项目。

use crate::domain::name::SkillName;
use crate::domain::scope::Scope;
use crate::error::{AppError, Msg};
use crate::i18n::MsgKey;
use crate::infra::fsops::{self, FsError};
use crate::infra::{frontmatter, paths};
use crate::output;
use crate::Ctx;

pub fn run(ctx: &Ctx, name: SkillName, force: bool) -> Result<(), AppError> {
    let global_dir = paths::skills_dir(&ctx.env, Scope::Global, &ctx.cwd)?;
    let source = global_dir.join(name.as_str());

    if !fsops::is_present(&source) {
        return Err(AppError::runtime(MsgKey::CopyNotFound)
            .with_args([name.as_str()])
            .with_hint(MsgKey::CopyNotFoundHint));
    }

    ensure_valid_source(&source, &name)?;

    let destination = paths::skill_dir(&ctx.env, Scope::Project, &ctx.cwd, &name)?;

    if fsops::is_same_path(&source, &destination)? {
        return Err(AppError::runtime(MsgKey::CopySamePath).with_hint(MsgKey::CopySamePathHint));
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
            MsgKey::CopyCopied,
            [name.as_str().to_string(), destination.display().to_string()],
        ),
    );

    Ok(())
}

/// 源必须有合法 `SKILL.md`，且 frontmatter `name` 与目录名一致。
fn ensure_valid_source(source: &std::path::Path, name: &SkillName) -> Result<(), AppError> {
    let meta = frontmatter::parse_skill_md(&source.join("SKILL.md")).map_err(|err| {
        AppError::runtime(MsgKey::CopySourceInvalid)
            .with_args([name.as_str().to_string(), frontmatter::describe(&err)])
            .with_hint(MsgKey::CopySourceInvalidHint)
    })?;

    if meta.name != *name {
        let detail = format!("name mismatch: {} vs {}", name.as_str(), meta.name.as_str());
        return Err(AppError::runtime(MsgKey::CopySourceInvalid)
            .with_args([name.as_str().to_string(), detail])
            .with_hint(MsgKey::CopySourceInvalidHint));
    }

    Ok(())
}
