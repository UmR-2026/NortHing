//! LSP server process command construction.
//!
//! `detect_runtime_type` selects `Bash` / `Node` / `Executable` from the
//! config or the binary extension. `build_command` constructs the platform
//! `tokio::process::Command`, including the cross-platform corner cases:
//!
//! - Windows `.bat` / `.cmd`: scan for an embedded `node "..."` invocation
//!   and resolve its path through `%SCRIPT_DIR%` indirection.
//! - Windows Bash runtime: discover Git Bash / WSL by trying a fixed list
//!   of candidate paths.
//! - Node runtime: refuse to start when `node.exe`/`node` is missing on
//!   PATH.

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

use super::process::LspServerProcess;
use super::types::{RuntimeType, ServerConfig};

impl LspServerProcess {
    /// Detects the runtime type.
    pub(super) fn detect_runtime_type(config: &ServerConfig, server_bin: &Path) -> RuntimeType {
        if let Some(runtime) = &config.runtime {
            debug!("Runtime explicitly specified: {}", runtime);
            return match runtime.to_lowercase().as_str() {
                "bash" | "sh" => RuntimeType::Bash,
                "node" | "nodejs" => RuntimeType::Node,
                "exe" | "executable" => RuntimeType::Executable,
                _ => {
                    warn!("Unknown runtime type '{}', defaulting to executable", runtime);
                    RuntimeType::Executable
                }
            };
        }

        if let Some(ext) = server_bin.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "sh" | "bash" => return RuntimeType::Bash,
                "js" | "mjs" | "cjs" => return RuntimeType::Node,
                _ => {}
            }
        }

        RuntimeType::Executable
    }

    /// Builds the command based on the runtime type.
    pub(super) fn build_command(
        runtime_type: &RuntimeType,
        server_bin: &PathBuf,
        config: &ServerConfig,
    ) -> Result<tokio::process::Command> {
        match runtime_type {
            RuntimeType::Executable => {
                #[cfg(windows)]
                {
                    if let Some(ext) = server_bin.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();

                        if ext_lower == "bat" || ext_lower == "cmd" {
                            debug!("Detected batch file (.{}), extracting node command", ext_lower);

                            if let Ok(content) = std::fs::read_to_string(server_bin) {
                                let mut script_path: Option<PathBuf> = None;

                                for line in content.lines() {
                                    let line = line.trim();

                                    if line.starts_with("node ") || line.starts_with("node.exe ") {
                                        info!("Found node execution command: {}", line);

                                        if let Some(start_quote) = line.find('"') {
                                            if let Some(end_quote) = line[start_quote + 1..].find('"') {
                                                let path_expr = &line[start_quote + 1..start_quote + 1 + end_quote];
                                                debug!("Extracted path expression: {}", path_expr);

                                                for prev_line in content.lines() {
                                                    let prev_line = prev_line.trim();
                                                    if prev_line.starts_with("set ")
                                                        && prev_line.contains(
                                                            path_expr.trim_matches('%').split('%').next().unwrap_or(""),
                                                        )
                                                    {
                                                        if let Some(eq_pos) = prev_line.find('=') {
                                                            let value_part = &prev_line[eq_pos + 1..].trim_matches('"');

                                                            if let Some(parent) = server_bin.parent() {
                                                                let mut resolved_path = parent.to_path_buf();

                                                                let rel_part = value_part.replace("%SCRIPT_DIR%", "");

                                                                for component in rel_part.split(['\\', '/']) {
                                                                    match component {
                                                                        "" | "." => continue,
                                                                        ".." => {
                                                                            resolved_path.pop();
                                                                        }
                                                                        part => resolved_path.push(part),
                                                                    }
                                                                }

                                                                if resolved_path.exists() {
                                                                    script_path = Some(resolved_path);
                                                                    break;
                                                                } else {
                                                                    warn!(
                                                                        "Resolved path does not exist: {:?}",
                                                                        resolved_path
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        break;
                                    }
                                }

                                if let Some(js_path) = script_path {
                                    let node_cmd = if cfg!(windows) { "node.exe" } else { "node" };

                                    let mut cmd = crate::util::process_manager::create_tokio_command(node_cmd);
                                    cmd.arg(js_path);
                                    cmd.args(&config.args);
                                    cmd.envs(&config.env);
                                    return Ok(cmd);
                                }
                            }

                            error!("Failed to extract node command from bat file");
                            error!("Bat files cannot be executed directly without cmd wrapper");
                            return Err(anyhow!(
                                "Failed to parse batch file. Please check the plugin installation."
                            ));
                        }
                    }
                }

                let mut cmd = crate::util::process_manager::create_tokio_command(server_bin);
                cmd.args(&config.args);
                cmd.envs(&config.env);
                Ok(cmd)
            }
            RuntimeType::Bash => {
                #[cfg(windows)]
                {
                    let bash_paths = vec![
                        "bash.exe",
                        "C:\\Program Files\\Git\\bin\\bash.exe",
                        "C:\\Program Files (x86)\\Git\\bin\\bash.exe",
                        "wsl.exe",
                    ];

                    let mut bash_exe = None;
                    for path in &bash_paths {
                        if crate::util::process_manager::create_command(path)
                            .arg("--version")
                            .output()
                            .is_ok()
                        {
                            bash_exe = Some(path.to_string());
                            break;
                        }
                    }

                    let bash_cmd = bash_exe.ok_or_else(|| {
                        error!("Bash not found on Windows. Searched paths: {:?}", bash_paths);
                        anyhow!(
                            "Bash not found on Windows. Please install Git Bash or WSL.\n\
                             - Git Bash: https://git-scm.com/download/win\n\
                             - WSL: https://docs.microsoft.com/windows/wsl/install"
                        )
                    })?;

                    let mut cmd = crate::util::process_manager::create_tokio_command(&bash_cmd);
                    cmd.arg(server_bin);
                    cmd.args(&config.args);
                    cmd.envs(&config.env);
                    Ok(cmd)
                }

                #[cfg(not(windows))]
                {
                    let mut cmd = crate::util::process_manager::create_tokio_command("bash");
                    cmd.arg(server_bin);
                    cmd.args(&config.args);
                    cmd.envs(&config.env);
                    Ok(cmd)
                }
            }
            RuntimeType::Node => {
                let node_cmd = if cfg!(windows) { "node.exe" } else { "node" };

                match crate::util::process_manager::create_command(node_cmd)
                    .arg("--version")
                    .output()
                {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Node.js not found: {}", e);
                        return Err(anyhow!(
                            "Node.js not found. Please install Node.js from https://nodejs.org/\n\
                             The LSP plugin requires Node.js to be installed and available in PATH."
                        ));
                    }
                }

                let mut cmd = crate::util::process_manager::create_tokio_command(node_cmd);
                cmd.arg(server_bin);
                cmd.args(&config.args);
                cmd.envs(&config.env);
                Ok(cmd)
            }
        }
    }
}
