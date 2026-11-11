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
        .subcommand(search_command(lang))
        .subcommand(pull_command(lang))
        .subcommand(push_command(lang))
        .subcommand(remove_command(lang))
        .subcommand(group_command(lang))
        .subcommand(upgrade_command(lang))
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
                .required(false)
                .value_name("SCOPE")
                .value_parser(SCOPE_VALUES)
                .default_value("project")
                .hide_default_value(true)
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
                .required(false)
                .value_name("SCOPE")
                .value_parser(SCOPE_VALUES)
                .default_value("project")
                .hide_default_value(true)
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

fn search_command(lang: Lang) -> Command {
    let command = Command::new("search")
        .about(tr(lang, MsgKey::SearchAbout))
        .arg(
            Arg::new("query")
                .required(true)
                .value_name("QUERY")
                .help(tr(lang, MsgKey::SearchQueryArg)),
        )
        .arg(
            Arg::new("scope")
                .required(false)
                .value_name("SCOPE")
                .value_parser(SCOPE_VALUES)
                .default_value("project")
                .hide_default_value(true)
                .help(tr(lang, MsgKey::SearchScopeArg)),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::SearchJsonFlag)),
        )
        .arg(
            Arg::new("name")
                .long("name")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::SearchNameFlag)),
        )
        .arg(
            Arg::new("description")
                .long("description")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::SearchDescriptionFlag)),
        );

    with_help_flag(command, lang)
}

fn pull_command(lang: Lang) -> Command {
    let command = Command::new("pull")
        .about(tr(lang, MsgKey::PullAbout))
        .arg(
            Arg::new("name")
                .num_args(1..)
                .value_name("NAME")
                .help(tr(lang, MsgKey::PullNameArg)),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::PullAllFlag)),
        )
        .arg(
            Arg::new("group")
                .long("group")
                .value_name("GROUP")
                .action(clap::ArgAction::Append)
                .help(tr(lang, MsgKey::PullGroupFlag)),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::PullForceFlag)),
        );

    with_help_flag(command, lang)
}

fn push_command(lang: Lang) -> Command {
    let command = Command::new("push")
        .about(tr(lang, MsgKey::PushAbout))
        .arg(
            Arg::new("name")
                .num_args(1..)
                .value_name("NAME")
                .help(tr(lang, MsgKey::PushNameArg)),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::PushAllFlag)),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::PushForceFlag)),
        );

    with_help_flag(command, lang)
}

fn remove_command(lang: Lang) -> Command {
    // clap 不允许变长位置参数之后再跟可选位置参数，因此 scope 不在这里声明：
    // 它由 `cli::parse` 从 name 末尾识别（Q51），规则写进 NAME 的帮助文案。
    let command = Command::new("remove")
        .about(tr(lang, MsgKey::RemoveAbout))
        .arg(
            Arg::new("name")
                .required(true)
                .num_args(1..)
                .value_name("NAME")
                .help(tr(lang, MsgKey::RemoveNameArg)),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::RemoveForceFlag)),
        );

    with_help_flag(command, lang)
}

fn group_command(lang: Lang) -> Command {
    let list = with_help_flag(
        Command::new("list")
            .about(tr(lang, MsgKey::GroupListAbout))
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(clap::ArgAction::SetTrue)
                    .help(tr(lang, MsgKey::GroupJsonFlag)),
            ),
        lang,
    );

    let set = with_help_flag(
        Command::new("set")
            .about(tr(lang, MsgKey::GroupSetAbout))
            .arg(
                Arg::new("group")
                    .required(true)
                    .value_name("GROUP")
                    .help(tr(lang, MsgKey::GroupNameArg)),
            )
            .arg(
                Arg::new("skill")
                    .required(true)
                    .num_args(1..)
                    .value_name("SKILL")
                    .help(tr(lang, MsgKey::GroupSkillArg)),
            )
            .arg(
                Arg::new("force")
                    .long("force")
                    .action(clap::ArgAction::SetTrue)
                    .help(tr(lang, MsgKey::GroupSetForceFlag)),
            ),
        lang,
    );

    let add = with_help_flag(
        Command::new("add")
            .about(tr(lang, MsgKey::GroupAddAbout))
            .arg(
                Arg::new("group")
                    .required(true)
                    .value_name("GROUP")
                    .help(tr(lang, MsgKey::GroupNameArg)),
            )
            .arg(
                Arg::new("skill")
                    .required(true)
                    .num_args(1..)
                    .value_name("SKILL")
                    .help(tr(lang, MsgKey::GroupSkillArg)),
            ),
        lang,
    );

    let remove = with_help_flag(
        Command::new("remove")
            .about(tr(lang, MsgKey::GroupRemoveAbout))
            .arg(
                Arg::new("group")
                    .required(true)
                    .num_args(1..)
                    .value_name("GROUP")
                    .help(tr(lang, MsgKey::GroupNameArg)),
            ),
        lang,
    );

    with_help_flag(
        Command::new("group")
            .about(tr(lang, MsgKey::GroupAbout))
            .subcommand_required(true)
            .arg_required_else_help(true)
            .subcommand(list)
            .subcommand(set)
            .subcommand(add)
            .subcommand(remove),
        lang,
    )
}

/// `upgrade` 不带位置参数，也没有短选项（不参与归一化）。
fn upgrade_command(lang: Lang) -> Command {
    let command = Command::new("upgrade")
        .about(tr(lang, MsgKey::UpgradeAbout))
        .arg(
            Arg::new("check")
                .long("check")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::UpgradeCheckFlag)),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::UpgradeForceFlag)),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::UpgradeVerboseFlag)),
        )
        .arg(
            Arg::new("progress")
                .long("progress")
                .action(clap::ArgAction::SetTrue)
                .help(tr(lang, MsgKey::UpgradeProgressFlag)),
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
