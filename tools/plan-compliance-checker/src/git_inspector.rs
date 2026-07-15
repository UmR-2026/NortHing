use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Commit {
    pub sha: String,
    pub message: String,
    pub files: Vec<String>,
}

pub fn commits_since(repo_root: &Path, since: &str) -> Result<Vec<Commit>> {
    let output = std::process::Command::new("git")
        .current_dir(repo_root)
        .args(["log", "--reverse", "--format=%H%n%s", "--name-only", since])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("git log failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    let mut current_sha: Option<String> = None;
    let mut current_msg: Option<String> = None;
    let mut current_files: Vec<String> = Vec::new();

    for line in stdout.lines() {
        if line.len() == 40 && line.chars().all(|c| c.is_ascii_hexdigit()) && current_sha.is_none() {
            current_sha = Some(line.to_string());
        } else if line.is_empty() && current_sha.is_some() {
            commits.push(Commit {
                sha: current_sha.take().unwrap(),
                message: current_msg.take().unwrap_or_default(),
                files: std::mem::take(&mut current_files),
            });
            current_msg = None;
        } else if current_sha.is_some() && current_msg.is_none() {
            current_msg = Some(line.to_string());
        } else if current_sha.is_some() {
            current_files.push(line.to_string());
        }
    }
    if let Some(sha) = current_sha {
        commits.push(Commit {
            sha,
            message: current_msg.unwrap_or_default(),
            files: current_files,
        });
    }
    Ok(commits)
}
