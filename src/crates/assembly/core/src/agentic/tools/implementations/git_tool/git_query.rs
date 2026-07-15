use crate::agentic::tools::framework::ToolUseContext;
use crate::service::git::{
    execute_git_command, execute_git_command_raw, GitAddParams, GitDiffParams, GitLogParams, GitService,
};
use crate::util::elapsed_ms_u64;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

use super::git_types::parse_diff_args;

pub(crate) fn get_repo_path(working_directory: Option<&str>, context: &ToolUseContext) -> NortHingResult<String> {
    if let Some(dir) = working_directory {
        let trimmed = dir.trim();
        if trimmed.is_empty() {
            return context
                .workspace
                .as_ref()
                .map(|w| w.root_path_string())
                .ok_or_else(|| NortHingError::tool("No workspace path available".to_string()));
        }
        context.resolve_workspace_tool_path(trimmed)
    } else {
        context
            .workspace
            .as_ref()
            .map(|w| w.root_path_string())
            .ok_or_else(|| NortHingError::tool("No workspace path available".to_string()))
    }
}

pub(crate) async fn execute_status(repo_path: &str) -> NortHingResult<Value> {
    let status = GitService::get_status(repo_path)
        .await
        .map_err(|e| NortHingError::tool(format!("Git status failed: {}", e)))?;

    let mut output_lines = vec![];
    output_lines.push(format!("On branch {}", status.current_branch));

    if status.ahead > 0 || status.behind > 0 {
        output_lines.push(format!(
            "Your branch is {} ahead, {} behind",
            status.ahead, status.behind
        ));
    }

    if !status.staged.is_empty() {
        output_lines.push("\nChanges to be committed:".to_string());
        for file in &status.staged {
            output_lines.push(format!("  {}: {}", file.status, file.path));
        }
    }

    if !status.unstaged.is_empty() {
        output_lines.push("\nChanges not staged for commit:".to_string());
        for file in &status.unstaged {
            output_lines.push(format!("  {}: {}", file.status, file.path));
        }
    }

    if !status.untracked.is_empty() {
        output_lines.push("\nUntracked files:".to_string());
        for file in &status.untracked {
            output_lines.push(format!("  {}", file));
        }
    }

    if status.staged.is_empty() && status.unstaged.is_empty() && status.untracked.is_empty() {
        output_lines.push("nothing to commit, working tree clean".to_string());
    }

    Ok(json!({
        "success": true,
        "exit_code": 0,
        "stdout": output_lines.join("\n"),
        "stderr": "",
        "data": status
    }))
}

pub(crate) async fn execute_diff(repo_path: &str, args: Option<&str>) -> NortHingResult<Value> {
    let parsed = parse_diff_args(args.unwrap_or(""));

    let params = GitDiffParams {
        staged: Some(parsed.staged),
        stat: Some(parsed.stat),
        source: parsed.source,
        target: parsed.target,
        files: parsed.files,
    };

    let diff_output = GitService::get_diff(repo_path, &params)
        .await
        .map_err(|e| NortHingError::tool(format!("Git diff failed: {}", e)))?;

    let stdout = if diff_output.trim().is_empty() {
        "No differences found.".to_string()
    } else {
        diff_output
    };

    Ok(json!({
        "success": true,
        "exit_code": 0,
        "stdout": stdout,
        "stderr": ""
    }))
}

pub(crate) async fn execute_log(repo_path: &str, args: Option<&str>) -> NortHingResult<Value> {
    let args_str = args.unwrap_or("");

    let mut max_count = 50;
    let oneline = args_str.contains("--oneline");
    let stat = args_str.contains("--stat");
    let mut since: Option<String> = None;
    let mut until: Option<String> = None;

    for prefix in &["--since=", "--until="] {
        if let Some(pos) = args_str.find(prefix) {
            let val = args_str[pos + prefix.len()..]
                .split_whitespace()
                .next()
                .map(|s| s.trim_matches('"').trim_matches('\'').to_string());
            if *prefix == "--since=" {
                since = val;
            } else {
                until = val;
            }
        }
    }

    if let Some(pos) = args_str.find("-n") {
        if let Some(num_str) = args_str.get(pos + 2..).and_then(|s| s.split_whitespace().next()) {
            if let Ok(n) = num_str.trim().parse::<i32>() {
                max_count = n;
            }
        }
    } else if let Some(pos) = args_str.find('-') {
        if let Some(num_str) = args_str.get(pos + 1..).and_then(|s| s.split_whitespace().next()) {
            if let Ok(n) = num_str.parse::<i32>() {
                max_count = n;
            }
        }
    }

    let params = GitLogParams {
        max_count: Some(max_count),
        stat: Some(stat),
        since,
        until,
        ..Default::default()
    };

    let commits = GitService::get_commits(repo_path, params)
        .await
        .map_err(|e| NortHingError::tool(format!("Git log failed: {}", e)))?;

    let output_lines: Vec<String> = commits
        .iter()
        .map(|c| {
            if oneline {
                format!("{} {}", c.short_hash, c.message.lines().next().unwrap_or(""))
            } else {
                format!(
                    "commit {}\nAuthor: {} <{}>\nDate:   {}\n\n    {}\n",
                    c.hash, c.author, c.author_email, c.date, c.message
                )
            }
        })
        .collect();

    Ok(json!({
        "success": true,
        "exit_code": 0,
        "stdout": output_lines.join(if oneline { "\n" } else { "" }),
        "stderr": "",
        "data": commits
    }))
}

pub(crate) async fn execute_add(repo_path: &str, args: Option<&str>) -> NortHingResult<Value> {
    let args_str = args.unwrap_or(".");
    let all = args_str.contains("-A") || args_str.contains("--all");
    let update = args_str.contains("-u") || args_str.contains("--update");

    let files: Vec<String> = if all || update {
        vec![]
    } else {
        args_str
            .split_whitespace()
            .filter(|s| !s.starts_with('-'))
            .map(|s| s.to_string())
            .collect()
    };

    let params = GitAddParams {
        files,
        all: Some(all),
        update: Some(update),
    };

    let result = GitService::add_files(repo_path, params)
        .await
        .map_err(|e| NortHingError::tool(format!("Git add failed: {}", e)))?;

    Ok(json!({
        "success": result.success,
        "exit_code": if result.success { 0 } else { 1 },
        "stdout": result.output.unwrap_or_default(),
        "stderr": result.error.unwrap_or_default(),
        "execution_time_ms": result.duration
    }))
}

pub(crate) async fn execute_generic(repo_path: &str, operation: &str, args: Option<&str>) -> NortHingResult<Value> {
    let mut cmd_args: Vec<&str> = vec![operation];

    if let Some(args_str) = args {
        for arg in args_str.split_whitespace() {
            cmd_args.push(arg);
        }
    }

    let start_time = std::time::Instant::now();

    match execute_git_command_raw(repo_path, &cmd_args).await {
        Ok(raw) => {
            let duration = elapsed_ms_u64(start_time);

            let is_diff_like = operation == "diff";
            let success = if raw.exit_code == 0 {
                true
            } else {
                is_diff_like && raw.exit_code == 1 && !raw.stdout.is_empty()
            };

            Ok(json!({
                "success": success,
                "exit_code": raw.exit_code,
                "stdout": raw.stdout,
                "stderr": raw.stderr,
                "execution_time_ms": duration
            }))
        }
        Err(e) => {
            let duration = elapsed_ms_u64(start_time);
            Ok(json!({
                "success": false,
                "exit_code": -1,
                "stdout": "",
                "stderr": e.to_string(),
                "execution_time_ms": duration
            }))
        }
    }
}
