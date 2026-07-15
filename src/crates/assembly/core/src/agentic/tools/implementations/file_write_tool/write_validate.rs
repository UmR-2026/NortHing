use super::write_error::preflight_write_error;
use crate::agentic::tools::file_tool_guidance::is_file_tool_guidance_message;
use crate::agentic::tools::framework::{ToolUseContext, ValidationResult};
use serde_json::{json, Value};
use tool_runtime::fs::WriteLocalFileMode;

const LARGE_WRITE_SOFT_LINE_LIMIT: usize = 200;
const LARGE_WRITE_SOFT_BYTE_LIMIT: usize = 20 * 1024;

pub fn parse_mode_value(mode: Option<&str>) -> Result<WriteLocalFileMode, String> {
    match mode.unwrap_or("w") {
        "w" => Ok(WriteLocalFileMode::Write),
        "a" => Ok(WriteLocalFileMode::Append),
        other => Err(format!(
            "mode must be either 'w' (overwrite) or 'a' (append), got '{}'",
            other
        )),
    }
}

pub fn input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "file_path": {
                "type": "string",
                "description": "The absolute path to the file to write (must be absolute, not relative)"
            },
            "content": {
                "type": "string",
                "description": "The content to write to the file"
            },
            "mode": {
                "type": "string",
                "enum": ["w", "a"],
                "description": "Write mode: 'w' overwrites the file (default), 'a' appends to the file and creates it if missing"
            }
        },
        "required": ["file_path", "content"],
        "additionalProperties": false
    })
}

pub fn description() -> String {
    r#"Writes a file to the local filesystem.

Usage:
- Always output `file_path` first when calling this tool. Example: `{"file_path": "src/main.rs", "content": "fn main() {}"}`.
- This tool defaults to `mode="w"`, which overwrites the existing file if there is one at the provided path.
- Use `mode="a"` to append content; it also creates the file if it does not exist.
- If this is an existing file, you MUST use the Read tool first to read the file's contents. This tool will fail if you did not read the file first.
- Prefer the Edit tool for modifying existing files — it only sends the diff. Only use this tool to create new files or for complete rewrites.
- NEVER create documentation files (*.md) or README files unless explicitly requested by the User.
- Only use emojis if the user explicitly requests it. Avoid writing emojis to files unless asked."#
        .to_string()
}

pub async fn validate_input(input: &Value, context: Option<&ToolUseContext>) -> ValidationResult {
    let file_path = match input.get("file_path").and_then(|v| v.as_str()) {
        Some(path) if !path.is_empty() => path,
        _ => {
            return ValidationResult {
                result: false,
                message: Some("file_path is required and cannot be empty".to_string()),
                error_code: Some(400),
                meta: None,
            };
        }
    };

    if input.get("content").is_none() {
        return ValidationResult {
            result: false,
            message: Some("content is required".to_string()),
            error_code: Some(400),
            meta: None,
        };
    }

    if let Err(message) = parse_mode_value(input.get("mode").and_then(|value| value.as_str())) {
        return ValidationResult {
            result: false,
            message: Some(message),
            error_code: Some(400),
            meta: None,
        };
    }

    let large_write_warning = input.get("content").and_then(|v| v.as_str()).and_then(|content| {
        let line_count = content.lines().count();
        let byte_count = content.len();
        if line_count > LARGE_WRITE_SOFT_LINE_LIMIT || byte_count > LARGE_WRITE_SOFT_BYTE_LIMIT {
            Some((line_count, byte_count))
        } else {
            None
        }
    });

    if let Some(ctx) = context {
        if let Some(message) = preflight_write_error(ctx, file_path).await {
            let is_guidance = is_file_tool_guidance_message(&message);
            return ValidationResult {
                result: false,
                message: Some(message),
                error_code: Some(400),
                meta: is_guidance.then(|| json!({ "failure_kind": "guidance" })),
            };
        }
    }

    if let Some((line_count, byte_count)) = large_write_warning {
        return ValidationResult {
            result: true,
            message: Some(format!(
                "Large Write payload: {} lines, {} bytes. This is allowed when necessary, but prefer a staged approach: for existing files use Read + focused Edit calls; for large new files write a stable scaffold first, then add sections in follow-up edits unless a complete initial body is required.",
                line_count, byte_count
            )),
            error_code: None,
            meta: Some(json!({
                "large_write": true,
                "line_count": line_count,
                "byte_count": byte_count,
                "soft_line_limit": LARGE_WRITE_SOFT_LINE_LIMIT,
                "soft_byte_limit": LARGE_WRITE_SOFT_BYTE_LIMIT
            })),
        };
    }

    ValidationResult::default()
}
