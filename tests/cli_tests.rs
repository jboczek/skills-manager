use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn help_lists_the_v1_commands() {
    let output = Command::new(assert_cmd::cargo::cargo_bin!("skills-manager"))
        .arg("--help")
        .output()
        .expect("help command runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help output is utf8");
    for command in ["list", "scan", "import", "remove", "config", "doctor"] {
        assert!(stdout.contains(command), "missing {command} in help output");
    }
}

#[test]
fn running_without_args_exits_successfully() {
    let output = Command::new(assert_cmd::cargo::cargo_bin!("skills-manager"))
        .output()
        .expect("binary runs");

    assert!(output.status.success());
}

fn config_bin_with_home(home: &TempDir) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("skills-manager"));
    cmd.env("HOME", home.path());
    cmd
}

fn config_path_for_home(home: &TempDir) -> PathBuf {
    let output = config_bin_with_home(home)
        .args(["config", "path"])
        .output()
        .expect("config path runs");

    assert!(output.status.success());
    PathBuf::from(String::from_utf8(output.stdout).expect("utf8").trim())
}

#[test]
fn config_path_prints_a_path() {
    let home = TempDir::new().unwrap();
    let output = config_bin_with_home(&home)
        .args(["config", "path"])
        .output()
        .expect("config path runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.trim().ends_with("config.toml"),
        "expected a path ending in config.toml, got: {stdout}"
    );
}

#[test]
fn config_init_creates_config_file() {
    let home = TempDir::new().unwrap();
    let output = config_bin_with_home(&home)
        .args(["config", "init"])
        .output()
        .expect("config init runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Created config"),
        "expected 'Created config' message, got: {stdout}"
    );
}

#[test]
fn config_init_twice_does_not_overwrite() {
    let home = TempDir::new().unwrap();
    let bin = assert_cmd::cargo::cargo_bin!("skills-manager");

    Command::new(bin)
        .args(["config", "init"])
        .env("HOME", home.path())
        .output()
        .expect("first init runs");

    let output = Command::new(bin)
        .args(["config", "init"])
        .env("HOME", home.path())
        .output()
        .expect("second init runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("already exists"),
        "expected 'already exists' message, got: {stdout}"
    );
}

#[test]
fn config_show_when_no_file_prints_hint() {
    let home = TempDir::new().unwrap();
    let output = config_bin_with_home(&home)
        .args(["config", "show"])
        .output()
        .expect("config show runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("config init"),
        "expected hint to run 'config init', got: {stdout}"
    );
}

#[test]
fn config_show_after_init_prints_toml() {
    let home = TempDir::new().unwrap();
    let bin = assert_cmd::cargo::cargo_bin!("skills-manager");

    Command::new(bin)
        .args(["config", "init"])
        .env("HOME", home.path())
        .output()
        .expect("init runs");

    let output = Command::new(bin)
        .args(["config", "show"])
        .env("HOME", home.path())
        .output()
        .expect("show runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("[skills]"),
        "expected TOML output with [skills], got: {stdout}"
    );
    assert!(
        stdout.contains("[preferences]"),
        "expected [preferences] section"
    );
    assert!(stdout.contains("claude"), "expected claude agent in output");
}

#[test]
fn skills_manager_scan_no_config_prints_no_skills() {
    let home = TempDir::new().unwrap();
    let output = config_bin_with_home(&home)
        .args(["scan"])
        .output()
        .expect("scan runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains("No skills found."),
        "unexpected stdout: {stdout}"
    );
}

#[test]
fn skills_manager_scan_finds_skills() {
    let home = TempDir::new().unwrap();
    let skills_root = home.path().join("skill-sources");
    let skill_dir = skills_root.join("code-review");
    fs::create_dir_all(&skill_dir).expect("create skill dir");
    fs::write(skill_dir.join("SKILL.md"), "# Code review").expect("write skill file");

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = skills_root.to_string_lossy().into_owned();
    config.skills.scan_parent_dirs = vec![];
    config.skills.max_scan_depth = 10;

    let config_path = config_path_for_home(&home);
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config parent");
    fs::write(&config_path, config.to_toml().expect("config toml")).expect("write config");

    let output = config_bin_with_home(&home)
        .args(["scan"])
        .output()
        .expect("scan runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains("code-review"),
        "unexpected stdout: {stdout}"
    );
}

#[test]
fn skills_manager_list_no_config_prints_hint() {
    let home = TempDir::new().unwrap();
    let output = config_bin_with_home(&home)
        .args(["list"])
        .output()
        .expect("list runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains("config init"),
        "unexpected stdout: {stdout}"
    );
}

#[test]
fn skills_manager_list_with_empty_targets_prints_no_skills() {
    let home = TempDir::new().unwrap();
    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = home
        .path()
        .join("missing-sources")
        .to_string_lossy()
        .into_owned();
    config.skills.scan_parent_dirs = vec![];
    config.skills.max_scan_depth = 10;
    for agent in config.agents.values_mut() {
        agent.global_dir = home
            .path()
            .join(format!("missing-{}", agent.display_name.to_lowercase()))
            .to_string_lossy()
            .into_owned();
        agent.project_dir = None;
        agent.shared_target_ids.clear();
    }
    config.shared_targets.clear();

    let config_path = config_path_for_home(&home);
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config parent");
    fs::write(&config_path, config.to_toml().expect("config toml")).expect("write config");

    let output = config_bin_with_home(&home)
        .args(["list"])
        .output()
        .expect("list runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains("No skills found."),
        "unexpected stdout: {stdout}"
    );
}

fn write_config(home: &TempDir, config: &skills_manager::config::Config) {
    let config_path = config_path_for_home(home);
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config parent");
    fs::write(&config_path, config.to_toml().expect("config toml")).expect("write config");
}

#[test]
fn import_unknown_skill_prints_not_found() {
    let home = TempDir::new().unwrap();
    let skills_root = home.path().join("skill-sources");
    fs::create_dir_all(&skills_root).unwrap();

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = skills_root.to_string_lossy().into_owned();
    config.skills.scan_parent_dirs = vec![];
    config.skills.max_scan_depth = 10;
    write_config(&home, &config);

    let output = config_bin_with_home(&home)
        .args(["import", "nonexistent/skill"])
        .output()
        .expect("import runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.to_lowercase().contains("not found"),
        "unexpected stdout: {stdout}"
    );
}

#[test]
fn remove_unknown_skill_prints_not_in_inventory() {
    let home = TempDir::new().unwrap();
    let skills_root = home.path().join("skill-sources");
    fs::create_dir_all(&skills_root).unwrap();

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = skills_root.to_string_lossy().into_owned();
    config.skills.scan_parent_dirs = vec![];
    config.skills.max_scan_depth = 10;
    write_config(&home, &config);

    let output = config_bin_with_home(&home)
        .args(["remove", "nonexistent/skill"])
        .output()
        .expect("remove runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stdout_lower = stdout.to_lowercase();
    assert!(
        stdout_lower.contains("not found") || stdout_lower.contains("nothing to remove"),
        "unexpected stdout: {stdout}"
    );
}

#[test]
fn doctor_with_no_config_warns() {
    let home = TempDir::new().unwrap();
    let output = config_bin_with_home(&home)
        .args(["doctor"])
        .output()
        .expect("doctor runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stdout_lower = stdout.to_lowercase();
    assert!(
        stdout.contains("WARN") || stdout_lower.contains("no config"),
        "unexpected stdout: {stdout}"
    );
}

#[test]
fn doctor_with_valid_config_shows_checks() {
    let home = TempDir::new().unwrap();
    let skills_root = home.path().join("skill-sources");
    fs::create_dir_all(&skills_root).unwrap();

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = skills_root.to_string_lossy().into_owned();
    config.skills.scan_parent_dirs = vec![];
    config.skills.max_scan_depth = 10;
    for agent in config.agents.values_mut() {
        let dir = home
            .path()
            .join(format!("{}-skills", agent.display_name.to_lowercase()));
        fs::create_dir_all(&dir).unwrap();
        agent.global_dir = dir.to_string_lossy().into_owned();
        agent.project_dir = None;
        agent.shared_target_ids.clear();
    }
    config.shared_targets.clear();
    write_config(&home, &config);

    let output = config_bin_with_home(&home)
        .args(["doctor"])
        .output()
        .expect("doctor runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Config"), "unexpected stdout: {stdout}");
    assert!(stdout.contains("Git"), "unexpected stdout: {stdout}");
}

#[test]
fn import_non_interactive_with_ambiguous_skill() {
    let home = TempDir::new().unwrap();
    let skills_root = home.path().join("skill-sources");
    let first = skills_root.join("one").join("myskill");
    let second = skills_root.join("two").join("myskill");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();
    fs::write(first.join("SKILL.md"), "# first").unwrap();
    fs::write(second.join("SKILL.md"), "# second").unwrap();

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = skills_root.to_string_lossy().into_owned();
    config.skills.scan_parent_dirs = vec![];
    config.skills.max_scan_depth = 10;
    write_config(&home, &config);

    let output = config_bin_with_home(&home)
        .args(["import", "myskill"])
        .output()
        .expect("import runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stdout_lower = stdout.to_lowercase();
    assert!(
        stdout_lower.contains("ambiguous") || stdout_lower.contains("not found"),
        "unexpected stdout: {stdout}"
    );
}

#[test]
fn skills_manager_list_maps_shared_agents_target_to_codex_and_copilot() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let shared_skill = project
        .path()
        .join(".agents")
        .join("skills")
        .join("shared-skill");
    fs::create_dir_all(&shared_skill).expect("create shared skill dir");
    fs::write(shared_skill.join("SKILL.md"), "# Shared skill").expect("write skill file");

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = home
        .path()
        .join("missing-sources")
        .to_string_lossy()
        .into_owned();
    config.skills.scan_parent_dirs = vec![];
    for agent in config.agents.values_mut() {
        agent.global_dir = home
            .path()
            .join(format!("missing-{}", agent.display_name.to_lowercase()))
            .to_string_lossy()
            .into_owned();
        agent.project_dir = None;
    }

    let config_path = config_path_for_home(&home);
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config parent");
    fs::write(&config_path, config.to_toml().expect("config toml")).expect("write config");

    let output = config_bin_with_home(&home)
        .current_dir(project.path())
        .args(["list"])
        .output()
        .expect("list runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains("shared-skill"),
        "unexpected stdout: {stdout}"
    );
    assert!(stdout.contains("CODEX"), "unexpected stdout: {stdout}");
    assert!(stdout.contains("COPILOT"), "unexpected stdout: {stdout}");
    assert!(!stdout.contains("AGENTS"), "unexpected stdout: {stdout}");
}

#[test]
fn skills_manager_list_includes_global_agents_skills_with_existing_config() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let shared_skill = home
        .path()
        .join(".agents")
        .join("skills")
        .join("global-shared-skill");
    fs::create_dir_all(&shared_skill).expect("create global shared skill dir");
    fs::write(shared_skill.join("SKILL.md"), "# Global shared skill").expect("write skill file");

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = home
        .path()
        .join("missing-sources")
        .to_string_lossy()
        .into_owned();
    config.skills.scan_parent_dirs = vec![];
    for agent in config.agents.values_mut() {
        agent.global_dir = home
            .path()
            .join(format!("missing-{}", agent.display_name.to_lowercase()))
            .to_string_lossy()
            .into_owned();
        agent.project_dir = None;
    }

    let current_toml = config.to_toml().expect("config toml");
    let legacy_toml = current_toml.replace(
        "global_dir = \"~/.agents/skills\"\nproject_dir = \".agents/skills\"",
        "project_dir = \".agents\"",
    );
    assert_ne!(legacy_toml, current_toml);
    let config_path = config_path_for_home(&home);
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config parent");
    fs::write(config_path, legacy_toml).expect("write legacy config");

    let output = config_bin_with_home(&home)
        .current_dir(project.path())
        .args(["list"])
        .output()
        .expect("list runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains("global-shared-skill"),
        "unexpected stdout: {stdout}"
    );
    let row = stdout
        .lines()
        .find(|line| line.contains("global-shared-skill"))
        .expect("global shared skill row");
    assert_eq!(
        row.split_whitespace().collect::<Vec<_>>(),
        vec![
            "global-shared-skill",
            "unknown",
            "-",
            "✓",
            "✓",
            "global",
            "physical"
        ]
    );
    assert!(!stdout.contains("AGENTS"), "unexpected stdout: {stdout}");
}
