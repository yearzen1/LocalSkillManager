//! 配置类 TOML 文件的通用读改写：保留注释与未知键、同目录临时文件 + rename。
//!
//! `config.toml` 与 `groups.toml` 共用这一套流程；临时文件名按目标文件名生成，
//! 两个文件在同一目录下不会互相覆盖。

use std::path::{Path, PathBuf};

/// 读改写 TOML 文件时的失败。
#[derive(Debug)]
pub enum TomlFileError {
    /// 读取失败。
    Io {
        path: PathBuf,
        source: anyhow::Error,
    },
    /// 建目录 / 临时文件 / rename 失败。
    Write {
        path: PathBuf,
        source: anyhow::Error,
    },
    /// TOML 语法非法。
    Parse {
        path: PathBuf,
        source: anyhow::Error,
    },
}

/// 读现有文档；文件不存在时返回空文档，第二个返回值为 `true`。
pub fn read(path: &Path) -> Result<(toml_edit::DocumentMut, bool), TomlFileError> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
            Ok((parse(path, text)?, false))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Ok((toml_edit::DocumentMut::new(), true))
        }
        Err(err) => Err(TomlFileError::Io {
            path: path.to_path_buf(),
            source: anyhow::Error::new(err),
        }),
    }
}

/// 渲染并写盘；非空内容保证以换行结尾。
pub fn write(path: &Path, doc: &toml_edit::DocumentMut) -> Result<(), TomlFileError> {
    let mut rendered = doc.to_string();
    if !rendered.is_empty() && !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    write_rendered(path, &rendered)
}

/// 同目录临时文件 + rename；失败时清理临时文件，旧文件保持原样。
pub fn write_rendered(path: &Path, rendered: &str) -> Result<(), TomlFileError> {
    let parent = path.parent().ok_or_else(|| TomlFileError::Write {
        path: path.to_path_buf(),
        source: anyhow::anyhow!("config file has no parent directory"),
    })?;

    std::fs::create_dir_all(parent).map_err(|err| TomlFileError::Write {
        path: parent.to_path_buf(),
        source: anyhow::Error::new(err),
    })?;

    let temp = parent.join(temp_name(path));
    std::fs::write(&temp, rendered).map_err(|err| TomlFileError::Write {
        path: temp.clone(),
        source: anyhow::Error::new(err),
    })?;

    if let Err(err) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(TomlFileError::Write {
            path: path.to_path_buf(),
            source: anyhow::Error::new(err),
        });
    }

    Ok(())
}

/// `.<文件名>.tmp-<pid>`：同一目录下的不同文件各有各的临时名。
fn temp_name(path: &Path) -> String {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config.toml".to_string());
    format!(".{file_name}.tmp-{}", std::process::id())
}

fn parse(path: &Path, text: &str) -> Result<toml_edit::DocumentMut, TomlFileError> {
    text.parse::<toml_edit::DocumentMut>()
        .map_err(|err| TomlFileError::Parse {
            path: path.to_path_buf(),
            source: anyhow::Error::new(err),
        })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn missing_file_reads_as_empty_document() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("groups.toml");

        let (doc, is_new) = read(&path).unwrap();

        assert!(is_new);
        assert!(doc.is_empty());
    }

    #[test]
    fn read_strips_bom_and_reports_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("groups.toml");
        fs::write(&path, "\u{feff}a = [\"b\"]\n").unwrap();

        let (doc, is_new) = read(&path).unwrap();

        assert!(!is_new);
        assert!(doc.get("a").is_some());
    }

    #[test]
    fn write_keeps_comments_and_unknown_keys() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("groups.toml");
        fs::write(&path, "# mine\ntheme = \"dark\"\na = [\"b\"]\n").unwrap();

        let (mut doc, _) = read(&path).unwrap();
        doc["a"] = toml_edit::value("changed");
        write(&path, &doc).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# mine"), "{content:?}");
        assert!(content.contains("theme = \"dark\""), "{content:?}");
    }

    #[test]
    fn temp_files_are_named_after_the_target() {
        let dir = TempDir::new().unwrap();
        let groups = dir.path().join("groups.toml");
        let config = dir.path().join("config.toml");

        write_rendered(&groups, "a = []\n").unwrap();
        write_rendered(&config, "lang = \"en\"\n").unwrap();

        let leftovers: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains("tmp"))
            .collect();
        assert!(leftovers.is_empty(), "leftovers: {leftovers:?}");
        assert!(groups.exists());
        assert!(config.exists());
    }

    #[test]
    fn empty_document_writes_an_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        write(&path, &toml_edit::DocumentMut::new()).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "");
    }
}
