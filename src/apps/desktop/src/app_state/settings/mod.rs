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
//! `func_agent_models`, config migrations, file IO helpers) while
//! AppSettings owns the in-memory representation and the CRUD API; disk IO
//! lives in this module's `io` submodule (`load_app_settings` /
//! `update_app_settings`).
//!
//! ## Why a separate file
//!
//! `ConfigManager` lives in `northhing-core` (shared product runtime). Putting
//! UI settings there would couple the shared core to the desktop UI shell
//! (was Slint; now Dioxus consult-room since 2026-08-28).
//! Keeping AppSettings under `apps/desktop/app_state/` honours the boundary in
//! `src/crates/assembly/AGENTS.md` ("Assembly may depend on adapter and service
//! crates for selected delivery forms, but should not implement their protocol
//! serialization, authentication, transport, or platform details").
//!
//! ## Persistence
//!
//! Settings are persisted to `~/.northhing/config/app.json` via the helper
//! functions at the bottom of this file. The companion [`AppSettingsState`]
//! wrapper layers debounced save + Mutex on top so the desktop UI (Dioxus
//! consult-room shell as of 2026-08-28) can mutate freely without blocking
//! the event loop.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

mod integrity;
mod io;
mod keyring;
mod sync;
mod types;

#[cfg(test)]
mod tests;

pub use io::*;
pub use keyring::*;
pub use sync::*;
pub use types::*;

// ===== Top-level =====

/// Schema version constant for forward-compat migrations.
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub schema_version: u32,
    pub workspaces: Vec<WorkspaceEntry>,
    pub current_workspace: Option<PathBuf>,
    /// True once the user has completed (or skipped) the 3-step welcome
    /// flow. Persisted so a fully-skipped onboarding does not reappear
    /// on the next launch. `#[serde(default)]` keeps pre-existing
    /// app.json files compatible (they lack the field → false).
    #[serde(default)]
    pub onboarding_completed: bool,
    /// MCP servers configured at user level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<MCPServerConfig>,
    /// Display mode: breathing animation toggle (Card 6, W9-7).
    /// Persisted across restarts; visual binding deferred to a future round
    /// per design discipline (breath belongs to logo + agent avatar only).
    #[serde(default = "default_display_breath")]
    pub display_breath: bool,
    /// Display mode: dual-optics (light/dark theme pair) toggle (Card 6, W9-7).
    /// Persisted across restarts; visual binding deferred to a future round.
    #[serde(default = "default_display_dual_optics")]
    pub display_dual_optics: bool,
}

fn default_display_breath() -> bool {
    true
}

fn default_display_dual_optics() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            workspaces: Vec::new(),
            current_workspace: None,
            onboarding_completed: false,
            mcp_servers: Vec::new(),
            display_breath: true,
            display_dual_optics: true,
        }
    }
}

impl AppSettings {
    /// Spec Q9=a: triggers the welcome flow when the user has done nothing yet.
    pub fn is_first_run(&self) -> bool {
        self.workspaces.is_empty()
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

    #[allow(dead_code)]
    pub fn upsert_mcp(&mut self, m: MCPServerConfig) {
        if let Some(slot) = self.mcp_servers.iter_mut().find(|x| x.id == m.id) {
            *slot = m;
        } else {
            self.mcp_servers.push(m);
        }
    }

    #[allow(dead_code)]
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
