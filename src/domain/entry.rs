//! `list` 的展示数据。

use std::path::PathBuf;

/// 扫描出的一个 skill 目录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub valid: bool,
    pub error: Option<String>,
}
