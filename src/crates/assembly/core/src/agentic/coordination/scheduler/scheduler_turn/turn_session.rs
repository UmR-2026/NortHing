use super::super::scheduler_types::DialogScheduler;
use crate::agentic::core::{InternalReminderKind, Message};
use northhing_agent_runtime::scheduler::AgentSessionReplyPlan;
use tracing::warn;

impl DialogScheduler {
    pub(crate) async fn forward_agent_session_reply(&self, responder_session_id: &str, plan: AgentSessionReplyPlan) {
        let reply_user_input = plan.user_input;
        let target_session_id = plan.target_session_id;
        let target_workspace_path = plan.target_workspace_path;
        let prepended_messages = vec![Message::internal_reminder(
            InternalReminderKind::SessionMessageReply,
            plan.reminder_text,
        )];

        if let Err(error) = self
            .submit_with_prepended_messages(
                target_session_id.clone(),
                reply_user_input.clone(),
                Some(reply_user_input),
                None,
                String::new(),
                Some(target_workspace_path),
                super::super::scheduler_types::DialogSubmissionPolicy::for_source(
                    super::super::super::coordinator::DialogTriggerSource::AgentSession,
                ),
                None,
                None,
                prepended_messages,
                None,
            )
            .await
        {
            warn!(
                "Failed to forward agent-session reply: responder_session_id={}, source_session_id={}, error={}",
                responder_session_id, target_session_id, error
            );
        }
    }
}
