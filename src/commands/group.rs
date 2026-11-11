//! `group list` / `set` / `add` / `remove` / `delete`：读写 `groups.toml` 的顶层表。

use std::collections::HashSet;
use std::path::Path;

use crate::domain::action::GroupCommand;
use crate::domain::group::{expand, find_path, Group, GroupName, Member};
use crate::domain::name::SkillName;
use crate::domain::resolve as name_resolve;
use crate::domain::resolve::Match;
use crate::domain::scope::Scope;
use crate::error::{AppError, Msg};
use crate::i18n::{tr, MsgKey};
use crate::infra::{groups as group_store, paths, scan};
use crate::output;
use crate::output::error as error_output;
use crate::Ctx;

use super::resolve::resolve_group;

pub fn run(ctx: &Ctx, command: GroupCommand) -> Result<(), AppError> {
    match command {
        GroupCommand::List { json, resolved } => list(ctx, json, resolved),
        GroupCommand::Set {
            name,
            members,
            force,
        } => set(ctx, &name, &members, force),
        GroupCommand::Add { name, members } => add(ctx, &name, &members),
        GroupCommand::Remove { name, members } => remove_members(ctx, &name, &members),
        GroupCommand::Delete { names, force } => delete(ctx, &names, force),
    }
}

fn list(ctx: &Ctx, json: bool, resolved: bool) -> Result<(), AppError> {
    let groups = group_store::load_groups(&ctx.env)?;

    if resolved {
        let mut expanded = Vec::with_capacity(groups.len());
        let mut failed = 0usize;

        for group in &groups {
            match expand(&groups, &group.name) {
                Ok(skills) => expanded.push(Some(skills)),
                Err(err) => {
                    failed += 1;
                    expanded.push(None);
                    output::emit_stderr(&error_output::render_block(&err.into(), ctx.lang));
                }
            }
        }

        if failed > 0 {
            output::emit_gap();
            return Err(AppError::runtime(MsgKey::GroupListFailed)
                .with_args([failed.to_string()])
                .with_hint(MsgKey::GroupListFailedHint));
        }

        if json {
            output::emit_stdout(&format!("{}\n", render_json(&groups, Some(&expanded))));
            return Ok(());
        }

        if groups.is_empty() {
            output::emit_line(ctx.lang, &Msg::new(MsgKey::GroupNoGroups));
            return Ok(());
        }

        for (group, skills) in groups.iter().zip(expanded.iter()) {
            let members = match skills.as_ref().map(|skills| render_skills(skills)) {
                Some(members) if !members.is_empty() => members,
                _ => tr(ctx.lang, MsgKey::GroupEmptyMembers).to_string(),
            };
            output::emit_line(
                ctx.lang,
                &Msg::with_args(MsgKey::GroupLine, [group.name.as_str(), members.as_str()]),
            );
        }
        return Ok(());
    }

    if json {
        output::emit_stdout(&format!("{}\n", render_json(&groups, None)));
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
                    .map(ToString::to_string)
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

fn set(ctx: &Ctx, name: &GroupName, members: &[Member], force: bool) -> Result<(), AppError> {
    let groups = group_store::load_groups(&ctx.env)?;
    let members = resolve_members(ctx, name, &groups, members)?;
    let report = group_store::set_group(&ctx.env, name, &members, force)?;
    warn_legacy(ctx, report.legacy_leftover.as_deref());
    emit_saved(ctx, name, &report);
    Ok(())
}

fn add(ctx: &Ctx, name: &GroupName, members: &[Member]) -> Result<(), AppError> {
    let groups = group_store::load_groups(&ctx.env)?;
    let members = resolve_members(ctx, name, &groups, members)?;
    let report = group_store::add_group(&ctx.env, name, &members)?;
    warn_legacy(ctx, report.legacy_leftover.as_deref());
    emit_saved(ctx, name, &report);
    Ok(())
}

/// 把成员解析成 global 中的全名或已定义组；任一失败则整批不写盘（Q60）。
fn resolve_members(
    ctx: &Ctx,
    target: &GroupName,
    groups: &[Group],
    members: &[Member],
) -> Result<Vec<Member>, AppError> {
    let directory = paths::skills_dir(&ctx.env, Scope::Global, &ctx.cwd)?;
    let available_skills = scan::dir_names(&directory)?;
    let available_groups = group_names(groups);

    let mut resolved: Vec<Member> = Vec::new();
    let mut seen = HashSet::new();
    let mut errors: Vec<AppError> = Vec::new();

    for member in members {
        let result = match member {
            Member::Skill(skill) => resolve_skill(skill, &available_skills),
            Member::Group(reference) => {
                resolve_reference(target, groups, &available_groups, reference)
            }
        };

        match result {
            Ok(member) => {
                if seen.insert(member.clone()) {
                    resolved.push(member);
                }
            }
            Err(err) => errors.push(err),
        }
    }

    finish_items(
        ctx,
        errors,
        MsgKey::GroupSetFailed,
        MsgKey::GroupSetFailedHint,
    )
    .map(|()| resolved)
}

fn resolve_skill(skill: &SkillName, available: &[String]) -> Result<Member, AppError> {
    match name_resolve::match_name(skill.as_str(), available) {
        Match::Found(name) => {
            // `scan::dir_names` 只返回合法名字，这里不会失败。
            let name = SkillName::parse(&name).expect("scan::dir_names filters invalid names");
            Ok(Member::Skill(name))
        }
        Match::Ambiguous(candidates) => Err(AppError::runtime(MsgKey::NameAmbiguous)
            .with_args([
                skill.as_str(),
                Scope::Global.label(),
                candidates.join(", ").as_str(),
            ])
            .with_hint_args(MsgKey::NameAmbiguousHint, [Scope::Global.label()])),
        Match::Missing => {
            let err = AppError::runtime(MsgKey::GroupMemberMissing).with_args([skill.as_str()]);
            let candidates = name_resolve::suggestions(skill.as_str(), available, 3);
            Err(if candidates.is_empty() {
                err.with_hint(MsgKey::GroupMemberMissingHint)
            } else {
                err.with_hint_args(MsgKey::NameSuggestion, [candidates.join(", ")])
            })
        }
    }
}

fn resolve_reference(
    target: &GroupName,
    groups: &[Group],
    available: &[String],
    reference: &GroupName,
) -> Result<Member, AppError> {
    let resolved = resolve_group(reference, available)?;

    if resolved == *target {
        return Err(AppError::runtime(MsgKey::GroupSelfReference).with_args([target.as_str()]));
    }

    if let Some(mut path) = find_path(groups, &resolved, target) {
        let mut cycle = vec![target.clone()];
        cycle.append(&mut path);
        let chain = cycle
            .iter()
            .map(GroupName::as_str)
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(AppError::runtime(MsgKey::GroupCycle)
            .with_args([chain])
            .with_hint(MsgKey::GroupCycleHint));
    }

    Ok(Member::Group(resolved))
}

fn remove_members(ctx: &Ctx, name: &GroupName, members: &[Member]) -> Result<(), AppError> {
    let groups = group_store::load_groups(&ctx.env)?;
    let available = group_names(&groups);
    let name = resolve_group(name, &available)?;
    let definition = groups
        .iter()
        .find(|group| group.name == name)
        .expect("resolve_group returns a defined group name");

    let available_skills: Vec<String> = definition
        .members
        .iter()
        .filter_map(|member| member.skill().map(SkillName::as_str))
        .map(str::to_string)
        .collect();
    let available_refs: Vec<String> = definition
        .members
        .iter()
        .filter_map(|member| member.group().map(GroupName::as_str))
        .map(str::to_string)
        .collect();

    let mut removals: Vec<Member> = Vec::new();
    let mut errors: Vec<AppError> = Vec::new();

    for member in members {
        let result = match member {
            Member::Skill(skill) => {
                match name_resolve::match_name(skill.as_str(), &available_skills) {
                    Match::Found(found) => Ok(Member::Skill(
                        SkillName::parse(&found).expect("group members are validated"),
                    )),
                    Match::Ambiguous(candidates) => Err(AppError::runtime(MsgKey::GroupAmbiguous)
                        .with_args([skill.as_str(), candidates.join(", ").as_str()])
                        .with_hint(MsgKey::GroupAmbiguousHint)),
                    Match::Missing => {
                        Err(missing_in_group(skill.as_str(), &name, &available_skills))
                    }
                }
            }
            Member::Group(reference) => {
                match name_resolve::match_name(reference.as_str(), &available_refs) {
                    Match::Found(found) => Ok(Member::Group(
                        GroupName::parse(&found).expect("group references are validated"),
                    )),
                    Match::Ambiguous(candidates) => Err(AppError::runtime(MsgKey::GroupAmbiguous)
                        .with_args([reference.as_str(), candidates.join(", ").as_str()])
                        .with_hint(MsgKey::GroupAmbiguousHint)),
                    Match::Missing => {
                        let display = format!("@{}", reference.as_str());
                        Err(missing_in_group(&display, &name, &available_refs))
                    }
                }
            }
        };

        match result {
            Ok(member) => {
                if !removals.contains(&member) {
                    removals.push(member);
                }
            }
            Err(err) => errors.push(err),
        }
    }

    if errors.len() == 1 {
        return Err(errors.into_iter().next().expect("checked non-empty"));
    }
    if !errors.is_empty() {
        emit_errors(ctx, &errors);
        return Err(AppError::runtime(MsgKey::GroupRemoveFailed)
            .with_args([errors.len().to_string(), name.as_str().to_string()])
            .with_hint(MsgKey::GroupRemoveFailedHint));
    }

    let remaining: Vec<Member> = definition
        .members
        .iter()
        .filter(|member| !removals.contains(member))
        .cloned()
        .collect();
    let report = group_store::replace_group_members(&ctx.env, &name, &remaining)?;
    warn_legacy(ctx, report.legacy_leftover.as_deref());

    for member in &removals {
        output::emit_line(
            ctx.lang,
            &Msg::with_args(
                MsgKey::GroupMemberRemoved,
                [member.to_string(), name.as_str().to_string()],
            ),
        );
    }

    Ok(())
}

fn missing_in_group(item: &str, group: &GroupName, available: &[String]) -> AppError {
    let err = AppError::runtime(MsgKey::GroupItemNotInGroup).with_args([item, group.as_str()]);
    let candidates = name_resolve::suggestions(item, available, 3);
    if candidates.is_empty() {
        err
    } else {
        err.with_hint_args(MsgKey::NameSuggestion, [candidates.join(", ")])
    }
}

fn delete(ctx: &Ctx, names: &[GroupName], force: bool) -> Result<(), AppError> {
    let groups = group_store::load_groups(&ctx.env)?;
    let available = group_names(&groups);

    if let [name] = names {
        let target = resolve_group(name, &available)?;
        ensure_deletable(&groups, &target, force)?;
        if force {
            strip_references(ctx, std::slice::from_ref(&target))?;
        }
        let report = group_store::remove_group(&ctx.env, &target)?;
        warn_legacy(ctx, report.legacy_leftover.as_deref());
        output::emit_line(
            ctx.lang,
            &Msg::with_args(MsgKey::GroupRemoved, [target.as_str()]),
        );
        return Ok(());
    }

    let mut targets: Vec<GroupName> = Vec::new();
    let mut errors: Vec<AppError> = Vec::new();

    for name in names {
        match resolve_group(name, &available) {
            Ok(target) => {
                if !targets.contains(&target) {
                    targets.push(target);
                }
            }
            Err(err) => errors.push(err),
        }
    }

    let deletable_set: HashSet<GroupName> = if force {
        targets.iter().cloned().collect()
    } else {
        // 只有所有依赖者也在本次删除集合里，目标才可删；迭代到稳定。
        let mut set: HashSet<GroupName> = targets.iter().cloned().collect();
        loop {
            let blocked: Vec<GroupName> = targets
                .iter()
                .filter(|target| set.contains(*target))
                .filter(|target| {
                    groups.iter().any(|group| {
                        !set.contains(&group.name)
                            && group
                                .members
                                .iter()
                                .any(|member| member.group() == Some(*target))
                    })
                })
                .cloned()
                .collect();
            if blocked.is_empty() {
                break;
            }
            for name in blocked {
                set.remove(&name);
            }
        }
        set
    };

    let mut deletable: Vec<GroupName> = Vec::new();
    for target in &targets {
        let exists = groups.iter().any(|group| group.name == *target);
        if !exists {
            errors.push(
                AppError::runtime(MsgKey::GroupNotFound)
                    .with_args([target.as_str()])
                    .with_hint(MsgKey::GroupNotFoundHint),
            );
        } else if deletable_set.contains(target) {
            deletable.push(target.clone());
        } else {
            let dependents: Vec<&str> = groups
                .iter()
                .filter(|group| group.name != *target && !deletable_set.contains(&group.name))
                .filter(|group| {
                    group
                        .members
                        .iter()
                        .any(|member| member.group() == Some(target))
                })
                .map(|group| group.name.as_str())
                .collect();
            errors.push(
                AppError::runtime(MsgKey::GroupDeleteReferenced)
                    .with_args([target.as_str(), &dependents.join(", ")])
                    .with_hint(MsgKey::GroupDeleteReferencedHint),
            );
        }
    }

    if force && !deletable.is_empty() {
        strip_references(ctx, &deletable)?;
    }

    let mut removed = 0usize;
    for target in &deletable {
        match group_store::remove_group(&ctx.env, target) {
            Ok(report) => {
                warn_legacy(ctx, report.legacy_leftover.as_deref());
                removed += 1;
                output::emit_line(
                    ctx.lang,
                    &Msg::with_args(MsgKey::GroupRemoved, [target.as_str()]),
                );
            }
            Err(err) => errors.push(err.into()),
        }
    }

    emit_errors(ctx, &errors);
    let failed = errors.len();
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

fn ensure_deletable(groups: &[Group], target: &GroupName, force: bool) -> Result<(), AppError> {
    if !groups.iter().any(|group| group.name == *target) {
        return Err(AppError::runtime(MsgKey::GroupNotFound)
            .with_args([target.as_str()])
            .with_hint(MsgKey::GroupNotFoundHint));
    }
    if force {
        return Ok(());
    }

    let dependents: Vec<&str> = groups
        .iter()
        .filter(|group| group.name != *target)
        .filter(|group| {
            group
                .members
                .iter()
                .any(|member| member.group() == Some(target))
        })
        .map(|group| group.name.as_str())
        .collect();

    if dependents.is_empty() {
        return Ok(());
    }

    Err(AppError::runtime(MsgKey::GroupDeleteReferenced)
        .with_args([target.as_str(), &dependents.join(", ")])
        .with_hint(MsgKey::GroupDeleteReferencedHint))
}

fn strip_references(ctx: &Ctx, targets: &[GroupName]) -> Result<(), AppError> {
    let report = group_store::strip_group_references(&ctx.env, targets)?;

    for (owner, reference) in report.stripped {
        output::warn(
            ctx.lang,
            &Msg::with_args(
                MsgKey::GroupRefStripped,
                [reference.as_str().to_string(), owner.as_str().to_string()],
            ),
        );
    }

    warn_legacy(ctx, report.legacy_leftover.as_deref());
    Ok(())
}

/// 旧 `config.toml` 里的 `[groups]` 没删掉时提醒一次：它已被忽略，但用户可能想手工清理。
fn warn_legacy(ctx: &Ctx, leftover: Option<&Path>) {
    if let Some(path) = leftover {
        output::warn(
            ctx.lang,
            &Msg::with_args(MsgKey::GroupLegacyLeftover, [path.display().to_string()]),
        );
    }
}

fn finish_items(
    ctx: &Ctx,
    errors: Vec<AppError>,
    failed_key: MsgKey,
    hint_key: MsgKey,
) -> Result<(), AppError> {
    if errors.is_empty() {
        return Ok(());
    }
    if errors.len() == 1 {
        return Err(errors.into_iter().next().expect("checked non-empty"));
    }

    let count = errors.len();
    emit_errors(ctx, &errors);
    output::emit_gap();
    Err(AppError::runtime(failed_key)
        .with_args([count.to_string()])
        .with_hint(hint_key))
}

fn emit_errors(ctx: &Ctx, errors: &[AppError]) {
    for (index, err) in errors.iter().enumerate() {
        if index > 0 {
            output::emit_gap();
        }
        output::emit_stderr(&error_output::render_block(err, ctx.lang));
    }
}

fn emit_saved(ctx: &Ctx, name: &GroupName, report: &group_store::GroupWriteReport) {
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

/// 已定义分组的组名列表，供唯一前缀解析使用。
fn group_names(groups: &[Group]) -> Vec<String> {
    groups
        .iter()
        .map(|group| group.name.as_str().to_string())
        .collect()
}

fn render_skills(skills: &[SkillName]) -> String {
    if skills.is_empty() {
        return String::new();
    }
    skills
        .iter()
        .map(SkillName::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(serde::Serialize)]
struct JsonGroup<'a> {
    name: &'a str,
    skills: Vec<&'a str>,
    includes: Vec<&'a str>,
}

/// JSON 字段固定英文，不参与 i18n；空组输出 `[]`。
fn render_json(groups: &[Group], resolved: Option<&[Option<Vec<SkillName>>]>) -> String {
    if groups.is_empty() {
        return "[]".to_string();
    }

    let rows: Vec<JsonGroup> = groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            let skills: Vec<&str> = match resolved.and_then(|values| values.get(index)) {
                Some(Some(skills)) => skills.iter().map(SkillName::as_str).collect(),
                _ => group
                    .members
                    .iter()
                    .filter_map(|member| member.skill().map(SkillName::as_str))
                    .collect(),
            };
            let includes: Vec<&str> = group
                .members
                .iter()
                .filter_map(|member| member.group().map(GroupName::as_str))
                .collect();
            JsonGroup {
                name: group.name.as_str(),
                skills,
                includes,
            }
        })
        .collect();

    serde_json::to_string_pretty(&rows).expect("serializing plain groups cannot fail")
}
