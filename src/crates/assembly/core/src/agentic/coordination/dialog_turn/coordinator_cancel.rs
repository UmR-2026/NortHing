//! Sub-domain: cancel.
//! Spec §2.1 — facade methods extracted from dialog_turn/mod.rs (R44a refactor).
//! Contains the 2 thin wrappers around `*_impl` cancel helpers in the `turn_cancel` sibling.

use super::super::coordinator::*;

use crate::util::errors::NortHingResult;
use tokio::time::Duration;

impl ConversationCoordinator {
    #[allow(clippy::too_many_arguments)]
    pub async fn cancel_dialog_turn(&self, session_id: &str, dialog_turn_id: &str) -> NortHingResult<()> {
        self.cancel_dialog_turn_impl(session_id, dialog_turn_id).await
    }

    pub async fn cancel_active_turn_for_session(
        &self,
        session_id: &str,
        wait_timeout: Duration,
    ) -> NortHingResult<Option<String>> {
        self.cancel_active_turn_for_session_impl(session_id, wait_timeout).await
    }
}
