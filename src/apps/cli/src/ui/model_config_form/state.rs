use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::types::{DisplayRow, FormField, ModelFormAction, ModelFormResult};

const PROVIDER_FORMATS: [&str; 2] = ["openai", "anthropic"];
const CUSTOM_HEADERS_MODES: [&str; 2] = ["merge", "replace"];

/// Model config form state
pub struct ModelConfigFormState {
    visible: bool,

    // ── Field values ──
    name: String,
    model_name: String,
    base_url: String,
    api_key: String,
    provider_format_index: usize,
    context_window: String,
    max_tokens: String,
    enable_thinking: bool,
    support_preserved_thinking: bool,
    skip_ssl_verify: bool,
    custom_headers: String,
    custom_headers_mode_index: usize,
    custom_request_body: String,

    // ── UI state ──
    active_field: FormField,
    cursor: usize,
    scroll_offset: usize,
    visible_rows: usize,
    /// Whether the advanced settings section is expanded
    show_advanced: bool,

    /// Preset provider name (if from a template), shown in title
    provider_name: Option<String>,
    /// If editing an existing model, this holds the model ID
    editing_model_id: Option<String>,
}

impl ModelConfigFormState {
    pub fn new() -> Self {
        Self {
            visible: false,
            name: String::new(),
            model_name: String::new(),
            base_url: String::new(),
            api_key: String::new(),
            provider_format_index: 0,
            context_window: "128000".into(),
            max_tokens: "8192".into(),
            enable_thinking: false,
            support_preserved_thinking: false,
            skip_ssl_verify: false,
            custom_headers: String::new(),
            custom_headers_mode_index: 0, // "merge" by default
            custom_request_body: String::new(),
            active_field: FormField::Name,
            cursor: 0,
            scroll_offset: 0,
            visible_rows: 0,
            show_advanced: false,
            provider_name: None,
            editing_model_id: None,
        }
    }

    /// Show the form for a custom model (empty fields)
    pub fn show_custom(&mut self) {
        self.visible = true;
        self.provider_name = None;
        self.editing_model_id = None;
        self.name.clear();
        self.model_name.clear();
        self.base_url = "https://".into();
        self.api_key.clear();
        self.provider_format_index = 0;
        self.context_window = "128000".into();
        self.max_tokens = "8192".into();
        self.enable_thinking = false;
        self.support_preserved_thinking = false;
        self.skip_ssl_verify = false;
        self.custom_headers.clear();
        self.custom_headers_mode_index = 0; // "merge" by default
        self.custom_request_body.clear();
        self.active_field = FormField::Name;
        self.cursor = 0;
        self.scroll_offset = 0;
        self.show_advanced = false;
    }

    /// Show the form pre-filled from a provider template
    pub fn show_from_provider(
        &mut self,
        provider_name: &str,
        base_url: &str,
        format: &str,
        default_model: &str,
    ) {
        self.visible = true;
        self.provider_name = Some(provider_name.to_string());
        self.editing_model_id = None;
        self.name = if default_model.is_empty() {
            String::new()
        } else {
            format!("{} - {}", provider_name, default_model)
        };
        self.model_name = default_model.to_string();
        self.base_url = base_url.to_string();
        self.api_key.clear();
        self.provider_format_index =
            PROVIDER_FORMATS.iter().position(|&f| f == format).unwrap_or(0);
        self.context_window = "128000".into();
        self.max_tokens = "8192".into();
        self.enable_thinking = false;
        self.support_preserved_thinking = false;
        self.skip_ssl_verify = false;
        self.custom_headers.clear();
        self.custom_headers_mode_index = 0; // "merge" by default
        self.custom_request_body.clear();
        self.active_field = FormField::ApiKey;
        self.cursor = 0;
        self.scroll_offset = 0;
        self.show_advanced = false;
    }

    /// Show the form pre-filled for editing an existing model
    pub fn show_for_edit(&mut self, model_id: &str, result: &ModelFormResult) {
        self.visible = true;
        self.editing_model_id = Some(model_id.to_string());
        self.provider_name = None;
        self.name = result.name.clone();
        self.model_name = result.model_name.clone();
        self.base_url = result.base_url.clone();
        self.api_key = result.api_key.clone();
        self.provider_format_index = PROVIDER_FORMATS
            .iter()
            .position(|&f| f == result.provider_format)
            .unwrap_or(0);
        self.context_window = result.context_window.to_string();
        self.max_tokens = result.max_tokens.to_string();
        self.enable_thinking = result.enable_thinking;
        self.support_preserved_thinking = result.support_preserved_thinking;
        self.skip_ssl_verify = result.skip_ssl_verify;
        self.custom_headers = result.custom_headers.clone();
        self.custom_headers_mode_index = CUSTOM_HEADERS_MODES
            .iter()
            .position(|&m| m == result.custom_headers_mode)
            .unwrap_or(0);
        self.custom_request_body = result.custom_request_body.clone();
        self.active_field = FormField::Name;
        self.cursor = self.name.chars().count();
        self.scroll_offset = 0;
        // Auto-expand advanced if any advanced fields have non-default values
        self.show_advanced = self.skip_ssl_verify
            || !self.custom_headers.is_empty()
            || self.custom_headers_mode_index != 0
            || !self.custom_request_body.is_empty()
            || (self.enable_thinking && self.support_preserved_thinking);
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Reshow the model config form (for back navigation)
    pub fn reshow(&mut self) {
        self.visible = true;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    // ── Dynamic field order ──

    /// Build the current field order based on toggle states.
    fn current_fields(&self) -> Vec<FormField> {
        let mut fields = vec![
            FormField::Name,
            FormField::ModelName,
            FormField::BaseUrl,
            FormField::ApiKey,
            FormField::ProviderFormat,
            FormField::ContextWindow,
            FormField::MaxTokens,
            FormField::EnableThinking,
        ];
        if self.show_advanced {
            if self.enable_thinking {
                fields.push(FormField::PreservedThinking);
            }
            fields.push(FormField::SkipSslVerify);
            fields.push(FormField::CustomHeaders);
            fields.push(FormField::CustomHeadersMode);
            fields.push(FormField::CustomRequestBody);
        }
        fields
    }

    /// Build the list of display rows. Each field gets 2 rows (label + input),
    /// plus an extra separator row before the advanced section.
    pub(crate) fn display_rows(&self) -> Vec<DisplayRow> {
        let fields = self.current_fields();
        let mut rows = Vec::new();
        let mut advanced_header_shown = false;
        for &f in &fields {
            // Show the advanced separator before the first advanced field
            if !advanced_header_shown && self.is_advanced_field(f) {
                rows.push(DisplayRow::AdvancedHeader);
                advanced_header_shown = true;
            }
            rows.push(DisplayRow::Label(f));
            rows.push(DisplayRow::Input(f));
        }
        rows
    }

    fn is_advanced_field(&self, field: FormField) -> bool {
        matches!(
            field,
            FormField::PreservedThinking
                | FormField::SkipSslVerify
                | FormField::CustomHeaders
                | FormField::CustomHeadersMode
                | FormField::CustomRequestBody
        )
    }

    // ── Field buffer access ──

    fn active_buffer(&self) -> &str {
        match self.active_field {
            FormField::Name => &self.name,
            FormField::ModelName => &self.model_name,
            FormField::BaseUrl => &self.base_url,
            FormField::ApiKey => &self.api_key,
            FormField::ContextWindow => &self.context_window,
            FormField::MaxTokens => &self.max_tokens,
            FormField::CustomHeaders => &self.custom_headers,
            FormField::CustomRequestBody => &self.custom_request_body,
            // Non-text fields
            FormField::ProviderFormat
            | FormField::CustomHeadersMode
            | FormField::EnableThinking
            | FormField::PreservedThinking
            | FormField::SkipSslVerify => "",
        }
    }

    fn active_buffer_mut(&mut self) -> Option<&mut String> {
        match self.active_field {
            FormField::Name => Some(&mut self.name),
            FormField::ModelName => Some(&mut self.model_name),
            FormField::BaseUrl => Some(&mut self.base_url),
            FormField::ApiKey => Some(&mut self.api_key),
            FormField::ContextWindow => Some(&mut self.context_window),
            FormField::MaxTokens => Some(&mut self.max_tokens),
            FormField::CustomHeaders => Some(&mut self.custom_headers),
            FormField::CustomRequestBody => Some(&mut self.custom_request_body),
            _ => None,
        }
    }

    /// Is the active field a non-text field that uses special controls?
    fn is_non_text_field(&self) -> bool {
        matches!(
            self.active_field,
            FormField::ProviderFormat
                | FormField::CustomHeadersMode
                | FormField::EnableThinking
                | FormField::PreservedThinking
                | FormField::SkipSslVerify
        )
    }

    /// Is the active field a boolean toggle?
    fn is_toggle_field(&self) -> bool {
        matches!(
            self.active_field,
            FormField::EnableThinking | FormField::PreservedThinking | FormField::SkipSslVerify
        )
    }

    fn toggle_active_bool(&mut self) {
        match self.active_field {
            FormField::EnableThinking => {
                self.enable_thinking = !self.enable_thinking;
                if !self.enable_thinking {
                    self.support_preserved_thinking = false;
                }
            }
            FormField::PreservedThinking => {
                self.support_preserved_thinking = !self.support_preserved_thinking;
            }
            FormField::SkipSslVerify => {
                self.skip_ssl_verify = !self.skip_ssl_verify;
            }
            _ => {}
        }
    }

    // ── Navigation ──

    fn next_field(&mut self) {
        let fields = self.current_fields();
        let idx = fields.iter().position(|f| *f == self.active_field).unwrap_or(0);
        let next = (idx + 1).min(fields.len() - 1);
        self.active_field = fields[next];
        self.cursor = self.active_buffer().chars().count();
        self.ensure_field_visible();
    }

    fn prev_field(&mut self) {
        let fields = self.current_fields();
        let idx = fields.iter().position(|f| *f == self.active_field).unwrap_or(0);
        let prev = idx.saturating_sub(1);
        self.active_field = fields[prev];
        self.cursor = self.active_buffer().chars().count();
        self.ensure_field_visible();
    }

    fn ensure_field_visible(&mut self) {
        let rows = self.display_rows();
        // Find the Label row for the active field
        let label_row_idx = rows
            .iter()
            .position(|r| matches!(r, DisplayRow::Label(f) if *f == self.active_field))
            .unwrap_or(0);
        // Also ensure the Input row is visible (+1)
        let input_row_idx = (label_row_idx + 1).min(rows.len().saturating_sub(1));

        if label_row_idx < self.scroll_offset {
            self.scroll_offset = label_row_idx;
        } else if self.visible_rows > 0 && input_row_idx >= self.scroll_offset + self.visible_rows {
            self.scroll_offset = input_row_idx.saturating_sub(self.visible_rows - 1);
        }
    }

    // ── Validation ──

    fn validate(&self) -> Option<String> {
        if self.name.trim().is_empty() {
            return Some("Name is required".into());
        }
        if self.model_name.trim().is_empty() {
            return Some("Model name is required".into());
        }
        if self.base_url.trim().is_empty() {
            return Some("Base URL is required".into());
        }
        if self.api_key.trim().is_empty() {
            return Some("API Key is required".into());
        }
        if self.context_window.trim().parse::<u32>().is_err() {
            return Some("Context window must be a number".into());
        }
        if self.max_tokens.trim().parse::<u32>().is_err() {
            return Some("Max tokens must be a number".into());
        }
        // Validate JSON fields if non-empty
        if !self.custom_headers.trim().is_empty() {
            if serde_json::from_str::<serde_json::Value>(self.custom_headers.trim()).is_err() {
                return Some("Custom headers must be valid JSON".into());
            }
        }
        if !self.custom_request_body.trim().is_empty() {
            if serde_json::from_str::<serde_json::Value>(self.custom_request_body.trim()).is_err() {
                return Some("Custom request body must be valid JSON".into());
            }
        }
        None
    }

    fn build_result(&self) -> ModelFormResult {
        ModelFormResult {
            editing_model_id: self.editing_model_id.clone(),
            name: self.name.trim().to_string(),
            model_name: self.model_name.trim().to_string(),
            base_url: self.base_url.trim().to_string(),
            api_key: self.api_key.trim().to_string(),
            provider_format: PROVIDER_FORMATS[self.provider_format_index].to_string(),
            context_window: self.context_window.trim().parse().unwrap_or(128000),
            max_tokens: self.max_tokens.trim().parse().unwrap_or(8192),
            enable_thinking: self.enable_thinking,
            support_preserved_thinking: self.support_preserved_thinking,
            skip_ssl_verify: self.skip_ssl_verify,
            custom_headers: self.custom_headers.trim().to_string(),
            custom_headers_mode: CUSTOM_HEADERS_MODES[self.custom_headers_mode_index].to_string(),
            custom_request_body: self.custom_request_body.trim().to_string(),
        }
    }

    // ── Key handling ──

    pub fn handle_key_event(&mut self, key: KeyEvent) -> ModelFormAction {
        if !self.visible {
            return ModelFormAction::None;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.hide();
                ModelFormAction::Cancel
            }

            // Ctrl+S: save
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => self.try_save(),

            // Ctrl+A: toggle advanced settings
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.show_advanced = !self.show_advanced;
                // If we were on an advanced field that's now hidden, move to a safe field
                if !self.show_advanced {
                    let fields = self.current_fields();
                    if !fields.contains(&self.active_field) {
                        self.active_field = *fields.last().unwrap_or(&FormField::Name);
                        self.cursor = self.active_buffer().chars().count();
                    }
                }
                ModelFormAction::None
            }

            // Tab: next field
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.next_field();
                ModelFormAction::None
            }

            // Shift-Tab: previous field
            (KeyCode::BackTab, _) => {
                self.prev_field();
                ModelFormAction::None
            }

            // Enter: toggle for boolean fields, next field for text, save on last
            (KeyCode::Enter, _) => {
                if self.is_toggle_field() {
                    self.toggle_active_bool();
                    ModelFormAction::None
                } else {
                    let fields = self.current_fields();
                    let idx = fields.iter().position(|f| *f == self.active_field).unwrap_or(0);
                    if idx == fields.len() - 1 {
                        self.try_save()
                    } else {
                        self.next_field();
                        ModelFormAction::None
                    }
                }
            }

            // Space: toggle for boolean fields
            (KeyCode::Char(' '), _) if self.is_toggle_field() => {
                self.toggle_active_bool();
                ModelFormAction::None
            }

            // For select fields: Left/Right toggle options
            (KeyCode::Left, KeyModifiers::NONE)
                if matches!(self.active_field, FormField::ProviderFormat) =>
            {
                if self.provider_format_index > 0 {
                    self.provider_format_index -= 1;
                }
                ModelFormAction::None
            }
            (KeyCode::Right, KeyModifiers::NONE)
                if matches!(self.active_field, FormField::ProviderFormat) =>
            {
                if self.provider_format_index < PROVIDER_FORMATS.len() - 1 {
                    self.provider_format_index += 1;
                }
                ModelFormAction::None
            }

            (KeyCode::Left, KeyModifiers::NONE)
                if matches!(self.active_field, FormField::CustomHeadersMode) =>
            {
                if self.custom_headers_mode_index > 0 {
                    self.custom_headers_mode_index -= 1;
                }
                ModelFormAction::None
            }
            (KeyCode::Right, KeyModifiers::NONE)
                if matches!(self.active_field, FormField::CustomHeadersMode) =>
            {
                if self.custom_headers_mode_index < CUSTOM_HEADERS_MODES.len() - 1 {
                    self.custom_headers_mode_index += 1;
                }
                ModelFormAction::None
            }

            // Up/Down: navigate fields
            (KeyCode::Up, KeyModifiers::NONE) => {
                self.prev_field();
                ModelFormAction::None
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                self.next_field();
                ModelFormAction::None
            }

            // Text editing keys for text fields only
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT)
                if !self.is_non_text_field() =>
            {
                let cursor = self.cursor;
                if let Some(buf) = self.active_buffer_mut() {
                    let byte_pos = char_to_byte(buf, cursor);
                    buf.insert(byte_pos, c);
                }
                self.cursor += 1;
                ModelFormAction::None
            }

            (KeyCode::Backspace, _) if !self.is_non_text_field() => {
                if self.cursor > 0 {
                    let cursor = self.cursor;
                    if let Some(buf) = self.active_buffer_mut() {
                        let byte_start = char_to_byte(buf, cursor - 1);
                        let byte_end = char_to_byte(buf, cursor);
                        buf.drain(byte_start..byte_end);
                    }
                    self.cursor -= 1;
                }
                ModelFormAction::None
            }

            (KeyCode::Left, KeyModifiers::NONE) if !self.is_non_text_field() => {
                self.cursor = self.cursor.saturating_sub(1);
                ModelFormAction::None
            }

            (KeyCode::Right, KeyModifiers::NONE) if !self.is_non_text_field() => {
                let max = self.active_buffer().chars().count();
                self.cursor = (self.cursor + 1).min(max);
                ModelFormAction::None
            }

            (KeyCode::Home, _) => {
                self.cursor = 0;
                ModelFormAction::None
            }

            (KeyCode::End, _) => {
                self.cursor = self.active_buffer().chars().count();
                ModelFormAction::None
            }

            _ => ModelFormAction::None,
        }
    }

    fn try_save(&mut self) -> ModelFormAction {
        if self.validate().is_some() {
            ModelFormAction::None
        } else {
            let result = self.build_result();
            self.hide();
            ModelFormAction::Save(result)
        }
    }

    // ── Shared accessors for rendering ──

    pub(super) fn active_field(&self) -> FormField {
        self.active_field
    }

    pub(super) fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub(super) fn set_visible_rows(&mut self, rows: usize) {
        self.visible_rows = rows;
    }

    pub(super) fn is_show_advanced(&self) -> bool {
        self.show_advanced
    }

    pub(super) fn editing_model_id(&self) -> Option<&str> {
        self.editing_model_id.as_deref()
    }

    pub(super) fn provider_name(&self) -> Option<&str> {
        self.provider_name.as_deref()
    }

    pub(super) fn enable_thinking(&self) -> bool {
        self.enable_thinking
    }

    pub(super) fn support_preserved_thinking(&self) -> bool {
        self.support_preserved_thinking
    }

    pub(super) fn skip_ssl_verify(&self) -> bool {
        self.skip_ssl_verify
    }

    pub(super) fn provider_format_index(&self) -> usize {
        self.provider_format_index
    }

    pub(super) fn custom_headers_mode_index(&self) -> usize {
        self.custom_headers_mode_index
    }

    pub(super) fn field_value(&self, field: FormField) -> &str {
        match field {
            FormField::Name => &self.name,
            FormField::ModelName => &self.model_name,
            FormField::BaseUrl => &self.base_url,
            FormField::ApiKey => &self.api_key,
            FormField::ContextWindow => &self.context_window,
            FormField::MaxTokens => &self.max_tokens,
            FormField::CustomHeaders => &self.custom_headers,
            FormField::CustomRequestBody => &self.custom_request_body,
            _ => "",
        }
    }

    pub(super) fn is_active_field(&self, field: FormField) -> bool {
        self.active_field == field
    }

    pub(super) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(super) fn validate_msg(&self) -> Option<String> {
        self.validate()
    }
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(s.len())
}
