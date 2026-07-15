//! Group 1: validation_result, input_validator, tool_result, tool_error,
//! steering_interrupted, invalid_tool_call, truncation_recovery tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn validation_result_default_preserves_success_contract() {
    assert!(ValidationResult::default().result);
    assert_eq!(ValidationResult::default().message, None);
}

#[test]
fn input_validator_preserves_required_field_error() {
    let result = InputValidator::new(&json!({})).validate_required("path").finish();

    assert!(!result.result);
    assert_eq!(result.message.as_deref(), Some("path is required"));
    assert_eq!(result.error_code, Some(400));
}

#[test]
fn tool_result_ok_keeps_result_shape() {
    let result = ToolResult::ok(json!({"ok": true}), Some("done".to_string()));
    let value = serde_json::to_value(result).expect("serialize tool result");

    assert_eq!(value["type"], "result");
    assert_eq!(value["data"]["ok"], true);
    assert_eq!(value["result_for_assistant"], "done");
}

#[test]
fn tool_result_assistant_fallback_prefers_pretty_json_and_non_empty_fallback() {
    let rendered = render_tool_result_for_assistant("Read", &json!({"path": "src/main.rs"}));
    assert_eq!(rendered, "{\n  \"path\": \"src/main.rs\"\n}");

    let rendered = render_tool_result_for_assistant("Empty", &json!(null));
    assert_eq!(rendered, "null");
}

#[test]
fn tool_error_preview_truncates_at_utf8_boundary_with_current_marker() {
    assert_eq!(TOOL_ERROR_ARGUMENTS_PREVIEW_BYTES, 1024);

    let raw = "ab😀cd";
    let preview = truncate_raw_tool_arguments_preview_to(raw, 5);

    assert_eq!(preview, "ab…[truncated, total 8 bytes]");
}

#[test]
fn tool_error_presentation_preserves_argument_echo_shape() {
    let arguments = json!({
        "path": "src/main.rs",
        "content": "hello"
    });
    let preview = truncate_tool_arguments_preview(&arguments);
    let presentation = build_tool_execution_error_presentation(
        "Write",
        "invalid_arguments",
        "path is required",
        Some(preview.clone()),
    );

    assert_eq!(presentation.result_json["category"], "invalid_arguments");
    assert_eq!(presentation.result_json["tool_name"], "Write");
    assert_eq!(presentation.result_json["provided_arguments"], preview);
    assert_eq!(
        presentation.result_for_assistant,
        format!("Tool 'Write' failed (invalid_arguments): path is required\nProvided arguments: {preview}")
    );
}

#[test]
fn steering_interrupted_presentation_preserves_current_contract() {
    let presentation = build_user_steering_interrupted_presentation("Read");

    assert_eq!(presentation.result_json["status"], "skipped");
    assert_eq!(presentation.result_json["category"], "user_steering_interrupted");
    assert_eq!(presentation.result_json["tool_name"], "Read");
    assert_eq!(presentation.result_for_assistant, USER_STEERING_INTERRUPTED_MESSAGE);
}

#[test]
fn invalid_tool_call_error_message_preserves_current_contract() {
    let message = build_invalid_tool_call_error_message("", true, false, Some("{\"path\"".to_string()));
    assert_eq!(
        message,
        "Missing valid tool name and arguments are invalid JSON. Raw arguments: {\"path\""
    );

    let message = build_invalid_tool_call_error_message("", false, false, None);
    assert_eq!(message, "Missing valid tool name.");

    let message = build_invalid_tool_call_error_message("Write", false, true, None);
    assert_eq!(
        message,
        "Tool arguments were truncated by the model (likely hit max_tokens). Refusing to execute a partial 'Write' call. Increase max_tokens, split the work into smaller calls, or retry."
    );

    let message = build_invalid_tool_call_error_message("Write", true, false, None);
    assert_eq!(message, "Arguments are invalid JSON.");
}

#[test]
fn truncation_recovery_notice_preserves_write_like_guidance() {
    assert!(is_write_like_tool_name("Write"));
    assert!(is_write_like_tool_name("file_write"));
    assert!(is_write_like_tool_name("write_notebook"));
    assert!(!is_write_like_tool_name("Read"));

    let notice = build_tool_call_truncation_recovery_notice("Write");

    assert!(notice.contains("latest Read result"));
    assert!(notice.contains("ONE Edit call"));
    assert!(notice.contains("Do NOT rewrite the whole file with Write"));
    assert!(notice.ends_with("Original tool result follows.\n\n"));
}

#[test]
fn truncation_recovery_notice_preserves_non_write_guidance() {
    let notice = build_tool_call_truncation_recovery_notice("AskUserQuestion");

    assert!(notice.contains("repaired, potentially incomplete arguments"));
    assert!(notice.contains("issue a fresh complete AskUserQuestion call"));
    assert!(!notice.contains("ONE Edit call"));
}
