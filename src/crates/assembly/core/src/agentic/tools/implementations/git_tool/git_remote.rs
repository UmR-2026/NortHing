use crate::agentic::tools::framework::ToolUseContext;
use crate::service::git::{GitPullParams, GitPushParams, GitService};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

use super::git_types::sh_quote;

pub(crate) async fn execute_remote_git_cli(
    repo_path: &str,
    operation: &str,
    args: Option<&str>,
    context: &ToolUseContext,
) -> NortHingResult<Value> {
    let shell = context
        .ws_shell()
        .ok_or_else(|| NortHingError::tool("Remote Git requires workspace shell (SSH)".to_string()))?;

    let args_str = args.unwrap_or("").trim();
    let cmd = if args_str.is_empty() {
        format!("git --no-pager -C {} {}", sh_quote(repo_path), operation)
    } else {
        format!("git --no-pager -C {} {} {}", sh_quote(repo_path), operation, args_str)
    };

    let (stdout, stderr, exit_code) = shell
        .exec(&cmd, Some(180_000))
        .await
        .map_err(|e| NortHingError::tool(format!("Remote git failed: {}", e)))?;

    Ok(json!({
        "success": exit_code == 0,
        "exit_code": exit_code,
        "stdout": stdout,
        "stderr": stderr,
        "command": cmd,
        "remote_execution": true,
    }))
}

pub(crate) async fn execute_push(repo_path: &str, args: Option<&str>) -> NortHingResult<Value> {
    let args_str = args.unwrap_or("");
    let parts: Vec<&str> = args_str.split_whitespace().filter(|s| !s.starts_with('-')).collect();

    let params = GitPushParams {
        remote: parts.first().map(|s| s.to_string()),
        branch: parts.get(1).map(|s| s.to_string()),
        force: Some(args_str.contains("--force") || args_str.contains("-f")),
        set_upstream: Some(args_str.contains("-u") || args_str.contains("--set-upstream")),
    };

    let result = GitService::push(repo_path, params)
        .await
        .map_err(|e| NortHingError::tool(format!("Git push failed: {}", e)))?;

    Ok(json!({
        "success": result.success,
        "exit_code": if result.success { 0 } else { 1 },
        "stdout": result.output.unwrap_or_default(),
        "stderr": result.error.unwrap_or_default(),
        "execution_time_ms": result.duration
    }))
}

pub(crate) async fn execute_pull(repo_path: &str, args: Option<&str>) -> NortHingResult<Value> {
    let args_str = args.unwrap_or("");
    let parts: Vec<&str> = args_str.split_whitespace().filter(|s| !s.starts_with('-')).collect();

    let params = GitPullParams {
        remote: parts.first().map(|s| s.to_string()),
        branch: parts.get(1).map(|s| s.to_string()),
        rebase: Some(args_str.contains("--rebase")),
    };

    let result = GitService::pull(repo_path, params)
        .await
        .map_err(|e| NortHingError::tool(format!("Git pull failed: {}", e)))?;

    Ok(json!({
        "success": result.success,
        "exit_code": if result.success { 0 } else { 1 },
        "stdout": result.output.unwrap_or_default(),
        "stderr": result.error.unwrap_or_default(),
        "execution_time_ms": result.duration
    }))
}
