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

/// 执行 `<exe> --version`，返回 stdout。
pub fn version_output(_exe: &Path) -> Result<String, ProcessError> {
    unimplemented!("run --version")
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
