//! Runtime / model-level configuration types: DebugModeConfig, LanguageDebugTemplate,
//! AgentSubagentOverrideState + type aliases, AIModelConfig + AuthConfig + Compat +
//! impl From + 2 inherent impls, `pub trait ConfigProvider`.
//!
//! Cross-sibling: imports `ModelCapability` + `ModelCategory` (ai sibling) for AIModelConfig.

use super::ai::{ModelCapability, ModelCategory, ReasoningMode};
use crate::util::errors::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Debug-mode configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DebugModeConfig {
    /// Custom log path (relative to the workspace; default: `.northhing/debug.log`).
    pub log_path: String,

    /// Ingest server port.
    pub ingest_port: u16,

    /// Enabled languages (auto-detected based on project type when empty).
    pub enabled_languages: Vec<String>,

    /// Debug template configuration per language.
    pub language_templates: HashMap<String, LanguageDebugTemplate>,
}

impl Default for DebugModeConfig {
    fn default() -> Self {
        Self {
            log_path: ".northhing/debug.log".to_string(),
            ingest_port: 7242,
            enabled_languages: Vec::new(),
            language_templates: Self::default_language_templates(),
        }
    }
}

impl DebugModeConfig {
    /// Returns the default language templates.
    ///
    /// Core languages (JavaScript) are enabled by default and cannot be disabled;
    /// they are included in the static prompt.
    /// Other languages (Python/Rust/Go/Java) are disabled by default and can be enabled as needed.
    pub fn default_language_templates() -> HashMap<String, LanguageDebugTemplate> {
        let mut templates = HashMap::new();

        templates.insert("javascript".to_string(), LanguageDebugTemplate {
            language: "javascript".to_string(),
            display_name: "JavaScript / TypeScript".to_string(),
            enabled: false,
            instrumentation_template: r#"fetch('http://127.0.0.1:{PORT}/ingest/{SESSION_ID}',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'{LOCATION}',message:'{MESSAGE}',data:{DATA},timestamp:Date.now(),sessionId:'{SESSION_ID}',hypothesisId:'{HYPOTHESIS_ID}',runId:'{RUN_ID}'})}).catch(()=>{});"#.to_string(),
            region_start: "// #region agent log".to_string(),
            region_end: "// #endregion".to_string(),
            notes: vec![
                "Send logs to the ingest server via HTTP POST.".to_string(),
                "{DATA} must be replaced with a JavaScript object expression.".to_string(),
            ],
        });

        templates.insert("python".to_string(), LanguageDebugTemplate {
            language: "python".to_string(),
            display_name: "Python".to_string(),
            enabled: false,
            instrumentation_template: r#"import json, time, os
with open(os.path.join(os.getcwd(), '{LOG_PATH}'), 'a', encoding='utf-8') as _f:
    _f.write(json.dumps({"location": "{LOCATION}", "message": "{MESSAGE}", "data": {DATA}, "timestamp": int(time.time()*1000), "sessionId": "{SESSION_ID}", "hypothesisId": "{HYPOTHESIS_ID}", "runId": "{RUN_ID}"}, ensure_ascii=False) + '\n')"#.to_string(),
            region_start: "# region agent log".to_string(),
            region_end: "# endregion".to_string(),
            notes: vec![
                "Append NDJSON logs directly to workspace LOG_PATH.".to_string(),
                "Use ensure_ascii=False to preserve non-ASCII characters.".to_string(),
                "{DATA} must be a Python expression (e.g., {\"var\": var} or locals()).".to_string(),
                "Imports only need to be declared once at the top.".to_string(),
            ],
        });

        templates.insert("rust".to_string(), LanguageDebugTemplate {
            language: "rust".to_string(),
            display_name: "Rust".to_string(),
            enabled: false,
            instrumentation_template: r##"{
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};
    if let Ok(mut _f) = OpenOptions::new().create(true).append(true).open("{LOG_PATH}") {
        let _ts = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        let _ = writeln!(_f, r#"{{"location":"{LOCATION}","message":"{MESSAGE}","data":{},"timestamp":{},"sessionId":"{SESSION_ID}","hypothesisId":"{HYPOTHESIS_ID}","runId":"{RUN_ID}"}}"#, serde_json::json!({DATA}), _ts);
    }
}"##.to_string(),
            region_start: "// #region agent log".to_string(),
            region_end: "// #endregion".to_string(),
            notes: vec![
                "Append NDJSON logs directly to LOG_PATH.".to_string(),
                "Requires serde_json: cargo add serde_json.".to_string(),
                "{DATA} must be a Rust expression (e.g., {\"var\": var}).".to_string(),
                "Use in sync code; for async code use tokio::fs.".to_string(),
            ],
        });

        templates.insert("go".to_string(), LanguageDebugTemplate {
            language: "go".to_string(),
            display_name: "Go".to_string(),
            enabled: false,
            instrumentation_template: r#"func() {
	f, err := os.OpenFile("{LOG_PATH}", os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err == nil {
		defer f.Close()
		data, _ := json.Marshal(map[string]interface{}{"location": "{LOCATION}", "message": "{MESSAGE}", "data": {DATA}, "timestamp": time.Now().UnixMilli(), "sessionId": "{SESSION_ID}", "hypothesisId": "{HYPOTHESIS_ID}", "runId": "{RUN_ID}"})
		f.Write(append(data, '\n'))
	}
}()"#.to_string(),
            region_start: "// #region agent log".to_string(),
            region_end: "// #endregion".to_string(),
            notes: vec![
                "Use an immediately-invoked anonymous function; can be inserted anywhere.".to_string(),
                "Append NDJSON logs directly to LOG_PATH.".to_string(),
                "Import \"os\", \"encoding/json\", and \"time\".".to_string(),
                "{DATA} must be a Go expression (e.g., map[string]interface{}{\"var\": var}).".to_string(),
            ],
        });

        templates.insert("java".to_string(), LanguageDebugTemplate {
            language: "java".to_string(),
            display_name: "Java".to_string(),
            enabled: false,
            instrumentation_template: r#"try {
    java.nio.file.Files.writeString(
        java.nio.file.Path.of("{LOG_PATH}"),
        String.format("{\"location\":\"{LOCATION}\",\"message\":\"{MESSAGE}\",\"data\":%s,\"timestamp\":%d,\"sessionId\":\"{SESSION_ID}\",\"hypothesisId\":\"{HYPOTHESIS_ID}\",\"runId\":\"{RUN_ID}\"}%n",
            new com.google.gson.Gson().toJson({DATA}), System.currentTimeMillis()),
        java.nio.file.StandardOpenOption.CREATE, java.nio.file.StandardOpenOption.APPEND);
} catch (Exception _e) { /* debug log */ }"#.to_string(),
            region_start: "// #region agent log".to_string(),
            region_end: "// #endregion".to_string(),
            notes: vec![
                "Append NDJSON logs directly to LOG_PATH.".to_string(),
                "Requires Gson (or use Jackson).".to_string(),
                "{DATA} must be a Java object (e.g., Map.of(\"var\", var)).".to_string(),
                "Java 11+ can use Files.writeString; older versions use Files.write + getBytes().".to_string(),
            ],
        });

        templates
    }

    /// Returns relevant templates based on detected project languages.
    pub fn get_templates_for_languages(&self, detected_languages: &[String]) -> Vec<&LanguageDebugTemplate> {
        let target_languages: Vec<&str> = if !self.enabled_languages.is_empty() {
            self.enabled_languages.iter().map(|s| s.as_str()).collect()
        } else {
            detected_languages.iter().map(|s| s.as_str()).collect()
        };

        let language_mapping: HashMap<&str, &str> = [
            ("typescript", "javascript"),
            ("javascript", "javascript"),
            ("python", "python"),
            ("rust", "rust"),
            ("go", "go"),
            ("java", "java"),
            ("kotlin", "java"),
        ]
        .into_iter()
        .collect();

        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for lang in &target_languages {
            let template_lang = language_mapping.get(lang).unwrap_or(lang);
            if !seen.contains(template_lang) {
                if let Some(template) = self.language_templates.get(*template_lang) {
                    if template.enabled {
                        result.push(template);
                        seen.insert(template_lang);
                    }
                }
            }
        }

        result
    }
}

/// Language debug template.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LanguageDebugTemplate {
    /// Language identifier (javascript, python, rust, go, java).
    pub language: String,

    /// Display name.
    pub display_name: String,

    /// Whether this language template is enabled (when enabled, user-defined templates override
    /// built-in logic).
    pub enabled: bool,

    /// Instrumentation code template.
    /// Placeholders: {LOCATION}, {MESSAGE}, {DATA}, {PORT}, {SESSION_ID}, {HYPOTHESIS_ID},
    /// {RUN_ID}, {LOG_PATH}
    pub instrumentation_template: String,

    /// Region marker start.
    pub region_start: String,

    /// Region marker end.
    pub region_end: String,

    /// Special notes.
    pub notes: Vec<String>,
}

impl Default for LanguageDebugTemplate {
    fn default() -> Self {
        Self {
            language: String::new(),
            display_name: String::new(),
            enabled: false,
            instrumentation_template: String::new(),
            region_start: "// #region agent log".to_string(),
            region_end: "// #endregion".to_string(),
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentSubagentOverrideState {
    Enabled,
    Disabled,
}

pub type ParentSubagentOverrideConfig = HashMap<String, AgentSubagentOverrideState>;
pub type AgentSubagentOverrideConfig = HashMap<String, ParentSubagentOverrideConfig>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, from = "AIModelConfigCompat")]
pub struct AIModelConfig {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model_name: String,
    pub base_url: String,

    /// Computed actual request URL (auto-derived from base_url + provider format).
    /// Stored by the frontend when config is saved; falls back to base_url if absent.
    #[serde(default)]
    pub request_url: Option<String>,

    pub api_key: String,
    /// Context window size (total token limit for input + output).
    pub context_window: Option<u32>,
    /// Max output tokens (request parameter limiting model output length).
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub enabled: bool,
    /// Model category (primary category used for UI filtering).
    pub category: ModelCategory,
    /// Capability tags (multi-select).
    pub capabilities: Vec<ModelCapability>,
    /// Recommended use cases.
    #[serde(default)]
    pub recommended_for: Vec<String>,
    /// Additional metadata (JSON, for extensibility).
    pub metadata: Option<serde_json::Value>,

    /// Compatibility-only input field for older saved configs.
    ///
    /// New code should use `reasoning_mode`. This field is deserialized for migration and
    /// compatibility, then omitted from future saves. When `reasoning_mode` is absent, `true`
    /// maps to `enabled` and `false` maps to `default`.
    #[serde(default, skip_serializing)]
    pub enable_thinking_process: bool,

    /// Provider-agnostic reasoning mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_mode: Option<ReasoningMode>,

    /// Whether to parse OpenAI-compatible text chunks containing `<think>...</think>` into
    /// streaming reasoning content.
    #[serde(default = "default_true")]
    pub inline_think_in_text: bool,

    /// Custom HTTP request headers.
    #[serde(default)]
    pub custom_headers: Option<std::collections::HashMap<String, String>>,

    /// Custom header mode: "replace" (default, full replacement) or "merge" (merge; apply
    /// defaults first, then custom).
    #[serde(default)]
    pub custom_headers_mode: Option<String>,

    /// Whether to skip SSL certificate verification (advanced; use only when necessary).
    #[serde(default)]
    pub skip_ssl_verify: bool,

    /// Reasoning effort level for providers that support explicit effort controls.
    /// Valid values are provider-specific. None = use API default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,

    /// Optional Anthropic manual thinking token budget.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_budget_tokens: Option<u32>,

    /// Custom request body (JSON string, used to override default request body fields).
    #[serde(default)]
    pub custom_request_body: Option<String>,

    /// Custom request body mode: "merge" (default) or "trim" (keep only essential runtime
    /// fields, then apply custom JSON).
    #[serde(default)]
    pub custom_request_body_mode: Option<String>,

    /// Authentication source for this model. Defaults to a static API key for
    /// backward compatibility; selecting a CLI source causes the AI client
    /// factory to look up `~/.codex/auth.json` or `~/.gemini/...` at request
    /// time and inject the resolved Bearer token / extra headers.
    #[serde(default)]
    pub auth: AuthConfig,
}

/// Where to obtain the runtime auth material for an `AIModelConfig`.
///
/// Stored on disk as `{"type":"api_key"}` / `{"type":"codex_cli"}` /
/// `{"type":"gemini_cli"}`; the concrete sub-mode (apikey vs OAuth) is
/// auto-detected from the CLI's on-disk state at resolution time so the user
/// only has to choose "use Codex CLI" once.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    /// Use the inline `api_key` string (default; legacy behavior).
    #[default]
    ApiKey,
    /// Reuse `~/.codex/auth.json` (apikey or ChatGPT-login).
    CodexCli,
    /// Reuse `~/.gemini/.env` or `~/.gemini/oauth_creds.json`.
    GeminiCli,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct AIModelConfigCompat {
    id: String,
    name: String,
    provider: String,
    model_name: String,
    base_url: String,
    request_url: Option<String>,
    api_key: String,
    context_window: Option<u32>,
    max_tokens: Option<u32>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    enabled: bool,
    category: ModelCategory,
    capabilities: Vec<ModelCapability>,
    recommended_for: Vec<String>,
    metadata: Option<serde_json::Value>,
    enable_thinking_process: Option<bool>,
    reasoning_mode: Option<ReasoningMode>,
    #[serde(default = "default_true")]
    inline_think_in_text: bool,
    custom_headers: Option<std::collections::HashMap<String, String>>,
    custom_headers_mode: Option<String>,
    skip_ssl_verify: bool,
    reasoning_effort: Option<String>,
    thinking_budget_tokens: Option<u32>,
    custom_request_body: Option<String>,
    custom_request_body_mode: Option<String>,
    #[serde(default)]
    auth: AuthConfig,
}

impl From<AIModelConfigCompat> for AIModelConfig {
    fn from(value: AIModelConfigCompat) -> Self {
        let reasoning_mode = value.reasoning_mode.or_else(|| {
            value.enable_thinking_process.map(|enabled| {
                if enabled {
                    ReasoningMode::Enabled
                } else {
                    ReasoningMode::Default
                }
            })
        });

        Self {
            id: value.id,
            name: value.name,
            provider: value.provider,
            model_name: value.model_name,
            base_url: value.base_url,
            request_url: value.request_url,
            api_key: value.api_key,
            context_window: value.context_window,
            max_tokens: value.max_tokens,
            temperature: value.temperature,
            top_p: value.top_p,
            enabled: value.enabled,
            category: value.category,
            capabilities: value.capabilities,
            recommended_for: value.recommended_for,
            metadata: value.metadata,
            enable_thinking_process: value.enable_thinking_process.unwrap_or(false),
            reasoning_mode,
            inline_think_in_text: value.inline_think_in_text,
            custom_headers: value.custom_headers,
            custom_headers_mode: value.custom_headers_mode,
            skip_ssl_verify: value.skip_ssl_verify,
            reasoning_effort: value.reasoning_effort,
            thinking_budget_tokens: value.thinking_budget_tokens,
            custom_request_body: value.custom_request_body,
            custom_request_body_mode: value.custom_request_body_mode,
            auth: value.auth,
        }
    }
}

impl AIModelConfig {
    pub fn effective_reasoning_mode(&self) -> ReasoningMode {
        self.reasoning_mode.unwrap_or({
            if self.enable_thinking_process {
                ReasoningMode::Enabled
            } else {
                ReasoningMode::Default
            }
        })
    }
}

/// Configuration provider interface.
#[async_trait]
pub trait ConfigProvider: Send + Sync {
    /// Provider name.
    fn name(&self) -> &str;

    /// Returns the default configuration.
    fn default_config(&self) -> serde_json::Value;

    /// Validates configuration.
    async fn validate_config(&self, config: &serde_json::Value) -> NortHingResult<Vec<String>>;

    /// Called when configuration changes.
    async fn on_config_changed(
        &self,
        old_config: &serde_json::Value,
        new_config: &serde_json::Value,
    ) -> NortHingResult<()>;

    /// Migrates configuration (used for version upgrades).
    async fn migrate_config(&self, version: &str, config: serde_json::Value) -> NortHingResult<serde_json::Value>;
}

fn default_true() -> bool {
    true
}

impl Default for AIModelConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            provider: String::new(),
            model_name: String::new(),
            base_url: String::new(),
            request_url: None,
            api_key: String::new(),
            context_window: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            enabled: false,
            category: ModelCategory::GeneralChat,
            capabilities: vec![],
            recommended_for: vec![],
            metadata: None,
            enable_thinking_process: false,
            reasoning_mode: None,
            inline_think_in_text: true,
            custom_headers: None,
            custom_headers_mode: None,
            skip_ssl_verify: false,
            reasoning_effort: None,
            thinking_budget_tokens: None,
            custom_request_body: None,
            custom_request_body_mode: None,
            auth: AuthConfig::ApiKey,
        }
    }
}

impl AIModelConfig {
    /// Legacy helper that infers the model category from the model name and provider.
    ///
    /// This is kept for one-off migrations/debugging, but runtime behavior should prefer
    /// explicitly configured `category`/`capabilities`.
    pub fn infer_category_from_model_name(&self) -> ModelCategory {
        let model_name_lower = self.model_name.to_lowercase();
        let provider_lower = self.provider.to_lowercase();

        if model_name_lower.contains("dall-e")
            || model_name_lower.contains("dalle")
            || model_name_lower.contains("stable-diffusion")
            || model_name_lower.contains("midjourney")
        {
            return ModelCategory::ImageGeneration;
        }

        if model_name_lower.contains("embedding") || model_name_lower.contains("text-embedding") {
            return ModelCategory::Embedding;
        }

        if provider_lower.contains("perplexity") || model_name_lower.contains("perplexity") {
            return ModelCategory::SearchEnhanced;
        }

        if model_name_lower.contains("vision")
            || model_name_lower.contains("gpt-4o")
            || model_name_lower.contains("gpt-4-turbo")
            || model_name_lower.contains("claude-3")
            || model_name_lower.contains("gemini-pro-vision")
            || model_name_lower.contains("gemini-1.5")
            || model_name_lower.starts_with("kimi")
        {
            return ModelCategory::Multimodal;
        }

        if model_name_lower.contains("deepseek")
            || model_name_lower.contains("codellama")
            || model_name_lower.contains("code-")
        {
            return ModelCategory::CodeSpecialized;
        }

        ModelCategory::GeneralChat
    }

    /// Legacy helper that infers capability tags from the model category and name.
    ///
    /// This is kept for one-off migrations/debugging, but runtime behavior should prefer
    /// explicitly configured `category`/`capabilities`.
    pub fn infer_capabilities_from_model(&self) -> Vec<ModelCapability> {
        let mut capabilities = vec![];
        let model_name_lower = self.model_name.to_lowercase();

        match self.category {
            ModelCategory::GeneralChat => {
                capabilities.push(ModelCapability::TextChat);
                if model_name_lower.contains("gpt-4")
                    || model_name_lower.contains("claude-3")
                    || model_name_lower.contains("gemini")
                {
                    capabilities.push(ModelCapability::FunctionCalling);
                }
            }
            ModelCategory::Multimodal => {
                capabilities.push(ModelCapability::TextChat);
                capabilities.push(ModelCapability::ImageUnderstanding);
                capabilities.push(ModelCapability::FunctionCalling);
            }
            ModelCategory::ImageGeneration => {
                capabilities.push(ModelCapability::ImageGeneration);
            }
            ModelCategory::Embedding => {
                capabilities.push(ModelCapability::Embedding);
            }
            ModelCategory::SearchEnhanced => {
                capabilities.push(ModelCapability::TextChat);
                capabilities.push(ModelCapability::Search);
            }
            ModelCategory::CodeSpecialized => {
                capabilities.push(ModelCapability::TextChat);
                capabilities.push(ModelCapability::CodeSpecialized);
                capabilities.push(ModelCapability::FunctionCalling);
            }
            ModelCategory::SpeechRecognition => {
                capabilities.push(ModelCapability::SpeechRecognition);
            }
        }

        capabilities
    }

    fn default_capabilities_for_category(&self) -> Vec<ModelCapability> {
        match self.category {
            ModelCategory::GeneralChat => vec![ModelCapability::TextChat],
            ModelCategory::Multimodal => {
                vec![ModelCapability::TextChat, ModelCapability::ImageUnderstanding]
            }
            ModelCategory::ImageGeneration => vec![ModelCapability::ImageGeneration],
            ModelCategory::Embedding => vec![ModelCapability::Embedding],
            ModelCategory::SearchEnhanced => {
                vec![ModelCapability::TextChat, ModelCapability::Search]
            }
            ModelCategory::CodeSpecialized => {
                vec![ModelCapability::TextChat, ModelCapability::CodeSpecialized]
            }
            ModelCategory::SpeechRecognition => vec![ModelCapability::SpeechRecognition],
        }
    }

    /// Auto-completes missing capability information without rewriting explicit configuration.
    ///
    /// Important: we intentionally do not upgrade `category` or append inferred capabilities
    /// based on the model name here. Runtime behavior should follow explicit configuration.
    pub fn ensure_category_and_capabilities(&mut self) {
        if self.capabilities.is_empty() {
            self.capabilities = self.default_capabilities_for_category();
        }
    }
}
