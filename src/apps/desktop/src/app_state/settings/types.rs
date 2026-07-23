use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

// ===== Provider =====

/// LLM provider type. Spec §5.6 (5 variants).
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

impl ProviderType {
    /// Default endpoint for the provider, when not user-overridden.
    pub fn default_base_url(&self) -> &'static str {
        match self {
            Self::Anthropic => "https://api.anthropic.com",
            Self::Openai => "https://api.openai.com/v1",
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Self::CustomOpenaiCompatible | Self::CustomAnthropicCompatible => "",
        }
    }

    /// Curated list of common models for the dropdown. Empty for `Custom*`
    /// variants (user must type the model name).
    pub fn default_models(&self) -> &'static [&'static str] {
        match self {
            Self::Anthropic => &["claude-sonnet-4-5", "claude-opus-4", "claude-haiku-4"],
            Self::Openai => &["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo"],
            Self::Gemini => &["gemini-2.0-flash", "gemini-1.5-pro"],
            Self::CustomOpenaiCompatible | Self::CustomAnthropicCompatible => &[],
        }
    }

    /// Slint-friendly display label (Chinese — matches AppStrings convention).
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::Openai => "OpenAI",
            Self::Gemini => "Gemini",
            Self::CustomOpenaiCompatible => "自定义 (OpenAI 兼容)",
            Self::CustomAnthropicCompatible => "自定义 (Anthropic 兼容)",
        }
    }
}

/// Single LLM provider entry.
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

impl ProviderConfig {
    pub fn new(name: String, provider_type: ProviderType) -> Self {
        let id = Uuid::new_v4().to_string();
        let base_url = provider_type.default_base_url().to_string();
        let model = provider_type
            .default_models()
            .first()
            .copied()
            .unwrap_or("")
            .to_string();
        Self {
            id,
            name,
            provider_type,
            base_url,
            api_key: String::new(),
            model,
            enabled: true,
            created_at: super::now_unix_secs(),
            last_verified_at: None,
            last_verified_ok: None,
        }
    }
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

// ===== Skill =====

/// Per-skill enable state. One entry per discovered builtin skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillState {
    /// Matches the folder name under `crates/assembly/core/builtin_skills/`.
    pub name: String,
    /// Default true; toggleable globally.
    pub global_enabled: bool,
    /// Per-workspace overrides (Q5 = E2 = c: global + per-workspace).
    /// Lookup uses `PathBuf` as key; serialization uses the path string.
    #[serde(with = "pathbuf_map_serde")]
    pub workspace_overrides: HashMap<PathBuf, bool>,
}

impl SkillState {
    /// Effective enable state for a given workspace: workspace override wins,
    /// otherwise fall back to global, otherwise default-on (true).
    pub fn effective_in(&self, workspace: &Path) -> bool {
        self.workspace_overrides
            .get(workspace)
            .copied()
            .unwrap_or(self.global_enabled)
    }
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
    /// `command` for stdio transports (e.g. `npx`, `node`).
    pub command: Option<String>,
    pub args: Vec<String>,
    /// `url` for SSE / StreamableHttp transports.
    pub url: Option<String>,
    /// Environment variables for the stdio subprocess.
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub last_verified_at: Option<i64>,
    pub last_verified_ok: Option<bool>,
    /// Tool names returned by the last successful `tools/list`.
    pub last_tools: Vec<String>,
}

impl MCPServerConfig {
    pub fn new(name: String, transport: MCPTransport) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            transport,
            enabled: true,
            command: None,
            args: Vec::new(),
            url: None,
            env: HashMap::new(),
            last_verified_at: None,
            last_verified_ok: None,
            last_tools: Vec::new(),
        }
    }
}

// ===== Default model =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRef {
    pub provider_id: String,
    pub model: String,
}

// `serde(default)` workaround: HashMap<PathBuf, V> requires a custom
// serializer for PathBuf keys (which serialize as strings on platforms
// where OsStr is valid UTF-8). We only target Windows + macOS + Linux in
// this crate and workspace paths are always UTF-8 in practice, so a
// string round-trip is safe.
mod pathbuf_map_serde {
    use serde::de::{MapAccess, Visitor};
    use serde::ser::SerializeMap;
    use serde::{Deserializer, Serializer};
    use std::collections::HashMap;
    use std::path::PathBuf;

    pub fn serialize<S, V>(map: &HashMap<PathBuf, V>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: serde::Serialize,
    {
        let mut ser = s.serialize_map(Some(map.len()))?;
        for (k, v) in map {
            let key_str = k.to_string_lossy().into_owned();
            ser.serialize_entry(&key_str, v)?;
        }
        ser.end()
    }

    pub fn deserialize<'de, D, V>(d: D) -> Result<HashMap<PathBuf, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: serde::Deserialize<'de>,
    {
        struct V<V2>(std::marker::PhantomData<V2>);
        impl<'de, V2> Visitor<'de> for V<V2>
        where
            V2: serde::Deserialize<'de>,
        {
            type Value = HashMap<PathBuf, V2>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a map of path string -> value")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut out = HashMap::new();
                while let Some((k, v)) = access.next_entry::<String, V2>()? {
                    out.insert(PathBuf::from(k), v);
                }
                Ok(out)
            }
        }
        d.deserialize_map(V(std::marker::PhantomData))
    }
}
