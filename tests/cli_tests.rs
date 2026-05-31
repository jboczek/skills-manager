use assert_cmd::Command;

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
