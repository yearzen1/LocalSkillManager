use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// 干净的命令：不继承开发机上的 LSM_* 变量。
fn lsm() -> Command {
    let mut cmd = Command::cargo_bin("lsm").unwrap();
    cmd.env_remove("LSM_LANG")
        .env_remove("LSM_HOME")
        .env_remove("LSM_CONFIG_DIR");
    cmd
}

#[test]
fn version_prints_package_version_and_exits_zero() {
    lsm()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")))
        .stdout(predicate::str::contains("lsm"));
}

#[test]
fn help_defaults_to_english() {
    lsm()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains(
            "Install a skill into global or project",
        ));
}

#[test]
fn help_follows_explicit_lang_flag() {
    lsm()
        .args(["--lang", "zh", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("用法:"))
        .stdout(predicate::str::contains("命令:"))
        .stdout(predicate::str::contains("安装 skill 到 global 或 project"))
        .stdout(predicate::str::contains("Usage:").not());
}

#[test]
fn help_follows_env_language() {
    lsm()
        .env("LSM_LANG", "zh-CN")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("用法:"));
}

#[test]
fn explicit_lang_beats_env_language() {
    lsm()
        .env("LSM_LANG", "zh")
        .args(["--lang=en", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("用法:").not());
}

#[test]
fn missing_subcommand_prints_help_to_stderr_and_exits_two() {
    lsm()
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Usage:"));
}

#[test]
fn missing_subcommand_is_localized() {
    lsm()
        .args(["--lang", "zh"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("用法:"));
}

#[test]
fn invalid_lang_is_a_usage_error_rendered_in_english() {
    lsm()
        .args(["--lang", "fr", "--help"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid language 'fr'"))
        .stderr(predicate::str::contains("hint:"));
}

#[test]
fn conflicting_scopes_exit_two() {
    lsm()
        .args(["-g", "-p"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("-g and -p cannot be combined"));
}

#[test]
fn copy_rejects_scope_flag() {
    lsm()
        .args(["-c", "foo", "-g"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("copy does not take -g or -p"));
}

#[test]
fn mixed_forms_exit_two() {
    lsm()
        .args(["add", "foo.md", "-g"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "cannot mix short flags with subcommands",
        ));
}

#[test]
fn short_flag_without_value_exits_two() {
    lsm()
        .arg("-a")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("'-a' requires a value"));
}

#[test]
fn copy_name_validation_blocks_path_traversal() {
    let project = TempDir::new().unwrap();

    for name in ["../evil", ".", "a/b", "My-Skill"] {
        lsm()
            .current_dir(project.path())
            .args(["copy", name])
            .assert()
            .code(2)
            .stderr(predicate::str::contains("invalid skill name"));
    }

    lsm()
        .current_dir(project.path())
        .args(["copy", "con"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("reserved on Windows"));

    assert!(!project.path().join(".agents").exists());
}

#[test]
fn config_without_subcommand_exits_two_but_explicit_help_exits_zero() {
    lsm()
        .arg("config")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Usage:"));

    lsm()
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage persistent settings"));
}

#[test]
fn config_set_rejects_unknown_language() {
    lsm()
        .args(["config", "set", "lang", "fr"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid language 'fr'"));
}

#[test]
fn config_rejects_unknown_key() {
    lsm().args(["config", "get", "scope"]).assert().code(2);
}
