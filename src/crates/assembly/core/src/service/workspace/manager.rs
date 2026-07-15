//! Workspace manager (R27b facade).
//!
//! Mavis take-over (impl-block god-impl, sub-domain split R27b). Split into 4
//! sibling files: types + workspace_info_impl + manager_lifecycle + manager_accessors.
//! impl+struct kept in same sibling for private field access.

pub use super::manager_accessors::*;
pub use super::manager_lifecycle::*;
pub use super::types::*;
pub use super::workspace_info_impl::*;
