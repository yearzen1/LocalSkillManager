//! 纯函数路径解析：环境由 [`Env`] 传入，测试不需要修改进程环境。

use std::path::{Path, PathBuf};

use crate::domain::name::SkillName;
use crate::domain::scope::Scope;

use super::env::Env;

/// 路径解析失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// 未设置 `LSM_HOME` 且无法确定家目录。
    HomeUnavailable,
    /// 未设置 `LSM_CONFIG_DIR` 且无法确定配置目录。
    ConfigDirUnavailable,
}

/// `<base>/skills`：global 用 `LSM_HOME`，project 固定为 `<cwd>/.agents`。
pub fn skills_dir(env: &Env, scope: Scope, cwd: &Path) -> Result<PathBuf, PathError> {
    match scope {
        Scope::Global => Ok(env
            .home
            .as_ref()
            .ok_or(PathError::HomeUnavailable)?
            .join("skills")),
        Scope::Project => Ok(cwd.join(".agents").join("skills")),
    }
}

/// `<skills_dir>/<name>`。
pub fn skill_dir(
    env: &Env,
    scope: Scope,
    cwd: &Path,
    name: &SkillName,
) -> Result<PathBuf, PathError> {
    Ok(skills_dir(env, scope, cwd)?.join(name.as_str()))
}

/// `config.toml` 的完整路径。
pub fn config_file(env: &Env) -> Result<PathBuf, PathError> {
    Ok(env
        .config_dir
        .as_ref()
        .ok_or(PathError::ConfigDirUnavailable)?
        .join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_with(home: Option<&str>, config_dir: Option<&str>) -> Env {
        Env {
            home: home.map(PathBuf::from),
            config_dir: config_dir.map(PathBuf::from),
            lang: None,
        }
    }

    #[test]
    fn global_skills_dir_uses_lsm_home() {
        let env = env_with(Some("/tmp/agents"), None);
        let dir = skills_dir(&env, Scope::Global, Path::new("/tmp/project")).unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/agents").join("skills"));
    }

    #[test]
    fn global_skills_dir_fails_without_home() {
        let env = env_with(None, None);
        assert_eq!(
            skills_dir(&env, Scope::Global, Path::new("/tmp/project")),
            Err(PathError::HomeUnavailable)
        );
    }

    #[test]
    fn project_skills_dir_is_fixed_to_cwd() {
        let env = env_with(Some("/tmp/agents"), None);
        let dir = skills_dir(&env, Scope::Project, Path::new("/tmp/project")).unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/project/.agents").join("skills"));
    }

    #[test]
    fn project_skills_dir_ignores_missing_home() {
        let env = env_with(None, None);
        assert!(skills_dir(&env, Scope::Project, Path::new("/tmp/project")).is_ok());
    }

    #[test]
    fn skill_dir_appends_validated_name() {
        let env = env_with(Some("/tmp/agents"), None);
        let name = SkillName::parse("my-skill").unwrap();
        let dir = skill_dir(&env, Scope::Global, Path::new("/tmp/project"), &name).unwrap();
        assert_eq!(
            dir,
            PathBuf::from("/tmp/agents").join("skills").join("my-skill")
        );
    }

    #[test]
    fn config_file_lives_in_config_dir() {
        let env = env_with(None, Some("/tmp/config/lsm"));
        assert_eq!(
            config_file(&env).unwrap(),
            PathBuf::from("/tmp/config/lsm").join("config.toml")
        );
    }

    #[test]
    fn config_file_fails_without_config_dir() {
        let env = env_with(Some("/tmp/agents"), None);
        assert_eq!(config_file(&env), Err(PathError::ConfigDirUnavailable));
    }
}
