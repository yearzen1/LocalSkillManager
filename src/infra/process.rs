//! 子进程调用：升级的冒烟校验要执行刚下载的二进制。

use std::fmt;
use std::path::{Path, PathBuf};

/// 执行外部二进制时的失败分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessError {
    /// 进程起不来：文件不存在、不可执行、没有执行权限。
    Spawn { path: PathBuf, detail: String },
    /// 进程退出码非 0。
    Failed { code: Option<i32> },
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::Spawn { path, detail } => {
                write!(f, "cannot run {}: {detail}", path.display())
            }
            ProcessError::Failed { code } => match code {
                Some(code) => write!(f, "exited with code {code}"),
                None => write!(f, "was terminated by a signal"),
            },
        }
    }
}

/// 当前可执行文件路径：升级要替换的就是它。
pub fn current_exe() -> Result<PathBuf, ProcessError> {
    std::env::current_exe().map_err(|err| ProcessError::Spawn {
        path: PathBuf::from("<current executable>"),
        detail: err.to_string(),
    })
}

/// 执行 `<exe> --version`，返回 stdout。
pub fn version_output(exe: &Path) -> Result<String, ProcessError> {
    let output = std::process::Command::new(exe)
        .arg("--version")
        .output()
        .map_err(|err| ProcessError::Spawn {
            path: exe.to_path_buf(),
            detail: err.to_string(),
        })?;

    if !output.status.success() {
        return Err(ProcessError::Failed {
            code: output.status.code(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_missing_binaries_as_spawn_failures() {
        let err = version_output(Path::new("no-such-binary-exists-here")).expect_err("should fail");

        assert!(matches!(err, ProcessError::Spawn { .. }), "{err:?}");
        assert!(err.to_string().contains("cannot run"));
    }

    #[test]
    fn renders_non_zero_exits() {
        assert_eq!(
            ProcessError::Failed { code: Some(2) }.to_string(),
            "exited with code 2"
        );
    }
}
