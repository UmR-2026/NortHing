use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::collections::{HashMap, HashSet, VecDeque};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::agent_selector::{AgentItem, AgentSelectorState};
use super::command_menu::CommandMenuState;
use super::command_palette::{CommandPaletteState, PaletteAction};
use super::markdown::MarkdownRenderer;
use super::mcp_add_dialog::{McpAddAction, McpAddDialogState};
use super::mcp_selector::{McpAction, McpItem, McpSelectorState};
use super::model_config_form::{ModelConfigFormState, ModelFormAction};
use super::model_selector::{ModelItem, ModelSelectorState};
use super::permission::render_permission_overlay;
use super::provider_selector::{ProviderSelection, ProviderSelectorState};
use super::question::render_question_overlay;
use super::session_selector::{SessionAction, SessionItem, SessionSelectorState};
use super::skill_selector::{SkillItem, SkillSelectorAction, SkillSelectorState};
use super::subagent_selector::{SubagentItem, SubagentSelectorAction, SubagentSelectorState};
use super::text_input::TextInput;
use super::theme::{StyleKind, Theme};
use super::theme_selector::{ThemeItem, ThemeSelectorState};
use super::widgets::Spinner;
use crate::chat_state::{ChatMessage, ChatState, FlowItem, MessageRole};

/// Types of popups that can be shown in the ChatView
#[derive(Debug, Clone, PartialEq)]
pub enum PopupType {
    CommandPalette,
    ModelSelector,
    AgentSelector,
    SessionSelector,
    SkillSelector,
    SubagentSelector,
    McpSelector,
    McpAddDialog,
    ProviderSelector,
    ModelConfigForm,
    ThemeSelector,
    InfoPopup,
}

/// Navigation stack for managing popup hierarchy
#[derive(Debug, Default)]
pub struct PopupStack {
    stack: Vec<PopupType>,
}

impl PopupStack {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Push a popup onto the stack
    pub fn push(&mut self, popup: PopupType) {
        // Avoid duplicates at the top
        if self.stack.last() != Some(&popup) {
            self.stack.push(popup);
        }
    }

    /// Pop the top popup from the stack
    pub fn pop(&mut self) -> Option<PopupType> {
        self.stack.pop()
    }

    /// Peek at the top popup without removing it
    pub fn peek(&self) -> Option<&PopupType> {
        self.stack.last()
    }

    /// Check if the stack is empty
    // reason: is_empty() kept for popup-stack inspection API completeness; current call sites use direct Vec::is_empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Clear all popups from the stack
    pub fn clear(&mut self) {
        self.stack.clear();
    }

    /// Remove a specific popup type from the stack (for when popup is closed directly)
    // reason: remove() reserved for direct popup dismissal that doesn't go through pop(); today's popups only pop the top
    #[allow(dead_code)]
    pub fn remove(&mut self, popup: &PopupType) {
        self.stack.retain(|p| p != popup);
    }

    /// Get the previous popup (for navigation back)
    // reason: previous() reserved for back-navigation between popups; today's key handlers pop instead of stepping back
    #[allow(dead_code)]
    pub fn previous(&self) -> Option<&PopupType> {
        if self.stack.len() >= 2 {
            self.stack.get(self.stack.len() - 2)
        } else {
            None
        }
    }
}

/// Cached render result for a single message
struct MessageRenderEntry {
    items: Vec<ListItem<'static>>,
    #[allow(dead_code)] // Used in Phase 3 (virtual scroll)
    line_count: usize,
    version: u64,
    width: u16,
    plain_lines: Vec<String>,
    /// Message-local clickable regions for block tools: (tool_id, y_start, y_end)
    tool_regions: Vec<(String, u16, u16)>,
    /// Message-local clickable regions for thinking blocks: (message_id, y_start, y_end)
    thinking_regions: Vec<(String, u16, u16)>,
}

struct MessageRenderResult {
    items: Vec<ListItem<'static>>,
    tool_regions: Vec<(String, u16, u16)>,
    thinking_regions: Vec<(String, u16, u16)>,
    plain_lines: Vec<String>,
}

// =============================================================================
// R2 ChatView Sub-structures
// =============================================================================

/// Manages all popup selector states and navigation stack
pub struct PopupManager {
    /// Model selector popup state
    pub model_selector: ModelSelectorState,
    /// Agent selector popup state
    pub agent_selector: AgentSelectorState,
    /// Session selector popup state
    pub session_selector: SessionSelectorState,
    /// Skill selector popup state
    pub skill_selector: SkillSelectorState,
    /// Subagent selector popup state
    pub subagent_selector: SubagentSelectorState,
    /// MCP selector popup state
    pub mcp_selector: McpSelectorState,
    /// MCP add dialog state
    pub mcp_add_dialog: McpAddDialogState,
    /// Provider selector popup state (step 1 of add model)
    pub provider_selector: ProviderSelectorState,
    /// Model config form state (step 2 of add model)
    pub model_config_form: ModelConfigFormState,
    /// Theme selector popup state
    pub theme_selector: ThemeSelectorState,
    /// Popup navigation stack for back navigation
    pub popup_stack: PopupStack,
}

impl PopupManager {
    pub fn new() -> Self {
        Self {
            model_selector: ModelSelectorState::new(),
            agent_selector: AgentSelectorState::new(),
            session_selector: SessionSelectorState::new(),
            skill_selector: SkillSelectorState::new(),
            subagent_selector: SubagentSelectorState::new(),
            mcp_selector: McpSelectorState::new(),
            mcp_add_dialog: McpAddDialogState::new(),
            provider_selector: ProviderSelectorState::new(),
            model_config_form: ModelConfigFormState::new(),
            theme_selector: ThemeSelectorState::new(),
            popup_stack: PopupStack::new(),
        }
    }
}

/// Tracks tool card and thinking block expand/collapse state
pub struct SelectionState {
    /// Set of collapsed tool IDs (block tools default to expanded; this tracks manually collapsed ones)
    pub collapsed_tools: HashSet<String>,
    /// Currently focused block tool ID (for Ctrl+O toggle)
    pub focused_block_tool: Option<String>,
    /// Set of assistant message IDs whose thinking blocks are collapsed
    pub collapsed_thinking: HashSet<String>,
    /// Tracks which messages have been auto-collapsed (so user re-expands won't be overridden)
    pub thinking_auto_collapsed: HashSet<String>,
    /// Tracks user manual toggles (auto-collapse won't override user intent)
    pub thinking_user_overrides: HashSet<String>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self {
            collapsed_tools: HashSet::new(),
            focused_block_tool: None,
            collapsed_thinking: HashSet::new(),
            thinking_auto_collapsed: HashSet::new(),
            thinking_user_overrides: HashSet::new(),
        }
    }
}

/// Point in text selection (line, column)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TextSelectionPoint {
    pub line: usize,
    pub col: usize,
}

/// Mouse click tracking and text selection state
pub struct MouseState {
    /// Pending command from mouse click on command menu (consumed by caller)
    pub pending_command: Option<String>,
    /// Pending theme preview selection (consumed by caller)
    pub pending_theme_preview: Option<ThemeItem>,
    /// Pending MCP toggle from mouse click (consumed by caller)
    pub pending_mcp_toggle: Option<String>,
    /// Pending skill selector action from mouse click (consumed by caller)
    pub pending_skill_action: Option<SkillSelectorAction>,
    /// Pending subagent selector action from mouse click (consumed by caller)
    pub pending_subagent_action: Option<SubagentSelectorAction>,
    /// Mouse selection anchor point in visible_plain_lines
    pub selection_anchor: Option<TextSelectionPoint>,
    /// Mouse selection focus point in visible_plain_lines
    pub selection_focus: Option<TextSelectionPoint>,
    /// Mouse down origin used to distinguish click vs drag
    pub selection_mouse_down: Option<(u16, u16)>,
    /// Whether current mouse gesture has moved enough to be treated as drag selection
    pub selection_dragged: bool,
}

impl MouseState {
    pub fn new() -> Self {
        Self {
            pending_command: None,
            pending_theme_preview: None,
            pending_mcp_toggle: None,
            pending_skill_action: None,
            pending_subagent_action: None,
            selection_anchor: None,
            selection_focus: None,
            selection_mouse_down: None,
            selection_dragged: false,
        }
    }
}

// =============================================================================
// ChatView
// =============================================================================

/// Chat interface state (input + view state only, no session data)
pub struct ChatView {
    /// Theme
    pub theme: Theme,
    /// Multiline text input component
    pub text_input: TextInput,
    /// Slash command menu state
    command_menu: CommandMenuState,
    /// Command palette state (Ctrl+P)
    command_palette: CommandPaletteState,
    /// List scroll state
    pub list_state: ListState,
    /// Whether to auto-scroll to bottom
    pub auto_scroll: bool,
    /// Loading animation
    pub spinner: Spinner,
    /// Status message
    pub status: Option<String>,
    /// Input history (for up/down arrows)
    pub input_history: VecDeque<String>,
    /// History position
    pub history_index: Option<usize>,
    /// Markdown renderer
    markdown_renderer: MarkdownRenderer,
    /// Whether in browse mode (for scrolling through history)
    pub browse_mode: bool,
    /// Message scroll offset (from bottom up)
    pub scroll_offset: usize,
    /// Popup manager (all popup states and navigation)
    pub popups: PopupManager,
    /// Selection state (tool/thinking expand-collapse)
    pub selection: SelectionState,
    /// Mouse state (pending actions, text selection)
    pub mouse: MouseState,
    /// Original theme before entering theme preview mode
    theme_preview_original: Option<Theme>,
    /// Info popup message (rendered as overlay, dismissed by any key)
    info_popup: Option<String>,
    /// Hovered thinking block (message_id) for mouse-over highlight
    hovered_thinking_block_id: Option<String>,
    /// Recorded y-coordinate regions for block tools: (tool_id, y_start, y_end)
    /// Updated each render frame for mouse click hit-testing.
    pub block_tool_regions: Vec<(String, u16, u16)>,
    /// Recorded y-coordinate regions for thinking blocks: (message_id, y_start, y_end)
    /// Updated each render frame for mouse click hit-testing.
    thinking_regions: Vec<(String, u16, u16)>,
    /// The messages area rect (for converting absolute mouse coords to relative)
    pub messages_area: Option<Rect>,
    /// Plain-text lines for the currently rendered message list subset.
    /// Index space matches the List rows before `list_state.offset`.
    visible_plain_lines: Vec<String>,

    // -- Render cache state (performance optimization) --
    /// Cached total rendered line count (updated each render frame)
    cached_total_lines: usize,
    /// Message count when cache was last updated
    cached_msg_count: usize,
    /// Terminal width when cache was last updated
    cached_width: u16,
    /// Whether the line cache needs recalculation (set true during streaming)
    lines_cache_dirty: bool,
    /// Per-message render cache: msg_id -> cached render result.
    /// Only caches completed (non-streaming) messages.
    render_cache: HashMap<String, MessageRenderEntry>,
}

impl ChatView {
    /// Create new Chat view
    pub fn new(theme: Theme) -> Self {
        let markdown_renderer = MarkdownRenderer::new(theme.clone());
        Self {
            spinner: Spinner::new(theme.style(StyleKind::Primary)),
            markdown_renderer,
            theme,
            text_input: TextInput::new(),
            command_menu: CommandMenuState::new(),
            command_palette: CommandPaletteState::new(),
            list_state: ListState::default(),
            auto_scroll: true,
            status: None,
            input_history: VecDeque::with_capacity(50),
            history_index: None,
            browse_mode: false,
            scroll_offset: 0,
            popups: PopupManager::new(),
            selection: SelectionState::new(),
            mouse: MouseState::new(),
            theme_preview_original: None,
            info_popup: None,
            hovered_thinking_block_id: None,
            block_tool_regions: Vec::new(),
            thinking_regions: Vec::new(),
            messages_area: None,
            visible_plain_lines: Vec::new(),
            cached_total_lines: 0,
            cached_msg_count: 0,
            cached_width: 0,
            lines_cache_dirty: true,
            render_cache: HashMap::new(),
        }
    }
}
