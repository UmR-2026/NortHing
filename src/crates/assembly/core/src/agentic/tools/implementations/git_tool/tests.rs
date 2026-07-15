use crate::agentic::tools::framework::Tool;
use crate::agentic::tools::implementations::git_tool::git_types::{
    git_operation_needs_light_checkpoint, normalize_git_input, parse_diff_args, ParsedDiffArgs,
};
use crate::agentic::tools::implementations::git_tool::GitTool;
use serde_json::json;

#[tokio::test]
async fn git_schema_requires_explicit_operation_instead_of_args_only() {
    let tool = GitTool::new();
    let schema = tool.input_schema();
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["required"], json!(["operation"]));
    assert!(schema["properties"]["operation"]["description"]
        .as_str()
        .unwrap()
        .contains("Do not prefix with \"git\""));
    assert!(schema["properties"]["args"]["description"]
        .as_str()
        .unwrap()
        .contains("Do not include \"git\" or repeat the operation"));

    let validation = tool
        .validate_input(&json!({"args": "--since=\"2026-05-02\" --oneline"}), None)
        .await;
    assert!(validation.result);

    let validation = tool.validate_input(&json!({"args": "log --oneline -10"}), None).await;
    assert!(validation.result);

    let validation = tool.validate_input(&json!({"command": "git status"}), None).await;
    assert!(validation.result);

    let validation = tool.validate_input(&json!("git diff --staged"), None).await;
    assert!(validation.result);

    let validation = tool.validate_input(&json!({"args": "--stat"}), None).await;
    assert!(validation.result);
}

#[test]
fn normalize_git_input_repairs_common_malformed_payloads() {
    assert_eq!(normalize_git_input(json!("git status")), json!({"operation": "status"}));
    assert_eq!(
        normalize_git_input(json!({"command": "git diff --staged"})),
        json!({"operation": "diff", "args": "--staged"})
    );
    assert_eq!(
        normalize_git_input(json!({"args": "log --oneline -10"})),
        json!({"operation": "log", "args": "--oneline -10"})
    );
    assert_eq!(
        normalize_git_input(json!({"args": "--since=\"2026-05-02\" --oneline"})),
        json!({
            "operation": "log",
            "args": "--since=\"2026-05-02\" --oneline"
        })
    );
    assert_eq!(
        normalize_git_input(json!({"operation": "status"})),
        json!({"operation": "status"})
    );
}

#[test]
fn checkpoint_detection_flags_mutating_git_operations() {
    assert!(git_operation_needs_light_checkpoint("checkout", Some("main")));
    assert!(git_operation_needs_light_checkpoint("reset", Some("--hard HEAD")));
    assert!(git_operation_needs_light_checkpoint("branch", Some("-D old")));
    assert!(!git_operation_needs_light_checkpoint("status", None));
    assert!(!git_operation_needs_light_checkpoint("diff", Some("-- src/lib.rs")));
    assert!(!git_operation_needs_light_checkpoint("branch", None));
}

#[test]
fn parse_diff_args_empty() {
    let r = parse_diff_args("");
    assert_eq!(
        r,
        ParsedDiffArgs {
            staged: false,
            stat: false,
            source: None,
            target: None,
            files: None,
        }
    );
}

#[test]
fn parse_diff_args_staged_only() {
    let r = parse_diff_args("--staged");
    assert_eq!(
        r,
        ParsedDiffArgs {
            staged: true,
            stat: false,
            source: None,
            target: None,
            files: None,
        }
    );
}

#[test]
fn parse_diff_args_cached_and_stat() {
    let r = parse_diff_args("--cached --stat");
    assert_eq!(
        r,
        ParsedDiffArgs {
            staged: true,
            stat: true,
            source: None,
            target: None,
            files: None,
        }
    );
}

#[test]
fn parse_diff_args_single_ref() {
    let r = parse_diff_args("HEAD");
    assert_eq!(
        r,
        ParsedDiffArgs {
            staged: false,
            stat: false,
            source: Some("HEAD".to_string()),
            target: None,
            files: None,
        }
    );
}

#[test]
fn parse_diff_args_single_ref_with_stat() {
    let r = parse_diff_args("HEAD --stat");
    assert_eq!(
        r,
        ParsedDiffArgs {
            staged: false,
            stat: true,
            source: Some("HEAD".to_string()),
            target: None,
            files: None,
        }
    );
}

#[test]
fn parse_diff_args_range_two_dot() {
    let r = parse_diff_args("HEAD~7..HEAD --stat");
    assert_eq!(
        r,
        ParsedDiffArgs {
            staged: false,
            stat: true,
            source: Some("HEAD~7".to_string()),
            target: Some("HEAD".to_string()),
            files: None,
        }
    );
}

#[test]
fn parse_diff_args_range_three_dot() {
    let r = parse_diff_args("origin/main...HEAD");
    assert_eq!(
        r,
        ParsedDiffArgs {
            staged: false,
            stat: false,
            source: Some("origin/main".to_string()),
            target: Some("HEAD".to_string()),
            files: None,
        }
    );
}

#[test]
fn parse_diff_args_range_with_files() {
    let r = parse_diff_args("HEAD~7..HEAD --stat -- src/foo.rs src/bar.rs");
    assert_eq!(
        r,
        ParsedDiffArgs {
            staged: false,
            stat: true,
            source: Some("HEAD~7".to_string()),
            target: Some("HEAD".to_string()),
            files: Some(vec!["src/foo.rs".to_string(), "src/bar.rs".to_string()]),
        }
    );
}

#[test]
fn parse_diff_args_single_ref_with_files() {
    let r = parse_diff_args("HEAD -- src/foo.rs");
    assert_eq!(
        r,
        ParsedDiffArgs {
            staged: false,
            stat: false,
            source: Some("HEAD".to_string()),
            target: None,
            files: None,
        }
    );
}

#[test]
fn parse_diff_args_files_only() {
    let r = parse_diff_args("-- -- src/foo.rs");
    assert_eq!(
        r,
        ParsedDiffArgs {
            staged: false,
            stat: false,
            source: None,
            target: None,
            files: Some(vec!["src/foo.rs".to_string()]),
        }
    );
}

#[test]
fn parse_diff_args_multi_token_range() {
    let r = parse_diff_args("feature/foo..main");
    assert_eq!(
        r,
        ParsedDiffArgs {
            staged: false,
            stat: false,
            source: Some("feature/foo".to_string()),
            target: Some("main".to_string()),
            files: None,
        }
    );
}
