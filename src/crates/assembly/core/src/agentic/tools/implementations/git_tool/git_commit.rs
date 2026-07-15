use crate::service::git::{GitCommitParams, GitService};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

pub(crate) async fn execute_commit(repo_path: &str, args: Option<&str>) -> NortHingResult<Value> {
    let args_str = args.unwrap_or("");

    let message = if let Some(pos) = args_str.find("-m") {
        let rest = &args_str[pos + 2..].trim_start();
        if rest.starts_with('"') {
            rest.trim_start_matches('"').split('"').next().unwrap_or("").to_string()
        } else if rest.starts_with('\'') {
            rest.trim_start_matches('\'')
                .split('\'')
                .next()
                .unwrap_or("")
                .to_string()
        } else {
            rest.split_whitespace().next().unwrap_or("").to_string()
        }
    } else {
        return Err(NortHingError::tool(
            "Commit message is required (-m \"message\")".to_string(),
        ));
    };

    let params = GitCommitParams {
        message,
        amend: Some(args_str.contains("--amend")),
        all: Some(args_str.contains("-a")),
        no_verify: Some(args_str.contains("--no-verify")),
        author: None,
    };

    let result = GitService::commit(repo_path, params)
        .await
        .map_err(|e| NortHingError::tool(format!("Git commit failed: {}", e)))?;

    Ok(json!({
        "success": result.success,
        "exit_code": if result.success { 0 } else { 1 },
        "stdout": result.output.unwrap_or_default(),
        "stderr": result.error.unwrap_or_default(),
        "execution_time_ms": result.duration
    }))
}
