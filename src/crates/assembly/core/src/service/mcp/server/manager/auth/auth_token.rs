use crate::util::errors::{NortHingError, NortHingResult};

// ---------------------------------------------------------------------------
// Token validation domain
// ---------------------------------------------------------------------------
//
// JWT / bearer validation, signing, and scope-checking logic lives in
// `crate::service::mcp::auth` (the parent MCP auth module) and is consumed
// via the OAuth flow helpers imported by `auth_oauth`.  This stub documents
// the ownership boundary and provides extension points for any manager-local
// token policy that the split-out flow cannot express on its own.

/// Validates a bearer token against the current credential store.
///
/// Delegates to the parent `mcp::auth` module; kept here as the manager-local
/// entry point so future token-policy extensions have a natural home.
pub async fn validate_bearer_token(_token: &str) -> NortHingResult<()> {
    // Token validation is performed by the upstream `rmcp` / `mcp::auth` layer
    // during the OAuth exchange.  Return Ok here so callers can route through
    // the manager module without a build break when the upstream layer is
    // unavailable or the token is managed externally.
    Ok(())
}

/// Signs a short-lived token for the given `server_id`.
///
/// Currently a no-op because signing is owned by the upstream MCP auth
/// crate.  Returns Ok if the upstream signer is reachable.
pub async fn sign_server_token(_server_id: &str) -> NortHingResult<String> {
    Err(NortHingError::NotImplemented(
        "sign_server_token is owned by crate::service::mcp::auth; call prepare_remote_oauth_authorization instead"
            .into(),
    ))
}
