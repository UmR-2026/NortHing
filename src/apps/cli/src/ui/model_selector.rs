/// Model selector popup for choosing AI model
///
/// Full-screen overlay popup that displays all available models
/// and allows the user to select one.
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::ui::theme::{StyleKind, Theme};
use northhing_core::service::config::AIModelConfig;

// ===== Shared model display helpers (used by startup/selectors and modes/chat/model) =====

/// Default context window size used when a model does not specify one.
pub const DEFAULT_CONTEXT_WINDOW: u32 = 128_000;
/// Default max tokens used when a model does not specify one.
pub const DEFAULT_MAX_TOKENS: u32 = 8_192;

/// Sentinel value for "no specific model ID resolved" in the primary fallback chain.
pub const PRIMARY_SENTINEL: &str = "primary";

/// Extract the provider name from a model config's display name.
///
/// Handles "Provider - ModelName" and "Provider/ModelName" formats.
#[inline]
pub fn provider_display_name(model: &AIModelConfig) -> String {
    let raw_name = model.name.trim();
    let model_name = model.model_name.trim();
    if !raw_name.is_empty() && !model_name.is_empty() {
        let dashed_suffix = format!(" - {}", model_name);
        let slash_suffix = format!("/{}", model_name);
        if let Some(provider) = raw_name.strip_suffix(&dashed_suffix) {
            return provider.trim().to_string();
        }
        if let Some(provider) = raw_name.strip_suffix(&slash_suffix) {
            return provider.trim().to_string();
        }
    }
    if raw_name.is_empty() {
        model.provider.clone()
    } else {
        raw_name.to_string()
    }
}

/// Build a human-readable display name: "model_name / provider".
#[inline]
pub fn model_display_name(model: &AIModelConfig) -> String {
    format!("{} / {}", model.model_name, provider_display_name(model))
}

/// Parse custom headers from form fields.
///
/// Returns `(parsed_headers_map, effective_mode)` where:
/// - `parsed_headers_map` is `Some(HashMap)` if valid JSON was provided, else `None`
/// - `effective_mode` is `Some(mode)` unless mode is empty or "merge" (the default)
pub fn parse_custom_headers(
    raw: &str,
    mode: &str,
) -> (Option<std::collections::HashMap<String, String>>, Option<String>) {
    let custom_headers = (!raw.is_empty())
        .then(|| serde_json::from_str(raw).ok())
        .flatten();
    let custom_headers_mode = (!mode.is_empty() && mode != "merge").then(|| mode.to_string());
    (custom_headers, custom_headers_mode)
}

/// A model item for display in the selector
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelItem {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model_name: String,
}

impl ModelItem {
    pub fn from_config(m: &AIModelConfig) -> Self {
        Self {
            id: m.id.clone(),
            name: m.name.clone(),
            provider: m.provider.clone(),
            model_name: m.model_name.clone(),
        }
    }
}

impl From<&AIModelConfig> for ModelItem {
    fn from(m: &AIModelConfig) -> Self {
        Self::from_config(m)
    }
}

/// Model selector popup state
pub struct ModelSelectorState {
    items: Vec<ModelItem>,
    list_state: ListState,
    visible: bool,
    /// Currently active model ID (for highlighting)
    current_model_id: Option<String>,
    last_area: Option<Rect>,
}

impl ModelSelectorState {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            list_state: ListState::default(),
            visible: false,
            current_model_id: None,
            last_area: None,
        }
    }

    /// Show the model selector with given model list
    pub fn show(&mut self, models: Vec<ModelItem>, current_model_id: Option<String>) {
        if models.is_empty() {
            return;
        }

        // Find current model index for initial selection
        let initial_idx = current_model_id
            .as_ref()
            .and_then(|id| models.iter().position(|m| m.id == *id))
            .unwrap_or(0);

        self.items = models;
        self.current_model_id = current_model_id;
        self.list_state.select(Some(initial_idx));
        self.visible = true;
    }

    /// Hide the model selector
    pub fn hide(&mut self) {
        self.visible = false;
        // Note: we don't clear items here to support back navigation
        self.last_area = None;
    }

    /// Reshow the model selector (for back navigation)
    pub fn reshow(&mut self) {
        if !self.items.is_empty() {
            self.visible = true;
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn move_up(&mut self) {
        if !self.visible || self.items.is_empty() {
            return;
        }
        let selected = self.list_state.selected().unwrap_or(0);
        let len = self.items.len();
        let next = (selected + len - 1) % len;
        self.list_state.select(Some(next));
    }

    pub fn move_down(&mut self) {
        if !self.visible || self.items.is_empty() {
            return;
        }
        let selected = self.list_state.selected().unwrap_or(0);
        let next = (selected + 1) % self.items.len();
        self.list_state.select(Some(next));
    }

    /// Get the selected model item (returns clone of ModelItem)
    pub fn confirm_selection(&self) -> Option<ModelItem> {
        if !self.visible {
            return None;
        }
        let idx = self.list_state.selected()?;
        self.items.get(idx).cloned()
    }

    /// Render the model selector popup as an overlay
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible || self.items.is_empty() {
            self.last_area = None;
            return;
        }

        // Calculate popup area (centered, leaving some margin)
        let popup_width = area.width.saturating_sub(4).min(70);
        let popup_height = (self.items.len() as u16 + 4).min(area.height.saturating_sub(2));
        if popup_height < 5 || popup_width < 20 {
            self.last_area = None;
            return;
        }

        let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };
        self.last_area = Some(popup_area);

        // Build list items
        let list_items: Vec<ListItem> = self
            .items
            .iter()
            .map(|model| {
                let is_current = self.current_model_id.as_ref().map_or(false, |id| id == &model.id);

                let marker = if is_current { "● " } else { "  " };
                let marker_style = if is_current {
                    theme.style(StyleKind::Success)
                } else {
                    theme.style(StyleKind::Muted)
                };

                let name_style = theme.style(StyleKind::Primary).add_modifier(Modifier::BOLD);
                let detail_style = theme.style(StyleKind::Muted);

                let line = Line::from(vec![
                    Span::styled(marker, marker_style),
                    Span::styled(&model.name, name_style),
                    Span::raw("  "),
                    Span::styled(format!("[{}/{}]", model.provider, model.model_name), detail_style),
                ]);
                ListItem::new(line)
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.style(StyleKind::Primary))
            .style(Style::default().bg(theme.background))
            .title(" Select Model (↑↓ Navigate, Enter Select, e Edit, Esc Cancel) ");

        let list = List::new(list_items)
            .block(block)
            .style(Style::default().bg(theme.background))
            .highlight_style(
                Style::default()
                    .bg(theme.primary)
                    .fg(theme.selection_foreground())
                    .add_modifier(Modifier::BOLD),
            );

        // Clear area first, then render
        frame.render_widget(Clear, popup_area);
        frame.render_stateful_widget(list, popup_area, &mut self.list_state);

        // Render hint at bottom
        let hint_area = Rect {
            x: popup_area.x,
            y: popup_area.y + popup_area.height,
            width: popup_area.width,
            height: 1.min(area.y + area.height - popup_area.y - popup_area.height),
        };
        if hint_area.height > 0 {
            let hint = Paragraph::new(Line::from(vec![Span::styled(
                " Selecting a model will apply to all modes ",
                theme.style(StyleKind::Info),
            )]))
            .alignment(Alignment::Center);
            frame.render_widget(hint, hint_area);
        }
    }

    /// Handle mouse events in the model selector
    /// Returns Some(model_id) if a model was clicked/selected, None otherwise
    pub fn handle_mouse_event(&mut self, mouse: &MouseEvent) -> Option<ModelItem> {
        if !self.visible {
            return None;
        }

        let area = match self.last_area {
            Some(area) => area,
            None => return None,
        };

        let in_popup = mouse.column >= area.x
            && mouse.column < area.x.saturating_add(area.width)
            && mouse.row >= area.y
            && mouse.row < area.y.saturating_add(area.height);

        match mouse.kind {
            MouseEventKind::ScrollUp if in_popup => {
                self.move_up();
                None
            }
            MouseEventKind::ScrollDown if in_popup => {
                self.move_down();
                None
            }
            MouseEventKind::Moved if in_popup => {
                if let Some(index) = self.item_index_at(mouse.row, area) {
                    self.list_state.select(Some(index));
                }
                None
            }
            MouseEventKind::Down(MouseButton::Left) if in_popup => {
                if let Some(index) = self.item_index_at(mouse.row, area) {
                    self.list_state.select(Some(index));
                    return self.confirm_selection();
                }
                None
            }
            // Click outside popup to dismiss
            MouseEventKind::Down(MouseButton::Left) if !in_popup => {
                self.hide();
                None
            }
            _ => None,
        }
    }

    /// Check if a mouse event is within the popup area (used to prevent event passthrough)
    pub fn captures_mouse(&self, _mouse: &MouseEvent) -> bool {
        if !self.visible {
            return false;
        }
        // When visible, capture all mouse events
        true
    }

    fn item_index_at(&self, row: u16, area: Rect) -> Option<usize> {
        if area.height < 3 {
            return None;
        }
        let inner_y = area.y.saturating_add(1); // border
        let inner_height = area.height.saturating_sub(2);

        if row < inner_y || row >= inner_y.saturating_add(inner_height) {
            return None;
        }

        let offset = self.list_state.offset();
        let index = (row - inner_y) as usize + offset;
        if index >= self.items.len() {
            return None;
        }

        Some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_item_from_config() {
        let config = AIModelConfig {
            id: "model_123".to_string(),
            name: "Claude 3.5 Sonnet".to_string(),
            provider: "anthropic".to_string(),
            model_name: "claude-3-5-sonnet".to_string(),
            ..Default::default()
        };

        let item = ModelItem::from_config(&config);
        assert_eq!(item.id, "model_123");
        assert_eq!(item.name, "Claude 3.5 Sonnet");
        assert_eq!(item.provider, "anthropic");
        assert_eq!(item.model_name, "claude-3-5-sonnet");

        let from_item: ModelItem = (&config).into();
        assert_eq!(from_item, item);
    }

    // --- provider_display_name tests ---

    #[test]
    fn test_provider_display_name_dash_suffix() {
        let model = AIModelConfig {
            name: "Anthropic - claude-3-5-sonnet".to_string(),
            model_name: "claude-3-5-sonnet".to_string(),
            provider: "anthropic".to_string(),
            ..Default::default()
        };
        assert_eq!(provider_display_name(&model), "Anthropic");
    }

    #[test]
    fn test_provider_display_name_slash_suffix() {
        let model = AIModelConfig {
            name: "openai/gpt-4o".to_string(),
            model_name: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            ..Default::default()
        };
        assert_eq!(provider_display_name(&model), "openai");
    }

    #[test]
    fn test_provider_display_name_cjk_passthrough() {
        let model = AIModelConfig {
            name: "阶跃星辰".to_string(),
            model_name: "step-2".to_string(),
            provider: "stepfun".to_string(),
            ..Default::default()
        };
        // CJK name without trailing pattern → returns name as-is
        assert_eq!(provider_display_name(&model), "阶跃星辰");
    }

    #[test]
    fn test_provider_display_name_empty_name_fallback() {
        let model = AIModelConfig {
            name: "".to_string(),
            model_name: "gpt-4".to_string(),
            provider: "openai".to_string(),
            ..Default::default()
        };
        assert_eq!(provider_display_name(&model), "openai");
    }

    // --- model_display_name tests ---

    #[test]
    fn test_model_display_name_basic() {
        let model = AIModelConfig {
            name: "openai/gpt-4o".to_string(),
            model_name: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            ..Default::default()
        };
        assert_eq!(model_display_name(&model), "gpt-4o / openai");
    }

    #[test]
    fn test_model_display_name_cjk() {
        let model = AIModelConfig {
            name: "阶跃星辰".to_string(),
            model_name: "step-2-16k".to_string(),
            provider: "stepfun".to_string(),
            ..Default::default()
        };
        assert_eq!(model_display_name(&model), "step-2-16k / 阶跃星辰");
    }

    // --- parse_custom_headers tests ---

    #[test]
    fn test_parse_custom_headers_empty() {
        let (headers, mode) = parse_custom_headers("", "");
        assert!(headers.is_none());
        assert!(mode.is_none());
    }

    #[test]
    fn test_parse_custom_headers_valid_json() {
        let json = r#"{"X-Key": "value"}"#;
        let (headers, mode) = parse_custom_headers(json, "merge");
        assert!(headers.is_some());
        assert_eq!(headers.unwrap().get("X-Key"), Some(&"value".to_string()));
        assert!(mode.is_none()); // "merge" is the default
    }

    #[test]
    fn test_parse_custom_headers_replace_mode() {
        let json = r#"{"X-Key": "value"}"#;
        let (headers, mode) = parse_custom_headers(json, "replace");
        assert!(headers.is_some());
        assert_eq!(mode, Some("replace".to_string()));
    }

    #[test]
    fn test_parse_custom_headers_invalid_json() {
        let (headers, mode) = parse_custom_headers("not-json", "merge");
        assert!(headers.is_none());
        assert!(mode.is_none());
    }
}
