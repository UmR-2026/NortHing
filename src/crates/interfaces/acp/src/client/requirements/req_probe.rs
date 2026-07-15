use std::collections::HashMap;
use std::time::Duration;

use northhing_core::service::remote_ssh::SSHConnectionManager;
use northhing_core::util::errors::NortHingResult;
use tokio::process::Command;

use super::super::config::AcpRequirementProbeItem;
use super::super::remote_shell::{remote_user_shell_command, render_remote_env_assignments, shell_escape};
use super::req_session::{
    command_error_summary, find_executable, find_executable_with_path, npm_ls_package_version, parse_version_text,
    remote_command_error_summary, render_remote_env_prefix, run_command_with_timeout, REQUIREMENT_PROBE_TIMEOUT,
};

pub(crate) fn npx_adapter_probe_item(package: &str) -> AcpRequirementProbeItem {
    AcpRequirementProbeItem {
        name: package.to_string(),
        installed: true,
        version: None,
        path: Some("npx auto-install".to_string()),
        error: None,
    }
}

pub(crate) async fn probe_executable(command: &str) -> AcpRequirementProbeItem {
    let path = find_executable(command);
    let mut item = AcpRequirementProbeItem {
        name: command.to_string(),
        installed: path.is_some(),
        version: None,
        path: path.as_ref().map(|path| path.to_string_lossy().to_string()),
        error: None,
    };

    if let Some(path) = path {
        match run_command_with_timeout(path.as_os_str(), ["--version"], REQUIREMENT_PROBE_TIMEOUT).await {
            Ok(output) if output.status.success() => {
                item.version = parse_version_text(&output.stdout).or_else(|| parse_version_text(&output.stderr));
            }
            Ok(output) => {
                item.error = Some(command_error_summary(&output.stderr, &output.stdout));
            }
            Err(error) => {
                item.error = Some(error);
            }
        }
    }

    item
}

pub(crate) async fn probe_npm_adapter(package: &str, bin: &str) -> AcpRequirementProbeItem {
    probe_npm_adapter_with_path(package, bin, None).await
}

pub(crate) async fn probe_npm_adapter_with_path(
    package: &str,
    bin: &str,
    configured_path: Option<&std::ffi::OsStr>,
) -> AcpRequirementProbeItem {
    let mut item = AcpRequirementProbeItem {
        name: package.to_string(),
        installed: false,
        version: None,
        path: None,
        error: None,
    };
    if find_executable_with_path("npx", configured_path).is_some() {
        return npx_adapter_probe_item(package);
    }

    let npm_path = find_executable_with_path("npm", configured_path);
    let Some(npm_path) = npm_path else {
        item.error = Some("npm and npx are not available on PATH".to_string());
        return item;
    };

    let global_args = ["ls", "-g", "--json", "--depth=0", package];
    match run_command_with_timeout(npm_path.as_os_str(), global_args, REQUIREMENT_PROBE_TIMEOUT).await {
        Ok(output) if output.status.success() => {
            if let Some(version) = npm_ls_package_version(&output.stdout, package) {
                item.installed = true;
                item.version = Some(version);
                item.path = Some("npm global".to_string());
                return item;
            }
        }
        Ok(output) => {
            item.error = Some(command_error_summary(&output.stderr, &output.stdout));
        }
        Err(error) => {
            item.error = Some(error);
        }
    }

    let offline_args = [
        "exec".to_string(),
        "--offline".to_string(),
        "--yes".to_string(),
        format!("--package={package}"),
        "--".to_string(),
        bin.to_string(),
        "--help".to_string(),
    ];
    match run_command_with_timeout(
        npm_path.as_os_str(),
        offline_args.iter().map(String::as_str),
        REQUIREMENT_PROBE_TIMEOUT,
    )
    .await
    {
        Ok(output) if output.status.success() => {
            item.installed = true;
            item.path = Some("npm offline cache".to_string());
            item.error = None;
        }
        Ok(output) => {
            item.error = Some(command_error_summary(&output.stderr, &output.stdout));
        }
        Err(error) => {
            item.error = Some(error);
        }
    }

    item
}

pub(crate) async fn probe_remote_executable(
    ssh_manager: &SSHConnectionManager,
    connection_id: &str,
    command: &str,
    env: Option<&HashMap<String, String>>,
) -> AcpRequirementProbeItem {
    let mut item = AcpRequirementProbeItem {
        name: command.to_string(),
        installed: false,
        version: None,
        path: None,
        error: None,
    };

    let env_prefix = render_remote_env_prefix(env);
    let resolve_command = remote_user_shell_command(&format!("{env_prefix}command -v {}", shell_escape(command)));
    match ssh_manager.execute_command(connection_id, &resolve_command).await {
        Ok((stdout, _stderr, exit_code)) if exit_code == 0 => {
            let resolved_path = stdout
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(ToString::to_string);
            item.installed = resolved_path.is_some();
            item.path = resolved_path;
        }
        Ok((stdout, stderr, _)) => {
            let summary = remote_command_error_summary(&stderr, &stdout);
            if !summary.is_empty() {
                item.error = Some(summary);
            }
        }
        Err(error) => {
            item.error = Some(error.to_string());
        }
    }

    if item.installed {
        let version_command = remote_user_shell_command(&format!("{env_prefix}{} --version", shell_escape(command)));
        match ssh_manager.execute_command(connection_id, &version_command).await {
            Ok((stdout, stderr, exit_code)) if exit_code == 0 => {
                item.version = parse_version_text(stdout.as_bytes()).or_else(|| parse_version_text(stderr.as_bytes()));
            }
            Ok((stdout, stderr, _)) => {
                item.error = Some(remote_command_error_summary(&stderr, &stdout));
            }
            Err(error) => {
                item.error = Some(error.to_string());
            }
        }
    }

    item
}

pub(crate) async fn probe_remote_npx_adapter(
    ssh_manager: &SSHConnectionManager,
    connection_id: &str,
    package: &str,
    env: Option<&HashMap<String, String>>,
) -> AcpRequirementProbeItem {
    let mut item = AcpRequirementProbeItem {
        name: package.to_string(),
        installed: false,
        version: None,
        path: None,
        error: None,
    };

    let env_prefix = render_remote_env_prefix(env);
    let resolve_command = remote_user_shell_command(&format!("{env_prefix}command -v npx"));
    match ssh_manager.execute_command(connection_id, &resolve_command).await {
        Ok((stdout, _stderr, exit_code)) if exit_code == 0 => {
            item.installed = true;
            item.path = stdout
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(ToString::to_string)
                .or_else(|| Some("remote npx auto-install".to_string()));
        }
        Ok((stdout, stderr, _)) => {
            let summary = remote_command_error_summary(&stderr, &stdout);
            item.error = Some(if summary.is_empty() {
                "npx is not available on remote PATH".to_string()
            } else {
                summary
            });
        }
        Err(error) => {
            item.error = Some(error.to_string());
        }
    }

    item
}
