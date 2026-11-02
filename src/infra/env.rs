//! 进程环境快照：`LSM_HOME` / `LSM_CONFIG_DIR` / `LSM_LANG` / `LSM_UPGRADE_API` 的唯一读取点。

use std::ffi::OsString;
use std::path::PathBuf;

/// 启动时的环境快照。读取后不再变化，后续设置环境变量不影响已读值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Env {
    /// skills 根目录的上级，即 `.agents` 这一层：`LSM_HOME` 或 `~/.agents`。
    /// 无法确定家目录且未设置 `LSM_HOME` 时为 `None`。
    pub home: Option<PathBuf>,
    /// 配置文件所在目录：`LSM_CONFIG_DIR` 或 `dirs::config_dir()/lsm`。
    pub config_dir: Option<PathBuf>,
    /// `LSM_LANG` 原始值，非空才算设置了；解析交给 `cli::lang`。
    pub lang: Option<String>,
    /// `LSM_UPGRADE_API` 原始值，空串按未设置处理；合法性由 `infra::net` 判断。
    pub upgrade_api: Option<String>,
    /// cargo 的 bin 目录：`$CARGO_HOME/bin` 或 `~/.cargo/bin`；用于识别 cargo 安装的副本。
    pub cargo_bin_dir: Option<PathBuf>,
}

impl Env {
    /// 读取一次进程环境。
    pub fn read() -> Self {
        Self {
            home: non_empty_os("LSM_HOME")
                .map(PathBuf::from)
                .or_else(|| dirs::home_dir().map(|home| home.join(".agents"))),
            config_dir: non_empty_os("LSM_CONFIG_DIR")
                .map(PathBuf::from)
                .or_else(|| dirs::config_dir().map(|dir| dir.join("lsm"))),
            lang: non_empty_string("LSM_LANG"),
            upgrade_api: non_empty_string("LSM_UPGRADE_API"),
            cargo_bin_dir: non_empty_os("CARGO_HOME")
                .map(PathBuf::from)
                .or_else(|| dirs::home_dir().map(|home| home.join(".cargo")))
                .map(|dir| dir.join("bin")),
        }
    }
}

/// 空串按未设置处理。
fn non_empty_os(name: &str) -> Option<OsString> {
    std::env::var_os(name).filter(|value| !value.is_empty())
}

/// 非 UTF-8 的语言标签按未设置处理（后续解析也无法识别）。
fn non_empty_string(name: &str) -> Option<String> {
    non_empty_os(name).and_then(|value| value.into_string().ok())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// 进程环境是全局状态，env 用例必须串行。
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner())
    }

    #[test]
    fn picks_up_override_variables() {
        let _guard = lock();
        std::env::set_var("LSM_HOME", "/tmp/lsm-home");
        std::env::set_var("LSM_CONFIG_DIR", "/tmp/lsm-config");
        std::env::set_var("LSM_LANG", "zh-CN");
        std::env::set_var("LSM_UPGRADE_API", "http://127.0.0.1:9/api");

        let env = Env::read();

        assert_eq!(env.home, Some(PathBuf::from("/tmp/lsm-home")));
        assert_eq!(env.config_dir, Some(PathBuf::from("/tmp/lsm-config")));
        assert_eq!(env.lang.as_deref(), Some("zh-CN"));
        assert_eq!(env.upgrade_api.as_deref(), Some("http://127.0.0.1:9/api"));
        assert!(env
            .cargo_bin_dir
            .as_ref()
            .is_some_and(|dir| dir.ends_with("bin")));

        std::env::remove_var("LSM_HOME");
        std::env::remove_var("LSM_CONFIG_DIR");
        std::env::remove_var("LSM_LANG");
        std::env::remove_var("LSM_UPGRADE_API");
    }

    #[test]
    fn treats_empty_values_as_unset() {
        let _guard = lock();
        std::env::set_var("LSM_HOME", "");
        std::env::set_var("LSM_CONFIG_DIR", "");
        std::env::set_var("LSM_LANG", "");
        std::env::set_var("LSM_UPGRADE_API", "");

        let env = Env::read();

        assert!(env.lang.is_none());
        assert!(env.upgrade_api.is_none());
        assert!(env
            .home
            .as_ref()
            .is_some_and(|home| home.ends_with(".agents")));
        assert!(env
            .config_dir
            .as_ref()
            .is_some_and(|dir| dir.ends_with("lsm")));

        std::env::remove_var("LSM_HOME");
        std::env::remove_var("LSM_CONFIG_DIR");
        std::env::remove_var("LSM_LANG");
        std::env::remove_var("LSM_UPGRADE_API");
    }

    #[test]
    fn read_is_a_snapshot() {
        let _guard = lock();
        std::env::set_var("LSM_HOME", "/tmp/before");
        let env = Env::read();

        std::env::set_var("LSM_HOME", "/tmp/after");

        assert_eq!(env.home, Some(PathBuf::from("/tmp/before")));

        std::env::remove_var("LSM_HOME");
    }
}
