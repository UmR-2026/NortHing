//! Mobile identity validation and persistence for the pairing flow.
//!
//! Carries the two `pub(crate)` static-style helpers that manage the trusted
//! mobile identity state held by `RemoteConnectService` during a relay
//! lifecycle. The helpers take the `Arc<RwLock<Option<TrustedMobileIdentity>>>`
//! as an explicit first argument rather than `&self` because they are
//! invoked from places that hold only the lock handle (e.g. command
//! handlers that have already borrowed the parent `pairing_arc`).

use std::sync::Arc;
use tokio::sync::RwLock;

use super::pairing;
use super::TrustedMobileIdentity;

impl super::RemoteConnectService {
    pub(crate) async fn validate_mobile_identity(
        trusted_mobile_identity: &Arc<RwLock<Option<TrustedMobileIdentity>>>,
        response: &pairing::PairingResponse,
    ) -> std::result::Result<TrustedMobileIdentity, String> {
        let mobile_install_id = response
            .mobile_install_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Missing mobile installation ID".to_string())?;
        let user_id = response
            .user_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Missing user ID".to_string())?;

        let submitted = TrustedMobileIdentity {
            mobile_install_id: mobile_install_id.to_string(),
            user_id: user_id.to_string(),
        };

        let trusted = trusted_mobile_identity.read().await.clone();
        match trusted {
            Some(existing) if existing.mobile_install_id == submitted.mobile_install_id => {
                if existing.user_id != submitted.user_id {
                    Err("This mobile device must continue using the previously confirmed user ID".to_string())
                } else {
                    Ok(submitted)
                }
            }
            Some(existing) if existing.user_id != submitted.user_id => Err(
                "This remote URL is already protected. Enter the previously confirmed user ID to continue.".to_string(),
            ),
            _ => Ok(submitted),
        }
    }

    pub(crate) async fn persist_mobile_identity(
        trusted_mobile_identity: &Arc<RwLock<Option<TrustedMobileIdentity>>>,
        identity: TrustedMobileIdentity,
    ) {
        *trusted_mobile_identity.write().await = Some(identity);
    }
}
