//! 命令行解析层：只处理 `argv`，不读环境、不碰文件系统。

pub mod help;
pub mod lang;
pub mod normalize;
pub mod spec;

use std::path::PathBuf;

use clap::ArgMatches;

use crate::domain::action::{Action, ConfigCommand};
use crate::domain::config::ConfigKey;
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
            NormalizeError::MissingScope { action } => {
                AppError::usage(MsgKey::UsageMissingScope).with_args([action])
            }
            NormalizeError::CopyWithScope => AppError::usage(MsgKey::UsageCopyWithScope),
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
        "copy" => Action::Copy {
            name: SkillName::parse(&required(sub, "name"))?,
            force: sub.get_flag("force"),
        },
        "config" => Action::Config(parse_config(sub)?),
        other => return Err(AppError::usage(MsgKey::UsageInvalidSubcommand).with_args([other])),
    };

    Ok(Outcome::Action(action))
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
