//! Browser detection, kind resolution, and executable path resolution.

use super::launcher_lifecycle::BrowserLauncher;
use super::launcher_types::BrowserKind;
use crate::infrastructure::app_paths::path_manager_arc;
use crate::util::errors::{NortHingError, NortHingResult};

impl BrowserLauncher {
    /// Detect the user's default browser on the current platform.
    pub fn detect_default_browser() -> NortHingResult<BrowserKind> {
        #[cfg(target_os = "macos")]
        {
            Self::detect_default_browser_macos()
        }
        #[cfg(target_os = "windows")]
        {
            Self::detect_default_browser_windows()
        }
        #[cfg(target_os = "linux")]
        {
            Self::detect_default_browser_linux()
        }
    }

    #[cfg(target_os = "macos")]
    fn detect_default_browser_macos() -> NortHingResult<BrowserKind> {
        let output = super::launcher_lifecycle::silent_command("defaults")
            .args([
                "read",
                "com.apple.LaunchServices/com.apple.launchservices.secure",
                "LSHandlers",
            ])
            .output()
            .ok();

        if let Some(out) = output {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if text.contains("com.google.chrome") {
                return Ok(BrowserKind::Chrome);
            } else if text.contains("com.microsoft.edgemac") {
                return Ok(BrowserKind::Edge);
            } else if text.contains("com.brave.browser") {
                return Ok(BrowserKind::Brave);
            } else if text.contains("company.thebrowser.browser") {
                return Ok(BrowserKind::Arc);
            }
        }

        // Fallback: check which browsers are installed
        let browsers = [
            ("/Applications/Google Chrome.app", BrowserKind::Chrome),
            ("/Applications/Microsoft Edge.app", BrowserKind::Edge),
            ("/Applications/Brave Browser.app", BrowserKind::Brave),
            ("/Applications/Arc.app", BrowserKind::Arc),
            ("/Applications/Chromium.app", BrowserKind::Chromium),
        ];

        for (path, kind) in &browsers {
            if std::path::Path::new(path).exists() {
                tracing::debug!("Found browser at {}", path);
                return Ok(kind.clone());
            }
        }

        Ok(BrowserKind::Chrome)
    }

    #[cfg(target_os = "windows")]
    fn detect_default_browser_windows() -> NortHingResult<BrowserKind> {
        let output = super::launcher_lifecycle::silent_command("reg")
            .args([
                "query",
                r"HKEY_CURRENT_USER\Software\Microsoft\Windows\Shell\Associations\UrlAssociations\http\UserChoice",
                "/v",
                "ProgId",
            ])
            .output()
            .ok();

        if let Some(out) = output {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if text.contains("chrome") {
                return Ok(BrowserKind::Chrome);
            } else if text.contains("edge") {
                return Ok(BrowserKind::Edge);
            } else if text.contains("brave") {
                return Ok(BrowserKind::Brave);
            }
        }

        Ok(BrowserKind::Chrome)
    }

    #[cfg(target_os = "linux")]
    fn detect_default_browser_linux() -> NortHingResult<BrowserKind> {
        let output = super::launcher_lifecycle::silent_command("xdg-settings")
            .args(["get", "default-web-browser"])
            .output()
            .ok();

        if let Some(out) = output {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if text.contains("chrome") || text.contains("google") {
                return Ok(BrowserKind::Chrome);
            } else if text.contains("edge") || text.contains("microsoft") {
                return Ok(BrowserKind::Edge);
            } else if text.contains("brave") {
                return Ok(BrowserKind::Brave);
            } else if text.contains("chromium") {
                return Ok(BrowserKind::Chromium);
            }
        }

        Ok(BrowserKind::Chrome)
    }

    /// Internal implementation of browser installation check.
    pub(super) fn check_browser_installed_impl(kind: &BrowserKind) -> bool {
        let exe = Self::browser_executable(kind);
        #[cfg(target_os = "macos")]
        {
            // On macOS, check the .app bundle instead of the inner executable
            let app_path = match kind {
                BrowserKind::Chrome => "/Applications/Google Chrome.app",
                BrowserKind::Edge => "/Applications/Microsoft Edge.app",
                BrowserKind::Brave => "/Applications/Brave Browser.app",
                BrowserKind::Arc => "/Applications/Arc.app",
                BrowserKind::Chromium => "/Applications/Chromium.app",
                BrowserKind::Unknown(_) => "",
            };
            if !app_path.is_empty() {
                return std::path::Path::new(app_path).exists();
            }
        }
        std::path::Path::new(&exe).exists()
    }

    /// Parse a `BrowserKind` from the CDP `/json/version` "Browser" field.
    /// The field typically looks like `"HeadlessChrome/130.0..."` or
    /// `"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36"`
    /// or `"Microsoft Edge/130.0..."`.
    pub fn browser_kind_from_cdp_version(version_str: &str) -> Option<BrowserKind> {
        let lower = version_str.to_ascii_lowercase();
        if lower.contains("edg") || lower.contains("edge") {
            Some(BrowserKind::Edge)
        } else if lower.contains("brave") {
            Some(BrowserKind::Brave)
        } else if lower.contains("chromium") {
            Some(BrowserKind::Chromium)
        } else if lower.contains("chrome") {
            Some(BrowserKind::Chrome)
        } else if lower.contains("arc") {
            Some(BrowserKind::Arc)
        } else {
            None
        }
    }

    pub fn browser_kind_from_config(value: &str) -> Option<BrowserKind> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "default" => None,
            "chrome" | "google-chrome" | "google_chrome" => Some(BrowserKind::Chrome),
            "edge" | "microsoft-edge" | "microsoft_edge" => Some(BrowserKind::Edge),
            "chromium" => Some(BrowserKind::Chromium),
            "brave" | "brave-browser" | "brave_browser" => Some(BrowserKind::Brave),
            "arc" => Some(BrowserKind::Arc),
            other => Some(BrowserKind::Unknown(other.to_string())),
        }
    }

    pub fn resolve_browser_kind(preferred_browser: Option<&str>) -> NortHingResult<BrowserKind> {
        if let Some(kind) = preferred_browser.and_then(Self::browser_kind_from_config) {
            Ok(kind)
        } else {
            Self::detect_default_browser()
        }
    }

    fn browser_profile_slug(kind: &BrowserKind) -> String {
        match kind {
            BrowserKind::Chrome => "chrome".to_string(),
            BrowserKind::Edge => "edge".to_string(),
            BrowserKind::Chromium => "chromium".to_string(),
            BrowserKind::Brave => "brave".to_string(),
            BrowserKind::Arc => "arc".to_string(),
            BrowserKind::Unknown(name) => name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
                .trim_matches('-')
                .to_string(),
        }
    }

    fn managed_user_data_dir(kind: &BrowserKind) -> std::path::PathBuf {
        path_manager_arc()
            .user_data_dir()
            .join("browser-control")
            .join(Self::browser_profile_slug(kind))
    }

    pub(super) fn ensure_managed_user_data_dir(kind: &BrowserKind) -> NortHingResult<std::path::PathBuf> {
        let dir = Self::managed_user_data_dir(kind);
        std::fs::create_dir_all(&dir)
            .map_err(|e| NortHingError::tool(format!("Failed to create browser control profile directory: {}", e)))?;
        Ok(dir)
    }

    #[cfg(target_os = "macos")]
    fn launch_app_name(kind: &BrowserKind) -> Option<&'static str> {
        match kind {
            BrowserKind::Chrome => Some("Google Chrome"),
            BrowserKind::Edge => Some("Microsoft Edge"),
            BrowserKind::Brave => Some("Brave Browser"),
            BrowserKind::Arc => Some("Arc"),
            BrowserKind::Chromium => Some("Chromium"),
            BrowserKind::Unknown(_) => None,
        }
    }

    #[cfg(target_os = "macos")]
    fn spawn_macos_browser(kind: &BrowserKind, exe: &str, args: &[String]) -> std::io::Result<std::process::Child> {
        if let Some(app_name) = Self::launch_app_name(kind) {
            let mut command = super::launcher_lifecycle::silent_command("open");
            command.args(["-na", app_name, "--args"]);
            command.args(args);
            command.spawn()
        } else {
            super::launcher_lifecycle::silent_command(exe).args(args).spawn()
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn spawn_browser(
        _kind: &BrowserKind,
        exe: &str,
        args: &[String],
    ) -> std::io::Result<std::process::Child> {
        super::launcher_lifecycle::silent_command(exe).args(args).spawn()
    }

    #[cfg(target_os = "macos")]
    pub(super) fn spawn_browser(
        kind: &BrowserKind,
        exe: &str,
        args: &[String],
    ) -> std::io::Result<std::process::Child> {
        Self::spawn_macos_browser(kind, exe, args)
    }

    /// Get the executable path or launch command for a browser kind.
    pub fn browser_executable(kind: &BrowserKind) -> String {
        #[cfg(target_os = "macos")]
        {
            match kind {
                BrowserKind::Chrome => "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into(),
                BrowserKind::Edge => "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge".into(),
                BrowserKind::Brave => "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser".into(),
                BrowserKind::Arc => "/Applications/Arc.app/Contents/MacOS/Arc".into(),
                BrowserKind::Chromium => "/Applications/Chromium.app/Contents/MacOS/Chromium".into(),
                BrowserKind::Unknown(name) => name.clone(),
            }
        }

        #[cfg(target_os = "windows")]
        {
            Self::windows_browser_executable(kind)
        }

        #[cfg(target_os = "linux")]
        {
            match kind {
                BrowserKind::Chrome => "google-chrome".into(),
                BrowserKind::Edge => "microsoft-edge".into(),
                BrowserKind::Brave => "brave-browser".into(),
                BrowserKind::Chromium => "chromium-browser".into(),
                BrowserKind::Arc => "arc".into(),
                BrowserKind::Unknown(name) => name.clone(),
            }
        }
    }

    /// Windows: resolve a browser's executable path by probing common install
    /// locations (Program Files / Program Files (x86) / per-user LocalAppData)
    /// and then falling back to the registry "App Paths" entry.
    #[cfg(target_os = "windows")]
    fn windows_browser_executable(kind: &BrowserKind) -> String {
        let (rel_paths, app_paths_key, fallback_cmd) = match kind {
            BrowserKind::Chrome => (
                vec![r"Google\Chrome\Application\chrome.exe"],
                Some("chrome.exe"),
                "chrome.exe",
            ),
            BrowserKind::Edge => (
                vec![r"Microsoft\Edge\Application\msedge.exe"],
                Some("msedge.exe"),
                "msedge.exe",
            ),
            BrowserKind::Brave => (
                vec![r"BraveSoftware\Brave-Browser\Application\brave.exe"],
                Some("brave.exe"),
                "brave.exe",
            ),
            BrowserKind::Chromium => (vec![r"Chromium\Application\chrome.exe"], None, "chromium.exe"),
            BrowserKind::Arc => (vec![r"Arc\Arc.exe"], None, "arc.exe"),
            BrowserKind::Unknown(name) => return name.clone(),
        };

        let env_roots = [
            std::env::var("ProgramFiles").ok(),
            std::env::var("ProgramFiles(x86)").ok(),
            std::env::var("ProgramW6432").ok(),
            std::env::var("LOCALAPPDATA").ok(),
        ];

        for root in env_roots.iter().flatten() {
            for rel in &rel_paths {
                let candidate = format!(r"{}\{}", root.trim_end_matches('\\'), rel);
                if std::path::Path::new(&candidate).exists() {
                    tracing::debug!("Found browser at {}", candidate);
                    return candidate;
                }
            }
        }

        // App Paths registry fallback: HKLM/HKCU \Software\Microsoft\Windows
        // \CurrentVersion\App Paths\<exe>  default value points to the .exe.
        if let Some(exe_name) = app_paths_key {
            for root in &["HKCU", "HKLM"] {
                let key = format!(
                    r"{}\Software\Microsoft\Windows\CurrentVersion\App Paths\{}",
                    root, exe_name
                );
                let output = super::launcher_lifecycle::silent_command("reg")
                    .args(["query", &key, "/ve"])
                    .output()
                    .ok();
                if let Some(out) = output {
                    let text = String::from_utf8_lossy(&out.stdout);
                    // Line looks like:  (Default)    REG_SZ    C:\Path\to\app.exe
                    for line in text.lines() {
                        let lower = line.to_ascii_lowercase();
                        if lower.contains("reg_sz") {
                            if let Some(idx) = lower.find("reg_sz") {
                                let value = line[idx + "REG_SZ".len()..].trim();
                                let unquoted = value.trim_matches('"').trim();
                                if !unquoted.is_empty() && std::path::Path::new(unquoted).exists() {
                                    tracing::debug!("Resolved {} via App Paths: {}", exe_name, unquoted);
                                    return unquoted.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }

        fallback_cmd.into()
    }
}
