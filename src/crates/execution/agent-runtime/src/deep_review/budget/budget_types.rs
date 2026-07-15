use super::super::concurrency_policy::{DeepReviewEffectiveConcurrencySnapshot, DeepReviewEffectiveConcurrencyState};
use super::super::diagnostics::DeepReviewRuntimeDiagnostics;
use super::super::shared_context::{DeepReviewSharedContextKey, DeepReviewSharedContextUseRecord};
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

pub(crate) const BUDGET_TTL: Duration = Duration::from_secs(60 * 60);
pub(crate) const PRUNE_INTERVAL: Duration = Duration::from_secs(300);

#[derive(Debug)]
pub(crate) struct DeepReviewTurnBudget {
    pub(crate) judge_calls: usize,
    /// Tracks total reviewer calls (across all roles) per turn.
    /// Capped by `max_same_role_instances * reviewer_agent_type_count() +
    /// extra_subagent_ids.len()` so the orchestrator cannot spawn an unbounded
    /// number of same-role instances.
    pub(crate) reviewer_calls: usize,
    pub(crate) reviewer_calls_by_subagent: HashMap<String, usize>,
    pub(crate) retries_used_by_subagent: HashMap<String, usize>,
    pub(crate) active_reviewers: usize,
    pub(crate) active_reviewer_launch_batches: BTreeMap<u64, usize>,
    pub(crate) concurrency_cap_rejections: usize,
    pub(crate) capacity_skips: usize,
    pub(crate) shared_context_uses: HashMap<DeepReviewSharedContextKey, DeepReviewSharedContextUseRecord>,
    pub(crate) effective_concurrency: Option<DeepReviewEffectiveConcurrencyState>,
    pub(crate) runtime_diagnostics: DeepReviewRuntimeDiagnostics,
    pub(crate) created_at: std::time::Instant,
    pub(crate) updated_at: std::time::Instant,
}

impl DeepReviewTurnBudget {
    pub(crate) fn new(now: std::time::Instant) -> Self {
        Self {
            judge_calls: 0,
            reviewer_calls: 0,
            reviewer_calls_by_subagent: HashMap::new(),
            retries_used_by_subagent: HashMap::new(),
            active_reviewers: 0,
            active_reviewer_launch_batches: BTreeMap::new(),
            concurrency_cap_rejections: 0,
            capacity_skips: 0,
            shared_context_uses: HashMap::new(),
            effective_concurrency: None,
            runtime_diagnostics: DeepReviewRuntimeDiagnostics::default(),
            created_at: now,
            updated_at: now,
        }
    }

    pub(crate) fn effective_concurrency_mut(
        &mut self,
        configured_max_parallel_instances: usize,
    ) -> &mut DeepReviewEffectiveConcurrencyState {
        let state = self
            .effective_concurrency
            .get_or_insert_with(|| DeepReviewEffectiveConcurrencyState::new(configured_max_parallel_instances));
        state.rebase_configured_max(configured_max_parallel_instances);
        state
    }
}

pub struct DeepReviewActiveReviewerGuard<'a> {
    pub(crate) tracker: &'a super::budget_state::DeepReviewBudgetTracker,
    pub(crate) parent_dialog_turn_id: String,
    pub(crate) launch_batch: Option<u64>,
    pub(crate) released: bool,
}

impl Drop for DeepReviewActiveReviewerGuard<'_> {
    fn drop(&mut self) {
        if !self.released {
            self.tracker
                .finish_active_reviewer(&self.parent_dialog_turn_id, self.launch_batch);
            self.released = true;
        }
    }
}
