//! CronService – private implementation methods (run loop, job processing, persistence).

use super::schedule::compute_next_run_after_ms;
use super::service::CronService;
use super::service_helpers::{
    cron_enqueue_error_is_missing_session, format_scheduled_job_user_input, next_wakeup_for_job,
    submit_target_session_id, EnqueueInput,
};
use super::types::{CronJob, CronJobTarget, CronJobTargetKind, DEFAULT_RETRY_DELAY_MS};
use crate::agentic::core::SessionConfig;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_agent_runtime::scheduled_job::ScheduledJobEnqueueFailureAction;
use northhing_runtime_ports::AgentDialogTurnRequest;
use std::collections::HashMap;
use tokio::time::Duration;
use tracing::{debug, info, warn};

impl CronService {
    pub(super) async fn handle_turn_state_change<F>(&self, turn_id: &str, update: F) -> NortHingResult<()>
    where
        F: FnOnce(&mut CronJob, i64),
    {
        let _guard = self.mutation_lock.lock().await;
        let mut jobs = self.jobs.write().await;
        let Some(job) = jobs
            .values_mut()
            .find(|job| job.state.active_turn_id.as_deref() == Some(turn_id))
        else {
            return Ok(());
        };

        update(job, super::service_helpers::now_ms());
        self.persist_jobs_locked(&jobs).await?;
        drop(jobs);
        self.wakeup.notify_one();
        Ok(())
    }

    pub(super) async fn run_loop(self: std::sync::Arc<Self>) {
        info!("Cron service loop started");

        loop {
            match self.next_wakeup_at().await {
                Some(next_wakeup_ms) => {
                    let current_ms = super::service_helpers::now_ms();
                    if next_wakeup_ms > current_ms {
                        let sleep_ms = (next_wakeup_ms - current_ms) as u64;
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_millis(sleep_ms)) => {}
                            _ = self.wakeup.notified() => {
                                continue;
                            }
                        }
                    }
                }
                None => {
                    self.wakeup.notified().await;
                    continue;
                }
            }

            if let Err(error) = self.process_due_jobs().await {
                warn!("Failed to process due scheduled jobs: {}", error);
                tokio::time::sleep(Duration::from_millis(1_000)).await;
            }
        }
    }

    pub(super) async fn next_wakeup_at(&self) -> Option<i64> {
        let jobs = self.jobs.read().await;
        jobs.values().filter_map(next_wakeup_for_job).min()
    }

    pub(super) async fn process_due_jobs(&self) -> NortHingResult<()> {
        let current_ms = super::service_helpers::now_ms();
        let due_job_ids = {
            let jobs = self.jobs.read().await;
            let mut due = jobs
                .values()
                .filter_map(|job| {
                    let wake_at = next_wakeup_for_job(job)?;
                    (wake_at <= current_ms).then(|| (job.id.clone(), wake_at))
                })
                .collect::<Vec<_>>();
            due.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
            due.into_iter().map(|(job_id, _)| job_id).collect::<Vec<_>>()
        };

        for job_id in due_job_ids {
            self.process_job(&job_id).await?;
        }

        Ok(())
    }

    pub(super) async fn process_job(&self, job_id: &str) -> NortHingResult<()> {
        let _guard = self.mutation_lock.lock().await;
        let mut jobs = self.jobs.write().await;
        let current_ms = super::service_helpers::now_ms();

        let mut should_persist = false;
        let mut should_attempt_enqueue = false;
        let mut scheduled_at_ms = None;
        let mut enqueue_input = None;

        {
            let Some(job) = jobs.get_mut(job_id) else {
                return Ok(());
            };

            if !job.enabled && job.state.pending_trigger_at_ms.is_none() {
                return Ok(());
            }

            if let Some(next_run_at_ms) = job.state.next_run_at_ms {
                if next_run_at_ms <= current_ms {
                    let next_run_after_ms = compute_next_run_after_ms(&job.schedule, job.created_at_ms, current_ms)?;
                    job.state.apply_due_scheduled_trigger(next_run_at_ms, next_run_after_ms);
                    job.updated_at_ms = current_ms;
                    should_persist = true;
                }
            }

            if job.state.active_turn_id.is_none() && job.state.pending_is_due(current_ms) {
                let pending_trigger_at_ms = job.state.pending_trigger_at_ms.ok_or_else(|| {
                    NortHingError::service(format!("Scheduled job {} is missing pending trigger timestamp", job.id))
                })?;

                let turn_id = format!("cronjob_{}_{}", job.id, pending_trigger_at_ms);
                scheduled_at_ms = Some(pending_trigger_at_ms);
                let (user_input, prepended_messages) = format_scheduled_job_user_input(&job.payload.text, current_ms);
                enqueue_input = Some(EnqueueInput {
                    job_id: job.id.clone(),
                    job_name: job.name.clone(),
                    turn_id,
                    target: job.target.clone(),
                    user_input,
                    prepended_messages,
                });
                should_attempt_enqueue = true;
            }
        }

        if should_persist {
            self.persist_jobs_locked(&jobs).await?;
        }

        if !should_attempt_enqueue {
            return Ok(());
        }

        let enqueue_input = enqueue_input.ok_or_else(|| {
            NortHingError::service(format!(
                "Scheduled job {} is missing enqueue input after due calculation",
                job_id
            ))
        })?;
        let scheduled_at_ms = scheduled_at_ms.ok_or_else(|| {
            NortHingError::service(format!(
                "Scheduled job {} is missing scheduled timestamp after due calculation",
                job_id
            ))
        })?;

        let submit_result = self.submit_enqueue_input(&enqueue_input).await;

        let now_after_submit = super::service_helpers::now_ms();
        let Some(job) = jobs.get_mut(job_id) else {
            return Ok(());
        };

        match submit_result {
            Ok(_) => {
                let one_shot = job.is_one_shot();
                job.state
                    .mark_enqueued(enqueue_input.turn_id.clone(), now_after_submit, one_shot);
                job.updated_at_ms = now_after_submit;

                if one_shot {
                    job.enabled = false;
                }

                debug!(
                    "Scheduled job enqueued: job_id={}, target_kind={:?}, target_session_id={}, scheduled_at_ms={}",
                    job.id,
                    job.target_kind(),
                    submit_target_session_id(&enqueue_input),
                    scheduled_at_ms
                );
            }
            Err(error) => {
                let missing_session = matches!(job.target_kind(), CronJobTargetKind::Session)
                    && cron_enqueue_error_is_missing_session(&error);
                let failure_action = job.state.mark_enqueue_failed(
                    now_after_submit,
                    error.clone(),
                    DEFAULT_RETRY_DELAY_MS,
                    missing_session,
                );
                job.updated_at_ms = now_after_submit;

                if matches!(failure_action, ScheduledJobEnqueueFailureAction::DisableMissingSession) {
                    job.enabled = false;
                    info!(
                        "Scheduled job auto-disabled (session no longer exists): job_id={}, session_id={}",
                        job.id,
                        submit_target_session_id(&enqueue_input)
                    );
                } else {
                    warn!(
                        "Failed to enqueue scheduled job: job_id={}, target_kind={:?}, target_session_id={}, error={}",
                        job.id,
                        job.target_kind(),
                        submit_target_session_id(&enqueue_input),
                        error
                    );
                }
            }
        }

        self.persist_jobs_locked(&jobs).await?;
        drop(jobs);
        self.wakeup.notify_one();
        Ok(())
    }

    pub(super) async fn persist_snapshot(&self) -> NortHingResult<()> {
        let jobs = self.jobs.read().await;
        self.persist_jobs_locked(&jobs).await
    }

    pub(super) async fn submit_enqueue_input(&self, enqueue_input: &EnqueueInput) -> Result<(), String> {
        let resolved = self.resolve_enqueue_submission(enqueue_input).await?;
        self.runtime
            .submit_dialog_turn(AgentDialogTurnRequest {
                session_id: resolved.session_id,
                message: enqueue_input.user_input.clone(),
                original_message: Some(enqueue_input.user_input.clone()),
                turn_id: Some(enqueue_input.turn_id.clone()),
                agent_type: resolved.agent_type,
                workspace_path: Some(resolved.workspace_path),
                policy: super::service_helpers::scheduled_job_policy(),
                reply_route: None,
                prepended_reminders: enqueue_input.prepended_messages.clone(),
                attachments: Vec::new(),
                metadata: serde_json::Map::new(),
            })
            .await
            .map_err(crate::service_agent_runtime::CoreServiceAgentRuntime::runtime_error_message)
            .map(|_| ())
    }

    pub(super) async fn resolve_enqueue_submission(
        &self,
        enqueue_input: &EnqueueInput,
    ) -> Result<super::service_helpers::ResolvedEnqueueSubmission, String> {
        match &enqueue_input.target {
            CronJobTarget::Session { session_id, workspace } => {
                let agent_type = self
                    .coordinator
                    .session_manager()
                    .get_session(session_id)
                    .map(|session| session.agent_type)
                    .unwrap_or_default();
                Ok(super::service_helpers::ResolvedEnqueueSubmission {
                    session_id: session_id.clone(),
                    workspace_path: workspace.workspace_path.clone(),
                    agent_type,
                })
            }
            CronJobTarget::Workspace { workspace, launch } => {
                let created = self
                    .coordinator
                    .create_session_with_workspace(
                        None,
                        format!("Scheduled: {}", enqueue_input.job_name.trim()),
                        launch.agent_type.clone(),
                        SessionConfig {
                            workspace_path: Some(workspace.workspace_path.clone()),
                            workspace_id: workspace.workspace_id.clone(),
                            remote_connection_id: workspace.remote_connection_id.clone(),
                            remote_ssh_host: workspace.remote_ssh_host.clone(),
                            model_id: launch.model_id.clone(),
                            ..Default::default()
                        },
                        workspace.workspace_path.clone(),
                    )
                    .await
                    .map_err(|error| {
                        format!(
                            "Failed to create session for scheduled job {}: {}",
                            enqueue_input.job_id, error
                        )
                    })?;

                Ok(super::service_helpers::ResolvedEnqueueSubmission {
                    session_id: created.session_id,
                    workspace_path: workspace.workspace_path.clone(),
                    agent_type: created.agent_type,
                })
            }
        }
    }

    pub(super) async fn persist_jobs_locked(&self, jobs: &HashMap<String, CronJob>) -> NortHingResult<()> {
        self.store.save_jobs(jobs.values().cloned().collect::<Vec<_>>()).await
    }
}
