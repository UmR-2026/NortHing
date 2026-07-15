//! Unit tests for the `ExecCommandTool` siblings. These mirror the original
//! `command.rs::tests` module exactly: each test exercises one behavior of
//! the split pieces (`argv_for_shell`, `remote_login_shell_command`,
//! `remote_non_tty_control_wrapper`, `parse_remote_shell_probe_output`,
//! `remote_shell_login_args`, `description_with_context`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use terminal_core::ShellType;

use super::super::env_snapshot::RemoteEnvSnapshot;
use super::local::argv_for_shell;
use super::remote::{remote_login_shell_command, remote_non_tty_control_wrapper};
use super::shell_helpers::{parse_remote_shell_probe_output, remote_shell_login_args};
use super::tool::ExecCommandTool;
use super::types::{RemoteShell, POWERSHELL_UTF8_OUTPUT_PREFIX};
use crate::agentic::tools::framework::{Tool, ToolUseContext};
use crate::agentic::tools::ToolRuntimeRestrictions;
use crate::agentic::workspace::WorkspaceBinding;
use crate::service::remote_ssh::workspace_state::workspace_session_identity;

#[test]
fn powershell_commands_force_utf8_output() {
    let argv = argv_for_shell(Path::new("pwsh"), &ShellType::PowerShellCore, "Get-Content README.md");

    assert_eq!(argv[1], "-Command");
    assert!(argv[2].starts_with(POWERSHELL_UTF8_OUTPUT_PREFIX));
    assert!(argv[2].contains("Get-Content README.md"));
}

#[test]
fn powershell_utf8_output_prefix_is_not_duplicated() {
    let script = format!("{POWERSHELL_UTF8_OUTPUT_PREFIX}Write-Output ok");
    let argv = argv_for_shell(Path::new("pwsh"), &ShellType::PowerShellCore, &script);

    assert_eq!(argv[2], script);
}

#[test]
fn remote_login_shell_command_wraps_workdir_env_shell_and_user_command() {
    let shell = RemoteShell {
        path: "/bin/bash".to_string(),
        shell_type: ShellType::Bash,
    };
    let command = remote_login_shell_command("/home/me/project", "printf 'hi'", &shell, None);

    assert!(command.starts_with("cd '/home/me/project' && env "));
    assert!(command.contains("'northhing_NONINTERACTIVE=1'"));
    assert!(command.ends_with(" '/bin/bash' -lc 'printf '\\''hi'\\'''"));
}

#[test]
fn remote_login_shell_command_injects_snapshot_before_tool_env() {
    let shell = RemoteShell {
        path: "/bin/bash".to_string(),
        shell_type: ShellType::Bash,
    };
    let snapshot = RemoteEnvSnapshot {
        env: HashMap::from([
            ("PATH".to_string(), "/home/me/.nvm/bin:/usr/bin".to_string()),
            ("TERM".to_string(), "xterm-256color".to_string()),
        ]),
    };
    let command = remote_login_shell_command("/home/me/project", "node --version", &shell, Some(&snapshot));

    assert!(command.contains("'PATH=/home/me/.nvm/bin:/usr/bin'"));
    assert!(command.contains("'TERM=dumb'"));
    assert!(!command.contains("'TERM=xterm-256color'"));
}

#[test]
fn remote_login_shell_command_uses_snapshot_without_interactive_startup() {
    let shell = RemoteShell {
        path: "/bin/bash".to_string(),
        shell_type: ShellType::Bash,
    };
    let snapshot = RemoteEnvSnapshot {
        env: HashMap::from([("PATH".to_string(), "/home/me/.nvm/bin:/usr/bin".to_string())]),
    };
    let command = remote_login_shell_command("/home/me/project", "node --version", &shell, Some(&snapshot));

    assert!(command.contains("'PATH=/home/me/.nvm/bin:/usr/bin'"));
    assert!(command.ends_with(" '/bin/bash' -lc 'node --version'"));
    assert!(!command.contains(" -lic "));
}

#[test]
fn remote_non_tty_control_wrapper_cleans_process_group_after_interrupt_grace() {
    let wrapper = remote_non_tty_control_wrapper("python3 -c 'print(1)'", "/bin/bash");

    assert!(wrapper.contains("setsid \"$__northhing_shell\" -lc \"$__northhing_cmd\" &"));
    assert!(wrapper.contains("trap '__northhing_stop INT 130 2' INT"));
    assert!(wrapper.contains("trap '__northhing_stop KILL 137 0' TERM"));
    assert!(wrapper.contains("__northhing_grace=${3:-2}"));
    assert!(wrapper.contains("sleep \"$__northhing_grace\""));
    assert!(wrapper.contains("kill -KILL \"-$__northhing_pgid\""));
    assert!(wrapper.contains("__northhing_cmd='python3 -c '\\''print(1)'\\'''"));
}

#[test]
fn remote_shell_probe_prefers_first_plausible_shell_path() {
    let shell = parse_remote_shell_probe_output("\n/bin/zsh\n/usr/bin/bash\n").expect("shell should parse");

    assert_eq!(shell.path, "/bin/zsh");
    assert_eq!(shell.shell_type, ShellType::Zsh);
}

#[test]
fn remote_shell_login_args_use_login_without_interactive_startup() {
    assert_eq!(remote_shell_login_args(), &["-lc"]);
}

#[tokio::test]
async fn description_with_context_stays_stable_for_local_and_remote_workspaces() {
    let tool = ExecCommandTool::new();
    let base = tool.description().await.expect("description should build");
    let session_identity = workspace_session_identity("/home/me/project", Some("conn-1"), Some("remote-host"))
        .expect("remote session identity should build");
    let remote_context = ToolUseContext {
        tool_call_id: None,
        agent_type: Some("agentic".to_string()),
        session_id: None,
        dialog_turn_id: None,
        workspace: Some(WorkspaceBinding::new_remote(
            None,
            PathBuf::from("/home/me/project"),
            "conn-1".to_string(),
            "Remote Host".to_string(),
            session_identity,
        )),
        unlocked_collapsed_tools: Vec::new(),
        custom_data: HashMap::new(),
        computer_use_host: None,
        runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
        runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
        actor_runtime: None,
    };

    assert_eq!(
        base,
        tool.description_with_context(Some(&remote_context))
            .await
            .expect("contextual description should build")
    );
}
