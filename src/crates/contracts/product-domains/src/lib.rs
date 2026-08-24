#![allow(clippy::too_many_arguments)]
//! Product domain owner crate.
//!
//! Product subdomains live here when they can be compiled without depending on
//! the full northhing core runtime assembly.

#[cfg(feature = "function-agents")]
pub mod function_agents;
