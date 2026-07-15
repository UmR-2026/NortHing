//! `SessionMessage` tool — facade for the split siblings.
//!
//! Originally a single 800-line file, the implementation is now
//! distributed across the following sub-modules:
//!
//! | Sub-module | Responsibility |
//! |---|---|
//! | [`tool`] | `SessionMessageTool` struct + `Default` impl + `Tool` trait implementation + thin `call_impl` dispatcher. |
//! | [`sm_types`] | `SessionMessageInput` struct, `SessionMessageAgentType` enum, and the `validate_session_id` shape validator. |
//! | [`sm_resolve`] | Workspace resolution (local + remote host), sender/creator extraction helpers, and the cross-session reminder envelope. |
//! | [`sm_send`] | Target-session preparation (`prepare_existing_target` / `prepare_new_target`) and the shared `submit_and_format` dialog-dispatch flow. |
//! | [`tests`] | Unit tests. |
//!
//! The public surface (`SessionMessageTool`) is preserved exactly; only
//! the internal layout has changed.

mod sm_resolve;
mod sm_send;
mod sm_types;
#[cfg(test)]
mod tests;
mod tool;

pub use tool::SessionMessageTool;
