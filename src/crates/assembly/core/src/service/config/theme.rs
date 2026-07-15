//! Theme configuration types.
//!
//! Standalone sibling — no cross-sibling dependencies.

use serde::{Deserialize, Serialize};

/// Theme configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub id: String,
    pub name: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub theme_type: String,
    pub colors: ThemeColors,
    pub fonts: ThemeFonts,
    pub spacing: ThemeSpacing,
    pub border_radius: ThemeBorderRadius,
    pub shadows: ThemeShadows,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeColors {
    pub primary: String,
    pub secondary: String,
    pub background: String,
    pub surface: String,
    pub text: String,
    pub text_secondary: String,
    pub border: String,
    pub accent: String,
    pub success: String,
    pub warning: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeFonts {
    pub primary: String,
    pub code: String,
    pub sizes: FontSizes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FontSizes {
    pub xs: String,
    pub sm: String,
    pub base: String,
    pub lg: String,
    pub xl: String,
    #[serde(rename = "2xl")]
    pub xxl: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeSpacing {
    pub xs: String,
    pub sm: String,
    pub md: String,
    pub lg: String,
    pub xl: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeBorderRadius {
    pub sm: String,
    pub md: String,
    pub lg: String,
    pub full: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeShadows {
    pub sm: String,
    pub md: String,
    pub lg: String,
}

/// Theme system configuration (new).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemesConfig {
    /// Currently active theme ID.
    pub current: String,
    /// User-defined themes (stored as JSON).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<serde_json::Value>,
}

impl Default for ThemesConfig {
    fn default() -> Self {
        Self {
            current: "northhing-light".to_string(),
            custom: None,
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            id: "dark".to_string(),
            name: "dark".to_string(),
            display_name: "Dark Theme".to_string(),
            theme_type: "dark".to_string(),
            colors: ThemeColors::default(),
            fonts: ThemeFonts::default(),
            spacing: ThemeSpacing::default(),
            border_radius: ThemeBorderRadius::default(),
            shadows: ThemeShadows::default(),
        }
    }
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            primary: "#007acc".to_string(),
            secondary: "#6c757d".to_string(),
            background: "#1e1e1e".to_string(),
            surface: "#2d2d30".to_string(),
            text: "#cccccc".to_string(),
            text_secondary: "#969696".to_string(),
            border: "#3e3e42".to_string(),
            accent: "#007acc".to_string(),
            success: "#28a745".to_string(),
            warning: "#ffc107".to_string(),
            error: "#dc3545".to_string(),
        }
    }
}

impl Default for ThemeFonts {
    fn default() -> Self {
        Self {
            primary: "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif".to_string(),
            code: "Consolas, \"Courier New\", monospace".to_string(),
            sizes: FontSizes::default(),
        }
    }
}

impl Default for FontSizes {
    fn default() -> Self {
        Self {
            xs: "0.75rem".to_string(),
            sm: "0.875rem".to_string(),
            base: "1rem".to_string(),
            lg: "1.125rem".to_string(),
            xl: "1.25rem".to_string(),
            xxl: "1.5rem".to_string(),
        }
    }
}

impl Default for ThemeSpacing {
    fn default() -> Self {
        Self {
            xs: "0.25rem".to_string(),
            sm: "0.5rem".to_string(),
            md: "1rem".to_string(),
            lg: "1.5rem".to_string(),
            xl: "2rem".to_string(),
        }
    }
}

impl Default for ThemeBorderRadius {
    fn default() -> Self {
        Self {
            sm: "0.125rem".to_string(),
            md: "0.25rem".to_string(),
            lg: "0.5rem".to_string(),
            full: "9999px".to_string(),
        }
    }
}

impl Default for ThemeShadows {
    fn default() -> Self {
        Self {
            sm: "0 1px 2px 0 rgba(0, 0, 0, 0.05)".to_string(),
            md: "0 4px 6px -1px rgba(0, 0, 0, 0.1)".to_string(),
            lg: "0 10px 15px -3px rgba(0, 0, 0, 0.1)".to_string(),
        }
    }
}
