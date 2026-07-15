//! Startup page module
//!
//! Full-featured startup page with:
//! - Centered logo and input box
//! - Slash command menu with real execution
//! - Model/Agent/Session/Skill/Subagent selector popups
//! - Random tips
//!
//! Split into 4 sibling submodules (R37i northhing-cli god-split):
//! - [`types`]: PopupType / PopupStack / StartupResult DTOs
//! - [`render`]: Frame layout and widget rendering
//! - [`input`]: Keyboard, command, palette event handling
//! - [`selectors`]: Selector popup show/apply/save/edit logic

use std::sync::Arc;

use northhing_core::agentic::coordination::ConversationCoordinator;

use super::agent_selector::AgentSelectorState;
use super::command_menu::CommandMenuState;
use super::command_palette::CommandPaletteState;
use super::model_config_form::ModelConfigFormState;
use super::model_selector::ModelSelectorState;
use super::provider_selector::ProviderSelectorState;
use super::session_selector::SessionSelectorState;
use super::skill_selector::SkillSelectorState;
use super::subagent_selector::SubagentSelectorState;
use super::text_input::TextInput;
use super::theme::{
    builtin_theme_json, resolve_appearance, resolve_effective_color_scheme, EffectiveColorScheme, Theme,
};
use super::theme_selector::ThemeSelectorState;
use crate::config::CliConfig;

pub mod input;
pub mod render;
pub mod selectors;
pub mod types;

pub use types::*;

// ── Shared constants (defined in facade so all sibling impl blocks can access via "super::") ──

/// Keyboard shortcuts help text for startup page
const KEYBOARD_SHORTCUTS_HELP: &str = "\
Keyboard Shortcuts\n\
─────────────────────────────────\n\
Tab / Shift+Tab   Switch Agent\n\
Ctrl+P            Command Palette\n\
Esc               Back / Interrupt\n\
Ctrl+W            Close All Windows\n\
Ctrl+C            Exit";

/// Random tips shown on the startup page
const TIPS: &[&str] = &[
    "Type / for slash commands (e.g. /help, /models, /agents)",
    "Press Tab to cycle between agents",
    "Use /init to explore your repo and generate AGENTS.md",
    "Press Ctrl+E to toggle browse mode for scrolling history",
    "Use /sessions to list and continue previous conversations",
    "Press Ctrl+O to expand/collapse tool output",
    "Use /skills to browse and execute available skills",
    "Use /usage inside a session to generate a usage report",
    "Use /theme to switch the CLI theme",
    "Use /acp to copy editor setup commands for ACP hosts",
    "Press Up/Down to cycle through input history",
    "Use /new to start a fresh conversation session",
];

/// Startup page
pub struct StartupPage {
    /// Multiline text input component
    text_input: TextInput,
    /// Theme
    theme: Theme,
    /// CLI config, including persisted theme preference.
    config: CliConfig,
    /// Current tip text
    tip: &'static str,

    // ── Command menu ──
    command_menu: CommandMenuState,

    // ── Command palette (Ctrl+P) ──
    command_palette: CommandPaletteState,

    // ── Selector popups ──
    model_selector: ModelSelectorState,
    agent_selector: AgentSelectorState,
    session_selector: SessionSelectorState,
    skill_selector: SkillSelectorState,
    subagent_selector: SubagentSelectorState,
    theme_selector: ThemeSelectorState,
    provider_selector: ProviderSelectorState,
    model_config_form: ModelConfigFormState,
    theme_preview_original: Option<Theme>,

    // ── System context ──
    coordinator: Arc<ConversationCoordinator>,

    // ── State ──
    /// Selected agent type (can be changed via /agents or Tab)
    agent_type: String,
    /// Display name of selected model
    model_display_name: String,
    /// Workspace path for display in bottom bar
    workspace_display: String,
    /// Status message (temporarily shown instead of tip)
    status: Option<String>,
    /// Info popup message (rendered as overlay, dismissed by any key)
    info_popup: Option<String>,

    /// Popup navigation stack for back navigation
    popup_stack: PopupStack,
}

impl StartupPage {
    pub fn new(coordinator: Arc<ConversationCoordinator>, default_agent: String, workspace: Option<String>) -> Self {
        let config = CliConfig::load().unwrap_or_default();
        let appearance = resolve_appearance(&config.ui.theme);
        let scheme = resolve_effective_color_scheme(&config.ui.color_scheme);
        let base_is_light = appearance.is_light();
        let base = match (base_is_light, scheme) {
            (_, EffectiveColorScheme::Monochrome) => Theme::monochrome(),
            (true, EffectiveColorScheme::Ansi16) => Theme::light_ansi16(),
            (true, EffectiveColorScheme::Truecolor) => Theme::light(),
            (false, EffectiveColorScheme::Ansi16) => Theme::dark_ansi16(),
            (false, EffectiveColorScheme::Truecolor) => Theme::dark(),
        };
        let theme = if scheme == EffectiveColorScheme::Monochrome {
            Theme::monochrome()
        } else {
            let id = config.ui.theme_id.trim();
            if id.is_empty() {
                base
            } else if let Some(json) = builtin_theme_json(id) {
                base.apply_opencode_theme_json(json, appearance)
                    .unwrap_or(base)
                    .with_effective_scheme(scheme)
            } else {
                base
            }
        };

        let tip_index = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as usize
            % TIPS.len();

        let mut page = Self {
            text_input: TextInput::new(),
            theme,
            config,
            tip: TIPS[tip_index],
            command_menu: CommandMenuState::new(),
            command_palette: CommandPaletteState::new(),
            model_selector: ModelSelectorState::new(),
            agent_selector: AgentSelectorState::new(),
            session_selector: SessionSelectorState::new(),
            skill_selector: SkillSelectorState::new(),
            subagent_selector: SubagentSelectorState::new(),
            theme_selector: ThemeSelectorState::new(),
            provider_selector: ProviderSelectorState::new(),
            model_config_form: ModelConfigFormState::new(),
            theme_preview_original: None,
            coordinator,
            agent_type: default_agent,
            model_display_name: String::new(),
            workspace_display: workspace.unwrap_or_else(|| {
                std::env::current_dir()
                    .ok()
                    .and_then(|p| dunce::canonicalize(&p).ok())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| ".".to_string())
            }),
            status: None,
            info_popup: None,
            popup_stack: PopupStack::new(),
        };

        // Load current model name
        page.load_current_model_name();
        page
    }

    /// Get the currently selected agent type
    pub fn agent_type(&self) -> &str {
        &self.agent_type
    }

    /// Get the current workspace path for this CLI process.
    pub fn workspace(&self) -> Option<String> {
        if self.workspace_display.is_empty() {
            None
        } else {
            Some(self.workspace_display.clone())
        }
    }

    /// Get the current CLI config after startup-page edits.
    pub fn config(&self) -> &CliConfig {
        &self.config
    }

    pub(super) fn workspace_path_buf(&self) -> std::path::PathBuf {
        self.workspace()
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    }

    /// Check if any popup is currently visible
    pub(super) fn any_popup_visible(&self) -> bool {
        self.command_palette.is_visible()
            || self.model_selector.is_visible()
            || self.agent_selector.is_visible()
            || self.session_selector.is_visible()
            || self.skill_selector.is_visible()
            || self.subagent_selector.is_visible()
            || self.theme_selector.is_visible()
            || self.provider_selector.is_visible()
            || self.model_config_form.is_visible()
    }
}
