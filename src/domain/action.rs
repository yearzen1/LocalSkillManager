//! 解析后的用户意图。`cli` 产出它，`commands` 消费它。

use std::path::PathBuf;

use super::config::ConfigKey;
use super::group::{GroupName, Member};
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
        field: SearchField,
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
    Group(GroupCommand),
    Config(ConfigCommand),
    Upgrade {
        check: bool,
        force: bool,
        verbose: bool,
        progress: bool,
    },
}

/// `pull` / `push` 的目标：一个或多个 skill、分组，或整个作用域。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferTarget {
    Many(Vec<SkillName>),
    Groups(Vec<GroupName>),
    All,
}

/// `group` 子命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupCommand {
    List {
        json: bool,
        resolved: bool,
    },
    Set {
        name: GroupName,
        members: Vec<Member>,
        force: bool,
    },
    Add {
        name: GroupName,
        members: Vec<Member>,
    },
    Remove {
        name: GroupName,
        members: Vec<Member>,
    },
    Delete {
        names: Vec<GroupName>,
        force: bool,
    },
}

/// `search` 的匹配字段（Q52）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchField {
    /// name 与 description 取并集（默认，也覆盖两个 flag 同给）。
    Both,
    /// 只匹配 name。
    Name,
    /// 只匹配 description。
    Description,
}

/// `config` 子命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigCommand {
    Get(ConfigKey),
    Set(ConfigKey, Lang),
}
