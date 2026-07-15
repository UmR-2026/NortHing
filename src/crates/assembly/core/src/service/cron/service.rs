//! Scheduled job service – facade.
//!
//! Public API stays here; private impl lives in `service_impl.rs` and
//! free helpers in `service_helpers.rs`.

use super::schedule::{compute_initial_next_run_at_ms, compute_next_run_after_ms, validate_schedule};
use super::store::CronJobStore;
use super::types::{
    CreateCronJobRequest, CronJob, CronJobPayload, CronJobTarget, CronJobTargetKind, CronLaunchSpec, CronSchedule,
    CronWorkspaceRef, UpdateCronJobRequest, DEFAULT_RETRY_DELAY_MS,
};
use crate::agentic::coordination::{ConversationCoordinator, DialogScheduler};
use crate::infrastructure::PathManager;
use crate::service_agent_runtime::CoreServiceAgentRuntime;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_agent_runtime::runtime::AgentRuntime;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex, Notify, RwLock};
use uuid::Uuid;

use super::service_helpers::{
    matches_workspace_filter, materialize_schedule, materialize_target, now_ms, reconcile_loaded_job,
    validate_request_fields,
};

static GLOBAL_CRON_SERVICE: OnceLock<Arc<CronService>> = OnceLock::new();

pub struct CronService {
    pub(super) coordinator: Arc<ConversationCoordinator>,
    pub(super) runtime: AgentRuntime,
    pub(super) store: Arc<CronJobStore>,
    pub(super) jobs: Arc<RwLock<HashMap<String, CronJob>>>,
    pub(super) mutation_lock: Arc<Mutex<()>>,
    pub(super) wakeup: Arc<Notify>,
    pub(super) runner_started: AtomicBool,
}

impl CronService {
    pub async fn new(
        path_manager: Arc<PathManager>,
        coordinator: Arc<ConversationCoordinator>,
        scheduler: Arc<DialogScheduler>,
    ) -> NortHingResult<Arc<Self>> {
        let store = Arc::new(CronJobStore::new(path_manager).await?);
        let loaded = store.load().await?;
        let current_ms = now_ms();

        let mut jobs = HashMap::new();
        let mut needs_save = false;

        for mut job in loaded.jobs {
            if jobs.contains_key(&job.id) {
                return Err(NortHingError::service(format!(
                    "Duplicate scheduled job id found in jobs.json: {}",
                    job.id
                )));
            }

            needs_save |= reconcile_loaded_job(&mut job, current_ms)?;
            jobs.insert(job.id.clone(), job);
        }

        let runtime = CoreServiceAgentRuntime::agent_runtime_with_dialog_turns(coordinator.clone(), scheduler)
            .map_err(NortHingError::service)?;

        let service = Arc::new(Self {
            coordinator,
            runtime,
            store,
            jobs: Arc::new(RwLock::new(jobs)),
            mutation_lock: Arc::new(Mutex::new(())),
            wakeup: Arc::new(Notify::new()),
            runner_started: AtomicBool::new(false),
        });

        if needs_save {
            service.persist_snapshot().await?;
        }

        Ok(service)
    }

    pub fn start(self: &Arc<Self>) {
        if self
            .runner_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        let service = Arc::clone(self);
        tokio::spawn(async move {
            service.run_loop().await;
        });
    }

    pub async fn list_jobs(&self) -> Vec<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect::<Vec<_>>()
    }

    pub async fn list_jobs_filtered(
        &self,
        workspace_path: Option<&str>,
        workspace_id: Option<&str>,
        remote_connection_id: Option<&str>,
        session_id: Option<&str>,
        target_kind: Option<CronJobTargetKind>,
    ) -> Vec<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.values()
            .filter(|job| {
                matches_workspace_filter(job.workspace(), workspace_path, workspace_id, remote_connection_id)
                    && session_id
                        .map(|session_id| job.session_id() == Some(session_id))
                        .unwrap_or(true)
                    && target_kind
                        .map(|target_kind| job.target_kind() == target_kind)
                        .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>()
    }

    pub async fn get_job(&self, job_id: &str) -> Option<CronJob> {
        self.jobs.read().await.get(job_id).cloned()
    }

    pub async fn create_job(&self, request: CreateCronJobRequest) -> NortHingResult<CronJob> {
        let _guard = self.mutation_lock.lock().await;
        let mut jobs = self.jobs.write().await;
        let current_ms = now_ms();
        let schedule = materialize_schedule(request.schedule, current_ms);
        let target = materialize_target(request.target);
        validate_request_fields(&request.name, &request.payload, &target)?;
        validate_schedule(&schedule, current_ms)?;

        let mut job = CronJob {
            id: format!("cron_{}", Uuid::new_v4().simple()),
            name: request.name.trim().to_string(),
            schedule,
            payload: request.payload,
            enabled: request.enabled,
            target,
            created_at_ms: current_ms,
            config_updated_at_ms: current_ms,
            updated_at_ms: current_ms,
            state: Default::default(),
        };

        if job.enabled {
            job.state.next_run_at_ms = compute_initial_next_run_at_ms(&job, current_ms)?;
        }

        jobs.insert(job.id.clone(), job.clone());
        self.persist_jobs_locked(&jobs).await?;
        drop(jobs);
        self.wakeup.notify_one();

        Ok(job)
    }

    pub async fn update_job(&self, job_id: &str, request: UpdateCronJobRequest) -> NortHingResult<CronJob> {
        let _guard = self.mutation_lock.lock().await;
        let mut jobs = self.jobs.write().await;
        let current_ms = now_ms();
        let job = jobs
            .get_mut(job_id)
            .ok_or_else(|| NortHingError::NotFound(format!("Scheduled job not found: {}", job_id)))?;

        if let Some(name) = request.name {
            job.name = name.trim().to_string();
        }
        if let Some(payload) = request.payload {
            job.payload = payload;
        }
        if let Some(target) = request.target {
            job.target = materialize_target(target);
        }
        if let Some(enabled) = request.enabled {
            job.enabled = enabled;
        }
        if let Some(schedule) = request.schedule {
            job.schedule = materialize_schedule(schedule, current_ms);
        }

        validate_request_fields(&job.name, &job.payload, &job.target)?;
        validate_schedule(&job.schedule, job.created_at_ms)?;

        job.config_updated_at_ms = current_ms;
        job.updated_at_ms = current_ms;
        job.state.clear_pending_trigger();

        if !job.enabled {
            job.state.next_run_at_ms = None;
        } else if job.state.active_turn_id.is_some() {
            if job.is_one_shot() {
                job.state.next_run_at_ms = None;
            } else {
                job.state.next_run_at_ms = compute_next_run_after_ms(&job.schedule, job.created_at_ms, current_ms)?;
            }
        } else {
            job.state.next_run_at_ms = compute_initial_next_run_at_ms(job, current_ms)?;
        }

        let updated = job.clone();
        self.persist_jobs_locked(&jobs).await?;
        drop(jobs);
        self.wakeup.notify_one();

        Ok(updated)
    }

    pub async fn set_job_enabled(&self, job_id: &str, enabled: bool) -> NortHingResult<CronJob> {
        self.update_job(
            job_id,
            UpdateCronJobRequest {
                enabled: Some(enabled),
                ..Default::default()
            },
        )
        .await
    }

    pub async fn delete_job(&self, job_id: &str) -> NortHingResult<bool> {
        let _guard = self.mutation_lock.lock().await;
        let mut jobs = self.jobs.write().await;
        let existed = jobs.remove(job_id).is_some();
        if existed {
            self.persist_jobs_locked(&jobs).await?;
            drop(jobs);
            self.wakeup.notify_one();
        }
        Ok(existed)
    }

    /// Remove all scheduled jobs bound to the given session (e.g. after session delete).
    pub async fn delete_jobs_for_session(&self, session_id: &str) -> NortHingResult<usize> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Ok(0);
        }
        let _guard = self.mutation_lock.lock().await;
        let mut jobs = self.jobs.write().await;
        let before = jobs.len();
        jobs.retain(|_, job| job.session_id() != Some(session_id));
        let removed = before - jobs.len();
        if removed > 0 {
            self.persist_jobs_locked(&jobs).await?;
            drop(jobs);
            self.wakeup.notify_one();
        }
        Ok(removed)
    }

    pub async fn run_job_now(&self, job_id: &str) -> NortHingResult<CronJob> {
        {
            let _guard = self.mutation_lock.lock().await;
            let mut jobs = self.jobs.write().await;
            let current_ms = now_ms();
            let job = jobs
                .get_mut(job_id)
                .ok_or_else(|| NortHingError::NotFound(format!("Scheduled job not found: {}", job_id)))?;

            job.state.mark_manual_trigger(current_ms);
            job.updated_at_ms = current_ms;

            self.persist_jobs_locked(&jobs).await?;
            drop(jobs);
            self.wakeup.notify_one();
        }

        self.process_job(job_id).await?;
        self.get_job(job_id)
            .await
            .ok_or_else(|| NortHingError::NotFound(format!("Scheduled job not found after run: {}", job_id)))
    }

    pub async fn handle_turn_started(&self, turn_id: &str) -> NortHingResult<()> {
        self.handle_turn_state_change(turn_id, |job, now_ms| {
            job.state.mark_turn_started(now_ms);
            job.updated_at_ms = now_ms;
        })
        .await
    }

    pub async fn handle_turn_completed(&self, turn_id: &str, duration_ms: u64) -> NortHingResult<()> {
        self.handle_turn_state_change(turn_id, |job, now_ms| {
            job.state.mark_turn_completed(now_ms, duration_ms);
            job.updated_at_ms = now_ms;
        })
        .await
    }

    pub async fn handle_turn_failed(&self, turn_id: &str, error: &str) -> NortHingResult<()> {
        self.handle_turn_state_change(turn_id, |job, now_ms| {
            job.state.mark_turn_failed(now_ms, error.to_string());
            job.updated_at_ms = now_ms;
        })
        .await
    }

    pub async fn handle_turn_cancelled(&self, turn_id: &str) -> NortHingResult<()> {
        self.handle_turn_state_change(turn_id, |job, now_ms| {
            job.state.mark_turn_cancelled(now_ms);
            job.updated_at_ms = now_ms;
        })
        .await
    }
}

pub fn global_cron_service() -> Option<Arc<CronService>> {
    GLOBAL_CRON_SERVICE.get().cloned()
}

pub fn set_global_cron_service(service: Arc<CronService>) {
    let _ = GLOBAL_CRON_SERVICE.set(service);
}
