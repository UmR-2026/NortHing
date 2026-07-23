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

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

mod integrity;
mod io;
mod sync;
mod types;

#[cfg(test)]
mod tests;

pub use integrity::*;
pub use io::*;
pub use sync::*;
pub use types::*;

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

// ===== Helpers =====

pub(crate) fn now_unix_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
