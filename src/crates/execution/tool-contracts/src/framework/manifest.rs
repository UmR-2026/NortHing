//! Tool manifest, exposure policy, and GetToolSpec contract helpers.
//!
//! R37b sibling: manifest definitions, exposure policy resolution, collapsed
//! stub building, GetToolSpec schema/validation/execution-plan helpers, and
//! prompt-visible manifest sorting. Split verbatim from `framework.rs`.

use super::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};
use std::fmt;

pub const GET_TOOL_SPEC_TOOL_NAME: &str = "GetToolSpec";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolManifestDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl ToolManifestDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>, parameters: Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PromptVisibleToolManifestItem {
    Expanded(ToolManifestDefinition),
    Collapsed { name: String, short_description: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolManifestPolicyTool {
    pub name: String,
    pub default_exposure: ToolExposure,
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolManifestPolicyResolution {
    pub allowed_tool_names: Vec<String>,
    pub expanded_tool_names: Vec<String>,
    pub collapsed_tool_names: Vec<String>,
}

#[derive(Clone)]
pub struct ContextualVisibleTools<Tool: ?Sized> {
    pub allowed_tool_names: Vec<String>,
    pub expanded_tools: Vec<ToolRef<Tool>>,
    pub collapsed_tool_names: Vec<String>,
    pub collapsed_tools: Vec<ToolRef<Tool>>,
}

#[derive(Clone)]
pub struct ContextualToolManifest<Tool: ?Sized> {
    pub allowed_tool_names: Vec<String>,
    pub expanded_tools: Vec<ToolRef<Tool>>,
    pub collapsed_tool_names: Vec<String>,
    pub collapsed_tools: Vec<ToolRef<Tool>>,
    pub tool_definitions: Vec<ToolManifestDefinition>,
}

pub fn resolve_tool_manifest_policy(
    tool_snapshot: &[ToolManifestPolicyTool],
    allowed_tools: &[String],
    exposure_overrides: &IndexMap<String, ToolExposure>,
    get_tool_spec_tool_name: &str,
) -> ToolManifestPolicyResolution {
    let allowed_set = allowed_tools
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    let mut allowed_tool_names = allowed_tools.to_vec();
    let mut expanded_tool_names = Vec::new();
    let mut collapsed_tool_names = Vec::new();

    for tool in tool_snapshot {
        if !tool.available || !allowed_set.contains(tool.name.as_str()) {
            continue;
        }

        let exposure = exposure_overrides
            .get(&tool.name)
            .copied()
            .unwrap_or(tool.default_exposure);
        match exposure {
            ToolExposure::Expanded => expanded_tool_names.push(tool.name.clone()),
            ToolExposure::Collapsed => collapsed_tool_names.push(tool.name.clone()),
        }
    }

    if !collapsed_tool_names.is_empty() {
        if !allowed_tool_names.iter().any(|name| name == get_tool_spec_tool_name) {
            allowed_tool_names.push(get_tool_spec_tool_name.to_string());
        }
        if tool_snapshot.iter().any(|tool| tool.name == get_tool_spec_tool_name) {
            expanded_tool_names.push(get_tool_spec_tool_name.to_string());
        }
    }

    ToolManifestPolicyResolution {
        allowed_tool_names,
        expanded_tool_names,
        collapsed_tool_names,
    }
}

pub fn build_tool_manifest_policy_tools<Tool: ToolRegistryItem + ?Sized>(
    tool_snapshot: &[ToolRef<Tool>],
    available_tool_names: &HashSet<String>,
) -> Vec<ToolManifestPolicyTool> {
    tool_snapshot
        .iter()
        .map(|tool| {
            let name = tool.name().to_string();
            ToolManifestPolicyTool {
                available: available_tool_names.contains(&name),
                default_exposure: tool.default_exposure(),
                name,
            }
        })
        .collect()
}

pub fn build_collapsed_tool_stub_definition(tool_name: &str, short_description: &str) -> ToolManifestDefinition {
    // Keep the prompt-visible stub stable for the life of the conversation.
    // GetToolSpec returns the full schema out-of-band; replacing this stub with
    // a different tool definition mid-session changes the request prefix and
    // causes provider-side prefix/KV cache misses on later rounds.
    // We still need a stub definition in the request because some providers
    // constrain model tool calls to the exact tool list attached to that
    // request. Without a prompt-visible stub entry, the model may be unable to
    // call the collapsed tool at all, even after GetToolSpec has described it.
    ToolManifestDefinition::new(
        tool_name,
        format!(
            "THIS IS A COLLAPSED TOOL. Before first use, call GetToolSpec({{\"tool_name\":\"{}\"}}) to load its schema. After that, you can call {} directly. Any direct call before loading will fail validation.\nSummary: {}",
            tool_name,
            tool_name,
            short_description,
        ),
        serde_json::json!({
            "type": "object",
            "additionalProperties": true,
            "properties": {}
        }),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetToolSpecCollapsedToolSummary {
    pub name: String,
    pub short_description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GetToolSpecDetail {
    pub tool_name: String,
    pub description: String,
    pub input_schema: Value,
}

impl GetToolSpecDetail {
    pub fn to_value(&self) -> Value {
        serde_json::json!({
            "tool_name": self.tool_name.clone(),
            "description": self.description.clone(),
            "input_schema": self.input_schema.clone(),
        })
    }
}

pub fn tool_spec_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["tool_name"],
        "properties": {
            "tool_name": {
                "type": "string",
                "description": "Exact collapsed tool name to load, using the tool's canonical casing from the catalog (for example, \"Git\"). Do not pass a command such as \"git status\" or an operation such as \"status\" here."
            }
        }
    })
}

pub fn tool_spec_short_description() -> String {
    "Discover collapsed tools and read their detailed definitions.".to_string()
}

pub fn build_get_tool_spec_description() -> String {
    r#"Read full schema before first calling a collapsed tool.

Do not call GetToolSpec again for a tool whose definition is already loaded in the current conversation."#
        .to_string()
}

pub fn build_get_tool_spec_catalog_description(collapsed_tools: &[GetToolSpecCollapsedToolSummary]) -> Option<String> {
    if collapsed_tools.is_empty() {
        return None;
    }

    let collapsed_tools_list = collapsed_tools
        .iter()
        .map(|tool| format!("- {}", tool.name))
        .collect::<Vec<_>>()
        .join("\n");

    Some(format!(
        "<collapsed_tools>\n{}\n</collapsed_tools>",
        collapsed_tools_list
    ))
}

pub fn tool_spec_is_readonly() -> bool {
    true
}

pub fn get_tool_spec_is_concurrency_safe(_input: Option<&Value>) -> bool {
    true
}

pub fn get_tool_spec_needs_permissions(_input: Option<&Value>) -> bool {
    false
}

pub fn render_get_tool_spec_tool_use_message(input: &Value) -> String {
    let tool_name = input.get("tool_name").and_then(|value| value.as_str()).unwrap_or("?");
    format!("Reading tool spec for '{}'.", tool_name)
}

pub fn validate_get_tool_spec_input(input: &Value) -> ValidationResult {
    let Some(tool_name) = input.get("tool_name").and_then(|value| value.as_str()) else {
        return ValidationResult {
            result: false,
            message: Some("tool_name is required and cannot be empty".to_string()),
            error_code: Some(400),
            meta: None,
        };
    };

    if tool_name.is_empty() {
        return ValidationResult {
            result: false,
            message: Some("tool_name is required and cannot be empty".to_string()),
            error_code: Some(400),
            meta: None,
        };
    }

    ValidationResult::default()
}

pub fn build_get_tool_spec_duplicate_load_hint(tool_name: &str) -> String {
    format!(
        "Tool '{}' is already loaded in the current conversation. Do not call GetToolSpec again for it. Use '{}' directly.",
        tool_name, tool_name
    )
}

pub fn build_get_tool_spec_duplicate_load_result(tool_name: &str) -> ToolResult {
    ToolResult::Result {
        data: serde_json::json!({
            "tool_name": tool_name,
            "already_loaded": true
        }),
        result_for_assistant: Some(build_get_tool_spec_duplicate_load_hint(tool_name)),
        image_attachments: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetToolSpecExecutionError {
    MissingToolName,
    Detail(String),
}

impl fmt::Display for GetToolSpecExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GetToolSpecExecutionError::MissingToolName => write!(f, "tool_name is required"),
            GetToolSpecExecutionError::Detail(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for GetToolSpecExecutionError {}

#[derive(Debug, Clone)]
pub enum GetToolSpecExecutionPlan<'a> {
    DuplicateLoad(ToolResult),
    LoadDetail { tool_name: &'a str },
}

pub fn resolve_get_tool_spec_execution_plan<'a>(
    input: &'a Value,
    loaded_collapsed_tools: &[String],
) -> Result<GetToolSpecExecutionPlan<'a>, GetToolSpecExecutionError> {
    let tool_name = input
        .get("tool_name")
        .and_then(|value| value.as_str())
        .ok_or(GetToolSpecExecutionError::MissingToolName)?;

    if loaded_collapsed_tools.iter().any(|loaded| loaded == tool_name) {
        return Ok(GetToolSpecExecutionPlan::DuplicateLoad(
            build_get_tool_spec_duplicate_load_result(tool_name),
        ));
    }

    Ok(GetToolSpecExecutionPlan::LoadDetail { tool_name })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetToolSpecLoadObservation<'a> {
    pub tool_name: &'a str,
    pub loaded_tool_name: Option<&'a str>,
    pub is_error: bool,
}

pub fn collect_loaded_collapsed_tool_names(
    observations: &[GetToolSpecLoadObservation<'_>],
    collapsed_tool_names: &[String],
    get_tool_spec_tool_name: &str,
) -> Vec<String> {
    let collapsed_set: HashSet<&str> = collapsed_tool_names.iter().map(String::as_str).collect();
    let mut loaded = BTreeSet::new();

    for observation in observations {
        if observation.is_error || observation.tool_name != get_tool_spec_tool_name {
            continue;
        }

        let Some(tool_name) = observation.loaded_tool_name else {
            continue;
        };

        if collapsed_set.contains(tool_name) {
            loaded.insert(tool_name.to_string());
        }
    }

    loaded.into_iter().collect()
}

pub fn build_get_tool_spec_assistant_detail(description: &str, input_schema: &Value) -> String {
    format!(
        "<description>\n{}\n</description>\n<input_schema>\n{}\n</input_schema>",
        escape_get_tool_spec_xml_text(description),
        escape_get_tool_spec_xml_text(&input_schema.to_string())
    )
}

pub fn build_get_tool_spec_detail_result(detail: &GetToolSpecDetail) -> ToolResult {
    ToolResult::Result {
        data: detail.to_value(),
        result_for_assistant: Some(build_get_tool_spec_assistant_detail(
            &detail.description,
            &detail.input_schema,
        )),
        image_attachments: None,
    }
}

fn escape_get_tool_spec_xml_text(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub fn tool_manifest_sort_rank(tool_name: &str) -> usize {
    match tool_name {
        "Task" => 1,
        "Bash" => 2,
        "TerminalControl" => 3,
        "Glob" => 4,
        "Grep" => 5,
        "Read" => 6,
        "Edit" => 7,
        "Write" => 8,
        "Delete" => 9,
        "WebFetch" => 10,
        "WebSearch" => 11,
        "TodoWrite" => 12,
        "Skill" => 13,
        "Log" => 14,
        GET_TOOL_SPEC_TOOL_NAME => 15,
        "ControlHub" => 16,
        _ => 100,
    }
}

pub fn sort_tool_manifest_definitions(tool_definitions: &mut [ToolManifestDefinition]) {
    tool_definitions.sort_by_key(|tool| tool_manifest_sort_rank(&tool.name));
}

pub fn build_prompt_visible_tool_manifest_definitions(
    items: &[PromptVisibleToolManifestItem],
) -> Vec<ToolManifestDefinition> {
    let mut definitions = items
        .iter()
        .map(|item| match item {
            PromptVisibleToolManifestItem::Expanded(definition) => definition.clone(),
            PromptVisibleToolManifestItem::Collapsed {
                name,
                short_description,
            } => build_collapsed_tool_stub_definition(name, short_description),
        })
        .collect::<Vec<_>>();
    sort_tool_manifest_definitions(&mut definitions);
    definitions
}
