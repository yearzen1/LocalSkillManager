use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;

static CONFIG_SEQ: AtomicU64 = AtomicU64::new(0);

/// 干净的命令：不继承开发机上的 LSM_* 变量，也不读真实的 config.toml。
fn lsm() -> Command {
    let mut cmd = Command::cargo_bin("lsm").unwrap();
    let config_dir = std::env::temp_dir().join(format!(
        "lsm-test-config-{}-{}",
        std::process::id(),
        CONFIG_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    cmd.env("LSM_CONFIG_DIR", config_dir)
        .env_remove("LSM_LANG")
        .env_remove("LSM_HOME");
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
fn pull_rejects_scope_flag() {
    lsm()
        .args(["pull", "foo", "-g"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("-g"));
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
fn pull_name_validation_blocks_path_traversal() {
    let project = TempDir::new().unwrap();

    for name in ["../evil", ".", "a/b", "My-Skill"] {
        lsm()
            .current_dir(project.path())
            .args(["pull", name])
            .assert()
            .code(2)
            .stderr(predicate::str::contains("invalid skill name"));
    }

    lsm()
        .current_dir(project.path())
        .args(["pull", "con"])
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

// ---------- list ----------

#[test]
fn list_empty_global_prints_hint_with_path() {
    let home = home_dir();

    lsm()
        .env("LSM_HOME", home.path())
        .args(["list", "global"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No skills found in"))
        .stdout(predicate::str::contains(
            home.path().join("skills").display().to_string(),
        ));
}

#[test]
fn list_empty_global_json_prints_empty_array() {
    let home = home_dir();

    lsm()
        .env("LSM_HOME", home.path())
        .args(["list", "global", "--json"])
        .assert()
        .success()
        .stdout("[]\n");
}

#[test]
fn list_renders_installed_skill() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("my-skill.md");
    write_skill_md(&source, "my-skill", "Does one thing.");

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .success();

    lsm()
        .env("LSM_HOME", home.path())
        .args(["list", "global"])
        .assert()
        .success()
        .stdout(predicate::str::contains("NAME"))
        .stdout(predicate::str::contains("my-skill"))
        .stdout(predicate::str::contains("Does one thing."));
}

#[test]
fn list_json_exposes_documented_fields() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("my-skill.md");
    write_skill_md(&source, "my-skill", "Does one thing.");

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .success();

    let output = lsm()
        .env("LSM_HOME", home.path())
        .args(["list", "global", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();

    let first = &value[0];
    assert_eq!(first["name"], "my-skill");
    assert_eq!(first["description"], "Does one thing.");
    assert_eq!(first["valid"], true);
    assert_eq!(
        first["path"],
        home.path()
            .join("skills")
            .join("my-skill")
            .display()
            .to_string()
    );
    assert!(first.get("error").is_none());
}

#[test]
fn list_marks_invalid_skill_directories() {
    let home = home_dir();
    let skills = home.path().join("skills");
    fs::create_dir_all(skills.join("broken")).unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .args(["list", "global"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[invalid]"))
        .stdout(predicate::str::contains("missing SKILL.md"));

    let output = lsm()
        .env("LSM_HOME", home.path())
        .args(["list", "global", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value[0]["name"], "broken");
    assert_eq!(value[0]["valid"], false);
    assert_eq!(value[0]["error"], "missing SKILL.md");
}

#[test]
fn list_marks_name_mismatch() {
    let home = home_dir();
    let skills = home.path().join("skills");
    write_skill_md(&skills.join("dir-name/SKILL.md"), "other-name", "Mismatch.");

    lsm()
        .env("LSM_HOME", home.path())
        .args(["list", "global"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "name mismatch: dir-name vs other-name",
        ));
}

#[test]
fn list_sorts_by_name_and_wraps_long_descriptions() {
    let home = home_dir();
    let skills = home.path().join("skills");
    write_skill_md(&skills.join("zeta/SKILL.md"), "zeta", &"z".repeat(70));
    write_skill_md(&skills.join("alpha/SKILL.md"), "alpha", "first");

    let stdout = lsm()
        .env("LSM_HOME", home.path())
        .args(["list", "global"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(stdout).unwrap();

    let alpha_at = text.find("alpha").unwrap();
    let zeta_at = text.find("zeta").unwrap();
    assert!(alpha_at < zeta_at, "entries must be sorted by name");
    // 条目之间空一行，换行后的 description 不会和下一个条目粘在一起
    assert!(
        text.contains("first\n\nzeta"),
        "entries must be separated by a blank line: {text:?}"
    );
    // 60 列换行后剩 10 个 z，续行从描述列（10 + 2）开始，不顶到第 0 列
    assert!(text.contains(&"z".repeat(60)));
    assert!(text.contains(&format!("            {}", "z".repeat(10))));
    assert!(!text.contains(&"z".repeat(61)));
}

#[test]
fn list_project_reads_cwd_agents_skills() {
    let project = TempDir::new().unwrap();
    write_skill_md(
        &project.path().join(".agents/skills/local-skill/SKILL.md"),
        "local-skill",
        "Project scope.",
    );

    lsm()
        .current_dir(project.path())
        .args(["list", "project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("local-skill"));
}

#[test]
fn list_short_flags_work() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("my-skill.md");
    write_skill_md(&source, "my-skill", "Short flags.");

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .success();

    lsm()
        .env("LSM_HOME", home.path())
        .args(["-l", "-g"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my-skill"));

    lsm()
        .env("LSM_HOME", home.path())
        .args(["-l", "-g", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["));
}

#[test]
fn list_json_stdout_contains_only_json() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("my-skill");
    write_skill_md(&source.join("SKILL.md"), "my-skill", "With git.");
    fs::create_dir_all(source.join(".git")).unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"));

    let stdout = lsm()
        .env("LSM_HOME", home.path())
        .args(["list", "global", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(serde_json::from_slice::<serde_json::Value>(&stdout).is_ok());
}

// ---------- pull ----------

fn install_global(home: &TempDir, name: &str, description: &str) {
    let work = TempDir::new().unwrap();
    let source = work.path().join(format!("{name}.md"));
    write_skill_md(&source, name, description);
    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .success();
}

#[test]
fn pull_moves_global_skill_into_project() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "my-skill", "Copy me.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "my-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pulled my-skill"));

    let copied = project.path().join(".agents/skills/my-skill/SKILL.md");
    assert!(copied.exists());
    assert!(fs::read_to_string(copied).unwrap().contains("Copy me."));
}

#[test]
fn copy_short_flag_is_removed() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "my-skill", "Copy me.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["-c", "my-skill"])
        .assert()
        .code(2);

    assert!(!project.path().join(".agents/skills/my-skill").exists());
}

#[test]
fn pull_all_output_is_localized() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "my-skill", "Copy me.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["--lang", "zh", "pull", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("成功 1 个，跳过 0 个，失败 0 个"));
}

#[test]
fn pull_rejects_name_together_with_all() {
    lsm()
        .args(["pull", "foo", "--all"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not both"));
}

#[test]
fn pull_requires_name_or_all() {
    lsm()
        .args(["pull"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("needs a skill name or --all"));
}

#[test]
fn copy_all_short_flag_is_removed() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "my-skill", "Copy me.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["-c", "--all"])
        .assert()
        .code(2);

    assert!(!project.path().join(".agents/skills/my-skill").exists());
}

#[test]
fn copy_command_is_removed() {
    lsm()
        .args(["copy", "my-skill"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("copy"));
}

#[test]
fn pull_missing_skill_points_at_list() {
    let home = home_dir();
    let project = TempDir::new().unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "nope"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not found in global"))
        .stderr(predicate::str::contains("lsm search nope global"));
}

#[test]
fn pull_rejects_invalid_source_skill() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    fs::create_dir_all(home.path().join("skills/broken")).unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "broken"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("is invalid"))
        .stderr(predicate::str::contains("missing SKILL.md"));
}

#[test]
fn pull_rejects_source_name_mismatch() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    write_skill_md(
        &home.path().join("skills/dir-name/SKILL.md"),
        "other-name",
        "Mismatch.",
    );

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "dir-name"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("name mismatch"));
}

#[test]
fn pull_refuses_existing_target_unless_forced() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "my-skill", "First.");
    write_skill_md(
        &project.path().join(".agents/skills/my-skill/SKILL.md"),
        "my-skill",
        "Local version.",
    );

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "my-skill"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("already exists"));

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "my-skill", "--force"])
        .assert()
        .success();

    let copied = project.path().join(".agents/skills/my-skill/SKILL.md");
    assert!(fs::read_to_string(copied).unwrap().contains("First."));
}

#[test]
fn pull_rejects_same_source_and_destination() {
    let root = TempDir::new().unwrap();
    let project = root.path().join("demo-project");
    let home = project.join(".agents");
    write_skill_md(&home.join("skills/my-skill/SKILL.md"), "my-skill", "Self.");

    lsm()
        .env("LSM_HOME", &home)
        .current_dir(&project)
        .args(["pull", "my-skill"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("same path"));
}

#[test]
fn pull_warns_about_git_directories() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    let skill = home.path().join("skills/my-skill");
    write_skill_md(&skill.join("SKILL.md"), "my-skill", "With git.");
    fs::create_dir_all(skill.join(".git")).unwrap();
    fs::write(skill.join(".git/HEAD"), "ref").unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "my-skill"])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"));

    assert!(project
        .path()
        .join(".agents/skills/my-skill/.git/HEAD")
        .exists());
}

#[test]
fn pull_all_copies_every_global_skill() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "alpha", "First.");
    install_global(&home, "beta", "Second.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pulled 2, skipped 0, failed 0"));

    for name in ["alpha", "beta"] {
        let copied = project
            .path()
            .join(format!(".agents/skills/{name}/SKILL.md"));
        assert!(copied.exists(), "{name} should have been copied");
    }
}

#[test]
fn pull_all_with_empty_global_succeeds_without_touching_project() {
    let home = home_dir();
    let project = TempDir::new().unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("no global skills to pull"));

    assert!(!project.path().join(".agents/skills").exists());
}

#[test]
fn pull_all_reports_conflicts_and_keeps_going() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "alpha", "From global.");
    install_global(&home, "beta", "From global.");
    write_skill_md(
        &project.path().join(".agents/skills/alpha/SKILL.md"),
        "alpha",
        "Local version.",
    );

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pulled 1, skipped 1, failed 0"))
        .stderr(predicate::str::contains("skipped alpha"));

    // 冲突不中断：后面的 skill 照常复制，冲突目标保持原样。
    assert!(project.path().join(".agents/skills/beta/SKILL.md").exists());
    let alpha = project.path().join(".agents/skills/alpha/SKILL.md");
    assert!(fs::read_to_string(alpha)
        .unwrap()
        .contains("Local version."));

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "--all", "--force"])
        .assert()
        .success();

    let alpha = project.path().join(".agents/skills/alpha/SKILL.md");
    assert!(fs::read_to_string(alpha).unwrap().contains("From global."));
}

// ---------- remove ----------

fn install_project(project: &TempDir, name: &str, description: &str) {
    let work = TempDir::new().unwrap();
    let source = work.path().join(format!("{name}.md"));
    write_skill_md(&source, name, description);
    lsm()
        .current_dir(project.path())
        .args(["add", source.to_str().unwrap(), "project"])
        .assert()
        .success();
}

#[test]
fn remove_deletes_project_skill() {
    let project = TempDir::new().unwrap();
    install_project(&project, "my-skill", "Remove me.");

    lsm()
        .current_dir(project.path())
        .args(["remove", "my-skill", "project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed my-skill"));

    assert!(!project.path().join(".agents/skills/my-skill").exists());
}

#[test]
fn remove_missing_skill_exits_one() {
    let home = home_dir();
    let project = TempDir::new().unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["remove", "nope", "project"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn remove_refuses_non_skill_dir_unless_forced() {
    let project = TempDir::new().unwrap();
    let stray = project.path().join(".agents/skills/odd");
    fs::create_dir_all(stray.join("nested")).unwrap();
    fs::write(stray.join("nested/file.txt"), "keep").unwrap();

    lsm()
        .current_dir(project.path())
        .args(["remove", "odd", "project"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("odd"));
    assert!(stray.join("nested/file.txt").exists());

    lsm()
        .current_dir(project.path())
        .args(["remove", "odd", "project", "--force"])
        .assert()
        .success();
    assert!(!stray.exists());
}

#[test]
fn remove_requires_scope() {
    lsm()
        .args(["remove", "foo"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("SCOPE").or(predicate::str::contains("scope")));
}

#[test]
fn remove_rejects_path_traversal() {
    lsm()
        .args(["remove", "..", "project"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid skill name"));
}

#[test]
fn remove_short_flag_works() {
    let project = TempDir::new().unwrap();
    install_project(&project, "my-skill", "Remove me.");

    lsm()
        .current_dir(project.path())
        .args(["-r", "my-skill", "-p"])
        .assert()
        .success();

    assert!(!project.path().join(".agents/skills/my-skill").exists());
}

#[test]
fn invalid_yaml_hints_how_to_fix() {
    let home = home_dir();
    let work = TempDir::new().unwrap();
    let source = work.path().join("colon.md");
    fs::write(
        &source,
        "---\nname: colon\ndescription: works: badly\n---\n",
    )
    .unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .args(["add", source.to_str().unwrap(), "global"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("invalid YAML"))
        .stderr(predicate::str::contains("quote the value"));
}

#[test]
fn invalid_subcommand_reports_only_the_name() {
    lsm()
        .args(["--lang", "zh", "symlink"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("非法的子命令 'symlink'"))
        .stderr(predicate::str::contains("unrecognized").not());
}

#[test]
fn invalid_subcommand_reports_only_the_name_in_english() {
    lsm()
        .args(["--lang", "en", "symlink"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid subcommand 'symlink'"))
        .stderr(predicate::str::contains("unrecognized").not());
}

/// `docs/impl/cli.md` §2.1 的命令表是真源：这里只校验「命令存在性」与「状态列」一致。
const CLI_REFERENCE: &str = include_str!("../docs/impl/cli.md");

fn cli_reference_rows() -> Vec<(String, String)> {
    let mut rows = Vec::new();
    let mut in_table = false;

    for line in CLI_REFERENCE.lines() {
        let trimmed = line.trim();

        if !in_table {
            in_table = trimmed.starts_with("| 命令") && trimmed.contains("状态");
            continue;
        }

        if !trimmed.starts_with('|') {
            break;
        }

        let cells: Vec<&str> = trimmed.split('|').map(str::trim).collect();
        if cells.len() < 5 {
            continue;
        }

        let status = cells[3].to_string();
        if status.starts_with("---") {
            continue;
        }

        rows.push((cells[1].trim_matches('`').to_string(), status));
    }

    rows
}

#[test]
fn cli_reference_table_matches_binary() {
    let rows = cli_reference_rows();
    assert!(
        rows.len() >= 6,
        "failed to parse the command table in docs/impl/cli.md: {rows:?}"
    );

    let mut implemented = 0;

    for (command, status) in &rows {
        let mut parts: Vec<&str> = command.split_whitespace().collect();

        match status.as_str() {
            "implemented" => {
                implemented += 1;
                parts.push("--help");
                lsm()
                    .args(&parts)
                    .assert()
                    .success()
                    .stdout(predicate::str::contains("Usage:"));
            }
            "planned" => {
                lsm()
                    .arg(parts[0])
                    .assert()
                    .code(2)
                    .stderr(predicate::str::contains(parts[0]));
            }
            other => panic!("unknown status '{other}' for command '{command}'"),
        }
    }

    assert!(implemented > 0, "no implemented rows parsed");
    // `planned` 行仍按未知子命令校验；当前没有 planned 命令时不做额外要求。
}

/// 解析 §2.1 的选项表，返回 (命令或 `全局`, 选项单元格)。
fn cli_option_rows() -> Vec<(String, String)> {
    let mut rows = Vec::new();
    let mut in_table = false;

    for line in CLI_REFERENCE.lines() {
        let trimmed = line.trim();

        if !in_table {
            in_table = trimmed.starts_with("| 命令") && trimmed.contains("选项");
            continue;
        }

        if !trimmed.starts_with('|') {
            break;
        }

        let cells: Vec<&str> = trimmed.split('|').map(str::trim).collect();
        if cells.len() < 4 || cells[1].starts_with("---") {
            continue;
        }

        rows.push((cells[1].trim_matches('`').to_string(), cells[2].to_string()));
    }

    rows
}

/// 把选项单元格里的长选项抽出来：`` `-h`, `--help` `` → `--help`。
fn long_options(cell: &str) -> Vec<String> {
    cell.split_whitespace()
        .map(|token| token.trim_matches(|character| character == '`' || character == ','))
        .filter(|token| token.starts_with("--"))
        .map(str::to_string)
        .collect()
}

/// 从 help 的 `Options:` 段抓出所有长选项（默认语言是英文）。
fn help_long_options(help: &str) -> Vec<String> {
    let mut options = Vec::new();
    let mut in_options = false;

    for line in help.lines() {
        if line.trim() == "Options:" {
            in_options = true;
            continue;
        }
        if !in_options {
            continue;
        }
        if !line.starts_with(' ') {
            break;
        }
        for token in line.split_whitespace() {
            let token = token.trim_end_matches(',');
            if token.starts_with("--") {
                options.push(token.to_string());
            }
        }
    }

    options
}

/// 跑 `<command> --help`（`None` 为根命令）并返回 stdout。
fn help_of(command: Option<&str>) -> String {
    let mut cmd = lsm();
    if let Some(command) = command {
        cmd.args(command.split_whitespace());
    }
    let output = cmd.arg("--help").output().expect("running lsm --help");

    assert!(output.status.success(), "`--help` failed for {command:?}");
    String::from_utf8(output.stdout).expect("help output is utf-8")
}

/// 选项表必须与二进制一致：文档写了什么，`--help` 就得有什么；反之亦然。
#[test]
fn cli_option_table_matches_help() {
    let rows = cli_option_rows();
    assert!(
        rows.len() >= 5,
        "failed to parse the option table in docs/impl/cli.md: {rows:?}"
    );

    let global: Vec<String> = rows
        .iter()
        .filter(|(command, _)| command == "全局")
        .flat_map(|(_, cell)| long_options(cell))
        .collect();
    assert!(!global.is_empty(), "option table has no 全局 rows");

    // `根命令` 的行只对 `lsm ...` 本身生效（如 `-V/--version`）。
    let root_only: Vec<String> = rows
        .iter()
        .filter(|(command, _)| command == "根命令")
        .flat_map(|(_, cell)| long_options(cell))
        .collect();
    let documented_root: Vec<String> = global.iter().cloned().chain(root_only).collect();

    let root_help = help_of(None);
    for option in &documented_root {
        assert!(
            root_help.contains(option.as_str()),
            "`{option}` is documented for the root command but missing from `lsm --help`"
        );
    }
    let root_options = help_long_options(&root_help);
    assert!(
        root_options.iter().any(|option| option == "--help"),
        "failed to parse options from `lsm --help`: {root_help:?}"
    );
    for option in root_options {
        assert!(
            documented_root.contains(&option),
            "`lsm --help` exposes `{option}` but the option table does not document it"
        );
    }

    let reference = cli_reference_rows();
    let known: Vec<String> = reference
        .iter()
        .map(|(command, _)| command.clone())
        .collect();
    let implemented: Vec<String> = reference
        .into_iter()
        .filter(|(_, status)| status == "implemented")
        .map(|(command, _)| command)
        .collect();

    // 表里出现的命令名必须是 `全局`、`根命令` 或命令表中的命令，拼错会静默错过检查。
    for (command, _) in &rows {
        assert!(
            command == "全局" || command == "根命令" || known.contains(command),
            "option table mentions unknown command `{command}`"
        );
    }

    // 按命令遍历而不是按表行遍历：漏写整行（如 `list` 一个选项都没写）也要能红。
    for command in &implemented {
        let documented: Vec<String> = global
            .iter()
            .cloned()
            .chain(
                rows.iter()
                    .filter(|(row_command, _)| row_command == command)
                    .flat_map(|(_, cell)| long_options(cell)),
            )
            .collect();

        let help = help_of(Some(command));
        let help_options = help_long_options(&help);
        assert!(
            help_options.iter().any(|option| option == "--help"),
            "failed to parse options from `{command} --help`: {help:?}"
        );

        for option in &documented {
            assert!(
                help.contains(option.as_str()),
                "`{command} {option}` is documented but missing from `{command} --help`"
            );
        }
        for option in help_options {
            assert!(
                documented.contains(&option),
                "`{command} --help` exposes `{option}` but the option table does not document it"
            );
        }
    }
}

// ---- upgrade（`quality.md` 37–44）----

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};

/// 起一个只回一次的 HTTP 桩，返回端点与线程句柄。
fn serve_stub(body: impl Into<Vec<u8>>) -> (String, JoinHandle<()>) {
    let body = body.into();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
    let address = listener.local_addr().expect("stub address");

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buffer = [0_u8; 4096];
        let _ = stream.read(&mut buffer).expect("read request");
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(header.as_bytes()).expect("write header");
        stream.write_all(&body).expect("write body");
        stream.flush().expect("flush");
    });

    (format!("http://{address}/metadata"), handle)
}

/// 复制一份 `lsm` 到临时目录，避免升级替换掉构建产物（Q42）。
fn upgrade_copy(dir: &TempDir) -> PathBuf {
    let name = if cfg!(windows) {
        "lsm-copy.exe"
    } else {
        "lsm-copy"
    };
    let dest = dir.path().join(name);
    fs::copy(env!("CARGO_BIN_EXE_lsm"), &dest).expect("copy binary");
    dest
}

fn current_tag() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

fn platform_asset() -> String {
    lsm::domain::release::asset_name(std::env::consts::OS, std::env::consts::ARCH)
        .expect("current platform must be in the asset table")
}

fn release_json(tag: &str, assets: &[(String, String)]) -> String {
    let assets: Vec<String> = assets
        .iter()
        .map(|(name, url)| format!(r#"{{ "name": "{name}", "browser_download_url": "{url}" }}"#))
        .collect();
    format!(
        r#"{{ "tag_name": "{tag}", "assets": [{}] }}"#,
        assets.join(", ")
    )
}

fn upgrade_cmd(exe: &Path, endpoint: &str, home: &Path) -> Command {
    let mut cmd = Command::new(exe);
    cmd.env("LSM_UPGRADE_API", endpoint)
        .env("LSM_HOME", home)
        .env_remove("LSM_LANG")
        .env("LSM_CONFIG_DIR", home.join("config"));
    cmd
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("read dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn upgrade_check_reports_up_to_date_without_writing() {
    let dir = TempDir::new().unwrap();
    let exe = upgrade_copy(&dir);
    let before = names_in(dir.path());
    let (endpoint, handle) = serve_stub(release_json(&current_tag(), &[]));

    upgrade_cmd(&exe, &endpoint, dir.path())
        .args(["upgrade", "--check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("up to date"))
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));

    handle.join().unwrap();
    assert_eq!(names_in(dir.path()), before, "--check must not write");
}

#[test]
fn upgrade_check_reports_a_newer_release_without_downloading() {
    let dir = TempDir::new().unwrap();
    let exe = upgrade_copy(&dir);
    let before = names_in(dir.path());
    let (endpoint, handle) = serve_stub(release_json("v99.0.0", &[]));

    upgrade_cmd(&exe, &endpoint, dir.path())
        .args(["upgrade", "--check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("new version available: v99.0.0"))
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));

    handle.join().unwrap();
    assert_eq!(names_in(dir.path()), before, "--check must not download");
}

#[test]
fn upgrade_force_replaces_the_binary_in_place() {
    let dir = TempDir::new().unwrap();
    let exe = upgrade_copy(&dir);
    let mut payload = fs::read(&exe).unwrap();
    payload.push(0);
    let (asset_url, asset_handle) = serve_stub(payload.clone());
    let asset = platform_asset();
    let (endpoint, meta_handle) = serve_stub(release_json(&current_tag(), &[(asset, asset_url)]));

    upgrade_cmd(&exe, &endpoint, dir.path())
        .args(["upgrade", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("upgraded lsm"));

    meta_handle.join().unwrap();
    asset_handle.join().unwrap();

    assert_eq!(fs::metadata(&exe).unwrap().len(), payload.len() as u64);

    let names = names_in(dir.path());
    assert!(
        names.iter().all(|name| !name.starts_with(".lsm-upgrade-")),
        "download temp file must be cleaned: {names:?}"
    );
    assert_eq!(names[0], exe.file_name().unwrap().to_string_lossy());

    // Windows 上正在运行的映像改名后仍被占用，旧文件留给下次升级清理（upgrade.md §5）。
    #[cfg(windows)]
    assert!(
        names.iter().all(|name| !name.contains(".old-")
            || name.starts_with(&*exe.file_name().unwrap().to_string_lossy())),
        "{names:?}"
    );
    #[cfg(not(windows))]
    assert_eq!(names.len(), 1, "{names:?}");
}

#[test]
fn upgrade_reports_a_missing_asset() {
    let dir = TempDir::new().unwrap();
    let exe = upgrade_copy(&dir);
    let (endpoint, handle) = serve_stub(release_json("v99.0.0", &[]));

    let expected = platform_asset();
    upgrade_cmd(&exe, &endpoint, dir.path())
        .arg("upgrade")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("v99.0.0"))
        .stderr(predicate::str::contains(expected));

    handle.join().unwrap();
}

#[test]
fn upgrade_maps_metadata_failures_to_hints() {
    for (status, body) in [(404_u16, ""), (200, "not json"), (403, "")] {
        let dir = TempDir::new().unwrap();
        let exe = upgrade_copy(&dir);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let body = body.to_string();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer).unwrap();
            let header = format!(
                "HTTP/1.1 {status} Stub\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(body.as_bytes()).unwrap();
            stream.flush().unwrap();
        });

        upgrade_cmd(&exe, &format!("http://{address}/metadata"), dir.path())
            .arg("upgrade")
            .assert()
            .code(1)
            .stderr(predicate::str::contains("releases"));

        handle.join().unwrap();
    }
}

#[cfg(unix)]
#[test]
fn upgrade_fails_cleanly_when_the_directory_is_not_writable() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let exe = upgrade_copy(&dir);
    let before = fs::read(&exe).unwrap();
    let mut payload = before.clone();
    payload.push(0);
    let (asset_url, asset_handle) = serve_stub(payload);
    let asset = platform_asset();
    let (endpoint, meta_handle) = serve_stub(release_json(&current_tag(), &[(asset, asset_url)]));

    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).unwrap();
    let assert = upgrade_cmd(&exe, &endpoint, dir.path())
        .args(["upgrade", "--force"])
        .assert()
        .code(1);
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();

    assert.stderr(predicate::str::contains("cannot replace"));
    assert_eq!(fs::read(&exe).unwrap(), before);
    let _ = meta_handle.join();
    let _ = asset_handle.join();
}

#[test]
fn upgrade_help_is_localized_and_unknown_flags_exit_two() {
    lsm()
        .args(["upgrade", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--check"))
        .stdout(predicate::str::contains("--force"));
    lsm()
        .args(["--lang", "zh", "upgrade", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--check"));
    lsm().arg("-u").assert().code(2);
    lsm().args(["upgrade", "--bogus"]).assert().code(2);
}

#[test]
fn upgrade_check_after_upgrade_reports_up_to_date() {
    let dir = TempDir::new().unwrap();
    let exe = upgrade_copy(&dir);
    let mut payload = fs::read(&exe).unwrap();
    payload.push(0);
    let (asset_url, asset_handle) = serve_stub(payload);
    let asset = platform_asset();
    let (endpoint, meta_handle) = serve_stub(release_json(&current_tag(), &[(asset, asset_url)]));

    upgrade_cmd(&exe, &endpoint, dir.path())
        .args(["upgrade", "--force"])
        .assert()
        .success();
    meta_handle.join().unwrap();
    asset_handle.join().unwrap();

    let (endpoint, handle) = serve_stub(release_json(&current_tag(), &[]));
    upgrade_cmd(&exe, &endpoint, dir.path())
        .args(["upgrade", "--check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("up to date"));
    handle.join().unwrap();
}

// ---------- v0.4.0: search ----------

#[test]
fn search_matches_name_and_description_case_insensitively() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "code-review", "Review a diff for correctness.");
    install_global(&home, "docs", "Write project documentation.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["search", "REVIEW", "global"])
        .assert()
        .success()
        .stdout(predicate::str::contains("code-review"))
        .stdout(predicate::str::contains("docs").not());

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["search", "documentation", "global"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docs"))
        .stdout(predicate::str::contains("code-review").not());
}

#[test]
fn search_json_reuses_list_fields() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "code-review", "Review a diff.");

    let output = lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["search", "code", "global", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value[0]["name"], "code-review");
    assert_eq!(value[0]["description"], "Review a diff.");
    assert_eq!(value[0]["valid"], true);
}

#[test]
fn search_without_matches_exits_zero() {
    let home = home_dir();
    let project = TempDir::new().unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["search", "zzz", "global"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No skills matched 'zzz' in global",
        ));

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["search", "zzz", "global", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^\[\]\s*$").unwrap());
}

#[test]
fn search_rejects_empty_query_and_missing_scope() {
    lsm()
        .args(["search", "", "global"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("non-empty query"));

    lsm()
        .args(["search", "foo"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("SCOPE"));
}

// ---------- v0.4.0: push ----------

#[test]
fn push_copies_project_skill_to_global() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_project(&project, "my-skill", "Push me.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["push", "my-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pushed my-skill"));

    let target = home.path().join("skills/my-skill/SKILL.md");
    assert!(target.exists());
    assert!(fs::read_to_string(target).unwrap().contains("Push me."));
}

#[test]
fn push_refuses_existing_global_skill_unless_forced() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_project(&project, "my-skill", "Project copy.");
    install_global(&home, "my-skill", "Global copy.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["push", "my-skill"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("already exists"));

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["push", "my-skill", "--force"])
        .assert()
        .success();

    let target = home.path().join("skills/my-skill/SKILL.md");
    assert!(fs::read_to_string(target)
        .unwrap()
        .contains("Project copy."));
}

#[test]
fn push_all_pushes_every_project_skill() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_project(&project, "alpha", "A.");
    install_project(&project, "beta", "B.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["push", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pushed 2, skipped 0, failed 0"));

    assert!(home.path().join("skills/alpha/SKILL.md").exists());
    assert!(home.path().join("skills/beta/SKILL.md").exists());
}

#[test]
fn push_missing_skill_points_at_project_list() {
    let home = home_dir();
    let project = TempDir::new().unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["push", "nope"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not found in project"))
        .stderr(predicate::str::contains("lsm list project"));
}

// ---------- v0.4.0: multi-name pull ----------

#[test]
fn pull_accepts_multiple_names() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "alpha", "A.");
    install_global(&home, "beta", "B.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "alpha", "beta"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pulled 2, skipped 0, failed 0"));

    assert!(project
        .path()
        .join(".agents/skills/alpha/SKILL.md")
        .exists());
    assert!(project.path().join(".agents/skills/beta/SKILL.md").exists());
}

#[test]
fn pull_multiple_keeps_going_and_reports_missing() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "alpha", "A.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "alpha", "missing"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not found in global"));

    assert!(project
        .path()
        .join(".agents/skills/alpha/SKILL.md")
        .exists());
}

#[test]
fn pull_multiple_skips_existing_without_force() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "alpha", "A.");
    install_global(&home, "beta", "B.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "beta"])
        .assert()
        .success();

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "alpha", "beta"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pulled 1, skipped 1, failed 0"))
        .stderr(predicate::str::contains("skipped beta"));

    assert!(project
        .path()
        .join(".agents/skills/alpha/SKILL.md")
        .exists());
}

#[test]
fn pull_multiple_validates_all_names_before_pulling() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "alpha", "A.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "alpha", "../evil"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid skill name"));

    assert!(!project.path().join(".agents/skills/alpha").exists());
}

#[test]
fn pull_duplicate_names_are_deduplicated() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "alpha", "A.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["pull", "alpha", "alpha"])
        .assert()
        .success();

    assert!(project
        .path()
        .join(".agents/skills/alpha/SKILL.md")
        .exists());
}

// ---------- v0.4.0: multi-name remove ----------

#[test]
fn remove_accepts_multiple_names() {
    let project = TempDir::new().unwrap();
    install_project(&project, "alpha", "A.");
    install_project(&project, "beta", "B.");

    lsm()
        .current_dir(project.path())
        .args(["remove", "alpha", "beta", "project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed 2, failed 0"));

    assert!(!project.path().join(".agents/skills/alpha").exists());
    assert!(!project.path().join(".agents/skills/beta").exists());
}

#[test]
fn remove_multiple_keeps_going_and_reports_missing() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_project(&project, "alpha", "A.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["remove", "alpha", "missing", "project"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not found"));

    assert!(!project.path().join(".agents/skills/alpha").exists());
}

#[test]
fn remove_multiple_validates_all_names_before_deleting() {
    let project = TempDir::new().unwrap();
    install_project(&project, "alpha", "A.");

    lsm()
        .current_dir(project.path())
        .args(["remove", "alpha", "../evil", "project"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid skill name"));

    assert!(project
        .path()
        .join(".agents/skills/alpha/SKILL.md")
        .exists());
}

#[test]
fn remove_short_flag_accepts_multiple_names() {
    let project = TempDir::new().unwrap();
    install_project(&project, "alpha", "A.");
    install_project(&project, "beta", "B.");

    lsm()
        .current_dir(project.path())
        .args(["-r", "alpha", "beta", "-p"])
        .assert()
        .success();

    assert!(!project.path().join(".agents/skills/alpha").exists());
    assert!(!project.path().join(".agents/skills/beta").exists());
}

#[test]
fn remove_missing_skill_hints_at_search() {
    let project = TempDir::new().unwrap();

    lsm()
        .current_dir(project.path())
        .args(["remove", "nope", "project"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("lsm search nope project"));
}

// ---------- v0.4.0: help ----------

#[test]
fn search_and_push_help_exit_zero_and_unknown_flags_exit_two() {
    lsm().args(["search", "--help"]).assert().success();
    lsm().args(["push", "--help"]).assert().success();

    lsm()
        .args(["search", "query", "global", "--bogus"])
        .assert()
        .code(2);
    lsm().args(["push", "name", "--bogus"]).assert().code(2);
}
