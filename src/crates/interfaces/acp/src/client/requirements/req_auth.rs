use northhing_core::service::remote_ssh::{SSHCommandOptions, SSHConnectionManager};
use northhing_core::util::errors::{NortHingError, NortHingResult};

use super::super::config::AcpRequirementProbeItem;
use super::super::remote_shell::{remote_user_shell_command, shell_escape};
use super::req_session::{
    command_error_summary, find_executable, remote_command_error_summary, run_command_with_timeout,
    ADAPTER_DOWNLOAD_TIMEOUT, CLI_INSTALL_TIMEOUT,
};

pub(crate) async fn predownload_npm_adapter(package: &str, bin: &str) -> NortHingResult<()> {
    let npm_path =
        find_executable("npm").ok_or_else(|| NortHingError::service("npm is not available on PATH".to_string()))?;
    let args = [
        "exec".to_string(),
        "--yes".to_string(),
        format!("--package={package}"),
        "--".to_string(),
        bin.to_string(),
        "--help".to_string(),
    ];

    match run_command_with_timeout(
        npm_path.as_os_str(),
        args.iter().map(String::as_str),
        ADAPTER_DOWNLOAD_TIMEOUT,
    )
    .await
    {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(NortHingError::service(format!(
            "Failed to predownload ACP adapter '{}': {}",
            package,
            command_error_summary(&output.stderr, &output.stdout)
        ))),
        Err(error) => Err(NortHingError::service(format!(
            "Failed to predownload ACP adapter '{}': {}",
            package, error
        ))),
    }
}

pub(crate) async fn install_npm_cli_package(package: &str) -> NortHingResult<()> {
    let npm_path =
        find_executable("npm").ok_or_else(|| NortHingError::service("npm is not available on PATH".to_string()))?;
    let args = ["install", "-g", package];

    match run_command_with_timeout(npm_path.as_os_str(), args, CLI_INSTALL_TIMEOUT).await {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(NortHingError::service(format!(
            "Failed to install ACP agent CLI '{}': {}",
            package,
            command_error_summary(&output.stderr, &output.stdout)
        ))),
        Err(error) => Err(NortHingError::service(format!(
            "Failed to install ACP agent CLI '{}': {}",
            package, error
        ))),
    }
}

pub(crate) async fn install_remote_npm_cli_package(
    ssh_manager: &SSHConnectionManager,
    connection_id: &str,
    package: &str,
) -> NortHingResult<()> {
    let command = remote_user_shell_command(&format!("npm install -g {}", shell_escape(package)));
    let timeout_ms = u64::try_from(CLI_INSTALL_TIMEOUT.as_millis()).unwrap_or(u64::MAX);
    match ssh_manager
        .execute_command_with_options(
            connection_id,
            &command,
            SSHCommandOptions {
                timeout_ms: Some(timeout_ms),
                cancellation_token: None,
            },
        )
        .await
    {
        Ok(output) if output.exit_code == 0 && !output.timed_out && !output.interrupted => Ok(()),
        Ok(output) if output.timed_out => Err(NortHingError::service(format!(
            "Failed to install remote ACP agent CLI '{}': command timed out",
            package
        ))),
        Ok(output) if output.interrupted => Err(NortHingError::service(format!(
            "Failed to install remote ACP agent CLI '{}': command was cancelled",
            package
        ))),
        Ok(output) => Err(NortHingError::service(format!(
            "Failed to install remote ACP agent CLI '{}': {}",
            package,
            remote_command_error_summary(&output.stderr, &output.stdout)
        ))),
        Err(error) => Err(NortHingError::service(format!(
            "Failed to install remote ACP agent CLI '{}': {}",
            package, error
        ))),
    }
}
