//! Browser state tracking and cache management.

use super::launcher_lifecycle::BrowserLauncher;
use super::launcher_types::BrowserKind;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

/// Cache for browser installation status to avoid repeated filesystem checks.
/// The cache is valid for the lifetime of the process since browser installations
/// don't change during a session.
static BROWSER_INSTALL_CACHE: Mutex<Option<HashMap<String, bool>>> = Mutex::new(None);

impl BrowserLauncher {
    /// Check if a CDP debug port is already listening.
    pub async fn is_cdp_available(port: u16) -> bool {
        let url = format!("http://127.0.0.1:{}/json/version", port);
        reqwest::Client::new()
            .get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Check whether a browser's executable (or app bundle) is present on disk.
    /// Results are cached for the process lifetime since browser installations
    /// don't change during a session.
    pub fn is_browser_installed(kind: &BrowserKind) -> bool {
        // Unknown browsers are never considered installed.
        if matches!(kind, BrowserKind::Unknown(_)) {
            return false;
        }

        let cache_key = format!("{:?}", kind);

        // Check cache first.
        {
            let cache = BROWSER_INSTALL_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref map) = *cache {
                if let Some(&cached) = map.get(&cache_key) {
                    return cached;
                }
            }
        }

        // Compute the result.
        let result = Self::check_browser_installed_impl(kind);

        // Store in cache.
        {
            let mut cache = BROWSER_INSTALL_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            let map = cache.get_or_insert_with(HashMap::new);
            map.insert(cache_key, result);
        }

        tracing::debug!("Browser {:?} installed: {}", kind, result);
        result
    }

    /// Check if a browser process is currently running.
    // reason: is_browser_running() is reserved for the upcoming liveness-check before launch (today launch always spawns a new browser)
    #[allow(dead_code)]
    pub(super) fn is_browser_running(kind: &BrowserKind) -> bool {
        // Per-platform process names.
        // macOS / Linux match against the executable filename via `pgrep -f`.
        // Windows must use the *.exe image name as it appears in `tasklist`.
        #[cfg(target_os = "macos")]
        let process_names: Vec<&str> = match kind {
            BrowserKind::Chrome => vec!["Google Chrome"],
            BrowserKind::Edge => vec!["Microsoft Edge"],
            BrowserKind::Brave => vec!["Brave Browser"],
            BrowserKind::Arc => vec!["Arc"],
            BrowserKind::Chromium => vec!["Chromium"],
            BrowserKind::Unknown(_) => return false,
        };

        #[cfg(target_os = "linux")]
        let process_names: Vec<&str> = match kind {
            BrowserKind::Chrome => vec!["chrome", "google-chrome"],
            BrowserKind::Edge => vec!["msedge", "microsoft-edge"],
            BrowserKind::Brave => vec!["brave", "brave-browser"],
            BrowserKind::Arc => vec!["arc"],
            BrowserKind::Chromium => vec!["chromium", "chromium-browser"],
            BrowserKind::Unknown(_) => return false,
        };

        #[cfg(target_os = "windows")]
        let process_names: Vec<&str> = match kind {
            BrowserKind::Chrome => vec!["chrome.exe"],
            BrowserKind::Edge => vec!["msedge.exe"],
            BrowserKind::Brave => vec!["brave.exe"],
            BrowserKind::Arc => vec!["arc.exe"],
            BrowserKind::Chromium => vec!["chrome.exe", "chromium.exe"],
            BrowserKind::Unknown(_) => return false,
        };

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            for name in &process_names {
                let output = super::launcher_lifecycle::silent_command("pgrep")
                    .args(["-f", name])
                    .output()
                    .ok();
                if let Some(out) = output {
                    if out.status.success() && !out.stdout.is_empty() {
                        return true;
                    }
                }
            }
            false
        }

        #[cfg(target_os = "windows")]
        {
            for image in &process_names {
                let filter = format!("IMAGENAME eq {}", image);
                let output = super::launcher_lifecycle::silent_command("tasklist")
                    .args(["/FI", &filter, "/NH", "/FO", "CSV"])
                    .output()
                    .ok();
                if let Some(out) = output {
                    let text = String::from_utf8_lossy(&out.stdout);
                    // tasklist prints "INFO: No tasks ..." when nothing matches;
                    // otherwise the first CSV column contains the image name.
                    if text.to_ascii_lowercase().contains(&image.to_ascii_lowercase()) {
                        return true;
                    }
                }
            }
            false
        }
    }

    /// Clear the browser installation cache. Useful for testing or when
    /// browser installations might have changed.
    #[cfg(test)]
    pub fn clear_install_cache() {
        let mut cache = BROWSER_INSTALL_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        *cache = None;
    }
}
