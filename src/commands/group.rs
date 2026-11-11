//! `group list` / `set` / `add` / `remove`：读写 `config.toml` 的 `[groups]`。

use std::collections::HashSet;

use crate::domain::action::GroupCommand;
use crate::domain::group::{Group, GroupName};
use crate::domain::name::SkillName;
use crate::domain::resolve as name_resolve;
use crate::domain::resolve::Match;
use crate::domain::scope::Scope;
use crate::error::{AppError, Msg};
use crate::i18n::{tr, MsgKey};
use crate::infra::{config, paths, scan};
use crate::output;
use crate::output::error as error_output;
use crate::Ctx;

use super::resolve::resolve_group;

pub fn run(ctx: &Ctx, command: GroupCommand) -> Result<(), AppError> {
    match command {
        GroupCommand::List { json } => list(ctx, json),
        GroupCommand::Set {
            name,
            members,
            force,
        } => set(ctx, &name, &members, force),
        GroupCommand::Add { name, members } => add(ctx, &name, &members),
        GroupCommand::Remove { names } => remove(ctx, &names),
    }
}

fn list(ctx: &Ctx, json: bool) -> Result<(), AppError> {
    let groups = config::load_groups(&ctx.env)?;

    if json {
        output::emit_stdout(&format!("{}\n", render_json(&groups)));
    } else if groups.is_empty() {
        output::emit_line(ctx.lang, &Msg::new(MsgKey::GroupNoGroups));
    } else {
        for group in &groups {
            let members = if group.members.is_empty() {
                tr(ctx.lang, MsgKey::GroupEmptyMembers).to_string()
            } else {
                group
                    .members
                    .iter()
                    .map(SkillName::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            output::emit_line(
                ctx.lang,
                &Msg::with_args(MsgKey::GroupLine, [group.name.as_str(), members.as_str()]),
            );
        }
    }

    Ok(())
}

fn set(ctx: &Ctx, name: &GroupName, members: &[SkillName], force: bool) -> Result<(), AppError> {
    let members = resolve_members(ctx, members)?;
    let report = config::set_group(&ctx.env, name, &members, force)?;
    emit_saved(ctx, name, &report);
    Ok(())
}

fn add(ctx: &Ctx, name: &GroupName, members: &[SkillName]) -> Result<(), AppError> {
    let members = resolve_members(ctx, members)?;
    let report = config::add_group(&ctx.env, name, &members)?;
    emit_saved(ctx, name, &report);
    Ok(())
}

/// 把成员解析成 global 中实际存在的全名；任一失败则整批不写盘（Q58）。
fn resolve_members(ctx: &Ctx, members: &[SkillName]) -> Result<Vec<SkillName>, AppError> {
    let directory = paths::skills_dir(&ctx.env, Scope::Global, &ctx.cwd)?;
    let available = scan::dir_names(&directory)?;

    let mut resolved: Vec<SkillName> = Vec::new();
    let mut seen = HashSet::new();
    let mut errors: Vec<AppError> = Vec::new();

    for member in members {
        match name_resolve::match_name(member.as_str(), &available) {
            Match::Found(name) => {
                // `scan::dir_names` 只返回合法名字，这里不会失败。
                let name = SkillName::parse(&name).expect("scan::dir_names filters invalid names");
                if seen.insert(name.clone()) {
                    resolved.push(name);
                }
            }
            Match::Ambiguous(candidates) => errors.push(
                AppError::runtime(MsgKey::NameAmbiguous)
                    .with_args([
                        member.as_str(),
                        Scope::Global.label(),
                        candidates.join(", ").as_str(),
                    ])
                    .with_hint_args(MsgKey::NameAmbiguousHint, [Scope::Global.label()]),
            ),
            Match::Missing => {
                let err =
                    AppError::runtime(MsgKey::GroupMemberMissing).with_args([member.as_str()]);
                let candidates = name_resolve::suggestions(member.as_str(), &available, 3);
                errors.push(if candidates.is_empty() {
                    err.with_hint(MsgKey::GroupMemberMissingHint)
                } else {
                    err.with_hint_args(MsgKey::NameSuggestion, [candidates.join(", ")])
                });
            }
        }
    }

    if errors.is_empty() {
        return Ok(resolved);
    }

    for err in &errors {
        output::emit_stderr(&error_output::render_block(err, ctx.lang));
    }
    output::emit_gap();

    Err(AppError::runtime(MsgKey::GroupMembersFailed)
        .with_args([errors.len().to_string()])
        .with_hint(MsgKey::GroupMembersFailedHint))
}

fn emit_saved(ctx: &Ctx, name: &GroupName, report: &config::GroupSaveReport) {
    output::emit_line(
        ctx.lang,
        &Msg::with_args(
            MsgKey::GroupSaved,
            [
                name.as_str().to_string(),
                report.members.to_string(),
                report.path.display().to_string(),
            ],
        ),
    );
}

fn remove(ctx: &Ctx, names: &[GroupName]) -> Result<(), AppError> {
    let available = group_names(ctx)?;

    if let [name] = names {
        let name = resolve_group(name, &available)?;
        config::remove_group(&ctx.env, &name)?;
        output::emit_line(
            ctx.lang,
            &Msg::with_args(MsgKey::GroupRemoved, [name.as_str()]),
        );
        return Ok(());
    }

    let mut removed = 0usize;
    let mut failed = 0usize;

    for name in names {
        let resolved = match resolve_group(name, &available) {
            Ok(resolved) => resolved,
            Err(err) => {
                failed += 1;
                output::emit_stderr(&error_output::render_block(&err, ctx.lang));
                continue;
            }
        };

        match config::remove_group(&ctx.env, &resolved) {
            Ok(_) => {
                removed += 1;
                output::emit_line(
                    ctx.lang,
                    &Msg::with_args(MsgKey::GroupRemoved, [resolved.as_str()]),
                );
            }
            Err(err) => {
                failed += 1;
                let app: AppError = err.into();
                output::emit_stderr(&error_output::render_block(&app, ctx.lang));
            }
        }
    }

    output::emit_line(
        ctx.lang,
        &Msg::with_args(
            MsgKey::GroupBatchSummary,
            [removed.to_string(), failed.to_string()],
        ),
    );

    if failed > 0 {
        output::emit_gap();
        return Err(AppError::runtime(MsgKey::GroupBatchFailed)
            .with_args([failed.to_string()])
            .with_hint(MsgKey::GroupBatchFailedHint));
    }

    Ok(())
}

/// 已定义分组的组名列表，供唯一前缀解析使用。
fn group_names(ctx: &Ctx) -> Result<Vec<String>, AppError> {
    let groups = config::load_groups(&ctx.env)?;
    Ok(groups
        .iter()
        .map(|group| group.name.as_str().to_string())
        .collect())
}

/// JSON 字段固定英文，不参与 i18n；空组输出 `[]`。
fn render_json(groups: &[Group]) -> String {
    if groups.is_empty() {
        return "[]".to_string();
    }

    let rows: Vec<JsonGroup> = groups
        .iter()
        .map(|group| JsonGroup {
            name: group.name.as_str(),
            skills: group.members.iter().map(SkillName::as_str).collect(),
        })
        .collect();

    serde_json::to_string_pretty(&rows).expect("serializing plain groups cannot fail")
}

#[derive(serde::Serialize)]
struct JsonGroup<'a> {
    name: &'a str,
    skills: Vec<&'a str>,
}
