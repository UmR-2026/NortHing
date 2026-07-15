use super::super::super::coordinator::DialogTriggerSource;
use super::super::scheduler_types::DialogScheduler;
use crate::agentic::core::Message;
use crate::agentic::goal_mode::{goal_internal_context_message, goal_objective_updated_message};
use northhing_agent_runtime::scheduler::{
    build_thread_goal_objective_updated_delivery_plan, build_thread_goal_resumed_delivery_plan,
    resolve_background_delivery_action, resolve_background_delivery_injection, BackgroundDeliveryAction,
    BackgroundDeliveryFacts, BackgroundInjectionKind, ThreadGoalDeliveryReminder, ThreadGoalDeliveryReminderKind,
};
use northhing_runtime_ports::{
    AgentBackgroundResultRequest, AgentLifecycleDeliveryPort, AgentThreadGoalDeliveryKind,
    AgentThreadGoalDeliveryRequest, PortError, PortErrorKind, PortResult, ThreadGoal,
};
use std::time::SystemTime;
use uuid::Uuid;

impl DialogScheduler {
    /// Resume auto-continuation toward an active thread goal (after pause / blocked / usage limit).
    pub async fn deliver_thread_goal_resumed(
        &self,
        session_id: String,
        agent_type: String,
        workspace_path: Option<String>,
        goal: ThreadGoal,
    ) -> Result<(), String> {
        let plan = build_thread_goal_resumed_delivery_plan(&goal);
        let state = self.session_manager.get_session(&session_id).map(|s| s.state.clone());

        match resolve_background_delivery_action(BackgroundDeliveryFacts {
            session_state: Self::session_state_fact(state.as_ref()),
        }) {
            BackgroundDeliveryAction::InjectIntoRunningTurn => {
                self.round_injection_buffer.push(
                    &session_id,
                    resolve_background_delivery_injection(
                        BackgroundInjectionKind::ThreadGoalObjectiveUpdated,
                        Uuid::new_v4().to_string(),
                        plan.injection_prompt,
                        Some(plan.injection_display),
                        SystemTime::now(),
                    ),
                );
                Ok(())
            }
            BackgroundDeliveryAction::SubmitAgentSessionFollowUp {
                queue_priority,
                skip_tool_confirmation,
            } => {
                let prepended = thread_goal_delivery_messages(plan.prepended_reminders);
                self.submit_with_prepended_messages(
                    session_id,
                    plan.follow_up_user_input,
                    plan.follow_up_original_user_input,
                    None,
                    agent_type,
                    workspace_path,
                    super::super::scheduler_types::DialogSubmissionPolicy::new(
                        DialogTriggerSource::AgentSession,
                        queue_priority,
                        skip_tool_confirmation,
                    ),
                    None,
                    Some(plan.user_message_metadata),
                    prepended,
                    None,
                )
                .await
                .map(|_| ())
            }
        }
    }

    /// Inject objective-updated steering into the running turn, or start a follow-up turn when idle.
    pub async fn deliver_thread_goal_objective_updated(
        &self,
        session_id: String,
        agent_type: String,
        workspace_path: Option<String>,
        goal: ThreadGoal,
    ) -> Result<(), String> {
        let plan = build_thread_goal_objective_updated_delivery_plan(&goal);
        let state = self.session_manager.get_session(&session_id).map(|s| s.state.clone());

        match resolve_background_delivery_action(BackgroundDeliveryFacts {
            session_state: Self::session_state_fact(state.as_ref()),
        }) {
            BackgroundDeliveryAction::InjectIntoRunningTurn => {
                self.round_injection_buffer.push(
                    &session_id,
                    resolve_background_delivery_injection(
                        BackgroundInjectionKind::ThreadGoalObjectiveUpdated,
                        Uuid::new_v4().to_string(),
                        plan.injection_prompt,
                        Some(plan.injection_display),
                        SystemTime::now(),
                    ),
                );
                Ok(())
            }
            BackgroundDeliveryAction::SubmitAgentSessionFollowUp {
                queue_priority,
                skip_tool_confirmation,
            } => {
                let prepended = thread_goal_delivery_messages(plan.prepended_reminders);
                self.submit_with_prepended_messages(
                    session_id,
                    plan.follow_up_user_input,
                    plan.follow_up_original_user_input,
                    None,
                    agent_type,
                    workspace_path,
                    super::super::scheduler_types::DialogSubmissionPolicy::new(
                        DialogTriggerSource::AgentSession,
                        queue_priority,
                        skip_tool_confirmation,
                    ),
                    None,
                    Some(plan.user_message_metadata),
                    prepended,
                    None,
                )
                .await
                .map(|_| ())
            }
        }
    }
}

fn thread_goal_delivery_messages(reminders: Vec<ThreadGoalDeliveryReminder>) -> Vec<Message> {
    reminders
        .into_iter()
        .map(|reminder| match reminder.kind {
            ThreadGoalDeliveryReminderKind::GoalContinuation => goal_internal_context_message(reminder.content),
            ThreadGoalDeliveryReminderKind::GoalObjectiveUpdated => goal_objective_updated_message(reminder.content),
        })
        .collect()
}

#[async_trait::async_trait]
impl AgentLifecycleDeliveryPort for DialogScheduler {
    async fn deliver_background_result(&self, request: AgentBackgroundResultRequest) -> PortResult<()> {
        let metadata = if request.metadata.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(request.metadata))
        };

        DialogScheduler::deliver_background_result(
            self,
            request.session_id,
            request.agent_type,
            request.workspace_path,
            request.content,
            request.display_content,
            metadata,
        )
        .await
        .map_err(|error| PortError::new(PortErrorKind::Backend, error))
    }

    async fn deliver_thread_goal(&self, request: AgentThreadGoalDeliveryRequest) -> PortResult<()> {
        let result = match request.kind {
            AgentThreadGoalDeliveryKind::Resumed => {
                DialogScheduler::deliver_thread_goal_resumed(
                    self,
                    request.session_id,
                    request.agent_type,
                    request.workspace_path,
                    request.goal,
                )
                .await
            }
            AgentThreadGoalDeliveryKind::ObjectiveUpdated => {
                DialogScheduler::deliver_thread_goal_objective_updated(
                    self,
                    request.session_id,
                    request.agent_type,
                    request.workspace_path,
                    request.goal,
                )
                .await
            }
        };

        result.map_err(|error| PortError::new(PortErrorKind::Backend, error))
    }
}
