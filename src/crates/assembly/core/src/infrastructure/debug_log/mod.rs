//! Debug Mode runtime logging utilities.
//! Provides a shared instrumentation pipeline for desktop/server/cli + web.
//!
//! ## Module Structure
//! - `types` - Types and handlers for the HTTP ingest server (Config, State, Request, Response)
//! - `http_server` - The actual HTTP server implementation (axum-based)
//!
//! K4a-T5 (2026-07-26): `log_event`, the `COMP_*` component constants,
//! and the underlying `append_log_async` / entry types now live in the
//! leaf `northhing-debug-log` service crate so product surfaces can
//! depend on them without reaching into core. They are re-exported here
//! so core-internal call sites (e.g. `types.rs` via `super::`) and the
//! `northhing_core::debug` compatibility path stay unchanged. The HTTP
//! ingest server stays in core because `types::handle_ingest` depends on
//! core's workspace service.

pub mod http_server;
pub mod types;

pub use types::{
    handle_ingest, IngestLogRequest, IngestResponse, IngestServerConfig, IngestServerState, DEFAULT_INGEST_PORT,
};

pub use http_server::IngestServerManager;

pub use northhing_debug_log::{
    append_log_async, log_event, DebugLogConfig, DebugLogEntry, COMP_ACTOR_RUNTIME, COMP_APP_LIFECYCLE,
    COMP_MODE_ROUTING, COMP_SESSION_LIFECYCLE, COMP_SKILL_PANEL,
};
