/// Result of the model config form
#[derive(Debug, Clone)]
pub struct ModelFormResult {
    /// If set, this is an edit of an existing model (contains the model ID)
    pub editing_model_id: Option<String>,
    pub name: String,
    pub model_name: String,
    pub base_url: String,
    pub api_key: String,
    /// "openai" or "anthropic"
    pub provider_format: String,
    pub context_window: u32,
    pub max_tokens: u32,
    pub enable_thinking: bool,
    pub support_preserved_thinking: bool,
    pub skip_ssl_verify: bool,
    /// JSON string for custom headers, empty if none
    pub custom_headers: String,
    /// "merge" or "replace"
    pub custom_headers_mode: String,
    /// JSON string for custom request body, empty if none
    pub custom_request_body: String,
}

/// Action returned by the form
#[derive(Debug, Clone)]
pub enum ModelFormAction {
    /// No action, key consumed
    None,
    /// User saved the form
    Save(ModelFormResult),
    /// User cancelled
    Cancel,
}

/// Which field is active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormField {
    // ── Basic fields ──
    Name,
    ModelName,
    BaseUrl,
    ApiKey,
    ProviderFormat,
    ContextWindow,
    MaxTokens,
    EnableThinking,
    /// Only visible when enable_thinking is true
    PreservedThinking,
    // ── Advanced fields (Ctrl+A) ──
    SkipSslVerify,
    CustomHeaders,
    CustomHeadersMode,
    CustomRequestBody,
}

/// Display row types
#[derive(Debug, Clone)]
pub(crate) enum DisplayRow {
    /// Section separator for advanced settings
    AdvancedHeader,
    /// Field label
    Label(FormField),
    /// Field input
    Input(FormField),
}
