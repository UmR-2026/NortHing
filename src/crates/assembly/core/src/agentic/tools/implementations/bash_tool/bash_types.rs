use crate::util::errors::NortHingResult;
use serde_json::Value;
use terminal_core::shell::ShellType;

/// Result of shell resolution for bash tool
pub(crate) struct ResolvedShell {
    /// Shell type to use (None means use system default)
    pub(crate) shell_type: Option<ShellType>,
    /// Display name for the shell (for tool description)
    pub(crate) display_name: String,
}

pub(crate) fn json_object_metadata(value: Value) -> serde_json::Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    }
}
