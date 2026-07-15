//! Browser launcher types and constants.

use serde::{Deserialize, Serialize};

/// Default CDP debug port.
pub const DEFAULT_CDP_PORT: u16 = 9222;

/// Known browser identifiers and their executable paths per platform.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrowserKind {
    Chrome,
    Edge,
    Chromium,
    Brave,
    Arc,
    Unknown(String),
}

impl std::fmt::Display for BrowserKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserKind::Chrome => write!(f, "Google Chrome"),
            BrowserKind::Edge => write!(f, "Microsoft Edge"),
            BrowserKind::Chromium => write!(f, "Chromium"),
            BrowserKind::Brave => write!(f, "Brave Browser"),
            BrowserKind::Arc => write!(f, "Arc"),
            BrowserKind::Unknown(name) => write!(f, "{}", name),
        }
    }
}

/// Result of browser detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub kind: BrowserKind,
    pub path: String,
    pub is_running: bool,
    pub cdp_available: bool,
}

/// Result of a browser launch attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LaunchResult {
    AlreadyConnected,
    Launched,
    LaunchedButCdpNotReady {
        port: u16,
        message: String,
    },
    BrowserRunningWithoutCdp {
        browser: String,
        executable: String,
        port: u16,
        instructions: String,
    },
}
