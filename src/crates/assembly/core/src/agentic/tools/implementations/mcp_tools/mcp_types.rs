//! Shared rendering helpers for MCP resource/prompt tools.

use crate::service::mcp::protocol::{MCPPrompt, MCPResource, MCPResourceContent};
use crate::util::errors::NortHingError;

pub(super) const DEFAULT_RENDER_CHAR_LIMIT: usize = 32_000;

pub(super) fn tool_error(message: impl Into<String>) -> NortHingError {
    NortHingError::tool(message.into())
}

pub(super) fn truncate_text(text: &str, max_chars: usize) -> (String, bool) {
    let truncated = text.chars().count() > max_chars;
    let rendered = if truncated {
        text.chars().take(max_chars).collect()
    } else {
        text.to_string()
    };
    (rendered, truncated)
}

pub(super) fn render_resource_catalog(resources: &[MCPResource]) -> String {
    if resources.is_empty() {
        return "No MCP resources available.".to_string();
    }

    resources
        .iter()
        .map(|resource| {
            let mut lines = vec![format!(
                "- {} ({})",
                resource.title.as_deref().unwrap_or(&resource.name),
                resource.uri
            )];
            if resource.title.as_deref() != Some(resource.name.as_str()) {
                lines.push(format!("  Name: {}", resource.name));
            }
            if let Some(description) = &resource.description {
                lines.push(format!("  Description: {}", description));
            }
            if let Some(mime_type) = &resource.mime_type {
                lines.push(format!("  MIME type: {}", mime_type));
            }
            if let Some(size) = resource.size {
                lines.push(format!("  Size: {} bytes", size));
            }
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub(super) fn render_resource_contents(contents: &[MCPResourceContent], max_chars: usize) -> String {
    let mut rendered = String::new();
    let mut remaining = max_chars;
    let mut truncated_any = false;

    for (index, content) in contents.iter().enumerate() {
        if index > 0 {
            rendered.push_str("\n\n---\n\n");
        }

        rendered.push_str(&format!("Resource URI: {}", content.uri));
        if let Some(mime_type) = &content.mime_type {
            rendered.push_str(&format!("\nMIME type: {}", mime_type));
        }

        if let Some(text) = &content.content {
            let slice_limit = remaining.max(1);
            let (text_chunk, truncated) = truncate_text(text, slice_limit);
            rendered.push_str("\n\n");
            rendered.push_str(&text_chunk);
            truncated_any |= truncated;
            remaining = remaining.saturating_sub(text_chunk.chars().count());
        } else if content.blob.is_some() {
            rendered.push_str("\n\n[Binary resource content omitted]");
        } else {
            rendered.push_str("\n\n[Empty resource content]");
        }

        if remaining == 0 {
            truncated_any = true;
            break;
        }
    }

    if truncated_any {
        rendered.push_str("\n\n[Output truncated after reaching the MCP resource tool size limit.]");
    }

    rendered
}

pub(super) fn render_prompt_catalog(prompts: &[MCPPrompt]) -> String {
    if prompts.is_empty() {
        return "No MCP prompts available.".to_string();
    }

    prompts
        .iter()
        .map(|prompt| {
            let mut lines = vec![format!("- {}", prompt.title.as_deref().unwrap_or(&prompt.name))];
            if prompt.title.as_deref() != Some(prompt.name.as_str()) {
                lines.push(format!("  Name: {}", prompt.name));
            }
            if let Some(description) = &prompt.description {
                lines.push(format!("  Description: {}", description));
            }
            if let Some(arguments) = &prompt.arguments {
                if !arguments.is_empty() {
                    let args = arguments
                        .iter()
                        .map(|argument| {
                            let required = if argument.required { "required" } else { "optional" };
                            match &argument.description {
                                Some(description) => {
                                    format!("{} ({}, {})", argument.name, required, description)
                                }
                                None => format!("{} ({})", argument.name, required),
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    lines.push(format!("  Arguments: {}", args));
                }
            }
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
