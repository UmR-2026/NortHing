use crate::agentic::deep_review::tool_context;
use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
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

/// Core-owned tool use context.
#[derive(Debug, Clone)]
pub struct ToolUseContext {
    pub tool_call_id: Option<String>,
    pub agent_type: Option<String>,
    pub session_id: Option<String>,
    pub dialog_turn_id: Option<String>,
    pub workspace: Option<WorkspaceBinding>,
    pub unlocked_collapsed_tools: Vec<String>,
    /// Extended context data passed from execution layer to tools.
    pub custom_data: HashMap<String, Value>,
    /// Desktop automation (Computer use); only set in northhing desktop.
    pub computer_use_host: Option<crate::agentic::tools::computer_use_host::ComputerUseHostRef>,
    pub runtime_tool_restrictions: ToolRuntimeRestrictions,
    /// Runtime handles such as workspace I/O services and cancellation.
    pub runtime_handles: ToolRuntimeHandles,
    /// K.2.3 follow-up: the optional `ActorRuntime` for tools that
    /// need to spawn long-running skills (currently only `TaskTool`).
    /// `None` when no actor runtime is wired (e.g. CLI/server apps,
    /// or pre-`set_actor_runtime` construction).
    pub actor_runtime: Option<Arc<northhing_agent_dispatch::ActorRuntime>>,
}

impl ToolUseContext {
    pub(crate) fn delegation_policy(&self) -> DelegationPolicy {
        let allow_subagent_spawn = self
            .custom_data
            .get("delegation_allow_subagent_spawn")
            .and_then(|value| value.as_bool())
            .unwrap_or(true);
        let nesting_depth = self
            .custom_data
            .get("delegation_nesting_depth")
            .and_then(|value| value.as_u64())
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(0);

        DelegationPolicy {
            allow_subagent_spawn,
            nesting_depth,
        }
    }

    pub fn workspace_root(&self) -> Option<&Path> {
        self.workspace.as_ref().map(|binding| binding.root_path())
    }

    pub fn is_remote(&self) -> bool {
        self.workspace.as_ref().map(|ws| ws.is_remote()).unwrap_or(false)
    }

    /// Whether the session primary model accepts image inputs (from tool-definition / pipeline context).
    /// Defaults to **true** when unset (e.g. API listings without model metadata).
    pub fn primary_model_supports_image_understanding(&self) -> bool {
        self.custom_data
            .get("primary_model_supports_image_understanding")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    }

    pub fn cancellation_token(&self) -> Option<&CancellationToken> {
        self.runtime_handles.cancellation_token()
    }

    /// K.2.3 follow-up: returns the wired `ActorRuntime` (if any).
    /// `TaskTool::call` uses this to pass the runtime into
    /// `coordinator.execute_subagent` so the A1 gate can fire.
    pub fn actor_runtime(&self) -> Option<&Arc<northhing_agent_dispatch::ActorRuntime>> {
        self.actor_runtime.as_ref()
    }

    pub fn workspace_services(&self) -> Option<&WorkspaceServices> {
        self.runtime_handles.workspace_services()
    }

    pub fn for_tool_listing(
        workspace: Option<WorkspaceBinding>,
        workspace_services: Option<WorkspaceServices>,
    ) -> Self {
        Self {
            tool_call_id: None,
            agent_type: None,
            session_id: None,
            dialog_turn_id: None,
            workspace,
            unlocked_collapsed_tools: Vec::new(),
            custom_data: HashMap::new(),
            computer_use_host: None,
            runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
            runtime_handles: ToolRuntimeHandles::new(workspace_services, None),
            actor_runtime: None,
        }
    }
}

pub(crate) fn build_tool_use_context_for_task(
    task: &ToolTask,
    computer_use_host: Option<crate::agentic::tools::computer_use_host::ComputerUseHostRef>,
    cancellation_token: CancellationToken,
    actor_runtime: Option<Arc<northhing_agent_dispatch::ActorRuntime>>,
) -> ToolUseContext {
    build_tool_use_context_for_execution_context(
        &task.context,
        Some(task.tool_call.tool_id.clone()),
        computer_use_host,
        cancellation_token,
        actor_runtime,
    )
}

pub(crate) fn build_tool_use_context_for_execution_context(
    context: &ToolExecutionContext,
    tool_call_id: Option<String>,
    computer_use_host: Option<crate::agentic::tools::computer_use_host::ComputerUseHostRef>,
    cancellation_token: CancellationToken,
    actor_runtime: Option<Arc<northhing_agent_dispatch::ActorRuntime>>,
) -> ToolUseContext {
    ToolUseContext {
        tool_call_id,
        agent_type: Some(context.agent_type.clone()),
        session_id: Some(context.session_id.clone()),
        dialog_turn_id: Some(context.dialog_turn_id.clone()),
        workspace: context.workspace.clone(),
        unlocked_collapsed_tools: context.unlocked_collapsed_tools.clone(),
        custom_data: build_tool_context_custom_data(context),
        computer_use_host,
        runtime_handles: ToolRuntimeHandles::new(context.workspace_services.clone(), Some(cancellation_token)),
        runtime_tool_restrictions: context.runtime_tool_restrictions.clone(),
        actor_runtime,
    }
}

pub(crate) fn build_tool_description_context(
    agent_type: &str,
    workspace: Option<&WorkspaceBinding>,
    workspace_services: Option<&WorkspaceServices>,
    primary_supports_image_understanding: bool,
    context_vars: &HashMap<String, String>,
    actor_runtime: Option<Arc<northhing_agent_dispatch::ActorRuntime>>,
) -> ToolUseContext {
    let mut custom_data = HashMap::new();
    custom_data.insert(
        "primary_model_supports_image_understanding".to_string(),
        Value::Bool(primary_supports_image_understanding),
    );
    for (key, value) in context_vars {
        custom_data.insert(key.clone(), Value::String(value.clone()));
    }

    ToolUseContext {
        tool_call_id: None,
        agent_type: Some(agent_type.to_string()),
        session_id: None,
        dialog_turn_id: None,
        workspace: workspace.cloned(),
        unlocked_collapsed_tools: Vec::new(),
        custom_data,
        computer_use_host: None,
        runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
        runtime_handles: ToolRuntimeHandles::new(workspace_services.cloned(), None),
        actor_runtime,
    }
}

fn build_tool_context_custom_data(context: &ToolExecutionContext) -> HashMap<String, Value> {
    let mut map = HashMap::new();

    map.insert(
        "delegation_allow_subagent_spawn".to_string(),
        serde_json::json!(context.delegation_policy.allow_subagent_spawn),
    );
    map.insert(
        "delegation_nesting_depth".to_string(),
        serde_json::json!(context.delegation_policy.nesting_depth),
    );

    if let Some(turn_index) = context.context_vars.get("turn_index") {
        if let Ok(n) = turn_index.parse::<u64>() {
            map.insert("turn_index".to_string(), serde_json::json!(n));
        }
    }

    if let Some(provider) = context.context_vars.get("primary_model_provider") {
        if !provider.is_empty() {
            map.insert("primary_model_provider".to_string(), serde_json::json!(provider));
        }
    }
    if let Some(supports_images) = context.context_vars.get("primary_model_supports_image_understanding") {
        if let Ok(flag) = supports_images.parse::<bool>() {
            map.insert(
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
    if let Some(remote_file_delivery) = context.context_vars.get(TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY) {
        if let Ok(flag) = remote_file_delivery.parse::<bool>() {
            map.insert(
                TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY.to_string(),
                serde_json::json!(flag),
            );
        }
    }

    let deep_review_parent_context =
        context
            .subagent_parent_info
            .as_ref()
            .map(|parent_info| tool_context::DeepReviewToolParentContext {
                tool_call_id: parent_info.tool_call_id.as_str(),
                session_id: parent_info.session_id.as_str(),
                dialog_turn_id: parent_info.dialog_turn_id.as_str(),
            });
    tool_context::append_tool_use_context_data(&context.context_vars, deep_review_parent_context, &mut map);

    map
}
