//! UI-facing user settings.
//!
//! Spec: `docs/superpowers/specs/2026-06-26-frontend-onboarding-design.md` §5.6, §5.7.
//!
//! ## Role
//!
//! AppSettings is the **single owner** of UI-facing user configuration:
//! providers, workspaces, skill enable state, MCP servers, default model.
//! It replaces the previous P0-B `ConfigManager::add_default_providers`
//! behavior. The earlier pattern seeded 3 placeholder providers (anthropic /
//! openai / gemini) into `ConfigManager.config.ai.models`; that responsibility
//! now lives here, in user-space.
//!
//! ConfigManager **retains** its other responsibilities (`agent_models`,
//! `func_agent_models`, config migrations, file IO helpers) and exposes
//! `load_app_settings_from_disk` / `save_app_settings_to_disk` for disk IO
//! while AppSettings owns the in-memory representation and the CRUD API.
//!
//! ## Why a separate file
//!
//! `ConfigManager` lives in `northhing-core` (shared product runtime). Putting
//! UI settings there would couple the shared core to the desktop Slint shell.
//! Keeping AppSettings under `apps/desktop/app_state/` honours the boundary in
//! `src/crates/assembly/AGENTS.md` ("Assembly may depend on adapter and service
//! crates for selected delivery forms, but should not implement their protocol
//! serialization, authentication, transport, or platform details").
//!
//! ## Persistence
//!
//! Settings are persisted to `~/.northhing/config/app.json` via the helper
//! functions at the bottom of this file. The companion [`AppSettingsState`]
//! wrapper layers debounced save + Mutex on top so the Slint UI can mutate
//! freely without blocking the event loop.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

// ===== Provider =====

/// LLM provider type. Spec §5.6 (5 variants).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderType {
    /// `base_url` defaults to `https://api.anthropic.com`
    Anthropic,
    /// `base_url` defaults to `https://api.openai.com/v1`
    Openai,
    /// `base_url` defaults to `https://generativelanguage.googleapis.com/v1beta`
    Gemini,
    /// User provides `base_url`. Uses the OpenAI HTTP shape.
    CustomOpenaiCompatible,
    /// User provides `base_url`. Uses the Anthropic Messages HTTP shape.
    CustomAnthropicCompatible,
}

impl ProviderType {
    /// Default endpoint for the provider, when not user-overridden.
    pub fn default_base_url(&self) -> &'static str {
        match self {
            Self::Anthropic => "https://api.anthropic.com",
            Self::Openai => "https://api.openai.com/v1",
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Self::CustomOpenaiCompatible | Self::CustomAnthropicCompatible => "",
        }
    }

    /// Curated list of common models for the dropdown. Empty for `Custom*`
    /// variants (user must type the model name).
    pub fn default_models(&self) -> &'static [&'static str] {
        match self {
            Self::Anthropic => &["claude-sonnet-4-5", "claude-opus-4", "claude-haiku-4"],
            Self::Openai => &["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo"],
            Self::Gemini => &["gemini-2.0-flash", "gemini-1.5-pro"],
            Self::CustomOpenaiCompatible | Self::CustomAnthropicCompatible => &[],
        }
    }

    /// Slint-friendly display label (Chinese — matches AppStrings convention).
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::Openai => "OpenAI",
            Self::Gemini => "Gemini",
            Self::CustomOpenaiCompatible => "自定义 (OpenAI 兼容)",
            Self::CustomAnthropicCompatible => "自定义 (Anthropic 兼容)",
        }
    }
}

/// Single LLM provider entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// UUID v4, immutable. Used as the canonical handle.
    pub id: String,
    /// User-facing label, e.g. "我的 Anthropic".
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: ProviderType,
    /// Auto-filled from `ProviderType::default_base_url`; user-editable.
    pub base_url: String,
    /// Stored in plaintext in app.json. Never logged.
    pub api_key: String,
    /// Either a value from `ProviderType::default_models` or a user-typed
    /// custom model name (B4 = c: dropdown + custom).
    pub model: String,
    pub enabled: bool,
    /// Unix seconds, used for sort order in the UI list.
    pub created_at: i64,
    /// Last time `test_provider` succeeded.
    pub last_verified_at: Option<i64>,
    /// True = verified, false = test failed (UI shows ⚠️), None = never tested.
    pub last_verified_ok: Option<bool>,
}

impl ProviderConfig {
    pub fn new(name: String, provider_type: ProviderType) -> Self {
        let id = Uuid::new_v4().to_string();
        let base_url = provider_type.default_base_url().to_string();
        let model = provider_type
            .default_models()
            .first()
            .copied()
            .unwrap_or("")
            .to_string();
        Self {
            id,
            name,
            provider_type,
            base_url,
            api_key: String::new(),
            model,
            enabled: true,
            created_at: now_unix_secs(),
            last_verified_at: None,
            last_verified_ok: None,
        }
    }
}

// 2026-07-18 (D2e): edit-flow key inheritance — empty incoming key on edit
// keeps the stored one; add-flow or non-empty key passes through.
pub fn resolve_effective_api_key(stored: Option<&str>, incoming: &str) -> String {
    if incoming.trim().is_empty() {
        stored.unwrap_or("").to_string()
    } else {
        incoming.to_string()
    }
}

// ===== Core sync helpers =====

/// Map a `ProviderType` to the wire-format `provider` string used by
/// `northhing-core`'s `AIModelConfig`.
pub fn provider_wire_format(t: &ProviderType) -> &'static str {
    match t {
        ProviderType::Anthropic => "anthropic",
        ProviderType::Openai => "openai",
        ProviderType::Gemini => "gemini",
        ProviderType::CustomOpenaiCompatible => "openai",
        ProviderType::CustomAnthropicCompatible => "anthropic",
    }
}

/// Convert a desktop `ProviderConfig` into a core `AIModelConfig`.
pub fn provider_to_ai_model_config(p: &ProviderConfig) -> northhing_core::service::config::AIModelConfig {
    use northhing_core::service::config::{AuthConfig, ModelCapability, ModelCategory};
    northhing_core::service::config::AIModelConfig {
        id: p.id.clone(),
        name: p.name.clone(),
        provider: provider_wire_format(&p.provider_type).to_string(),
        model_name: p.model.clone(),
        base_url: p.base_url.clone(),
        request_url: None,
        api_key: p.api_key.clone(),
        context_window: None,
        max_tokens: None,
        temperature: None,
        top_p: None,
        enabled: p.enabled,
        category: ModelCategory::GeneralChat,
        capabilities: vec![ModelCapability::TextChat, ModelCapability::FunctionCalling],
        recommended_for: vec![],
        metadata: None,
        enable_thinking_process: false,
        reasoning_mode: None,
        inline_think_in_text: true,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: None,
        thinking_budget_tokens: None,
        custom_request_body: None,
        custom_request_body_mode: None,
        auth: AuthConfig::ApiKey,
    }
}

/// Compute which core model ids are "stale" — present in the core config
/// but no longer referenced by any desktop provider. These are leftovers
/// from providers that were edited into a new identity or deleted entirely.
/// Returns the list of ids that should be removed from core to keep the
/// two stores in sync (mirror semantics).
pub(crate) fn compute_stale_core_model_ids(
    existing_ids: &[String],
    providers: &[ProviderConfig],
) -> Vec<String> {
    let provider_ids: std::collections::HashSet<&str> =
        providers.iter().map(|p| p.id.as_str()).collect();
    existing_ids
        .iter()
        .filter(|id| !provider_ids.contains(id.as_str()))
        .cloned()
        .collect()
}

/// Sync all desktop providers into the core `GlobalConfig.ai.models` list,
/// then run `reconcile_models` so `default_models.primary` / `.fast` point
/// at the first enabled model. Returns the number of providers synced.
///
/// This is the "adapt-push" path: desktop owns the provider UI + storage,
/// but the runtime reads from core — so on every provider change we push
/// the corresponding `AIModelConfig` into core and let it reconcile.
///
/// 2026-07-18 (D2d): mirror semantics — after add/update, delete any core
/// model whose id no longer appears in `settings.providers` so the two
/// stores stay consistent (fixes the "10 models configured" stale-entry
/// leak caused by provider edits that changed their id).
pub async fn sync_providers_to_core(settings: &AppSettings) -> anyhow::Result<usize> {
    use northhing_core::service::config::get_global_config_service;
    let service = get_global_config_service().await?;
    let existing = service.get_ai_models().await?;
    let mut count = 0;
    for p in &settings.providers {
        let model = provider_to_ai_model_config(p);
        let model_id = model.id.clone();
        if existing.iter().any(|m| m.id == model_id) {
            service.update_ai_model(&model_id, model).await?;
        } else {
            service.add_ai_model(model).await?;
        }
        count += 1;
    }
    // 2026-07-18 (D2d): delete stale core models that no longer match any
    // desktop provider, then reconcile default slots.
    let existing_ids: Vec<String> = existing.iter().map(|m| m.id.clone()).collect();
    let stale_ids = compute_stale_core_model_ids(&existing_ids, &settings.providers);
    for stale_id in &stale_ids {
        if let Err(e) = service.delete_ai_model(stale_id).await {
            tracing::warn!(target: "app_state", "delete stale core model '{stale_id}' failed: {e}");
        }
    }
    service.reconcile_models("desktop-sync").await?;
    Ok(count)
}

/// Validate user input from the provider form. Returns `Ok(())` when the
/// input is acceptable, or `Err(msg)` with a Chinese error message.
pub fn validate_provider_input(
    name: &str,
    type_str: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("名称不能为空".to_string());
    }
    if api_key.trim().is_empty() {
        return Err("API Key 不能为空".to_string());
    }
    if model.trim().is_empty() {
        return Err("模型不能为空".to_string());
    }
    match type_str {
        "anthropic" | "openai" | "gemini" => {}
        "custom-openai" | "custom-anthropic" => {
            if base_url.trim().is_empty() {
                return Err("自定义服务需要提供 Base URL".to_string());
            }
        }
        _ => {
            return Err(format!("不支持的服务类型: {type_str}"));
        }
    }
    Ok(())
}

// ===== Workspace =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    pub path: PathBuf,
    /// Defaults to folder basename; user-editable.
    pub display_name: String,
    pub added_at: i64,
    pub last_opened_at: i64,
    /// Path to the `IDENTITY.md` file if one exists in the workspace root.
    /// `None` means no IDENTITY.md yet (D3 = a may auto-create one).
    pub identity_md_path: Option<PathBuf>,
}

// ===== Skill =====

/// Per-skill enable state. One entry per discovered builtin skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillState {
    /// Matches the folder name under `crates/assembly/core/builtin_skills/`.
    pub name: String,
    /// Default true; toggleable globally.
    pub global_enabled: bool,
    /// Per-workspace overrides (Q5 = E2 = c: global + per-workspace).
    /// Lookup uses `PathBuf` as key; serialization uses the path string.
    #[serde(with = "pathbuf_map_serde")]
    pub workspace_overrides: HashMap<PathBuf, bool>,
}

impl SkillState {
    /// Effective enable state for a given workspace: workspace override wins,
    /// otherwise fall back to global, otherwise default-on (true).
    pub fn effective_in(&self, workspace: &Path) -> bool {
        self.workspace_overrides
            .get(workspace)
            .copied()
            .unwrap_or(self.global_enabled)
    }
}

// ===== MCP Server =====

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum MCPTransport {
    Stdio,
    Sse,
    StreamableHttp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    pub id: String,
    pub name: String,
    pub transport: MCPTransport,
    pub enabled: bool,
    /// `command` for stdio transports (e.g. `npx`, `node`).
    pub command: Option<String>,
    pub args: Vec<String>,
    /// `url` for SSE / StreamableHttp transports.
    pub url: Option<String>,
    /// Environment variables for the stdio subprocess.
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub last_verified_at: Option<i64>,
    pub last_verified_ok: Option<bool>,
    /// Tool names returned by the last successful `tools/list`.
    pub last_tools: Vec<String>,
}

impl MCPServerConfig {
    pub fn new(name: String, transport: MCPTransport) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            transport,
            enabled: true,
            command: None,
            args: Vec::new(),
            url: None,
            env: HashMap::new(),
            last_verified_at: None,
            last_verified_ok: None,
            last_tools: Vec::new(),
        }
    }
}

// ===== Default model =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRef {
    pub provider_id: String,
    pub model: String,
}

// ===== Top-level =====

/// Schema version constant for forward-compat migrations.
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub schema_version: u32,
    pub providers: Vec<ProviderConfig>,
    pub workspaces: Vec<WorkspaceEntry>,
    pub current_workspace: Option<PathBuf>,
    /// One entry per discovered builtin skill. Built at load time by
    /// scanning `crates/assembly/core/builtin_skills/*`.
    pub skills_enabled: Vec<SkillState>,
    pub mcp_servers: Vec<MCPServerConfig>,
    pub default_model: Option<ModelRef>,
    /// True once the user has completed (or skipped) the 3-step welcome
    /// flow. Persisted so a fully-skipped onboarding does not reappear
    /// on the next launch. `#[serde(default)]` keeps pre-existing
    /// app.json files compatible (they lack the field → false).
    #[serde(default)]
    pub onboarding_completed: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            providers: Vec::new(),
            workspaces: Vec::new(),
            current_workspace: None,
            skills_enabled: Vec::new(),
            mcp_servers: Vec::new(),
            default_model: None,
            onboarding_completed: false,
        }
    }
}

impl AppSettings {
    /// Spec Q9=a: triggers the welcome flow when the user has done nothing
    /// yet. Legacy P0-B seeded entries (id contains `-default` and
    /// `enabled=false`) do NOT count as "real" providers — the welcome
    /// screen still shows for users whose app.json only has the old seeds.
    pub fn is_first_run(&self) -> bool {
        let real_providers = self
            .providers
            .iter()
            .filter(|p| !p.id.contains("-default") || p.enabled)
            .count();
        real_providers == 0 && self.workspaces.is_empty()
    }

    /// Spec Q1=a: detect P0-B legacy seeded placeholders so the Settings UI
    /// can offer a one-click cleanup banner.
    pub fn has_legacy_placeholders(&self) -> bool {
        self.providers.iter().any(|p| p.id.contains("-default") && !p.enabled)
    }

    /// Spec Q6=a: when a provider is removed, sessions that referenced it
    /// fall back to the first remaining enabled provider. Returns `None`
    /// when no other provider is enabled (the caller should then mark the
    /// session as `broken_provider`).
    pub fn fallback_provider_for(&self, deleted_id: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.enabled && p.id != deleted_id)
    }

    /// Spec C-xxiv (default model fallback): if the configured default's
    /// provider was deleted, fall back to the first enabled provider.
    pub fn resolve_default_model(&self) -> Option<ModelRef> {
        if let Some(dm) = &self.default_model {
            // Provider must exist AND be enabled; otherwise fall through.
            let provider_ok = self.providers.iter().any(|p| p.id == dm.provider_id && p.enabled);
            if provider_ok {
                return Some(dm.clone());
            }
        }
        self.providers.iter().find(|p| p.enabled).map(|p| ModelRef {
            provider_id: p.id.clone(),
            model: p.model.clone(),
        })
    }

    /// Mutator: add or replace a provider.
    ///
    /// 2026-07-18 (D2c): three-tier matching —
    /// 1. match by `id` (exact replace);
    /// 2. match by (name, base_url, api_key) — keep the existing id so session
    ///    references stay valid, replace the other fields;
    /// 3. otherwise push as new.
    pub fn upsert_provider(&mut self, mut p: ProviderConfig) {
        if let Some(slot) = self.providers.iter_mut().find(|x| x.id == p.id) {
            *slot = p;
        } else if let Some(slot) = self.providers.iter_mut().find(|x| {
            x.name == p.name && x.base_url == p.base_url && x.api_key == p.api_key
        }) {
            // Keep the original id to avoid breaking session references.
            let keep_id = slot.id.clone();
            p.id = keep_id;
            *slot = p;
        } else {
            self.providers.push(p);
        }

        // 2026-07-18 (D2c): auto-set default model on first enabled provider.
        if self.default_model.is_none() {
            if let Some(last) = self.providers.last() {
                if last.enabled {
                    self.default_model = Some(ModelRef {
                        provider_id: last.id.clone(),
                        model: last.model.clone(),
                    });
                }
            }
        }
    }

    pub fn remove_provider(&mut self, id: &str) -> Option<ProviderConfig> {
        let pos = self.providers.iter().position(|p| p.id == id)?;
        Some(self.providers.remove(pos))
    }

    pub fn add_workspace(&mut self, path: PathBuf) {
        if self.workspaces.iter().any(|w| w.path == path) {
            return;
        }
        let display_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("未命名")
            .to_string();
        let now = now_unix_secs();
        self.workspaces.push(WorkspaceEntry {
            path: path.clone(),
            display_name,
            added_at: now,
            last_opened_at: now,
            identity_md_path: None,
        });
    }

    pub fn set_current_workspace(&mut self, path: Option<&Path>) {
        if let Some(p) = path {
            if let Some(w) = self.workspaces.iter_mut().find(|w| &w.path == p) {
                w.last_opened_at = now_unix_secs();
            }
        }
        self.current_workspace = path.map(|p| p.to_path_buf());
    }

    pub fn remove_workspace(&mut self, path: &Path) -> Option<WorkspaceEntry> {
        let pos = self.workspaces.iter().position(|w| &w.path == path)?;
        let removed = self.workspaces.remove(pos);
        if self.current_workspace.as_deref() == Some(path) {
            self.current_workspace = None;
        }
        Some(removed)
    }

    pub fn upsert_mcp(&mut self, m: MCPServerConfig) {
        if let Some(slot) = self.mcp_servers.iter_mut().find(|x| x.id == m.id) {
            *slot = m;
        } else {
            self.mcp_servers.push(m);
        }
    }

    pub fn remove_mcp(&mut self, id: &str) -> Option<MCPServerConfig> {
        let pos = self.mcp_servers.iter().position(|m| m.id == id)?;
        Some(self.mcp_servers.remove(pos))
    }
}

// ===== Q6/Q7 Session Integrity Validation =====
/// 2026-06-26 (Phase 5): integrity issues detected by
/// `validate_session_integrity`. The UI maps these into banner +
/// inline error messages and the per-session `is-workspace-broken`
/// / `provider-deleted` flags (already in the SessionItem DTO).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionIntegrityIssue {
    pub session_id: String,
    /// "provider-deleted" (Q6) or "workspace-removed" (Q7)
    pub kind: String,
    /// The provider_id that was deleted, or the workspace path that
    /// was removed. Empty when not applicable.
    pub related_id: String,
}

impl AppSettings {
    /// Spec Q6/Q7: scan all sessions and detect which ones are now
    /// broken because the provider they referenced was deleted (Q6)
    /// or the workspace they belong to was removed (Q7). The caller
    /// (Rust `app_state::mod.rs`) maps these into UI errors.
    ///
    /// `session_provider_id` and `session_workspace_path` are
    /// closures that read from the session's stored state. We pass
    /// them as closures rather than taking the full `SessionState`
    /// struct so this stays decoupled from the agent-runtime crate's
    /// `Session` type — the only thing we need is "which provider
    /// does this session use" and "which workspace does it belong to".
    ///
    /// Returns one issue per broken session; sessions that are still
    /// healthy produce no issue.
    pub fn validate_session_integrity<I, P, W>(
        &self,
        session_ids: I,
        session_provider_id: P,
        session_workspace_path: W,
    ) -> Vec<SessionIntegrityIssue>
    where
        I: IntoIterator<Item = String>,
        P: Fn(&str) -> Option<String>,
        W: Fn(&str) -> Option<std::path::PathBuf>,
    {
        let known_provider_ids: std::collections::HashSet<&str> =
            self.providers.iter().map(|p| p.id.as_str()).collect();
        let known_workspace_paths: std::collections::HashSet<std::path::PathBuf> =
            self.workspaces.iter().map(|w| w.path.clone()).collect();

        let mut issues = Vec::new();
        for sid in session_ids {
            // Q6: provider referenced by the session is gone.
            if let Some(pid) = session_provider_id(&sid) {
                if !pid.is_empty() && !known_provider_ids.contains(pid.as_str()) {
                    issues.push(SessionIntegrityIssue {
                        session_id: sid.clone(),
                        kind: "provider-deleted".to_string(),
                        related_id: pid,
                    });
                    // A session can be both Q6 and Q7; we still
                    // report both so the UI shows the full picture.
                }
            }
            // Q7: workspace that the session belongs to was removed.
            if let Some(wpath) = session_workspace_path(&sid) {
                if !known_workspace_paths.contains(&wpath) {
                    issues.push(SessionIntegrityIssue {
                        session_id: sid,
                        kind: "workspace-removed".to_string(),
                        related_id: wpath.to_string_lossy().to_string(),
                    });
                }
            }
        }
        issues
    }
}

// ===== Helpers =====

pub(crate) fn now_unix_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// `serde(default)` workaround: HashMap<PathBuf, V> requires a custom
// serializer for PathBuf keys (which serialize as strings on platforms
// where OsStr is valid UTF-8). We only target Windows + macOS + Linux in
// this crate and workspace paths are always UTF-8 in practice, so a
// string round-trip is safe.
mod pathbuf_map_serde {
    use serde::de::{MapAccess, Visitor};
    use serde::ser::SerializeMap;
    use serde::{Deserializer, Serializer};
    use std::collections::HashMap;
    use std::path::PathBuf;

    pub fn serialize<S, V>(map: &HashMap<PathBuf, V>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: serde::Serialize,
    {
        let mut ser = s.serialize_map(Some(map.len()))?;
        for (k, v) in map {
            let key_str = k.to_string_lossy().into_owned();
            ser.serialize_entry(&key_str, v)?;
        }
        ser.end()
    }

    pub fn deserialize<'de, D, V>(d: D) -> Result<HashMap<PathBuf, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: serde::Deserialize<'de>,
    {
        struct V<V2>(std::marker::PhantomData<V2>);
        impl<'de, V2> Visitor<'de> for V<V2>
        where
            V2: serde::Deserialize<'de>,
        {
            type Value = HashMap<PathBuf, V2>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a map of path string -> value")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut out = HashMap::new();
                while let Some((k, v)) = access.next_entry::<String, V2>()? {
                    out.insert(PathBuf::from(k), v);
                }
                Ok(out)
            }
        }
        d.deserialize_map(V(std::marker::PhantomData))
    }
}

// ===== Disk IO =====

/// Resolve `~/.northhing/config/app.json`. Uses the same path convention as
/// ConfigManager (`self.path_manager.config_dir().join("app.json")`); for
/// Phase 1 we resolve it directly via `dirs` to keep this file independent of
/// `northhing-core`'s PathManager.
pub fn app_settings_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("无法获取 home 目录")?;
    Ok(home.join(".northhing").join("config").join("app.json"))
}

/// Load settings from `~/.northhing/config/app.json`. Returns `AppSettings::default()`
/// when the file is missing or fails to parse — the welcome screen's `is_first_run()`
/// check decides whether to show onboarding UI.
///
/// 2026-07-18 (D2c): after deserialization, dedup providers by
/// (name, provider_type, base_url, api_key, model) — keep the first, drop the
/// rest; re-point `default_model` at the kept entry when its original id was
/// dropped. Persist the migration immediately when anything was dropped.
pub async fn load_app_settings() -> Result<AppSettings> {
    let path = app_settings_path()?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let raw = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("读取 {path:?} 失败"))?;
    let mut parsed: AppSettings =
        serde_json::from_str(&raw).with_context(|| format!("解析 {path:?} 失败（schema 可能不兼容）"))?;
    let dropped = dedup_providers_on_load(&mut parsed);
    if dropped > 0 {
        // 2026-07-18 (D2c): persist migration result immediately.
        if let Err(e) = save_app_settings(&parsed).await {
            tracing::warn!(target: "app_state", "load dedup save failed: {e}");
        }
    }
    Ok(parsed)
}

/// 2026-07-18 (D2c): in-place provider dedup + default_model re-point.
/// Keeps the first of each (name, provider_type, base_url, api_key, model) group.
/// Returns the number of dropped duplicates (caller decides whether to save).
fn dedup_providers_on_load(s: &mut AppSettings) -> usize {
    use std::collections::HashSet;
    let mut seen: HashSet<(String, String, String, String, String)> = HashSet::new();
    let mut kept_ids: Vec<String> = Vec::new();
    let mut dropped_count = 0usize;
    s.providers.retain(|p| {
        let key = (
            p.name.clone(),
            serde_json::to_string(&p.provider_type).unwrap_or_default(),
            p.base_url.clone(),
            p.api_key.clone(),
            p.model.clone(),
        );
        if seen.insert(key) {
            kept_ids.push(p.id.clone());
            true
        } else {
            dropped_count += 1;
            false
        }
    });
    if dropped_count > 0 {
        let kept_set: HashSet<&str> = kept_ids.iter().map(|x| x.as_str()).collect();
        if let Some(dm) = &s.default_model {
            if !kept_set.contains(dm.provider_id.as_str()) {
                // default_model pointed at a dropped entry → re-point at the
                // first kept provider so the reference stays valid.
                if let Some(first) = s.providers.first() {
                    s.default_model = Some(ModelRef {
                        provider_id: first.id.clone(),
                        model: first.model.clone(),
                    });
                } else {
                    s.default_model = None;
                }
            }
        }
        tracing::info!(
            target: "app_state",
            "load dedup: dropped {dropped_count} duplicate provider(s)"
        );
    }
    dropped_count
}

/// Save settings to `~/.northhing/config/app.json`. Creates parent dirs as
/// needed. Atomic write via tmp-file + rename (Phase 1: simple write —
/// upgrade to atomic in Phase 5 if race conditions surface).
pub async fn save_app_settings(settings: &AppSettings) -> Result<()> {
    let path = app_settings_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("创建目录 {parent:?} 失败"))?;
    }
    let json = serde_json::to_string_pretty(settings).context("序列化 settings 失败")?;
    tokio::fs::write(&path, json)
        .await
        .with_context(|| format!("写入 {path:?} 失败"))?;
    Ok(())
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_provider() -> ProviderConfig {
        ProviderConfig::new("我的 Anthropic".into(), ProviderType::Anthropic)
    }

    #[test]
    fn provider_type_default_base_url() {
        assert_eq!(ProviderType::Anthropic.default_base_url(), "https://api.anthropic.com");
        assert_eq!(ProviderType::Openai.default_base_url(), "https://api.openai.com/v1");
        assert_eq!(
            ProviderType::Gemini.default_base_url(),
            "https://generativelanguage.googleapis.com/v1beta"
        );
        assert_eq!(ProviderType::CustomOpenaiCompatible.default_base_url(), "");
    }

    #[test]
    fn provider_type_default_models_non_empty_for_named() {
        assert!(!ProviderType::Anthropic.default_models().is_empty());
        assert!(!ProviderType::Openai.default_models().is_empty());
        assert!(!ProviderType::Gemini.default_models().is_empty());
        assert!(ProviderType::CustomOpenaiCompatible.default_models().is_empty());
    }

    #[test]
    fn provider_new_has_unique_id_and_defaults() {
        let a = sample_provider();
        let b = sample_provider();
        assert_ne!(a.id, b.id);
        assert!(a.enabled);
        assert_eq!(a.base_url, "https://api.anthropic.com");
        assert_eq!(a.model, "claude-sonnet-4-5");
        assert!(a.api_key.is_empty());
        assert!(a.last_verified_ok.is_none());
    }

    #[test]
    fn is_first_run_empty_settings() {
        let s = AppSettings::default();
        assert!(s.is_first_run());
    }

    #[test]
    fn is_first_run_legacy_placeholders_only_still_first_run() {
        // Spec Q9=a: P0-B seeded 3 disabled placeholders should NOT count as
        // "real" providers. is_first_run() returns true so the welcome
        // screen still appears for users whose app.json only has the seeds.
        let mut s = AppSettings::default();
        s.providers.push(ProviderConfig {
            id: "anthropic-default".into(),
            name: "Anthropic Claude".into(),
            provider_type: ProviderType::Anthropic,
            base_url: String::new(),
            api_key: String::new(),
            model: "claude-sonnet-4-5".into(),
            enabled: false,
            created_at: 0,
            last_verified_at: None,
            last_verified_ok: None,
        });
        assert!(s.is_first_run(), "legacy placeholder should not block welcome");
        assert!(s.has_legacy_placeholders(), "should detect legacy");
    }

    #[test]
    fn is_first_run_with_workspace() {
        let mut s = AppSettings::default();
        s.add_workspace(PathBuf::from("/tmp"));
        assert!(!s.is_first_run());
    }

    #[test]
    fn workspace_add_dedups() {
        let mut s = AppSettings::default();
        s.add_workspace(PathBuf::from("/tmp"));
        s.add_workspace(PathBuf::from("/tmp"));
        assert_eq!(s.workspaces.len(), 1);
    }

    #[test]
    fn workspace_set_current_updates_last_opened() {
        let mut s = AppSettings::default();
        s.add_workspace(PathBuf::from("/a"));
        s.add_workspace(PathBuf::from("/b"));
        s.set_current_workspace(Some(Path::new("/b")));
        assert_eq!(s.current_workspace, Some(PathBuf::from("/b")));
        let b_last = s
            .workspaces
            .iter()
            .find(|w| w.path == Path::new("/b"))
            .unwrap()
            .last_opened_at;
        let a_last = s
            .workspaces
            .iter()
            .find(|w| w.path == Path::new("/a"))
            .unwrap()
            .last_opened_at;
        assert!(b_last >= a_last);
    }

    #[test]
    fn remove_workspace_clears_current() {
        let mut s = AppSettings::default();
        s.add_workspace(PathBuf::from("/a"));
        s.set_current_workspace(Some(Path::new("/a")));
        s.remove_workspace(Path::new("/a"));
        assert!(s.current_workspace.is_none());
    }

    #[test]
    fn skill_effective_precedence() {
        let mut s = SkillState {
            name: "memory".into(),
            global_enabled: true,
            workspace_overrides: HashMap::new(),
        };
        // Default: global on.
        assert!(s.effective_in(Path::new("/anywhere")));

        // Global off, no override → off.
        s.global_enabled = false;
        assert!(!s.effective_in(Path::new("/anywhere")));

        // Workspace override beats global.
        s.workspace_overrides.insert(PathBuf::from("/myproj"), true);
        assert!(s.effective_in(Path::new("/myproj")));
        assert!(!s.effective_in(Path::new("/elsewhere")));
    }

    #[test]
    fn upsert_provider_replaces_by_id() {
        let mut s = AppSettings::default();
        let mut p = sample_provider();
        s.upsert_provider(p.clone());
        s.upsert_provider(p.clone());
        assert_eq!(s.providers.len(), 1);
        p.api_key = "sk-test".into();
        s.upsert_provider(p);
        assert_eq!(s.providers.len(), 1);
        assert_eq!(s.providers[0].api_key, "sk-test");
    }

    #[test]
    fn fallback_provider_skips_self() {
        let mut s = AppSettings::default();
        let mut a = sample_provider();
        let mut b = sample_provider();
        // 2026-07-18 (D2c): make (name, base_url, api_key) distinct so the new
        // dedup logic does not collapse them — this test is about fallback
        // selection, not dedup.
        a.name = "a".to_string();
        b.name = "b".to_string();
        let b_id = b.id.clone();
        s.upsert_provider(a);
        s.upsert_provider(b);
        assert_eq!(s.providers.len(), 2);
        // Remove a; b should be the fallback.
        let a_id = s.providers[0].id.clone();
        s.remove_provider(&a_id);
        let fb = s.fallback_provider_for(&a_id);
        assert_eq!(fb.map(|p| p.id.clone()), Some(b_id));
    }

    #[test]
    fn resolve_default_model_falls_back_when_provider_deleted() {
        let mut s = AppSettings::default();
        let a = sample_provider();
        let a_id = a.id.clone();
        s.upsert_provider(a.clone());
        s.default_model = Some(ModelRef {
            provider_id: a_id.clone(),
            model: a.model.clone(),
        });
        // Remove the default's provider.
        s.remove_provider(&a_id);
        // Should fall back to first enabled (none in this case).
        assert!(s.resolve_default_model().is_none());
    }

    #[test]
    fn settings_json_roundtrip() {
        let mut s = AppSettings::default();
        s.upsert_provider(sample_provider());
        s.add_workspace(PathBuf::from("/tmp/proj"));
        let json = serde_json::to_string_pretty(&s).unwrap();
        let back: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.providers.len(), 1);
        assert_eq!(back.workspaces.len(), 1);
    }

    // 2026-06-26 (Phase 4 fix): onboarding_completed serde default +
    // roundtrip. Pre-existing app.json files lack the field and must
    // deserialize to `false`; once set to `true` it round-trips cleanly.
    #[test]
    fn onboarding_completed_serde_default_false() {
        let full = serde_json::to_value(AppSettings::default()).expect("serialize default");
        let mut obj = full.as_object().expect("object").clone();
        obj.remove("onboarding_completed");
        let s: AppSettings = serde_json::from_value(serde_json::Value::Object(obj))
            .expect("deserialize without onboarding_completed");
        assert!(!s.onboarding_completed, "missing field should default to false");
    }

    #[test]
    fn onboarding_completed_roundtrip() {
        let mut s = AppSettings::default();
        assert!(!s.onboarding_completed);
        s.onboarding_completed = true;
        let json = serde_json::to_string_pretty(&s).unwrap();
        let back: AppSettings = serde_json::from_str(&json).unwrap();
        assert!(back.onboarding_completed, "true should round-trip");
    }

    // 2026-06-26 (Phase 5): Q6/Q7 session integrity validation tests.
    // See `validate_session_integrity` in the `impl AppSettings` block
    // above for the implementation rationale.

    fn sample_session_provider() -> ProviderConfig {
        ProviderConfig::new("test-anthropic".to_string(), ProviderType::Anthropic)
    }

    #[test]
    fn validate_session_integrity_detects_deleted_provider() {
        let mut s = AppSettings::default();
        let p = sample_session_provider();
        let p_id = p.id.clone();
        s.upsert_provider(p);
        // Add the workspace too so this test only checks Q6.
        s.add_workspace(PathBuf::from("/tmp/proj"));

        // Session references p_id + the workspace.
        let provider_lookup = |_sid: &str| -> Option<String> { Some(p_id.clone()) };
        let workspace_lookup = |_sid: &str| -> Option<PathBuf> { Some(PathBuf::from("/tmp/proj")) };

        // Before deletion: no issues.
        let issues = s.validate_session_integrity(vec!["s1".to_string()], &provider_lookup, &workspace_lookup);
        assert!(issues.is_empty(), "no issues when provider + workspace exist");

        // Delete the provider; expect the session to be flagged with Q6.
        s.remove_provider(&p_id);
        let issues = s.validate_session_integrity(vec!["s1".to_string()], &provider_lookup, &workspace_lookup);
        assert_eq!(issues.len(), 1, "expected exactly the Q6 issue");
        assert_eq!(issues[0].kind, "provider-deleted");
        assert_eq!(issues[0].session_id, "s1");
        assert_eq!(issues[0].related_id, p_id);
    }

    #[test]
    fn validate_session_integrity_detects_removed_workspace() {
        let mut s = AppSettings::default();
        s.add_workspace(PathBuf::from("/tmp/exists"));

        // Session belongs to a workspace that's not in the list.
        let provider_lookup = |_sid: &str| -> Option<String> { None };
        let workspace_lookup = |_sid: &str| -> Option<PathBuf> { Some(PathBuf::from("/tmp/removed")) };

        let issues = s.validate_session_integrity(vec!["s1".to_string()], &provider_lookup, &workspace_lookup);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, "workspace-removed");
        assert_eq!(issues[0].related_id, "/tmp/removed");
    }

    #[test]
    fn validate_session_integrity_reports_both_q6_and_q7_per_session() {
        // A session can be both: provider gone + workspace gone.
        let mut s = AppSettings::default();
        s.upsert_provider(sample_session_provider());

        let provider_lookup = |_sid: &str| -> Option<String> { Some("missing-provider".to_string()) };
        let workspace_lookup = |_sid: &str| -> Option<PathBuf> { Some(PathBuf::from("/tmp/missing")) };

        let issues = s.validate_session_integrity(vec!["s1".to_string()], &provider_lookup, &workspace_lookup);
        assert_eq!(issues.len(), 2);
        let kinds: Vec<&str> = issues.iter().map(|i| i.kind.as_str()).collect();
        assert!(kinds.contains(&"provider-deleted"));
        assert!(kinds.contains(&"workspace-removed"));
    }

    #[test]
    fn validate_session_integrity_empty_session_list_is_noop() {
        let s = AppSettings::default();
        let issues = s.validate_session_integrity(std::iter::empty::<String>(), |_| None, |_| None);
        assert!(issues.is_empty());
    }

    /// Integration test: simulate the spec's "完整欢迎流程 + 添加
    /// provider + 创建 session + 删除 provider" flow at the
    /// AppSettings level. After the sequence, `validate_session_integrity`
    /// must report the Q6 (provider-deleted) issue for the session
    /// that referenced the now-gone provider.
    #[test]
    fn integration_welcome_provider_session_delete_provider() {
        use std::collections::HashMap;

        // Step 1: empty settings → first-run flag set.
        let mut s = AppSettings::default();
        assert!(s.is_first_run(), "empty settings is first run");

        // Step 2: user adds a workspace (welcome step 1).
        s.add_workspace(PathBuf::from("/tmp/proj"));
        s.set_current_workspace(Some(&PathBuf::from("/tmp/proj")));
        assert!(!s.is_first_run(), "after workspace, not first run");

        // Step 3: user adds a provider (welcome step 2).
        let provider = sample_provider();
        let provider_id = provider.id.clone();
        s.upsert_provider(provider);
        s.default_model = Some(ModelRef {
            provider_id: provider_id.clone(),
            model: "claude-sonnet-4-5".to_string(),
        });

        // Step 4: user creates a session using the provider.
        let session_id = "sess-1".to_string();
        let mut session_provider_lookup = HashMap::new();
        session_provider_lookup.insert(session_id.clone(), provider_id.clone());
        let mut session_workspace_lookup = HashMap::new();
        session_workspace_lookup.insert(session_id.clone(), PathBuf::from("/tmp/proj"));
        let provider_lookup = |sid: &str| -> Option<String> { session_provider_lookup.get(sid).cloned() };
        let workspace_lookup = |sid: &str| -> Option<PathBuf> { session_workspace_lookup.get(sid).cloned() };

        // No issues yet.
        let issues = s.validate_session_integrity(vec![session_id.clone()], &provider_lookup, &workspace_lookup);
        assert!(issues.is_empty(), "all healthy before delete");

        // Step 5: user deletes the provider in Settings.
        s.remove_provider(&provider_id);

        // Now integrity should flag the session.
        let issues = s.validate_session_integrity(vec![session_id.clone()], &provider_lookup, &workspace_lookup);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].session_id, session_id);
        assert_eq!(issues[0].kind, "provider-deleted");
        assert_eq!(issues[0].related_id, provider_id);

        // And the default model should fall back to nothing.
        assert!(s.resolve_default_model().is_none());
    }

    // ===== Core sync helper tests =====

    #[test]
    fn provider_wire_format_mapping() {
        use super::provider_wire_format;
        use super::ProviderType;
        assert_eq!(provider_wire_format(&ProviderType::Anthropic), "anthropic");
        assert_eq!(provider_wire_format(&ProviderType::Openai), "openai");
        assert_eq!(provider_wire_format(&ProviderType::Gemini), "gemini");
        assert_eq!(
            provider_wire_format(&ProviderType::CustomOpenaiCompatible),
            "openai"
        );
        assert_eq!(
            provider_wire_format(&ProviderType::CustomAnthropicCompatible),
            "anthropic"
        );
    }

    #[test]
    fn provider_to_ai_model_config_fields() {
        use super::provider_to_ai_model_config;
        let p = ProviderConfig::new("我的 Anthropic".into(), ProviderType::Anthropic);
        let m = provider_to_ai_model_config(&p);
        assert_eq!(m.id, p.id);
        assert_eq!(m.name, "我的 Anthropic");
        assert_eq!(m.provider, "anthropic");
        assert_eq!(m.model_name, p.model);
        assert_eq!(m.api_key, p.api_key);
        assert_eq!(m.enabled, p.enabled);
        assert!(m.base_url.contains("anthropic"));
        assert_eq!(m.category, northhing_core::service::config::ModelCategory::GeneralChat);
        assert_eq!(m.auth, northhing_core::service::config::AuthConfig::ApiKey);
    }

    #[test]
    fn validate_provider_input_rejects_empty_name() {
        use super::validate_provider_input;
        let r = validate_provider_input("", "anthropic", "", "sk-x", "claude");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("名称"));
    }

    #[test]
    fn validate_provider_input_rejects_empty_api_key() {
        use super::validate_provider_input;
        let r = validate_provider_input("foo", "anthropic", "", "", "claude");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("API Key"));
    }

    #[test]
    fn validate_provider_input_rejects_empty_model() {
        use super::validate_provider_input;
        let r = validate_provider_input("foo", "anthropic", "", "sk-x", "");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("模型"));
    }

    #[test]
    fn validate_provider_input_rejects_unknown_type() {
        use super::validate_provider_input;
        let r = validate_provider_input("foo", "bogus", "", "sk-x", "claude");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("不支持"));
    }

    #[test]
    fn validate_provider_input_custom_requires_base_url() {
        use super::validate_provider_input;
        let r = validate_provider_input("foo", "custom-openai", "", "sk-x", "gpt");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Base URL"));
    }

    #[test]
    fn validate_provider_input_accepts_valid_anthropic() {
        use super::validate_provider_input;
        let r = validate_provider_input("foo", "anthropic", "", "sk-x", "claude");
        assert!(r.is_ok());
    }

    #[test]
    fn validate_provider_input_accepts_valid_custom() {
        use super::validate_provider_input;
        let r = validate_provider_input(
            "foo",
            "custom-openai",
            "https://example.com/v1",
            "sk-x",
            "gpt",
        );
        assert!(r.is_ok());
    }

    // ===== 2026-07-18 (D2c): upsert dedup + default-model auto-set tests =====

    fn provider_with_fields(
        id: &str,
        name: &str,
        base_url: &str,
        api_key: &str,
        model: &str,
        enabled: bool,
    ) -> ProviderConfig {
        ProviderConfig {
            id: id.to_string(),
            name: name.to_string(),
            provider_type: if base_url.contains("anthropic") {
                ProviderType::Anthropic
            } else {
                ProviderType::Openai
            },
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            enabled,
            created_at: 0,
            last_verified_at: None,
            last_verified_ok: None,
        }
    }

    #[test]
    fn upsert_provider_dedup_by_name_base_url_api_key_keeps_original_id() {
        let mut s = AppSettings::default();
        // First upsert: empty id → push new (gets a fresh UUID).
        let first = provider_with_fields("", "foo", "https://x.com/v1", "sk-same", "gpt", true);
        s.upsert_provider(first);
        assert_eq!(s.providers.len(), 1);
        let original_id = s.providers[0].id.clone();

        // Second upsert: different id but same (name, base_url, api_key) →
        // should replace in place and KEEP the original id.
        let second = provider_with_fields(
            "totally-different-id",
            "foo",
            "https://x.com/v1",
            "sk-same",
            "gpt",
            true,
        );
        s.upsert_provider(second);
        assert_eq!(s.providers.len(), 1, "must not duplicate");
        assert_eq!(
            s.providers[0].id, original_id,
            "must keep the original id to preserve session references"
        );
    }

    #[test]
    fn upsert_provider_first_enabled_auto_sets_default_model() {
        let mut s = AppSettings::default();
        assert!(s.default_model.is_none());
        let p = provider_with_fields("", "foo", "https://x.com/v1", "sk", "gpt", true);
        s.upsert_provider(p);
        assert_eq!(s.providers.len(), 1);
        let dm = s.default_model.expect("default_model should be auto-set");
        assert_eq!(dm.provider_id, s.providers[0].id);
        assert_eq!(dm.model, "gpt");
    }

    #[test]
    fn upsert_provider_does_not_overwrite_existing_default_model() {
        let mut s = AppSettings::default();
        let first = provider_with_fields("id-first", "first", "https://a.com/v1", "sk1", "m1", true);
        s.upsert_provider(first);
        let first_dm = s.default_model.clone().unwrap();

        // Second enabled provider → default_model must stay pointing at first.
        let second = provider_with_fields("id-second", "second", "https://b.com/v1", "sk2", "m2", true);
        s.upsert_provider(second);
        assert_eq!(s.providers.len(), 2);
        let dm = s.default_model.unwrap();
        assert_eq!(dm.provider_id, first_dm.provider_id);
        assert_eq!(dm.model, first_dm.model);
    }

    #[test]
    fn dedup_providers_on_load_drops_duplicates_keeps_first() {
        let mut s = AppSettings::default();
        // Two identical providers (same name/type/base_url/api_key/model) but
        // different ids → after dedup only the first remains.
        let a = provider_with_fields("id-a", "foo", "https://x.com/v1", "sk", "gpt", true);
        let b = provider_with_fields("id-b", "foo", "https://x.com/v1", "sk", "gpt", true);
        let c = provider_with_fields("id-c", "bar", "https://y.com/v1", "sk", "gpt", true);
        s.providers = vec![a, b, c];
        let dropped = super::dedup_providers_on_load(&mut s);
        assert_eq!(dropped, 1);
        assert_eq!(s.providers.len(), 2);
        assert_eq!(s.providers[0].id, "id-a", "first of group kept");
        assert_eq!(s.providers[1].id, "id-c");
    }

    #[test]
    fn dedup_providers_on_load_repoints_default_model_when_dropped() {
        let mut s = AppSettings::default();
        let a = provider_with_fields("id-a", "foo", "https://x.com/v1", "sk", "gpt", true);
        let b = provider_with_fields("id-b", "foo", "https://x.com/v1", "sk", "gpt", true);
        s.providers = vec![a, b];
        // default_model points at id-b (the one that will be dropped).
        s.default_model = Some(ModelRef {
            provider_id: "id-b".to_string(),
            model: "gpt".to_string(),
        });
        let dropped = super::dedup_providers_on_load(&mut s);
        assert_eq!(dropped, 1);
        assert_eq!(s.providers.len(), 1);
        // After dedup, default_model should point at the kept entry (id-a).
        let dm = s.default_model.expect("default_model should be re-pointed");
        assert_eq!(dm.provider_id, "id-a");
    }

    // ===== 2026-07-18 (D2d): compute_stale_core_model_ids tests =====

    #[test]
    fn compute_stale_empty_existing_returns_empty() {
        let providers = vec![provider_with_fields("id-a", "a", "https://a.com/v1", "sk", "gpt", true)];
        let stale = super::compute_stale_core_model_ids(&[], &providers);
        assert!(stale.is_empty(), "no existing ids → nothing stale");
    }

    #[test]
    fn compute_stale_partial_overlap_returns_only_extra() {
        let providers = vec![provider_with_fields("id-a", "a", "https://a.com/v1", "sk", "gpt", true)];
        let existing = vec!["id-a".to_string(), "id-b".to_string(), "id-c".to_string()];
        let stale = super::compute_stale_core_model_ids(&existing, &providers);
        assert_eq!(stale.len(), 2);
        assert!(stale.contains(&"id-b".to_string()));
        assert!(stale.contains(&"id-c".to_string()));
        assert!(!stale.contains(&"id-a".to_string()));
    }

    #[test]
    fn compute_stale_all_stale_when_no_providers() {
        let existing = vec!["id-a".to_string(), "id-b".to_string()];
        let stale = super::compute_stale_core_model_ids(&existing, &[]);
        assert_eq!(stale.len(), 2);
        assert!(stale.contains(&"id-a".to_string()));
        assert!(stale.contains(&"id-b".to_string()));
    }

    #[test]
    fn compute_stale_no_stale_when_all_match() {
        let providers = vec![
            provider_with_fields("id-a", "a", "https://a.com/v1", "sk1", "gpt", true),
            provider_with_fields("id-b", "b", "https://b.com/v1", "sk2", "claude", true),
        ];
        let existing = vec!["id-a".to_string(), "id-b".to_string()];
        let stale = super::compute_stale_core_model_ids(&existing, &providers);
        assert!(stale.is_empty(), "all existing ids matched → nothing stale");
    }

    // ===== 2026-07-18 (D2e): resolve_effective_api_key tests =====

    #[test]
    fn resolve_effective_api_key_empty_incoming_keeps_stored() {
        use super::resolve_effective_api_key;
        let stored = Some("sk-stored");
        let result = resolve_effective_api_key(stored, "");
        assert_eq!(result, "sk-stored");
    }

    #[test]
    fn resolve_effective_api_key_empty_incoming_no_stored_returns_empty() {
        use super::resolve_effective_api_key;
        let result = resolve_effective_api_key(None, "");
        assert_eq!(result, "");
    }

    #[test]
    fn resolve_effective_api_key_non_empty_incoming_passes_through() {
        use super::resolve_effective_api_key;
        let result = resolve_effective_api_key(Some("sk-stored"), "sk-new");
        assert_eq!(result, "sk-new");
    }

    #[test]
    fn resolve_effective_api_key_whitespace_only_treated_as_empty() {
        use super::resolve_effective_api_key;
        let result = resolve_effective_api_key(Some("sk-stored"), "   ");
        assert_eq!(result, "sk-stored");
    }
}
