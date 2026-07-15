//! Provider-neutral tool-call accumulation: stream delta buffering, JSON
//! repair, and finalization contracts.
//!
//! This module is the public entry point for the
//! `northhing-agent-stream::tool_call_accumulator` path. The behavioral code
//! lives in three private siblings:
//!
//! * [`tool_call_types`] — data shapes (`PendingToolCall`,
//!   `FinalizedToolCall`, `ToolCallBoundary`, …) plus the small predicate
//!   helpers (`is_write_like_tool_name`, `is_truncation_safe_to_recover`).
//! * [`tool_call_repair`] — `repair_truncated_json`, the byte-walker that
//!   closes open brackets/strings for safe-recovery tools.
//! * [`tool_call_state`] — `impl` blocks: `PendingToolCall::finalize`,
//!   `PendingToolCalls::apply_delta`, Git command normalization, etc.
//!
//! External callers should `use northhing_agent_stream::tool_call_accumulator::*`
//! (the re-exports below). Internal callers reach into the siblings directly.

// Sibling files `tool_call_types.rs`, `tool_call_repair.rs`, and
// `tool_call_state.rs` are declared in `lib.rs` so the module path resolves
// correctly. This facade only re-exports the items the public API needs.

pub use crate::tool_call_types::{
    is_write_like_tool_name, EarlyDetectedToolCall, FinalizedToolCall, PendingToolCall, PendingToolCalls,
    ToolCallBoundary, ToolCallDeltaOutcome, ToolCallParamsChunk, ToolCallStreamKey,
};

#[cfg(test)]
mod tests {
    use super::{
        EarlyDetectedToolCall, PendingToolCall, PendingToolCalls, ToolCallBoundary, ToolCallParamsChunk,
        ToolCallStreamKey,
    };
    use crate::tool_call_repair::repair_truncated_json;
    use serde_json::json;

    #[test]
    fn finalizes_complete_json_only_at_boundary() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("tool_a".to_string()));
        pending.append_arguments("{\"a\":1}");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.tool_id, "call_1");
        assert_eq!(finalized.tool_name, "tool_a");
        assert_eq!(finalized.arguments, json!({"a": 1}));
        assert!(!finalized.is_error);
        assert!(!pending.has_pending());
    }

    #[test]
    fn invalid_json_becomes_error_with_empty_object() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("tool_a".to_string()));
        pending.append_arguments("{\"a\":");

        let finalized = pending.finalize(ToolCallBoundary::StreamEnd).expect("finalized tool");

        assert_eq!(finalized.arguments, json!({}));
        assert!(finalized.is_error);
    }

    #[test]
    fn repairs_git_raw_command_arguments() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Git".to_string()));
        pending.append_arguments("git status");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.raw_arguments, "git status");
        assert_eq!(finalized.arguments, json!({"operation": "status"}));
        assert!(!finalized.is_error);
    }

    #[test]
    fn repairs_git_json_string_command_arguments() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Git".to_string()));
        pending.append_arguments("\"git diff --staged\"");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!({"operation": "diff", "args": "--staged"}));
        assert!(!finalized.is_error);
    }

    #[test]
    fn git_args_only_object_is_left_for_tool_schema_diagnostic() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Git".to_string()));
        pending.append_arguments("{\"args\": \"--since=\\\"2026-05-02\\\" --oneline\"}");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!({"args": "--since=\"2026-05-02\" --oneline"}));
        assert!(!finalized.is_error);
    }

    #[test]
    fn git_duplicate_subcommand_in_args_is_left_for_tool_schema_diagnostic() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Git".to_string()));
        pending.append_arguments("{\"args\": \"log --oneline -10\"}");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!({"args": "log --oneline -10"}));
        assert!(!finalized.is_error);
    }

    #[test]
    fn does_not_infer_git_operation_from_ambiguous_args_only_object() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Git".to_string()));
        pending.append_arguments("{\"args\": \"--stat\"}");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!({"args": "--stat"}));
        assert!(!finalized.is_error);
    }

    #[test]
    fn raw_string_arguments_for_single_field_tools_stay_invalid_json() {
        let cases = [
            ("Bash", "pnpm test"),
            ("Skill", "openai-docs"),
            ("Read", "src/main.rs"),
            ("GetFileDiff", "src/lib.rs"),
            ("LS", "src/crates"),
            ("Delete", "tmp/output.log"),
            ("Glob", "**/*.rs"),
            ("Grep", "Arguments are invalid JSON"),
            ("WebSearch", "OpenAI Agents SDK"),
            ("WebFetch", "https://example.com"),
            ("InitMiniApp", "Markdown Viewer"),
        ];

        for (tool_name, raw_arguments) in cases {
            let mut pending = PendingToolCall::default();
            pending.start_new("call_1".to_string(), Some(tool_name.to_string()));
            pending.append_arguments(raw_arguments);

            let finalized = pending
                .finalize(ToolCallBoundary::FinishReason)
                .expect("finalized tool");

            assert_eq!(finalized.arguments, json!({}), "tool={tool_name}");
            assert!(finalized.is_error, "tool={tool_name}");
        }
    }

    #[test]
    fn incomplete_json_object_for_single_field_tools_stays_invalid() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Bash".to_string()));
        pending.append_arguments("{\"command\": \"git log --since=\\\"2026-05-02\\\" --oneline --stat");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!({}));
        assert!(finalized.is_error);
    }

    #[test]
    fn does_not_wrap_incomplete_json_object_as_raw_string_argument() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Bash".to_string()));
        pending.append_arguments("{\"command\": ");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!({}));
        assert!(finalized.is_error);
    }

    #[test]
    fn does_not_repair_incomplete_json_object_for_multifield_tools() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Task".to_string()));
        pending.append_arguments(
            "{\"description\":\"Explore northhing project structure\",\"prompt\":\"read README\\n\\nthoroughness: very",
        );

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!({}));
        assert!(finalized.is_error);
    }

    #[test]
    fn does_not_repair_object_without_key_value_payload() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Bash".to_string()));
        pending.append_arguments("{");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!({}));
        assert!(finalized.is_error);
    }

    #[test]
    fn does_not_execute_truncated_incomplete_json_object() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Bash".to_string()));
        pending.append_arguments("{\"command\": \"git log --since=\\\"2026-05-02\\\" --on");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!({}));
        assert!(finalized.is_error);
    }

    #[test]
    fn json_string_arguments_for_single_field_tools_are_schema_errors_not_rewritten() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Bash".to_string()));
        pending.append_arguments("\"git status\"");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!("git status"));
        assert!(!finalized.is_error);
    }

    #[test]
    fn fenced_raw_arguments_for_single_field_tools_stay_invalid_json() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Bash".to_string()));
        pending.append_arguments("```bash\npnpm run lint:web\n```");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!({}));
        assert!(finalized.is_error);
    }

    #[test]
    fn does_not_repair_raw_string_arguments_for_multifield_tools() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Edit".to_string()));
        pending.append_arguments("src/main.rs");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!({}));
        assert!(finalized.is_error);
    }

    #[test]
    fn json_with_one_extra_trailing_right_brace_stays_invalid() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("tool_a".to_string()));
        pending.append_arguments("{\"a\":1}}");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.raw_arguments, "{\"a\":1}}");
        assert_eq!(finalized.arguments, json!({}));
        assert!(finalized.is_error);
    }

    #[test]
    fn finalized_arguments_preserve_object_fields() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("tool_a".to_string()));
        pending.append_arguments("{\"a\":1,\"b\":\"x\"}");

        let finalized = pending
            .finalize(ToolCallBoundary::EndOfAggregation)
            .expect("finalized tool");

        assert_eq!(finalized.arguments["a"], json!(1));
        assert_eq!(finalized.arguments["b"], json!("x"));
    }

    #[test]
    fn replace_arguments_overwrites_partial_buffer() {
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("tool_a".to_string()));
        pending.append_arguments("{\"city\":\"Bei");
        pending.replace_arguments("{\"city\":\"Beijing\"}");

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert_eq!(finalized.arguments, json!({"city": "Beijing"}));
        assert!(!finalized.is_error);
    }

    #[test]
    fn manages_multiple_pending_tool_calls_by_index() {
        let mut pending = PendingToolCalls::default();

        assert_eq!(
            pending
                .apply_delta(
                    ToolCallStreamKey::Indexed(0),
                    Some("call_1".to_string()),
                    Some("tool_a".to_string()),
                    None,
                    false,
                )
                .early_detected,
            Some(EarlyDetectedToolCall {
                tool_id: "call_1".to_string(),
                tool_name: "tool_a".to_string(),
            })
        );
        assert_eq!(
            pending
                .apply_delta(
                    ToolCallStreamKey::Indexed(1),
                    Some("call_2".to_string()),
                    Some("tool_b".to_string()),
                    None,
                    false,
                )
                .early_detected,
            Some(EarlyDetectedToolCall {
                tool_id: "call_2".to_string(),
                tool_name: "tool_b".to_string(),
            })
        );

        pending.apply_delta(
            ToolCallStreamKey::Indexed(0),
            None,
            None,
            Some("{\"a\":1}".to_string()),
            false,
        );
        pending.apply_delta(
            ToolCallStreamKey::Indexed(1),
            None,
            None,
            Some("{\"b\":2}".to_string()),
            false,
        );

        let finalized = pending.finalize_all(ToolCallBoundary::FinishReason);
        assert_eq!(finalized.len(), 2);
        assert_eq!(finalized[0].tool_id, "call_1");
        assert_eq!(finalized[0].arguments, json!({"a": 1}));
        assert_eq!(finalized[1].tool_id, "call_2");
        assert_eq!(finalized[1].arguments, json!({"b": 2}));
    }

    #[test]
    fn id_only_prelude_is_attached_to_following_payload_without_id() {
        let mut pending = PendingToolCalls::default();

        let prelude = pending.apply_delta(
            ToolCallStreamKey::Indexed(0),
            Some("call_1".to_string()),
            None,
            None,
            false,
        );
        assert_eq!(prelude.early_detected, None);
        assert_eq!(prelude.params_partial, None);

        let payload = pending.apply_delta(
            ToolCallStreamKey::Indexed(0),
            None,
            Some("tool_a".to_string()),
            Some("{\"a\":1}".to_string()),
            false,
        );
        assert_eq!(
            payload.early_detected,
            Some(EarlyDetectedToolCall {
                tool_id: "call_1".to_string(),
                tool_name: "tool_a".to_string(),
            })
        );
        assert_eq!(
            payload.params_partial,
            Some(ToolCallParamsChunk {
                tool_id: "call_1".to_string(),
                tool_name: "tool_a".to_string(),
                params_chunk: "{\"a\":1}".to_string(),
            })
        );
    }

    #[test]
    fn id_only_orphan_is_dropped_on_finalize() {
        let mut pending = PendingToolCalls::default();

        let outcome = pending.apply_delta(
            ToolCallStreamKey::Indexed(1),
            Some("call_orphan".to_string()),
            None,
            None,
            false,
        );
        assert!(outcome.finalized_previous.is_none());
        assert!(outcome.early_detected.is_none());
        assert!(outcome.params_partial.is_none());
        assert!(pending.finalize_all(ToolCallBoundary::FinishReason).is_empty());
    }

    #[test]
    fn empty_argument_delta_is_ignored() {
        let mut pending = PendingToolCalls::default();

        let header = pending.apply_delta(
            ToolCallStreamKey::Indexed(0),
            Some("call_1".to_string()),
            Some("tool_a".to_string()),
            Some(String::new()),
            false,
        );
        assert_eq!(
            header.early_detected,
            Some(EarlyDetectedToolCall {
                tool_id: "call_1".to_string(),
                tool_name: "tool_a".to_string(),
            })
        );
        assert!(header.params_partial.is_none());

        let empty_delta = pending.apply_delta(ToolCallStreamKey::Indexed(0), None, None, Some(String::new()), false);
        assert!(empty_delta.finalized_previous.is_none());
        assert!(empty_delta.early_detected.is_none());
        assert!(empty_delta.params_partial.is_none());
    }

    // ------------------------------------------------------------------
    // Truncation recovery tests
    // ------------------------------------------------------------------

    #[test]
    fn write_truncated_mid_content_string_is_recovered() {
        // Reproduces the deep-research dump: the model hit max_tokens while
        // streaming `content`, so the JSON ends inside the string literal
        // with no closing `"` and no closing `}`.
        let raw = "{\"file_path\": \"/tmp/report.md\", \"content\": \"# Report\\n\\nA long body that was cut";
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Write".to_string()));
        pending.append_arguments(raw);

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert!(!finalized.is_error, "Write recovery should succeed");
        assert!(finalized.recovered_from_truncation);
        assert_eq!(
            finalized.arguments,
            json!({
                "file_path": "/tmp/report.md",
                "content": "# Report\n\nA long body that was cut"
            })
        );
    }

    #[test]
    fn write_like_recovery_classification_matches_tool_presentation_contract() {
        for tool_name in [
            "Write",
            "file_write",
            "write_notebook",
            "Read",
            "Edit",
            "AskUserQuestion",
            "TodoWrite",
        ] {
            assert_eq!(
                super::is_write_like_tool_name(tool_name),
                northhing_agent_tools::is_write_like_tool_name(tool_name),
                "tool_name={tool_name}"
            );
        }
    }

    #[test]
    fn write_truncated_with_chinese_multibyte_is_recovered() {
        let raw = "{\"file_path\": \"/tmp/r.md\", \"content\": \"深度研究报告：未完";
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Write".to_string()));
        pending.append_arguments(raw);

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert!(!finalized.is_error);
        assert!(finalized.recovered_from_truncation);
        assert_eq!(finalized.arguments["content"].as_str(), Some("深度研究报告：未完"));
    }

    #[test]
    fn bash_truncated_mid_command_still_errors_but_records_truncation() {
        let raw = r#"{"command": "git log --since=\"2026-05-02\" --on"#;
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("Bash".to_string()));
        pending.append_arguments(raw);

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        // We never execute a partial shell command.
        assert!(finalized.is_error);
        assert_eq!(finalized.arguments, json!({}));
        // But the truncation is recorded so the surface error message and
        // diagnostic dump can distinguish "truncated" from "model emitted
        // bad JSON".
        assert!(finalized.recovered_from_truncation);
    }

    #[test]
    fn repair_refuses_truncation_after_colon() {
        // We can't invent the missing value, so this must not auto-repair.
        assert!(repair_truncated_json(r#"{"a": 1, "b":"#).is_none());
    }

    #[test]
    fn repair_refuses_truncation_after_comma() {
        assert!(repair_truncated_json(r#"{"a": 1,"#).is_none());
    }

    #[test]
    fn repair_returns_none_for_already_valid_json() {
        // Already balanced — repair has nothing to do (parser would have
        // succeeded anyway).
        assert!(repair_truncated_json(r#"{"a": 1}"#).is_none());
    }

    #[test]
    fn repair_closes_nested_brackets_in_correct_order() {
        let raw = r#"{"a": [1, 2, {"b": "incomplete"#;
        let repaired = repair_truncated_json(raw).expect("repaired");
        let parsed: serde_json::Value = serde_json::from_str(&repaired).expect("repaired is valid JSON");
        assert_eq!(parsed, json!({"a": [1, 2, {"b": "incomplete"}]}));
    }

    #[test]
    fn repair_preserves_escaped_quote_inside_truncated_string() {
        let raw = r#"{"content": "she said \"hello\" and then"#;
        let repaired = repair_truncated_json(raw).expect("repaired");
        let parsed: serde_json::Value = serde_json::from_str(&repaired).expect("valid JSON");
        assert_eq!(parsed["content"].as_str(), Some("she said \"hello\" and then"));
    }

    #[test]
    fn ask_user_question_truncated_mid_chinese_string_is_recovered() {
        let raw = r#"{"questions": [{"header": "重试场景", "multiSelect": true, "options": [{"description": "当消息发送后后端返回失败（消息气泡显示为红色失败状态，有 model rounds 但 status='error'），在失败气泡旁增加重试按钮，点击后重新发送该消息", "label": "失败消息气泡上加重试按钮"}]}]}"#;
        // Truncate mid-Chinese-string, after a colon that opened the value
        let truncated = &raw[..raw.find("消息气泡显示为红色失败状态").unwrap() + "消息气泡显示为红色失败状态".len()];
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("AskUserQuestion".to_string()));
        pending.append_arguments(truncated);

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert!(!finalized.is_error);
        assert!(finalized.recovered_from_truncation);
    }

    #[test]
    fn ask_user_question_truncated_mid_options_is_recovered() {
        // Truncation right after a completed description value's closing quote + comma
        let raw = r#"{"questions": [{"header": "场景", "multiSelect": true, "options": [{"description": "第一条描述", "label": "选项一"}, {"description": "第二条描"#;
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("AskUserQuestion".to_string()));
        pending.append_arguments(raw);

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert!(!finalized.is_error);
        assert!(finalized.recovered_from_truncation);
        let questions = finalized.arguments["questions"].as_array().unwrap();
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0]["options"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn todo_write_truncated_mid_content_is_recovered() {
        let raw = r#"{"todos": [{"id": "1", "content": "完成重构并优化性能", "status": "in_progress"}, {"id": "2", "content": "编写单元测"#;
        let mut pending = PendingToolCall::default();
        pending.start_new("call_1".to_string(), Some("TodoWrite".to_string()));
        pending.append_arguments(raw);

        let finalized = pending
            .finalize(ToolCallBoundary::FinishReason)
            .expect("finalized tool");

        assert!(!finalized.is_error);
        assert!(finalized.recovered_from_truncation);
        let todos = finalized.arguments["todos"].as_array().unwrap();
        assert_eq!(todos.len(), 2);
    }
}
