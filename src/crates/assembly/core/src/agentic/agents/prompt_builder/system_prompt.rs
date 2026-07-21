use super::{
    PromptBuilder, PromptBuilderContext, PLACEHOLDER_AGENT_MEMORY, PLACEHOLDER_CLAW_WORKSPACE,
    PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK, PLACEHOLDER_LANGUAGE_PREFERENCE, PLACEHOLDER_PERSONA,
    PLACEHOLDER_SESSION_ID, PLACEHOLDER_VISUAL_MODE,
};
use crate::agentic::remote_file_delivery::user_workspace_relative_file_link;
use crate::service::agent_memory::build_workspace_agent_memory_prompt;
use crate::service::bootstrap::build_workspace_persona_prompt;
use crate::util::errors::NortHingResult;
use std::path::Path;
use tracing::warn;

impl PromptBuilder {
    pub async fn build_prompt_from_template(&self, template: &str) -> NortHingResult<String> {
        let mut result = template.to_string();

        // Replace {PERSONA}
        if result.contains(PLACEHOLDER_PERSONA) {
            let persona = if self.context.remote_execution.is_some() {
                "# Workspace persona\nMarkdown persona files (e.g. BOOTSTRAP.md, SOUL.md) live on the **remote** workspace. Use Read or Glob under the workspace root above to load them.\n\n"
                    .to_string()
            } else {
                let workspace = Path::new(&self.context.workspace_path);
                match build_workspace_persona_prompt(workspace).await {
                    Ok(prompt) => prompt.unwrap_or_default(),
                    Err(e) => {
                        warn!(
                            "Failed to build workspace persona prompt: path={} error={}",
                            workspace.display(),
                            e
                        );
                        String::new()
                    }
                }
            };
            result = result.replace(PLACEHOLDER_PERSONA, &persona);
        }

        // Replace {LANGUAGE_PREFERENCE}
        if result.contains(PLACEHOLDER_LANGUAGE_PREFERENCE) {
            let language_preference = self.get_language_preference().await?;
            result = result.replace(PLACEHOLDER_LANGUAGE_PREFERENCE, &language_preference);
        }

        // Replace {CLAW_WORKSPACE}
        if result.contains(PLACEHOLDER_CLAW_WORKSPACE) {
            let claw_workspace = self.get_claw_workspace_instruction();
            result = result.replace(PLACEHOLDER_CLAW_WORKSPACE, &claw_workspace);
        }

        // Replace {AGENT_MEMORY}
        if result.contains(PLACEHOLDER_AGENT_MEMORY) {
            let agent_memory = if self.context.remote_execution.is_some() {
                "# Agent memory\nSession memory under `.northhing/` is stored on the **remote** host for this workspace. Use file tools with POSIX paths under the workspace root if you need to read it.\n\n"
                    .to_string()
            } else {
                let workspace = Path::new(&self.context.workspace_path);
                match build_workspace_agent_memory_prompt(workspace).await {
                    Ok(prompt) => prompt,
                    Err(e) => {
                        warn!(
                            "Failed to build workspace agent memory prompt: path={} error={}",
                            workspace.display(),
                            e
                        );
                        String::new()
                    }
                }
            };
            result = result.replace(PLACEHOLDER_AGENT_MEMORY, &agent_memory);
        }

        // Replace {VISUAL_MODE}
        if result.contains(PLACEHOLDER_VISUAL_MODE) {
            let visual_mode = self.get_visual_mode_instruction().await;
            result = result.replace(PLACEHOLDER_VISUAL_MODE, &visual_mode);
        }

        // Replace {SESSION_ID} — used by deep-research Pro mode to anchor a per-session
        // work_dir under .northhing/sessions/{SESSION_ID}/research/. Falls back to a
        // timestamp slug when no session is bound (e.g. one-shot prompt builds in tests).
        let mut resolved_session_id: Option<String> = None;
        if result.contains(PLACEHOLDER_SESSION_ID) || result.contains(PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK) {
            let session_id = self
                .context
                .session_id
                .clone()
                .unwrap_or_else(|| format!("unbound-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")));
            resolved_session_id = Some(session_id.clone());
            result = result.replace(PLACEHOLDER_SESSION_ID, &session_id);
        }

        if result.contains(PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK) {
            let session_id = resolved_session_id.unwrap_or_else(|| {
                self.context
                    .session_id
                    .clone()
                    .unwrap_or_else(|| format!("unbound-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")))
            });
            let report_link = user_workspace_relative_file_link(
                &format!(".northhing/sessions/{session_id}/research/report.md"),
                self.context.remote_file_delivery_channel,
            );
            result = result.replace(PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK, &report_link);
        }

        if self.context.supports_image_understanding == Some(false) {
            result.push_str(
                "\n\n# Computer use (text-only primary model)\n\n\
The configured **primary model does not accept image inputs**. When using **`ComputerUse`** (or **`ControlHub`** with **`domain: \"browser\"`**):\n\
- **Do not** use **`screenshot`** (desktop) and **avoid** `domain:\"browser\" action:\"screenshot\"` — the JPEG bytes will be unreadable.\n\
- **ACTION PRIORITY:** 1) Terminal/CLI/system commands (`ExecCommand`, or `ComputerUse` `run_script`; use `WriteStdin`/`ExecControl` for running ExecCommand sessions) 2) Keyboard shortcuts (**`key_chord`**, **`type_text`**) 3) UI control: **`click_element`** (AX) → **`locate`** → **`move_to_text`** (use **`move_to_text_match_index`** when multiple OCR hits listed) → **`mouse_move`** (**`use_screen_coordinates`: true** with coordinates from tool JSON) → **`click`**. For browser work prefer `snapshot` → click by `@e*` ref over screenshots.\n\
- **Never guess coordinates** — always use precise methods (AX, OCR, system coordinates from tool results, or browser snapshot refs).\n",
            );
        }

        // Inject runtime model info (agent-prompt layer only).
        if self.context.model_name.is_some() {
            let mut runtime_lines = vec![format!("# Runtime\n- Current model: {}", self.context.model_name.as_ref().unwrap())];
            if let Some(ctx_window) = self.context.context_window {
                runtime_lines.push(format!("- Context window: {} tokens", ctx_window));
            }
            if let Some(max_out) = self.context.max_output_tokens {
                runtime_lines.push(format!("- Max output: {} tokens", max_out));
            }
            runtime_lines.push("Use the context window as your budget: prefer targeted reads over whole-file dumps for large files, and summarize rather than repeat long content.".to_string());
            result.push_str(&format!("\n\n{}", runtime_lines.join("\n")));
        }

        Ok(result.trim().to_string())
    }

    /// Build Layer 2: Agent prompt (template + persona + language + memory + claw + visual mode).
    ///
    /// This layer changes rarely — only when workspace persona/memory files change.
    /// Cached per (template_name, workspace_path, session_id).
    pub async fn build_agent_prompt_layer(&self, template: &str) -> NortHingResult<String> {
        let mut result = template.to_string();

        // Replace {PERSONA}
        if result.contains(PLACEHOLDER_PERSONA) {
            let persona = if self.context.remote_execution.is_some() {
                "# Workspace persona\nMarkdown persona files (e.g. BOOTSTRAP.md, SOUL.md) live on the **remote** workspace. Use Read or Glob under the workspace root above to load them.\n\n"
                    .to_string()
            } else {
                let workspace = Path::new(&self.context.workspace_path);
                match build_workspace_persona_prompt(workspace).await {
                    Ok(prompt) => prompt.unwrap_or_default(),
                    Err(e) => {
                        warn!(
                            "Failed to build workspace persona prompt: path={} error={}",
                            workspace.display(),
                            e
                        );
                        String::new()
                    }
                }
            };
            result = result.replace(PLACEHOLDER_PERSONA, &persona);
        }

        // Replace {LANGUAGE_PREFERENCE}
        if result.contains(PLACEHOLDER_LANGUAGE_PREFERENCE) {
            let language_preference = self.get_language_preference().await?;
            result = result.replace(PLACEHOLDER_LANGUAGE_PREFERENCE, &language_preference);
        }

        // Replace {CLAW_WORKSPACE}
        if result.contains(PLACEHOLDER_CLAW_WORKSPACE) {
            let claw_workspace = self.get_claw_workspace_instruction();
            result = result.replace(PLACEHOLDER_CLAW_WORKSPACE, &claw_workspace);
        }

        // Replace {AGENT_MEMORY}
        if result.contains(PLACEHOLDER_AGENT_MEMORY) {
            let agent_memory = if self.context.remote_execution.is_some() {
                "# Agent memory\nSession memory under `.northhing/` is stored on the **remote** host for this workspace. Use file tools with POSIX paths under the workspace root if you need to read it.\n\n"
                    .to_string()
            } else {
                let workspace = Path::new(&self.context.workspace_path);
                match build_workspace_agent_memory_prompt(workspace).await {
                    Ok(prompt) => prompt,
                    Err(e) => {
                        warn!(
                            "Failed to build workspace agent memory prompt: path={} error={}",
                            workspace.display(),
                            e
                        );
                        String::new()
                    }
                }
            };
            result = result.replace(PLACEHOLDER_AGENT_MEMORY, &agent_memory);
        }

        // Replace {VISUAL_MODE}
        if result.contains(PLACEHOLDER_VISUAL_MODE) {
            let visual_mode = self.get_visual_mode_instruction().await;
            result = result.replace(PLACEHOLDER_VISUAL_MODE, &visual_mode);
        }

        // Inject runtime model info (agent-prompt layer only).
        if self.context.model_name.is_some() {
            let mut runtime_lines = vec![format!("# Runtime\n- Current model: {}", self.context.model_name.as_ref().unwrap())];
            if let Some(ctx_window) = self.context.context_window {
                runtime_lines.push(format!("- Context window: {} tokens", ctx_window));
            }
            if let Some(max_out) = self.context.max_output_tokens {
                runtime_lines.push(format!("- Max output: {} tokens", max_out));
            }
            runtime_lines.push("Use the context window as your budget: prefer targeted reads over whole-file dumps for large files, and summarize rather than repeat long content.".to_string());
            result.push_str(&format!("\n\n{}", runtime_lines.join("\n")));
        }

        Ok(result)
    }

    /// Build Layer 3: System prompt (agent_prompt + session_id + report_link + visual mode append).
    ///
    /// This layer changes per turn but is still cacheable when tool definitions don't change.
    pub async fn build_system_prompt_layer(
        &self,
        agent_prompt: &str,
        _tool_defs: Option<&str>,
    ) -> NortHingResult<String> {
        let mut result = agent_prompt.to_string();

        // Replace {SESSION_ID}
        let mut resolved_session_id: Option<String> = None;
        if result.contains(PLACEHOLDER_SESSION_ID) || result.contains(PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK) {
            let session_id = self
                .context
                .session_id
                .clone()
                .unwrap_or_else(|| format!("unbound-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")));
            resolved_session_id = Some(session_id.clone());
            result = result.replace(PLACEHOLDER_SESSION_ID, &session_id);
        }

        if result.contains(PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK) {
            let session_id = resolved_session_id.unwrap_or_else(|| {
                self.context
                    .session_id
                    .clone()
                    .unwrap_or_else(|| format!("unbound-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")))
            });
            let report_link = user_workspace_relative_file_link(
                &format!(".northhing/sessions/{session_id}/research/report.md"),
                self.context.remote_file_delivery_channel,
            );
            result = result.replace(PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK, &report_link);
        }

        if self.context.supports_image_understanding == Some(false) {
            result.push_str(
                "\n\n# Computer use (text-only primary model)\n\n\
The configured **primary model does not accept image inputs**. When using **`ComputerUse`** (or **`ControlHub`** with **`domain: \"browser\"`**):\n\
- **Do not** use **`screenshot`** (desktop) and **avoid** `domain:\"browser\" action:\"screenshot\"` — the JPEG bytes will be unreadable.\n\
- **ACTION PRIORITY:** 1) Terminal/CLI/system commands (`ExecCommand`, or `ComputerUse` `run_script`; use `WriteStdin`/`ExecControl` for running ExecCommand sessions) 2) Keyboard shortcuts (**`key_chord`**, **`type_text`**) 3) UI control: **`click_element`** (AX) → **`locate`** → **`move_to_text`** (use **`move_to_text_match_index`** when multiple OCR hits listed) → **`mouse_move`** (**`use_screen_coordinates`: true** with coordinates from tool JSON) → **`click`**. For browser work prefer `snapshot` → click by `@e*` ref over screenshots.\n\
- **Never guess coordinates** — always use precise methods (AX, OCR, system coordinates from tool results, or browser snapshot refs).\n",
            );
        }

        Ok(result.trim().to_string())
    }
}
