//! Sub-domain: concurrency state + context profile policy.
//! Spec step-3.7 — extracted from subagent_orchestrator.rs (R50b refactor).

use super::super::coordinator::*;
use crate::agentic::agents::agent_registry;
use crate::agentic::context_profile::ContextProfilePolicy;
use crate::agentic::core::SessionConfig;
use crate::agentic::tools::pipeline::SubagentParentInfo;
use crate::service::config::global::GlobalConfigManager;
use crate::util::errors::{NortHingError, NortHingResult};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

const DEFAULT_SUBAGENT_MAX_CONCURRENCY: usize = 5;
const MAX_SUBAGENT_MAX_CONCURRENCY: usize = 64;

impl ConversationCoordinator {
    pub(crate) async fn get_subagent_concurrency_limiter(&self) -> SubagentConcurrencyLimiter {
        let configured = match GlobalConfigManager::service().await {
            Ok(config_service) => match config_service
                .config::<usize>(Some("ai.subagent_max_concurrency"))
                .await
            {
                Ok(value) => value,
                Err(error) => {
                    warn!(
                        "Failed to read ai.subagent_max_concurrency, using default {}: {}",
                        DEFAULT_SUBAGENT_MAX_CONCURRENCY, error
                    );
                    DEFAULT_SUBAGENT_MAX_CONCURRENCY
                }
            },
            Err(error) => {
                warn!(
                    "Config service unavailable while reading ai.subagent_max_concurrency, using default {}: {}",
                    DEFAULT_SUBAGENT_MAX_CONCURRENCY, error
                );
                DEFAULT_SUBAGENT_MAX_CONCURRENCY
            }
        };

        let normalized = normalize_subagent_max_concurrency(configured);
        if normalized != configured {
            warn!(
                "Normalized ai.subagent_max_concurrency from {} to {}",
                configured, normalized
            );
        }

        {
            let limiter_guard = self.subagent_concurrency_limiter.read().await;
            if let Some(limiter) = limiter_guard.as_ref() {
                if limiter.max_concurrency == normalized {
                    return limiter.clone();
                }
            }
        }

        let mut limiter_guard = self.subagent_concurrency_limiter.write().await;
        if let Some(limiter) = limiter_guard.as_ref() {
            if limiter.max_concurrency == normalized {
                return limiter.clone();
            }
        }

        let limiter = SubagentConcurrencyLimiter {
            semaphore: Arc::new(Semaphore::new(normalized)),
            max_concurrency: normalized,
        };
        *limiter_guard = Some(limiter.clone());
        limiter
    }

    pub(crate) async fn get_subagent_profile_concurrency_limiter(
        &self,
        max_concurrency: usize,
    ) -> SubagentConcurrencyLimiter {
        let max_concurrency = normalize_subagent_max_concurrency(max_concurrency);

        {
            let limiter_guard = self.subagent_profile_concurrency_limiters.read().await;
            if let Some(limiter) = limiter_guard.get(&max_concurrency) {
                return limiter.clone();
            }
        }

        let mut limiter_guard = self.subagent_profile_concurrency_limiters.write().await;
        if let Some(limiter) = limiter_guard.get(&max_concurrency) {
            return limiter.clone();
        }

        let limiter = SubagentConcurrencyLimiter {
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
            max_concurrency,
        };
        limiter_guard.insert(max_concurrency, limiter.clone());
        limiter
    }

    async fn acquire_permit_from_limiter(
        &self,
        limiter: &SubagentConcurrencyLimiter,
        agent_type: &str,
        cancel_token: Option<&CancellationToken>,
        deadline: Option<Instant>,
        label: &str,
    ) -> NortHingResult<OwnedSemaphorePermit> {
        let semaphore = limiter.semaphore.clone();
        let permit = match (cancel_token, deadline) {
            (Some(token), Some(deadline)) => {
                tokio::select! {
                    result = semaphore.acquire_owned() => result
                        .map_err(|error| NortHingError::Semaphore(error.to_string()))?,
                    _ = token.cancelled() => {
                        return Err(NortHingError::Cancelled(
                            "Subagent task was cancelled while waiting for a concurrency slot".to_string(),
                        ));
                    }
                    _ = tokio::time::sleep_until(deadline) => {
                        return Err(NortHingError::Timeout(format!(
                            "Timed out while waiting for a {} concurrency slot for subagent '{}'",
                            label, agent_type
                        )));
                    }
                }
            }
            (Some(token), None) => {
                tokio::select! {
                    result = semaphore.acquire_owned() => result
                        .map_err(|error| NortHingError::Semaphore(error.to_string()))?,
                    _ = token.cancelled() => {
                        return Err(NortHingError::Cancelled(
                            "Subagent task was cancelled while waiting for a concurrency slot".to_string(),
                        ));
                    }
                }
            }
            (None, Some(deadline)) => {
                tokio::select! {
                    result = semaphore.acquire_owned() => result
                        .map_err(|error| NortHingError::Semaphore(error.to_string()))?,
                    _ = tokio::time::sleep_until(deadline) => {
                        return Err(NortHingError::Timeout(format!(
                            "Timed out while waiting for a {} concurrency slot for subagent '{}'",
                            label, agent_type
                        )));
                    }
                }
            }
            (None, None) => semaphore
                .acquire_owned()
                .await
                .map_err(|error| NortHingError::Semaphore(error.to_string()))?,
        };

        let active_subagents = limiter
            .max_concurrency
            .saturating_sub(limiter.semaphore.available_permits());
        debug!(
            "Acquired subagent {} concurrency permit: agent_type={}, active_subagents={}, max_concurrency={}",
            label, agent_type, active_subagents, limiter.max_concurrency
        );

        Ok(permit)
    }

    pub(crate) async fn acquire_subagent_concurrency_permit(
        &self,
        agent_type: &str,
        profile_concurrency_cap: usize,
        cancel_token: Option<&CancellationToken>,
        deadline: Option<Instant>,
    ) -> NortHingResult<(Vec<(OwnedSemaphorePermit, SubagentConcurrencyLimiter)>, u128)> {
        let started_waiting = Instant::now();

        let profile_limiter = self
            .get_subagent_profile_concurrency_limiter(profile_concurrency_cap)
            .await;
        let profile_permit = self
            .acquire_permit_from_limiter(&profile_limiter, agent_type, cancel_token, deadline, "profile")
            .await?;

        let global_limiter = self.get_subagent_concurrency_limiter().await;
        let global_permit = self
            .acquire_permit_from_limiter(&global_limiter, agent_type, cancel_token, deadline, "global")
            .await?;

        let wait_ms = started_waiting.elapsed().as_millis();
        debug!(
            "Acquired subagent concurrency permits: agent_type={}, wait_ms={}, profile_max_concurrency={}, global_max_concurrency={}",
            agent_type, wait_ms, profile_limiter.max_concurrency, global_limiter.max_concurrency
        );

        Ok((
            vec![(profile_permit, profile_limiter), (global_permit, global_limiter)],
            wait_ms,
        ))
    }

    pub(crate) fn context_profile_policy_for_subagent(
        &self,
        agent_type: &str,
        session_config: &SessionConfig,
        subagent_parent_info: Option<&SubagentParentInfo>,
    ) -> ContextProfilePolicy {
        if let Some(parent_info) = subagent_parent_info {
            if let Some(parent_session) = self.session_manager.get_session(&parent_info.session_id) {
                let parent_is_review_subagent = agent_registry()
                    .get_subagent_is_review(&parent_session.agent_type)
                    .unwrap_or(false);
                let is_review_subagent = agent_registry().get_subagent_is_review(agent_type).unwrap_or(false);
                return ContextProfilePolicy::for_subagent_context_and_models(
                    agent_type,
                    is_review_subagent,
                    session_config.model_id.as_deref(),
                    Some(&parent_session.agent_type),
                    parent_is_review_subagent,
                    parent_session.config.model_id.as_deref(),
                );
            }
        }

        let is_review_subagent = agent_registry().get_subagent_is_review(agent_type).unwrap_or(false);
        let model_id = session_config.model_id.as_deref().unwrap_or_default();
        ContextProfilePolicy::for_agent_context_and_model(agent_type, is_review_subagent, model_id, model_id)
    }
}
