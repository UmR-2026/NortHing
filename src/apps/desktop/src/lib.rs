#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! northhing Desktop Shell Library
//!
//! Re-exports for the desktop application.

pub mod app_state;
pub mod flags;
pub mod mcp_adapter;

// Re-export the kernel facade handle so app_state modules can call
// `kernel_facade()` without a `northhing_core::` import path (K4a-T23).
pub use northhing_core::kernel_facade::kernel_facade;
