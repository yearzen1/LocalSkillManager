//! `config.toml` 的读写：TOML、保留注释与未知键、原子写入。错误为 [`ConfigError`]。

use std::path::PathBuf;

use crate::domain::lang::Lang;

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
    let path = paths::config_file(env).map_err(ConfigError::Path)?;
    let parent = path.parent().ok_or_else(|| ConfigError::Write {
        path: path.clone(),
        source: anyhow::anyhow!("config file has no parent directory"),
    })?;

    std::fs::create_dir_all(parent).map_err(|err| ConfigError::Write {
        path: parent.to_path_buf(),
        source: anyhow::Error::new(err),
    })?;

    let (mut doc, is_new) = read_document(&path)?;
    doc["lang"] = toml_edit::value(value.code());

    let mut rendered = doc.to_string();
    if is_new {
        rendered = format!("# Output language: en | zh\n{rendered}");
    }
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    let temp = parent.join(format!(".config.toml.tmp-{}", std::process::id()));
    std::fs::write(&temp, rendered).map_err(|err| ConfigError::Write {
        path: temp.clone(),
        source: anyhow::Error::new(err),
    })?;

    if let Err(err) = std::fs::rename(&temp, &path) {
        let _ = std::fs::remove_file(&temp);
        return Err(ConfigError::Write {
            path,
            source: anyhow::Error::new(err),
        });
    }

    Ok(path)
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

fn parse_document(path: &std::path::Path, text: &str) -> Result<toml_edit::DocumentMut, ConfigError> {
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
}

