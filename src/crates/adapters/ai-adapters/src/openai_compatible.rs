//! OpenAI-compatible provider adapter.
//!
//! This adapter supports any API that implements the OpenAI chat completions protocol.
//! Simply configure base_url, api_key, and model_id to use a new provider.

/// Configuration for an OpenAI-compatible provider
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpenAICompatibleConfig {
    /// Provider identifier (e.g., "openai", "anthropic", "ollama", "custom")
    pub provider_id: String,
    /// Human-readable name
    pub display_name: String,
    /// Base URL for API requests (e.g., "https://api.openai.com/v1")
    pub base_url: String,
    /// API key (optional for local providers like Ollama)
    pub api_key: Option<String>,
    /// Default model ID
    pub default_model: String,
    /// Available models
    #[serde(default)]
    pub models: Vec<String>,
    /// Whether this provider supports streaming
    #[serde(default = "default_true")]
    pub supports_streaming: bool,
    /// Whether this provider supports tool calls
    #[serde(default = "default_true")]
    pub supports_tools: bool,
    /// Whether this provider supports vision
    #[serde(default)]
    pub supports_vision: bool,
    /// Custom headers to add to requests
    #[serde(default)]
    pub custom_headers: Vec<(String, String)>,
}

fn default_true() -> bool {
    true
}

impl OpenAICompatibleConfig {
    /// Create a config for OpenAI
    pub fn openai(api_key: String) -> Self {
        Self {
            provider_id: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: Some(api_key),
            default_model: "gpt-4o".to_string(),
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
            ],
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
            custom_headers: vec![],
        }
    }

    /// Create a config for Anthropic (OpenAI-compatible endpoint)
    pub fn anthropic(api_key: String) -> Self {
        Self {
            provider_id: "anthropic".to_string(),
            display_name: "Anthropic".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            api_key: Some(api_key),
            default_model: "claude-3-5-sonnet-20241022".to_string(),
            models: vec![
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-5-haiku-20241022".to_string(),
                "claude-3-opus-20240229".to_string(),
            ],
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
            custom_headers: vec![],
        }
    }

    /// Create a config for Ollama (local)
    pub fn ollama() -> Self {
        Self {
            provider_id: "ollama".to_string(),
            display_name: "Ollama (Local)".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            api_key: None,
            default_model: "llama3".to_string(),
            models: vec![],
            supports_streaming: true,
            supports_tools: true,
            supports_vision: false,
            custom_headers: vec![],
        }
    }

    /// Create a custom provider config
    pub fn custom(
        provider_id: String,
        display_name: String,
        base_url: String,
        api_key: Option<String>,
        default_model: String,
    ) -> Self {
        Self {
            provider_id,
            display_name,
            base_url,
            api_key,
            default_model,
            models: vec![],
            supports_streaming: true,
            supports_tools: true,
            supports_vision: false,
            custom_headers: vec![],
        }
    }
}

/// Provider registry that manages multiple OpenAI-compatible providers
#[derive(Debug, Clone, Default)]
pub struct ProviderRegistry {
    providers: Vec<OpenAICompatibleConfig>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { providers: vec![] }
    }

    /// Add a provider to the registry
    pub fn add_provider(&mut self, config: OpenAICompatibleConfig) {
        self.providers.push(config);
    }

    /// Get a provider by ID
    pub fn get_provider(&self, provider_id: &str) -> Option<&OpenAICompatibleConfig> {
        self.providers.iter().find(|p| p.provider_id == provider_id)
    }

    /// List all providers
    pub fn list_providers(&self) -> &[OpenAICompatibleConfig] {
        &self.providers
    }

    /// Create default registry with common providers
    pub fn with_defaults() -> Self {
        Self::new()
    }
}
