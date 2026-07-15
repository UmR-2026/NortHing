//! Skill registry
//!
//! Manages skill discovery, mode-specific filtering, and loading.
//!
//! The implementation is split across sibling modules to keep this file
//! small and focused. The split mirrors the four concerns of the registry:
//!
//! - [`registry_types`]: shared constants, slot tables, internal candidate
//!   types, and pure helpers.
//! - [`registry_store`]: local + remote filesystem scan and cache management.
//! - [`registry_dispatch`]: mode filtering and explicit skill lookup/loading.
//! - [`registry_meta`]: public query and serialization API.
//!
//! This file owns the [`SkillRegistry`] struct definition, its `OnceLock`
//! global instance, and the constructors. All other methods of
//! `SkillRegistry` live in the sibling impl blocks.

use super::types::SkillInfo;
use std::sync::OnceLock;
use tokio::sync::RwLock;

/// Global Skill registry instance
static SKILL_REGISTRY: OnceLock<SkillRegistry> = OnceLock::new();

/// Skill registry
pub struct SkillRegistry {
    /// Cached raw user-level skills (no workspace-specific project skills).
    pub(super) cache: RwLock<Vec<SkillInfo>>,
}

impl SkillRegistry {
    fn new() -> Self {
        Self {
            cache: RwLock::new(Vec::new()),
        }
    }

    /// Get the global Skill registry instance.
    pub fn global() -> &'static Self {
        SKILL_REGISTRY.get_or_init(Self::new)
    }
}
