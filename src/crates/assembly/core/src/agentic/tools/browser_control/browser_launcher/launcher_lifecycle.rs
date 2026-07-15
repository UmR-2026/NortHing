//! Browser launch, shutdown, cleanup lifecycle.

use super::launcher_types::{BrowserKind, LaunchResult};
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::process_manager;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// Build a `Command` that suppresses transient Windows console windows while
/// preserving normal process behavior on other platforms.
pub(super) fn silent_command<S: AsRef<OsStr>>(program: S) -> Command {
    process_manager::create_command(program)
}

pub struct BrowserLauncher;

impl BrowserLauncher {
    /// Launch the browser with the CDP debug port flag.
    /// Returns instructions if the browser is already running without CDP.
    pub async fn launch_with_cdp(kind: &BrowserKind, port: u16) -> NortHingResult<LaunchResult> {
        Self::launch_with_cdp_opts(kind, port, None).await
    }

    /// Same as [`launch_with_cdp`] but allows passing an isolated
    /// `--user-data-dir`. When the user is already running their main
    /// browser without CDP, an isolated profile lets us start a sibling
    /// instance with debugging enabled instead of asking them to quit.
    pub async fn launch_with_cdp_opts(
        kind: &BrowserKind,
        port: u16,
        user_data_dir: Option<&str>,
    ) -> NortHingResult<LaunchResult> {
        if Self::is_cdp_available(port).await {
            tracing::info!("CDP already available on port {} for {}", port, kind);
            return Ok(LaunchResult::AlreadyConnected);
        }

        let exe = Self::browser_executable(kind);
        let profile_dir = match user_data_dir {
            Some(dir) => Path::new(dir).to_path_buf(),
            None => Self::ensure_managed_user_data_dir(kind)?,
        };
        let flag = format!("--remote-debugging-port={}", port);
        let profile_flag = format!("--user-data-dir={}", profile_dir.display());
        let extra: Vec<String> = vec![
            flag.clone(),
            profile_flag,
            "--no-first-run".to_string(),
            "--no-default-browser-check".to_string(),
        ];

        tracing::info!(
            "Launching {} with CDP on port {} (user_data_dir={})",
            kind,
            port,
            profile_dir.display()
        );
        let result = Self::spawn_browser(kind, &exe, &extra);

        match result {
            Ok(_child) => {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                if Self::is_cdp_available(port).await {
                    Ok(LaunchResult::Launched)
                } else {
                    Ok(LaunchResult::LaunchedButCdpNotReady {
                        port,
                        message: format!(
                            "{} was launched but CDP is not yet responding on port {}. \
                             It may need a few more seconds to initialize.",
                            kind, port
                        ),
                    })
                }
            }
            Err(e) => Err(NortHingError::tool(format!("Failed to launch {}: {}", kind, e))),
        }
    }

    // reason: terminate_browser() is reserved for the upcoming graceful browser shutdown on restart (today restart relaunches without explicit termination)
    #[allow(dead_code)]
    fn terminate_browser(kind: &BrowserKind) -> NortHingResult<()> {
        #[cfg(target_os = "macos")]
        {
            let app_name = match kind {
                BrowserKind::Chrome => "Google Chrome",
                BrowserKind::Edge => "Microsoft Edge",
                BrowserKind::Brave => "Brave Browser",
                BrowserKind::Arc => "Arc",
                BrowserKind::Chromium => "Chromium",
                BrowserKind::Unknown(name) => name.as_str(),
            };
            let script = format!("tell application \"{}\" to quit", app_name.replace('"', "\\\""));
            let output = silent_command("osascript")
                .args(["-e", &script])
                .output()
                .map_err(|e| NortHingError::tool(format!("Failed to quit {}: {}", kind, e)))?;
            if output.status.success() {
                return Ok(());
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NortHingError::tool(format!(
                "Failed to quit {}: {}",
                kind,
                stderr.trim()
            )));
        }

        #[cfg(target_os = "windows")]
        {
            let process_names: &[&str] = match kind {
                BrowserKind::Chrome => &["chrome.exe"],
                BrowserKind::Edge => &["msedge.exe"],
                BrowserKind::Brave => &["brave.exe"],
                BrowserKind::Arc => &["arc.exe"],
                BrowserKind::Chromium => &["chromium.exe", "chrome.exe"],
                BrowserKind::Unknown(_) => {
                    return Err(NortHingError::tool(
                        "Unsupported browser kind for restart on Windows".to_string(),
                    ))
                }
            };
            for process_name in process_names {
                let output = silent_command("taskkill")
                    .args(["/IM", process_name, "/F"])
                    .output()
                    .map_err(|e| NortHingError::tool(format!("Failed to terminate {}: {}", process_name, e)))?;
                let stdout = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
                let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
                if output.status.success()
                    || stdout.contains("no instance")
                    || stdout.contains("not found")
                    || stderr.contains("no instance")
                    || stderr.contains("not found")
                {
                    continue;
                }
                return Err(NortHingError::tool(format!(
                    "Failed to terminate {}: {}{}",
                    process_name,
                    String::from_utf8_lossy(&output.stdout).trim(),
                    String::from_utf8_lossy(&output.stderr).trim()
                )));
            }
            Ok(())
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = kind;
            Err(NortHingError::tool(
                "Browser restart with CDP is not supported on this platform".to_string(),
            ))
        }
    }

    // reason: wait_for_browser_exit() is reserved for the upcoming restart sequencing (today the restart path does not wait for graceful exit)
    #[allow(dead_code)]
    async fn wait_for_browser_exit(kind: &BrowserKind, timeout: Duration) -> NortHingResult<()> {
        let started = std::time::Instant::now();
        while Self::is_browser_running(kind) {
            if started.elapsed() >= timeout {
                return Err(NortHingError::tool(format!(
                    "Timed out waiting for {} to exit before restart",
                    kind
                )));
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Ok(())
    }

    /// Create a macOS `.app` wrapper that launches the browser with CDP enabled.
    #[cfg(target_os = "macos")]
    pub fn create_cdp_launcher_app(kind: &BrowserKind, port: u16) -> NortHingResult<String> {
        let app_name = format!("{} Debug", kind);
        let app_dir = format!("/Applications/{}.app", app_name);
        let macos_dir = format!("{}/Contents/MacOS", app_dir);
        let script_path = format!("{}/launch", macos_dir);
        let exe = Self::browser_executable(kind);

        std::fs::create_dir_all(&macos_dir)
            .map_err(|e| NortHingError::tool(format!("Failed to create app bundle: {}", e)))?;

        let script = format!(
            "#!/bin/bash\nexec \"{}\" --remote-debugging-port={} \"$@\"\n",
            exe, port
        );
        std::fs::write(&script_path, &script)
            .map_err(|e| NortHingError::tool(format!("Failed to write launcher script: {}", e)))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| NortHingError::tool(format!("Failed to set executable permission: {}", e)))?;
        }

        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{}</string>
    <key>CFBundleExecutable</key>
    <string>launch</string>
    <key>CFBundleIdentifier</key>
    <string>com.northhing.browser-debug-launcher</string>
</dict>
</plist>"#,
            app_name
        );

        std::fs::write(format!("{}/Contents/Info.plist", app_dir), &plist)
            .map_err(|e| NortHingError::tool(format!("Failed to write Info.plist: {}", e)))?;

        tracing::info!("Created CDP launcher app at {}", app_dir);
        Ok(app_dir)
    }
}
