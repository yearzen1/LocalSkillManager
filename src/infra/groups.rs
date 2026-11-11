//! `groups.toml` 的读写：顶层表「组名 → 成员数组」。
//!
//! 旧版本把分组存在 `config.toml` 的 `[groups]` 表里。读取时若 `groups.toml`
//! 不存在则回退到旧表；第一次写盘会把分组落进 `groups.toml`，并尽力删除旧表。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::domain::group::{Group, GroupName, Member};

use super::env::Env;
use super::paths::{self, PathError};
use super::tomlfile::{self, TomlFileError};

/// `groups.toml` 的写盘文件头，只在新建文件时写入。
const HEADER: &str = "# Skill groups: name = [skill, \"@group\"]\n";

/// 分组读写失败。
#[derive(Debug)]
pub enum GroupError {
    /// 无法确定配置目录。
    Path(PathError),
    /// 读取文件失败。
    Io {
        path: PathBuf,
        source: anyhow::Error,
    },
    /// 写入文件失败。
    Write {
        path: PathBuf,
        source: anyhow::Error,
    },
    /// TOML 语法非法：不覆盖、不忽略。
    Parse {
        path: PathBuf,
        source: anyhow::Error,
    },
    /// 分组文件不是「组名 → 成员数组」的结构。
    Invalid { path: PathBuf, detail: String },
    /// `group set` 要覆盖已存在的组但没加 `--force`。
    GroupExists { name: String },
    /// 要 `add` / `remove` / `delete` 的组不存在。
    GroupNotFound { name: String },
}

/// 一次分组写盘的结果。
pub struct GroupWriteReport {
    pub path: PathBuf,
    /// 写盘后的成员总数；`group delete` 固定为 0。
    pub members: usize,
    /// 旧 `config.toml` 里的 `[groups]` 未能删除时带回路径，由命令层打 warning。
    pub legacy_leftover: Option<PathBuf>,
}

/// 一次引用清理的结果。
pub struct StripReport {
    /// 被移除的（拥有者, 引用）。
    pub stripped: Vec<(GroupName, GroupName)>,
    /// 旧 `config.toml` 里的 `[groups]` 未能删除时带回路径。
    pub legacy_leftover: Option<PathBuf>,
}

/// 读取全部分组；文件缺失返回空列表，结构非法才是错误。
pub fn load_groups(env: &Env) -> Result<Vec<Group>, GroupError> {
    let effective = load_effective(env)?;
    let source = source_path(env, &effective)?;
    groups_from_document(&source, &effective.doc)
}

/// 创建或整体替换一个分组；`force` 为假时拒绝覆盖已有组。
pub fn set_group(
    env: &Env,
    name: &GroupName,
    members: &[Member],
    force: bool,
) -> Result<GroupWriteReport, GroupError> {
    let mut effective = load_effective(env)?;
    let members = dedupe(members);

    {
        let table = effective.doc.as_table_mut();
        if !force && table.contains_key(name.as_str()) {
            return Err(GroupError::GroupExists {
                name: name.as_str().to_string(),
            });
        }
        set_members(table, name.as_str(), &members);
    }

    let (path, legacy_leftover) = commit(env, effective)?;
    Ok(GroupWriteReport {
        path,
        members: members.len(),
        legacy_leftover,
    })
}

/// 向分组追加成员；组不存在时自动创建，重复成员静默去重。
pub fn add_group(
    env: &Env,
    name: &GroupName,
    members: &[Member],
) -> Result<GroupWriteReport, GroupError> {
    let mut effective = load_effective(env)?;
    let source = source_path(env, &effective)?;

    let merged = {
        let table = effective.doc.as_table_mut();
        let mut merged = match table.get(name.as_str()) {
            Some(item) => members_from_item(&source, name.as_str(), item)?,
            None => Vec::new(),
        };
        let mut seen: HashSet<Member> = merged.iter().cloned().collect();
        for member in members {
            if seen.insert(member.clone()) {
                merged.push(member.clone());
            }
        }
        set_members(table, name.as_str(), &merged);
        merged
    };

    let (path, legacy_leftover) = commit(env, effective)?;
    Ok(GroupWriteReport {
        path,
        members: merged.len(),
        legacy_leftover,
    })
}

/// 用给定成员整体替换一个已存在的分组；组不存在返回 [`GroupError::GroupNotFound`]。
pub fn replace_group_members(
    env: &Env,
    name: &GroupName,
    members: &[Member],
) -> Result<GroupWriteReport, GroupError> {
    let mut effective = load_effective(env)?;
    let members = dedupe(members);

    {
        let table = effective.doc.as_table_mut();
        if !table.contains_key(name.as_str()) {
            return Err(GroupError::GroupNotFound {
                name: name.as_str().to_string(),
            });
        }
        set_members(table, name.as_str(), &members);
    }

    let (path, legacy_leftover) = commit(env, effective)?;
    Ok(GroupWriteReport {
        path,
        members: members.len(),
        legacy_leftover,
    })
}

/// 从所有分组中移除指向 `targets` 的 `@ref`。
pub fn strip_group_references(env: &Env, targets: &[GroupName]) -> Result<StripReport, GroupError> {
    let mut effective = load_effective(env)?;
    let source = source_path(env, &effective)?;
    let mut stripped = Vec::new();

    {
        let table = effective.doc.as_table_mut();
        let names: Vec<String> = table.iter().map(|(name, _)| name.to_string()).collect();

        for raw_name in names {
            let owner = GroupName::parse(&raw_name)
                .map_err(|_| invalid(&source, format!("invalid group name '{raw_name}'")))?;
            let Some(item) = table.get(&raw_name) else {
                continue;
            };
            let mut members = members_from_item(&source, &raw_name, item)?;
            let before = members.len();

            members.retain(|member| match member {
                Member::Group(reference) if targets.contains(reference) => {
                    stripped.push((owner.clone(), reference.clone()));
                    false
                }
                _ => true,
            });

            if members.len() != before {
                set_members(table, &raw_name, &members);
            }
        }
    }

    if stripped.is_empty() {
        return Ok(StripReport {
            stripped,
            legacy_leftover: None,
        });
    }

    let (_, legacy_leftover) = commit(env, effective)?;
    Ok(StripReport {
        stripped,
        legacy_leftover,
    })
}

/// 删除一个分组；组不存在返回 [`GroupError::GroupNotFound`]。
pub fn remove_group(env: &Env, name: &GroupName) -> Result<GroupWriteReport, GroupError> {
    let mut effective = load_effective(env)?;

    if effective.doc.as_table_mut().remove(name.as_str()).is_none() {
        return Err(GroupError::GroupNotFound {
            name: name.as_str().to_string(),
        });
    }

    let (path, legacy_leftover) = commit(env, effective)?;
    Ok(GroupWriteReport {
        path,
        members: 0,
        legacy_leftover,
    })
}

/// 当前生效的分组文档：`groups.toml` 优先，缺失时才回退到 `config.toml` 的 `[groups]`。
struct Effective {
    doc: toml_edit::DocumentMut,
    /// 从旧表迁入时为 `config.toml` 的路径。
    legacy_from: Option<PathBuf>,
    /// `groups.toml` 是否已存在；决定写盘时是否补文件头。
    exists: bool,
}

fn load_effective(env: &Env) -> Result<Effective, GroupError> {
    let target = groups_path(env)?;
    let (doc, missing) = tomlfile::read(&target)?;
    if !missing {
        return Ok(Effective {
            doc,
            legacy_from: None,
            exists: true,
        });
    }

    let config = config_path(env)?;
    let (config_doc, config_missing) = tomlfile::read(&config)?;
    if config_missing {
        return Ok(Effective {
            doc: toml_edit::DocumentMut::new(),
            legacy_from: None,
            exists: false,
        });
    }

    let Some(item) = config_doc.get("groups") else {
        return Ok(Effective {
            doc: toml_edit::DocumentMut::new(),
            legacy_from: None,
            exists: false,
        });
    };

    let table = item.as_table().ok_or_else(|| {
        invalid(
            &config,
            "groups must be a table of skill-name arrays".to_string(),
        )
    })?;

    let mut doc = toml_edit::DocumentMut::new();
    for (name, value) in table.iter() {
        doc[name] = value.clone();
    }

    Ok(Effective {
        doc,
        legacy_from: Some(config),
        exists: false,
    })
}

/// 写盘：先落 `groups.toml`，再尽力删除 `config.toml` 里的旧 `[groups]`。
fn commit(env: &Env, effective: Effective) -> Result<(PathBuf, Option<PathBuf>), GroupError> {
    let target = groups_path(env)?;
    let mut rendered = effective.doc.to_string();

    if !effective.exists && !rendered.starts_with("# Skill groups") {
        rendered = format!("{HEADER}{rendered}");
    }
    if !rendered.is_empty() && !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    tomlfile::write_rendered(&target, &rendered)?;

    let legacy_leftover = match effective.legacy_from {
        Some(config) => strip_legacy(&config).err().map(|_| config),
        None => None,
    };

    Ok((target, legacy_leftover))
}

/// 删除旧 `config.toml` 里的 `[groups]` 表；其他键与注释保持不变。
fn strip_legacy(config: &Path) -> Result<(), TomlFileError> {
    let (mut doc, missing) = tomlfile::read(config)?;
    if missing || doc.get("groups").is_none() {
        return Ok(());
    }
    doc.as_table_mut().remove("groups");
    tomlfile::write(config, &doc)
}

fn groups_path(env: &Env) -> Result<PathBuf, GroupError> {
    paths::groups_file(env).map_err(GroupError::Path)
}

fn config_path(env: &Env) -> Result<PathBuf, GroupError> {
    paths::config_file(env).map_err(GroupError::Path)
}

/// 读取报错时应指向的文件：旧表在 `config.toml`，否则是 `groups.toml`。
fn source_path(env: &Env, effective: &Effective) -> Result<PathBuf, GroupError> {
    match &effective.legacy_from {
        Some(path) => Ok(path.clone()),
        None => groups_path(env),
    }
}

fn groups_from_document(
    path: &Path,
    doc: &toml_edit::DocumentMut,
) -> Result<Vec<Group>, GroupError> {
    let mut groups = Vec::new();

    for (raw_name, item) in doc.as_table().iter() {
        let name = GroupName::parse(raw_name)
            .map_err(|_| invalid(path, format!("invalid group name '{raw_name}'")))?;
        groups.push(Group {
            name,
            members: members_from_item(path, raw_name, item)?,
        });
    }

    groups.sort_by(|left, right| left.name.as_str().cmp(right.name.as_str()));
    Ok(groups)
}

fn members_from_item(
    path: &Path,
    group: &str,
    item: &toml_edit::Item,
) -> Result<Vec<Member>, GroupError> {
    let array = item.as_array().ok_or_else(|| {
        invalid(
            path,
            format!("group '{group}' must be an array of skill names"),
        )
    })?;

    let mut members = Vec::new();
    let mut seen = HashSet::new();
    for element in array.iter() {
        let raw = element
            .as_str()
            .ok_or_else(|| invalid(path, format!("group '{group}' members must be strings")))?;
        let member = Member::parse(raw)
            .map_err(|_| invalid(path, format!("group '{group}' has invalid member '{raw}'")))?;
        if seen.insert(member.clone()) {
            members.push(member);
        }
    }

    Ok(members)
}

fn members_item(members: &[Member]) -> toml_edit::Item {
    let mut array = toml_edit::Array::new();
    for member in members {
        array.push(member.to_string());
    }
    toml_edit::Item::Value(toml_edit::Value::Array(array))
}

/// 原地替换成员数组：保留键上挂着的注释，不重建 `KeyValue`。
fn set_members(table: &mut toml_edit::Table, name: &str, members: &[Member]) {
    let item = members_item(members);
    match table.get_mut(name) {
        Some(slot) => *slot = item,
        None => {
            table.insert(name, item);
        }
    }
}

fn dedupe(members: &[Member]) -> Vec<Member> {
    let mut seen = HashSet::new();
    members
        .iter()
        .filter(|member| seen.insert((*member).clone()))
        .cloned()
        .collect()
}

fn invalid(path: &Path, detail: String) -> GroupError {
    GroupError::Invalid {
        path: path.to_path_buf(),
        detail,
    }
}

impl From<TomlFileError> for GroupError {
    fn from(err: TomlFileError) -> Self {
        match err {
            TomlFileError::Io { path, source } => GroupError::Io { path, source },
            TomlFileError::Write { path, source } => GroupError::Write { path, source },
            TomlFileError::Parse { path, source } => GroupError::Parse { path, source },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn env_in(dir: &TempDir) -> Env {
        Env {
            home: None,
            config_dir: Some(dir.path().join("lsm")),
            lang: None,
            upgrade_api: None,
            cargo_bin_dir: None,
        }
    }

    fn groups_file(env: &Env) -> PathBuf {
        env.config_dir.as_ref().unwrap().join("groups.toml")
    }

    fn config_file(env: &Env) -> PathBuf {
        env.config_dir.as_ref().unwrap().join("config.toml")
    }

    fn write_groups(env: &Env, content: &str) -> PathBuf {
        let path = groups_file(env);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    fn write_legacy_config(env: &Env, content: &str) -> PathBuf {
        let path = config_file(env);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    fn group(name: &str) -> GroupName {
        GroupName::parse(name).unwrap()
    }

    fn member(name: &str) -> Member {
        Member::parse(name).unwrap()
    }

    fn members_of(group: &Group) -> Vec<String> {
        group.members.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn missing_file_means_empty() {
        let dir = TempDir::new().unwrap();
        assert!(load_groups(&env_in(&dir)).unwrap().is_empty());
    }

    #[test]
    fn reads_top_level_table_sorted_and_dedupes_in_order() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        write_groups(&env, "zeta = [\"b\", \"a\", \"b\"]\nalpha = [\"c\"]\n");

        let groups = load_groups(&env).unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name.as_str(), "alpha");
        assert_eq!(members_of(&groups[0]), ["c"]);
        assert_eq!(groups[1].name.as_str(), "zeta");
        assert_eq!(members_of(&groups[1]), ["b", "a"]);
    }

    #[test]
    fn reads_group_references() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        write_groups(&env, "frontend = [\"@common\", \"tdd\"]\n");

        let groups = load_groups(&env).unwrap();

        assert_eq!(members_of(&groups[0]), ["@common", "tdd"]);
    }

    #[test]
    fn rejects_invalid_structure() {
        let cases = [
            "frontend = [1]\n",
            "Frontend = []\n",
            "frontend = [\"Bad\"]\n",
            "frontend = \"tdd\"\n",
        ];

        for content in cases {
            let dir = TempDir::new().unwrap();
            let env = env_in(&dir);
            let path = write_groups(&env, content);
            assert!(
                matches!(load_groups(&env), Err(GroupError::Invalid { .. })),
                "expected Invalid for {content:?}"
            );
            assert!(path.exists());
        }
    }

    #[test]
    fn set_group_writes_a_new_file_with_a_header() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);

        let report = set_group(&env, &group("frontend"), &[member("tdd")], false).unwrap();

        assert_eq!(report.path, groups_file(&env));
        assert_eq!(report.members, 1);
        let content = fs::read_to_string(groups_file(&env)).unwrap();
        assert!(content.starts_with("# Skill groups"), "{content:?}");
        assert!(content.contains("frontend = [\"tdd\"]"), "{content:?}");
        assert!(!config_file(&env).exists());
    }

    #[test]
    fn group_writes_leave_config_toml_untouched() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        write_legacy_config(&env, "lang = \"zh\"\n\n# keep me\n");

        set_group(&env, &group("frontend"), &[member("tdd")], false).unwrap();

        let content = fs::read_to_string(config_file(&env)).unwrap();
        assert!(content.contains("lang = \"zh\""), "{content:?}");
        assert!(content.contains("# keep me"), "{content:?}");
    }

    #[test]
    fn set_group_requires_force_to_replace() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        let name = group("frontend");

        set_group(&env, &name, &[member("tdd"), member("grill")], false).unwrap();

        assert!(matches!(
            set_group(&env, &name, &[member("grill")], false),
            Err(GroupError::GroupExists { .. })
        ));
        assert_eq!(members_of(&load_groups(&env).unwrap()[0]), ["tdd", "grill"]);

        let report = set_group(&env, &name, &[member("grill")], true).unwrap();
        assert_eq!(report.members, 1);
        assert_eq!(members_of(&load_groups(&env).unwrap()[0]), ["grill"]);
    }

    #[test]
    fn add_group_creates_then_appends_without_duplicates() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        let name = group("frontend");

        assert_eq!(add_group(&env, &name, &[member("tdd")]).unwrap().members, 1);
        assert_eq!(
            add_group(&env, &name, &[member("grill"), member("tdd")])
                .unwrap()
                .members,
            2
        );

        assert_eq!(members_of(&load_groups(&env).unwrap()[0]), ["tdd", "grill"]);
    }

    #[test]
    fn replace_group_members_requires_an_existing_group() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        let name = group("frontend");

        assert!(matches!(
            replace_group_members(&env, &name, &[member("tdd")]),
            Err(GroupError::GroupNotFound { .. })
        ));

        set_group(&env, &name, &[member("tdd"), member("grill")], false).unwrap();
        replace_group_members(&env, &name, &[member("tdd")]).unwrap();
        assert_eq!(members_of(&load_groups(&env).unwrap()[0]), ["tdd"]);
    }

    #[test]
    fn strip_group_references_removes_only_refs() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        write_groups(
            &env,
            "common = [\"tdd\"]\nfrontend = [\"@common\", \"grill\"]\n",
        );

        let report = strip_group_references(&env, &[group("common")]).unwrap();

        assert_eq!(report.stripped.len(), 1);
        assert_eq!(report.stripped[0].0.as_str(), "frontend");
        assert_eq!(report.stripped[0].1.as_str(), "common");
        let groups = load_groups(&env).unwrap();
        let frontend = groups
            .iter()
            .find(|g| g.name.as_str() == "frontend")
            .unwrap();
        assert_eq!(members_of(frontend), ["grill"]);
    }

    #[test]
    fn remove_group_deletes_it_and_keeps_the_file_valid() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        let name = group("frontend");
        set_group(&env, &name, &[member("tdd")], false).unwrap();

        let report = remove_group(&env, &name).unwrap();

        assert_eq!(report.path, groups_file(&env));
        assert!(load_groups(&env).unwrap().is_empty());
        assert!(matches!(
            remove_group(&env, &name),
            Err(GroupError::GroupNotFound { .. })
        ));
    }

    #[test]
    fn legacy_groups_are_read_when_groups_file_is_missing() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        write_legacy_config(
            &env,
            "lang = \"en\"\n\n[groups]\nfrontend = [\"@common\", \"tdd\"]\ncommon = [\"grill\"]\n",
        );

        let groups = load_groups(&env).unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(members_of(&groups[0]), ["grill"]);
        assert_eq!(members_of(&groups[1]), ["@common", "tdd"]);
        assert!(!groups_file(&env).exists());
    }

    #[test]
    fn first_write_migrates_legacy_groups_and_strips_them() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        write_legacy_config(
            &env,
            "# mine\nlang = \"zh\"\n\n# groups below\n[groups]\nfrontend = [\"tdd\"]\n",
        );

        let report = add_group(&env, &group("frontend"), &[member("grill")]).unwrap();

        assert_eq!(report.members, 2);
        assert!(report.legacy_leftover.is_none());
        assert_eq!(members_of(&load_groups(&env).unwrap()[0]), ["tdd", "grill"]);

        let config = fs::read_to_string(config_file(&env)).unwrap();
        assert!(!config.contains("[groups]"), "{config:?}");
        assert!(config.contains("lang = \"zh\""), "{config:?}");
        assert!(config.contains("# mine"), "{config:?}");
    }

    #[test]
    fn groups_file_wins_when_both_exist() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        write_legacy_config(&env, "[groups]\nfrontend = [\"tdd\"]\n");
        write_groups(&env, "frontend = [\"grill\"]\n");

        let groups = load_groups(&env).unwrap();

        assert_eq!(members_of(&groups[0]), ["grill"]);
    }

    #[test]
    fn broken_legacy_table_only_matters_until_groups_file_exists() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        write_legacy_config(&env, "groups = \"bad\"\n");

        assert!(matches!(load_groups(&env), Err(GroupError::Invalid { .. })));

        write_groups(&env, "frontend = [\"tdd\"]\n");
        assert_eq!(load_groups(&env).unwrap().len(), 1);
    }

    #[test]
    fn broken_groups_file_does_not_fall_back_to_legacy() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        write_legacy_config(&env, "[groups]\nfrontend = [\"tdd\"]\n");
        let path = write_groups(&env, "frontend = [\n");

        match load_groups(&env) {
            Err(GroupError::Parse { path: reported, .. }) => assert_eq!(reported, path),
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn group_writes_keep_comments_and_unknown_keys() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        let path = write_groups(&env, "# mine\n# groups below\nfrontend = [\"tdd\"]\n");

        set_group(&env, &group("frontend"), &[member("grill")], true).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# mine"), "{content:?}");
        assert!(content.contains("# groups below"), "{content:?}");
        assert!(content.contains("frontend = [\"grill\"]"), "{content:?}");
    }
}
