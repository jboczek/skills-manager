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
    for command in [
        "list", "scan", "source", "import", "remove", "config", "doctor",
    ] {
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
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd
}

#[test]
fn config_bin_with_home_isolates_xdg_config_home() {
    let home = TempDir::new().unwrap();
    let cmd = config_bin_with_home(&home);
    let expected = home.path().join(".config");

    let xdg_config_home = cmd
        .get_envs()
        .find(|(key, _)| *key == std::ffi::OsStr::new("XDG_CONFIG_HOME"))
        .and_then(|(_, value)| value.map(PathBuf::from));

    assert_eq!(xdg_config_home.as_deref(), Some(expected.as_path()));
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

    config_bin_with_home(&home)
        .args(["config", "init"])
        .output()
        .expect("first init runs");

    let output = config_bin_with_home(&home)
        .args(["config", "init"])
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

    config_bin_with_home(&home)
        .args(["config", "init"])
        .output()
        .expect("init runs");

    let output = config_bin_with_home(&home)
        .args(["config", "show"])
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
fn source_add_previews_without_mutating_in_non_interactive_mode() {
    let home = TempDir::new().unwrap();
    let remote = home.path().join("remote-skills");
    let central = home.path().join("managed-sources");
    fs::create_dir_all(&remote).unwrap();

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = central.to_string_lossy().into_owned();
    write_config(&home, &config);
    let remote_url = format!("file://{}", remote.display());

    let output = config_bin_with_home(&home)
        .args(["source", "add", &remote_url])
        .output()
        .expect("source add runs");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains(&format!("Source URL: {remote_url}")));
    assert!(stdout.contains(&format!(
        "Destination: {}",
        central.join("remote-skills").display()
    )));
    assert!(stderr.contains("non-interactive mode"));
    assert!(!central.exists(), "preview must not create central_dir");
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
fn config_show_normalizes_legacy_project_dirs_silently() {
    let home = TempDir::new().unwrap();
    let config_path = config_path_for_home(&home);
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(
        &config_path,
        r#"
[skills]
central_dir = "~/skills"
scan_parent_dirs = []
max_scan_depth = 10

[agents.claude]
display_name = "Claude"
global_dir = "~/.claude/skills"
project_dir = ".claude/skills"
enabled = true
shared_target_ids = []

[shared_targets]

[preferences]
default_connection = "symlink"
confirm_physical_delete = true
"#,
    )
    .unwrap();

    let output = config_bin_with_home(&home)
        .args(["config", "show"])
        .output()
        .expect("config show runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stdout.contains("project_dir"), "{stdout}");
    assert!(!stderr.contains("agents.claude.project_dir"), "{stderr}");
    assert!(!stderr.contains("ignored"), "{stderr}");
}

#[test]
fn doctor_omits_ignored_legacy_project_dir_warnings() {
    let home = TempDir::new().unwrap();
    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = home.path().join("skills").to_string_lossy().into_owned();
    let toml = config.to_toml().unwrap().replace(
        "global_dir = \"~/.claude/skills\"",
        "global_dir = \"~/.claude/skills\"\nproject_dir = \".claude/skills\"",
    );
    let config_path = config_path_for_home(&home);
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(config_path, toml).unwrap();

    let output = config_bin_with_home(&home)
        .args(["doctor"])
        .output()
        .expect("doctor runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("agents.claude.project_dir"), "{stdout}");
    assert!(
        !stdout.contains("ignored in global execution context"),
        "{stdout}"
    );
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
fn skills_manager_list_ignores_project_local_shared_target() {
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
        !stdout.contains("shared-skill"),
        "unexpected stdout: {stdout}"
    );
    assert!(
        stdout.contains("No skills found."),
        "unexpected stdout: {stdout}"
    );
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
        "global_dir = \"~/.agents/skills\"\nenabled = true",
        "project_dir = \".agents\"\nenabled = true",
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
    let columns = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(columns[0], "global-shared-skill");
    assert!(
        columns[1].ends_with(".agents/skills/global-shared-skill"),
        "{row}"
    );
    assert_eq!(&columns[2..], &["-", "✓", "✓", "global", "physical"]);
    assert!(!stdout.contains("AGENTS"), "unexpected stdout: {stdout}");
}

#[test]
fn skills_manager_list_is_invariant_across_launch_directories() {
    let home = TempDir::new().unwrap();
    let first_cwd = TempDir::new().unwrap();
    let second_cwd = TempDir::new().unwrap();
    let shared_skill = home
        .path()
        .join(".agents")
        .join("skills")
        .join("global-shared-skill");
    fs::create_dir_all(&shared_skill).expect("create global shared skill dir");
    fs::write(shared_skill.join("SKILL.md"), "# Global shared skill").expect("write skill file");
    fs::create_dir_all(first_cwd.path().join(".agents/skills/local-only")).unwrap();

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = home
        .path()
        .join("missing-sources")
        .to_string_lossy()
        .into_owned();
    for agent in config.agents.values_mut() {
        agent.global_dir = home
            .path()
            .join(format!("missing-{}", agent.display_name.to_lowercase()))
            .to_string_lossy()
            .into_owned();
    }
    write_config(&home, &config);

    let first = config_bin_with_home(&home)
        .current_dir(first_cwd.path())
        .args(["list"])
        .output()
        .expect("first list runs");
    let second = config_bin_with_home(&home)
        .current_dir(second_cwd.path())
        .args(["list"])
        .output()
        .expect("second list runs");

    assert!(first.status.success());
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert!(
        !String::from_utf8(first.stdout)
            .unwrap()
            .contains("local-only")
    );
}

#[test]
fn skills_manager_scan_rejects_relative_source_before_discovery() {
    let home = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();
    let relative_skill = cwd.path().join("relative-source").join("local-only");
    fs::create_dir_all(&relative_skill).unwrap();
    fs::write(relative_skill.join("SKILL.md"), "# Local only").unwrap();

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = "relative-source".to_string();
    write_config(&home, &config);

    let output = config_bin_with_home(&home)
        .current_dir(cwd.path())
        .args(["scan"])
        .output()
        .expect("scan runs");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("skills.central_dir"), "{stderr}");
    assert!(stderr.contains("relative-source"), "{stderr}");
}

#[test]
fn skills_manager_import_rejects_relative_global_target_before_planning() {
    let home = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();
    let skills_root = home.path().join("skill-sources");
    let skill_dir = skills_root.join("code-review");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "# Code review").unwrap();

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = skills_root.to_string_lossy().into_owned();
    config.agents.get_mut("claude").unwrap().global_dir = "relative-agent".to_string();
    write_config(&home, &config);

    let output = config_bin_with_home(&home)
        .current_dir(cwd.path())
        .args(["import", "code-review", "--to", "claude"])
        .output()
        .expect("import runs");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("agents.claude.global_dir"), "{stderr}");
    assert!(stderr.contains("relative-agent"), "{stderr}");
}

#[test]
fn skills_manager_remove_ignores_legacy_project_target() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let project_skill = project.path().join(".claude/skills/local-only");
    fs::create_dir_all(&project_skill).unwrap();
    fs::write(project_skill.join("SKILL.md"), "# Local only").unwrap();

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = home
        .path()
        .join("missing-sources")
        .to_string_lossy()
        .into_owned();
    for agent in config.agents.values_mut() {
        agent.global_dir = home
            .path()
            .join(format!("missing-{}", agent.display_name.to_lowercase()))
            .to_string_lossy()
            .into_owned();
        agent.shared_target_ids.clear();
    }
    config.shared_targets.clear();
    let current_toml = config.to_toml().unwrap();
    let legacy_toml = current_toml.replace(
        "global_dir = \"",
        "project_dir = \".claude/skills\"\nglobal_dir = \"",
    );
    let config_path = config_path_for_home(&home);
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(config_path, legacy_toml).unwrap();

    let output = config_bin_with_home(&home)
        .current_dir(project.path())
        .args(["remove", "local-only", "--from", "claude"])
        .output()
        .expect("remove runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("not found in inventory"), "{stdout}");
    assert!(project_skill.exists());
}

#[test]
fn skills_manager_lists_project_local_exposure_but_refuses_to_remove_it() {
    let home = TempDir::new().unwrap();
    let sources = TempDir::new().unwrap();
    let project = sources.path().join("analystloop");
    let project_skill = project.join(".agents/skills/adx-intake");
    fs::create_dir_all(project.join(".git")).unwrap();
    fs::create_dir_all(&project_skill).unwrap();
    fs::write(project_skill.join("SKILL.md"), "# ADX intake").unwrap();

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = home
        .path()
        .join("missing-central")
        .to_string_lossy()
        .into_owned();
    config.skills.scan_parent_dirs = vec![sources.path().to_string_lossy().into_owned()];
    for (agent_id, agent) in &mut config.agents {
        agent.global_dir = home
            .path()
            .join(format!("missing-{agent_id}"))
            .to_string_lossy()
            .into_owned();
        agent.project_dir = None;
        agent.shared_target_ids.clear();
    }
    config.shared_targets.clear();
    write_config(&home, &config);

    let list = config_bin_with_home(&home)
        .args(["list"])
        .output()
        .expect("list runs");
    assert!(list.status.success());
    let stdout = String::from_utf8(list.stdout).unwrap();
    let row = stdout
        .lines()
        .find(|line| line.contains("analystloop/adx-intake"))
        .expect("project-local row");
    assert!(row.contains(".agents/skills/adx-intake"), "{row}");
    assert_eq!(
        row.split_whitespace()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>(),
        vec!["-", "✓", "✓", "project-local", "physical"]
    );

    let remove = config_bin_with_home(&home)
        .args(["remove", "analystloop/adx-intake"])
        .output()
        .expect("remove runs");
    assert!(remove.status.success());
    let stdout = String::from_utf8(remove.stdout).unwrap();
    assert!(stdout.contains("read-only"), "{stdout}");
    assert!(project_skill.exists());
}

#[test]
fn skills_manager_import_plan_is_invariant_across_launch_directories() {
    let home = TempDir::new().unwrap();
    let first_cwd = TempDir::new().unwrap();
    let second_cwd = TempDir::new().unwrap();
    let skills_root = home.path().join("skill-sources");
    let skill_dir = skills_root.join("code-review");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "# Code review").unwrap();

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = skills_root.to_string_lossy().into_owned();
    for (agent_id, agent) in &mut config.agents {
        agent.global_dir = home
            .path()
            .join(format!("{agent_id}-skills"))
            .to_string_lossy()
            .into_owned();
        agent.shared_target_ids.clear();
    }
    config.shared_targets.clear();
    write_config(&home, &config);

    let first = config_bin_with_home(&home)
        .current_dir(first_cwd.path())
        .args(["import", "code-review", "--to", "claude"])
        .output()
        .expect("first import runs");
    let second = config_bin_with_home(&home)
        .current_dir(second_cwd.path())
        .args(["import", "code-review", "--to", "claude"])
        .output()
        .expect("second import runs");

    assert!(!first.status.success());
    assert!(!second.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    assert!(String::from_utf8(first.stdout).unwrap().contains("Expose"));
}

#[test]
fn skills_manager_remove_plan_is_invariant_across_launch_directories() {
    let home = TempDir::new().unwrap();
    let first_cwd = TempDir::new().unwrap();
    let second_cwd = TempDir::new().unwrap();
    let claude_target = home.path().join("claude-skills");
    let exposed_skill = claude_target.join("global-only");
    fs::create_dir_all(&exposed_skill).unwrap();
    fs::write(exposed_skill.join("SKILL.md"), "# Global only").unwrap();

    let mut config = skills_manager::config::Config::default_config();
    config.skills.central_dir = home
        .path()
        .join("missing-sources")
        .to_string_lossy()
        .into_owned();
    for (agent_id, agent) in &mut config.agents {
        agent.global_dir = if agent_id == "claude" {
            claude_target.to_string_lossy().into_owned()
        } else {
            home.path()
                .join(format!("missing-{agent_id}"))
                .to_string_lossy()
                .into_owned()
        };
        agent.shared_target_ids.clear();
    }
    config.shared_targets.clear();
    write_config(&home, &config);

    let first = config_bin_with_home(&home)
        .current_dir(first_cwd.path())
        .args(["remove", "global-only", "--from", "claude"])
        .output()
        .expect("first remove runs");
    let second = config_bin_with_home(&home)
        .current_dir(second_cwd.path())
        .args(["remove", "global-only", "--from", "claude"])
        .output()
        .expect("second remove runs");

    assert!(!first.status.success());
    assert!(!second.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    assert!(
        String::from_utf8(first.stdout)
            .unwrap()
            .contains("DELETE physical copy")
    );
    assert!(exposed_skill.exists());
}
