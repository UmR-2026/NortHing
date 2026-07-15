//! Convert northhing `ToolDefinition` values into Gemini native tools
//! (e.g. `googleSearch`, `urlContext`, `codeExecution`) and/or
//! `functionDeclarations`.
//!
//! The main entry point is [`convert_tools`]. Helpers in this file are
//! module-private — they only support tool conversion.
//!
//! Schema sanitization is delegated to [`super::schema_sanitizer`].

use crate::types::ToolDefinition;
use serde_json::{json, Value};

use super::schema_sanitizer;

/// Convert northhing tool definitions into Gemini native tools and/or
/// function declarations.
///
/// Gemini providers such as AIHubMix reject requests that mix built-in
/// tools with custom function declarations. When custom tools are present,
/// keep all tools in function-calling mode so northhing's local tool
/// pipeline still works.
pub fn convert_tools(tools: Option<Vec<ToolDefinition>>) -> Option<Vec<Value>> {
    tools.and_then(|tool_defs| {
        let mut native_tools = Vec::new();
        let mut custom_tools = Vec::new();

        for tool in tool_defs {
            if let Some(native_tool) = convert_native_tool(&tool) {
                native_tools.push(native_tool);
            } else {
                custom_tools.push(tool);
            }
        }

        let should_fallback_to_function_calling = !native_tools.is_empty() && !custom_tools.is_empty();

        let declarations: Vec<Value> = if should_fallback_to_function_calling {
            custom_tools
                .into_iter()
                .chain(
                    native_tools
                        .iter()
                        .cloned()
                        .filter_map(convert_native_tool_to_custom_definition),
                )
                .map(convert_custom_tool)
                .collect()
        } else {
            custom_tools.into_iter().map(convert_custom_tool).collect()
        };

        let mut result_tools = if should_fallback_to_function_calling {
            Vec::new()
        } else {
            native_tools
        };

        if !declarations.is_empty() {
            result_tools.push(json!({
                "functionDeclarations": declarations,
            }));
        }

        if result_tools.is_empty() {
            None
        } else {
            Some(result_tools)
        }
    })
}

fn convert_native_tool(tool: &ToolDefinition) -> Option<Value> {
    let native_name = native_tool_name(&tool.name)?;
    let config = native_tool_config(&tool.parameters);
    Some(json!({
        native_name: config,
    }))
}

fn convert_native_tool_to_custom_definition(native_tool: Value) -> Option<ToolDefinition> {
    let map = native_tool.as_object()?;
    let (name, _config) = map.iter().next()?;

    Some(ToolDefinition {
        name: native_tool_fallback_name(name).to_string(),
        description: native_tool_fallback_description(name).to_string(),
        parameters: native_tool_fallback_schema(name),
    })
}

fn convert_custom_tool(tool: ToolDefinition) -> Value {
    let parameters = schema_sanitizer::sanitize_schema(tool.parameters);
    json!({
        "name": tool.name,
        "description": tool.description,
        "parameters": parameters,
    })
}

fn native_tool_name(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "WebSearch" | "googleSearch" | "GoogleSearch" => Some("googleSearch"),
        "WebFetch" | "urlContext" | "UrlContext" | "URLContext" => Some("urlContext"),
        "googleSearchRetrieval" | "GoogleSearchRetrieval" => Some("googleSearchRetrieval"),
        "codeExecution" | "CodeExecution" => Some("codeExecution"),
        _ => None,
    }
}

fn native_tool_fallback_name(native_name: &str) -> &'static str {
    match native_name {
        "googleSearch" => "WebSearch",
        "urlContext" => "WebFetch",
        "googleSearchRetrieval" => "googleSearchRetrieval",
        "codeExecution" => "codeExecution",
        _ => "unknown_native_tool",
    }
}

fn native_tool_fallback_description(native_name: &str) -> &'static str {
    match native_name {
        "googleSearch" => "Search the web for up-to-date information.",
        "urlContext" => "Fetch content from a URL for context.",
        "googleSearchRetrieval" => "Retrieve grounded results from Google Search.",
        "codeExecution" => "Execute model-generated code and return the result.",
        _ => "Gemini native tool fallback.",
    }
}

fn native_tool_fallback_schema(native_name: &str) -> Value {
    match native_name {
        "googleSearch" | "googleSearchRetrieval" => json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                }
            },
            "required": ["query"]
        }),
        "urlContext" => json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                }
            },
            "required": ["url"]
        }),
        "codeExecution" => json!({
            "type": "object",
            "properties": {}
        }),
        _ => json!({
            "type": "object",
            "properties": {}
        }),
    }
}

fn native_tool_config(parameters: &Value) -> Value {
    if looks_like_schema(parameters) {
        json!({})
    } else {
        match parameters {
            Value::Object(map) if !map.is_empty() => parameters.clone(),
            _ => json!({}),
        }
    }
}

fn looks_like_schema(parameters: &Value) -> bool {
    let Some(map) = parameters.as_object() else {
        return false;
    };

    map.contains_key("type")
        || map.contains_key("properties")
        || map.contains_key("required")
        || map.contains_key("$schema")
        || map.contains_key("items")
        || map.contains_key("allOf")
        || map.contains_key("anyOf")
        || map.contains_key("oneOf")
        || map.contains_key("enum")
        || map.contains_key("nullable")
        || map.contains_key("format")
}
