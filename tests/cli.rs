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
fn list_sorts_by_name_and_truncates_long_descriptions() {
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
    assert!(text.contains(&format!("{}...", "z".repeat(60))));
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

// ---------- copy ----------

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
fn copy_moves_global_skill_into_project() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "my-skill", "Copy me.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["copy", "my-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("copied my-skill"));

    let copied = project.path().join(".agents/skills/my-skill/SKILL.md");
    assert!(copied.exists());
    assert!(fs::read_to_string(copied).unwrap().contains("Copy me."));
}

#[test]
fn copy_short_flag_works() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    install_global(&home, "my-skill", "Copy me.");

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["-c", "my-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("copied my-skill"));
}

#[test]
fn copy_missing_skill_points_at_list() {
    let home = home_dir();
    let project = TempDir::new().unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["copy", "nope"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not found in global"))
        .stderr(predicate::str::contains("lsm list global"));
}

#[test]
fn copy_rejects_invalid_source_skill() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    fs::create_dir_all(home.path().join("skills/broken")).unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["copy", "broken"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("is invalid"))
        .stderr(predicate::str::contains("missing SKILL.md"));
}

#[test]
fn copy_rejects_source_name_mismatch() {
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
        .args(["copy", "dir-name"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("name mismatch"));
}

#[test]
fn copy_refuses_existing_target_unless_forced() {
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
        .args(["copy", "my-skill"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("already exists"));

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["copy", "my-skill", "--force"])
        .assert()
        .success();

    let copied = project.path().join(".agents/skills/my-skill/SKILL.md");
    assert!(fs::read_to_string(copied).unwrap().contains("First."));
}

#[test]
fn copy_rejects_same_source_and_destination() {
    let root = TempDir::new().unwrap();
    let project = root.path().join("demo-project");
    let home = project.join(".agents");
    write_skill_md(&home.join("skills/my-skill/SKILL.md"), "my-skill", "Self.");

    lsm()
        .env("LSM_HOME", &home)
        .current_dir(&project)
        .args(["copy", "my-skill"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("same path"));
}

#[test]
fn copy_warns_about_git_directories() {
    let home = home_dir();
    let project = TempDir::new().unwrap();
    let skill = home.path().join("skills/my-skill");
    write_skill_md(&skill.join("SKILL.md"), "my-skill", "With git.");
    fs::create_dir_all(skill.join(".git")).unwrap();
    fs::write(skill.join(".git/HEAD"), "ref").unwrap();

    lsm()
        .env("LSM_HOME", home.path())
        .current_dir(project.path())
        .args(["copy", "my-skill"])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"));

    assert!(project
        .path()
        .join(".agents/skills/my-skill/.git/HEAD")
        .exists());
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
