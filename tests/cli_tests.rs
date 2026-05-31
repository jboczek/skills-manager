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
fn running_without_args_routes_to_the_tui_placeholder() {
    let output = Command::new(assert_cmd::cargo::cargo_bin!("skills-manager"))
        .output()
        .expect("binary runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    assert!(
        stdout.contains("TUI placeholder"),
        "missing TUI route output"
    );
}

fn config_bin_with_home(home: &TempDir) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("skills-manager"));
    cmd.env("HOME", home.path());
    cmd
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

    Command::new(&bin)
        .args(["config", "init"])
        .env("HOME", home.path())
        .output()
        .expect("first init runs");

    let output = Command::new(&bin)
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

    Command::new(&bin)
        .args(["config", "init"])
        .env("HOME", home.path())
        .output()
        .expect("init runs");

    let output = Command::new(&bin)
        .args(["config", "show"])
        .env("HOME", home.path())
        .output()
        .expect("show runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("[skills]"), "expected TOML output with [skills], got: {stdout}");
    assert!(stdout.contains("[preferences]"), "expected [preferences] section");
    assert!(stdout.contains("claude"), "expected claude agent in output");
}
