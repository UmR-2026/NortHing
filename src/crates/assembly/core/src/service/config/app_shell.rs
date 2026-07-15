//! Application-shell-level configuration types: GlobalConfig, AppConfig, AppLogging,
//! AppSession, AIExperience, ProjectConfig, AgentCompanion, font snapshots.
//!
//! Cross-sibling: imports ThemeConfig + EditorConfig + TerminalConfig + WorkspaceConfig
//! + SidebarConfig + RightPanelConfig + NotificationConfig + AIConfig from sibling modules.

use super::ai::AIConfig;
use super::editor::EditorConfig;
use super::terminal::TerminalConfig;
use super::theme::ThemeConfig;
use super::theme::ThemesConfig;
use super::workspace::{NotificationConfig, RightPanelConfig, SidebarConfig, WorkspaceConfig};
use serde::{Deserialize, Serialize};

/// Web UI font preferences (settings → basics). Keys match `FontPreference` in the frontend (camelCase).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontPreferenceSnapshot {
    pub ui_size: UiFontSizeSnapshot,
    pub flow_chat: FlowChatFontSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiFontSizeSnapshot {
    pub level: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_px: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowChatFontSnapshot {
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_px: Option<u32>,
}

/// Global configuration structure - matches the frontend `GlobalConfig` exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    pub app: AppConfig,
    pub theme: ThemeConfig,
    pub editor: EditorConfig,
    pub terminal: TerminalConfig,
    pub workspace: WorkspaceConfig,
    pub ai: AIConfig,
    /// Project-scoped overlays stored in the shared config document.
    #[serde(default, skip_serializing_if = "ProjectConfig::is_empty")]
    pub project: ProjectConfig,
    /// MCP server configuration (stored uniformly; supports both JSON and structured formats).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<serde_json::Value>,
    /// ACP client configuration (stored as `{ "acpClients": { ... } }`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acp_clients: Option<serde_json::Value>,
    /// Theme system configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub themes: Option<ThemesConfig>,
    /// Web UI font size preferences (`get_config` / `set_config` path `font`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<FontPreferenceSnapshot>,
    pub version: String,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

/// Project-scoped configuration overlay.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    /// Project-level MCP server configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<serde_json::Value>,
}

impl ProjectConfig {
    fn is_empty(&self) -> bool {
        self.mcp_servers.is_none()
    }
}

/// App configuration.
fn default_close_button_behavior() -> String {
    "minimize_to_tray".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub language: String,
    pub auto_update: bool,
    pub telemetry: bool,
    pub startup_behavior: String,
    pub confirm_on_exit: bool,
    pub restore_windows: bool,
    pub zoom_level: f64,
    #[serde(default)]
    pub logging: AppLoggingConfig,
    pub sidebar: SidebarConfig,
    pub right_panel: RightPanelConfig,
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub session_config: AppSessionConfig,
    pub ai_experience: AIExperienceConfig,
    /// User-defined keyboard shortcut overrides.
    /// Stored as opaque JSON so the backend remains schema-agnostic;
    /// the frontend owns the versioned format (StoredKeybindingsV1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keybindings: Option<serde_json::Value>,
    /// What happens when the window close button is clicked on Windows / Linux.
    /// Allowed values: "quit" | "minimize_to_tray" | "ask".
    #[serde(default = "default_close_button_behavior")]
    pub close_button_behavior: String,
}

/// App logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppLoggingConfig {
    /// Runtime backend log level.
    /// Allowed values: trace, debug, info, warn, error, off.
    pub level: String,
    /// Whether diagnostic logs may include sensitive troubleshooting payloads.
    #[serde(default = "default_true")]
    pub include_sensitive_diagnostics: bool,
    /// Per-request AI model exchange tracing configuration for developer diagnostics.
    #[serde(default)]
    pub model_exchange_tracing: ModelExchangeTracingConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelExchangeTracingMode {
    #[default]
    Off,
    Full,
    UsageOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelExchangeTracingConfig {
    pub mode: ModelExchangeTracingMode,
}

/// Session-related UI preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSessionConfig {
    /// Default new session mode used by the frontend.
    /// Supported values: "code", "cowork".
    pub default_mode: String,
}

/// A user-defined quick action for the FlowChat post-coding actions menu.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiExperienceQuickAction {
    pub id: String,
    pub label: String,
    pub prompt: String,
    pub enabled: bool,
}

/// AI experience configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AIExperienceConfig {
    /// Whether to enable automatic AI-generated summaries for session titles.
    pub enable_session_title_generation: bool,
    /// Whether to enable AI analysis of work status on the FlowChat welcome page.
    pub enable_welcome_panel_ai_analysis: bool,
    /// Whether to enable visual mode.
    pub enable_visual_mode: bool,
    /// Whether to show the pixel Agent companion in the collapsed chat input.
    pub enable_agent_companion: bool,
    /// Where to show the Agent companion: "input" or "desktop".
    pub agent_companion_display_mode: String,
    /// Optional Petdex-compatible companion package selected by the user.
    #[serde(default = "default_agent_companion_pet", skip_serializing_if = "Option::is_none")]
    pub agent_companion_pet: Option<AgentCompanionPetSelection>,
    /// Whether to enable flashgrep-backed accelerated workspace search.
    pub enable_workspace_search: bool,
    /// User-defined quick actions (post-coding menu); persisted for the web UI.
    #[serde(default)]
    pub quick_actions: Vec<AiExperienceQuickAction>,
}

/// User-selected Agent companion pet package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentCompanionPetSelection {
    pub id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source: String,
    pub package_path: String,
    pub spritesheet_path: String,
    pub spritesheet_mime_type: String,
}

fn default_agent_companion_pet() -> Option<AgentCompanionPetSelection> {
    Some(AgentCompanionPetSelection {
        id: "northhing".to_string(),
        display_name: "northhing".to_string(),
        description: Some(
            "northhing's mascot — Bifang, a figure from Chinese mythology said to live on Mount Zhang'e. In the Classic of Mountains and Seas (Shan Hai Jing · Western Mountains), Bifang is described as crane-like with one foot, blue feathers marked with red, and a white beak.".to_string(),
        ),
        source: "preset".to_string(),
        package_path: "/agent-companion-pets/northhing".to_string(),
        spritesheet_path: "/agent-companion-pets/northhing/spritesheet.webp".to_string(),
        spritesheet_mime_type: "image/webp".to_string(),
    })
}

fn default_true() -> bool {
    true
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            theme: ThemeConfig::default(),
            editor: EditorConfig::default(),
            terminal: TerminalConfig::default(),
            workspace: WorkspaceConfig::default(),
            ai: AIConfig::default(),
            project: ProjectConfig::default(),
            mcp_servers: None,
            acp_clients: None,
            themes: Some(ThemesConfig::default()),
            font: None,
            version: "1.0.0".to_string(),
            last_modified: chrono::Utc::now(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: "zh-CN".to_string(),
            auto_update: true,
            telemetry: false,
            startup_behavior: "lastWorkspace".to_string(),
            confirm_on_exit: true,
            restore_windows: true,
            zoom_level: 1.0,
            logging: AppLoggingConfig::default(),
            sidebar: SidebarConfig {
                width: 300,
                collapsed: false,
            },
            right_panel: RightPanelConfig {
                width: 400,
                collapsed: true,
            },
            notifications: NotificationConfig {
                enabled: true,
                position: "topRight".to_string(),
                duration: 5000,
                dialog_completion_notify: true,
                enable_startup_tips: true,
            },
            session_config: AppSessionConfig::default(),
            ai_experience: AIExperienceConfig::default(),
            keybindings: None,
            close_button_behavior: default_close_button_behavior(),
        }
    }
}

impl Default for AppLoggingConfig {
    fn default() -> Self {
        Self {
            // Set to Debug in early development for easier diagnostics
            level: "debug".to_string(),
            include_sensitive_diagnostics: true,
            model_exchange_tracing: ModelExchangeTracingConfig::default(),
        }
    }
}

impl Default for ModelExchangeTracingConfig {
    fn default() -> Self {
        Self {
            mode: ModelExchangeTracingMode::Off,
        }
    }
}

impl Default for AppSessionConfig {
    fn default() -> Self {
        Self {
            default_mode: "code".to_string(),
        }
    }
}

impl Default for AIExperienceConfig {
    fn default() -> Self {
        Self {
            enable_session_title_generation: true,
            enable_welcome_panel_ai_analysis: false,
            enable_visual_mode: false,
            enable_agent_companion: true,
            agent_companion_display_mode: "desktop".to_string(),
            agent_companion_pet: default_agent_companion_pet(),
            enable_workspace_search: false,
            quick_actions: Vec::new(),
        }
    }
}
