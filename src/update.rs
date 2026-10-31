use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::process::Command;

const FORMULA: &str = "skills-manager";

pub fn check() -> Result<Option<String>> {
    check_with(run_brew_output)
}

fn check_with<F>(mut run: F) -> Result<Option<String>>
where
    F: FnMut(&[&str]) -> Result<String>,
{
    run(&["update"])?;
    Ok(available_version(&run(&[
        "outdated",
        "--json=v2",
        "--formula",
        FORMULA,
    ])?))
}

fn run_brew_output(args: &[&str]) -> Result<String> {
    let output = Command::new("brew")
        .args(args)
        .output()
        .context("could not check Homebrew for Skills Manager updates")?;

    if !output.status.success() {
        bail!("Homebrew update check failed");
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn install() -> Result<()> {
    install_with(run_brew)
}

fn install_with<F>(mut run: F) -> Result<()>
where
    F: FnMut(&[&str]) -> Result<()>,
{
    run(&["update"])?;
    run(&["upgrade", "--formula", "--no-ask", FORMULA])
}

pub fn restart() -> Result<()> {
    let output = Command::new("brew")
        .args(["--prefix", FORMULA])
        .output()
        .context("could not locate Homebrew Skills Manager installation")?;
    if !output.status.success() {
        bail!("could not locate Homebrew Skills Manager installation");
    }

    let mut command = Command::new(launcher_path(&String::from_utf8_lossy(&output.stdout)));
    command.args(std::env::args_os().skip(1));

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        // Replace this process so shell wrappers keep the terminal foreground.
        let error = command.exec();
        Err(error).context("could not restart Skills Manager")
    }

    #[cfg(not(unix))]
    {
        command
            .spawn()
            .context("could not restart Skills Manager")?;
        Ok(())
    }
}

fn run_brew(args: &[&str]) -> Result<()> {
    let output = Command::new("brew")
        .args(args)
        .output()
        .context("could not run Homebrew")?;
    ensure_brew_success(output.status.success(), &output.stderr)
}

fn ensure_brew_success(success: bool, stderr: &[u8]) -> Result<()> {
    if success {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        bail!("Homebrew command failed")
    } else {
        bail!("Homebrew command failed: {stderr}")
    }
}

fn available_version(output: &str) -> Option<String> {
    let formulae = output.split_once("\"formulae\"")?.1.split_once('[')?.1;
    let formula = formulae.split_once(']')?.0;
    let (_, version) = formula.split_once("\"current_version\":\"")?;
    Some(version.split_once('"')?.0.to_string())
}

fn launcher_path(prefix: &str) -> PathBuf {
    PathBuf::from(prefix.trim()).join("bin").join(FORMULA)
}

#[cfg(test)]
mod tests {
    use super::{
        available_version, check_with, ensure_brew_success, install_with, launcher_path, restart,
    };
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::process::Command;

    #[test]
    fn finds_available_version_in_homebrew_outdated_json() {
        assert_eq!(
            available_version(
                r#"{"formulae":[{"name":"skills-manager","current_version":"0.2.0"}],"casks":[]}"#
            ),
            Some("0.2.0".to_string())
        );
    }

    #[test]
    fn ignores_empty_homebrew_outdated_result() {
        assert_eq!(available_version(r#"{"formulae":[],"casks":[]}"#), None);
    }

    #[test]
    fn refreshes_homebrew_before_checking_for_updates() {
        let mut commands = Vec::new();

        let update = check_with(|args| {
            commands.push(args.iter().map(ToString::to_string).collect::<Vec<_>>());
            Ok(
                r#"{"formulae":[{"name":"skills-manager","current_version":"0.2.0"}],"casks":[]}"#
                    .to_string(),
            )
        })
        .unwrap();

        assert_eq!(update, Some("0.2.0".to_string()));
        assert_eq!(
            commands,
            vec![
                vec!["update".to_string()],
                vec![
                    "outdated".to_string(),
                    "--json=v2".to_string(),
                    "--formula".to_string(),
                    "skills-manager".to_string(),
                ],
            ]
        );
    }

    #[test]
    fn stops_check_when_homebrew_refresh_fails() {
        let mut commands = Vec::new();

        assert!(
            check_with(|args| {
                commands.push(args.iter().map(ToString::to_string).collect::<Vec<_>>());
                anyhow::bail!("Homebrew command failed")
            })
            .is_err()
        );
        assert_eq!(commands, vec![vec!["update".to_string()]]);
    }

    #[test]
    fn reports_captured_homebrew_errors() {
        let error = ensure_brew_success(false, b"Error: the formula is not trusted\n").unwrap_err();

        assert_eq!(
            error.to_string(),
            "Homebrew command failed: Error: the formula is not trusted"
        );
    }

    #[test]
    fn upgrades_the_formula_after_refreshing_homebrew() {
        let mut commands = Vec::new();

        install_with(|args| {
            commands.push(args.iter().map(ToString::to_string).collect::<Vec<_>>());
            Ok(())
        })
        .unwrap();

        assert_eq!(
            commands,
            vec![
                vec!["update".to_string()],
                vec![
                    "upgrade".to_string(),
                    "--formula".to_string(),
                    "--no-ask".to_string(),
                    "skills-manager".to_string(),
                ],
            ]
        );
    }

    #[test]
    fn restarts_from_homebrew_stable_formula_launcher() {
        assert_eq!(
            launcher_path("/opt/homebrew/opt/skills-manager\n"),
            PathBuf::from("/opt/homebrew/opt/skills-manager/bin/skills-manager")
        );
    }

    #[cfg(unix)]
    #[test]
    fn restart_replaces_the_current_process() {
        let temp = tempfile::tempdir().unwrap();
        let bin = temp.path().join("bin");
        let prefix_bin = temp.path().join("prefix/bin");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&prefix_bin).unwrap();

        let fake_brew = bin.join("brew");
        fs::write(
            &fake_brew,
            "#!/bin/sh\nprintf '%s\\n' \"$SKILLS_MANAGER_TEST_PREFIX\"\n",
        )
        .unwrap();
        fs::set_permissions(&fake_brew, fs::Permissions::from_mode(0o755)).unwrap();

        let launcher = prefix_bin.join("skills-manager");
        fs::write(&launcher, "#!/bin/sh\nexit 37\n").unwrap();
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();

        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "update::tests::restart_exec_child",
                "--nocapture",
            ])
            .env("SKILLS_MANAGER_TEST_RESTART_CHILD", "1")
            .env("SKILLS_MANAGER_TEST_PREFIX", temp.path().join("prefix"))
            .env("PATH", &bin)
            .status()
            .unwrap();

        assert_eq!(status.code(), Some(37));
    }

    #[cfg(unix)]
    #[test]
    fn restart_exec_child() {
        if std::env::var_os("SKILLS_MANAGER_TEST_RESTART_CHILD").is_some() {
            restart().unwrap();
        }
    }
}
