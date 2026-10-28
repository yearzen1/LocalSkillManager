use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
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

// ---------- add ----------

fn home_dir() -> TempDir {
    TempDir::new().unwrap()
}

fn write_skill_md(path: &std::path::Path, name: &str, description: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        format!("---\nname: {name}\ndescription: {description}\n---\n\n# Body\n"),
    )
    .unwrap();
}

#[test]
fn add_markdown_file_to_global_installs_byte_identical_copy() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("my-skill.md");
    write_skill_md(&source, "my-skill", "Does one thing.");

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .success()
        .stdout(predicate::str::contains("installed my-skill"));

    let installed = home.path().join("skills").join("my-skill").join("SKILL.md");
    assert_eq!(
        fs::read_to_string(&installed).unwrap(),
        fs::read_to_string(&source).unwrap()
    );
}

#[test]
fn add_markdown_file_to_project_uses_cwd() {
    let project = TempDir::new().unwrap();
    let source = project.path().join("my-skill.md");
    write_skill_md(&source, "my-skill", "Does one thing.");

    lsm()
        .current_dir(project.path())
        .args(["add", "my-skill.md", "project"])
        .assert()
        .success();

    assert!(project
        .path()
        .join(".agents/skills/my-skill/SKILL.md")
        .exists());
}

#[test]
fn add_directory_copies_everything() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("my-skill");
    write_skill_md(&source.join("SKILL.md"), "my-skill", "Does one thing.");
    fs::create_dir_all(source.join("scripts")).unwrap();
    fs::write(source.join("scripts/run.sh"), "run").unwrap();
    fs::create_dir_all(source.join("assets")).unwrap();
    fs::write(source.join("assets/logo.txt"), "logo").unwrap();
    fs::write(source.join(".hidden"), "hidden").unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .success();

    let installed = home.path().join("skills/my-skill");
    assert_eq!(
        fs::read_to_string(installed.join("scripts/run.sh")).unwrap(),
        "run"
    );
    assert_eq!(
        fs::read_to_string(installed.join("assets/logo.txt")).unwrap(),
        "logo"
    );
    assert_eq!(
        fs::read_to_string(installed.join(".hidden")).unwrap(),
        "hidden"
    );
}

#[test]
fn add_directory_with_git_warns_but_copies() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("my-skill");
    write_skill_md(&source.join("SKILL.md"), "my-skill", "Does one thing.");
    fs::create_dir_all(source.join(".git")).unwrap();
    fs::write(source.join(".git/HEAD"), "ref").unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"))
        .stderr(predicate::str::contains(".git"));

    assert!(home.path().join("skills/my-skill/.git/HEAD").exists());
}

#[test]
fn add_rejects_file_without_frontmatter() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("bad.md");
    fs::write(&source, "# no frontmatter\n").unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("frontmatter"));

    assert!(!home.path().join("skills").exists());
}

#[test]
fn add_rejects_invalid_or_oversized_name_and_description() {
    let home = home_dir();
    let work = TempDir::new().unwrap();

    let bad_name = work.path().join("bad-name.md");
    write_skill_md(&bad_name, "Bad-Name", "ok");
    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", bad_name.to_str().unwrap(), "global"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("invalid skill name"));

    let long_desc = work.path().join("long.md");
    fs::write(
        &long_desc,
        format!("---\nname: long\ndescription: {}\n---\n", "d".repeat(1025)),
    )
    .unwrap();
    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", long_desc.to_str().unwrap(), "global"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("too long"));
}

#[test]
fn duplicate_add_fails_and_force_replaces() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("my-skill.md");
    write_skill_md(&source, "my-skill", "First version.");

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .success();

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("already exists"))
        .stderr(predicate::str::contains("--force"));

    write_skill_md(&source, "my-skill", "Second version.");
    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global", "--force"])
        .assert()
        .success();

    let installed = home.path().join("skills/my-skill/SKILL.md");
    assert!(fs::read_to_string(&installed)
        .unwrap()
        .contains("Second version."));
}

#[test]
fn add_rejects_destination_inside_source() {
    let root = TempDir::new().unwrap();
    let project = root.path().join("demo-project");
    write_skill_md(&project.join("SKILL.md"), "demo-project", "Self copy.");

    lsm()
        .current_dir(&project)
        .args(["add", ".", "project"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("inside source"));
}

#[test]
fn add_rejects_directory_name_mismatch() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("dir-name");
    write_skill_md(&source.join("SKILL.md"), "other-name", "Mismatch.");

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("does not match"));
}

#[test]
fn add_rejects_non_markdown_file() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("skill.txt");
    fs::write(&source, "x").unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(".md"));
}

#[test]
fn add_reports_missing_source() {
    let home = home_dir();
    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", "nope.md", "global"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn short_flags_work_for_add() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("my-skill.md");
    write_skill_md(&source, "my-skill", "Short flags.");

    lsm()
        .env("LSM_HOME", home.path())
        .args(["-a", source.to_str().unwrap(), "-g"])
        .assert()
        .success()
        .stdout(predicate::str::contains("installed my-skill"));

    assert!(home.path().join("skills/my-skill/SKILL.md").exists());
}
