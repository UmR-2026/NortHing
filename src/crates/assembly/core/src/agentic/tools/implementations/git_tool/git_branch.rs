use crate::service::git::{execute_git_command, GitService};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

pub(crate) async fn execute_checkout(repo_path: &str, args: Option<&str>) -> NortHingResult<Value> {
    let args_str = args.unwrap_or("");
    let create_branch = args_str.contains("-b");

    let branch_name = args_str
        .split_whitespace()
        .rfind(|s| !s.starts_with('-'))
        .ok_or_else(|| NortHingError::tool("Branch name is required".to_string()))?;

    let result = if create_branch {
        let start_point = args_str
            .split_whitespace()
            .rfind(|s| !s.starts_with('-') && *s != branch_name);
        GitService::create_branch(repo_path, branch_name, start_point).await
    } else {
        GitService::checkout_branch(repo_path, branch_name).await
    }
    .map_err(|e| NortHingError::tool(format!("Git checkout failed: {}", e)))?;

    Ok(json!({
        "success": result.success,
        "exit_code": if result.success { 0 } else { 1 },
        "stdout": result.output.unwrap_or_default(),
        "stderr": result.error.unwrap_or_default(),
        "execution_time_ms": result.duration
    }))
}

pub(crate) async fn execute_branch(repo_path: &str, args: Option<&str>) -> NortHingResult<Value> {
    let args_str = args.unwrap_or("");

    let is_list = args_str.is_empty()
        || args_str.contains("-l")
        || args_str.contains("--list")
        || args_str.contains("-a")
        || args_str.contains("-r");

    if is_list {
        let include_remote = args_str.contains("-a") || args_str.contains("-r");
        let branches = GitService::get_branches(repo_path, include_remote)
            .await
            .map_err(|e| NortHingError::tool(format!("Git branch failed: {}", e)))?;

        let output: Vec<String> = branches
            .iter()
            .map(|b| {
                if b.current {
                    format!("* {}", b.name)
                } else {
                    format!("  {}", b.name)
                }
            })
            .collect();

        Ok(json!({
            "success": true,
            "exit_code": 0,
            "stdout": output.join("\n"),
            "stderr": "",
            "data": branches
        }))
    } else if args_str.contains("-d") || args_str.contains("-D") {
        let force = args_str.contains("-D");
        let branch_name = args_str
            .split_whitespace()
            .find(|s| !s.starts_with('-'))
            .ok_or_else(|| NortHingError::tool("Branch name is required for deletion".to_string()))?;

        let result = GitService::delete_branch(repo_path, branch_name, force)
            .await
            .map_err(|e| NortHingError::tool(format!("Git branch delete failed: {}", e)))?;

        Ok(json!({
            "success": result.success,
            "exit_code": if result.success { 0 } else { 1 },
            "stdout": result.output.unwrap_or_default(),
            "stderr": result.error.unwrap_or_default()
        }))
    } else {
        let mut cmd_args: Vec<&str> = vec!["branch"];
        for arg in args_str.split_whitespace() {
            cmd_args.push(arg);
        }

        let output = execute_git_command(repo_path, &cmd_args)
            .await
            .map_err(|e| NortHingError::tool(format!("Git branch failed: {}", e)))?;

        Ok(json!({
            "success": true,
            "exit_code": 0,
            "stdout": output,
            "stderr": ""
        }))
    }
}
