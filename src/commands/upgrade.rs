//! `upgrade`：比较当前版本与最新 release。下载与替换在后续切片接入。

use std::cmp::Ordering;

use crate::domain::lang::Lang;
use crate::domain::version::Version;
use crate::error::AppError;
use crate::i18n::{tr_args, MsgKey};
use crate::Ctx;

/// 版本比较的结论（`upgrade.md` §1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// 已是最新。
    UpToDate { current: Version },
    /// release 比当前新。
    Available { current: Version, latest: Version },
    /// 当前版本更新（latest 更旧）。
    CurrentIsNewer { current: Version, latest: Version },
}

/// 编译期版本；`CARGO_PKG_VERSION` 由 `Cargo.toml` 保证合法。
pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION is a valid version")
}

/// `--check`：取最新 release 后比版本，不下载、不写盘。
pub fn check(_ctx: &Ctx) -> Result<Check, AppError> {
    unimplemented!("check latest release")
}

/// 纯比较：不联网、不排版。
pub fn decide(_current: Version, _latest: Version) -> Check {
    unimplemented!("compare versions")
}

/// 结论对应的单行文案。
pub fn render(_check: Check, _lang: Lang) -> String {
    unimplemented!("render check result")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;
    use crate::domain::lang::Lang;
    use crate::infra::env::Env;
    use crate::infra::net::stub::Stub;

    fn context(endpoint: &str, dir: &TempDir) -> Ctx {
        Ctx {
            env: Env {
                home: Some(dir.path().join("agents")),
                config_dir: Some(dir.path().join("config")),
                lang: None,
                upgrade_api: Some(endpoint.to_string()),
            },
            lang: Lang::En,
            cwd: dir.path().to_path_buf(),
            lang_explicit: false,
        }
    }

    fn release_json(tag: &str) -> String {
        format!(r#"{{ "tag_name": "{tag}", "assets": [] }}"#)
    }

    /// 递归列出目录内容；`--check` 前后必须完全一致。
    fn snapshot(root: &Path) -> Vec<PathBuf> {
        fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                found.push(path.clone());
                if path.is_dir() {
                    walk(&path, found);
                }
            }
        }

        let mut found = Vec::new();
        walk(root, &mut found);
        found.sort();
        found
    }

    #[test]
    fn reports_up_to_date_without_touching_the_disk() {
        let dir = TempDir::new().expect("temp dir");
        let before = snapshot(dir.path());
        let stub = Stub::start(200, release_json(env!("CARGO_PKG_VERSION")));
        let ctx = context(stub.endpoint(), &dir);

        let check = check(&ctx).expect("should check");

        assert_eq!(
            check,
            Check::UpToDate {
                current: current_version()
            }
        );
        assert_eq!(
            render(check, Lang::En),
            format!("lsm {} is up to date", current_version())
        );

        stub.request();
        assert_eq!(snapshot(dir.path()), before);
    }

    #[test]
    fn reports_a_newer_release_without_downloading() {
        let dir = TempDir::new().expect("temp dir");
        let before = snapshot(dir.path());
        let stub = Stub::start(200, release_json("v99.0.0"));
        let ctx = context(stub.endpoint(), &dir);

        let check = check(&ctx).expect("should check");

        assert_eq!(
            check,
            Check::Available {
                current: current_version(),
                latest: Version::parse("v99.0.0").unwrap()
            }
        );
        assert_eq!(
            render(check, Lang::En),
            format!("new version available: v99.0.0 (current {})", current_version())
        );

        stub.request();
        assert_eq!(snapshot(dir.path()), before);
    }

    #[test]
    fn reports_a_current_version_that_is_newer_than_the_release() {
        let check = decide(current_version(), Version::parse("v0.0.1").unwrap());

        assert_eq!(
            check,
            Check::CurrentIsNewer {
                current: current_version(),
                latest: Version::parse("v0.0.1").unwrap()
            }
        );
        assert_eq!(
            render(check, Lang::En),
            format!("lsm {} is newer than v0.0.1", current_version())
        );
    }

    #[test]
    fn rate_limits_become_runtime_errors() {
        let dir = TempDir::new().expect("temp dir");
        let stub = Stub::start(403, "");
        let ctx = context(stub.endpoint(), &dir);

        let err = check(&ctx).expect_err("should fail");

        assert_eq!(err.exit_code(), 1);
        assert_eq!(err.msg.key, MsgKey::UpgradeRateLimited);
        stub.request();
    }
}
