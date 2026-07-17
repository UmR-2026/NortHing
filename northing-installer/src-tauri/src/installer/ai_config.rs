use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::installer::types::ModelConfig;

const APP_CONFIG_DIR: &str = "northhing";
const APP_CONFIG_FILE: &str = "app.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModelCategory {
    GeneralChat,
    Multimodal,
    ImageGeneration,
    Embedding,
    SearchEnhanced,
    CodeSpecialized,
    SpeechRecognition,
}

impl Default for ModelCategory {
    fn default() -> Self {
        ModelCategory::GeneralChat
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    TextChat,
    ImageUnderstanding,
    ImageGeneration,
    Embedding,
    Search,
    CodeSpecialized,
    FunctionCalling,
    SpeechRecognition,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    #[default]
    ApiKey,
    CodexCli,
    GeminiCli,
}

fn deserialize_category<'de, D>(deserializer: D) -> Result<ModelCategory, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    Ok(match s.as_deref() {
        Some("multimodal") => ModelCategory::Multimodal,
        Some("image_generation") => ModelCategory::ImageGeneration,
        Some("embedding") => ModelCategory::Embedding,
        Some("search_enhanced") => ModelCategory::SearchEnhanced,
        Some("code_specialized") => ModelCategory::CodeSpecialized,
        Some("speech_recognition") => ModelCategory::SpeechRecognition,
        _ => ModelCategory::GeneralChat,
    })
}

fn deserialize_capabilities<'de, D>(deserializer: D) -> Result<Vec<ModelCapability>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let items = Option::<Vec<String>>::deserialize(deserializer)?;
    Ok(items
        .unwrap_or_default()
        .into_iter()
        .filter_map(|s| match s.as_str() {
            "text_chat" => Some(ModelCapability::TextChat),
            "image_understanding" => Some(ModelCapability::ImageUnderstanding),
            "image_generation" => Some(ModelCapability::ImageGeneration),
            "embedding" => Some(ModelCapability::Embedding),
            "search" => Some(ModelCapability::Search),
            "code_specialized" => Some(ModelCapability::CodeSpecialized),
            "function_calling" => Some(ModelCapability::FunctionCalling),
            "speech_recognition" => Some(ModelCapability::SpeechRecognition),
            _ => None,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    #[serde(default)]
    pub ai: AiConfigSection,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ai: AiConfigSection::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfigSection {
    #[serde(default)]
    pub models: Vec<ModelEntry>,
}

impl Default for AiConfigSection {
    fn default() -> Self {
        Self { models: Vec::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelEntry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub model_name: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub request_url: Option<String>,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub context_window: Option<u32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub top_p: Option<f64>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, deserialize_with = "deserialize_category")]
    pub category: ModelCategory,
    #[serde(default, deserialize_with = "deserialize_capabilities")]
    pub capabilities: Vec<ModelCapability>,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub recommended_for: Vec<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub reasoning_mode: Option<String>,
    #[serde(default)]
    pub inline_think_in_text: bool,
    #[serde(default)]
    pub custom_headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub custom_headers_mode: Option<String>,
    #[serde(default)]
    pub skip_ssl_verify: bool,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub thinking_budget_tokens: Option<u32>,
    #[serde(default)]
    pub custom_request_body: Option<String>,
    #[serde(default)]
    pub custom_request_body_mode: Option<String>,
}

impl Default for ModelEntry {
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
            enabled: true,
            category: ModelCategory::GeneralChat,
            capabilities: vec![ModelCapability::TextChat, ModelCapability::FunctionCalling],
            auth: AuthConfig::ApiKey,
            recommended_for: Vec::new(),
            metadata: None,
            reasoning_mode: None,
            inline_think_in_text: true,
            custom_headers: None,
            custom_headers_mode: None,
            skip_ssl_verify: false,
            reasoning_effort: None,
            thinking_budget_tokens: None,
            custom_request_body: None,
            custom_request_body_mode: None,
        }
    }
}

pub fn app_config_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("NORTHHING_INSTALLER_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    dirs::config_dir().map(|d| d.join(APP_CONFIG_DIR).join("config"))
}

pub fn app_config_path() -> Option<PathBuf> {
    app_config_dir().map(|d| d.join(APP_CONFIG_FILE))
}

pub fn load_app_config() -> AppConfig {
    let Some(path) = app_config_path() else {
        return AppConfig::default();
    };
    let Ok(raw) = fs::read_to_string(&path) else {
        return AppConfig::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_app_config(config: &AppConfig) -> Result<()> {
    let dir = app_config_dir().context("failed to resolve app config dir")?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create config dir: {}", dir.display()))?;
    let path = dir.join(APP_CONFIG_FILE);
    let raw = serde_json::to_string_pretty(config).context("failed to serialize app config")?;
    fs::write(&path, raw)
        .with_context(|| format!("failed to write app config: {}", path.display()))?;
    Ok(())
}

/// Read the raw on-disk config as a JSON value (empty object if missing/invalid).
fn read_raw_config() -> serde_json::Value {
    let Some(path) = app_config_path() else {
        return serde_json::Value::Object(serde_json::Map::new());
    };
    let Ok(raw) = fs::read_to_string(&path) else {
        return serde_json::Value::Object(serde_json::Map::new());
    };
    serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()))
}

/// Merge a JSON patch into the on-disk config, preserving all other keys.
fn merge_app_config(patch: serde_json::Value) -> Result<()> {
    let dir = app_config_dir().context("failed to resolve app config dir")?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create config dir: {}", dir.display()))?;
    let path = dir.join(APP_CONFIG_FILE);

    let mut existing = read_raw_config();
    if !existing.is_object() {
        existing = serde_json::Value::Object(serde_json::Map::new());
    }
    if let (Some(existing_obj), Some(patch_obj)) = (existing.as_object_mut(), patch.as_object()) {
        for (key, value) in patch_obj {
            existing_obj.insert(key.clone(), value.clone());
        }
    }

    let raw = serde_json::to_string_pretty(&existing)
        .context("failed to serialize merged config")?;
    fs::write(&path, raw)
        .with_context(|| format!("failed to write app config: {}", path.display()))?;
    Ok(())
}

pub fn model_config_to_entry(config: &ModelConfig) -> ModelEntry {
    let id = if config.config_name.as_deref().unwrap_or("").is_empty() {
        format!("{}-{}", config.provider, config.model_name)
    } else {
        config.config_name.clone().unwrap_or_default()
    };
    ModelEntry {
        id,
        name: config.config_name.clone().unwrap_or_else(|| config.provider.clone()),
        provider: config.provider.clone(),
        model_name: config.model_name.clone(),
        base_url: config.base_url.clone(),
        request_url: None,
        api_key: config.api_key.clone(),
        context_window: None,
        max_tokens: None,
        temperature: None,
        top_p: None,
        enabled: true,
        category: match config.category.as_deref() {
            Some("multimodal") => ModelCategory::Multimodal,
            Some("image_generation") => ModelCategory::ImageGeneration,
            Some("embedding") => ModelCategory::Embedding,
            Some("search_enhanced") => ModelCategory::SearchEnhanced,
            Some("code_specialized") => ModelCategory::CodeSpecialized,
            Some("speech_recognition") => ModelCategory::SpeechRecognition,
            Some("general_chat") | None | _ => ModelCategory::GeneralChat,
        },
        capabilities: config
            .capabilities
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| match s.as_str() {
                "text_chat" => Some(ModelCapability::TextChat),
                "image_understanding" => Some(ModelCapability::ImageUnderstanding),
                "image_generation" => Some(ModelCapability::ImageGeneration),
                "embedding" => Some(ModelCapability::Embedding),
                "search" => Some(ModelCapability::Search),
                "code_specialized" => Some(ModelCapability::CodeSpecialized),
                "function_calling" => Some(ModelCapability::FunctionCalling),
                "speech_recognition" => Some(ModelCapability::SpeechRecognition),
                _ => None,
            })
            .collect(),
        auth: AuthConfig::ApiKey,
        recommended_for: Vec::new(),
        metadata: None,
        reasoning_mode: None,
        inline_think_in_text: true,
        custom_headers: config.custom_headers.clone(),
        custom_headers_mode: config.custom_headers_mode.clone(),
        skip_ssl_verify: config.skip_ssl_verify.unwrap_or(false),
        reasoning_effort: None,
        thinking_budget_tokens: None,
        custom_request_body: config.custom_request_body.clone(),
        custom_request_body_mode: None,
    }
}

pub fn write_model_config(config: &ModelConfig) -> Result<()> {
    let entry = model_config_to_entry(config);
    let entry_value = serde_json::to_value(&entry)
        .context("failed to serialize model entry")?;

    let existing = read_raw_config();
    let mut models = match existing.get("ai").and_then(|ai| ai.get("models")) {
        Some(serde_json::Value::Array(arr)) => arr.clone(),
        _ => Vec::new(),
    };

    let replaced = if let Some(pos) = models.iter().position(|m| {
        m.get("id").and_then(|v| v.as_str()) == Some(entry.id.as_str())
    }) {
        models[pos] = entry_value;
        true
    } else {
        models.push(entry_value);
        false
    };
    let _ = replaced;

    merge_app_config(serde_json::json!({ "ai": { "models": models } }))
}

pub fn write_theme_preference(theme_id: &str) -> Result<()> {
    merge_app_config(serde_json::json!({ "theme": { "theme": { "current": theme_id } } }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_temp_dir() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "northhing-installer-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        p
    }

    fn make_model_config(id: &str) -> ModelConfig {
        ModelConfig {
            provider: "test-provider".to_string(),
            api_key: "test-api-key".to_string(),
            base_url: "https://example.com/v1".to_string(),
            model_name: "test-model".to_string(),
            format: "openai".to_string(),
            config_name: Some(id.to_string()),
            custom_request_body: None,
            skip_ssl_verify: None,
            custom_headers: None,
            custom_headers_mode: None,
            capabilities: Some(vec!["text_chat".to_string()]),
            category: Some("general_chat".to_string()),
        }
    }

    #[test]
    fn write_model_then_theme_preserves_both() {
        let dir = unique_temp_dir();
        let _ = fs::remove_dir_all(&dir);
        std::env::set_var("NORTHHING_INSTALLER_CONFIG_DIR", &dir);

        let model = make_model_config("model-1");
        write_model_config(&model).expect("write_model_config should succeed");
        write_theme_preference("northhing-dark").expect("write_theme_preference should succeed");

        let path = app_config_path().expect("app_config_path should resolve");
        let raw = fs::read_to_string(&path).expect("config file should exist");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("config should be valid json");

        let models = value
            .get("ai")
            .and_then(|ai| ai.get("models"))
            .and_then(|m| m.as_array())
            .expect("ai.models should exist");
        assert_eq!(models.len(), 1);
        assert_eq!(
            models[0].get("id").and_then(|v| v.as_str()),
            Some("model-1")
        );

        let theme = value
            .get("theme")
            .and_then(|t| t.get("theme"))
            .and_then(|t| t.get("current"))
            .and_then(|c| c.as_str())
            .expect("theme.theme.current should exist");
        assert_eq!(theme, "northhing-dark");

        let _ = fs::remove_dir_all(&dir);
        std::env::remove_var("NORTHHING_INSTALLER_CONFIG_DIR");
    }

    #[test]
    fn write_theme_then_model_preserves_both() {
        let dir = unique_temp_dir();
        let _ = fs::remove_dir_all(&dir);
        std::env::set_var("NORTHHING_INSTALLER_CONFIG_DIR", &dir);

        write_theme_preference("northhing-light").expect("write_theme_preference should succeed");
        let model = make_model_config("model-2");
        write_model_config(&model).expect("write_model_config should succeed");

        let path = app_config_path().expect("app_config_path should resolve");
        let raw = fs::read_to_string(&path).expect("config file should exist");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("config should be valid json");

        let models = value
            .get("ai")
            .and_then(|ai| ai.get("models"))
            .and_then(|m| m.as_array())
            .expect("ai.models should exist");
        assert_eq!(models.len(), 1);
        assert_eq!(
            models[0].get("id").and_then(|v| v.as_str()),
            Some("model-2")
        );

        let theme = value
            .get("theme")
            .and_then(|t| t.get("theme"))
            .and_then(|t| t.get("current"))
            .and_then(|c| c.as_str())
            .expect("theme.theme.current should exist");
        assert_eq!(theme, "northhing-light");

        let _ = fs::remove_dir_all(&dir);
        std::env::remove_var("NORTHHING_INSTALLER_CONFIG_DIR");
    }
}
