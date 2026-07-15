pub mod context_format;
pub mod context_init;
pub mod context_persist;
pub mod context_runtime;

#[allow(unused_imports)]
pub(crate) use context_init::build_tool_description_context;
#[allow(unused_imports)]
pub(crate) use context_init::build_tool_use_context_for_execution_context;
#[allow(unused_imports)]
pub(crate) use context_init::build_tool_use_context_for_task;
pub use context_init::ToolUseContext;
#[allow(unused_imports)]
pub(crate) use context_runtime::call_tool_with_runtime_hooks;
#[allow(unused_imports)]
pub(crate) use context_runtime::call_with_tool_runtime_hooks;

#[cfg(test)]
mod tests {
    use super::context_init::build_tool_description_context;
    use super::context_init::build_tool_use_context_for_task;
    use super::context_runtime::call_tool_with_runtime_hooks;
    use super::context_runtime::call_with_tool_runtime_hooks;
    use super::*;
    use crate::agentic::deep_review_policy::deep_review_shared_context_measurement_snapshot;
    use crate::agentic::tools::{ToolPathOperation, ToolPathPolicy, ToolRuntimeRestrictions};
    use crate::agentic::WorkspaceBinding;
    use crate::service::remote_ssh::workspace_state::workspace_session_identity;
    use crate::{NortHingError, NortHingResult};
    use northhing_agent_tools::{PortableToolContextProvider, ToolWorkspaceKind};
    use northhing_runtime_ports::DelegationPolicy;
    use serde_json::json;
    use std::collections::{BTreeSet, HashMap};
    use std::path::PathBuf;
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    #[cfg(test)]
    mod context_facts_tests {
        use super::*;

        fn local_context(root: &str) -> ToolUseContext {
            ToolUseContext {
                tool_call_id: None,
                agent_type: None,
                session_id: None,
                dialog_turn_id: None,
                workspace: Some(WorkspaceBinding::new(None, PathBuf::from(root))),
                unlocked_collapsed_tools: Vec::new(),
                custom_data: HashMap::new(),
                computer_use_host: None,
                runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
                runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
                actor_runtime: None,
            }
        }

        #[test]
        fn tool_context_facts_preserve_portable_fields_without_runtime_handles() {
            let context = ToolUseContext {
                tool_call_id: Some("call-1".to_string()),
                agent_type: Some("Agentic".to_string()),
                session_id: Some("session-1".to_string()),
                dialog_turn_id: Some("turn-1".to_string()),
                workspace: Some(WorkspaceBinding::new(None, PathBuf::from("/repo/project"))),
                unlocked_collapsed_tools: vec!["WebFetch".to_string()],
                custom_data: HashMap::new(),
                computer_use_host: None,
                runtime_tool_restrictions: ToolRuntimeRestrictions {
                    allowed_tool_names: BTreeSet::from(["Read".to_string()]),
                    denied_tool_names: BTreeSet::from(["Bash".to_string()]),
                    denied_tool_messages: Default::default(),
                    path_policy: Default::default(),
                },
                runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
                actor_runtime: None,
            };

            let facts = context.to_tool_context_facts();

            assert_eq!(facts.tool_call_id.as_deref(), Some("call-1"));
            assert_eq!(facts.agent_type.as_deref(), Some("Agentic"));
            assert_eq!(facts.session_id.as_deref(), Some("session-1"));
            assert_eq!(facts.dialog_turn_id.as_deref(), Some("turn-1"));
            assert_eq!(facts.workspace_kind, Some(ToolWorkspaceKind::Local));
            assert_eq!(facts.workspace_root.as_deref(), Some("/repo/project"));
            assert!(facts.runtime_tool_restrictions.is_tool_allowed("Read"));
            assert!(!facts.runtime_tool_restrictions.is_tool_allowed("Bash"));

            let value = serde_json::to_value(&facts).expect("serialize context facts");
            assert!(value.get("unlockedCollapsedTools").is_none());
            assert!(value.get("computer_use_host").is_none());
            assert!(value.get("workspace_services").is_none());
            assert!(value.get("cancellationToken").is_none());
        }

        #[test]
        fn tool_context_facts_omit_runtime_owner_fields_even_when_context_is_populated() {
            let mut custom_data = HashMap::new();
            custom_data.insert("checkpoint".to_string(), serde_json::json!({ "kind": "runtime-only" }));

            let context = ToolUseContext {
                tool_call_id: Some("tool-runtime".to_string()),
                agent_type: Some("Agentic".to_string()),
                session_id: Some("session-runtime".to_string()),
                dialog_turn_id: Some("turn-runtime".to_string()),
                workspace: Some(WorkspaceBinding::new(None, PathBuf::from("/repo/runtime"))),
                unlocked_collapsed_tools: vec!["WebFetch".to_string(), "Git".to_string()],
                custom_data,
                computer_use_host: None,
                runtime_tool_restrictions: ToolRuntimeRestrictions {
                    allowed_tool_names: BTreeSet::from(["Read".to_string(), "GetToolSpec".to_string()]),
                    denied_tool_names: BTreeSet::from(["Bash".to_string()]),
                    denied_tool_messages: Default::default(),
                    path_policy: Default::default(),
                },
                runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::new(
                    None,
                    Some(tokio_util::sync::CancellationToken::new()),
                ),
                actor_runtime: None,
            };

            let facts = PortableToolContextProvider::tool_context_facts(&context);

            assert_eq!(facts.tool_call_id.as_deref(), Some("tool-runtime"));
            assert_eq!(facts.workspace_kind, Some(ToolWorkspaceKind::Local));
            assert_eq!(facts.workspace_root.as_deref(), Some("/repo/runtime"));
            assert!(facts.runtime_tool_restrictions.is_tool_allowed("Read"));
            assert!(facts.runtime_tool_restrictions.is_tool_allowed("GetToolSpec"));
            assert!(!facts.runtime_tool_restrictions.is_tool_allowed("Bash"));

            let value = serde_json::to_value(&facts).expect("serialize runtime context facts");
            for runtime_only_field in [
                "unlockedCollapsedTools",
                "customData",
                "computerUseHost",
                "cancellationToken",
                "workspaceServices",
            ] {
                assert!(
                    value.get(runtime_only_field).is_none(),
                    "{runtime_only_field} must remain outside portable facts"
                );
            }
        }

        #[test]
        fn tool_context_facts_use_normalized_remote_workspace_identity() {
            let session_identity =
                workspace_session_identity("/home/wsp//projects/test/", Some("conn-1"), Some("ssh.dev"))
                    .expect("remote identity");
            let context = ToolUseContext {
                tool_call_id: None,
                agent_type: None,
                session_id: Some("session-remote".to_string()),
                dialog_turn_id: None,
                workspace: Some(WorkspaceBinding::new_remote(
                    Some("workspace-remote".to_string()),
                    PathBuf::from("/home/wsp//projects/test/"),
                    "conn-1".to_string(),
                    "Dev SSH".to_string(),
                    session_identity,
                )),
                unlocked_collapsed_tools: Vec::new(),
                custom_data: HashMap::new(),
                computer_use_host: None,
                runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
                runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
                actor_runtime: None,
            };

            let facts = context.to_tool_context_facts();

            assert_eq!(facts.workspace_kind, Some(ToolWorkspaceKind::Remote));
            assert_eq!(facts.workspace_root.as_deref(), Some("/home/wsp/projects/test"));

            let value = serde_json::to_value(&facts).expect("serialize remote context facts");
            assert!(value.get("connectionId").is_none());
            assert!(value.get("connectionName").is_none());
            assert!(value.get("workspace_services").is_none());
        }

        #[test]
        fn tool_use_context_implements_portable_context_provider() {
            fn assert_provider<T: PortableToolContextProvider>() {}
            assert_provider::<ToolUseContext>();

            let context = local_context("/repo/project");

            let facts = PortableToolContextProvider::tool_context_facts(&context);

            assert_eq!(facts.workspace_kind, Some(ToolWorkspaceKind::Local));
            assert_eq!(facts.workspace_root.as_deref(), Some("/repo/project"));
        }
    }

    #[cfg(test)]
    mod path_resolution_tests {
        use super::*;

        fn local_context(root: &str) -> ToolUseContext {
            ToolUseContext {
                tool_call_id: None,
                agent_type: None,
                session_id: None,
                dialog_turn_id: None,
                workspace: Some(WorkspaceBinding::new(None, PathBuf::from(root))),
                unlocked_collapsed_tools: Vec::new(),
                custom_data: HashMap::new(),
                computer_use_host: None,
                runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
                runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
                actor_runtime: None,
            }
        }

        fn remote_context(root: &str, workspace_id: Option<String>) -> ToolUseContext {
            let session_identity =
                workspace_session_identity(root, Some("conn-1"), Some("ssh.dev")).expect("remote identity");
            ToolUseContext {
                tool_call_id: None,
                agent_type: None,
                session_id: None,
                dialog_turn_id: None,
                workspace: Some(WorkspaceBinding::new_remote(
                    workspace_id,
                    PathBuf::from(root),
                    "conn-1".to_string(),
                    "Dev SSH".to_string(),
                    session_identity,
                )),
                unlocked_collapsed_tools: Vec::new(),
                custom_data: HashMap::new(),
                computer_use_host: None,
                runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
                runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
                actor_runtime: None,
            }
        }

        fn context_with_restrictions(root: &str, runtime_tool_restrictions: ToolRuntimeRestrictions) -> ToolUseContext {
            ToolUseContext {
                runtime_tool_restrictions,
                ..local_context(root)
            }
        }

        fn context_without_workspace() -> ToolUseContext {
            ToolUseContext {
                tool_call_id: None,
                agent_type: None,
                session_id: None,
                dialog_turn_id: None,
                workspace: None,
                unlocked_collapsed_tools: Vec::new(),
                custom_data: HashMap::new(),
                computer_use_host: None,
                runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
                runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
                actor_runtime: None,
            }
        }

        #[test]
        fn workspace_path_resolution_allows_absolute_paths_outside_local_workspace() {
            let context = local_context("/repo/project");

            let resolved = context
                .resolve_workspace_tool_path("/tmp/pr_body.md")
                .expect("local sessions may resolve paths outside the workspace root");

            assert_eq!(PathBuf::from(resolved), PathBuf::from("/tmp/pr_body.md"));
        }

        #[test]
        fn workspace_path_resolution_rejects_absolute_paths_outside_remote_workspace() {
            let context = remote_context("/home/wsp/projects/test", None);

            let err = context
                .resolve_workspace_tool_path("/tmp/pr_body.md")
                .expect_err("remote sessions must stay within the workspace root");

            assert!(err.to_string().contains("outside current workspace"));
        }

        #[test]
        fn workspace_path_resolution_rejects_root_without_workspace() {
            let context = context_without_workspace();

            let err = context
                .resolve_workspace_tool_path("/")
                .expect_err("workspace tools must not scan the host root without a workspace");

            assert!(err.to_string().contains("workspace path is required"));
        }

        #[test]
        fn workspace_path_resolution_allows_paths_inside_local_workspace() {
            let context = local_context("/repo/project");

            let resolved = context
                .resolve_workspace_tool_path("/repo/project/src/main.rs")
                .expect("absolute paths inside the workspace remain valid");

            assert_eq!(PathBuf::from(resolved), PathBuf::from("/repo/project/src/main.rs"));
        }

        #[test]
        fn remote_runtime_artifact_reference_uses_runtime_uri_scope() {
            let context = remote_context("/home/wsp/projects/test", Some("workspace-123".to_string()));

            let reference = context
                .build_runtime_artifact_reference(r"plans\demo.plan.md")
                .expect("remote runtime artifacts should use URI references");

            assert_eq!(reference, "northhing://runtime/workspace-123/plans/demo.plan.md");
        }

        #[test]
        fn runtime_uri_resolution_rejects_different_workspace_scope() {
            let context = remote_context("/home/wsp/projects/test", Some("workspace-123".to_string()));

            let err = context
                .resolve_tool_path("northhing://runtime/workspace-456/plans/demo.plan.md")
                .expect_err("runtime artifact scopes must match the active workspace");

            assert!(err.to_string().contains("does not match the current workspace"));
        }

        #[test]
        fn runtime_uri_scope_error_takes_precedence_without_workspace() {
            let context = context_without_workspace();

            let err = context
                .resolve_tool_path("northhing://runtime/workspace-456/plans/demo.plan.md")
                .expect_err("runtime URI scope should be validated before runtime root lookup");

            assert!(err.to_string().contains("does not match the current workspace"));
        }

        #[test]
        fn workspace_absolute_detection_uses_remote_posix_semantics() {
            let context = remote_context("/home/wsp/projects/test", None);

            assert!(context.workspace_path_is_effectively_absolute("/home/wsp/projects/test/src/lib.rs"));
            assert!(!context.workspace_path_is_effectively_absolute("src/lib.rs"));
        }

        #[test]
        fn path_policy_allows_only_configured_local_roots() {
            let temp_root =
                std::env::temp_dir().join(format!("northhing-tool-context-policy-{}", uuid::Uuid::new_v4()));
            let allowed_root = temp_root.join("allowed");
            std::fs::create_dir_all(&allowed_root).expect("create allowed root");
            let context = context_with_restrictions(
                temp_root.to_string_lossy().as_ref(),
                ToolRuntimeRestrictions {
                    path_policy: ToolPathPolicy {
                        write_roots: vec![allowed_root.to_string_lossy().to_string()],
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );

            let allowed = context
                .resolve_tool_path(&allowed_root.join("file.txt").to_string_lossy())
                .expect("allowed path should resolve");
            context
                .enforce_path_operation(ToolPathOperation::Write, &allowed)
                .expect("path within configured root should be allowed");

            let blocked = context
                .resolve_tool_path(&temp_root.join("blocked/file.txt").to_string_lossy())
                .expect("blocked path should still resolve before policy enforcement");
            let err = context
                .enforce_path_operation(ToolPathOperation::Write, &blocked)
                .expect_err("path outside configured root should be blocked");

            assert!(err.to_string().contains("is not allowed for write"));

            let _ = std::fs::remove_dir_all(&temp_root);
        }
    }

    #[cfg(test)]
    mod call_runtime_tests {
        use super::*;

        struct MeasurementReadTool;

        #[async_trait::async_trait]
        impl crate::agentic::tools::framework::Tool for MeasurementReadTool {
            fn name(&self) -> &str {
                "Read"
            }

            async fn description(&self) -> NortHingResult<String> {
                Ok("Read file".to_string())
            }

            fn short_description(&self) -> String {
                "Read file".to_string()
            }

            fn input_schema(&self) -> serde_json::Value {
                json!({
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string" }
                    }
                })
            }

            async fn call_impl(
                &self,
                _input: &serde_json::Value,
                _context: &ToolUseContext,
            ) -> NortHingResult<Vec<crate::agentic::tools::framework::ToolResult>> {
                Ok(vec![crate::agentic::tools::framework::ToolResult::ok(
                    json!({ "ok": true }),
                    Some("ok".to_string()),
                )])
            }
        }

        fn context_with_cancellation(cancellation_token: CancellationToken) -> ToolUseContext {
            ToolUseContext {
                tool_call_id: None,
                agent_type: None,
                session_id: None,
                dialog_turn_id: None,
                workspace: None,
                unlocked_collapsed_tools: Vec::new(),
                custom_data: HashMap::new(),
                computer_use_host: None,
                runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
                runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::new(None, Some(cancellation_token)),
                actor_runtime: None,
            }
        }

        #[tokio::test]
        async fn tool_call_runtime_hook_returns_cancelled_before_impl_completes() {
            let cancellation_token = CancellationToken::new();
            cancellation_token.cancel();
            let context = context_with_cancellation(cancellation_token);

            let result = call_with_tool_runtime_hooks("Read", &json!({}), &context, async {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                Ok(vec![crate::agentic::tools::framework::ToolResult::ok(
                    json!({ "unexpected": true }),
                    None,
                )])
            })
            .await;

            assert!(matches!(result, Err(NortHingError::Cancelled(message)) if message == "Tool execution cancelled"));
        }

        #[tokio::test]
        async fn tool_call_runtime_hook_preserves_success_result_without_cancellation() {
            let context = ToolUseContext {
                tool_call_id: None,
                agent_type: None,
                session_id: None,
                dialog_turn_id: None,
                workspace: None,
                unlocked_collapsed_tools: Vec::new(),
                custom_data: HashMap::new(),
                computer_use_host: None,
                runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
                runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
                actor_runtime: None,
            };

            let result: NortHingResult<Vec<crate::agentic::tools::framework::ToolResult>> =
                call_with_tool_runtime_hooks("Read", &json!({}), &context, async {
                    Ok(vec![crate::agentic::tools::framework::ToolResult::ok(
                        json!({ "ok": true }),
                        Some("ok".to_string()),
                    )])
                })
                .await;

            let result = result.expect("tool result should pass through");
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].content()["ok"], true);
        }

        #[tokio::test]
        async fn call_records_deep_review_read_file_measurement_without_touching_result() {
            let parent_turn_id = format!("turn-runtime-measure-{}", uuid::Uuid::new_v4());
            let mut custom_data = HashMap::new();
            custom_data.insert(
                "deep_review_parent_dialog_turn_id".to_string(),
                json!(parent_turn_id.clone()),
            );
            custom_data.insert("deep_review_subagent_role".to_string(), json!("reviewer"));
            custom_data.insert("deep_review_subagent_type".to_string(), json!("ReviewSecurity"));
            let context = ToolUseContext {
                tool_call_id: Some("tool-read".to_string()),
                agent_type: Some("ReviewSecurity".to_string()),
                session_id: Some("subagent-session".to_string()),
                dialog_turn_id: Some("subagent-turn".to_string()),
                workspace: None,
                unlocked_collapsed_tools: Vec::new(),
                custom_data,
                computer_use_host: None,
                runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
                runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
                actor_runtime: None,
            };
            let tool = MeasurementReadTool;

            let result = call_tool_with_runtime_hooks(&tool, &json!({ "file_path": ".\\src\\lib.rs" }), &context)
                .await
                .expect("read tool call should succeed");
            call_tool_with_runtime_hooks(&tool, &json!({ "file_path": "src/lib.rs" }), &context)
                .await
                .expect("read tool call should succeed");

            assert_eq!(result.len(), 1);
            let snapshot = deep_review_shared_context_measurement_snapshot(&parent_turn_id);
            assert_eq!(snapshot.total_calls, 2);
            assert_eq!(snapshot.duplicate_calls, 1);
            assert_eq!(snapshot.repeated_contexts[0].tool_name, "Read");
            assert_eq!(snapshot.repeated_contexts[0].file_path, "src/lib.rs");
        }
    }

    #[cfg(test)]
    mod context_builder_tests {
        use super::*;

        #[test]
        fn tool_description_context_preserves_manifest_custom_data_shape() {
            let mut context_vars = HashMap::new();
            context_vars.insert(
                "primary_model_supports_image_understanding".to_string(),
                "false".to_string(),
            );

            let context = build_tool_description_context("coding", None, None, true, &context_vars, None);

            assert_eq!(context.agent_type.as_deref(), Some("coding"));
            assert!(context.tool_call_id.is_none());
            assert!(context.session_id.is_none());
            assert!(context.dialog_turn_id.is_none());
            assert!(context.workspace.is_none());
            assert!(context.unlocked_collapsed_tools.is_empty());
            assert!(context.cancellation_token().is_none());
            assert!(context.workspace_services().is_none());
            assert!(context.runtime_tool_restrictions.is_tool_allowed("Write"));
            assert_eq!(
                context.custom_data["primary_model_supports_image_understanding"],
                json!("false")
            );
        }
    }

    #[cfg(test)]
    mod task_context_tests {
        use super::*;
        use crate::agentic::core::ToolCall;
        use crate::agentic::tools::pipeline::{
            SubagentParentInfo, ToolExecutionContext, ToolExecutionOptions, ToolTask,
        };

        fn task_with_context_vars() -> ToolTask {
            let mut context_vars = HashMap::new();
            context_vars.insert("turn_index".to_string(), "7".to_string());
            context_vars.insert("primary_model_provider".to_string(), "openai".to_string());
            context_vars.insert(
                "primary_model_supports_image_understanding".to_string(),
                "true".to_string(),
            );
            context_vars.insert("acp_transport".to_string(), "true".to_string());
            context_vars.insert(
                "deep_review_run_manifest".to_string(),
                r#"{"run_id":"run-1"}"#.to_string(),
            );
            context_vars.insert("deep_review_subagent_role".to_string(), "reviewer".to_string());
            context_vars.insert("deep_review_subagent_type".to_string(), "ReviewSecurity".to_string());

            ToolTask::new(
                ToolCall {
                    tool_id: "tool_context_1".to_string(),
                    tool_name: "WebFetch".to_string(),
                    arguments: json!({ "url": "https://example.com" }),
                    raw_arguments: None,
                    is_error: false,
                    recovered_from_truncation: false,
                },
                ToolExecutionContext {
                    session_id: "session_1".to_string(),
                    dialog_turn_id: "turn_1".to_string(),
                    round_id: "round_1".to_string(),
                    agent_type: "agent".to_string(),
                    workspace: None,
                    context_vars,
                    subagent_parent_info: Some(SubagentParentInfo {
                        tool_call_id: "parent_tool".to_string(),
                        session_id: "parent_session".to_string(),
                        dialog_turn_id: "parent_turn".to_string(),
                    }),
                    delegation_policy: DelegationPolicy::top_level().spawn_child(),
                    collapsed_tools: vec!["WebFetch".to_string()],
                    unlocked_collapsed_tools: vec!["WebFetch".to_string()],
                    allowed_tools: vec!["WebFetch".to_string()],
                    runtime_tool_restrictions: ToolRuntimeRestrictions {
                        allowed_tool_names: BTreeSet::from(["WebFetch".to_string()]),
                        denied_tool_names: BTreeSet::from(["Bash".to_string()]),
                        denied_tool_messages: Default::default(),
                        path_policy: Default::default(),
                    },
                    steering_interrupt: None,
                    workspace_services: None,
                },
                ToolExecutionOptions::default(),
            )
        }

        #[test]
        fn tool_task_context_materialization_preserves_runtime_fields() {
            let task = task_with_context_vars();
            let context = build_tool_use_context_for_task(&task, None, CancellationToken::new(), None);

            assert_eq!(context.tool_call_id.as_deref(), Some("tool_context_1"));
            assert_eq!(context.agent_type.as_deref(), Some("agent"));
            assert_eq!(context.session_id.as_deref(), Some("session_1"));
            assert_eq!(context.dialog_turn_id.as_deref(), Some("turn_1"));
            assert_eq!(context.unlocked_collapsed_tools, vec!["WebFetch"]);
            assert!(context.cancellation_token().is_some());
            assert!(context.runtime_tool_restrictions.is_tool_allowed("WebFetch"));
            assert!(!context.runtime_tool_restrictions.is_tool_allowed("Bash"));
            assert_eq!(context.custom_data["turn_index"], json!(7));
            assert_eq!(context.custom_data["primary_model_provider"], json!("openai"));
            assert_eq!(
                context.custom_data["primary_model_supports_image_understanding"],
                json!(true)
            );
            assert_eq!(context.custom_data["acp_transport"], json!(true));
            assert_eq!(
                context.custom_data["deep_review_run_manifest"],
                json!({ "run_id": "run-1" })
            );
            assert_eq!(
                context.custom_data["deep_review_parent_tool_call_id"],
                json!("parent_tool")
            );
            assert_eq!(
                context.custom_data["deep_review_parent_session_id"],
                json!("parent_session")
            );
            assert_eq!(
                context.custom_data["deep_review_parent_dialog_turn_id"],
                json!("parent_turn")
            );

            let facts = context.to_tool_context_facts();
            let value = serde_json::to_value(&facts).expect("serialize context facts");
            assert_eq!(value["toolCallId"], "tool_context_1");
            assert_eq!(value["sessionId"], "session_1");
            assert!(value.get("unlockedCollapsedTools").is_none());
            assert!(value.get("customData").is_none());
            assert!(value.get("cancellationToken").is_none());
        }
    }
}
