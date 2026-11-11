//! `config.toml` 的读写：TOML、保留注释与未知键、原子写入。错误为 [`ConfigError`]。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::domain::group::{Group, GroupName};
use crate::domain::lang::Lang;
use crate::domain::name::SkillName;

use super::env::Env;
use super::paths::{self, PathError};

/// 配置文件读写失败。
#[derive(Debug)]
pub enum ConfigError {
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
    /// `lang` 值不是受支持的语言。
    InvalidValue { path: PathBuf, raw: String },
    /// `[groups]` 不是「组名 → skill 名数组」的结构。
    InvalidGroups { path: PathBuf, detail: String },
    /// `group set` 要覆盖已存在的组但没加 `--force`。
    GroupExists { name: String },
    /// 要 `add` / `remove` 的组不存在。
    GroupNotFound { name: String },
}

/// 一次分组写盘的结果。
pub struct GroupSaveReport {
    pub path: PathBuf,
    pub members: usize,
}

/// 读取 `lang`。文件或键缺失返回 `None`，不是错误。
pub fn load(env: &Env) -> Result<Option<Lang>, ConfigError> {
    let path = paths::config_file(env).map_err(ConfigError::Path)?;

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(ConfigError::Io {
                path,
                source: anyhow::Error::new(err),
            })
        }
    };

    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let doc = parse_document(&path, text)?;

    let Some(item) = doc.get("lang") else {
        return Ok(None);
    };

    let raw = item
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| item.to_string().trim().to_string());

    match Lang::parse(&raw) {
        Ok(lang) => Ok(Some(lang)),
        Err(_) => Err(ConfigError::InvalidValue { path, raw }),
    }
}

/// 写入 `lang`（规范形式 `en`/`zh`），返回写入路径。
pub fn set_lang(env: &Env, value: Lang) -> Result<PathBuf, ConfigError> {
    let path = config_path(env)?;

    let (mut doc, is_new) = read_document(&path)?;
    doc["lang"] = toml_edit::value(value.code());

    let mut rendered = doc.to_string();
    if is_new {
        rendered = format!("# Output language: en | zh\n{rendered}");
    }
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    write_rendered(&path, &rendered)?;
    Ok(path)
}

/// 读取全部分组；文件或 `[groups]` 缺失返回空列表，结构非法才是错误。
pub fn load_groups(env: &Env) -> Result<Vec<Group>, ConfigError> {
    let path = config_path(env)?;
    let (doc, _) = read_document(&path)?;
    groups_from_document(&path, &doc)
}

/// 创建或整体替换一个分组；`force` 为假时拒绝覆盖已有组。
pub fn set_group(
    env: &Env,
    name: &GroupName,
    members: &[SkillName],
    force: bool,
) -> Result<GroupSaveReport, ConfigError> {
    let path = config_path(env)?;
    let (mut doc, _) = read_document(&path)?;
    let members = dedupe(members);

    {
        let table = groups_table_mut(&mut doc, &path)?;
        if !force && table.contains_key(name.as_str()) {
            return Err(ConfigError::GroupExists {
                name: name.as_str().to_string(),
            });
        }
        table.insert(name.as_str(), members_item(&members));
    }

    write_document(&path, &doc)?;
    Ok(GroupSaveReport {
        path,
        members: members.len(),
    })
}

/// 向分组追加成员；组不存在时自动创建，重复成员静默去重。
pub fn add_group(
    env: &Env,
    name: &GroupName,
    members: &[SkillName],
) -> Result<GroupSaveReport, ConfigError> {
    let path = config_path(env)?;
    let (mut doc, _) = read_document(&path)?;

    let merged = {
        let table = groups_table_mut(&mut doc, &path)?;
        let mut merged = match table.get(name.as_str()) {
            Some(item) => members_from_item(&path, name.as_str(), item)?,
            None => Vec::new(),
        };
        let mut seen: HashSet<SkillName> = merged.iter().cloned().collect();
        for member in members {
            if seen.insert(member.clone()) {
                merged.push(member.clone());
            }
        }
        table.insert(name.as_str(), members_item(&merged));
        merged
    };

    write_document(&path, &doc)?;
    Ok(GroupSaveReport {
        path,
        members: merged.len(),
    })
}

/// 删除一个分组；组不存在返回 [`ConfigError::GroupNotFound`]。
pub fn remove_group(env: &Env, name: &GroupName) -> Result<PathBuf, ConfigError> {
    let path = config_path(env)?;
    let (mut doc, _) = read_document(&path)?;

    let emptied = {
        let table = groups_table_mut(&mut doc, &path)?;
        if table.remove(name.as_str()).is_none() {
            return Err(ConfigError::GroupNotFound {
                name: name.as_str().to_string(),
            });
        }
        table.is_empty()
    };

    if emptied {
        doc.as_table_mut().remove("groups");
    }

    write_document(&path, &doc)?;
    Ok(path)
}

fn config_path(env: &Env) -> Result<PathBuf, ConfigError> {
    paths::config_file(env).map_err(ConfigError::Path)
}

fn groups_from_document(
    path: &Path,
    doc: &toml_edit::DocumentMut,
) -> Result<Vec<Group>, ConfigError> {
    let Some(item) = doc.get("groups") else {
        return Ok(Vec::new());
    };

    let table = item.as_table().ok_or_else(|| {
        invalid_groups(
            path,
            "groups must be a table of skill-name arrays".to_string(),
        )
    })?;

    let mut groups = Vec::new();
    for (raw_name, value) in table.iter() {
        let name = GroupName::parse(raw_name)
            .map_err(|_| invalid_groups(path, format!("invalid group name '{raw_name}'")))?;
        groups.push(Group {
            name,
            members: members_from_item(path, raw_name, value)?,
        });
    }

    groups.sort_by(|left, right| left.name.as_str().cmp(right.name.as_str()));
    Ok(groups)
}

fn members_from_item(
    path: &Path,
    group: &str,
    item: &toml_edit::Item,
) -> Result<Vec<SkillName>, ConfigError> {
    let array = item.as_array().ok_or_else(|| {
        invalid_groups(
            path,
            format!("group '{group}' must be an array of skill names"),
        )
    })?;

    let mut members = Vec::new();
    let mut seen = HashSet::new();
    for element in array.iter() {
        let raw = element.as_str().ok_or_else(|| {
            invalid_groups(path, format!("group '{group}' members must be strings"))
        })?;
        let member = SkillName::parse(raw).map_err(|_| {
            invalid_groups(
                path,
                format!("group '{group}' has invalid skill name '{raw}'"),
            )
        })?;
        if seen.insert(member.clone()) {
            members.push(member);
        }
    }

    Ok(members)
}

fn groups_table_mut<'a>(
    doc: &'a mut toml_edit::DocumentMut,
    path: &Path,
) -> Result<&'a mut toml_edit::Table, ConfigError> {
    if doc.get("groups").is_none() {
        doc["groups"] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    doc["groups"].as_table_mut().ok_or_else(|| {
        invalid_groups(
            path,
            "groups must be a table of skill-name arrays".to_string(),
        )
    })
}

fn members_item(members: &[SkillName]) -> toml_edit::Item {
    let mut array = toml_edit::Array::new();
    for member in members {
        array.push(member.as_str());
    }
    toml_edit::Item::Value(toml_edit::Value::Array(array))
}

fn dedupe(members: &[SkillName]) -> Vec<SkillName> {
    let mut seen = HashSet::new();
    members
        .iter()
        .filter(|member| seen.insert((*member).clone()))
        .cloned()
        .collect()
}

fn invalid_groups(path: &Path, detail: String) -> ConfigError {
    ConfigError::InvalidGroups {
        path: path.to_path_buf(),
        detail,
    }
}

fn write_document(path: &Path, doc: &toml_edit::DocumentMut) -> Result<(), ConfigError> {
    let mut rendered = doc.to_string();
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    write_rendered(path, &rendered)
}

/// 同目录临时文件 + rename；失败时清理临时文件，旧文件保持原样。
fn write_rendered(path: &Path, rendered: &str) -> Result<(), ConfigError> {
    let parent = path.parent().ok_or_else(|| ConfigError::Write {
        path: path.to_path_buf(),
        source: anyhow::anyhow!("config file has no parent directory"),
    })?;

    std::fs::create_dir_all(parent).map_err(|err| ConfigError::Write {
        path: parent.to_path_buf(),
        source: anyhow::Error::new(err),
    })?;

    let temp = parent.join(format!(".config.toml.tmp-{}", std::process::id()));
    std::fs::write(&temp, rendered).map_err(|err| ConfigError::Write {
        path: temp.clone(),
        source: anyhow::Error::new(err),
    })?;

    if let Err(err) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(ConfigError::Write {
            path: path.to_path_buf(),
            source: anyhow::Error::new(err),
        });
    }

    Ok(())
}

/// 读现有文档；文件不存在时返回空文档并标记为新建。
fn read_document(path: &std::path::Path) -> Result<(toml_edit::DocumentMut, bool), ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
            Ok((parse_document(path, text)?, false))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Ok((toml_edit::DocumentMut::new(), true))
        }
        Err(err) => Err(ConfigError::Io {
            path: path.to_path_buf(),
            source: anyhow::Error::new(err),
        }),
    }
}

fn parse_document(
    path: &std::path::Path,
    text: &str,
) -> Result<toml_edit::DocumentMut, ConfigError> {
    text.parse::<toml_edit::DocumentMut>()
        .map_err(|err| ConfigError::Parse {
            path: path.to_path_buf(),
            source: anyhow::Error::new(err),
        })
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

    fn write_config(dir: &TempDir, content: &str) -> PathBuf {
        let path = dir.path().join("lsm").join("config.toml");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn missing_file_means_unset() {
        let dir = TempDir::new().unwrap();
        assert_eq!(load(&env_in(&dir)).unwrap(), None);
    }

    #[test]
    fn missing_key_means_unset() {
        let dir = TempDir::new().unwrap();
        write_config(&dir, "theme = \"dark\"\n");
        assert_eq!(load(&env_in(&dir)).unwrap(), None);
    }

    #[test]
    fn reads_normalized_language() {
        let dir = TempDir::new().unwrap();
        write_config(&dir, "lang = \"zh\"\n");
        assert_eq!(load(&env_in(&dir)).unwrap(), Some(Lang::Zh));
    }

    #[test]
    fn ignores_unknown_keys() {
        let dir = TempDir::new().unwrap();
        write_config(&dir, "theme = \"dark\"\nlang = \"en\"\n");
        assert_eq!(load(&env_in(&dir)).unwrap(), Some(Lang::En));
    }

    #[test]
    fn strips_bom() {
        let dir = TempDir::new().unwrap();
        write_config(&dir, "\u{feff}lang = \"zh\"\n");
        assert_eq!(load(&env_in(&dir)).unwrap(), Some(Lang::Zh));
    }

    #[test]
    fn rejects_broken_toml() {
        let dir = TempDir::new().unwrap();
        write_config(&dir, "lang = \"zh\n");
        assert!(matches!(
            load(&env_in(&dir)),
            Err(ConfigError::Parse { .. })
        ));
    }

    #[test]
    fn rejects_unsupported_language_value() {
        let dir = TempDir::new().unwrap();
        write_config(&dir, "lang = \"fr\"\n");
        assert!(matches!(
            load(&env_in(&dir)),
            Err(ConfigError::InvalidValue { ref raw, .. }) if raw == "fr"
        ));
    }

    #[test]
    fn writes_new_file_with_header() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);

        let path = set_lang(&env, Lang::Zh).unwrap();

        assert_eq!(path, env.config_dir.as_ref().unwrap().join("config.toml"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Output language: en | zh"));
        assert!(content.contains("lang = \"zh\""));
        assert_eq!(load(&env).unwrap(), Some(Lang::Zh));
    }

    #[test]
    fn keeps_comments_and_unknown_keys() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            "# my settings\ntheme = \"dark\"\n\n# output language\nlang = \"en\"\n",
        );

        set_lang(&env_in(&dir), Lang::Zh).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# my settings"));
        assert!(content.contains("theme = \"dark\""));
        assert!(content.contains("# output language"));
        assert!(content.contains("lang = \"zh\""));
    }

    #[test]
    fn rewrites_same_value_without_error() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        write_config(&dir, "lang = \"zh\"\n");

        assert!(set_lang(&env, Lang::Zh).is_ok());
        assert_eq!(load(&env).unwrap(), Some(Lang::Zh));
    }

    #[test]
    fn refuses_to_overwrite_broken_file() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "lang = \"zh\n");
        let before = fs::read_to_string(&path).unwrap();

        assert!(matches!(
            set_lang(&env_in(&dir), Lang::En),
            Err(ConfigError::Parse { .. })
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
    }

    #[test]
    fn leaves_no_temp_file_behind() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);

        set_lang(&env, Lang::En).unwrap();

        let config_dir = env.config_dir.as_ref().unwrap();
        let leftovers: Vec<String> = fs::read_dir(config_dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains("tmp"))
            .collect();
        assert!(leftovers.is_empty(), "leftovers: {leftovers:?}");
    }

    fn group(name: &str) -> GroupName {
        GroupName::parse(name).unwrap()
    }

    fn skill(name: &str) -> SkillName {
        SkillName::parse(name).unwrap()
    }

    #[test]
    fn missing_groups_means_empty() {
        let dir = TempDir::new().unwrap();
        assert!(load_groups(&env_in(&dir)).unwrap().is_empty());

        write_config(&dir, "lang = \"en\"\n");
        assert!(load_groups(&env_in(&dir)).unwrap().is_empty());
    }

    #[test]
    fn reads_groups_sorted_and_dedupes_members_in_order() {
        let dir = TempDir::new().unwrap();
        write_config(
            &dir,
            "[groups]\nzeta = [\"b\", \"a\", \"b\"]\nalpha = [\"c\"]\n",
        );

        let groups = load_groups(&env_in(&dir)).unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name.as_str(), "alpha");
        assert_eq!(groups[0].members[0].as_str(), "c");
        assert_eq!(groups[1].name.as_str(), "zeta");
        let members: Vec<&str> = groups[1].members.iter().map(SkillName::as_str).collect();
        assert_eq!(members, ["b", "a"]);
    }

    #[test]
    fn rejects_invalid_groups_structure() {
        let cases = [
            "groups = \"bad\"\n",
            "[groups]\nfrontend = [1]\n",
            "[groups]\nFrontend = []\n",
            "[groups]\nfrontend = [\"Bad\"]\n",
            "[groups]\nfrontend = \"tdd\"\n",
        ];

        for content in cases {
            let dir = TempDir::new().unwrap();
            write_config(&dir, content);
            assert!(
                matches!(
                    load_groups(&env_in(&dir)),
                    Err(ConfigError::InvalidGroups { .. })
                ),
                "expected InvalidGroups for {content:?}"
            );
        }
    }

    #[test]
    fn set_group_writes_and_requires_force_to_replace() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        let name = group("frontend");

        let report = set_group(&env, &name, &[skill("tdd"), skill("grill")], false).unwrap();
        assert_eq!(report.members, 2);
        assert_eq!(load_groups(&env).unwrap()[0].members.len(), 2);

        assert!(matches!(
            set_group(&env, &name, &[skill("grill")], false),
            Err(ConfigError::GroupExists { .. })
        ));
        assert_eq!(load_groups(&env).unwrap()[0].members.len(), 2);

        let report = set_group(&env, &name, &[skill("grill")], true).unwrap();
        assert_eq!(report.members, 1);
        assert_eq!(load_groups(&env).unwrap()[0].members[0].as_str(), "grill");
    }

    #[test]
    fn add_group_creates_then_appends_without_duplicates() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        let name = group("frontend");

        let report = add_group(&env, &name, &[skill("tdd")]).unwrap();
        assert_eq!(report.members, 1);

        let report = add_group(&env, &name, &[skill("grill"), skill("tdd")]).unwrap();
        assert_eq!(report.members, 2);

        let groups = load_groups(&env).unwrap();
        let members: Vec<&str> = groups[0].members.iter().map(SkillName::as_str).collect();
        assert_eq!(members, ["tdd", "grill"]);
    }

    #[test]
    fn remove_group_deletes_it_and_drops_the_empty_table() {
        let dir = TempDir::new().unwrap();
        let env = env_in(&dir);
        let name = group("frontend");
        set_group(&env, &name, &[skill("tdd")], false).unwrap();

        remove_group(&env, &name).unwrap();

        let content = fs::read_to_string(config_file_path(&env)).unwrap();
        assert!(!content.contains("[groups]"), "{content:?}");
        assert!(matches!(
            remove_group(&env, &name),
            Err(ConfigError::GroupNotFound { .. })
        ));
    }

    #[test]
    fn group_writes_keep_comments_and_unknown_keys() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            "# mine\ntheme = \"dark\"\nlang = \"en\"\n\n# groups below\n",
        );

        set_group(&env_in(&dir), &group("frontend"), &[skill("tdd")], false).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# mine"), "{content:?}");
        assert!(content.contains("theme = \"dark\""), "{content:?}");
        assert!(content.contains("# groups below"), "{content:?}");
        assert!(content.contains("[groups]"), "{content:?}");
    }

    fn config_file_path(env: &Env) -> PathBuf {
        env.config_dir.as_ref().unwrap().join("config.toml")
    }
}
