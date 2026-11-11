//! 命令行解析层：只处理 `argv`，不读环境、不碰文件系统。

pub mod help;
pub mod lang;
pub mod normalize;
pub mod spec;

use std::collections::HashSet;
use std::path::PathBuf;

use clap::ArgMatches;

use crate::domain::action::{Action, ConfigCommand, GroupCommand, SearchField, TransferTarget};
use crate::domain::config::ConfigKey;
use crate::domain::group::GroupName;
use crate::domain::lang::Lang;
use crate::domain::name::SkillName;
use crate::domain::scope::Scope;
use crate::error::AppError;
use crate::i18n::MsgKey;

use self::normalize::NormalizeError;

/// 解析结果：要么是一个待执行的动作，要么是一段要直接输出的文本。
#[derive(Debug)]
pub enum Outcome {
    Action(crate::domain::action::Action),
    Text {
        content: String,
        stream: Stream,
        exit_code: u8,
    },
}

/// 文本输出去向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

impl From<NormalizeError> for AppError {
    fn from(err: NormalizeError) -> Self {
        use crate::i18n::MsgKey;

        match err {
            NormalizeError::MultipleActions => {
                AppError::usage(MsgKey::UsageMultipleActions).with_hint(MsgKey::UsageHintForms)
            }
            NormalizeError::MissingActionValue { flag } => {
                AppError::usage(MsgKey::UsageMissingActionValue)
                    .with_args([flag])
                    .with_hint(MsgKey::UsageHintForms)
            }
            NormalizeError::MultipleScopes => {
                AppError::usage(MsgKey::UsageMultipleScopes).with_hint(MsgKey::UsageHintForms)
            }
            NormalizeError::MixedForms => {
                AppError::usage(MsgKey::UsageMixedForms).with_hint(MsgKey::UsageHintForms)
            }
        }
    }
}

/// 解析整条命令行。`lang` 决定 help/错误的语言。
pub fn parse(lang: Lang, argv: Vec<std::ffi::OsString>) -> Result<Outcome, AppError> {
    let argv = normalize::normalize(argv)?;

    let mut full = Vec::with_capacity(argv.len() + 1);
    full.push(std::ffi::OsString::from("lsm"));
    full.extend(argv);

    let matches = match spec::build(lang).try_get_matches_from_mut(full) {
        Ok(matches) => matches,
        Err(err) => return clap_outcome(err, lang),
    };

    let (name, sub) = matches
        .subcommand()
        .ok_or_else(|| AppError::usage(MsgKey::UsageMissingSubcommand))?;

    let action = match name {
        "add" => Action::Add {
            source: PathBuf::from(required(sub, "source")),
            scope: scope_of(&required(sub, "scope")),
            force: sub.get_flag("force"),
        },
        "list" => Action::List {
            scope: scope_of(&required(sub, "scope")),
            json: sub.get_flag("json"),
        },
        "search" => {
            let query = required(sub, "query");
            let query = query.trim().to_string();
            if query.is_empty() {
                return Err(
                    AppError::usage(MsgKey::UsageEmptyQuery).with_hint(MsgKey::UsageHelpHint)
                );
            }
            Action::Search {
                query,
                scope: scope_of(&required(sub, "scope")),
                field: search_field(sub),
                json: sub.get_flag("json"),
            }
        }
        "pull" => {
            let target = transfer_target(sub, "pull")?;
            Action::Pull {
                target,
                force: sub.get_flag("force"),
            }
        }
        "push" => {
            let target = transfer_target(sub, "push")?;
            Action::Push {
                target,
                force: sub.get_flag("force"),
            }
        }
        "config" => Action::Config(parse_config(sub)?),
        "group" => Action::Group(parse_group(sub)?),
        "remove" => {
            let (names, scope) = remove_target(sub)?;
            Action::Remove {
                scope,
                names,
                force: sub.get_flag("force"),
            }
        }
        "upgrade" => Action::Upgrade {
            check: sub.get_flag("check"),
            force: sub.get_flag("force"),
            verbose: sub.get_flag("verbose"),
            progress: sub.get_flag("progress"),
        },
        other => return Err(AppError::usage(MsgKey::UsageInvalidSubcommand).with_args([other])),
    };

    Ok(Outcome::Action(action))
}

/// `pull` / `push` 的目标：`<NAME>...`、`--group`、`--all` 三者只能选一个。
fn transfer_target(
    matches: &ArgMatches,
    command: &'static str,
) -> Result<TransferTarget, AppError> {
    let names = skill_names(matches)?;
    let groups = group_names(matches)?;
    let all = matches.get_flag("all");

    let selected = [!names.is_empty(), !groups.is_empty(), all]
        .into_iter()
        .filter(|chosen| *chosen)
        .count();

    match selected {
        0 => Err(AppError::usage(MsgKey::UsagePullTargetMissing)
            .with_args([command])
            .with_hint(MsgKey::UsageHelpHint)),
        1 if !names.is_empty() => Ok(TransferTarget::Many(names)),
        1 if !groups.is_empty() => Ok(TransferTarget::Groups(groups)),
        1 => Ok(TransferTarget::All),
        _ => Err(AppError::usage(MsgKey::UsagePullTargetConflict)
            .with_args([command])
            .with_hint(MsgKey::UsageHelpHint)),
    }
}

/// 读取全部 `name` 并校验；重复 name 去重，保留首次出现的顺序。
fn skill_names(matches: &ArgMatches) -> Result<Vec<SkillName>, AppError> {
    skill_values(matches, "name")
}

/// 读取全部 `--group` 并校验；重复组名去重，保留首次出现的顺序。
fn group_names(matches: &ArgMatches) -> Result<Vec<GroupName>, AppError> {
    let Ok(values) = matches.try_get_many::<String>("group") else {
        return Ok(Vec::new()); // `push` 没有 `--group`
    };

    let mut groups = Vec::new();
    let mut seen = HashSet::new();
    for raw in values.into_iter().flatten() {
        let group = GroupName::parse(raw)?;
        if seen.insert(group.clone()) {
            groups.push(group);
        }
    }
    Ok(groups)
}

fn skill_values(matches: &ArgMatches, id: &str) -> Result<Vec<SkillName>, AppError> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    for raw in matches.get_many::<String>(id).into_iter().flatten() {
        let name = SkillName::parse(raw)?;
        if seen.insert(name.clone()) {
            names.push(name);
        }
    }
    Ok(names)
}

/// `remove` 的 scope 可省略：末尾恰好是 `global` / `project` 时当作用域（Q51）。
fn remove_target(matches: &ArgMatches) -> Result<(Vec<SkillName>, Scope), AppError> {
    let mut raw: Vec<String> = matches
        .get_many::<String>("name")
        .into_iter()
        .flatten()
        .cloned()
        .collect();

    let scope = match trailing_scope(&raw) {
        Some(scope) => {
            raw.pop();
            scope
        }
        None => Scope::Project,
    };

    // `remove global` 会被读成「scope=global 但没有 name」：明确报用法错误。
    if raw.is_empty() {
        return Err(AppError::usage(MsgKey::UsageRemoveMissingName)
            .with_hint(MsgKey::UsageRemoveMissingNameHint));
    }

    let mut names = Vec::new();
    let mut seen = HashSet::new();
    for value in &raw {
        let name = SkillName::parse(value)?;
        if seen.insert(name.clone()) {
            names.push(name);
        }
    }

    Ok((names, scope))
}

/// 末尾的 scope 词：只有 `remove` 的变长 name 需要这个判定。
fn trailing_scope(raw: &[String]) -> Option<Scope> {
    match raw.last()?.as_str() {
        "global" => Some(Scope::Global),
        "project" => Some(Scope::Project),
        _ => None,
    }
}

/// `group` 的嵌套子命令；所有组名/成员名在构造 `Action` 前完成校验。
fn parse_group(matches: &ArgMatches) -> Result<GroupCommand, AppError> {
    let (name, sub) = matches
        .subcommand()
        .ok_or_else(|| AppError::usage(MsgKey::UsageMissingSubcommand))?;

    match name {
        "list" => Ok(GroupCommand::List {
            json: sub.get_flag("json"),
        }),
        "set" => Ok(GroupCommand::Set {
            name: group_value(sub, "group")?,
            members: skill_values(sub, "skill")?,
            force: sub.get_flag("force"),
        }),
        "add" => Ok(GroupCommand::Add {
            name: group_value(sub, "group")?,
            members: skill_values(sub, "skill")?,
        }),
        "remove" => Ok(GroupCommand::Remove {
            names: group_values(sub, "group")?,
        }),
        other => Err(AppError::usage(MsgKey::UsageInvalidSubcommand).with_args([other])),
    }
}

fn group_value(matches: &ArgMatches, id: &str) -> Result<GroupName, AppError> {
    Ok(GroupName::parse(&required(matches, id))?)
}

fn group_values(matches: &ArgMatches, id: &str) -> Result<Vec<GroupName>, AppError> {
    let mut groups = Vec::new();
    let mut seen = HashSet::new();
    for raw in matches.get_many::<String>(id).into_iter().flatten() {
        let group = GroupName::parse(raw)?;
        if seen.insert(group.clone()) {
            groups.push(group);
        }
    }
    Ok(groups)
}

fn parse_config(matches: &ArgMatches) -> Result<ConfigCommand, AppError> {
    let (name, sub) = matches
        .subcommand()
        .ok_or_else(|| AppError::usage(MsgKey::UsageMissingSubcommand))?;

    let key = match required(sub, "key").as_str() {
        "lang" => ConfigKey::Lang,
        other => return Err(AppError::usage(MsgKey::ConfigUnknownKey).with_args([other])),
    };

    match name {
        "get" => Ok(ConfigCommand::Get(key)),
        "set" => {
            let raw = required(sub, "value");
            let value = Lang::parse(&raw).map_err(|_| {
                AppError::usage(MsgKey::ConfigInvalidValue)
                    .with_args([raw.clone()])
                    .with_hint(MsgKey::UsageInvalidLanguageHint)
            })?;
            Ok(ConfigCommand::Set(key, value))
        }
        other => Err(AppError::usage(MsgKey::UsageInvalidSubcommand).with_args([other])),
    }
}

/// 必填参数由 clap 保证存在。
fn required(matches: &ArgMatches, name: &str) -> String {
    matches.get_one::<String>(name).cloned().unwrap_or_default()
}

fn scope_of(value: &str) -> Scope {
    if value == Scope::Global.label() {
        Scope::Global
    } else {
        Scope::Project
    }
}

/// `--name` / `--description` 同时给出等价于默认的「都匹配」（Q52）。
fn search_field(matches: &ArgMatches) -> SearchField {
    match (matches.get_flag("name"), matches.get_flag("description")) {
        (true, false) => SearchField::Name,
        (false, true) => SearchField::Description,
        _ => SearchField::Both,
    }
}

/// clap 的 help / version / 缺子命令：转成直接输出的文本。
fn clap_outcome(err: clap::Error, lang: Lang) -> Result<Outcome, AppError> {
    use clap::error::ErrorKind as Kind;

    match err.kind() {
        Kind::DisplayHelp | Kind::DisplayVersion => Ok(Outcome::Text {
            content: help::localize(&err.to_string(), lang),
            stream: Stream::Stdout,
            exit_code: 0,
        }),
        Kind::DisplayHelpOnMissingArgumentOrSubcommand | Kind::MissingSubcommand => {
            Ok(Outcome::Text {
                content: help::localize(&err.to_string(), lang),
                stream: Stream::Stderr,
                exit_code: 2,
            })
        }
        kind => Err(map_clap_error(kind, &err)),
    }
}

/// 常见参数错误映射到本地化文案；未覆盖的类型回退英文原文。
fn map_clap_error(kind: clap::error::ErrorKind, err: &clap::Error) -> AppError {
    use clap::error::ErrorKind as Kind;

    let key = match kind {
        Kind::MissingRequiredArgument => MsgKey::UsageMissingArgument,
        Kind::InvalidValue | Kind::ValueValidation => MsgKey::UsageInvalidValue,
        Kind::UnknownArgument => MsgKey::UsageUnknownArgument,
        Kind::TooManyValues | Kind::TooFewValues => MsgKey::UsageUnexpectedArgument,
        Kind::InvalidSubcommand => MsgKey::UsageInvalidSubcommand,
        _ => MsgKey::UsageInvalidArguments,
    };

    let detail = offending(err).unwrap_or_else(|| first_line(&err.to_string()));

    AppError::usage(key)
        .with_args([detail])
        .with_hint(MsgKey::UsageHelpHint)
}

/// clap 记录的第一个「出问题的参数」。
fn offending(err: &clap::Error) -> Option<String> {
    use clap::error::{ContextKind, ContextValue};

    err.context().find_map(|(kind, value)| match (kind, value) {
        (ContextKind::InvalidArg | ContextKind::InvalidSubcommand, ContextValue::String(name)) => {
            Some(name.clone())
        }
        (
            ContextKind::InvalidArg | ContextKind::InvalidSubcommand,
            ContextValue::Strings(names),
        ) => Some(names.join(", ")),
        _ => None,
    })
}

fn first_line(text: &str) -> String {
    let line = text.lines().next().unwrap_or_default();
    line.strip_prefix("error: ").unwrap_or(line).to_string()
}
