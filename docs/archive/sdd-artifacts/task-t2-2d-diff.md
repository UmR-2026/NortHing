BASE: 9c14d22 (working-tree diff, task not yet committed) — ROUND 2 (post-fix: 3 formatting hunks reverted)

## git log (base context)
9c14d22 sdd: T2-2c ledger line + recon/brief/report/review/diff artifacts
fa88342 chore: remove core remote_connect module + SAR remote adapters (T2-2c, review clean)
bdc3f9c sdd: T2-2b ledger line + brief/report/review/diff artifacts

## git diff --stat
 .../core/src/agentic/agents/prompt_builder/mod.rs  |  9 ---
 .../agentic/agents/prompt_builder/system_prompt.rs | 11 +---
 .../src/agentic/agents/prompt_builder/tests.rs     | 15 -----
 .../core/src/agentic/coordination/coordinator.rs   |  3 -
 .../agentic/coordination/dialog_turn/compaction.rs |  3 -
 .../agentic/coordination/dialog_turn/session.rs    |  3 -
 .../coordination/dialog_turn/sub_handle_out.rs     |  7 ---
 .../coordination/dialog_turn/sub_handle_state.rs   | 10 +---
 .../coordination/dialog_turn/thread_goal.rs        |  3 -
 .../agentic/coordination/dialog_turn/workspace.rs  |  3 -
 .../core/src/agentic/execution/ai_message_build.rs |  1 -
 .../core/src/agentic/execution/execution_engine.rs |  1 -
 .../core/src/agentic/execution/health_snapshot.rs  |  1 -
 .../core/src/agentic/execution/loop_detection.rs   |  1 -
 .../core/src/agentic/execution/multimodal.rs       |  1 -
 .../core/src/agentic/execution/token_pressure.rs   |  1 -
 .../core/src/agentic/execution/turn_finalize.rs    |  1 -
 .../core/src/agentic/execution/turn_init.rs        |  1 -
 .../core/src/agentic/execution/turn_lifecycle.rs   |  9 +--
 .../core/src/agentic/execution/turn_main_loop.rs   |  1 -
 .../core/src/agentic/execution/turn_tick.rs        |  1 -
 src/crates/assembly/core/src/agentic/mod.rs        |  1 -
 .../core/src/agentic/remote_file_delivery.rs       | 69 ----------------------
 .../tools/implementations/create_plan_tool.rs      | 21 +++----
 .../tools/tool_context_runtime/context_init.rs     |  9 ---
 25 files changed, 13 insertions(+), 173 deletions(-)

## git diff -U10
diff --git a/src/crates/assembly/core/src/agentic/agents/prompt_builder/mod.rs b/src/crates/assembly/core/src/agentic/agents/prompt_builder/mod.rs
index 7d34ce8..90e2ea1 100644
--- a/src/crates/assembly/core/src/agentic/agents/prompt_builder/mod.rs
+++ b/src/crates/assembly/core/src/agentic/agents/prompt_builder/mod.rs
@@ -1,12 +1,11 @@
 //! System prompts module providing main dialogue and agent dialogue prompts
-use crate::agentic::remote_file_delivery::user_workspace_relative_file_link;
 use crate::agentic::tools::implementations::ExecCommandTool;
 use crate::agentic::util::remote_workspace_layout::build_remote_workspace_layout_preview;
 use crate::agentic::workspace::WorkspaceBackend;
 use crate::agentic::WorkspaceBinding;
 use crate::service::agent_memory::{
     build_workspace_agent_memory_prompt, build_workspace_instruction_files_context,
     build_workspace_memory_files_context,
 };
 use crate::service::bootstrap::build_workspace_persona_prompt;
 use crate::service::config::get_app_language_code;
@@ -89,41 +88,38 @@ pub struct PromptBuilderContext {
     /// When set, file/shell tools target this remote environment; OS and path instructions follow it.
     pub remote_execution: Option<RemoteExecutionHints>,
     /// Pre-built tree text for `{PROJECT_LAYOUT}` when the workspace is not on the local disk.
     pub remote_project_layout: Option<String>,
     /// When `Some(false)`, system prompt append Computer use text-only guidance (no screenshot tool output).
     pub supports_image_understanding: Option<bool>,
     /// Dynamic tool listings injected outside tool descriptions for cache stability.
     pub tool_listing_sections: ToolListingSections,
     /// Runtime facts needed by the current model-visible tool set.
     pub runtime_context_needs: RuntimeContextNeeds,
-    /// Remote mobile/bot turns need `computer://` links for file delivery.
-    pub remote_file_delivery_channel: bool,
     /// Context window size from model config (tokens).
     pub context_window: Option<u32>,
     /// Max output tokens from model config.
     pub max_output_tokens: Option<u32>,
 }
 
 impl PromptBuilderContext {
     pub fn new(workspace_path: impl Into<String>, session_id: Option<String>, model_name: Option<String>) -> Self {
         Self {
             workspace_path: workspace_path.into().replace("\\", "/"),
             related_paths: Vec::new(),
             session_id,
             model_name,
             remote_execution: None,
             remote_project_layout: None,
             supports_image_understanding: None,
             tool_listing_sections: ToolListingSections::default(),
             runtime_context_needs: RuntimeContextNeeds::default(),
-            remote_file_delivery_channel: false,
             context_window: None,
             max_output_tokens: None,
         }
     }
 
     pub fn with_supports_image_understanding(mut self, supports: bool) -> Self {
         self.supports_image_understanding = Some(supports);
         self
     }
 
@@ -145,25 +141,20 @@ impl PromptBuilderContext {
     pub fn with_remote_prompt_overlay(
         mut self,
         execution: RemoteExecutionHints,
         project_layout: Option<String>,
     ) -> Self {
         self.remote_execution = Some(execution);
         self.remote_project_layout = project_layout;
         self
     }
 
-    pub fn with_remote_file_delivery_channel(mut self, enabled: bool) -> Self {
-        self.remote_file_delivery_channel = enabled;
-        self
-    }
-
     pub fn with_context_window(mut self, context_window: u32) -> Self {
         self.context_window = Some(context_window);
         self
     }
 
     pub fn with_max_output_tokens(mut self, max_output_tokens: u32) -> Self {
         self.max_output_tokens = Some(max_output_tokens);
         self
     }
 }
diff --git a/src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt.rs b/src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt.rs
index 6c9cdca..bb2a792 100644
--- a/src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt.rs
+++ b/src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt.rs
@@ -1,17 +1,16 @@
 use super::{
     PromptBuilder, PromptBuilderContext, PLACEHOLDER_AGENT_MEMORY, PLACEHOLDER_CLAW_WORKSPACE,
     PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK, PLACEHOLDER_LANGUAGE_PREFERENCE, PLACEHOLDER_PERSONA,
     PLACEHOLDER_SESSION_ID, PLACEHOLDER_VISUAL_MODE,
 };
 use crate::agentic::identity::{identity_exists, load_identity};
-use crate::agentic::remote_file_delivery::user_workspace_relative_file_link;
 use crate::service::agent_memory::build_workspace_agent_memory_prompt;
 use crate::service::bootstrap::build_workspace_persona_prompt;
 use crate::util::errors::NortHingResult;
 use std::path::Path;
 use tracing::warn;
 
 impl PromptBuilder {
     async fn build_workspace_persona_with_identity(&self, workspace: &Path) -> String {
         let persona = match build_workspace_persona_prompt(workspace).await {
             Ok(prompt) => prompt.unwrap_or_default(),
@@ -110,24 +109,21 @@ impl PromptBuilder {
             result = result.replace(PLACEHOLDER_SESSION_ID, &session_id);
         }
 
         if result.contains(PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK) {
             let session_id = resolved_session_id.unwrap_or_else(|| {
                 self.context
                     .session_id
                     .clone()
                     .unwrap_or_else(|| format!("unbound-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")))
             });
-            let report_link = user_workspace_relative_file_link(
-                &format!(".northhing/sessions/{session_id}/research/report.md"),
-                self.context.remote_file_delivery_channel,
-            );
+            let report_link = format!(".northhing/sessions/{session_id}/research/report.md");
             result = result.replace(PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK, &report_link);
         }
 
         if self.context.supports_image_understanding == Some(false) {
             result.push_str(
                 "\n\n# Computer use (text-only primary model)\n\n\
 The configured **primary model does not accept image inputs**. When using **`ComputerUse`** (or **`ControlHub`** with **`domain: \"browser\"`**):\n\
 - **Do not** use **`screenshot`** (desktop) and **avoid** `domain:\"browser\" action:\"screenshot\"` — the JPEG bytes will be unreadable.\n\
 - **ACTION PRIORITY:** 1) Terminal/CLI/system commands (`ExecCommand`, or `ComputerUse` `run_script`; use `WriteStdin`/`ExecControl` for running ExecCommand sessions) 2) Keyboard shortcuts (**`key_chord`**, **`type_text`**) 3) UI control: **`click_element`** (AX) → **`locate`** → **`move_to_text`** (use **`move_to_text_match_index`** when multiple OCR hits listed) → **`mouse_move`** (**`use_screen_coordinates`: true** with coordinates from tool JSON) → **`click`**. For browser work prefer `snapshot` → click by `@e*` ref over screenshots.\n\
 - **Never guess coordinates** — always use precise methods (AX, OCR, system coordinates from tool results, or browser snapshot refs).\n",
@@ -247,24 +243,21 @@ The configured **primary model does not accept image inputs**. When using **`Com
             result = result.replace(PLACEHOLDER_SESSION_ID, &session_id);
         }
 
         if result.contains(PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK) {
             let session_id = resolved_session_id.unwrap_or_else(|| {
                 self.context
                     .session_id
                     .clone()
                     .unwrap_or_else(|| format!("unbound-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")))
             });
-            let report_link = user_workspace_relative_file_link(
-                &format!(".northhing/sessions/{session_id}/research/report.md"),
-                self.context.remote_file_delivery_channel,
-            );
+            let report_link = format!(".northhing/sessions/{session_id}/research/report.md");
             result = result.replace(PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK, &report_link);
         }
 
         if self.context.supports_image_understanding == Some(false) {
             result.push_str(
                 "\n\n# Computer use (text-only primary model)\n\n\
 The configured **primary model does not accept image inputs**. When using **`ComputerUse`** (or **`ControlHub`** with **`domain: \"browser\"`**):\n\
 - **Do not** use **`screenshot`** (desktop) and **avoid** `domain:\"browser\" action:\"screenshot\"` — the JPEG bytes will be unreadable.\n\
 - **ACTION PRIORITY:** 1) Terminal/CLI/system commands (`ExecCommand`, or `ComputerUse` `run_script`; use `WriteStdin`/`ExecControl` for running ExecCommand sessions) 2) Keyboard shortcuts (**`key_chord`**, **`type_text`**) 3) UI control: **`click_element`** (AX) → **`locate`** → **`move_to_text`** (use **`move_to_text_match_index`** when multiple OCR hits listed) → **`mouse_move`** (**`use_screen_coordinates`: true** with coordinates from tool JSON) → **`click`**. For browser work prefer `snapshot` → click by `@e*` ref over screenshots.\n\
 - **Never guess coordinates** — always use precise methods (AX, OCR, system coordinates from tool results, or browser snapshot refs).\n",
diff --git a/src/crates/assembly/core/src/agentic/agents/prompt_builder/tests.rs b/src/crates/assembly/core/src/agentic/agents/prompt_builder/tests.rs
index 663c3b6..c38d321 100644
--- a/src/crates/assembly/core/src/agentic/agents/prompt_builder/tests.rs
+++ b/src/crates/assembly/core/src/agentic/agents/prompt_builder/tests.rs
@@ -209,35 +209,20 @@ async fn deep_research_report_link_defaults_to_workspace_relative_path() {
         .build_prompt_from_template("[View full report]({DEEP_RESEARCH_REPORT_LINK})")
         .await
         .expect("prompt should build");
 
     assert_eq!(
         prompt,
         "[View full report](.northhing/sessions/session-1/research/report.md)"
     );
 }
 
-#[tokio::test]
-async fn deep_research_report_link_uses_computer_scheme_for_remote_delivery() {
-    let context = PromptBuilderContext::new("workspace/root", Some("session-1".to_string()), None)
-        .with_remote_file_delivery_channel(true);
-    let prompt = PromptBuilder::new(context)
-        .build_prompt_from_template("[View full report]({DEEP_RESEARCH_REPORT_LINK})")
-        .await
-        .expect("prompt should build");
-
-    assert_eq!(
-        prompt,
-        "[View full report](computer://.northhing/sessions/session-1/research/report.md)"
-    );
-}
-
 #[test]
 fn workspace_context_renders_related_directories() {
     let context = PromptBuilderContext::new(r"workspace\root", None, None).with_related_paths(vec![
         RelatedPath {
             path: r"legacy-ts\client".to_string(),
             description: Some("Legacy TypeScript implementation".to_string()),
         },
         RelatedPath {
             path: r"monorepo\billing".to_string(),
             description: Some("Billing package".to_string()),
diff --git a/src/crates/assembly/core/src/agentic/coordination/coordinator.rs b/src/crates/assembly/core/src/agentic/coordination/coordinator.rs
index 67da564..900b381 100644
--- a/src/crates/assembly/core/src/agentic/coordination/coordinator.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/coordinator.rs
@@ -18,23 +18,20 @@ use crate::agentic::events::{
     AgenticEvent, DeepReviewQueueState, EventPriority, EventQueue, EventRouter, EventSubscriber,
 };
 use crate::agentic::execution::{ContextCompactionOutcome, ExecutionContext, ExecutionEngine, ExecutionResult};
 use crate::agentic::fork_agent::ForkAgentContextSnapshot;
 use crate::agentic::goal_mode::{
     effective_subagent_timeout_seconds, is_usage_limit_error, maybe_build_continuation_after_turn,
     should_skip_goal_continuation_after_turn, should_skip_goal_for_turn, thread_goal_status_is_resumable,
     user_facing_thread_goal_error, ThreadGoalRuntime, ThreadGoalStore,
 };
 use crate::agentic::image_analysis::ImageContextData;
-use crate::agentic::remote_file_delivery::{
-    needs_computer_links_for_source, remote_file_delivery_reminder, TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY,
-};
 use crate::agentic::round_preempt::DialogRoundInjectionSource;
 use crate::agentic::session::SessionManager;
 use crate::agentic::side_question::build_btw_user_input;
 use crate::agentic::skill_agent_snapshot::{
     diff_skill_agent_snapshot, resolve_skill_agent_snapshot, TurnSkillAgentSnapshot,
 };
 use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
 use crate::agentic::tools::{
     is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
 };
diff --git a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/compaction.rs b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/compaction.rs
index 66b0bfe..925da9c 100644
--- a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/compaction.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/compaction.rs
@@ -23,23 +23,20 @@ use crate::agentic::events::{
     AgenticEvent, DeepReviewQueueState, EventPriority, EventQueue, EventRouter, EventSubscriber,
 };
 use crate::agentic::execution::{ContextCompactionOutcome, ExecutionContext, ExecutionEngine, ExecutionResult};
 use crate::agentic::fork_agent::ForkAgentContextSnapshot;
 use crate::agentic::goal_mode::{
     effective_subagent_timeout_seconds, is_usage_limit_error, maybe_build_continuation_after_turn,
     should_skip_goal_continuation_after_turn, should_skip_goal_for_turn, thread_goal_status_is_resumable,
     user_facing_thread_goal_error, ThreadGoalRuntime, ThreadGoalStore,
 };
 use crate::agentic::image_analysis::ImageContextData;
-use crate::agentic::remote_file_delivery::{
-    needs_computer_links_for_source, remote_file_delivery_reminder, TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY,
-};
 use crate::agentic::round_preempt::DialogRoundInjectionSource;
 use crate::agentic::session::SessionManager;
 use crate::agentic::side_question::build_btw_user_input;
 use crate::agentic::skill_agent_snapshot::{
     diff_skill_agent_snapshot, resolve_skill_agent_snapshot, TurnSkillAgentSnapshot,
 };
 use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
 use crate::agentic::tools::{
     is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
 };
diff --git a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs
index 3ba1842..bbeecb8 100644
--- a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs
@@ -23,23 +23,20 @@ use crate::agentic::events::{
     AgenticEvent, DeepReviewQueueState, EventPriority, EventQueue, EventRouter, EventSubscriber,
 };
 use crate::agentic::execution::{ContextCompactionOutcome, ExecutionContext, ExecutionEngine, ExecutionResult};
 use crate::agentic::fork_agent::ForkAgentContextSnapshot;
 use crate::agentic::goal_mode::{
     effective_subagent_timeout_seconds, is_usage_limit_error, maybe_build_continuation_after_turn,
     should_skip_goal_continuation_after_turn, should_skip_goal_for_turn, thread_goal_status_is_resumable,
     user_facing_thread_goal_error, ThreadGoalRuntime, ThreadGoalStore,
 };
 use crate::agentic::image_analysis::ImageContextData;
-use crate::agentic::remote_file_delivery::{
-    needs_computer_links_for_source, remote_file_delivery_reminder, TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY,
-};
 use crate::agentic::round_preempt::DialogRoundInjectionSource;
 use crate::agentic::session::SessionManager;
 use crate::agentic::side_question::build_btw_user_input;
 use crate::agentic::skill_agent_snapshot::{
     diff_skill_agent_snapshot, resolve_skill_agent_snapshot, TurnSkillAgentSnapshot,
 };
 use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
 use crate::agentic::tools::{
     is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
 };
diff --git a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs
index f76c59d..23e7114 100644
--- a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs
@@ -16,21 +16,20 @@ use super::super::ports::*;
 use super::super::scheduler::*;
 use super::super::scheduler::{
     abort_thread_goal_continuation_for_session, clear_thread_goal_continuation_abort, DialogSubmissionPolicy,
 };
 
 use super::sub_handle_types::TurnContext;
 
 use crate::agentic::core::{ProcessingPhase, SessionState};
 use crate::agentic::events::{AgenticEvent, EventPriority};
 use crate::agentic::execution::ExecutionContext;
-use crate::agentic::remote_file_delivery::needs_computer_links_for_source;
 use crate::agentic::session::SessionManager;
 use crate::agentic::tools::{
     is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
 };
 use crate::util::errors::{NortHingError, NortHingResult};
 use northhing_runtime_ports::DelegationPolicy;
 use std::sync::atomic::{AtomicUsize, Ordering};
 use std::sync::Arc;
 use tokio::time::Duration;
 use tracing::{debug, error, warn};
@@ -143,26 +142,20 @@ impl ConversationCoordinator {
             context_vars.insert("deep_review_run_manifest".to_string(), run_manifest.to_string());
         }
         if user_message_metadata
             .as_ref()
             .and_then(|metadata| metadata.get("acp_transport"))
             .and_then(|value| value.as_bool())
             .unwrap_or(false)
         {
             context_vars.insert("acp_transport".to_string(), "true".to_string());
         }
-        if needs_computer_links_for_source(submission_policy.trigger_source) {
-            context_vars.insert(
-                crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY.to_string(),
-                "true".to_string(),
-            );
-        }
         let session_workspace_path = session_workspace.as_ref().map(|workspace| workspace.root_path_string());
         let session_storage_path = session_workspace
             .as_ref()
             .map(|workspace| workspace.session_storage_path().to_path_buf());
         let runtime_tool_restrictions =
             if is_miniapp_headless_agent_run(user_message_metadata.as_ref(), session.created_by.as_deref()) {
                 miniapp_headless_agent_tool_restrictions()
             } else {
                 ToolRuntimeRestrictions::default()
             };
diff --git a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_state.rs b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_state.rs
index dec22a4..a1f96eb 100644
--- a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_state.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_state.rs
@@ -11,35 +11,33 @@
 //! `use super::super::scheduler::DialogSubmissionPolicy` for the policy type.
 
 use super::super::coordinator::*;
 use super::super::scheduler::*;
 use super::super::scheduler::{
     abort_thread_goal_continuation_for_session, clear_thread_goal_continuation_abort, DialogSubmissionPolicy,
 };
 
 use super::sub_handle_types::TurnContext;
 
-use crate::agentic::core::{InternalReminderKind, Message};
+use crate::agentic::core::Message;
 use crate::agentic::goal_mode::should_skip_goal_for_turn;
-use crate::agentic::remote_file_delivery::{needs_computer_links_for_source, remote_file_delivery_reminder};
 use crate::util::errors::NortHingResult;
 use std::collections::HashMap;
 use tracing::info;
 
 impl ConversationCoordinator {
     pub(super) async fn dispatch_turn(&self, ctx: &mut TurnContext) -> NortHingResult<()> {
         let session_id = ctx.session_id.clone();
         let turn_id = ctx.turn_id.clone();
         let user_input = ctx.user_input.clone();
         let original_user_input = ctx.original_user_input.clone();
         let image_contexts = ctx.image_contexts.clone();
-        let submission_policy = ctx.submission_policy.clone();
         let additional_prepended_messages = ctx.additional_prepended_messages.clone();
         let mut extra_user_message_metadata = ctx.extra_user_message_metadata.clone();
         let session = ctx.session.clone().expect("prepare_turn must set ctx.session first");
         let effective_agent_type = ctx.effective_agent_type.clone();
         let previous_agent_type = ctx.previous_agent_type.clone();
         let turn_index = ctx.turn_index;
         let original_user_input = original_user_input.unwrap_or_else(|| user_input.clone());
         let mut user_message_metadata = extra_user_message_metadata;
         if let Some(imgs) = image_contexts.as_ref().filter(|imgs| !imgs.is_empty()) {
             let image_meta: Vec<serde_json::Value> = imgs
@@ -109,26 +107,20 @@ impl ConversationCoordinator {
                     .filter(|value| !value.is_empty()),
                 user_input,
                 session_workspace.as_ref(),
                 workspace_services.as_ref(),
                 session.config.enable_tools,
                 &skill_agent_context_vars,
             )
             .await?;
         let effective_user_input = wrapped_user_input_payload.content.clone();
         let mut prepended_messages = additional_prepended_messages;
-        if needs_computer_links_for_source(submission_policy.trigger_source) {
-            prepended_messages.push(Message::internal_reminder(
-                InternalReminderKind::RemoteFileDelivery,
-                remote_file_delivery_reminder(),
-            ));
-        }
         prepended_messages.extend(wrapped_user_input_payload.prepended_messages.clone());
         if original_user_input != effective_user_input {
             let mut metadata = Self::ensure_user_message_metadata_object(user_message_metadata.take());
             if let Some(obj) = metadata.as_object_mut() {
                 obj.insert(
                     "original_text".to_string(),
                     serde_json::json!(original_user_input.clone()),
                 );
             }
             user_message_metadata = Some(metadata);
diff --git a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/thread_goal.rs b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/thread_goal.rs
index 6b94e87..dabd981 100644
--- a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/thread_goal.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/thread_goal.rs
@@ -23,23 +23,20 @@ use crate::agentic::events::{
     AgenticEvent, DeepReviewQueueState, EventPriority, EventQueue, EventRouter, EventSubscriber,
 };
 use crate::agentic::execution::{ContextCompactionOutcome, ExecutionContext, ExecutionEngine, ExecutionResult};
 use crate::agentic::fork_agent::ForkAgentContextSnapshot;
 use crate::agentic::goal_mode::{
     effective_subagent_timeout_seconds, is_usage_limit_error, maybe_build_continuation_after_turn,
     should_skip_goal_continuation_after_turn, should_skip_goal_for_turn, thread_goal_status_is_resumable,
     user_facing_thread_goal_error, ThreadGoalRuntime, ThreadGoalStore,
 };
 use crate::agentic::image_analysis::ImageContextData;
-use crate::agentic::remote_file_delivery::{
-    needs_computer_links_for_source, remote_file_delivery_reminder, TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY,
-};
 use crate::agentic::round_preempt::DialogRoundInjectionSource;
 use crate::agentic::session::SessionManager;
 use crate::agentic::side_question::build_btw_user_input;
 use crate::agentic::skill_agent_snapshot::{
     diff_skill_agent_snapshot, resolve_skill_agent_snapshot, TurnSkillAgentSnapshot,
 };
 use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
 use crate::agentic::tools::{
     is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
 };
diff --git a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/workspace.rs b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/workspace.rs
index 8e08c65..efc096f 100644
--- a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/workspace.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/workspace.rs
@@ -23,23 +23,20 @@ use crate::agentic::events::{
     AgenticEvent, DeepReviewQueueState, EventPriority, EventQueue, EventRouter, EventSubscriber,
 };
 use crate::agentic::execution::{ContextCompactionOutcome, ExecutionContext, ExecutionEngine, ExecutionResult};
 use crate::agentic::fork_agent::ForkAgentContextSnapshot;
 use crate::agentic::goal_mode::{
     effective_subagent_timeout_seconds, is_usage_limit_error, maybe_build_continuation_after_turn,
     should_skip_goal_continuation_after_turn, should_skip_goal_for_turn, thread_goal_status_is_resumable,
     user_facing_thread_goal_error, ThreadGoalRuntime, ThreadGoalStore,
 };
 use crate::agentic::image_analysis::ImageContextData;
-use crate::agentic::remote_file_delivery::{
-    needs_computer_links_for_source, remote_file_delivery_reminder, TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY,
-};
 use crate::agentic::round_preempt::DialogRoundInjectionSource;
 use crate::agentic::session::SessionManager;
 use crate::agentic::side_question::build_btw_user_input;
 use crate::agentic::skill_agent_snapshot::{
     diff_skill_agent_snapshot, resolve_skill_agent_snapshot, TurnSkillAgentSnapshot,
 };
 use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
 use crate::agentic::tools::{
     is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
 };
diff --git a/src/crates/assembly/core/src/agentic/execution/ai_message_build.rs b/src/crates/assembly/core/src/agentic/execution/ai_message_build.rs
index f4901ed..df2d263 100644
--- a/src/crates/assembly/core/src/agentic/execution/ai_message_build.rs
+++ b/src/crates/assembly/core/src/agentic/execution/ai_message_build.rs
@@ -18,21 +18,20 @@ use crate::agentic::agents::{
 use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
 use crate::agentic::core::{
     render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
     MessageSemanticKind, RequestReasoningTokenPolicy, Session,
 };
 use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
 use crate::agentic::execution::types::FinishReason;
 use crate::agentic::image_analysis::{
     build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
 };
-use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
 use crate::agentic::round_preempt::RoundInjectionKind;
 use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
 use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
 use crate::agentic::tools::implementations::{SkillTool, TaskTool};
 use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
 use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
 use crate::agentic::WorkspaceBinding;
 use crate::infrastructure::ai::get_global_ai_client_factory;
 use crate::service::config::get_global_config_service;
 use crate::service::config::types::{ModelCapability, ModelCategory};
diff --git a/src/crates/assembly/core/src/agentic/execution/execution_engine.rs b/src/crates/assembly/core/src/agentic/execution/execution_engine.rs
index b84d5c5..01caa4e 100644
--- a/src/crates/assembly/core/src/agentic/execution/execution_engine.rs
+++ b/src/crates/assembly/core/src/agentic/execution/execution_engine.rs
@@ -9,21 +9,20 @@ use crate::agentic::agents::{
 use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
 use crate::agentic::core::{
     render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
     MessageSemanticKind, RequestReasoningTokenPolicy, Session,
 };
 use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
 use crate::agentic::execution::types::FinishReason;
 use crate::agentic::image_analysis::{
     build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
 };
-use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
 use crate::agentic::round_preempt::RoundInjectionKind;
 use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
 use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
 use crate::agentic::tools::implementations::{SkillTool, TaskTool};
 use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
 use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
 use crate::agentic::WorkspaceBinding;
 use crate::infrastructure::ai::get_global_ai_client_factory;
 use crate::service::config::get_global_config_service;
 use crate::service::config::types::{ModelCapability, ModelCategory};
diff --git a/src/crates/assembly/core/src/agentic/execution/health_snapshot.rs b/src/crates/assembly/core/src/agentic/execution/health_snapshot.rs
index 547cf05..3191099 100644
--- a/src/crates/assembly/core/src/agentic/execution/health_snapshot.rs
+++ b/src/crates/assembly/core/src/agentic/execution/health_snapshot.rs
@@ -17,21 +17,20 @@ use crate::agentic::agents::{
 use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
 use crate::agentic::core::{
     render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
     MessageSemanticKind, RequestReasoningTokenPolicy, Session,
 };
 use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
 use crate::agentic::execution::types::FinishReason;
 use crate::agentic::image_analysis::{
     build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
 };
-use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
 use crate::agentic::round_preempt::RoundInjectionKind;
 use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
 use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
 use crate::agentic::tools::implementations::{SkillTool, TaskTool};
 use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
 use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
 use crate::agentic::WorkspaceBinding;
 use crate::infrastructure::ai::get_global_ai_client_factory;
 use crate::service::config::get_global_config_service;
 use crate::service::config::types::{ModelCapability, ModelCategory};
diff --git a/src/crates/assembly/core/src/agentic/execution/loop_detection.rs b/src/crates/assembly/core/src/agentic/execution/loop_detection.rs
index 3a79d3c..f23ad0a 100644
--- a/src/crates/assembly/core/src/agentic/execution/loop_detection.rs
+++ b/src/crates/assembly/core/src/agentic/execution/loop_detection.rs
@@ -18,21 +18,20 @@ use crate::agentic::agents::{
 use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
 use crate::agentic::core::{
     render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
     MessageSemanticKind, RequestReasoningTokenPolicy, Session,
 };
 use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
 use crate::agentic::execution::types::FinishReason;
 use crate::agentic::image_analysis::{
     build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
 };
-use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
 use crate::agentic::round_preempt::RoundInjectionKind;
 use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
 use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
 use crate::agentic::tools::implementations::{SkillTool, TaskTool};
 use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
 use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
 use crate::agentic::WorkspaceBinding;
 use crate::infrastructure::ai::get_global_ai_client_factory;
 use crate::service::config::get_global_config_service;
 use crate::service::config::types::{ModelCapability, ModelCategory};
diff --git a/src/crates/assembly/core/src/agentic/execution/multimodal.rs b/src/crates/assembly/core/src/agentic/execution/multimodal.rs
index dfeabcf..abe3e67 100644
--- a/src/crates/assembly/core/src/agentic/execution/multimodal.rs
+++ b/src/crates/assembly/core/src/agentic/execution/multimodal.rs
@@ -18,21 +18,20 @@ use crate::agentic::agents::{
 use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
 use crate::agentic::core::{
     render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
     MessageSemanticKind, RequestReasoningTokenPolicy, Session,
 };
 use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
 use crate::agentic::execution::types::FinishReason;
 use crate::agentic::image_analysis::{
     build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
 };
-use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
 use crate::agentic::round_preempt::RoundInjectionKind;
 use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
 use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
 use crate::agentic::tools::implementations::{SkillTool, TaskTool};
 use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
 use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
 use crate::agentic::WorkspaceBinding;
 use crate::infrastructure::ai::get_global_ai_client_factory;
 use crate::service::config::get_global_config_service;
 use crate::service::config::types::{ModelCapability, ModelCategory};
diff --git a/src/crates/assembly/core/src/agentic/execution/token_pressure.rs b/src/crates/assembly/core/src/agentic/execution/token_pressure.rs
index cb23048..893c6fe 100644
--- a/src/crates/assembly/core/src/agentic/execution/token_pressure.rs
+++ b/src/crates/assembly/core/src/agentic/execution/token_pressure.rs
@@ -18,21 +18,20 @@ use crate::agentic::agents::{
 use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
 use crate::agentic::core::{
     render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
     MessageSemanticKind, RequestReasoningTokenPolicy, Session,
 };
 use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
 use crate::agentic::execution::types::FinishReason;
 use crate::agentic::image_analysis::{
     build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
 };
-use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
 use crate::agentic::round_preempt::RoundInjectionKind;
 use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
 use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
 use crate::agentic::tools::implementations::{SkillTool, TaskTool};
 use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
 use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
 use crate::agentic::WorkspaceBinding;
 use crate::infrastructure::ai::get_global_ai_client_factory;
 use crate::service::config::get_global_config_service;
 use crate::service::config::types::{ModelCapability, ModelCategory};
diff --git a/src/crates/assembly/core/src/agentic/execution/turn_finalize.rs b/src/crates/assembly/core/src/agentic/execution/turn_finalize.rs
index 50baf41..be42a93 100644
--- a/src/crates/assembly/core/src/agentic/execution/turn_finalize.rs
+++ b/src/crates/assembly/core/src/agentic/execution/turn_finalize.rs
@@ -18,21 +18,20 @@ use crate::agentic::agents::{
 use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
 use crate::agentic::core::{
     render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
     MessageSemanticKind, RequestReasoningTokenPolicy, Session,
 };
 use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
 use crate::agentic::execution::types::FinishReason;
 use crate::agentic::image_analysis::{
     build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
 };
-use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
 use crate::agentic::round_preempt::RoundInjectionKind;
 use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
 use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
 use crate::agentic::tools::implementations::{SkillTool, TaskTool};
 use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
 use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
 use crate::agentic::WorkspaceBinding;
 use crate::infrastructure::ai::get_global_ai_client_factory;
 use crate::service::config::get_global_config_service;
 use crate::service::config::types::{ModelCapability, ModelCategory};
diff --git a/src/crates/assembly/core/src/agentic/execution/turn_init.rs b/src/crates/assembly/core/src/agentic/execution/turn_init.rs
index ff9f207..ae2e2eb 100644
--- a/src/crates/assembly/core/src/agentic/execution/turn_init.rs
+++ b/src/crates/assembly/core/src/agentic/execution/turn_init.rs
@@ -18,21 +18,20 @@ use crate::agentic::agents::{
 use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
 use crate::agentic::core::{
     render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
     MessageSemanticKind, RequestReasoningTokenPolicy, Session,
 };
 use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
 use crate::agentic::execution::types::FinishReason;
 use crate::agentic::image_analysis::{
     build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
 };
-use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
 use crate::agentic::round_preempt::RoundInjectionKind;
 use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
 use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
 use crate::agentic::tools::implementations::{SkillTool, TaskTool};
 use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
 use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
 use crate::agentic::WorkspaceBinding;
 use crate::infrastructure::ai::get_global_ai_client_factory;
 use crate::service::config::get_global_config_service;
 use crate::service::config::types::{ModelCapability, ModelCategory};
diff --git a/src/crates/assembly/core/src/agentic/execution/turn_lifecycle.rs b/src/crates/assembly/core/src/agentic/execution/turn_lifecycle.rs
index 522acae..b178c5e 100644
--- a/src/crates/assembly/core/src/agentic/execution/turn_lifecycle.rs
+++ b/src/crates/assembly/core/src/agentic/execution/turn_lifecycle.rs
@@ -18,21 +18,20 @@ use crate::agentic::agents::{
 use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
 use crate::agentic::core::{
     render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
     MessageSemanticKind, RequestReasoningTokenPolicy, Session,
 };
 use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
 use crate::agentic::execution::types::FinishReason;
 use crate::agentic::image_analysis::{
     build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
 };
-use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
 use crate::agentic::round_preempt::RoundInjectionKind;
 use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
 use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
 use crate::agentic::tools::implementations::{SkillTool, TaskTool};
 use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
 use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
 use crate::agentic::WorkspaceBinding;
 use crate::infrastructure::ai::get_global_ai_client_factory;
 use crate::service::config::get_global_config_service;
 use crate::service::config::types::{ModelCapability, ModelCategory};
@@ -94,37 +93,31 @@ impl ExecutionEngine {
     }
 
     pub(super) async fn build_prompt_context(
         context: &ExecutionContext,
         model_name: &str,
         supports_image_understanding: bool,
         tool_listing_sections: ToolListingSections,
         runtime_context_needs: RuntimeContextNeeds,
     ) -> Option<PromptBuilderContext> {
         let workspace = context.workspace.as_ref()?;
-        let remote_file_delivery_channel = context
-            .context
-            .get(TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY)
-            .and_then(|value| value.parse::<bool>().ok())
-            .unwrap_or(false);
 
         let mut prompt_context = build_prompt_context_for_workspace(
             workspace,
             workspace.workspace_id.as_deref(),
             &context.session_id,
             Some(model_name.to_string()),
             Some(supports_image_understanding),
             tool_listing_sections,
             runtime_context_needs,
         )
-        .await
-        .map(|ctx| ctx.with_remote_file_delivery_channel(remote_file_delivery_channel))?;
+        .await?;
 
         // Look up model config and fill context_window / max_output_tokens.
         if let Ok(config_service) = get_global_config_service().await {
             if let Ok(models) = config_service.get_ai_models().await {
                 if let Some(model_config) = models.iter().find(|m| m.name == model_name) {
                     prompt_context.context_window = model_config.context_window;
                     prompt_context.max_output_tokens = model_config.max_tokens;
                 }
             }
         }
diff --git a/src/crates/assembly/core/src/agentic/execution/turn_main_loop.rs b/src/crates/assembly/core/src/agentic/execution/turn_main_loop.rs
index e2a50d0..88d8656 100644
--- a/src/crates/assembly/core/src/agentic/execution/turn_main_loop.rs
+++ b/src/crates/assembly/core/src/agentic/execution/turn_main_loop.rs
@@ -18,21 +18,20 @@ use crate::agentic::agents::{
 use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
 use crate::agentic::core::{
     render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
     MessageSemanticKind, RequestReasoningTokenPolicy, Session,
 };
 use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
 use crate::agentic::execution::types::FinishReason;
 use crate::agentic::image_analysis::{
     build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
 };
-use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
 use crate::agentic::round_preempt::RoundInjectionKind;
 use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
 use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
 use crate::agentic::tools::implementations::{SkillTool, TaskTool};
 use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
 use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
 use crate::agentic::WorkspaceBinding;
 use crate::infrastructure::ai::get_global_ai_client_factory;
 use crate::service::config::get_global_config_service;
 use crate::service::config::types::{ModelCapability, ModelCategory};
diff --git a/src/crates/assembly/core/src/agentic/execution/turn_tick.rs b/src/crates/assembly/core/src/agentic/execution/turn_tick.rs
index 7e86875..f9d8255 100644
--- a/src/crates/assembly/core/src/agentic/execution/turn_tick.rs
+++ b/src/crates/assembly/core/src/agentic/execution/turn_tick.rs
@@ -18,21 +18,20 @@ use crate::agentic::agents::{
 use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
 use crate::agentic::core::{
     render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
     MessageSemanticKind, RequestReasoningTokenPolicy, Session,
 };
 use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
 use crate::agentic::execution::types::FinishReason;
 use crate::agentic::image_analysis::{
     build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
 };
-use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
 use crate::agentic::round_preempt::RoundInjectionKind;
 use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
 use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
 use crate::agentic::tools::implementations::{SkillTool, TaskTool};
 use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
 use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
 use crate::agentic::WorkspaceBinding;
 use crate::infrastructure::ai::get_global_ai_client_factory;
 use crate::service::config::get_global_config_service;
 use crate::service::config::types::{ModelCapability, ModelCategory};
diff --git a/src/crates/assembly/core/src/agentic/mod.rs b/src/crates/assembly/core/src/agentic/mod.rs
index 358b4b5..d9ba4fb 100644
--- a/src/crates/assembly/core/src/agentic/mod.rs
+++ b/src/crates/assembly/core/src/agentic/mod.rs
@@ -21,21 +21,20 @@ pub mod tools;
 // Coordination module
 pub mod context_profile;
 pub mod coordination;
 pub mod deep_review;
 pub mod deep_review_policy;
 pub(crate) mod subagent_runtime;
 
 // Shared-context fork-agent execution module
 pub mod fork_agent;
 
-pub(crate) mod remote_file_delivery;
 /// Round-boundary injection support for steering/background updates
 pub mod round_preempt;
 
 // Image analysis module
 pub mod image_analysis;
 
 // Ephemeral side-question module (used by desktop /btw overlay)
 pub mod side_question;
 
 // Session goal mode (/goal command)
diff --git a/src/crates/assembly/core/src/agentic/remote_file_delivery.rs b/src/crates/assembly/core/src/agentic/remote_file_delivery.rs
deleted file mode 100644
index 35b97db..0000000
--- a/src/crates/assembly/core/src/agentic/remote_file_delivery.rs
+++ /dev/null
@@ -1,69 +0,0 @@
-use northhing_runtime_ports::DialogTriggerSource;
-use std::path::Path;
-
-pub const TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY: &str = "remote_file_delivery_channel";
-
-pub fn needs_computer_links_for_source(source: DialogTriggerSource) -> bool {
-    matches!(source, DialogTriggerSource::RemoteRelay | DialogTriggerSource::Bot)
-}
-
-pub fn remote_file_delivery_reminder() -> &'static str {
-    r#"The user is messaging through a remote mobile or bot channel.
-
-When referencing a plan, report, presentation, spreadsheet, document, image, or archive, add `computer://` before the file path so the user can click to download it, for example [report.md](computer://artifacts/report.md)."#
-}
-
-pub fn workspace_relative_link(path: &Path, workspace_root: Option<&Path>) -> Option<String> {
-    workspace_root
-        .and_then(|root| path.strip_prefix(root).ok())
-        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
-}
-
-pub fn computer_link(path: &Path, workspace_root: Option<&Path>) -> String {
-    workspace_relative_link(path, workspace_root)
-        .map(|rel| format!("computer://{rel}"))
-        .unwrap_or_else(|| format!("computer://{}", path.to_string_lossy().replace('\\', "/")))
-}
-
-pub fn user_file_link(path: &Path, workspace_root: Option<&Path>, use_computer_link: bool) -> String {
-    if use_computer_link {
-        computer_link(path, workspace_root)
-    } else {
-        workspace_relative_link(path, workspace_root).unwrap_or_else(|| path.to_string_lossy().replace('\\', "/"))
-    }
-}
-
-pub fn user_workspace_relative_file_link(relative_path: &str, use_computer_link: bool) -> String {
-    let normalized = relative_path.replace('\\', "/");
-    if use_computer_link {
-        format!("computer://{normalized}")
-    } else {
-        normalized
-    }
-}
-
-#[cfg(test)]
-mod tests {
-    use super::{user_file_link, user_workspace_relative_file_link, workspace_relative_link};
-    use std::path::Path;
-
-    #[test]
-    fn desktop_links_prefer_workspace_relative_paths() {
-        let root = Path::new("/repo");
-        let report = Path::new("/repo/artifacts/report.md");
-
-        assert_eq!(
-            workspace_relative_link(report, Some(root)).as_deref(),
-            Some("artifacts/report.md")
-        );
-        assert_eq!(user_file_link(report, Some(root), false), "artifacts/report.md");
-    }
-
-    #[test]
-    fn remote_delivery_links_use_computer_scheme() {
-        assert_eq!(
-            user_workspace_relative_file_link(r".northhing\sessions\s1\research\report.md", true),
-            "computer://.northhing/sessions/s1/research/report.md"
-        );
-    }
-}
diff --git a/src/crates/assembly/core/src/agentic/tools/implementations/create_plan_tool.rs b/src/crates/assembly/core/src/agentic/tools/implementations/create_plan_tool.rs
index 8887e2b..8acebc8 100644
--- a/src/crates/assembly/core/src/agentic/tools/implementations/create_plan_tool.rs
+++ b/src/crates/assembly/core/src/agentic/tools/implementations/create_plan_tool.rs
@@ -1,17 +1,14 @@
 //! CreatePlan tool implementation
 //!
 //! Used to create and store plan files during the planning phase
 
-use crate::agentic::remote_file_delivery::{
-    computer_link as build_computer_link, user_file_link, TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY,
-};
 use crate::agentic::tools::framework::{Tool, ToolExposure, ToolResult, ToolUseContext};
 use crate::util::errors::{NortHingError, NortHingResult};
 use async_trait::async_trait;
 use serde::Serialize;
 use serde_json::{json, Value};
 use tokio::fs;
 
 /// YAML frontmatter structure for Plan files
 #[derive(Serialize)]
 struct PlanFrontmatter {
@@ -207,59 +204,59 @@ Additional guidelines:
                             obj.insert("status".to_string(), json!("pending"));
                         }
                     }
                     todo_obj
                 })
                 .collect()
         } else {
             vec![]
         };
 
-        let use_computer_link = context
-            .custom_data
-            .get(TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY)
-            .and_then(|value| value.as_bool())
-            .unwrap_or(false);
         let plan_path = std::path::Path::new(&plan_file_path_str);
-        let computer_link = build_computer_link(plan_path, context.workspace_root());
-        let user_link = user_file_link(plan_path, context.workspace_root(), use_computer_link);
+        let user_link = workspace_relative_user_link(plan_path, context.workspace_root());
 
         let plan_reference = context.build_runtime_artifact_reference(&format!("plans/{}", plan_file_name))?;
 
         let result_for_assistant = format!(
             "Plan file created at: {}
 Clickable link for user: [{}]({})
 Your next reply MUST show the clickable link and then end the conversation turn. Do not continue with more planning details or additional questions.",
             plan_reference,
             plan_file_name,
             user_link,
         );
 
         let result = json!({
             "success": true,
             "plan_file_path": plan_reference,
-            "computer_link": computer_link.clone(),
-            "user_link": user_link.clone(),
+            "user_link": user_link,
             "plan_file_name": plan_file_name,
             "name": name,
             "overview": overview,
             "todos": processed_todos
         });
 
         Ok(vec![ToolResult::Result {
             data: result,
             result_for_assistant: Some(result_for_assistant),
             image_attachments: None,
         }])
     }
 }
 
+fn workspace_relative_user_link(path: &std::path::Path, workspace_root: Option<&std::path::Path>) -> String {
+    workspace_root
+        .and_then(|root| path.strip_prefix(root).ok())
+        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
+        .unwrap_or_else(|| path.to_string_lossy().replace('\\', "/"))
+}
+
 /// Generate plan file content
 fn generate_plan_file_content(name: &str, overview: &str, plan: &str, todos: Option<&Vec<Value>>) -> String {
     // Convert todos
     let todos_vec: Vec<TodoItem> = todos
         .map(|arr| {
             arr.iter()
                 .filter_map(|todo| {
                     let id = todo.get("id").and_then(|v| v.as_str())?;
                     let content = todo.get("content").and_then(|v| v.as_str())?;
                     let dependencies = todo
diff --git a/src/crates/assembly/core/src/agentic/tools/tool_context_runtime/context_init.rs b/src/crates/assembly/core/src/agentic/tools/tool_context_runtime/context_init.rs
index 709069c..abee0a4 100644
--- a/src/crates/assembly/core/src/agentic/tools/tool_context_runtime/context_init.rs
+++ b/src/crates/assembly/core/src/agentic/tools/tool_context_runtime/context_init.rs
@@ -1,12 +1,11 @@
 use crate::agentic::deep_review::tool_context;
-use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
 use crate::agentic::tools::pipeline::{ToolExecutionContext, ToolTask};
 use crate::agentic::tools::ToolRuntimeRestrictions;
 use crate::agentic::workspace::WorkspaceServices;
 use crate::agentic::WorkspaceBinding;
 use northhing_runtime_ports::{DelegationPolicy, ToolRuntimeHandles};
 use serde_json::Value;
 use std::collections::HashMap;
 use std::path::Path;
 use std::sync::Arc;
 use tokio_util::sync::CancellationToken;
@@ -204,28 +203,20 @@ fn build_tool_context_custom_data(context: &ToolExecutionContext) -> HashMap<Str
                 "primary_model_supports_image_understanding".to_string(),
                 serde_json::json!(flag),
             );
         }
     }
     if let Some(acp_transport) = context.context_vars.get("acp_transport") {
         if let Ok(flag) = acp_transport.parse::<bool>() {
             map.insert("acp_transport".to_string(), serde_json::json!(flag));
         }
     }
-    if let Some(remote_file_delivery) = context.context_vars.get(TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY) {
-        if let Ok(flag) = remote_file_delivery.parse::<bool>() {
-            map.insert(
-                TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY.to_string(),
-                serde_json::json!(flag),
-            );
-        }
-    }
 
     let deep_review_parent_context =
         context
             .subagent_parent_info
             .as_ref()
             .map(|parent_info| tool_context::DeepReviewToolParentContext {
                 tool_call_id: parent_info.tool_call_id.as_str(),
                 session_id: parent_info.session_id.as_str(),
                 dialog_turn_id: parent_info.dialog_turn_id.as_str(),
             });
