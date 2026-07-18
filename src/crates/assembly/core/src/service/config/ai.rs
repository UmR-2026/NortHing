//! AI configuration types: ModelCapability, ModelCategory, DefaultModelsConfig,
//! ReviewTeamConfig, AIConfig (with 5 inherent methods), AgentProfileConfig/View,
//! ConfirmationMode, ShellSecurityConfig + free fn `deserialize_agent_profiles` +
//! DEFAULT_MAX_ROUNDS constant + default_* helpers.
//!
//! Cross-sibling: imports AIModelConfig (runtime), ModelCapability/ModelCategory (runtime),
//! DebugModeConfig (runtime), ShellSecurityConfig local, ParentSubagentOverrideConfig (runtime).
//! Re-exports `northhing_core_types::ReasoningMode` for `AIConfig.agent_profiles` etc.

use super::runtime::{DebugModeConfig, ParentSubagentOverrideConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use northhing_core_types::ReasoningMode;

/// Model capability type (a model can have multiple capabilities).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    /// Text chat (primary capability).
    TextChat,
    /// Image understanding (vision).
    ImageUnderstanding,
    /// Image generation.
    ImageGeneration,
    /// Embeddings (semantic vectors).
    Embedding,
    /// Search API (e.g. Perplexity).
    Search,
    /// Code specialized.
    CodeSpecialized,
    /// Function calling / tool use.
    FunctionCalling,
    /// Speech-to-text.
    SpeechRecognition,
}

/// Model category (for UI display and filtering).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ModelCategory {
    /// General chat model.
    #[default]
    GeneralChat,
    /// Multimodal model (text + image understanding).
    Multimodal,
    /// Image generation model.
    ImageGeneration,
    /// Embedding / vector model.
    Embedding,
    /// Search-enhanced model.
    SearchEnhanced,
    /// Code-specialized model.
    CodeSpecialized,
    /// Speech recognition model.
    SpeechRecognition,
}

/// Default model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct DefaultModelsConfig {
    /// Primary model ID (for complex tasks).
    pub primary: Option<String>,
    /// Fast model ID (for simple tasks).
    pub fast: Option<String>,
    /// Search model.
    pub search: Option<String>,
    /// Image understanding model.
    pub image_understanding: Option<String>,
    /// Image generation model.
    pub image_generation: Option<String>,
    /// Speech recognition model.
    pub speech_recognition: Option<String>,
}

/// Default review-team execution policy and membership configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ReviewTeamConfig {
    /// Additional reviewer subagent IDs configured by the user.
    pub extra_subagent_ids: Vec<String>,
    /// Default review depth used by the whole review team.
    pub strategy_level: String,
    /// Per-reviewer review depth overrides keyed by subagent ID.
    pub member_strategy_overrides: HashMap<String, String>,
    /// Optional timeout applied to reviewer Task calls. 0 disables the cap.
    pub reviewer_timeout_seconds: u64,
    /// Optional timeout applied to ReviewJudge Task calls. 0 disables the cap.
    pub judge_timeout_seconds: u64,
    /// Whether ReviewFixer may be launched by DeepReview.
    pub auto_fix_enabled: bool,
    /// Minimum number of target files that triggers same-role reviewer splitting.
    /// 0 disables file splitting.
    pub reviewer_file_split_threshold: usize,
    /// Maximum number of same-role reviewer instances per role when file splitting is active.
    pub max_same_role_instances: usize,
}

impl Default for ReviewTeamConfig {
    fn default() -> Self {
        Self {
            extra_subagent_ids: Vec::new(),
            strategy_level: "normal".to_string(),
            member_strategy_overrides: HashMap::new(),
            reviewer_timeout_seconds: 3600,
            judge_timeout_seconds: 2400,
            auto_fix_enabled: false,
            reviewer_file_split_threshold: 20,
            max_same_role_instances: 3,
        }
    }
}

fn default_review_team_configs() -> HashMap<String, ReviewTeamConfig> {
    HashMap::from([("default".to_string(), ReviewTeamConfig::default())])
}

fn default_review_team_rate_limit_status() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

pub use northhing_core_types::ProxyConfig;

/// AI configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AIConfig {
    /// All configured models.
    pub models: Vec<super::runtime::AIModelConfig>,

    /// Model mapping for primary agents (e.g. Explore, FileFinder).
    /// agent_type -> model_id
    pub agent_models: HashMap<String, String>,

    /// Model mapping for functional agents (e.g. startchat-func-agent, session-title-func-agent).
    /// func_agent_name -> model_id
    #[serde(default)]
    pub func_agent_models: HashMap<String, String>,

    /// Default model configuration.
    #[serde(default)]
    pub default_models: DefaultModelsConfig,

    /// Shared agent-profile configuration.
    /// profile_id -> AgentProfileConfig
    #[serde(default, deserialize_with = "deserialize_agent_profiles")]
    pub agent_profiles: HashMap<String, AgentProfileConfig>,

    /// Review team configuration.
    /// team_id -> ReviewTeamConfig
    #[serde(default = "default_review_team_configs")]
    pub review_teams: HashMap<String, ReviewTeamConfig>,

    /// Runtime rate-limit snapshot for Review Team launches.
    #[serde(default = "default_review_team_rate_limit_status")]
    pub review_team_rate_limit_status: serde_json::Value,

    /// Workspace path -> Review Team strategy override.
    #[serde(default)]
    pub review_team_project_strategy_overrides: HashMap<String, String>,

    /// Maximum number of subagents that may execute concurrently.
    #[serde(default = "default_subagent_max_concurrency")]
    pub subagent_max_concurrency: usize,

    /// Global proxy configuration.
    pub proxy: ProxyConfig,

    /// Streaming idle timeout in seconds; `None` means wait indefinitely.
    #[serde(default = "default_stream_idle_timeout")]
    pub stream_idle_timeout_secs: Option<u64>,

    /// Time-to-first-token timeout in seconds while opening a streaming request;
    /// `None` means wait indefinitely.
    #[serde(default = "default_stream_ttft_timeout")]
    pub stream_ttft_timeout_secs: Option<u64>,

    /// Tool execution timeout in seconds; `None` means wait indefinitely.
    #[serde(default = "default_tool_execution_timeout")]
    pub tool_execution_timeout_secs: Option<u64>,

    /// Tool confirmation timeout in seconds; `None` means wait indefinitely.
    #[serde(default = "default_tool_confirmation_timeout")]
    pub tool_confirmation_timeout_secs: Option<u64>,

    /// Skip tool execution confirmation (global, applies to all modes).
    /// Deprecated: use `shell_security.confirmation_mode` instead.
    /// This field is kept for backward compatibility and acts as a fallback
    /// when `shell_security` is not set.
    #[serde(default = "default_skip_tool_confirmation")]
    pub skip_tool_confirmation: bool,

    /// Shell security configuration (R1 Phase 3).
    ///
    /// Replaces the global `skip_tool_confirmation` with mode-based control.
    /// Default: `Permissive` (current behavior — skip confirmation in coding modes).
    #[serde(default)]
    pub shell_security: ShellSecurityConfig,

    /// Debug-mode configuration (log path, language templates, etc.).
    #[serde(default)]
    pub debug_mode_config: DebugModeConfig,

    /// Allow Computer use (desktop automation) when the desktop host is available (all session modes).
    #[serde(default)]
    pub computer_use_enabled: bool,

    /// Preferred browser for CDP browser control. Empty/default uses the system default browser.
    #[serde(default)]
    pub browser_control_preferred_browser: String,

    /// Maximum number of rounds per dialog turn before soft-pausing.
    #[serde(default = "default_max_rounds")]
    pub max_rounds: usize,
}

impl AIConfig {
    /// Resolves a configured model reference by `id`, `name`, or `model_name`.
    ///
    /// Returns the model id only when the matched model is `enabled`. This is the
    /// single source of truth for "is this model usable right now?" and is the
    /// variant every runtime path (client factory, execution engine, etc.) should
    /// use. UI / migration code that needs to look up disabled entries should call
    /// [`Self::resolve_model_reference_any`] instead.
    pub fn resolve_model_reference(&self, model_ref: &str) -> Option<String> {
        self.models
            .iter()
            .find(|m| m.enabled && (m.id == model_ref || m.name == model_ref || m.model_name == model_ref))
            .map(|m| m.id.clone())
    }

    /// Resolves a model reference regardless of `enabled` state. UI / migration
    /// only — never use this on the runtime model-selection path.
    pub fn resolve_model_reference_any(&self, model_ref: &str) -> Option<String> {
        self.models
            .iter()
            .find(|m| m.id == model_ref || m.name == model_ref || m.model_name == model_ref)
            .map(|m| m.id.clone())
    }

    /// Returns true if the given reference points to a model that exists and is
    /// currently enabled.
    pub fn is_model_reference_active(&self, model_ref: &str) -> bool {
        self.resolve_model_reference(model_ref).is_some()
    }

    /// Returns the id of the first enabled model, if any. Used as a final
    /// fallback when a configured default points to a disabled / missing model.
    pub fn first_enabled_model_id(&self) -> Option<String> {
        self.models.iter().find(|m| m.enabled).map(|m| m.id.clone())
    }

    /// Resolves a model selector value.
    ///
    /// Special values:
    /// - `primary`: must resolve to a valid (enabled) primary model
    /// - `fast`: first tries the configured fast model, then falls back to primary
    ///
    /// Regular values are resolved by `id`, `name`, or `model_name`. All lookups
    /// require the target model to be enabled — disabled models are treated as if
    /// they did not exist.
    pub fn resolve_model_selection(&self, model_ref: &str) -> Option<String> {
        match model_ref {
            "primary" => self
                .default_models
                .primary
                .as_deref()
                .and_then(|value| self.resolve_model_reference(value)),
            "fast" => self
                .default_models
                .fast
                .as_deref()
                .and_then(|value| self.resolve_model_reference(value))
                .or_else(|| {
                    self.default_models
                        .primary
                        .as_deref()
                        .and_then(|value| self.resolve_model_reference(value))
                }),
            _ => self.resolve_model_reference(model_ref),
        }
    }
}

/// Shared agent-profile configuration.
///
/// Model mapping has moved to `AIConfig.agent_models`, keyed by agent id.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct AgentProfileConfig {
    /// Shared profile ID (e.g. agentic, coding_shared, requirement, ui-design).
    pub profile_id: String,

    /// Tools explicitly enabled by the user that are not part of the mode defaults.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_tools: Vec<String>,

    /// Default tools explicitly disabled by the user.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_tools: Vec<String>,

    /// User-level skills disabled for this mode.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_user_skills: Vec<String>,

    /// User-level built-in skills explicitly enabled even though the mode default disables them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_user_skills: Vec<String>,

    /// User-level subagent availability overrides for this shared profile.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub subagent_overrides: ParentSubagentOverrideConfig,
}

/// API view of a mode configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct AgentProfileView {
    pub profile_id: String,
    pub enabled_tools: Vec<String>,
    pub default_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_user_skills: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_user_skills: Vec<String>,
}

/// Default streaming idle timeout between chunks.
fn default_stream_idle_timeout() -> Option<u64> {
    Some(45)
}

/// Default time-to-first-token timeout while opening a stream.
fn default_stream_ttft_timeout() -> Option<u64> {
    Some(30)
}

// 2026-07-18 (W3a-2): 300s is the floor fallback for interactive desktop agents.
// Tools that manage their own execution timeout (ExecCommand, Task, etc.) bypass
// the pipeline timeout via `manages_own_execution_timeout() == true` and are unaffected.

/// Default tool execution timeout: 300 seconds (5 minutes).
fn default_tool_execution_timeout() -> Option<u64> {
    Some(300)
}

/// Default tool confirmation timeout: 300 seconds (5 minutes).
fn default_tool_confirmation_timeout() -> Option<u64> {
    Some(300)
}

fn default_skip_tool_confirmation() -> bool {
    true
}

// ═══════════════════════════════════════════════════════════════════
// R1 Phase 3: ConfirmationMode + ShellSecurityConfig
// ═══════════════════════════════════════════════════════════════════

/// Per-mode confirmation policy (R1 Phase 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ConfirmationMode {
    /// Skip confirmation for all LLM-triggered shell commands (default for coding modes)
    #[default]
    Permissive,
    /// Require user confirmation for all LLM-triggered shell commands
    Strict,
}

/// Shell security configuration (R1 Phase 3).
///
/// Provides mode-aware confirmation gating for shell-exec paths. Replaces
/// the global `skip_tool_confirmation` boolean with a more flexible scheme:
///
/// 1. `mode_overrides[mode]` (highest priority) — per-mode policy
/// 2. `confirmation_mode` (fallback) — global default policy
/// 3. `skip_tool_confirmation` (legacy fallback) — backward compat
///
/// Default mode mapping:
/// - coding-related modes (agentic, plan, multitask, debug, team, cowork, claw)
///   → `Permissive` (skip confirmation)
/// - admin/dangerous modes → `Strict` (require confirmation)
///
/// Spec: `docs/superpowers/specs/2026-06-23-r1-shell-exec-sandbox-design.md`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSecurityConfig {
    /// Global default mode (when no per-mode override matches).
    #[serde(default)]
    pub confirmation_mode: ConfirmationMode,

    /// Per-mode overrides. Key is the mode name (e.g. "agentic", "admin").
    /// Value is the confirmation mode for that mode.
    #[serde(default)]
    pub mode_overrides: std::collections::HashMap<String, ConfirmationMode>,

    /// Default mapping from common mode names to confirmation policies.
    /// Used as a fallback when no explicit override is set.
    #[serde(default = "default_mode_policies")]
    pub default_mode_policies: std::collections::HashMap<String, ConfirmationMode>,
}

impl Default for ShellSecurityConfig {
    fn default() -> Self {
        Self {
            confirmation_mode: ConfirmationMode::Permissive,
            mode_overrides: std::collections::HashMap::new(),
            default_mode_policies: default_mode_policies(),
        }
    }
}

fn default_mode_policies() -> std::collections::HashMap<String, ConfirmationMode> {
    let mut m = std::collections::HashMap::new();
    // Coding-related modes default to Permissive (current behavior)
    for mode in ["agentic", "plan", "multitask", "debug", "team", "cowork", "claw"] {
        m.insert(mode.to_string(), ConfirmationMode::Permissive);
    }
    m
}

impl ShellSecurityConfig {
    /// Resolve the effective confirmation mode for a given agent mode.
    ///
    /// Lookup order:
    /// 1. `mode_overrides[mode]` (explicit user override — highest priority)
    /// 2. `confirmation_mode` (global default — used for ALL modes unless overridden)
    ///
    /// Note: `default_mode_policies` is a separate map documenting known modes
    /// but is NOT used in resolve() — `confirmation_mode` is the global
    /// fallback for any mode.
    pub fn resolve(&self, mode: &str) -> ConfirmationMode {
        if let Some(m) = self.mode_overrides.get(mode) {
            return *m;
        }
        self.confirmation_mode
    }

    /// Convenience: should we skip confirmation for this mode?
    pub fn should_skip_confirmation(&self, mode: &str) -> bool {
        matches!(self.resolve(mode), ConfirmationMode::Permissive)
    }
}

fn default_subagent_max_concurrency() -> usize {
    5
}

pub const DEFAULT_MAX_ROUNDS: usize = 200;

fn default_max_rounds() -> usize {
    DEFAULT_MAX_ROUNDS
}

fn deserialize_agent_profiles<'de, D>(deserializer: D) -> Result<HashMap<String, AgentProfileConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<HashMap<String, Option<AgentProfileConfig>>>::deserialize(deserializer)?;
    Ok(raw
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(profile_id, config)| config.map(|config| (profile_id, config)))
        .collect())
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            models: vec![],
            agent_models: std::collections::HashMap::new(),
            func_agent_models: std::collections::HashMap::new(),
            default_models: DefaultModelsConfig::default(),
            agent_profiles: std::collections::HashMap::new(),
            review_teams: default_review_team_configs(),
            review_team_rate_limit_status: default_review_team_rate_limit_status(),
            review_team_project_strategy_overrides: std::collections::HashMap::new(),
            subagent_max_concurrency: default_subagent_max_concurrency(),
            proxy: ProxyConfig::default(),
            stream_idle_timeout_secs: default_stream_idle_timeout(),
            stream_ttft_timeout_secs: default_stream_ttft_timeout(),
            tool_execution_timeout_secs: default_tool_execution_timeout(),
            tool_confirmation_timeout_secs: default_tool_confirmation_timeout(),
            skip_tool_confirmation: true,
            shell_security: ShellSecurityConfig::default(),
            debug_mode_config: DebugModeConfig::default(),
            computer_use_enabled: false,
            browser_control_preferred_browser: String::new(),
            max_rounds: default_max_rounds(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ai_config_tool_timeouts_are_some_300() {
        let config = AIConfig::default();
        assert_eq!(config.tool_execution_timeout_secs, Some(300));
        assert_eq!(config.tool_confirmation_timeout_secs, Some(300));
    }
}
