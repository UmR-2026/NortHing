use super::PromptBuilder;
use super::PromptBuilderContext;
use super::RemoteExecutionHints;
use super::RuntimeContextNeeds;
use super::ToolListingSections;
use super::USER_CONTEXT_PROMPT;
use crate::agentic::agents::UserContextPolicy;
use crate::service::workspace::RelatedPath;

#[tokio::test]
async fn builds_ordered_prepended_reminders_from_tool_listings_and_user_context() {
    let tool_sections = ToolListingSections {
        skill_listing: Some("<available_skills>\n- pdf\n</available_skills>".to_string()),
        agent_listing: Some("<available_agents>\n- Explore\n</available_agents>".to_string()),
        collapsed_tool_listing: Some("<collapsed_tools>\n- WebFetch\n</collapsed_tools>".to_string()),
    };
    let context = PromptBuilderContext::new(r"workspace\root", None, None)
        .with_tool_listing_sections(tool_sections)
        .with_runtime_context_needs(RuntimeContextNeeds::from_tool_names(["Read"]));
    let reminders = PromptBuilder::new(context)
        .build_prepended_reminders(
            &UserContextPolicy::empty()
                .with_workspace_context()
                .with_workspace_instructions(),
        )
        .await;
    let reminders_for_order = reminders.clone();
    let ordered_reminders = reminders_for_order.ordered_reminders();

    let skill_listing = reminders.skill_listing.expect("skill listing reminder should build");
    let agent_listing = reminders.agent_listing.expect("agent listing reminder should build");
    let collapsed_tool_listing = reminders
        .collapsed_tool_listing
        .expect("collapsed tool listing reminder should build");
    let user_context = reminders.user_context.expect("user context should build");
    let runtime_context = reminders.runtime_context.expect("runtime context should build");

    assert!(skill_listing.contains("# Skill Listing"));
    assert!(skill_listing.contains("<available_skills>"));
    assert!(!skill_listing.contains("# Agent Listing"));
    assert!(agent_listing.contains("# Agent Listing"));
    assert!(agent_listing.contains("<available_agents>"));
    assert!(!agent_listing.contains("# Collapsed Tool Listing"));
    assert!(collapsed_tool_listing.contains("# Collapsed Tool Listing"));
    assert!(collapsed_tool_listing.contains("<collapsed_tools>"));
    assert!(user_context.contains("# User Context"));
    assert!(user_context.contains(USER_CONTEXT_PROMPT));
    assert!(user_context.contains("Current Working Directory: workspace/root"));
    assert!(runtime_context.contains("# Runtime Context"));
    assert!(runtime_context.contains("## Workspace Execution"));
    assert!(runtime_context.contains("Workspace file and shell tools operate on the local filesystem"));
    assert!(!runtime_context.contains("## ExecCommand Shell"));
    assert!(!runtime_context.contains("## Local Client"));
    assert!(!runtime_context.contains("ExecCommand shell:"));
    assert_eq!(
        ordered_reminders,
        vec![
            collapsed_tool_listing.as_str(),
            skill_listing.as_str(),
            agent_listing.as_str(),
            runtime_context.as_str(),
            user_context.as_str(),
        ]
    );
}

#[tokio::test]
async fn prepended_reminders_omit_runtime_context_without_runtime_tool_needs() {
    let context = PromptBuilderContext::new(r"workspace\root", None, None);
    let reminders = PromptBuilder::new(context)
        .build_prepended_reminders(&UserContextPolicy::empty())
        .await;

    assert_eq!(reminders.skill_listing, None);
    assert_eq!(reminders.agent_listing, None);
    assert_eq!(reminders.collapsed_tool_listing, None);
    assert_eq!(reminders.user_context, None);
    assert_eq!(reminders.runtime_context, None);
}

#[tokio::test]
async fn runtime_context_includes_workspace_info_for_workspace_tools() {
    let context = PromptBuilderContext::new(r"workspace\root", None, None)
        .with_runtime_context_needs(RuntimeContextNeeds::from_tool_names(["Read"]));
    let runtime_context = PromptBuilder::new(context)
        .build_runtime_context_reminder()
        .await
        .expect("runtime context should build");

    assert!(runtime_context.contains("# Runtime Context"));
    assert!(runtime_context.contains("## Workspace Execution"));
    assert!(runtime_context.contains("Workspace file and shell tools operate on the local filesystem"));
    assert!(!runtime_context.contains("## ExecCommand Shell"));
    assert!(!runtime_context.contains("## Local Client"));
    assert!(!runtime_context.contains("ExecCommand shell:"));
}

#[tokio::test]
async fn runtime_context_includes_shell_info_when_exec_command_is_available() {
    let context = PromptBuilderContext::new(r"workspace\root", None, None)
        .with_runtime_context_needs(RuntimeContextNeeds::from_tool_names(["ExecCommand"]));
    let runtime_context = PromptBuilder::new(context)
        .build_runtime_context_reminder()
        .await
        .expect("runtime context should build");

    assert!(runtime_context.contains("# Runtime Context"));
    assert!(runtime_context.contains("## Workspace Execution"));
    assert!(runtime_context.contains("## ExecCommand Shell"));
    assert!(runtime_context.contains("ExecCommand shell:"));
    assert!(runtime_context.contains("invoked as `"));
    assert!(!runtime_context.contains("## Local Client"));
}

#[test]
fn local_exec_shell_runtime_guidance_is_added_for_powershell_shells() {
    let guidance = PromptBuilder::local_exec_shell_runtime_guidance("powershell");

    assert_eq!(
            guidance,
            &[
                "- For inline Python or other embedded scripts, prefer PowerShell-friendly forms such as `@'\\nprint(\"Hello\")\\n'@ | python -` instead of heavily nested quoting.",
                "- In PowerShell, the escape character is the backtick (`), not backslash. `\\\"` is not a reliable way to escape a double quote for the shell.",
                "- For environment variables, process filtering, and file traversal, prefer native PowerShell cmdlets and syntax over shell-specific Unix patterns.",
                "- Avoid mixing PowerShell with `cmd.exe` or bash in the same command unless cross-shell behavior is explicitly required.",
            ]
        );
}

#[test]
fn local_exec_shell_runtime_guidance_is_empty_for_non_powershell_shells() {
    assert!(PromptBuilder::local_exec_shell_runtime_guidance("bash").is_empty());
}

#[test]
fn exec_control_runtime_guidance_is_added_for_local_windows() {
    let guidance = PromptBuilder::exec_control_runtime_guidance("windows", false, true);

    assert_eq!(
            guidance,
            vec![
                "- On local Windows ExecCommand sessions, `ExecControl` `interrupt` is effectively the same as `kill` for non-TTY processes.".to_string()
            ]
        );
}

#[test]
fn exec_control_runtime_guidance_is_empty_when_exec_control_is_unavailable() {
    assert!(PromptBuilder::exec_control_runtime_guidance("windows", false, false).is_empty());
}

#[test]
fn exec_control_runtime_guidance_is_empty_for_remote_or_non_windows_hosts() {
    assert!(PromptBuilder::exec_control_runtime_guidance("linux", false, true).is_empty());
    assert!(PromptBuilder::exec_control_runtime_guidance("windows", true, true).is_empty());
}

#[tokio::test]
async fn runtime_context_includes_computer_use_info_only_when_needed() {
    let context = PromptBuilderContext::new(r"workspace\root", None, None)
        .with_runtime_context_needs(RuntimeContextNeeds::from_tool_names(["ComputerUse"]));
    let runtime_context = PromptBuilder::new(context)
        .build_runtime_context_reminder()
        .await
        .expect("runtime context should build");

    assert!(runtime_context.contains("## Local Client"));
    assert!(runtime_context.contains("Local northhing client OS:"));
    assert!(runtime_context.contains("Computer use / `key_chord`"));
    assert!(!runtime_context.contains("## Workspace Execution"));
    assert!(!runtime_context.contains("## ExecCommand Shell"));
    assert!(!runtime_context.contains("ExecCommand shell:"));
}

#[tokio::test]
async fn runtime_context_omits_workspace_root_for_remote_execution() {
    let context = PromptBuilderContext::new("/workspace/project", None, None)
        .with_runtime_context_needs(RuntimeContextNeeds::from_tool_names([
            "Read",
            "ExecCommand",
            "ComputerUse",
        ]))
        .with_remote_prompt_overlay(
            RemoteExecutionHints {
                connection_display_name: "dev-server".to_string(),
                kernel_name: "Linux".to_string(),
                hostname: "devbox".to_string(),
            },
            None,
        );
    let runtime_context = PromptBuilder::new(context)
        .build_runtime_context_reminder()
        .await
        .expect("runtime context should build");

    assert!(runtime_context.contains("Workspace file and shell tools operate on remote SSH connection"));
    assert!(runtime_context.contains("## Workspace Execution"));
    assert!(runtime_context.contains("## ExecCommand Shell"));
    assert!(runtime_context.contains("## Local Client"));
    assert!(runtime_context.contains("Local northhing client OS:"));
    assert!(runtime_context.contains("Computer use and UI automation operate on the local northhing desktop, even when workspace file and shell tools target a remote host."));
    assert!(runtime_context.contains("ExecCommand uses the remote user's default POSIX shell"));
}

#[tokio::test]
async fn deep_research_report_link_defaults_to_workspace_relative_path() {
    let context = PromptBuilderContext::new("workspace/root", Some("session-1".to_string()), None);
    let prompt = PromptBuilder::new(context)
        .build_prompt_from_template("[View full report]({DEEP_RESEARCH_REPORT_LINK})")
        .await
        .expect("prompt should build");

    assert_eq!(
        prompt,
        "[View full report](.northhing/sessions/session-1/research/report.md)"
    );
}

#[tokio::test]
async fn deep_research_report_link_uses_computer_scheme_for_remote_delivery() {
    let context = PromptBuilderContext::new("workspace/root", Some("session-1".to_string()), None)
        .with_remote_file_delivery_channel(true);
    let prompt = PromptBuilder::new(context)
        .build_prompt_from_template("[View full report]({DEEP_RESEARCH_REPORT_LINK})")
        .await
        .expect("prompt should build");

    assert_eq!(
        prompt,
        "[View full report](computer://.northhing/sessions/session-1/research/report.md)"
    );
}

#[test]
fn workspace_context_renders_related_directories() {
    let context = PromptBuilderContext::new(r"workspace\root", None, None).with_related_paths(vec![
        RelatedPath {
            path: r"legacy-ts\client".to_string(),
            description: Some("Legacy TypeScript implementation".to_string()),
        },
        RelatedPath {
            path: r"monorepo\billing".to_string(),
            description: Some("Billing package".to_string()),
        },
    ]);

    let workspace_context = PromptBuilder::new(context).workspace_context();

    assert!(workspace_context.contains("Related directories"));
    assert!(workspace_context.contains("legacy-ts/client"));
    assert!(workspace_context.contains("Legacy TypeScript implementation"));
    assert!(workspace_context.contains("monorepo/billing"));
}

#[test]
fn workspace_context_renders_related_directories_without_description() {
    let context = PromptBuilderContext::new(r"workspace\root", None, None).with_related_paths(vec![RelatedPath {
        path: r"monorepo\packages\payments".to_string(),
        description: None,
    }]);

    let workspace_context = PromptBuilder::new(context).workspace_context();

    assert!(workspace_context.contains("Related directories"));
    assert!(workspace_context.contains("  - monorepo/packages/payments"));
    assert!(!workspace_context.contains("payments —"));
}
