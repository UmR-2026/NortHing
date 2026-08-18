//! Product tool materialization owner.

use crate::agentic::tools::framework::Tool;
use crate::agentic::tools::implementations::*;
use crate::agentic::tools::registry::ProductToolDecoratorRef;
use northhing_agent_tools::{
    StaticToolProviderFactory, StaticToolProviderPlan, ToolRegistry as AgentToolRegistry, ToolRuntimeAssembly,
};
use std::sync::Arc;

pub(in crate::agentic::tools) const PRODUCT_TOOL_GROUPS: &[(&str, &[&str])] = &[
    (
        "core.basic",
        &[
            "LS",
            "Read",
            "Glob",
            "Grep",
            "Write",
            "Edit",
            "Delete",
            "ExecCommand",
            "WriteStdin",
            "ExecControl",
            "GetTime",
        ],
    ),
    (
        "core.agent",
        &[
            "Task",
            "Skill",
            "AskUserQuestion",
            "TodoWrite",
            "get_goal",
            "create_goal",
            "update_goal",
            "CreatePlan",
            "submit_code_review",
            "GetToolSpec",
            "GetFileDiff",
            "Log",
        ],
    ),
    (
        "core.session",
        &["SessionControl", "SessionMessage", "SessionHistory", "Cron"],
    ),
    (
        "core.integration",
        &[
            "WebSearch",
            "WebFetch",
            "ListMCPResources",
            "ReadMCPResource",
            "ListMCPPrompts",
            "GetMCPPrompt",
            "GenerativeUI",
            "Git",
            "ReviewPlatform",
            "InitMiniApp",
            "ControlHub",
            "ComputerUse",
            "Playbook",
        ],
    ),
];

#[derive(Debug, Clone, Copy, Default)]
pub(in crate::agentic::tools) struct ProductConcreteToolFactory;

impl StaticToolProviderFactory<dyn Tool> for ProductConcreteToolFactory {
    fn materialize_tool(&self, tool_name: &str) -> Option<Arc<dyn Tool>> {
        match tool_name {
            "LS" => Some(Arc::new(LSTool::new())),
            "Read" => Some(Arc::new(FileReadTool::new())),
            "Glob" => Some(Arc::new(GlobTool::new())),
            "Grep" => Some(Arc::new(GrepTool::new())),
            "Write" => Some(Arc::new(FileWriteTool::new())),
            "Edit" => Some(Arc::new(FileEditTool::new())),
            "Delete" => Some(Arc::new(DeleteFileTool::new())),
            "ExecCommand" => Some(Arc::new(ExecCommandTool::new())),
            "WriteStdin" => Some(Arc::new(WriteStdinTool::new())),
            "ExecControl" => Some(Arc::new(ExecControlTool::new())),
            "GetTime" => Some(Arc::new(GetTimeTool::new())),
            "Task" => Some(Arc::new(TaskTool::new())),
            "Skill" => Some(Arc::new(SkillTool::new())),
            "AskUserQuestion" => Some(Arc::new(AskUserQuestionTool::new())),
            "TodoWrite" => Some(Arc::new(TodoWriteTool::new())),
            "get_goal" => Some(Arc::new(GetGoalTool::new())),
            "create_goal" => Some(Arc::new(CreateGoalTool::new())),
            "update_goal" => Some(Arc::new(UpdateGoalTool::new())),
            "CreatePlan" => Some(Arc::new(CreatePlanTool::new())),
            "submit_code_review" => Some(Arc::new(CodeReviewTool::new())),
            "GetToolSpec" => Some(Arc::new(GetToolSpecTool::new())),
            "GetFileDiff" => Some(Arc::new(GetFileDiffTool::new())),
            "Log" => Some(Arc::new(LogTool::new())),
            "SessionControl" => Some(Arc::new(SessionControlTool::new())),
            "SessionMessage" => Some(Arc::new(SessionMessageTool::new())),
            "SessionHistory" => Some(Arc::new(SessionHistoryTool::new())),
            "Cron" => Some(Arc::new(CronTool::new())),
            "WebSearch" => Some(Arc::new(WebSearchTool::new())),
            "WebFetch" => Some(Arc::new(WebFetchTool::new())),
            "ListMCPResources" => Some(Arc::new(ListMCPResourcesTool::new())),
            "ReadMCPResource" => Some(Arc::new(ReadMCPResourceTool::new())),
            "ListMCPPrompts" => Some(Arc::new(ListMCPPromptsTool::new())),
            "GetMCPPrompt" => Some(Arc::new(GetMCPPromptTool::new())),
            "GenerativeUI" => Some(Arc::new(GenerativeUITool::new())),
            "Git" => Some(Arc::new(GitTool::new())),
            "ReviewPlatform" => Some(Arc::new(ReviewPlatformTool::new())),
            "InitMiniApp" => Some(Arc::new(InitMiniAppTool::new())),
            "ControlHub" => Some(Arc::new(ControlHubTool::new())),
            "ComputerUse" => Some(Arc::new(ComputerUseTool::new())),
            "Playbook" => Some(Arc::new(PlaybookTool::new())),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ProductToolProviderPlanAdapter {
    provider_id: &'static str,
    tool_names: &'static [&'static str],
}

impl StaticToolProviderPlan for ProductToolProviderPlanAdapter {
    fn provider_id(&self) -> &'static str {
        self.provider_id
    }

    fn tool_names(&self) -> &'static [&'static str] {
        self.tool_names
    }
}

pub(in crate::agentic::tools) fn create_product_tool_registry_from_plan(
    tool_decorator: ProductToolDecoratorRef,
) -> AgentToolRegistry<dyn Tool> {
    let adapters = PRODUCT_TOOL_GROUPS
        .iter()
        .copied()
        .map(|(provider_id, tool_names)| ProductToolProviderPlanAdapter {
            provider_id,
            tool_names,
        })
        .collect::<Vec<_>>();

    ToolRuntimeAssembly::with_tool_decorator(tool_decorator)
        .create_registry_from_static_provider_plans(&adapters, &ProductConcreteToolFactory)
        .expect("product capability tool provider plan must reference concrete core tools")
}
