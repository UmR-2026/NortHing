//! Computer Use desktop and OS/system action implementations.
//!
//! This module owns the action logic that used to live behind ControlHub's
//! desktop/system domains. ControlHub may still share the common error envelope
//! types, but it no longer owns these Computer Use behaviors.

//! Cross-platform utility helpers used by the system-domain dispatcher.
//!
//! Pure free-standing functions that exist so other modules in the crate
//! can reuse the same OS-detection / PATH-lookup / clipboard primitives.
//! Most call sites are inside `handle_system`; the `pub(crate)` ones
//! (`truncate_with_marker`, `which_exists`, `linux_clipboard_install_hints`,
//! `linux_session_info`) are also consumed by `control_hub_tool_*` (browser,
//! meta, tests) which need direct access to the same helpers.
//!
//! Visibility rules (R37h):
//! * `pub(crate)` for items already part of the crate's external
//!   surface (preserves existing import paths).
//! * `pub(super)` for free functions that were previously private but
//!   are now called from sibling files (e.g. `clipboard_write` from
//!   `desktop_actions::handle_desktop`).
//! * `fn` (no modifier) for helpers used only inside `utilities.rs`.

use crate::util::process_manager;
/// Truncate `s` to at most `max_bytes`, appending an explicit marker so the
/// model can see that data was dropped (and how much). Returns
/// `(truncated_string, was_truncated)`.
pub(crate) fn truncate_with_marker(s: &str, max_bytes: usize) -> (String, bool) {
    if s.len() <= max_bytes {
        return (s.to_string(), false);
    }
    let head_n = max_bytes.saturating_sub(64);
    let head = safe_str_slice(s, head_n);
    let omitted = s.len().saturating_sub(head_n);
    (format!("{}\n... [{} bytes omitted] ...\n", head, omitted), true)
}
/// Slice `s` to ≤ `n` bytes without splitting a UTF-8 codepoint.
fn safe_str_slice(s: &str, n: usize) -> &str {
    if n >= s.len() {
        return s;
    }
    let mut cut = n;
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    &s[..cut]
}

/// Read a short OS version string. Best-effort: returns `None` on platforms
/// where we can't determine it cheaply.
pub(super) fn read_os_version() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(format!("macOS {}", s))
        }
    }
    #[cfg(target_os = "windows")]
    {
        let out = crate::util::process_manager::create_command("cmd")
            .args(["/C", "ver"])
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
    #[cfg(target_os = "linux")]
    {
        // /etc/os-release is the canonical lookup.
        let txt = std::fs::read_to_string("/etc/os-release").ok()?;
        for line in txt.lines() {
            if let Some(rest) = line.strip_prefix("PRETTY_NAME=") {
                return Some(rest.trim_matches('"').to_string());
            }
        }
        None
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        None
    }
}

pub(super) fn hostname() -> std::io::Result<String> {
    // Prefer environment variables on each OS so we never have to spawn a
    // subprocess for a value that's already in our address space, and so we
    // never ingest a non-UTF-8 byte stream from `hostname.exe` on Windows
    // running a CJK code page.
    #[cfg(target_os = "windows")]
    {
        if let Ok(name) = std::env::var("COMPUTERNAME") {
            if !name.is_empty() {
                return Ok(name);
            }
        }
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        if let Ok(name) = std::env::var("HOSTNAME") {
            if !name.is_empty() {
                return Ok(name);
            }
        }
        if let Ok(bytes) = std::fs::read("/etc/hostname") {
            let s = String::from_utf8_lossy(&bytes).trim().to_string();
            if !s.is_empty() {
                return Ok(s);
            }
        }
    }
    let out = process_manager::create_command("hostname").output()?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Cheap PATH lookup for an executable name. Used to decide between e.g.
/// `pwsh` and `powershell`, or to surface a structured `NOT_AVAILABLE`
/// error when the requested interpreter isn't installed.
pub(crate) fn which_exists(name: &str) -> bool {
    let paths = match std::env::var_os("PATH") {
        Some(p) => p,
        None => return false,
    };
    let exts: Vec<String> = if cfg!(target_os = "windows") {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.BAT;.CMD;.COM".to_string())
            .split(';')
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(&paths) {
        for ext in &exts {
            let mut candidate = dir.join(name);
            if !ext.is_empty() {
                let stem = candidate.file_name().map(|n| n.to_os_string());
                if let Some(mut stem) = stem {
                    stem.push(ext);
                    candidate.set_file_name(stem);
                }
            }
            if candidate.exists() {
                return true;
            }
        }
    }
    false
}

/// Build a `(program, args)` pair for invoking a PowerShell snippet on Windows
/// with UTF-8 output forced. Centralised so the "shell" alias and an explicit
/// `script_type='powershell'` produce the same encoding.
#[cfg(target_os = "windows")]
pub(super) fn powershell_invocation(script: &str) -> (String, Vec<String>) {
    let prog = if which_exists("pwsh") { "pwsh" } else { "powershell" };
    (
        prog.to_string(),
        vec![
            "-NoProfile".to_string(),
            "-NonInteractive".to_string(),
            "-Command".to_string(),
            format!("[Console]::OutputEncoding=[Text.Encoding]::UTF8; {}", script),
        ],
    )
}

/// Build OS-specific install hints for the clipboard helper. On Linux we
/// inspect the session type so the suggestion matches what the user actually
/// needs (Wayland users wasting time installing xclip is a real failure mode).
pub(crate) fn linux_clipboard_install_hints() -> Vec<String> {
    match std::env::consts::OS {
        "linux" => {
            #[cfg(target_os = "linux")]
            {
                let (server, _) = linux_session_info();
                match server.as_deref() {
                    Some("wayland") => vec![
                        "Wayland session detected — install wl-clipboard (e.g. `sudo apt install wl-clipboard` / `sudo dnf install wl-clipboard`)".to_string(),
                        "Fallback for XWayland apps: also install xclip or xsel".to_string(),
                    ],
                    Some("x11") | Some("tty") => vec![
                        "X11 session detected — install xclip (`sudo apt install xclip`) or xsel (`sudo apt install xsel`)".to_string(),
                    ],
                    _ => vec![
                        "Install wl-clipboard (Wayland) OR xclip/xsel (X11). Run `echo $XDG_SESSION_TYPE` to know which one applies.".to_string(),
                    ],
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                vec!["Install wl-clipboard (Wayland) or xclip/xsel (X11)".to_string()]
            }
        }
        _ => vec!["Make sure the system clipboard helper is available on this host".to_string()],
    }
}
/// Best-effort detection of the Linux desktop session metadata (display
/// server + desktop environment). Returns `(display_server, desktop_env)`,
/// either of which may be `None` if the environment doesn't expose it.
#[cfg(target_os = "linux")]
pub(crate) fn linux_session_info() -> (Option<String>, Option<String>) {
    let server = std::env::var("XDG_SESSION_TYPE").ok().filter(|s| !s.is_empty());
    let de = std::env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .or_else(|| std::env::var("DESKTOP_SESSION").ok())
        .filter(|s| !s.is_empty());
    (server, de)
}

/// Cross-platform clipboard read. Shells out to the canonical helper for
/// the current OS so we don't pull in a heavyweight dependency for what is
/// fundamentally a 1-line operation. Linux auto-detects Wayland → X11.
pub(super) async fn clipboard_read() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let out = process_manager::create_tokio_command("pbpaste")
            .output()
            .await
            .map_err(|e| format!("spawn pbpaste: {}", e))?;
        if !out.status.success() {
            return Err(format!("pbpaste exit={:?}", out.status.code()));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
    #[cfg(target_os = "windows")]
    {
        let (program, args) = powershell_invocation("Get-Clipboard -Raw");
        let out = process_manager::create_tokio_command(&program)
            .args(&args)
            .output()
            .await
            .map_err(|e| format!("spawn {}: {}", program, e))?;
        if !out.status.success() {
            return Err(format!("Get-Clipboard exit={:?}", out.status.code()));
        }
        // PowerShell appends CRLF; trim a single trailing newline so the
        // returned text matches what the user actually copied.
        let mut s = String::from_utf8_lossy(&out.stdout).to_string();
        if s.ends_with("\r\n") {
            s.truncate(s.len() - 2);
        } else if s.ends_with('\n') {
            s.truncate(s.len() - 1);
        }
        Ok(s)
    }
    #[cfg(target_os = "linux")]
    {
        // Wayland first (modern session), then X11 fallbacks.
        let candidates: &[(&str, &[&str])] = if std::env::var("WAYLAND_DISPLAY").is_ok() {
            &[
                ("wl-paste", &["--no-newline"]),
                ("xclip", &["-selection", "clipboard", "-o"]),
                ("xsel", &["--clipboard", "--output"]),
            ]
        } else {
            &[
                ("xclip", &["-selection", "clipboard", "-o"]),
                ("xsel", &["--clipboard", "--output"]),
                ("wl-paste", &["--no-newline"]),
            ]
        };
        for (bin, args) in candidates {
            if let Ok(out) = process_manager::create_tokio_command(bin).args(*args).output().await {
                if out.status.success() {
                    return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                }
            }
        }
        Err("no clipboard helper found (install wl-clipboard, xclip, or xsel)".to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("clipboard not implemented for this OS".to_string())
    }
}

/// Cross-platform clipboard write. Streams `text` into the helper's stdin
/// rather than embedding it in argv so newlines / quotes / shell metachars
/// are preserved verbatim.
pub(super) async fn clipboard_write(text: &str) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;

    async fn pipe(bin: &str, args: &[&str], text: &str) -> Result<(), String> {
        let mut child = process_manager::create_tokio_command(bin)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn {}: {}", bin, e))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(text.as_bytes())
                .await
                .map_err(|e| format!("write {} stdin: {}", bin, e))?;
        }
        let out = child
            .wait_with_output()
            .await
            .map_err(|e| format!("wait {}: {}", bin, e))?;
        if !out.status.success() {
            return Err(format!("{} exit={:?}", bin, out.status.code()));
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        pipe("pbcopy", &[], text).await
    }
    #[cfg(target_os = "windows")]
    {
        // PowerShell's Set-Clipboard reads from the pipeline; pipe text in
        // via stdin to preserve binary fidelity.
        pipe(
            "powershell",
            &["-NoProfile", "-Command", "$input | Set-Clipboard"],
            text,
        )
        .await
    }
    #[cfg(target_os = "linux")]
    {
        let candidates: &[(&str, &[&str])] = if std::env::var("WAYLAND_DISPLAY").is_ok() {
            &[
                ("wl-copy", &[]),
                ("xclip", &["-selection", "clipboard"]),
                ("xsel", &["--clipboard", "--input"]),
            ]
        } else {
            &[
                ("xclip", &["-selection", "clipboard"]),
                ("xsel", &["--clipboard", "--input"]),
                ("wl-copy", &[]),
            ]
        };
        let mut last_err = String::new();
        for (bin, args) in candidates {
            match pipe(bin, args, text).await {
                Ok(()) => return Ok(()),
                Err(e) => last_err = e,
            }
        }
        Err(format!(
            "no clipboard helper succeeded (install wl-clipboard, xclip, or xsel): {}",
            last_err
        ))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = text;
        Err("clipboard not implemented for this OS".to_string())
    }
}
