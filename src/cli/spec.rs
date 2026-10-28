//! clap builder：所有用户可见文案按当前语言注入。

use clap::{Arg, Command};

use crate::domain::lang::Lang;
use crate::i18n::{tr, MsgKey};

/// 作用域的合法取值，与 [`crate::domain::scope::Scope`] 一一对应。
pub const SCOPE_VALUES: [&str; 2] = ["global", "project"];

/// v1 只有 `lang` 一个配置键。
pub const CONFIG_KEY_VALUES: [&str; 1] = ["lang"];

/// 按语言构建根命令。
pub fn build(lang: Lang) -> Command {
    let root = Command::new("lsm")
        .about(tr(lang, MsgKey::HelpAbout))
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .disable_help_subcommand(true)
        .arg(
            Arg::new("lang")
                .long("lang")
                .value_name("LANG")
                .global(true)
                .help(tr(lang, MsgKey::LangHelp)),
        )
        .subcommand(add_command(lang))
        .subcommand(list_command(lang))
        .subcommand(copy_command(lang))
        .subcommand(config_command(lang));

    with_help_flag(root, lang).disable_version_flag(true).arg(
        Arg::new("version")
            .short('V')
            .long("version")
            .action(clap::ArgAction::Version)
            .help(tr(lang, MsgKey::VersionFlagHelp)),
    )
}

/// clap 自带的 `-h` 说明是英文，换成自建参数以便本地化。
fn with_help_flag(command: Command, lang: Lang) -> Command {
    command.disable_help_flag(true).arg(
        Arg::new("help")
            .short('h')
            .long("help")
            .action(clap::ArgAction::Help)
            .help(tr(lang, MsgKey::HelpFlagHelp)),
    )
}

fn add_command(lang: Lang) -> Command {
    let command = Command::new("add")
        .about(tr(lang, MsgKey::AddAbout))
        .arg(
            Arg::new("source")
                .required(true)
                .value_name("SOURCE")
                .help(tr(lang, MsgKey::AddSourceArg)),
        )
        .arg(
            Arg::new("scope")
                .required(true)
                .value_name("SCOPE")
                .value_parser(SCOPE_VALUES)
                .help(tr(lang, MsgKey::AddScopeArg)),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::AddForceFlag)),
        );

    with_help_flag(command, lang)
}

fn list_command(lang: Lang) -> Command {
    let command = Command::new("list")
        .about(tr(lang, MsgKey::ListAbout))
        .arg(
            Arg::new("scope")
                .required(true)
                .value_name("SCOPE")
                .value_parser(SCOPE_VALUES)
                .help(tr(lang, MsgKey::ListScopeArg)),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::ListJsonFlag)),
        );

    with_help_flag(command, lang)
}

fn copy_command(lang: Lang) -> Command {
    let command = Command::new("copy")
        .about(tr(lang, MsgKey::CopyAbout))
        .arg(
            Arg::new("name")
                .required(true)
                .value_name("NAME")
                .help(tr(lang, MsgKey::CopyNameArg)),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::CopyForceFlag)),
        );

    with_help_flag(command, lang)
}

fn config_command(lang: Lang) -> Command {
    let command = Command::new("config")
        .about(tr(lang, MsgKey::ConfigAbout))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(with_help_flag(
            Command::new("get")
                .about(tr(lang, MsgKey::ConfigGetAbout))
                .arg(
                    Arg::new("key")
                        .required(true)
                        .value_name("KEY")
                        .value_parser(CONFIG_KEY_VALUES)
                        .help(tr(lang, MsgKey::ConfigKeyLangHelp)),
                ),
            lang,
        ))
        .subcommand(with_help_flag(
            Command::new("set")
                .about(tr(lang, MsgKey::ConfigSetAbout))
                .arg(
                    Arg::new("key")
                        .required(true)
                        .value_name("KEY")
                        .value_parser(CONFIG_KEY_VALUES)
                        .help(tr(lang, MsgKey::ConfigKeyLangHelp)),
                )
                .arg(
                    Arg::new("value")
                        .required(true)
                        .value_name("VALUE")
                        .help(tr(lang, MsgKey::ConfigValueArg)),
                ),
            lang,
        ));

    with_help_flag(command, lang)
}
