//! Browser crash/reconnect recovery logic.

use super::launcher_lifecycle::BrowserLauncher;
use super::launcher_types::BrowserKind;

impl BrowserLauncher {
    /// Restart the browser with CDP enabled.
    pub async fn restart_with_cdp(
        kind: &BrowserKind,
        port: u16,
    ) -> crate::util::errors::NortHingResult<super::launcher_types::LaunchResult> {
        Self::launch_with_cdp_opts(kind, port, None).await
    }
}
