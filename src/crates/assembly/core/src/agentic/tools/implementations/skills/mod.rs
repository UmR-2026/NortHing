//! Skill management module
//!
//! Provides Skill registry, loading, and configuration management functionality

pub mod builtin;
pub mod catalog;
pub mod mode_overrides;
pub mod policy;
pub mod registry;
mod registry_dispatch;
mod registry_meta;
mod registry_store;
mod registry_types;
pub mod resolver;
pub mod resolver_v2;
pub mod types;

pub use registry::SkillRegistry;
pub use resolver_v2::{resolve_for_prompt, resolve_for_prompt_with_max, ResolvedSkill};
pub use types::{ModeSkillInfo, ModeSkillStateReason, SkillData, SkillInfo, SkillLocation};

/// Get global Skill registry instance
pub fn skill_registry() -> &'static SkillRegistry {
    SkillRegistry::global()
}
