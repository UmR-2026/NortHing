use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ===== Provider =====

/// LLM provider type. Spec §5.6 (5 variants).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderType {
    /// `base_url` defaults to `https://api.anthropic.com`
    Anthropic,
    /// `base_url` defaults to `https://api.openai.com/v1`
    Openai,
    /// `base_url` defaults to `https://generativelanguage.googleapis.com/v1beta`
    Gemini,
    /// User provides `base_url`. Uses the OpenAI HTTP shape.
    CustomOpenaiCompatible,
    /// User provides `base_url`. Uses the Anthropic Messages HTTP shape.
    CustomAnthropicCompatible,
}

/// Single LLM provider entry.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// UUID v4, immutable. Used as the canonical handle.
    pub id: String,
    /// User-facing label, e.g. "我的 Anthropic".
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: ProviderType,
    /// Auto-filled from `ProviderType::default_base_url`; user-editable.
    pub base_url: String,
    /// Stored in plaintext in app.json. Never logged.
    pub api_key: String,
    /// Either a value from `ProviderType::default_models` or a user-typed
    /// custom model name (B4 = c: dropdown + custom).
    pub model: String,
    pub enabled: bool,
    /// Unix seconds, used for sort order in the UI list.
    pub created_at: i64,
    /// Last time `test_provider` succeeded.
    pub last_verified_at: Option<i64>,
    /// True = verified, false = test failed (UI shows ⚠️), None = never tested.
    pub last_verified_ok: Option<bool>,
}

// ===== Workspace =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    pub path: PathBuf,
    /// Defaults to folder basename; user-editable.
    pub display_name: String,
    pub added_at: i64,
    pub last_opened_at: i64,
    /// Path to the `IDENTITY.md` file if one exists in the workspace root.
    /// `None` means no IDENTITY.md yet (D3 = a may auto-create one).
    pub identity_md_path: Option<PathBuf>,
}

// ===== MCP Server =====

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum MCPTransport {
    Stdio,
    Sse,
    StreamableHttp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    pub id: String,
    pub name: String,
    pub transport: MCPTransport,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Environment variables for the stdio subprocess.
    ///
    /// On disk in user-level `app.json`, sensitive env variables are replaced
    /// with the keyring sentinel `__kr_env__` and stored in the OS keyring
    /// under `mcp-env:{id}` (P1c, P1-8).
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verified_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verified_ok: Option<bool>,
    #[serde(default)]
    pub last_tools: Vec<String>,
}
