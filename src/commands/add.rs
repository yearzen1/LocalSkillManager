//! `add`：把 `.md` 文件或 skill 目录安装到 global / project。

use std::path::{Path, PathBuf};

use crate::domain::meta::SkillMeta;
use crate::domain::scope::Scope;
use crate::error::{AppError, Msg};
use crate::i18n::MsgKey;
use crate::infra::frontmatter;
use crate::infra::fsops::{self, SourceKind};
use crate::infra::paths;
use crate::output;
use crate::Ctx;

pub fn run(ctx: &Ctx, source: PathBuf, scope: Scope, force: bool) -> Result<(), AppError> {
    let source = absolute(&ctx.cwd, source);
    let kind = fsops::inspect_source(&source)?;
    let meta = load_meta(kind, &source)?;

    let destination = paths::skill_dir(&ctx.env, scope, &ctx.cwd, &meta.name)?;
    reject_nested(&source, &destination)?;

    if !force && fsops::is_present(&destination) {
        return Err(crate::infra::fsops::FsError::AlreadyExists {
            path: destination.clone(),
        }
        .into());
    }

    if kind == SourceKind::Directory && fsops::contains_git_dir(&source) {
        warn(ctx, Msg::new(MsgKey::AddGitWarning));
    }

    let report = match kind {
        SourceKind::File => fsops::install_skill_file(&source, &destination, force)?,
        SourceKind::Directory => fsops::install_skill_dir(&source, &destination, force)?,
    };

    if let Some(backup) = report.leftover_backup {
        warn(
            ctx,
            Msg::with_args(MsgKey::FsBackupLeftover, [backup.display().to_string()]),
        );
    }

    output::emit_line(
        ctx.lang,
        &Msg::with_args(
            MsgKey::AddInstalled,
            [
                meta.name.as_str().to_string(),
                destination.display().to_string(),
            ],
        ),
    );

    Ok(())
}

fn load_meta(kind: SourceKind, source: &Path) -> Result<SkillMeta, AppError> {
    match kind {
        SourceKind::File => {
            if !has_markdown_extension(source) {
                return Err(AppError::runtime(MsgKey::AddUnsupportedSource)
                    .with_args([source.display().to_string()]));
            }
            Ok(frontmatter::parse_skill_md(source)?)
        }
        SourceKind::Directory => {
            let meta = frontmatter::parse_skill_md(&source.join("SKILL.md"))?;
            let directory = directory_name(source);

            if directory != meta.name.as_str() {
                return Err(AppError::runtime(MsgKey::SkillDirMismatch)
                    .with_args([directory, meta.name.as_str().to_string()])
                    .with_hint_args(
                        MsgKey::SkillDirMismatchHint,
                        [meta.name.as_str().to_string()],
                    ));
            }

            Ok(meta)
        }
    }
}

/// 目标位于源内部时硬报错，典型触发：`lsm add . project`。
fn reject_nested(source: &Path, destination: &Path) -> Result<(), AppError> {
    if !fsops::contains_path(source, destination)? {
        return Ok(());
    }

    let scope_dir = destination
        .parent()
        .and_then(Path::parent)
        .map(|path| path.display().to_string())
        .unwrap_or_default();

    Err(AppError::runtime(MsgKey::AddNestedTarget)
        .with_hint_args(MsgKey::AddNestedTargetHint, [scope_dir]))
}

fn warn(ctx: &Ctx, message: Msg) {
    output::emit_stderr(&format!(
        "{}\n",
        output::prefixed_line(ctx.lang, MsgKey::WarningPrefix, &message)
    ));
}

fn absolute(cwd: &Path, source: PathBuf) -> PathBuf {
    if source.is_absolute() {
        source
    } else {
        cwd.join(source)
    }
}

fn has_markdown_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

fn directory_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}
