diff --git a/src/crates/assembly/core/src/agentic/agents/mod.rs b/src/crates/assembly/core/src/agentic/agents/mod.rs
index 7940e6f..f4074ad 100644
--- a/src/crates/assembly/core/src/agentic/agents/mod.rs
+++ b/src/crates/assembly/core/src/agentic/agents/mod.rs
@@ -93,7 +93,6 @@ pub fn shared_coding_mode_tools() -> Vec<String> {
         "Git".to_string(),
         "Log".to_string(),
         "ControlHub".to_string(),
-        "InitMiniApp".to_string(),
     ]
 }
 
diff --git a/src/crates/assembly/core/src/agentic/coordination/coordinator.rs b/src/crates/assembly/core/src/agentic/coordination/coordinator.rs
index 900b381..2ab8475 100644
--- a/src/crates/assembly/core/src/agentic/coordination/coordinator.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/coordinator.rs
@@ -32,9 +32,7 @@ use crate::agentic::skill_agent_snapshot::{
     diff_skill_agent_snapshot, resolve_skill_agent_snapshot, TurnSkillAgentSnapshot,
 };
 use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
-use crate::agentic::tools::{
-    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
-};
+use crate::agentic::tools::ToolRuntimeRestrictions;
 use crate::agentic::workspace::WorkspaceServices;
 use crate::agentic::WorkspaceBinding;
 use crate::service::bootstrap::{ensure_workspace_persona_files_for_prompt, is_workspace_bootstrap_pending};
diff --git a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/compaction.rs b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/compaction.rs
index 925da9c..aa4f210 100644
--- a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/compaction.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/compaction.rs
@@ -37,9 +37,7 @@ use crate::agentic::skill_agent_snapshot::{
     diff_skill_agent_snapshot, resolve_skill_agent_snapshot, TurnSkillAgentSnapshot,
 };
 use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
-use crate::agentic::tools::{
-    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
-};
+use crate::agentic::tools::ToolRuntimeRestrictions;
 use crate::agentic::workspace::WorkspaceServices;
 use crate::agentic::WorkspaceBinding;
 use crate::service::bootstrap::{ensure_workspace_persona_files_for_prompt, is_workspace_bootstrap_pending};
diff --git a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs
index bbeecb8..9d54201 100644
--- a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs
@@ -37,9 +37,7 @@ use crate::agentic::skill_agent_snapshot::{
     diff_skill_agent_snapshot, resolve_skill_agent_snapshot, TurnSkillAgentSnapshot,
 };
 use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
-use crate::agentic::tools::{
-    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
-};
+use crate::agentic::tools::ToolRuntimeRestrictions;
 use crate::agentic::workspace::WorkspaceServices;
 use crate::agentic::WorkspaceBinding;
 use crate::service::bootstrap::{ensure_workspace_persona_files_for_prompt, is_workspace_bootstrap_pending};
diff --git a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs
index 23e7114..c53fba4 100644
--- a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs
@@ -24,9 +24,7 @@ use crate::agentic::core::{ProcessingPhase, SessionState};
 use crate::agentic::events::{AgenticEvent, EventPriority};
 use crate::agentic::execution::ExecutionContext;
 use crate::agentic::session::SessionManager;
-use crate::agentic::tools::{
-    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
-};
+use crate::agentic::tools::ToolRuntimeRestrictions;
 use crate::util::errors::{NortHingError, NortHingResult};
 use northhing_runtime_ports::DelegationPolicy;
 use std::sync::atomic::{AtomicUsize, Ordering};
@@ -153,12 +151,7 @@ impl ConversationCoordinator {
         let session_storage_path = session_workspace
             .as_ref()
             .map(|workspace| workspace.session_storage_path().to_path_buf());
-        let runtime_tool_restrictions =
-            if is_miniapp_headless_agent_run(user_message_metadata.as_ref(), session.created_by.as_deref()) {
-                miniapp_headless_agent_tool_restrictions()
-            } else {
-                ToolRuntimeRestrictions::default()
-            };
+        let runtime_tool_restrictions = ToolRuntimeRestrictions::default();
         let execution_context = ExecutionContext {
             session_id: session_id.clone(),
             dialog_turn_id: turn_id.clone(),
diff --git a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/thread_goal.rs b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/thread_goal.rs
index dabd981..519bb24 100644
--- a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/thread_goal.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/thread_goal.rs
@@ -37,9 +37,7 @@ use crate::agentic::skill_agent_snapshot::{
     diff_skill_agent_snapshot, resolve_skill_agent_snapshot, TurnSkillAgentSnapshot,
 };
 use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
-use crate::agentic::tools::{
-    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
-};
+use crate::agentic::tools::ToolRuntimeRestrictions;
 use crate::agentic::workspace::WorkspaceServices;
 use crate::agentic::WorkspaceBinding;
 use crate::service::bootstrap::{ensure_workspace_persona_files_for_prompt, is_workspace_bootstrap_pending};
diff --git a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/workspace.rs b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/workspace.rs
index efc096f..c3c068b 100644
--- a/src/crates/assembly/core/src/agentic/coordination/dialog_turn/workspace.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/dialog_turn/workspace.rs
@@ -37,9 +37,7 @@ use crate::agentic::skill_agent_snapshot::{
     diff_skill_agent_snapshot, resolve_skill_agent_snapshot, TurnSkillAgentSnapshot,
 };
 use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
-use crate::agentic::tools::{
-    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
-};
+use crate::agentic::tools::ToolRuntimeRestrictions;
 use crate::agentic::workspace::WorkspaceServices;
 use crate::agentic::WorkspaceBinding;
 use crate::service::bootstrap::{ensure_workspace_persona_files_for_prompt, is_workspace_bootstrap_pending};
diff --git a/src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_dispatch.rs b/src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_dispatch.rs
index bc4c53a..2295737 100644
--- a/src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_dispatch.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_dispatch.rs
@@ -13,9 +13,7 @@ use crate::agentic::goal_mode::{
     user_facing_thread_goal_error, ThreadGoalRuntime, ThreadGoalStore,
 };
 use crate::agentic::tools::pipeline::SubagentParentInfo;
-use crate::agentic::tools::{
-    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
-};
+use crate::agentic::tools::ToolRuntimeRestrictions;
 use crate::service_agent_runtime::CoreServiceAgentRuntime;
 use crate::util::errors::{NortHingError, NortHingResult};
 use northhing_agent_dispatch::{ActorRuntime, USE_LIGHTWEIGHT_ACTOR};
diff --git a/src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_types.rs b/src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_types.rs
index b449968..e0ae500 100644
--- a/src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_types.rs
+++ b/src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_types.rs
@@ -4,9 +4,7 @@
 use super::super::coordinator::SubagentResult;
 use crate::agentic::skill_agent_snapshot::TurnSkillAgentSnapshot;
 use crate::agentic::tools::pipeline::SubagentParentInfo;
-use crate::agentic::tools::{
-    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
-};
+use crate::agentic::tools::ToolRuntimeRestrictions;
 use crate::service::session::{SessionRelationship, SessionRelationshipKind};
 use crate::util::errors::NortHingError;
 use northhing_runtime_ports::DelegationPolicy;
diff --git a/src/crates/assembly/core/src/agentic/tools/agent-tool-exposure.md b/src/crates/assembly/core/src/agentic/tools/agent-tool-exposure.md
index 7fc33d1..57ccd22 100644
--- a/src/crates/assembly/core/src/agentic/tools/agent-tool-exposure.md
+++ b/src/crates/assembly/core/src/agentic/tools/agent-tool-exposure.md
@@ -41,7 +41,6 @@ Notes:
 | `GetMCPPrompt` | Collapsed | None | - |
 | `GenerativeUI` | Collapsed | None | - |
 | `Git` | Collapsed | `ReviewFixer`, `ReviewBusinessLogic`, `ReviewPerformance`, `ReviewSecurity`, `ReviewArchitecture`, `ReviewFrontend`, `ReviewJudge` | Expanded |
-| `InitMiniApp` | Collapsed | None | - |
 | `ControlHub` | Collapsed | `ComputerUse` | Expanded |
 | `ComputerUse` | Collapsed | `ComputerUse` | Expanded |
 | `Playbook` | Collapsed | None | - |
diff --git a/src/crates/assembly/core/src/agentic/tools/implementations/miniapp_init_tool.rs b/src/crates/assembly/core/src/agentic/tools/implementations/miniapp_init_tool.rs
deleted file mode 100644
index 217a029..0000000
--- a/src/crates/assembly/core/src/agentic/tools/implementations/miniapp_init_tool.rs
+++ /dev/null
@@ -1,221 +0,0 @@
-//! InitMiniApp tool — create a new MiniApp skeleton; AI then uses generic file tools to edit.
-
-use crate::agentic::tools::framework::{Tool, ToolResult, ToolUseContext};
-use crate::infrastructure::events::{emit_global_event, BackendEvent};
-use crate::miniapp::try_get_global_miniapp_manager;
-use crate::miniapp::types::{FsPermissions, MiniAppPermissions, MiniAppSource, NetPermissions, ShellPermissions};
-use crate::util::errors::{NortHingError, NortHingResult};
-use async_trait::async_trait;
-use serde_json::{json, Value};
-
-const SKELETON_HTML: &str = r#"<!DOCTYPE html>
-<html data-theme-type="dark">
-<head><meta charset="utf-8"></head>
-<body>
-  <div id="app"></div>
-</body>
-</html>"#;
-
-const SKELETON_UI_JS: &str = r#"// ESM module — use import, not require. Example:
-// import React from 'react';
-// const files = await app.fs.readdir('.');
-// document.getElementById('app').textContent = JSON.stringify(files, null, 2);
-"#;
-
-const SKELETON_WORKER_JS: &str = r#"// Node.js Worker — export methods callable via app.call('methodName', params).
-// module.exports = {
-//   async 'myMethod'(params) { return { result: 'ok' }; },
-// };
-"#;
-
-const SKELETON_CSS: &str = r#"/* MiniApp skeleton — uses host theme via --northhing-* variables */
-* { box-sizing: border-box; margin: 0; padding: 0; }
-body {
-  font-family: var(--northhing-font-sans, -apple-system, BlinkMacSystemFont, 'PingFang SC', 'Hiragino Sans GB', 'Segoe UI', 'Microsoft YaHei UI', 'Microsoft YaHei', 'Helvetica Neue', Helvetica, Arial, sans-serif);
-  font-size: 13px;
-  color: var(--northhing-text, #e8e8e8);
-  background: var(--northhing-bg, #121214);
-  min-height: 100vh;
-}
-#app { min-height: 100vh; }
-"#;
-
-pub struct InitMiniAppTool;
-
-impl InitMiniAppTool {
-    pub fn new() -> Self {
-        Self
-    }
-}
-
-impl Default for InitMiniAppTool {
-    fn default() -> Self {
-        Self::new()
-    }
-}
-
-#[cfg(test)]
-mod tests {
-    use super::InitMiniAppTool;
-    use crate::agentic::tools::framework::{Tool, ToolExposure};
-
-    #[test]
-    fn init_miniapp_stays_expanded_for_assistant_creation() {
-        let tool = InitMiniAppTool::new();
-        assert_eq!(tool.default_exposure(), ToolExposure::Expanded);
-    }
-}
-
-#[async_trait]
-impl Tool for InitMiniAppTool {
-    fn name(&self) -> &str {
-        "InitMiniApp"
-    }
-
-    async fn description(&self) -> NortHingResult<String> {
-        Ok(r#"Create a new MiniApp skeleton in the Toolbox. After creation, use Read/Write/Edit file tools to modify the source files directly.
-
-Input: name, description, icon, category. The tool creates the app directory and skeleton files:
-- manifest (meta.json), source/index.html, source/style.css, source/ui.js, source/worker.js,
-  package.json, storage.json.
-
-Returns app_id and the app root directory. Use the root directory and file names above with Read/Write/Edit to implement the app. The MiniApp uses window.app (app.fs, app.call, app.dialog, etc.) — see miniapp-dev skill for API reference."#
-            .to_string())
-    }
-
-    fn short_description(&self) -> String {
-        "Create a new MiniApp skeleton in the Toolbox.".to_string()
-    }
-
-    fn input_schema(&self) -> Value {
-        json!({
-            "type": "object",
-            "additionalProperties": false,
-            "required": ["name"],
-            "properties": {
-                "name": {
-                    "type": "string",
-                    "description": "Short app name (e.g. 'Image Compressor', 'Markdown Viewer')"
-                },
-                "description": {
-                    "type": "string",
-                    "description": "One-sentence description. Default empty."
-                },
-                "icon": {
-                    "type": "string",
-                    "description": "Emoji or icon identifier. Default '📦'."
-                },
-                "category": {
-                    "type": "string",
-                    "description": "Category: utility, media, dev, productivity. Default 'utility'."
-                }
-            }
-        })
-    }
-
-    fn is_readonly(&self) -> bool {
-        false
-    }
-
-    fn needs_permissions(&self, _input: Option<&Value>) -> bool {
-        false
-    }
-
-    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
-        let manager = try_get_global_miniapp_manager()
-            .ok_or_else(|| NortHingError::tool("MiniAppManager not initialized".to_string()))?;
-
-        let name = input
-            .get("name")
-            .and_then(|v| v.as_str())
-            .ok_or_else(|| NortHingError::validation("Missing required field: name"))?
-            .to_string();
-        let description = input
-            .get("description")
-            .and_then(|v| v.as_str())
-            .unwrap_or("")
-            .to_string();
-        let icon = input.get("icon").and_then(|v| v.as_str()).unwrap_or("📦").to_string();
-        let category = input
-            .get("category")
-            .and_then(|v| v.as_str())
-            .unwrap_or("utility")
-            .to_string();
-
-        let source = MiniAppSource {
-            html: SKELETON_HTML.to_string(),
-            css: SKELETON_CSS.to_string(),
-            ui_js: SKELETON_UI_JS.to_string(),
-            esm_dependencies: Vec::new(),
-            worker_js: SKELETON_WORKER_JS.to_string(),
-            npm_dependencies: Vec::new(),
-        };
-
-        let permissions = MiniAppPermissions {
-            fs: Some(FsPermissions {
-                read: Some(vec!["{appdata}".to_string(), "{workspace}".to_string()]),
-                write: Some(vec!["{appdata}".to_string()]),
-            }),
-            shell: Some(ShellPermissions {
-                allow: Some(Vec::new()),
-            }),
-            net: Some(NetPermissions {
-                allow: Some(vec!["*".to_string()]),
-            }),
-            node: None,
-            ai: None,
-            ..Default::default()
-        };
-
-        let app = manager
-            .create(
-                name.clone(),
-                description,
-                icon,
-                category,
-                Vec::new(),
-                source,
-                permissions,
-                None,
-                context.workspace_root(),
-            )
-            .await
-            .map_err(|e| NortHingError::tool(format!("Failed to create MiniApp: {}", e)))?;
-
-        let path_manager = manager.path_manager();
-        let app_dir = path_manager.miniapp_dir(&app.id);
-        let app_dir_str = app_dir.to_string_lossy().to_string();
-        let source_dir = app_dir.join("source");
-
-        let files = json!({
-            "manifest": app_dir.join("meta.json").to_string_lossy(),
-            "ui": source_dir.join("ui.js").to_string_lossy(),
-            "worker": source_dir.join("worker.js").to_string_lossy(),
-            "style": source_dir.join("style.css").to_string_lossy(),
-            "html": source_dir.join("index.html").to_string_lossy(),
-            "package": app_dir.join("package.json").to_string_lossy(),
-            "storage": app_dir.join("storage.json").to_string_lossy(),
-        });
-
-        let _ = emit_global_event(BackendEvent::Custom {
-            event_name: "miniapp-created".to_string(),
-            payload: json!({ "id": app.id, "name": app.name }),
-        })
-        .await;
-
-        let result_text = format!(
-            "MiniApp '{}' skeleton created. app_id: {}. Root directory: {}. Use Read/Write/Edit tools with files under this root, then open in Toolbox to run.",
-            app.name, app.id, app_dir_str
-        );
-
-        Ok(vec![ToolResult::Result {
-            data: json!({
-                "app_id": app.id,
-                "path": app_dir_str,
-                "files": files,
-            }),
-            result_for_assistant: Some(result_text),
-            image_attachments: None,
-        }])
-    }
-}
diff --git a/src/crates/assembly/core/src/agentic/tools/implementations/mod.rs b/src/crates/assembly/core/src/agentic/tools/implementations/mod.rs
index f64c8a8..29be38a 100644
--- a/src/crates/assembly/core/src/agentic/tools/implementations/mod.rs
+++ b/src/crates/assembly/core/src/agentic/tools/implementations/mod.rs
@@ -48,7 +48,6 @@ pub mod grep_tool;
 pub mod log_tool;
 pub mod ls_tool;
 pub mod mcp_tools;
-pub mod miniapp_init_tool;
 pub mod playbook_tool;
 pub mod review_platform_tool;
 pub mod session_control_tool;
@@ -90,7 +89,6 @@ pub use grep_tool::GrepTool;
 pub use log_tool::LogTool;
 pub use ls_tool::LSTool;
 pub use mcp_tools::{GetMCPPromptTool, ListMCPPromptsTool, ListMCPResourcesTool, ReadMCPResourceTool};
-pub use miniapp_init_tool::InitMiniAppTool;
 pub use playbook_tool::PlaybookTool;
 pub use review_platform_tool::ReviewPlatformTool;
 pub use session_control_tool::SessionControlTool;
diff --git a/src/crates/assembly/core/src/agentic/tools/mod.rs b/src/crates/assembly/core/src/agentic/tools/mod.rs
index 184e2aa..5b3ce6b 100644
--- a/src/crates/assembly/core/src/agentic/tools/mod.rs
+++ b/src/crates/assembly/core/src/agentic/tools/mod.rs
@@ -36,7 +36,4 @@ pub use registry::{
     all_tools, create_tool_registry, get_all_registered_tool_names, get_all_registered_tools,
     get_readonly_registered_tool_names, get_readonly_tools,
 };
-pub use restrictions::{
-    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolPathOperation, ToolPathPolicy,
-    ToolRuntimeRestrictions,
-};
+pub use restrictions::{ToolPathOperation, ToolPathPolicy, ToolRuntimeRestrictions};
diff --git a/src/crates/assembly/core/src/agentic/tools/product_runtime/materialization.rs b/src/crates/assembly/core/src/agentic/tools/product_runtime/materialization.rs
index 0fcd540..34711f9 100644
--- a/src/crates/assembly/core/src/agentic/tools/product_runtime/materialization.rs
+++ b/src/crates/assembly/core/src/agentic/tools/product_runtime/materialization.rs
@@ -58,7 +58,6 @@ pub(in crate::agentic::tools) const PRODUCT_TOOL_GROUPS: &[(&str, &[&str])] = &[
             "GenerativeUI",
             "Git",
             "ReviewPlatform",
-            "InitMiniApp",
             "ControlHub",
             "ComputerUse",
             "Playbook",
@@ -108,7 +107,6 @@ impl StaticToolProviderFactory<dyn Tool> for ProductConcreteToolFactory {
             "GenerativeUI" => Some(Arc::new(GenerativeUITool::new())),
             "Git" => Some(Arc::new(GitTool::new())),
             "ReviewPlatform" => Some(Arc::new(ReviewPlatformTool::new())),
-            "InitMiniApp" => Some(Arc::new(InitMiniAppTool::new())),
             "ControlHub" => Some(Arc::new(ControlHubTool::new())),
             "ComputerUse" => Some(Arc::new(ComputerUseTool::new())),
             "Playbook" => Some(Arc::new(PlaybookTool::new())),
diff --git a/src/crates/assembly/core/src/agentic/tools/registry/tests.rs b/src/crates/assembly/core/src/agentic/tools/registry/tests.rs
index 97e34bd..62b07e9 100644
--- a/src/crates/assembly/core/src/agentic/tools/registry/tests.rs
+++ b/src/crates/assembly/core/src/agentic/tools/registry/tests.rs
@@ -206,7 +206,6 @@ mod tests {
             "GenerativeUI",
             "Git",
             "ReviewPlatform",
-            "InitMiniApp",
             "ControlHub",
             "ComputerUse",
             "Playbook",
@@ -346,7 +345,6 @@ mod tests {
         assert!(!registry.is_tool_collapsed("GetToolSpec"));
         assert!(registry.is_tool_collapsed("Git"));
         assert!(registry.is_tool_collapsed("ReviewPlatform"));
-        assert!(!registry.is_tool_collapsed("InitMiniApp"));
     }
 
     #[test]
diff --git a/src/crates/assembly/core/src/agentic/tools/restrictions.rs b/src/crates/assembly/core/src/agentic/tools/restrictions.rs
index f6f2586..c757762 100644
--- a/src/crates/assembly/core/src/agentic/tools/restrictions.rs
+++ b/src/crates/assembly/core/src/agentic/tools/restrictions.rs
@@ -2,93 +2,8 @@ use crate::util::errors::{NortHingError, NortHingResult};
 pub use northhing_agent_tools::{
     is_remote_posix_path_within_root, ToolPathOperation, ToolPathPolicy, ToolRestrictionError, ToolRuntimeRestrictions,
 };
-use std::collections::{BTreeMap, BTreeSet};
 use std::path::{Path, PathBuf};
 
-/// MiniApp agent runs execute inside a MiniApp iframe without Flow Chat tool cards
-/// or AskUserQuestion UI. Treat those sessions as headless even on follow-up turns
-/// that reuse the hidden session via `created_by`.
-pub fn is_miniapp_headless_agent_run(
-    user_message_metadata: Option<&serde_json::Value>,
-    created_by: Option<&str>,
-) -> bool {
-    if user_message_metadata
-        .and_then(|metadata| metadata.get("surface"))
-        .and_then(|value| value.as_str())
-        == Some("miniapp_agent")
-    {
-        return true;
-    }
-    created_by.is_some_and(|owner| owner.starts_with("miniapp-agent:"))
-}
-
-/// Tools that require Flow Chat / desktop UI interaction must not run inside MiniApps.
-pub fn miniapp_headless_agent_tool_restrictions() -> ToolRuntimeRestrictions {
-    const DENIED_TOOLS: &[(&str, &str)] = &[
-        (
-            "AskUserQuestion",
-            "AskUserQuestion is unavailable in MiniApp headless agent runs. Decide yourself and record assumptions in project files.",
-        ),
-        (
-            "ControlHub",
-            "ControlHub is unavailable in MiniApp headless agent runs.",
-        ),
-        (
-            "GenerativeUI",
-            "GenerativeUI is unavailable in MiniApp headless agent runs.",
-        ),
-        (
-            "ComputerUse",
-            "ComputerUse is unavailable in MiniApp headless agent runs.",
-        ),
-        (
-            "ComputerUseMouseClick",
-            "ComputerUseMouseClick is unavailable in MiniApp headless agent runs.",
-        ),
-        (
-            "ComputerUseMouseStep",
-            "ComputerUseMouseStep is unavailable in MiniApp headless agent runs.",
-        ),
-        (
-            "ComputerUseMousePrecise",
-            "ComputerUseMousePrecise is unavailable in MiniApp headless agent runs.",
-        ),
-        (
-            "ReviewPlatform",
-            "ReviewPlatform is unavailable in MiniApp headless agent runs.",
-        ),
-        (
-            "MiniappInit",
-            "MiniappInit is unavailable in MiniApp headless agent runs.",
-        ),
-        (
-            "Playbook",
-            "Playbook is unavailable in MiniApp headless agent runs.",
-        ),
-        (
-            "Cron",
-            "Cron is unavailable in MiniApp headless agent runs.",
-        ),
-        (
-            "SessionControl",
-            "SessionControl is unavailable in MiniApp headless agent runs.",
-        ),
-    ];
-
-    let mut denied_tool_names = BTreeSet::new();
-    let mut denied_tool_messages = BTreeMap::new();
-    for (name, message) in DENIED_TOOLS {
-        denied_tool_names.insert((*name).to_string());
-        denied_tool_messages.insert((*name).to_string(), (*message).to_string());
-    }
-
-    ToolRuntimeRestrictions {
-        denied_tool_names,
-        denied_tool_messages,
-        ..Default::default()
-    }
-}
-
 impl From<ToolRestrictionError> for NortHingError {
     fn from(error: ToolRestrictionError) -> Self {
         NortHingError::tool(error.to_string())
@@ -145,27 +60,6 @@ fn canonicalize_best_effort(path: &Path) -> NortHingResult<PathBuf> {
 mod tests {
     use super::*;
 
-    #[test]
-    fn miniapp_headless_restrictions_block_interactive_tools() {
-        let restrictions = miniapp_headless_agent_tool_restrictions();
-
-        assert!(!restrictions.is_tool_allowed("AskUserQuestion"));
-        assert!(!restrictions.is_tool_allowed("ControlHub"));
-        assert!(restrictions.is_tool_allowed("Task"));
-        assert!(restrictions.is_tool_allowed("WebSearch"));
-    }
-
-    #[test]
-    fn miniapp_headless_run_detection_uses_surface_and_created_by() {
-        let metadata = serde_json::json!({ "surface": "miniapp_agent" });
-        assert!(is_miniapp_headless_agent_run(Some(&metadata), None));
-        assert!(is_miniapp_headless_agent_run(
-            None,
-            Some("miniapp-agent:builtin-ppt-live:run-1")
-        ));
-        assert!(!is_miniapp_headless_agent_run(None, Some("desktop-user")));
-    }
-
     #[test]
     fn runtime_restrictions_allow_all_when_empty() {
         let restrictions = ToolRuntimeRestrictions::default();
diff --git a/src/crates/assembly/core/src/service/announcement/content/tips/en-US/013_miniapp.md b/src/crates/assembly/core/src/service/announcement/content/tips/en-US/013_miniapp.md
deleted file mode 100644
index 8a91e84..0000000
--- a/src/crates/assembly/core/src/service/announcement/content/tips/en-US/013_miniapp.md
+++ /dev/null
@@ -1,9 +0,0 @@
----
-id: miniapp
-nth_open: 15
-auto_dismiss_secs: 10
----
-
-# MiniApp instant apps
-
-Ask AI to generate a runnable mini-application directly through conversation
diff --git a/src/crates/assembly/core/src/service/announcement/content/tips/zh-CN/013_miniapp.md b/src/crates/assembly/core/src/service/announcement/content/tips/zh-CN/013_miniapp.md
deleted file mode 100644
index 6cfcc60..0000000
--- a/src/crates/assembly/core/src/service/announcement/content/tips/zh-CN/013_miniapp.md
+++ /dev/null
@@ -1,9 +0,0 @@
----
-id: miniapp
-nth_open: 15
-auto_dismiss_secs: 10
----
-
-# MiniApp 即时应用
-
-通过对话让 AI 直接生成可运行的小应用
diff --git a/src/crates/assembly/core/src/service/announcement/content/tips/zh-TW/013_miniapp.md b/src/crates/assembly/core/src/service/announcement/content/tips/zh-TW/013_miniapp.md
deleted file mode 100644
index e4e12d6..0000000
--- a/src/crates/assembly/core/src/service/announcement/content/tips/zh-TW/013_miniapp.md
+++ /dev/null
@@ -1,9 +0,0 @@
----
-id: miniapp
-nth_open: 15
-auto_dismiss_secs: 10
----
-
-# MiniApp 即時應用
-
-通過對話讓 AI 直接生成可運行的小應用
diff --git a/src/crates/assembly/product-capabilities/src/lib.rs b/src/crates/assembly/product-capabilities/src/lib.rs
index 024a5aa..2a7df93 100644
--- a/src/crates/assembly/product-capabilities/src/lib.rs
+++ b/src/crates/assembly/product-capabilities/src/lib.rs
@@ -14,7 +14,6 @@ pub enum ProductCapabilityId {
     CodeAgent,
     DeepReview,
     DeepResearch,
-    MiniApp,
 }
 
 impl ProductCapabilityId {
@@ -23,7 +22,6 @@ impl ProductCapabilityId {
             Self::CodeAgent => "code-agent",
             Self::DeepReview => "deep-review",
             Self::DeepResearch => "deep-research",
-            Self::MiniApp => "miniapp",
         }
     }
 }
@@ -363,12 +361,6 @@ const DEEP_RESEARCH_SERVICES: &[RuntimeServiceCapability] = &[
     RuntimeServiceCapability::Permission,
     RuntimeServiceCapability::Events,
 ];
-const MINIAPP_SERVICES: &[RuntimeServiceCapability] = &[
-    RuntimeServiceCapability::FileSystem,
-    RuntimeServiceCapability::Workspace,
-    RuntimeServiceCapability::Permission,
-    RuntimeServiceCapability::Events,
-];
 
 const DEFAULT_PRODUCT_CAPABILITY_PACKS: &[ProductCapabilityPack] = &[
     ProductCapabilityPack::new(
@@ -383,10 +375,6 @@ const DEFAULT_PRODUCT_CAPABILITY_PACKS: &[ProductCapabilityPack] = &[
         ProductCapabilityId::DeepResearch,
         DEEP_RESEARCH_SERVICES,
     ),
-    ProductCapabilityPack::new(
-        ProductCapabilityId::MiniApp,
-        MINIAPP_SERVICES,
-    ),
 ];
 
 pub fn default_product_capability_registry() -> ProductCapabilityRegistry {
diff --git a/src/crates/assembly/product-capabilities/tests/product_capabilities.rs b/src/crates/assembly/product-capabilities/tests/product_capabilities.rs
index 8546dd1..0300158 100644
--- a/src/crates/assembly/product-capabilities/tests/product_capabilities.rs
+++ b/src/crates/assembly/product-capabilities/tests/product_capabilities.rs
@@ -16,7 +16,7 @@ fn capability_packs_describe_service_requirements() {
         .collect::<Vec<_>>();
     assert_eq!(
         capability_ids,
-        vec!["code-agent", "deep-review", "deep-research", "miniapp"]
+        vec!["code-agent", "deep-review", "deep-research"]
     );
 
     let service_capabilities = registry.required_service_capabilities();
@@ -28,7 +28,7 @@ fn capability_packs_describe_service_requirements() {
 
 #[test]
 fn product_assembly_plan_makes_delivery_profile_explicit_without_reducing_capabilities() {
-    let expected_capabilities = vec!["code-agent", "deep-review", "deep-research", "miniapp"];
+    let expected_capabilities = vec!["code-agent", "deep-review", "deep-research"];
 
     for profile in DeliveryProfile::all_current_product_profiles().iter().copied() {
         let plan = product_assembly_plan_for_profile(profile);
@@ -83,7 +83,7 @@ fn default_capability_assembly_keeps_service_facts_together() {
         .collect::<Vec<_>>();
     assert_eq!(
         capability_ids,
-        vec!["code-agent", "deep-review", "deep-research", "miniapp"]
+        vec!["code-agent", "deep-review", "deep-research"]
     );
 
     let service_capabilities = assembly.required_service_capabilities();
diff --git a/tests/e2e/specs/l0-navigation.spec.ts b/tests/e2e/specs/l0-navigation.spec.ts
index edc4b6c..72025a0 100644
--- a/tests/e2e/specs/l0-navigation.spec.ts
+++ b/tests/e2e/specs/l0-navigation.spec.ts
@@ -11,7 +11,6 @@ const NAV_ENTRY_SELECTORS = [
   '.northhing-nav-panel__workspace-item-name-btn',
   '.northhing-nav-panel__inline-item',
   '.northhing-nav-panel__workspace-create-main',
-  '.northhing-nav-panel__miniapp-entry',
 ];
 
 async function getNavigationEntries() {
diff --git a/tests/e2e/specs/l1-navigation.spec.ts b/tests/e2e/specs/l1-navigation.spec.ts
index 302f3a7..ff2b271 100644
--- a/tests/e2e/specs/l1-navigation.spec.ts
+++ b/tests/e2e/specs/l1-navigation.spec.ts
@@ -15,7 +15,6 @@ const NAV_ENTRY_SELECTORS = [
   '.northhing-nav-panel__workspace-item-name-btn',
   '.northhing-nav-panel__inline-item',
   '.northhing-nav-panel__workspace-create-main',
-  '.northhing-nav-panel__miniapp-entry',
 ];
 
 async function getNavigationEntryCounts(): Promise<Record<string, number>> {
@@ -170,7 +169,7 @@ describe('L1 Navigation', () => {
         return;
       }
 
-      const activeItems = await browser.$$('.northhing-nav-panel__item.is-active, .northhing-nav-panel__inline-item.is-active, .northhing-nav-panel__miniapp-entry.is-active');
+      const activeItems = await browser.$$('.northhing-nav-panel__item.is-active, .northhing-nav-panel__inline-item.is-active');
       const activeCount = activeItems.length;
       console.log('[L1] Active navigation items:', activeCount);
 
@@ -191,7 +190,7 @@ describe('L1 Navigation', () => {
       }
 
       // Get initial active item
-      const initialActive = await browser.$$('.northhing-nav-panel__item.is-active, .northhing-nav-panel__inline-item.is-active, .northhing-nav-panel__miniapp-entry.is-active');
+      const initialActive = await browser.$$('.northhing-nav-panel__item.is-active, .northhing-nav-panel__inline-item.is-active');
       const initialActiveCount = initialActive.length;
       console.log('[L1] Initial active items:', initialActiveCount);
 
@@ -229,7 +228,7 @@ describe('L1 Navigation', () => {
       }
 
       // Check for active state (don't fail if state doesn't change)
-      const afterActive = await browser.$$('.northhing-nav-panel__item.is-active, .northhing-nav-panel__inline-item.is-active, .northhing-nav-panel__miniapp-entry.is-active');
+      const afterActive = await browser.$$('.northhing-nav-panel__item.is-active, .northhing-nav-panel__inline-item.is-active');
       console.log('[L1] Active items after click:', afterActive.length);
 
       // Verify active state detection completed
