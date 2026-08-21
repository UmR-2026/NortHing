use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

// ===== Default model =====

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRef {
    pub provider_id: String,
    pub model: String,
}
