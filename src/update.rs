use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::process::Command;

const FORMULA: &str = "skills-manager";

pub fn check() -> Result<Option<String>> {
    let output = Command::new("brew")
        .args(["outdated", "--json=v2", FORMULA])
        .output()
        .context("could not check Homebrew for Skills Manager updates")?;

    if !output.status.success() {
        bail!("Homebrew update check failed");
    }

    Ok(available_version(&String::from_utf8_lossy(&output.stdout)))
}

pub fn install() -> Result<()> {
    run_brew(["update"])?;
    run_brew(["upgrade", FORMULA])
}

pub fn restart() -> Result<()> {
    let output = Command::new("brew")
        .args(["--prefix", FORMULA])
        .output()
        .context("could not locate Homebrew Skills Manager installation")?;
    if !output.status.success() {
        bail!("could not locate Homebrew Skills Manager installation");
    }

    Command::new(launcher_path(&String::from_utf8_lossy(&output.stdout)))
        .args(std::env::args_os().skip(1))
        .spawn()
        .context("could not restart Skills Manager")?;
    Ok(())
}

fn run_brew<const N: usize>(args: [&str; N]) -> Result<()> {
    let status = Command::new("brew")
        .args(args)
        .status()
        .context("could not run Homebrew")?;
    if status.success() {
        Ok(())
    } else {
        bail!("Homebrew command failed")
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
    use super::{available_version, launcher_path};
    use std::path::PathBuf;

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
    fn restarts_from_homebrew_stable_formula_launcher() {
        assert_eq!(
            launcher_path("/opt/homebrew/opt/skills-manager\n"),
            PathBuf::from("/opt/homebrew/opt/skills-manager/bin/skills-manager")
        );
    }
}
