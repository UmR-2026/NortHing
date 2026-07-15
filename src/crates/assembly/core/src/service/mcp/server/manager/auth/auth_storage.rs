use crate::util::errors::{NortHingError, NortHingResult};

// ---------------------------------------------------------------------------
// Credential storage domain
// ---------------------------------------------------------------------------
//
// Credential persistence is owned by `crate::service::mcp::auth` (the parent
// MCP auth module).  The public methods `clear_remote_oauth_credentials` live
// in `auth_oauth` because they compose the cancel-then-clear sequence, but
// any manager-local credential hooks belong here.

/// Removes all stored OAuth credentials for `server_id`.
///
/// Delegates to `crate::service::mcp::auth::clear_stored_oauth_credentials`.
/// Kept as a thin wrapper so that future persistence backends can be swapped
/// in without changing the public `MCPServerManager` API surface.
pub async fn clear_credentials(server_id: &str) -> NortHingResult<()> {
    crate::service::mcp::auth::clear_stored_oauth_credentials(server_id).await
}
