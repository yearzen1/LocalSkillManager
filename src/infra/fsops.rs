//! 文件系统操作：递归复制、原子安装、覆盖与自包含检测。错误为 [`FsError`]，不构造 `AppError`。

use std::path::{Path, PathBuf};

/// 文件系统操作失败。
#[derive(Debug)]
pub enum FsError {
    Io {
        path: PathBuf,
        source: anyhow::Error,
    },
    /// 遇到符号链接：不跳过、不跟随。
    SymlinkUnsupported { path: PathBuf },
    /// 目标已存在且未传 `--force`。
    AlreadyExists { path: PathBuf },
    /// 源路径不存在。
    SourceMissing { path: PathBuf },
    /// 源类型不符合该操作要求（目录操作收到文件，或反之）。
    SourceNotExpectedType { path: PathBuf },
}

/// 安装结果中需要在命令层转成 warning 的残留。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReport {
    /// 备份目录删除失败时保留其路径，由调用方打印 warning。
    pub leftover_backup: Option<PathBuf>,
}

/// 删除结果中需要在命令层转成 warning 的残留。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveReport {
    /// 临时目录删除失败时保留其路径，由调用方打印 warning。
    pub leftover_temp: Option<PathBuf>,
}

/// 源路径的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Directory,
    File,
}

/// 路径是否已存在（不跟随符号链接）。
pub fn is_present(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

/// 两个路径规范化后是否指向同一位置（目标可以尚不存在）。
pub fn is_same_path(left: &Path, right: &Path) -> Result<bool, FsError> {
    Ok(normalize(left)? == normalize(right)?)
}

/// 判定源类型：符号链接直接拒绝，其它非常规类型也拒绝。
pub fn inspect_source(src: &Path) -> Result<SourceKind, FsError> {
    let meta = std::fs::symlink_metadata(src).map_err(|err| source_error(src, err))?;

    if meta.file_type().is_symlink() {
        return Err(FsError::SymlinkUnsupported {
            path: src.to_path_buf(),
        });
    }

    if meta.is_dir() {
        Ok(SourceKind::Directory)
    } else if meta.is_file() {
        Ok(SourceKind::File)
    } else {
        Err(FsError::SourceNotExpectedType {
            path: src.to_path_buf(),
        })
    }
}

/// 递归复制，保留相对结构，不做任何排除。
pub fn copy_recursive(src: &Path, dst: &Path) -> Result<(), FsError> {
    let meta = std::fs::symlink_metadata(src).map_err(|err| source_error(src, err))?;
    let file_type = meta.file_type();

    if file_type.is_symlink() {
        return Err(FsError::SymlinkUnsupported {
            path: src.to_path_buf(),
        });
    }

    if file_type.is_dir() {
        std::fs::create_dir_all(dst).map_err(|err| io_error(dst, err))?;
        let mut entries = std::fs::read_dir(src)
            .map_err(|err| io_error(src, err))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| io_error(src, err))?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        }
        return Ok(());
    }

    if file_type.is_file() {
        std::fs::copy(src, dst).map_err(|err| io_error(dst, err))?;
        return Ok(());
    }

    Err(FsError::Io {
        path: src.to_path_buf(),
        source: anyhow::anyhow!("unsupported file type: neither file nor directory"),
    })
}

/// 判断 `dst` 是否位于 `src` 内部（`dst` 可以尚不存在）。
pub fn contains_path(src: &Path, dst: &Path) -> Result<bool, FsError> {
    let src = normalize(src)?;
    let dst = normalize(dst)?;
    Ok(dst != src && dst.starts_with(&src))
}

/// 目录源是否包含 `.git` 条目。
pub fn contains_git_dir(src: &Path) -> bool {
    src.join(".git").symlink_metadata().is_ok()
}

/// 原子安装整个 skill 目录。
pub fn install_skill_dir(src: &Path, dst: &Path, force: bool) -> Result<InstallReport, FsError> {
    ensure_source_type(src, SourceType::Directory)?;
    ensure_parent(dst)?;
    install_atomically(dst, force, |tmp| copy_recursive(src, tmp))
}

/// 原子安装单个 `.md` 文件为 `<dst>/SKILL.md`。
pub fn install_skill_file(src: &Path, dst: &Path, force: bool) -> Result<InstallReport, FsError> {
    ensure_source_type(src, SourceType::File)?;
    ensure_parent(dst)?;
    install_atomically(dst, force, |tmp| {
        std::fs::create_dir_all(tmp).map_err(|err| io_error(tmp, err))?;
        std::fs::copy(src, tmp.join("SKILL.md")).map_err(|err| io_error(src, err))?;
        Ok(())
    })
}

/// 原子删除目录或文件：先 rename 到同级临时目录，成功后再递归删除。
pub fn remove_entry_atomically(target: &Path) -> Result<RemoveReport, FsError> {
    let meta = std::fs::symlink_metadata(target).map_err(|err| source_error(target, err))?;

    if meta.file_type().is_symlink() {
        return Err(FsError::SymlinkUnsupported {
            path: target.to_path_buf(),
        });
    }

    let tmp = sibling(target, ".tmp-remove-")?;
    let _ = remove_existing(&tmp);

    if let Err(err) = std::fs::rename(target, &tmp) {
        return Err(io_error(target, err));
    }

    let leftover_temp = match remove_existing(&tmp) {
        Ok(()) => None,
        Err(_) => Some(tmp),
    };

    Ok(RemoveReport { leftover_temp })
}

/// 删除文件，忽略「不存在」与删除失败（升级临时文件清理用）。
pub fn discard(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// 原地替换的结果：旧文件删不掉时把路径带回给调用方打 warning。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceReport {
    pub leftover_backup: Option<PathBuf>,
}

/// 用 `new_binary` 原地替换 `exe`：路径与文件名不变（Q31）。
///
/// Windows 走「改名旧文件 → 放入新文件 → 删旧文件」，第二步失败会把旧文件改回来；
/// Unix 一步 `rename` 覆盖。
pub fn replace_self(_exe: &Path, _new_binary: &Path) -> Result<ReplaceReport, FsError> {
    unimplemented!("replace the running binary")
}

/// 清理上次升级留下的 `<exe>.old-*`；失败忽略。
pub fn clean_stale_backups(_exe: &Path) {
    unimplemented!("clean stale backups")
}

enum SourceType {
    Directory,
    File,
}

fn ensure_source_type(src: &Path, expected: SourceType) -> Result<(), FsError> {
    let meta = std::fs::symlink_metadata(src).map_err(|err| source_error(src, err))?;

    if meta.file_type().is_symlink() {
        return Err(FsError::SymlinkUnsupported {
            path: src.to_path_buf(),
        });
    }

    let matches = match expected {
        SourceType::Directory => meta.is_dir(),
        SourceType::File => meta.is_file(),
    };

    if !matches {
        return Err(FsError::SourceNotExpectedType {
            path: src.to_path_buf(),
        });
    }

    Ok(())
}

fn ensure_parent(dst: &Path) -> Result<(), FsError> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|err| io_error(parent, err))?;
    }
    Ok(())
}

/// 同级临时目录 → 构建 → 换位，任一步失败都不留残骸。
fn install_atomically(
    dst: &Path,
    force: bool,
    build: impl FnOnce(&Path) -> Result<(), FsError>,
) -> Result<InstallReport, FsError> {
    if dst.symlink_metadata().is_ok() && !force {
        return Err(FsError::AlreadyExists {
            path: dst.to_path_buf(),
        });
    }

    let tmp = sibling(dst, ".tmp-")?;
    let _ = remove_existing(&tmp);

    if let Err(err) = build(&tmp) {
        let _ = remove_existing(&tmp);
        return Err(err);
    }

    move_into_place(&tmp, dst, force)
}

fn move_into_place(tmp: &Path, dst: &Path, force: bool) -> Result<InstallReport, FsError> {
    let mut backup = None;

    if dst.symlink_metadata().is_ok() {
        if !force {
            let _ = remove_existing(tmp);
            return Err(FsError::AlreadyExists {
                path: dst.to_path_buf(),
            });
        }

        let bak = sibling(dst, ".bak-")?;
        let _ = remove_existing(&bak);
        if let Err(err) = std::fs::rename(dst, &bak) {
            let _ = remove_existing(tmp);
            return Err(io_error(dst, err));
        }
        backup = Some(bak);
    }

    if let Err(err) = std::fs::rename(tmp, dst) {
        if let Some(bak) = &backup {
            let _ = std::fs::rename(bak, dst);
        }
        let _ = remove_existing(tmp);
        return Err(io_error(dst, err));
    }

    let leftover_backup = match backup {
        Some(bak) if remove_existing(&bak).is_err() => Some(bak),
        _ => None,
    };

    Ok(InstallReport { leftover_backup })
}

/// `.tmp-<name>-<pid>` / `.bak-<name>-<pid>`：与目标同卷，保证 `rename` 原子性。
fn sibling(dst: &Path, prefix: &str) -> Result<PathBuf, FsError> {
    let parent = dst.parent().ok_or_else(|| FsError::Io {
        path: dst.to_path_buf(),
        source: anyhow::anyhow!("destination has no parent directory"),
    })?;
    let name = dst
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    Ok(parent.join(format!("{prefix}{name}-{}", std::process::id())))
}

/// 规范化路径：已存在的部分交给 `canonicalize`，不存在的后缀原样拼回。
fn normalize(path: &Path) -> Result<PathBuf, FsError> {
    let mut current = path.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();

    loop {
        match current.canonicalize() {
            Ok(base) => {
                let mut resolved = base;
                for part in tail.iter().rev() {
                    resolved.push(part);
                }
                return Ok(resolved);
            }
            Err(err) => match (current.file_name(), current.parent()) {
                (Some(name), Some(parent)) => {
                    tail.push(name.to_os_string());
                    current = parent.to_path_buf();
                }
                _ => return Err(io_error(&current, err)),
            },
        }
    }
}

fn remove_existing(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn io_error(path: &Path, err: std::io::Error) -> FsError {
    FsError::Io {
        path: path.to_path_buf(),
        source: anyhow::Error::new(err),
    }
}

/// 读源路径时区分「不存在」与其它 I/O 失败。
fn source_error(path: &Path, err: std::io::Error) -> FsError {
    if err.kind() == std::io::ErrorKind::NotFound {
        FsError::SourceMissing {
            path: path.to_path_buf(),
        }
    } else {
        io_error(path, err)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn leftovers(parent: &Path) -> Vec<String> {
        fs::read_dir(parent)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(".tmp-") || name.starts_with(".bak-"))
            .collect()
    }

    #[test]
    fn copies_nested_directories_and_hidden_files() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src");
        write(&src.join("SKILL.md"), "skill");
        write(&src.join("scripts/run.sh"), "run");
        write(&src.join(".hidden"), "hidden");
        write(&src.join("assets/deep/file.txt"), "deep");

        let dst = dir.path().join("dst");
        copy_recursive(&src, &dst).unwrap();

        assert_eq!(fs::read_to_string(dst.join("SKILL.md")).unwrap(), "skill");
        assert_eq!(
            fs::read_to_string(dst.join("scripts/run.sh")).unwrap(),
            "run"
        );
        assert_eq!(fs::read_to_string(dst.join(".hidden")).unwrap(), "hidden");
        assert_eq!(
            fs::read_to_string(dst.join("assets/deep/file.txt")).unwrap(),
            "deep"
        );
    }

    #[test]
    fn copies_everything_including_git() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src");
        write(&src.join("SKILL.md"), "skill");
        write(&src.join(".git/HEAD"), "ref");

        let dst = dir.path().join("dst");
        copy_recursive(&src, &dst).unwrap();

        assert_eq!(fs::read_to_string(dst.join(".git/HEAD")).unwrap(), "ref");
        assert!(contains_git_dir(&src));
        assert!(!contains_git_dir(&dst.join("missing")));
    }

    #[test]
    fn detects_missing_source() {
        let dir = TempDir::new().unwrap();
        let err = copy_recursive(&dir.path().join("nope"), &dir.path().join("dst")).unwrap_err();
        assert!(matches!(err, FsError::SourceMissing { .. }));
    }

    #[test]
    fn contains_path_detects_children_even_when_missing() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("project");
        fs::create_dir_all(&src).unwrap();

        assert!(contains_path(&src, &src.join(".agents/skills/x")).unwrap());
        assert!(!contains_path(&src, &dir.path().join("other")).unwrap());
    }

    #[test]
    fn contains_path_is_not_fooled_by_shared_prefix() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("project");
        let sibling = dir.path().join("project-2");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&sibling).unwrap();

        assert!(!contains_path(&src, &sibling).unwrap());
        assert!(!contains_path(&src, &src).unwrap());
    }

    #[test]
    fn installs_directory_into_new_destination() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src");
        write(&src.join("SKILL.md"), "skill");

        let dst = dir.path().join("skills/my-skill");
        let report = install_skill_dir(&src, &dst, false).unwrap();

        assert_eq!(report.leftover_backup, None);
        assert_eq!(fs::read_to_string(dst.join("SKILL.md")).unwrap(), "skill");
        assert!(leftovers(dst.parent().unwrap()).is_empty());
    }

    #[test]
    fn refuses_to_overwrite_without_force() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src");
        write(&src.join("SKILL.md"), "new");
        let dst = dir.path().join("skills/my-skill");
        write(&dst.join("SKILL.md"), "old");

        let err = install_skill_dir(&src, &dst, false).unwrap_err();

        assert!(matches!(err, FsError::AlreadyExists { .. }));
        assert_eq!(fs::read_to_string(dst.join("SKILL.md")).unwrap(), "old");
        assert!(leftovers(dst.parent().unwrap()).is_empty());
    }

    #[test]
    fn force_replaces_old_contents() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src");
        write(&src.join("SKILL.md"), "new");
        let dst = dir.path().join("skills/my-skill");
        write(&dst.join("SKILL.md"), "old");
        write(&dst.join("stale.txt"), "stale");

        let report = install_skill_dir(&src, &dst, true).unwrap();

        assert_eq!(report.leftover_backup, None);
        assert_eq!(fs::read_to_string(dst.join("SKILL.md")).unwrap(), "new");
        assert!(!dst.join("stale.txt").exists());
        assert!(leftovers(dst.parent().unwrap()).is_empty());
    }

    #[test]
    fn rejects_directory_source_for_file_install() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();

        let err = install_skill_file(&src, &dir.path().join("skills/x"), false).unwrap_err();
        assert!(matches!(err, FsError::SourceNotExpectedType { .. }));
    }

    #[test]
    fn installs_file_as_skill_md() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("foo.md");
        write(&src, "---\nname: foo\ndescription: Foo.\n---\nbody");

        let dst = dir.path().join("skills/foo");
        install_skill_file(&src, &dst, false).unwrap();

        assert_eq!(
            fs::read_to_string(dst.join("SKILL.md")).unwrap(),
            "---\nname: foo\ndescription: Foo.\n---\nbody"
        );
    }

    #[test]
    fn removes_entry_atomically() {
        let dir = TempDir::new().unwrap();
        let skills = dir.path().join("skills");
        let target = skills.join("my-skill");
        write(&target.join("SKILL.md"), "skill");

        let report = remove_entry_atomically(&target).unwrap();

        assert_eq!(report.leftover_temp, None);
        assert!(!target.exists());
        assert!(leftovers(&skills).is_empty());
    }

    #[test]
    fn remove_reports_missing_target() {
        let dir = TempDir::new().unwrap();

        let err = remove_entry_atomically(&dir.path().join("nope")).unwrap_err();

        assert!(matches!(err, FsError::SourceMissing { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symbolic_links() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src");
        write(&src.join("SKILL.md"), "skill");
        symlink(src.join("SKILL.md"), src.join("link.md")).unwrap();

        let err = copy_recursive(&src, &dir.path().join("dst")).unwrap_err();
        assert!(matches!(err, FsError::SymlinkUnsupported { .. }));
    }

    #[test]
    fn replaces_the_binary_in_place_without_renaming_it() {
        let dir = TempDir::new().unwrap();
        let exe = dir.path().join("myskillmgr.exe");
        let new_binary = dir.path().join(".lsm-upgrade-1234");
        write(&exe, "old");
        write(&new_binary, "new");

        let report = replace_self(&exe, &new_binary).expect("should replace");

        assert_eq!(fs::read_to_string(&exe).unwrap(), "new");
        assert!(report.leftover_backup.is_none());
        assert!(!new_binary.exists(), "temp binary must be consumed");
        let names: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["myskillmgr.exe"], "no versioned leftovers");
    }

    #[test]
    fn rolls_back_when_the_new_binary_is_missing() {
        let dir = TempDir::new().unwrap();
        let exe = dir.path().join("lsm.exe");
        write(&exe, "old");
        let missing = dir.path().join(".lsm-upgrade-404");

        let err = replace_self(&exe, &missing).expect_err("should fail");

        assert!(matches!(err, FsError::Io { .. }));
        assert_eq!(fs::read_to_string(&exe).unwrap(), "old");
        let stale: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".old-"))
            .collect();
        assert!(stale.is_empty(), "rollback must not leave backups: {stale:?}");
    }

    #[test]
    fn cleans_backups_left_by_an_earlier_upgrade() {
        let dir = TempDir::new().unwrap();
        let exe = dir.path().join("lsm.exe");
        write(&exe, "current");
        write(&dir.path().join("lsm.exe.old-99"), "stale");
        write(&dir.path().join("unrelated.txt"), "keep");

        clean_stale_backups(&exe);

        assert!(!dir.path().join("lsm.exe.old-99").exists());
        assert!(dir.path().join("unrelated.txt").exists());
    }

    #[cfg(windows)]
    #[test]
    fn reports_a_backup_that_cannot_be_deleted() {
        let dir = TempDir::new().unwrap();
        let exe = dir.path().join("lsm.exe");
        let new_binary = dir.path().join(".lsm-upgrade-7");
        write(&exe, "old");
        write(&new_binary, "new");

        let mut permissions = fs::metadata(&exe).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&exe, permissions).unwrap();

        let report = replace_self(&exe, &new_binary).expect("replacement itself succeeds");

        assert_eq!(fs::read_to_string(&exe).unwrap(), "new");
        let backup = report.leftover_backup.expect("backup must be reported");
        assert!(backup.exists());

        let mut permissions = fs::metadata(&backup).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&backup, permissions).unwrap();
    }
}
