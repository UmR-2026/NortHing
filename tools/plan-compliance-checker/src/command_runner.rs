use anyhow::Result;
use std::path::Path;

pub fn run_command(cwd: &Path, cmd: &str, args: &[&str]) -> Result<(i32, String)> {
    let output = std::process::Command::new(cmd).current_dir(cwd).args(args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok((output.status.code().unwrap_or(-1), stdout))
}
