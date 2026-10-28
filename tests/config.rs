use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// 只用临时配置目录的干净命令。
fn lsm(config_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("lsm").unwrap();
    cmd.env("LSM_CONFIG_DIR", config_dir.path())
        .env_remove("LSM_LANG")
        .env_remove("LSM_HOME");
    cmd
}

fn config_file(config_dir: &TempDir) -> std::path::PathBuf {
    config_dir.path().join("config.toml")
}

#[test]
fn get_without_config_file_prints_default_english() {
    let dir = TempDir::new().unwrap();

    lsm(&dir)
        .args(["config", "get", "lang"])
        .assert()
        .success()
        .stdout("en\n");
}

#[test]
fn set_then_get_round_trips() {
    let dir = TempDir::new().unwrap();

    lsm(&dir)
        .args(["config", "set", "lang", "zh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("lang = zh"));

    let content = fs::read_to_string(config_file(&dir)).unwrap();
    assert!(content.contains("lang = \"zh\""));

    lsm(&dir)
        .args(["config", "get", "lang"])
        .assert()
        .success()
        .stdout("zh\n");
}

#[test]
fn set_normalizes_regional_tags() {
    let dir = TempDir::new().unwrap();

    lsm(&dir)
        .args(["config", "set", "lang", "zh-CN"])
        .assert()
        .success();

    let content = fs::read_to_string(config_file(&dir)).unwrap();
    assert!(content.contains("lang = \"zh\""));
    assert!(!content.contains("zh-CN"));
}

#[test]
fn stored_language_localizes_help() {
    let dir = TempDir::new().unwrap();

    lsm(&dir)
        .args(["config", "set", "lang", "zh"])
        .assert()
        .success();

    lsm(&dir)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("用法:"));
}

#[test]
fn explicit_lang_overrides_stored_value() {
    let dir = TempDir::new().unwrap();
    lsm(&dir)
        .args(["config", "set", "lang", "zh"])
        .assert()
        .success();

    lsm(&dir)
        .args(["--lang", "en", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn broken_config_is_a_runtime_error_with_path_and_hint() {
    let dir = TempDir::new().unwrap();
    let path = config_file(&dir);
    fs::write(&path, "lang = \"zh\n").unwrap();

    lsm(&dir)
        .arg("--help")
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("cannot parse config file"))
        .stderr(predicate::str::contains(path.display().to_string()))
        .stderr(predicate::str::contains("fix or delete"));
}

#[test]
fn explicit_language_bypasses_broken_config() {
    let dir = TempDir::new().unwrap();
    fs::write(config_file(&dir), "lang = \"zh\n").unwrap();

    lsm(&dir)
        .args(["--lang", "en", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));

    lsm(&dir)
        .env("LSM_LANG", "en")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn invalid_value_in_config_is_reported() {
    let dir = TempDir::new().unwrap();
    fs::write(config_file(&dir), "lang = \"fr\"\n").unwrap();

    lsm(&dir)
        .arg("--help")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("invalid language 'fr'"))
        .stderr(predicate::str::contains("allowed: en, zh"));
}

#[test]
fn broken_config_is_not_overwritten() {
    let dir = TempDir::new().unwrap();
    let path = config_file(&dir);
    fs::write(&path, "lang = \"zh\n").unwrap();

    lsm(&dir)
        .args(["config", "set", "lang", "en"])
        .assert()
        .code(1);

    assert_eq!(fs::read_to_string(&path).unwrap(), "lang = \"zh\n");
}

#[test]
fn config_get_keeps_stdout_empty_on_broken_file() {
    let dir = TempDir::new().unwrap();
    fs::write(config_file(&dir), "lang = \"zh\n").unwrap();

    lsm(&dir)
        .args(["config", "get", "lang"])
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty());
}

#[test]
fn unknown_keys_and_comments_survive_set() {
    let dir = TempDir::new().unwrap();
    let path = config_file(&dir);
    fs::write(&path, "# mine\ntheme = \"dark\"\nlang = \"en\"\n").unwrap();

    lsm(&dir)
        .args(["config", "set", "lang", "zh"])
        .assert()
        .success();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("# mine"));
    assert!(content.contains("theme = \"dark\""));
    assert!(content.contains("lang = \"zh\""));
}
