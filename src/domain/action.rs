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
    Copy {
        target: CopyTarget,
        force: bool,
    },
    Remove {
        scope: Scope,
        name: SkillName,
        force: bool,
    },
    Config(ConfigCommand),
}

/// `copy` 的复制对象：单个 skill 或全局全部。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyTarget {
    One(SkillName),
    All,
}

/// `config` 子命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigCommand {
    Get(ConfigKey),
    Set(ConfigKey, Lang),
}
