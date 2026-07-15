//! LSP server process callback type aliases.
//!
//! Public callback type aliases consumed by `LspServerProcess::spawn`
//! and forwarded to the three background tokio tasks in `process_runtime.rs`.
//!
//! These types are also imported by `super::process` (re-exported via
//! `pub use`) so external callers can keep using
//! `super::process::{CrashCallback, ProgressCallback, ...}` paths.

use std::sync::Arc;

/// Process crash callback type.
pub type CrashCallback = Arc<dyn Fn(String) + Send + Sync>;

/// Progress notification callback type.
/// Parameters: `(kind: "begin" | "report" | "end", token: String, percentage: Option<u32>, message: String)`.
pub type ProgressCallback = Arc<dyn Fn(String, String, Option<u32>, String) + Send + Sync>;

/// Token creation callback type.
/// Parameters: `(token: String)`.
pub type TokenCreateCallback = Arc<dyn Fn(String) + Send + Sync>;

/// Diagnostics callback type.
/// Parameters: `(uri: String, diagnostics: Vec<serde_json::Value>)`.
pub type DiagnosticsCallback = Arc<dyn Fn(String, Vec<serde_json::Value>) + Send + Sync>;
