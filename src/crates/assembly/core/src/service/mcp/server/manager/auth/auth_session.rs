//! Session lifecycle helpers.
//!
//! The `MCPServerManager` impl block for session helpers lives in `auth/mod.rs`
//! so that sibling submodules (`auth_oauth`, etc.) can reach the `pub(super)`
//! associated functions via the parent module scope.
