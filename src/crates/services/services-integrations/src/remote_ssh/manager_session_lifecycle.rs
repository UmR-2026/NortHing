//! SSH session lifecycle: handshake, auth, execute, and transparent reconnect.
//!
//! Owns the heavy lift of `establish_session` (TCP connect -> handshake ->
//! auth -> server-info probe), the cancellation-aware `execute_command_internal`
//! event loop, and the reconnect path `ensure_alive_or_reconnect` that keeps
//! stale sessions usable across network blips.
//!
//! Split from `manager.rs` in Round 13b. The 3 god methods inside this file
//! were further split into phase helpers in Round 13c.
//!
//! The `impl SSHConnectionManager` blocks live in three sibling modules
//! declared in the parent `remote_ssh/mod.rs`:
//!
//! - [`super::mgr_lifecycle_handlers`] — `establish_session`: TCP + handshake +
//!   auth + server-info probe (`prepare_session_transport`,
//!   `perform_session_handshake`, `perform_session_auth`,
//!   `resolve_session_server_info`, `get_server_info_internal`,
//!   `probe_remote_home_dir`, `interrupt_exec_channel`, plus the
//!   `load_private_key_for_auth` / `read_private_key_file` /
//!   `build_session_client_config` / `map_handshake_error` helpers).
//! - [`super::mgr_lifecycle_state`] — `execute_command_internal`: open channel,
//!   pump loop until exit / timeout / cancellation, and finalize exit-code
//!   fallback (`execute_open_channel`, `execute_pump_loop`,
//!   `execute_finalize_result`).
//! - [`super::mgr_lifecycle_persist`] — `ensure_alive_or_reconnect`: drift
//!   detect, reconnect-lock, prepare config (vault refresh), perform reconnect
//!   (`check_alive_and_drift`, `recheck_under_lock`, `prepare_reconnect_config`,
//!   `perform_reconnect`).
//!
//! Public API stays on `SSHConnectionManager` via `impl` blocks aggregated
//! across the sibling files; methods retain the same signatures so callers
//! and tests continue to work unchanged.
