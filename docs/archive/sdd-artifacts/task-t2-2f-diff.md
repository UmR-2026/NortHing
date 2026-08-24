BASE: 65145cf (working-tree diff, task not yet committed)

## git diff --stat
 .../rules/source/required-rules.mjs                |  45 +------
 scripts/core-boundaries/self-test.mjs              |  16 +--
 src/crates/assembly/core/tests/product_assembly.rs |   2 -
 src/crates/contracts/core-types/src/surface.rs     |   2 -
 .../core-types/tests/surface_contracts.rs          |  16 +--
 src/crates/contracts/runtime-ports/src/lib.rs      |   6 +-
 .../contracts/runtime-ports/src/port_core.rs       |   8 --
 src/crates/contracts/runtime-ports/src/remote.rs   | 143 ---------------------
 .../runtime-ports/src/runtime_facade_tests.rs      |  43 +------
 .../runtime-ports/src/session_workspace.rs         |   6 -
 src/crates/execution/runtime-services/src/lib.rs   |  66 +---------
 .../execution/runtime-services/src/test_support.rs |  86 +------------
 .../tests/runtime_services_contracts.rs            |  49 ++-----
 13 files changed, 31 insertions(+), 457 deletions(-)

## deleted remote.rs head (was 143 lines)
//! R26 sibling 3/4: remote 鈥?remote workspace, projection, capability, runtime host traits.
//!
//! Mavis take-over (interface crate, all items `pub`).

use serde;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::port_core::RuntimeServicePort;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteWorkspaceKind {
    Normal,
    Assistant,

## git diff -U10 (modified files)
diff --git a/scripts/core-boundaries/rules/source/required-rules.mjs b/scripts/core-boundaries/rules/source/required-rules.mjs
index 56389c7..56e4549 100644
--- a/scripts/core-boundaries/rules/source/required-rules.mjs
+++ b/scripts/core-boundaries/rules/source/required-rules.mjs
@@ -40,41 +40,37 @@ export const requiredContentRules = [
     path: 'src/crates/execution/runtime-services/tests/runtime_services_contracts.rs',
     reason:
       'runtime-services must keep behavior-equivalence contracts for required services, optional capabilities, registry assembly, and remote port exposure',
     patterns: [
       {
         regex: /\bbuilder_requires_mandatory_runtime_services\b/,
         message: 'missing mandatory runtime services regression',
       },
       {
         regex:
-          /\bfake_provider_registers_required_and_remote_services_through_registry\b/,
+          /\bfake_provider_registers_required_services_through_registry\b/,
         message: 'missing provider registry assembly regression',
       },
       {
         regex:
           /\bmissing_optional_capability_returns_typed_unsupported_error\b/,
         message: 'missing optional capability unsupported regression',
       },
       {
         regex:
           /\bcapability_availability_reports_optional_service_status_without_side_effects\b/,
         message: 'missing capability availability regression',
       },
       {
         regex: /\bbuilder_rejects_port_registered_under_the_wrong_capability\b/,
         message: 'missing capability mismatch regression',
       },
-      {
-        regex: /\bregistered_remote_ports_expose_owner_contract_methods\b/,
-        message: 'missing remote port owner contract regression',
-      },
     ],
   },
   {
     path: 'src/crates/execution/agent-runtime/src/runtime.rs',
     reason:
       'agent-runtime must expose a narrow port-backed SDK facade without depending on core, apps, or concrete service managers',
     patterns: [
       {
         regex: /\bpub struct AgentRuntime\b/,
         message: 'missing agent runtime facade type',
@@ -3010,51 +3006,20 @@ export const requiredContentRules = [
       {
         regex: /\bpub struct ThreadGoalContinuationPlan\b/,
         message: 'missing thread goal continuation plan contract',
       },
       {
         regex: /\bpub struct ThreadGoalToolResponse\b/,
         message: 'missing thread goal tool response contract',
       },
     ],
   },
-  {
-    path: 'src/crates/contracts/runtime-ports/src/remote.rs',
-    reason:
-      'runtime-ports must keep remote and subagent runtime boundary contracts DTO/trait-only',
-    patterns: [
-      {
-        regex: /\bpub struct RemoteWorkspaceFacts\b/,
-        message: 'missing remote workspace facts contract',
-      },
-      {
-        regex: /\bpub trait RemoteWorkspaceRuntimeHost\b/,
-        message: 'missing remote workspace runtime host contract',
-      },
-      {
-        regex: /\bpub trait RemoteWorkspacePort\b/,
-        message: 'missing remote workspace service port contract',
-      },
-      {
-        regex: /\bpub trait RemoteWorkspaceFileRuntimeHost\b/,
-        message: 'missing remote workspace file runtime host contract',
-      },
-      {
-        regex: /\bpub trait RemoteProjectionPort\b/,
-        message: 'missing remote projection service port contract',
-      },
-      {
-        regex: /\bpub trait RemoteInitialSyncRuntimeHost\b/,
-        message: 'missing remote initial sync runtime host contract',
-      },
-    ],
-  },
   {
     path: 'src/crates/contracts/runtime-ports/src/session_workspace.rs',
     reason:
       'runtime-ports must keep remote and subagent runtime boundary contracts DTO/trait-only',
     patterns: [
       {
         regex: /\bpub trait WorkspaceFileSystem\b/,
         message: 'missing workspace file-system port contract',
       },
       {
@@ -3077,28 +3042,20 @@ export const requiredContentRules = [
         regex: /\bpub struct WorkspaceDirEntry\b/,
         message: 'missing workspace dir-entry contract',
       },
     ],
   },
   {
     path: 'src/crates/contracts/runtime-ports/src/runtime_facade_tests.rs',
     reason:
       'runtime-ports must keep remote and subagent runtime boundary contracts DTO/trait-only',
     patterns: [
-      {
-        regex: /\bremote_workspace_contracts_preserve_workspace_and_session_facts\b/,
-        message: 'missing remote workspace contract regression',
-      },
-      {
-        regex: /\bremote_projection_contract_preserves_file_chunk_identity\b/,
-        message: 'missing remote projection contract regression',
-      },
       {
         regex: /\bworkspace_services_contract_is_runtime_port_owned\b/,
         message: 'missing workspace service ownership regression',
       },
     ],
   },
   {
     path: 'src/crates/contracts/runtime-ports/src/agent_facade_tests.rs',
     reason:
       'runtime-ports must keep remote and subagent runtime boundary contracts DTO/trait-only',
diff --git a/scripts/core-boundaries/self-test.mjs b/scripts/core-boundaries/self-test.mjs
index 87c73c8..c49d67b 100644
--- a/scripts/core-boundaries/self-test.mjs
+++ b/scripts/core-boundaries/self-test.mjs
@@ -835,47 +835,34 @@ export function runManifestParserSelfTest({
       contracts: [
         'AgentThreadGoalDeliveryKind',
         'AgentThreadGoalDeliveryRequest',
         'ThreadGoalStatus',
         'ThreadGoal',
         'SetThreadGoalResult',
         'ThreadGoalContinuationPlan',
         'ThreadGoalToolResponse',
       ],
     },
-    {
-      path: 'src/crates/contracts/runtime-ports/src/remote.rs',
-      contracts: [
-        'RemoteWorkspaceFacts',
-        'RemoteWorkspaceRuntimeHost',
-        'RemoteWorkspacePort',
-        'RemoteWorkspaceFileRuntimeHost',
-        'RemoteProjectionPort',
-        'RemoteInitialSyncRuntimeHost',
-      ],
-    },
     {
       path: 'src/crates/contracts/runtime-ports/src/session_workspace.rs',
       contracts: [
         'WorkspaceFileSystem',
         'WorkspaceShell',
         'WorkspaceServices',
         'WorkspaceCommandOptions',
         'WorkspaceCommandResult',
         'WorkspaceDirEntry',
       ],
     },
     {
       path: 'src/crates/contracts/runtime-ports/src/runtime_facade_tests.rs',
       contracts: [
-        'remote_workspace_contracts_preserve_workspace_and_session_facts',
-        'remote_projection_contract_preserves_file_chunk_identity',
         'workspace_services_contract_is_runtime_port_owned',
       ],
     },
     {
       path: 'src/crates/contracts/runtime-ports/src/agent_facade_tests.rs',
       contracts: [
         'agent_dialog_turn_request_serializes_lifecycle_contract',
         'agent_background_result_request_serializes_lifecycle_contract',
         'agent_thread_goal_delivery_request_serializes_lifecycle_contract',
         'agent_session_management_contracts_serialize_stable_shape',
@@ -910,25 +897,24 @@ export function runManifestParserSelfTest({
         'RuntimeServicesProvider',
         'RuntimeServicesRegistry',
         'CapabilityMismatch',
         'require_capability',
       ],
     },
     {
       path: 'src/crates/execution/runtime-services/tests/runtime_services_contracts.rs',
       contracts: [
         'builder_requires_mandatory_runtime_services',
-        'fake_provider_registers_required_and_remote_services_through_registry',
+        'fake_provider_registers_required_services_through_registry',
         'missing_optional_capability_returns_typed_unsupported_error',
         'capability_availability_reports_optional_service_status_without_side_effects',
         'builder_rejects_port_registered_under_the_wrong_capability',
-        'registered_remote_ports_expose_owner_contract_methods',
       ],
     },
     {
       path: 'src/crates/execution/agent-runtime/src/runtime.rs',
       contracts: [
         'AgentRuntime',
         'AgentSubmissionPort',
         'AgentDialogTurnPort',
         'submit_dialog_turn',
         'AgentLifecycleDeliveryPort',
diff --git a/src/crates/assembly/core/tests/product_assembly.rs b/src/crates/assembly/core/tests/product_assembly.rs
index 4fd4f5c..bd639fd 100644
--- a/src/crates/assembly/core/tests/product_assembly.rs
+++ b/src/crates/assembly/core/tests/product_assembly.rs
@@ -20,22 +20,20 @@ fn core_runtime_services_provider_registers_existing_adapters_and_capability_mar
         .expect("core product assembly provider should register concrete adapters");
 
     assert_eq!(
         services.session_store.capability(),
         RuntimeServiceCapability::SessionStore
     );
     assert!(services.has_capability(RuntimeServiceCapability::Terminal));
     assert!(services.has_capability(RuntimeServiceCapability::Network));
     assert!(services.has_capability(RuntimeServiceCapability::Git));
     assert!(services.has_capability(RuntimeServiceCapability::McpCatalog));
-    assert!(services.has_capability(RuntimeServiceCapability::RemoteWorkspace));
-    assert!(services.has_capability(RuntimeServiceCapability::RemoteProjection));
 }
 
 #[test]
 fn product_assembly_facade_preserves_legacy_provider_import_path() {
     let registry = RuntimeServicesRegistry::new()
         .with_provider(FakeRuntimeServicesProvider::with_all_required())
         .with_provider(product_assembly::CoreRuntimeServicesProvider::new());
 
     let services = registry
         .build(RuntimeServicesBuilder::new())
diff --git a/src/crates/contracts/core-types/src/surface.rs b/src/crates/contracts/core-types/src/surface.rs
index b5f0848..33f2ac1 100644
--- a/src/crates/contracts/core-types/src/surface.rs
+++ b/src/crates/contracts/core-types/src/surface.rs
@@ -6,32 +6,30 @@
 use serde::{Deserialize, Serialize};
 use std::collections::BTreeMap;
 
 pub type SurfaceMetadata = BTreeMap<String, String>;
 
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
 #[serde(rename_all = "snake_case")]
 pub enum SurfaceKind {
     Desktop,
     Cli,
-    Remote,
     Acp,
     Server,
 }
 
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
 #[serde(rename_all = "snake_case")]
 pub enum ThreadEnvironmentKind {
     Local,
     Worktree,
     RemoteSsh,
-    RemoteConnect,
     CloudLike,
     Acp,
 }
 
 #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
 #[serde(rename_all = "camelCase")]
 pub struct ThreadEnvironment {
     pub kind: ThreadEnvironmentKind,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub workspace_path: Option<String>,
diff --git a/src/crates/contracts/core-types/tests/surface_contracts.rs b/src/crates/contracts/core-types/tests/surface_contracts.rs
index da9e640..664219e 100644
--- a/src/crates/contracts/core-types/tests/surface_contracts.rs
+++ b/src/crates/contracts/core-types/tests/surface_contracts.rs
@@ -24,54 +24,54 @@ fn surface_contract_serializes_observational_runtime_facts() {
     assert_eq!(json["producerSurface"], "cli");
     assert!(json.get("parentArtifactId").is_none());
 }
 
 #[test]
 fn permission_and_capability_contracts_keep_source_identity() {
     let request = CapabilityRequest {
         request_id: "cap-1".to_string(),
         kind: CapabilityRequestKind::PermissionDecision,
         source: ApprovalSource {
-            surface: SurfaceKind::Remote,
-            thread_id: Some("thread-remote".to_string()),
-            turn_id: Some("turn-remote".to_string()),
+            surface: SurfaceKind::Server,
+            thread_id: Some("thread-server".to_string()),
+            turn_id: Some("turn-server".to_string()),
             subagent_thread_id: Some("child-1".to_string()),
         },
         artifact: None,
         permission: Some(PermissionScope {
             tool_id: Some("bash".to_string()),
             command_prefix: Some("git status".to_string()),
             path_pattern: Some("src/**".to_string()),
             agent_role: Some("reviewer".to_string()),
-            surface: Some(SurfaceKind::Remote),
-            thread_id: Some("thread-remote".to_string()),
+            surface: Some(SurfaceKind::Server),
+            thread_id: Some("thread-server".to_string()),
         }),
         decision: Some(PermissionDecision::ApproveSession),
         metadata: BTreeMap::new(),
     };
 
     let json = serde_json::to_value(&request).expect("serialize request");
 
     assert_eq!(json["kind"], "permission_decision");
-    assert_eq!(json["source"]["surface"], "remote");
+    assert_eq!(json["source"]["surface"], "server");
     assert_eq!(json["source"]["subagentThreadId"], "child-1");
     assert_eq!(json["permission"]["commandPrefix"], "git status");
     assert_eq!(json["decision"], "approve_session");
 }
 
 #[test]
 fn thread_environment_contract_does_not_require_surface_specific_fields() {
     let env = ThreadEnvironment {
-        kind: ThreadEnvironmentKind::RemoteConnect,
+        kind: ThreadEnvironmentKind::RemoteSsh,
         workspace_path: None,
         remote_connection_id: Some("paired-phone".to_string()),
         label: None,
         metadata: BTreeMap::new(),
     };
 
     let json = serde_json::to_value(&env).expect("serialize environment");
 
-    assert_eq!(json["kind"], "remote_connect");
+    assert_eq!(json["kind"], "remote_ssh");
     assert_eq!(json["remoteConnectionId"], "paired-phone");
     assert!(json.get("workspacePath").is_none());
     assert!(json.get("label").is_none());
 }
diff --git a/src/crates/contracts/runtime-ports/src/lib.rs b/src/crates/contracts/runtime-ports/src/lib.rs
index bc98437..d97c521 100644
--- a/src/crates/contracts/runtime-ports/src/lib.rs
+++ b/src/crates/contracts/runtime-ports/src/lib.rs
@@ -1,39 +1,37 @@
 #![allow(clippy::too_many_arguments)]
 //! Thin runtime ports for boundaries that currently cross service and agentic
 //! concrete implementations.
 //!
 //! This crate intentionally contains only DTOs and traits. It must not depend
 //! on concrete managers, platform adapters, `northhing-core`, or app crates.
 //!
-//! R26 god-split: facade with 4 sibling sub-domain files (port_core,
-//! session_workspace, remote, agent).
+//! R26 god-split: facade with 3 sibling sub-domain files (port_core,
+//! session_workspace, agent).
 
 pub mod agent;
 pub mod deep_research;
 pub mod lightweight_task;
 pub mod mcp;
 pub mod port_core;
-pub mod remote;
 pub mod session_workspace;
 
 pub use agent::*;
 pub use deep_research::{
     renumber_research_report, ResearchCitationDisplayMapEntry, ResearchCitationRenumberOutput,
     ResearchCitationRenumberStats,
 };
 pub use lightweight_task::{
     LightweightTaskOutput, LightweightTaskRequest, LightweightTelemetrySink, ToolDispatcherPort,
 };
 pub use mcp::{
     format_mcp_status, format_mcp_status_err, McpCatalogError, McpCatalogReader, McpServerDto, McpServerStatusDto,
 };
 pub use port_core::*;
-pub use remote::*;
 pub use session_workspace::*;
 
 #[cfg(test)]
 mod agent_facade_tests;
 #[cfg(test)]
 mod port_facade_tests;
 #[cfg(test)]
 mod runtime_facade_tests;
diff --git a/src/crates/contracts/runtime-ports/src/port_core.rs b/src/crates/contracts/runtime-ports/src/port_core.rs
index a6df68f..844c093 100644
--- a/src/crates/contracts/runtime-ports/src/port_core.rs
+++ b/src/crates/contracts/runtime-ports/src/port_core.rs
@@ -48,43 +48,35 @@ pub enum RuntimeServiceCapability {
     FileSystem,
     Workspace,
     SessionStore,
     Permission,
     Events,
     Clock,
     Terminal,
     Network,
     Git,
     McpCatalog,
-    RemoteConnection,
-    RemoteWorkspace,
-    RemoteProjection,
-    RemoteCapabilities,
 }
 
 impl RuntimeServiceCapability {
     pub const fn as_str(self) -> &'static str {
         match self {
             Self::FileSystem => "filesystem",
             Self::Workspace => "workspace",
             Self::SessionStore => "session_store",
             Self::Permission => "permission",
             Self::Events => "events",
             Self::Clock => "clock",
             Self::Terminal => "terminal",
             Self::Network => "network",
             Self::Git => "git",
             Self::McpCatalog => "mcp_catalog",
-            Self::RemoteConnection => "remote_connection",
-            Self::RemoteWorkspace => "remote_workspace",
-            Self::RemoteProjection => "remote_projection",
-            Self::RemoteCapabilities => "remote_capabilities",
         }
     }
 }
 
 impl std::fmt::Display for RuntimeServiceCapability {
     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
         f.write_str(self.as_str())
     }
 }
 
diff --git a/src/crates/contracts/runtime-ports/src/runtime_facade_tests.rs b/src/crates/contracts/runtime-ports/src/runtime_facade_tests.rs
index 3ac3ac3..14ab1ae 100644
--- a/src/crates/contracts/runtime-ports/src/runtime_facade_tests.rs
+++ b/src/crates/contracts/runtime-ports/src/runtime_facade_tests.rs
@@ -1,55 +1,16 @@
-//! Contract tests for remote/workspace re-exports on the runtime-ports facade.
+//! Contract tests for workspace re-exports on the runtime-ports facade.
 //!
-//! R39d sibling: split facade-test bulk from lib.rs (remote + workspace).
+//! R39d sibling: split facade-test bulk from lib.rs (workspace).
 
 use crate::*;
 
-#[test]
-fn remote_workspace_contracts_preserve_workspace_and_session_facts() {
-    let workspace = RemoteWorkspaceFacts {
-        path: "/workspace/project".to_string(),
-        name: "project".to_string(),
-        git_branch: Some("main".to_string()),
-        kind: RemoteWorkspaceKind::Remote,
-        assistant_id: Some("assistant_1".to_string()),
-    };
-    let session = RemoteSessionMetadata {
-        session_id: "session_1".to_string(),
-        name: "Research".to_string(),
-        agent_type: "CodeAgent".to_string(),
-        created_at_ms: 10,
-        last_active_at_ms: 20,
-        turn_count: 3,
-    };
-
-    assert_eq!(workspace.kind.as_wire_str(), "remote");
-    assert_eq!(workspace.assistant_id.as_deref(), Some("assistant_1"));
-    assert_eq!(session.turn_count, 3);
-}
-
-#[test]
-fn remote_projection_contract_preserves_file_chunk_identity() {
-    let chunk = RemoteWorkspaceFileChunk {
-        name: "report.md".to_string(),
-        bytes: b"chunk".to_vec(),
-        offset: 6,
-        chunk_size: 5,
-        total_size: 11,
-        mime_type: "text/markdown",
-    };
-
-    assert_eq!(chunk.name, "report.md");
-    assert_eq!(chunk.bytes, b"chunk");
-    assert_eq!(chunk.offset + chunk.chunk_size, chunk.total_size);
-}
-
 #[test]
 fn remote_control_state_snapshot_serializes_active_turn_contract() {
     let snapshot = RemoteControlStateSnapshot {
         session_id: "session_1".to_string(),
         state: RemoteControlSessionState::Processing,
         active_turn_id: Some("turn_1".to_string()),
         queue_depth: 2,
         metadata: serde_json::Map::new(),
     };
 
diff --git a/src/crates/contracts/runtime-ports/src/session_workspace.rs b/src/crates/contracts/runtime-ports/src/session_workspace.rs
index 8d617f0..aacaa4f 100644
--- a/src/crates/contracts/runtime-ports/src/session_workspace.rs
+++ b/src/crates/contracts/runtime-ports/src/session_workspace.rs
@@ -529,16 +529,10 @@ pub trait TerminalPort: RuntimeServicePort {}
 pub trait NetworkPort: RuntimeServicePort {}
 
 pub trait GitPort: RuntimeServicePort {}
 
 /// Marker: any `McpCatalogPort` (the rich async trait in `mcp.rs`) is
 /// also a `RuntimeServicePort` for registration through the
 /// `RuntimeServicesBuilder`. Kept as a separate marker so the rich
 /// port trait stays narrow (single async method) while the runtime
 /// services registry can still use the standard builder pattern.
 pub trait McpCatalogPort: RuntimeServicePort {}
-
-/// Typed registration boundary for remote connection providers.
-///
-/// PR1 intentionally keeps this trait handle-free; PR2 adds owner-specific
-/// lifecycle methods once behavior-equivalence tests are in place.
-pub trait RemoteConnectionPort: RuntimeServicePort {}
diff --git a/src/crates/execution/runtime-services/src/lib.rs b/src/crates/execution/runtime-services/src/lib.rs
index 835e655..14d55a2 100644
--- a/src/crates/execution/runtime-services/src/lib.rs
+++ b/src/crates/execution/runtime-services/src/lib.rs
@@ -1,19 +1,18 @@
 #![allow(clippy::too_many_arguments)]
 //! Typed Runtime Services assembly.
 
 use std::sync::Arc;
 
 use northhing_runtime_ports::{
-    ClockPort, FileSystemPort, GitPort, McpCatalogPort, NetworkPort, PermissionPort, RemoteCapabilityPort,
-    RemoteConnectionPort, RemoteProjectionPort, RemoteWorkspacePort, RuntimeEventSink, RuntimeServiceCapability,
-    RuntimeServicePort, SessionStorePort, TerminalPort, WorkspacePort,
+    ClockPort, FileSystemPort, GitPort, McpCatalogPort, NetworkPort, PermissionPort, RuntimeEventSink,
+    RuntimeServiceCapability, RuntimeServicePort, SessionStorePort, TerminalPort, WorkspacePort,
 };
 
 pub mod test_support;
 
 #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
 pub enum RuntimeServicesError {
     #[error("required runtime service {capability} is not registered")]
     MissingRequired { capability: RuntimeServiceCapability },
     #[error("runtime service {capability} is not registered")]
     Unsupported { capability: RuntimeServiceCapability },
@@ -35,76 +34,52 @@ pub struct RuntimeServices {
     pub filesystem: Arc<dyn FileSystemPort>,
     pub workspace: Arc<dyn WorkspacePort>,
     pub session_store: Arc<dyn SessionStorePort>,
     pub permission: Arc<dyn PermissionPort>,
     pub events: Arc<dyn RuntimeEventSink>,
     pub clock: Arc<dyn ClockPort>,
     pub terminal: Option<Arc<dyn TerminalPort>>,
     pub network: Option<Arc<dyn NetworkPort>>,
     pub git: Option<Arc<dyn GitPort>>,
     pub mcp_catalog: Option<Arc<dyn McpCatalogPort>>,
-    pub remote_connection: Option<Arc<dyn RemoteConnectionPort>>,
-    pub remote_workspace: Option<Arc<dyn RemoteWorkspacePort>>,
-    pub remote_projection: Option<Arc<dyn RemoteProjectionPort>>,
-    pub remote_capabilities: Option<Arc<dyn RemoteCapabilityPort>>,
 }
 
 impl std::fmt::Debug for RuntimeServices {
     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
         f.debug_struct("RuntimeServices")
             .field("filesystem", &self.filesystem.capability())
             .field("workspace", &self.workspace.capability())
             .field("session_store", &self.session_store.capability())
             .field("permission", &self.permission.capability())
             .field("events", &RuntimeServiceCapability::Events)
             .field("clock", &self.clock.capability())
             .field("terminal", &self.terminal.as_ref().map(|port| port.capability()))
             .field("network", &self.network.as_ref().map(|port| port.capability()))
             .field("git", &self.git.as_ref().map(|port| port.capability()))
             .field("mcp_catalog", &self.mcp_catalog.as_ref().map(|port| port.capability()))
-            .field(
-                "remote_connection",
-                &self.remote_connection.as_ref().map(|port| port.capability()),
-            )
-            .field(
-                "remote_workspace",
-                &self.remote_workspace.as_ref().map(|port| port.capability()),
-            )
-            .field(
-                "remote_projection",
-                &self.remote_projection.as_ref().map(|port| port.capability()),
-            )
-            .field(
-                "remote_capabilities",
-                &self.remote_capabilities.as_ref().map(|port| port.capability()),
-            )
             .finish()
     }
 }
 
 impl RuntimeServices {
     pub fn has_capability(&self, capability: RuntimeServiceCapability) -> bool {
         match capability {
             RuntimeServiceCapability::FileSystem
             | RuntimeServiceCapability::Workspace
             | RuntimeServiceCapability::SessionStore
             | RuntimeServiceCapability::Permission
             | RuntimeServiceCapability::Events
             | RuntimeServiceCapability::Clock => true,
             RuntimeServiceCapability::Terminal => self.terminal.is_some(),
             RuntimeServiceCapability::Network => self.network.is_some(),
             RuntimeServiceCapability::Git => self.git.is_some(),
             RuntimeServiceCapability::McpCatalog => self.mcp_catalog.is_some(),
-            RuntimeServiceCapability::RemoteConnection => self.remote_connection.is_some(),
-            RuntimeServiceCapability::RemoteWorkspace => self.remote_workspace.is_some(),
-            RuntimeServiceCapability::RemoteProjection => self.remote_projection.is_some(),
-            RuntimeServiceCapability::RemoteCapabilities => self.remote_capabilities.is_some(),
         }
     }
 
     pub fn capability_availability(&self, capability: RuntimeServiceCapability) -> CapabilityAvailability {
         CapabilityAvailability {
             capability,
             available: self.has_capability(capability),
         }
     }
 
@@ -122,24 +97,20 @@ pub struct RuntimeServicesBuilder {
     filesystem: Option<Arc<dyn FileSystemPort>>,
     workspace: Option<Arc<dyn WorkspacePort>>,
     session_store: Option<Arc<dyn SessionStorePort>>,
     permission: Option<Arc<dyn PermissionPort>>,
     events: Option<Arc<dyn RuntimeEventSink>>,
     clock: Option<Arc<dyn ClockPort>>,
     terminal: Option<Arc<dyn TerminalPort>>,
     network: Option<Arc<dyn NetworkPort>>,
     git: Option<Arc<dyn GitPort>>,
     mcp_catalog: Option<Arc<dyn McpCatalogPort>>,
-    remote_connection: Option<Arc<dyn RemoteConnectionPort>>,
-    remote_workspace: Option<Arc<dyn RemoteWorkspacePort>>,
-    remote_projection: Option<Arc<dyn RemoteProjectionPort>>,
-    remote_capabilities: Option<Arc<dyn RemoteCapabilityPort>>,
 }
 
 impl RuntimeServicesBuilder {
     pub fn new() -> Self {
         Self::default()
     }
 
     pub fn with_filesystem(mut self, port: Arc<dyn FileSystemPort>) -> Self {
         self.filesystem = Some(port);
         self
@@ -183,65 +154,32 @@ impl RuntimeServicesBuilder {
     pub fn with_optional_git(mut self, port: Option<Arc<dyn GitPort>>) -> Self {
         self.git = port;
         self
     }
 
     pub fn with_optional_mcp_catalog(mut self, port: Option<Arc<dyn McpCatalogPort>>) -> Self {
         self.mcp_catalog = port;
         self
     }
 
-    pub fn with_optional_remote_connection(mut self, port: Option<Arc<dyn RemoteConnectionPort>>) -> Self {
-        self.remote_connection = port;
-        self
-    }
-
-    pub fn with_optional_remote_workspace(mut self, port: Option<Arc<dyn RemoteWorkspacePort>>) -> Self {
-        self.remote_workspace = port;
-        self
-    }
-
-    pub fn with_optional_remote_projection(mut self, port: Option<Arc<dyn RemoteProjectionPort>>) -> Self {
-        self.remote_projection = port;
-        self
-    }
-
-    pub fn with_optional_remote_capabilities(mut self, port: Option<Arc<dyn RemoteCapabilityPort>>) -> Self {
-        self.remote_capabilities = port;
-        self
-    }
-
     pub fn build(self) -> Result<RuntimeServices, RuntimeServicesError> {
         Ok(RuntimeServices {
             filesystem: Self::required_service(self.filesystem, RuntimeServiceCapability::FileSystem)?,
             workspace: Self::required_service(self.workspace, RuntimeServiceCapability::Workspace)?,
             session_store: Self::required_service(self.session_store, RuntimeServiceCapability::SessionStore)?,
             permission: Self::required_service(self.permission, RuntimeServiceCapability::Permission)?,
             events: Self::required(self.events, RuntimeServiceCapability::Events)?,
             clock: Self::required_service(self.clock, RuntimeServiceCapability::Clock)?,
             terminal: Self::optional_service(self.terminal, RuntimeServiceCapability::Terminal)?,
             network: Self::optional_service(self.network, RuntimeServiceCapability::Network)?,
             git: Self::optional_service(self.git, RuntimeServiceCapability::Git)?,
             mcp_catalog: Self::optional_service(self.mcp_catalog, RuntimeServiceCapability::McpCatalog)?,
-            remote_connection: Self::optional_service(
-                self.remote_connection,
-                RuntimeServiceCapability::RemoteConnection,
-            )?,
-            remote_workspace: Self::optional_service(self.remote_workspace, RuntimeServiceCapability::RemoteWorkspace)?,
-            remote_projection: Self::optional_service(
-                self.remote_projection,
-                RuntimeServiceCapability::RemoteProjection,
-            )?,
-            remote_capabilities: Self::optional_service(
-                self.remote_capabilities,
-                RuntimeServiceCapability::RemoteCapabilities,
-            )?,
         })
     }
 
     fn required<T>(port: Option<Arc<T>>, capability: RuntimeServiceCapability) -> Result<Arc<T>, RuntimeServicesError>
     where
         T: ?Sized,
     {
         port.ok_or(RuntimeServicesError::MissingRequired { capability })
     }
 
diff --git a/src/crates/execution/runtime-services/src/test_support.rs b/src/crates/execution/runtime-services/src/test_support.rs
index 51f1a3d..a70757c 100644
--- a/src/crates/execution/runtime-services/src/test_support.rs
+++ b/src/crates/execution/runtime-services/src/test_support.rs
@@ -1,19 +1,17 @@
 use std::sync::Arc;
 
 use northhing_runtime_ports::{
     ClockPort, FileSystemPort, GitPort, McpCatalogPort, NetworkPort, PermissionDecision, PermissionPort,
-    PermissionRequest, PortResult, RemoteAssistantWorkspaceFacts, RemoteCapabilityPort, RemoteConnectionPort,
-    RemoteProjectionPort, RemoteRecentWorkspaceFacts, RemoteWorkspaceFacts, RemoteWorkspaceFileRuntimeHost,
-    RemoteWorkspaceKind, RemoteWorkspacePort, RemoteWorkspaceRuntimeHost, RemoteWorkspaceUpdate, RuntimeEventEnvelope,
-    RuntimeEventSink, RuntimeServiceCapability, RuntimeServicePort, SessionStoragePathRequest,
-    SessionStoragePathResolution, SessionStorePort, TerminalPort, WorkspacePort,
+    PermissionRequest, PortResult, RuntimeEventEnvelope, RuntimeEventSink, RuntimeServiceCapability,
+    RuntimeServicePort, SessionStoragePathRequest, SessionStoragePathResolution, SessionStorePort, TerminalPort,
+    WorkspacePort,
 };
 
 use crate::{RuntimeServices, RuntimeServicesBuilder, RuntimeServicesError, RuntimeServicesProvider};
 
 #[derive(Debug)]
 pub struct FakeRuntimePort {
     capability: RuntimeServiceCapability,
 }
 
 impl FakeRuntimePort {
@@ -36,64 +34,20 @@ impl SessionStorePort for FakeRuntimePort {
         &self,
         request: SessionStoragePathRequest,
     ) -> PortResult<SessionStoragePathResolution> {
         Ok(SessionStoragePathResolution::local(request.workspace_path))
     }
 }
 impl TerminalPort for FakeRuntimePort {}
 impl NetworkPort for FakeRuntimePort {}
 impl GitPort for FakeRuntimePort {}
 impl McpCatalogPort for FakeRuntimePort {}
-impl RemoteConnectionPort for FakeRuntimePort {}
-impl RemoteCapabilityPort for FakeRuntimePort {}
-
-#[async_trait::async_trait]
-impl RemoteWorkspaceRuntimeHost for FakeRuntimePort {
-    async fn current_workspace(&self) -> Option<RemoteWorkspaceFacts> {
-        Some(RemoteWorkspaceFacts {
-            path: "/remote/project".to_string(),
-            name: "project".to_string(),
-            git_branch: Some("main".to_string()),
-            kind: RemoteWorkspaceKind::Remote,
-            assistant_id: None,
-        })
-    }
-
-    async fn recent_workspaces(&self) -> Vec<RemoteRecentWorkspaceFacts> {
-        Vec::new()
-    }
-
-    async fn open_workspace(&self, path: &str) -> Result<RemoteWorkspaceUpdate, String> {
-        Ok(RemoteWorkspaceUpdate {
-            path: path.to_string(),
-            name: "project".to_string(),
-        })
-    }
-
-    async fn assistant_workspaces(&self) -> Vec<RemoteAssistantWorkspaceFacts> {
-        Vec::new()
-    }
-
-    async fn open_assistant_workspace(&self, path: &str) -> Result<RemoteWorkspaceUpdate, String> {
-        Ok(RemoteWorkspaceUpdate {
-            path: path.to_string(),
-            name: "assistant".to_string(),
-        })
-    }
-}
-
-#[async_trait::async_trait]
-impl RemoteWorkspaceFileRuntimeHost for FakeRuntimePort {
-    async fn resolve_remote_file_workspace_root(&self, _session_id: Option<&str>) -> Option<std::path::PathBuf> {
-        Some(std::path::PathBuf::from("/remote/project"))
-    }
-}
 
 #[async_trait::async_trait]
 impl PermissionPort for FakeRuntimePort {
     async fn request_permission(&self, _request: PermissionRequest) -> PortResult<PermissionDecision> {
         Ok(PermissionDecision::Allow)
     }
 }
 
 impl ClockPort for FakeRuntimePort {
     fn now_unix_millis(&self) -> i64 {
@@ -105,67 +59,41 @@ impl ClockPort for FakeRuntimePort {
 pub struct FakeRuntimeEventSink;
 
 #[async_trait::async_trait]
 impl RuntimeEventSink for FakeRuntimeEventSink {
     async fn publish_runtime_event(&self, _event: RuntimeEventEnvelope) -> PortResult<()> {
         Ok(())
     }
 }
 
 #[derive(Debug, Clone, Default)]
-pub struct FakeRuntimeServicesProvider {
-    include_remote: bool,
-}
+pub struct FakeRuntimeServicesProvider;
 
 impl FakeRuntimeServicesProvider {
     pub fn with_all_required() -> Self {
-        Self { include_remote: false }
-    }
-
-    pub fn with_all_remote(mut self) -> Self {
-        self.include_remote = true;
-        self
+        Self
     }
 
     pub fn build_services(self) -> Result<RuntimeServices, RuntimeServicesError> {
         self.register(RuntimeServicesBuilder::new()).build()
     }
 }
 
 impl RuntimeServicesProvider for FakeRuntimeServicesProvider {
     fn register(&self, builder: RuntimeServicesBuilder) -> RuntimeServicesBuilder {
         let filesystem: Arc<dyn FileSystemPort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::FileSystem));
         let workspace: Arc<dyn WorkspacePort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::Workspace));
         let session_store: Arc<dyn SessionStorePort> =
             Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::SessionStore));
         let permission: Arc<dyn PermissionPort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::Permission));
         let events: Arc<dyn RuntimeEventSink> = Arc::new(FakeRuntimeEventSink);
         let clock: Arc<dyn ClockPort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::Clock));
 
-        let builder = builder
+        builder
             .with_filesystem(filesystem)
             .with_workspace(workspace)
             .with_session_store(session_store)
             .with_permission(permission)
             .with_events(events)
-            .with_clock(clock);
-
-        if !self.include_remote {
-            return builder;
-        }
-
-        let remote_connection: Arc<dyn RemoteConnectionPort> =
-            Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::RemoteConnection));
-        let remote_workspace: Arc<dyn RemoteWorkspacePort> =
-            Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::RemoteWorkspace));
-        let remote_projection: Arc<dyn RemoteProjectionPort> =
-            Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::RemoteProjection));
-        let remote_capabilities: Arc<dyn RemoteCapabilityPort> =
-            Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::RemoteCapabilities));
-
-        builder
-            .with_optional_remote_connection(Some(remote_connection))
-            .with_optional_remote_workspace(Some(remote_workspace))
-            .with_optional_remote_projection(Some(remote_projection))
-            .with_optional_remote_capabilities(Some(remote_capabilities))
+            .with_clock(clock)
     }
 }
diff --git a/src/crates/execution/runtime-services/tests/runtime_services_contracts.rs b/src/crates/execution/runtime-services/tests/runtime_services_contracts.rs
index 48ece55..44daba7 100644
--- a/src/crates/execution/runtime-services/tests/runtime_services_contracts.rs
+++ b/src/crates/execution/runtime-services/tests/runtime_services_contracts.rs
@@ -1,89 +1,83 @@
 use std::sync::Arc;
 
 use northhing_runtime_ports::FileSystemPort;
-use northhing_runtime_ports::{
-    RemoteWorkspaceKind, RuntimeServiceCapability, SessionStorageKind, SessionStoragePathRequest,
-};
+use northhing_runtime_ports::{RuntimeServiceCapability, SessionStorageKind, SessionStoragePathRequest};
 use northhing_runtime_services::test_support::{FakeRuntimePort, FakeRuntimeServicesProvider};
 use northhing_runtime_services::{
     CapabilityAvailability, RuntimeServicesBuilder, RuntimeServicesError, RuntimeServicesProvider,
     RuntimeServicesRegistry,
 };
 
 #[test]
 fn builder_requires_mandatory_runtime_services() {
     let error = RuntimeServicesBuilder::new().build().unwrap_err();
 
     assert_eq!(
         error,
         RuntimeServicesError::MissingRequired {
             capability: RuntimeServiceCapability::FileSystem,
         }
     );
 }
 
 #[test]
-fn fake_provider_registers_required_and_remote_services_through_registry() {
-    let registry = RuntimeServicesRegistry::new()
-        .with_provider(FakeRuntimeServicesProvider::with_all_required().with_all_remote());
+fn fake_provider_registers_required_services_through_registry() {
+    let registry =
+        RuntimeServicesRegistry::new().with_provider(FakeRuntimeServicesProvider::with_all_required());
     let services = registry
         .build(RuntimeServicesBuilder::new())
         .expect("fake provider should satisfy runtime services");
 
     assert!(services.has_capability(RuntimeServiceCapability::FileSystem));
     assert!(services.has_capability(RuntimeServiceCapability::Workspace));
     assert!(services.has_capability(RuntimeServiceCapability::SessionStore));
     assert!(services.has_capability(RuntimeServiceCapability::Permission));
     assert!(services.has_capability(RuntimeServiceCapability::Events));
     assert!(services.has_capability(RuntimeServiceCapability::Clock));
-    assert!(services.has_capability(RuntimeServiceCapability::RemoteConnection));
-    assert!(services.has_capability(RuntimeServiceCapability::RemoteWorkspace));
-    assert!(services.has_capability(RuntimeServiceCapability::RemoteProjection));
-    assert!(services.has_capability(RuntimeServiceCapability::RemoteCapabilities));
 }
 
 #[test]
 fn missing_optional_capability_returns_typed_unsupported_error() {
     let services = FakeRuntimeServicesProvider::with_all_required()
         .build_services()
         .expect("required fake services should build");
 
     let error = services
-        .require_capability(RuntimeServiceCapability::RemoteConnection)
+        .require_capability(RuntimeServiceCapability::Terminal)
         .unwrap_err();
 
     assert_eq!(
         error,
         RuntimeServicesError::Unsupported {
-            capability: RuntimeServiceCapability::RemoteConnection,
+            capability: RuntimeServiceCapability::Terminal,
         }
     );
 }
 
 #[test]
 fn capability_availability_reports_optional_service_status_without_side_effects() {
     let services = FakeRuntimeServicesProvider::with_all_required()
         .build_services()
         .expect("required fake services should build");
 
     assert_eq!(
         services.capability_availability(RuntimeServiceCapability::FileSystem),
         CapabilityAvailability {
             capability: RuntimeServiceCapability::FileSystem,
             available: true,
         }
     );
     assert_eq!(
-        services.capability_availability(RuntimeServiceCapability::RemoteWorkspace),
+        services.capability_availability(RuntimeServiceCapability::Terminal),
         CapabilityAvailability {
-            capability: RuntimeServiceCapability::RemoteWorkspace,
+            capability: RuntimeServiceCapability::Terminal,
             available: false,
         }
     );
 }
 
 #[test]
 fn builder_rejects_port_registered_under_the_wrong_capability() {
     let mismatched_filesystem: Arc<dyn FileSystemPort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::Git));
     let builder = FakeRuntimeServicesProvider::with_all_required()
         .register(RuntimeServicesBuilder::new())
@@ -93,47 +87,20 @@ fn builder_rejects_port_registered_under_the_wrong_capability() {
 
     assert_eq!(
         error,
         RuntimeServicesError::CapabilityMismatch {
             expected: RuntimeServiceCapability::FileSystem,
             actual: RuntimeServiceCapability::Git,
         }
     );
 }
 
-#[tokio::test]
-async fn registered_remote_ports_expose_owner_contract_methods() {
-    let services = FakeRuntimeServicesProvider::with_all_required()
-        .with_all_remote()
-        .build_services()
-        .expect("remote fake services should build");
-
-    let workspace = services
-        .remote_workspace
-        .as_ref()
-        .expect("remote workspace port")
-        .current_workspace()
-        .await
-        .expect("fake remote workspace facts");
-    let projection_root = services
-        .remote_projection
-        .as_ref()
-        .expect("remote projection port")
-        .resolve_remote_file_workspace_root(Some("session_1"))
-        .await
-        .expect("fake remote projection root");
-
-    assert_eq!(workspace.kind, RemoteWorkspaceKind::Remote);
-    assert_eq!(workspace.path, "/remote/project");
-    assert_eq!(projection_root.to_string_lossy(), "/remote/project");
-}
-
 #[tokio::test]
 async fn registered_session_store_port_exposes_storage_path_resolution() {
     let services = FakeRuntimeServicesProvider::with_all_required()
         .build_services()
         .expect("required fake services should build");
 
     let resolution = services
         .session_store
         .resolve_session_storage_path(SessionStoragePathRequest {
             workspace_path: "/workspace".into(),
