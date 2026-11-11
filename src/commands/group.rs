//! `group list` / `set` / `add` / `remove`：读写 `config.toml` 的 `[groups]`。

use crate::domain::action::GroupCommand;
use crate::domain::group::{Group, GroupName};
use crate::domain::name::SkillName;
use crate::error::{AppError, Msg};
use crate::i18n::{tr, MsgKey};
use crate::infra::config;
use crate::output;
use crate::output::error as error_output;
use crate::Ctx;

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
    let report = config::set_group(&ctx.env, name, members, force)?;
    emit_saved(ctx, name, &report);
    Ok(())
}

fn add(ctx: &Ctx, name: &GroupName, members: &[SkillName]) -> Result<(), AppError> {
    let report = config::add_group(&ctx.env, name, members)?;
    emit_saved(ctx, name, &report);
    Ok(())
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
    if let [name] = names {
        config::remove_group(&ctx.env, name)?;
        output::emit_line(
            ctx.lang,
            &Msg::with_args(MsgKey::GroupRemoved, [name.as_str()]),
        );
        return Ok(());
    }

    let mut removed = 0usize;
    let mut failed = 0usize;

    for name in names {
        match config::remove_group(&ctx.env, name) {
            Ok(_) => {
                removed += 1;
                output::emit_line(
                    ctx.lang,
                    &Msg::with_args(MsgKey::GroupRemoved, [name.as_str()]),
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
