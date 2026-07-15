use super::PrependedPromptReminders;
use super::PromptBuilder;
use northhing_agent_runtime::prompt::UserContextPolicy;

impl PromptBuilder {
    pub fn build_skill_listing_reminder(&self) -> Option<String> {
        self.context.tool_listing_sections.render_skill_listing_reminder()
    }

    pub fn build_agent_listing_reminder(&self) -> Option<String> {
        self.context.tool_listing_sections.render_agent_listing_reminder()
    }

    pub fn build_collapsed_tool_listing_reminder(&self) -> Option<String> {
        self.context
            .tool_listing_sections
            .render_collapsed_tool_listing_reminder()
    }

    pub async fn build_prepended_reminders(&self, user_context_policy: &UserContextPolicy) -> PrependedPromptReminders {
        PrependedPromptReminders {
            collapsed_tool_listing: self.build_collapsed_tool_listing_reminder(),
            skill_listing: self.build_skill_listing_reminder(),
            agent_listing: self.build_agent_listing_reminder(),
            runtime_context: self.build_runtime_context_reminder().await,
            user_context: self.build_user_context_reminder(user_context_policy).await,
        }
    }
}
