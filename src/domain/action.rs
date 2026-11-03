//! 解析后的用户意图。`cli` 产出它，`commands` 消费它。

use std::path::PathBuf;

use super::config::ConfigKey;
use super::lang::Lang;
use super::name::SkillName;
use super::scope::Scope;

/// 一次调用的动作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Add {
        source: PathBuf,
        scope: Scope,
        force: bool,
    },
    List {
        scope: Scope,
        json: bool,
    },
    Search {
        query: String,
        scope: Scope,
        json: bool,
    },
    Pull {
        target: TransferTarget,
        force: bool,
    },
    Push {
        target: TransferTarget,
        force: bool,
    },
    Remove {
        scope: Scope,
        names: Vec<SkillName>,
        force: bool,
    },
    Config(ConfigCommand),
    Upgrade {
        check: bool,
        force: bool,
    },
}

/// `pull` / `push` 的目标：一个或多个 skill，或整个作用域。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferTarget {
    Many(Vec<SkillName>),
    All,
}

/// `config` 子命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigCommand {
    Get(ConfigKey),
    Set(ConfigKey, Lang),
}
