use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub last_install_path: Mutex<Option<PathBuf>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            last_install_path: Mutex::new(None),
        }
    }
}

pub const INSTALL_PATH_ERROR_PREFIX: &str = "INSTALL_PATH::";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchContext {
    pub mode: String,
    pub uninstall_path: Option<String>,
    pub app_language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPathValidation {
    pub install_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExistingInstallation {
    pub detected: bool,
    pub install_location: Option<String>,
    pub display_version: Option<String>,
    pub uninstall_string: Option<String>,
    pub main_binary_present: bool,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskSpaceInfo {
    pub total: u64,
    pub available: u64,
    pub required: u64,
    pub sufficient: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallOptions {
    pub install_path: String,
    pub desktop_shortcut: bool,
    pub start_menu: bool,
    pub launch_after_install: bool,
    pub app_language: String,
    pub theme_preference: String,
    pub model_config: Option<ModelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfig {
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model_name: String,
    pub format: String,
    pub config_name: Option<String>,
    pub custom_request_body: Option<String>,
    pub skip_ssl_verify: Option<bool>,
    pub custom_headers: Option<HashMap<String, String>>,
    pub custom_headers_mode: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    pub success: bool,
    pub response_time_ms: u64,
    pub model_response: Option<String>,
    pub message_code: Option<String>,
    pub error_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteModelInfo {
    pub id: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub step: String,
    pub percent: u32,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartInstallationRequest {
    pub options: InstallOptions,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetModelConfigRequest {
    pub model_config: ModelConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestModelConfigRequest {
    pub model_config: ModelConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListModelConfigRequest {
    pub model_config: ModelConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetThemePreferenceRequest {
    pub theme_preference: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchApplicationRequest {
    pub install_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRegisteredUninstallerRequest {
    pub uninstall_command: String,
    pub install_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallRequest {
    pub install_path: String,
    #[serde(default)]
    pub delete_user_data: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateInstallPathRequest {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDiskSpaceRequest {
    pub path: String,
}

pub fn install_path_error(code: &str) -> String {
    format!("{}{}", INSTALL_PATH_ERROR_PREFIX, code)
}
