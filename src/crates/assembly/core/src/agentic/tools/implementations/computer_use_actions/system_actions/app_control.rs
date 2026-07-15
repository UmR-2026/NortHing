//! App launch, script execution, URL and file open handlers.

use crate::agentic::tools::framework::{ToolResult, ToolUseContext};
use crate::agentic::tools::implementations::computer_use_actions::{utilities, ComputerUseActions};
use crate::agentic::tools::implementations::control_hub::{err_response, ControlHubError, ErrorCode};
use crate::util::elapsed_ms_u64;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::process_manager;
use serde_json::{json, Value};

impl ComputerUseActions {
    fn platform_open_command(app_name: &str) -> (String, Vec<String>) {
        #[cfg(target_os = "macos")]
        {
            ("open".to_string(), vec!["-a".to_string(), app_name.to_string()])
        }
        #[cfg(target_os = "windows")]
        {
            (
                "cmd".to_string(),
                vec![
                    "/C".to_string(),
                    "start".to_string(),
                    "".to_string(),
                    app_name.to_string(),
                ],
            )
        }
        #[cfg(target_os = "linux")]
        {
            if utilities::which_exists("gtk-launch") {
                ("gtk-launch".to_string(), vec![app_name.to_string()])
            } else {
                (app_name.to_string(), vec![])
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            ("open".to_string(), vec![app_name.to_string()])
        }
    }

    pub(crate) async fn handle_open_app(
        &self,
        params: &Value,
        context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let app_name = params
            .get("app_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("open_app requires 'app_name'".to_string()))?;

        let mut host_attempted = false;
        let mut host_error: Option<String> = None;
        let method = "shell";

        let prefer_host = cfg!(target_os = "macos") && context.computer_use_host.is_some();
        if prefer_host {
            host_attempted = true;
            let cu_input = json!({ "action": "open_app", "app_name": app_name });
            match self.handle_desktop("open_app", &cu_input, context).await {
                Ok(results) => {
                    let host_payload = results.first().map(|r| r.content()).unwrap_or(Value::Null);
                    return Ok(vec![ToolResult::ok(
                        json!({
                            "launched": true,
                            "app": app_name,
                            "method": "computer_use_host",
                            "host_payload": host_payload,
                        }),
                        Some(format!("Opened {} via host", app_name)),
                    )]);
                }
                Err(e) => {
                    host_error = Some(e.to_string());
                }
            }
        }

        let attempts: Vec<(String, Vec<String>)> = {
            let primary = Self::platform_open_command(app_name);
            #[cfg(target_os = "linux")]
            {
                let mut v = vec![primary];
                let lower = app_name.to_lowercase();
                if v.iter().all(|(c, _)| c != &lower) {
                    v.push((lower, vec![]));
                }
                v.push(("xdg-open".to_string(), vec![app_name.to_string()]));
                v
            }
            #[cfg(not(target_os = "linux"))]
            {
                vec![primary]
            }
        };

        let mut last_err: Option<String> = None;
        let mut output_opt = None;
        let mut chosen_cmd = String::new();
        let mut chosen_args: Vec<String> = vec![];
        for (cmd, args) in &attempts {
            match crate::util::process_manager::create_command(cmd).args(args).output() {
                Ok(out) => {
                    if out.status.success() {
                        chosen_cmd = cmd.clone();
                        chosen_args = args.clone();
                        output_opt = Some(out);
                        break;
                    } else {
                        last_err = Some(format!(
                            "{} exit={:?} stderr={}",
                            cmd,
                            out.status.code(),
                            String::from_utf8_lossy(&out.stderr).trim()
                        ));
                    }
                }
                Err(e) => {
                    last_err = Some(format!("spawn {}: {}", cmd, e));
                }
            }
        }
        let _ = chosen_args;
        let output = output_opt.ok_or_else(|| {
            NortHingError::tool(format!(
                "open_app failed for '{}' across {} strategies: {} (host_error: {:?})",
                app_name,
                attempts.len(),
                last_err.as_deref().unwrap_or("(no error)"),
                host_error
            ))
        })?;

        if output.status.success() {
            let warning =
                host_error.map(|e| format!("computer_use_host open_app failed; shell fallback succeeded: {}", e));
            Ok(vec![ToolResult::ok(
                json!({
                    "launched": true,
                    "app": app_name,
                    "method": method,
                    "via_command": chosen_cmd,
                    "host_attempted": host_attempted,
                    "warning": warning,
                }),
                Some(format!("Opened {} via {}", app_name, chosen_cmd)),
            )])
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(NortHingError::tool(format!(
                "open_app failed for '{}'. host_attempted={}, host_error={:?}, last_command='{}', stderr='{}'",
                app_name, host_attempted, host_error, chosen_cmd, stderr
            )))
        }
    }

    pub(crate) async fn handle_run_script(
        &self,
        params: &Value,
        context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let _context = context;
        let script = params
            .get("script")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("run_script requires 'script'".to_string()))?;
        let script_type = params
            .get("script_type")
            .and_then(|v| v.as_str())
            .unwrap_or("applescript");
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .filter(|value| *value > 0);
        let max_output_bytes = params
            .get("max_output_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(16 * 1024)
            .clamp(1024, 256 * 1024) as usize;

        let (program, args) = match script_type {
            "applescript" => {
                #[cfg(target_os = "macos")]
                {
                    (
                        "/usr/bin/osascript".to_string(),
                        vec!["-e".to_string(), script.to_string()],
                    )
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = script;
                    return Ok(err_response(
                        "system",
                        "run_script",
                        ControlHubError::new(
                            ErrorCode::NotAvailable,
                            "AppleScript is only available on macOS",
                        )
                        .with_hint("Use script_type='shell' (sh on Unix, PowerShell on Windows) or script_type='powershell'/'bash'"),
                    ));
                }
            }
            "shell" => {
                #[cfg(target_os = "windows")]
                {
                    utilities::powershell_invocation(script)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    (
                        "sh".to_string(),
                        vec!["-c".to_string(), script.to_string()],
                    )
                }
            }
            "bash" => {
                if utilities::which_exists("bash") {
                    return Ok(err_response(
                        "system",
                        "run_script",
                        ControlHubError::new(
                            ErrorCode::NotAvailable,
                            "bash is not on PATH",
                        )
                        .with_hint("Install Git for Windows / WSL, or use script_type='shell' / 'powershell' / 'cmd'"),
                    ));
                }
                (
                    "bash".to_string(),
                    vec!["-c".to_string(), script.to_string()],
                )
            }
            "powershell" => {
                let prog = if utilities::which_exists("pwsh") {
                    "pwsh"
                } else if utilities::which_exists("powershell") {
                    "powershell"
                } else {
                    return Ok(err_response(
                        "system",
                        "run_script",
                        ControlHubError::new(
                            ErrorCode::NotAvailable,
                            "Neither pwsh nor powershell are on PATH",
                        )
                        .with_hint("Install PowerShell, or use script_type='shell' / 'bash'"),
                    ));
                };
                (
                    prog.to_string(),
                    vec![
                        "-NoProfile".to_string(),
                        "-NonInteractive".to_string(),
                        "-Command".to_string(),
                        format!(
                            "[Console]::OutputEncoding=[Text.Encoding]::UTF8; {}",
                            script
                        ),
                    ],
                )
            }
            "cmd" => {
                #[cfg(target_os = "windows")]
                {
                    (
                        "cmd".to_string(),
                        vec![
                            "/U".to_string(),
                            "/C".to_string(),
                            format!("chcp 65001>nul && {}", script),
                        ],
                    )
                }
                #[cfg(not(target_os = "windows"))]
                {
                    return Ok(err_response(
                        "system",
                        "run_script",
                        ControlHubError::new(
                            ErrorCode::NotAvailable,
                            "script_type='cmd' is only available on Windows",
                        )
                        .with_hint("Use script_type='shell' / 'bash' / 'powershell'"),
                    ));
                }
            }
            other => {
                return Err(NortHingError::tool(format!(
                    "Unknown script_type: '{}'. Valid: applescript (macOS), shell (OS default), bash, powershell, cmd (Windows)",
                    other
                )))
            }
        };

        let started = std::time::Instant::now();
        let child = process_manager::create_tokio_command(&program)
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| NortHingError::tool(format!("Failed to spawn run_script ({}): {}", script_type, e)))?;

        let wait = child.wait_with_output();
        let output = if let Some(timeout_ms) = timeout_ms {
            match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), wait).await {
                Err(_) => {
                    return Ok(err_response(
                        "system",
                        "run_script",
                        ControlHubError::new(
                            ErrorCode::Timeout,
                            format!(
                                "run_script timed out after {} ms (script_type={}); child process killed",
                                timeout_ms, script_type
                            ),
                        )
                        .with_hint("Increase 'timeout_ms', set it to 0, or omit it to wait without a timeout"),
                    ));
                }
                Ok(Err(e)) => {
                    return Err(NortHingError::tool(format!(
                        "Failed to wait for run_script ({}): {}",
                        script_type, e
                    )));
                }
                Ok(Ok(o)) => o,
            }
        } else {
            match wait.await {
                Ok(o) => o,
                Err(e) => {
                    return Err(NortHingError::tool(format!(
                        "Failed to wait for run_script ({}): {}",
                        script_type, e
                    )));
                }
            }
        };

        let elapsed_ms = elapsed_ms_u64(started);
        let stdout_full = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_full = String::from_utf8_lossy(&output.stderr).to_string();
        let (stdout, stdout_truncated) = utilities::truncate_with_marker(&stdout_full, max_output_bytes);
        let (stderr, stderr_truncated) = utilities::truncate_with_marker(&stderr_full, max_output_bytes);

        if output.status.success() {
            Ok(vec![ToolResult::ok(
                json!({
                    "success": true,
                    "output": stdout,
                    "stderr": stderr,
                    "stdout_truncated": stdout_truncated,
                    "stderr_truncated": stderr_truncated,
                    "exit_code": output.status.code(),
                    "elapsed_ms": elapsed_ms,
                    "script_type": script_type,
                }),
                Some(if stdout.is_empty() {
                    format!("Script executed in {} ms", elapsed_ms)
                } else {
                    stdout.lines().take(1).collect::<String>()
                }),
            )])
        } else {
            Ok(err_response(
                "system",
                "run_script",
                ControlHubError::new(
                    ErrorCode::Internal,
                    format!(
                        "Script exited with {:?}: {}",
                        output.status.code(),
                        stderr.lines().next().unwrap_or("(no stderr)")
                    ),
                )
                .with_hints([format!("stderr={}", stderr), format!("elapsed_ms={}", elapsed_ms)]),
            ))
        }
    }

    pub(crate) async fn handle_open_url(
        &self,
        params: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("open_url requires 'url'".to_string()))?;
        if !(url.starts_with("http://")
            || url.starts_with("https://")
            || url.starts_with("file://")
            || url.starts_with("mailto:"))
        {
            return Ok(err_response(
                "system",
                "open_url",
                ControlHubError::new(
                    ErrorCode::InvalidParams,
                    format!("Refusing to open URL with unsupported scheme: {}", url),
                )
                .with_hint(
                    "Pass an http(s)://, file://, or mailto: URL. Use 'open_file' for local paths without a scheme.",
                ),
            ));
        }

        let (program, args) = match std::env::consts::OS {
            "macos" => ("open".to_string(), vec![url.to_string()]),
            "windows" => (
                "rundll32".to_string(),
                vec!["url.dll,FileProtocolHandler".to_string(), url.to_string()],
            ),
            _ => ("xdg-open".to_string(), vec![url.to_string()]),
        };
        let status = process_manager::create_command(&program)
            .args(&args)
            .status()
            .map_err(|e| NortHingError::tool(format!("Failed to spawn '{}': {}", program, e)))?;
        if status.success() {
            Ok(vec![ToolResult::ok(
                json!({ "opened": true, "url": url, "method": program }),
                Some(format!("Opened {} in default handler", url)),
            )])
        } else {
            Ok(err_response(
                "system",
                "open_url",
                ControlHubError::new(
                    ErrorCode::Internal,
                    format!("'{}' exited with {:?}", program, status.code()),
                ),
            ))
        }
    }

    pub(crate) async fn handle_open_file(
        &self,
        params: &Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("open_file requires 'path'".to_string()))?;
        let app_name = params.get("app").and_then(|v| v.as_str());

        let path = std::path::Path::new(path_str);
        if !path.exists() {
            return Ok(err_response(
                "system",
                "open_file",
                ControlHubError::new(ErrorCode::NotFound, format!("File does not exist: {}", path_str))
                    .with_hint("Check the absolute path; ~ is not expanded"),
            ));
        }

        let (program, args) = match (std::env::consts::OS, app_name) {
            ("macos", Some(app)) => (
                "open".to_string(),
                vec!["-a".to_string(), app.to_string(), path_str.to_string()],
            ),
            ("macos", None) => ("open".to_string(), vec![path_str.to_string()]),
            ("windows", _) => (
                "rundll32".to_string(),
                vec!["url.dll,FileProtocolHandler".to_string(), path_str.to_string()],
            ),
            _ => ("xdg-open".to_string(), vec![path_str.to_string()]),
        };
        let status = process_manager::create_command(&program)
            .args(&args)
            .status()
            .map_err(|e| NortHingError::tool(format!("Failed to spawn '{}': {}", program, e)))?;
        if status.success() {
            Ok(vec![ToolResult::ok(
                json!({
                    "opened": true,
                    "path": path_str,
                    "with_app": app_name,
                    "method": program,
                }),
                Some(match app_name {
                    Some(a) => format!("Opened {} with {}", path_str, a),
                    None => format!("Opened {} with default handler", path_str),
                }),
            )])
        } else {
            Ok(err_response(
                "system",
                "open_file",
                ControlHubError::new(
                    ErrorCode::Internal,
                    format!("'{}' exited with {:?}", program, status.code()),
                ),
            ))
        }
    }
}
