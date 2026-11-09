//! 扫描 skills 目录，产出 `list` 用的 [`SkillEntry`]。非法项只标记，不中断。

use std::path::{Path, PathBuf};

use crate::domain::entry::SkillEntry;
use crate::domain::name::SkillName;

use super::frontmatter;
use super::fsops::FsError;

/// 扫描 `dir` 的直接子目录。目录不存在视为空。
pub fn scan_skills(dir: &Path) -> Result<Vec<SkillEntry>, FsError> {
    Ok(visible_dirs(dir)?
        .into_iter()
        .map(|(name, path)| build_entry(&name, &path))
        .collect())
}

/// 列出 `dir` 下的直接子目录名：跳过隐藏项、非目录与非法 skill 名，按字典序排序。
///
/// 只读目录名，不解析 `SKILL.md`；供 name 前缀解析使用（Q50）。
pub fn dir_names(dir: &Path) -> Result<Vec<String>, FsError> {
    let mut names: Vec<String> = visible_dirs(dir)?
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| SkillName::is_valid(name))
        .collect();
    names.sort();
    Ok(names)
}

/// 直接子目录（名字 + 路径），跳过 `list` 不展示的隐藏项；结果按名字排序。
fn visible_dirs(dir: &Path) -> Result<Vec<(String, PathBuf)>, FsError> {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(io_error(dir, err)),
    };

    let mut dirs = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(|err| io_error(dir, err))?;
        let name = entry.file_name().to_string_lossy().into_owned();

        // 临时目录、备份目录与隐藏条目一律跳过。
        if name.starts_with('.') {
            continue;
        }

        if !entry
            .file_type()
            .map_err(|err| io_error(dir, err))?
            .is_dir()
        {
            continue;
        }

        dirs.push((name, entry.path()));
    }

    dirs.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(dirs)
}

fn build_entry(name: &str, path: &Path) -> SkillEntry {
    let invalid = |error: String| SkillEntry {
        name: name.to_string(),
        description: String::new(),
        path: path.to_path_buf(),
        valid: false,
        error: Some(error),
    };

    if !SkillName::is_valid(name) {
        return invalid("invalid directory name".to_string());
    }

    match frontmatter::parse_skill_md(&path.join("SKILL.md")) {
        Ok(meta) if meta.name.as_str() == name => SkillEntry {
            name: name.to_string(),
            description: meta.description,
            path: path.to_path_buf(),
            valid: true,
            error: None,
        },
        Ok(meta) => invalid(format!("name mismatch: {name} vs {}", meta.name.as_str())),
        Err(err) => invalid(frontmatter::describe(&err)),
    }
}

fn io_error(path: &Path, err: std::io::Error) -> FsError {
    FsError::Io {
        path: path.to_path_buf(),
        source: anyhow::Error::new(err),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn skill_md(dir: &std::path::Path, content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn returns_empty_for_missing_directory() {
        let temp = TempDir::new().unwrap();
        assert!(scan_skills(&temp.path().join("nope")).unwrap().is_empty());
    }

    #[test]
    fn parses_valid_skill() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        skill_md(
            &root.join("my-skill"),
            "---\nname: my-skill\ndescription: One line.\n---\n",
        );

        let entries = scan_skills(root).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "my-skill");
        assert_eq!(entries[0].description, "One line.");
        assert!(entries[0].valid);
        assert_eq!(entries[0].error, None);
        assert_eq!(entries[0].path, root.join("my-skill"));
    }

    #[test]
    fn marks_missing_skill_file_invalid() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("broken")).unwrap();

        let entries = scan_skills(root).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "broken");
        assert!(!entries[0].valid);
        assert_eq!(entries[0].error.as_deref(), Some("missing SKILL.md"));
    }

    #[test]
    fn marks_name_mismatch_invalid() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        skill_md(
            &root.join("dir-name"),
            "---\nname: other-name\ndescription: One line.\n---\n",
        );

        let entries = scan_skills(root).unwrap();

        assert_eq!(entries[0].name, "dir-name");
        assert!(!entries[0].valid);
        assert_eq!(
            entries[0].error.as_deref(),
            Some("name mismatch: dir-name vs other-name")
        );
    }

    #[test]
    fn skips_dot_prefixed_entries_and_files() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        skill_md(
            &root.join(".tmp-my-skill-1"),
            "---\nname: .tmp-my-skill-1\ndescription: x\n---\n",
        );
        skill_md(&root.join("real"), "---\nname: real\ndescription: x\n---\n");
        fs::write(root.join("stray.txt"), "x").unwrap();

        let entries = scan_skills(root).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "real");
    }

    #[test]
    fn sorts_entries_by_name() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        for name in ["zeta", "alpha", "mike"] {
            skill_md(
                &root.join(name),
                &format!("---\nname: {name}\ndescription: x\n---\n"),
            );
        }

        let names: Vec<String> = scan_skills(root)
            .unwrap()
            .into_iter()
            .map(|entry| entry.name)
            .collect();

        assert_eq!(names, ["alpha", "mike", "zeta"]);
    }

    #[test]
    fn dir_names_returns_empty_for_missing_directory() {
        let temp = TempDir::new().unwrap();
        assert!(dir_names(&temp.path().join("nope")).unwrap().is_empty());
    }

    #[test]
    fn dir_names_skips_hidden_items_files_and_invalid_names() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join(".tmp-my-skill-1")).unwrap();
        fs::create_dir_all(root.join("broken")).unwrap();
        fs::create_dir_all(root.join("Not-A-Name")).unwrap();
        fs::write(root.join("stray.txt"), "x").unwrap();

        assert_eq!(dir_names(root).unwrap(), ["broken"]);
    }

    #[test]
    fn dir_names_sorts_by_name() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        for name in ["zeta", "alpha", "mike"] {
            fs::create_dir_all(root.join(name)).unwrap();
        }

        assert_eq!(dir_names(root).unwrap(), ["alpha", "mike", "zeta"]);
    }
}
