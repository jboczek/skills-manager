use std::path::Path;
use std::process::Command;

pub fn origin_url(repo: &Path) -> anyhow::Result<Option<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["remote", "get-url", "origin"])
        .output()?;

    if output.status.success() {
        let url = String::from_utf8(output.stdout)?.trim().to_owned();
        Ok(Some(url))
    } else {
        Ok(None)
    }
}
