//! Workspace and panel layout configuration types.
//!
//! Cross-sibling: imports EditorConfig + MinimapConfig + TerminalConfig + TerminalThemeConfig
//! from sibling modules.

use super::editor::{EditorConfig, MinimapConfig};
use super::terminal::{TerminalConfig, TerminalThemeConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SidebarConfig {
    pub width: u32,
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RightPanelConfig {
    pub width: u32,
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub position: String,
    pub duration: u32,
    /// Whether to show a toast notification when a dialog turn completes while the window is not focused.
    #[serde(default = "default_true")]
    pub dialog_completion_notify: bool,
    /// Whether to show built-in tip cards on startup (can be disabled by the user).
    #[serde(default = "default_true")]
    pub enable_startup_tips: bool,
}

/// Workspace configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceConfig {
    pub exclude_patterns: Vec<String>,
    pub include_patterns: Vec<String>,
    pub watch_ignore: Vec<String>,
    /// Maximum file size in bytes.
    pub max_file_size: u64,
    pub encoding: String,
    pub line_ending: String,
    pub trim_trailing_whitespace: bool,
    pub insert_final_newline: bool,
}

fn default_true() -> bool {
    true
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            exclude_patterns: vec![
                "**/node_modules/**".to_string(),
                "**/target/**".to_string(),
                "**/.git/**".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
            ],
            include_patterns: vec!["**/*".to_string()],
            watch_ignore: vec![
                "**/node_modules/**".to_string(),
                "**/target/**".to_string(),
                "**/.git/**".to_string(),
            ],
            max_file_size: 50 * 1024 * 1024,
            encoding: "utf8".to_string(),
            line_ending: "auto".to_string(),
            trim_trailing_whitespace: true,
            insert_final_newline: true,
        }
    }
}

impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            width: 300,
            collapsed: false,
        }
    }
}

impl Default for RightPanelConfig {
    fn default() -> Self {
        Self {
            width: 400,
            collapsed: true,
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: "topRight".to_string(),
            duration: 5000,
            dialog_completion_notify: true,
            enable_startup_tips: true,
        }
    }
}

// Compile-time touch references to ensure cross-sibling types remain visible.
#[allow(dead_code)]
fn _ensure_cross_sibling_refs(_: EditorConfig, _: MinimapConfig, _: TerminalConfig, _: TerminalThemeConfig) {}
