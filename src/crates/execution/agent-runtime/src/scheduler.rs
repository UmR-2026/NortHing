//! Scheduler owner decisions.
//!
//! Facade for the scheduler owner. The original 877-line `scheduler.rs`
//! has been split into three sibling modules that each own a focused
//! sub-domain:
//!
//! - [`sched_types`]   — pure data types: structs, enums, and their
//!                       inherent impls (active-turn shape, outcome enums,
//!                       delivery-plan shapes, status / queue-action tags).
//! - [`sched_state`]   — mutable per-session state stores
//!                       ([`ActiveDialogTurnStore`], [`DialogReplySuppressionSet`],
//!                       [`SessionAbortFlags`]), the priority-bounded
//!                       [`DialogTurnQueue`], and the round-injection
//!                       sources ([`NoopDialogRoundInjectionSource`],
//!                       [`DialogRoundInjectionInterrupt`],
//!                       [`SessionRoundInjectionBuffer`]).
//! - [`sched_filter`]  — pure decide / resolve functions that take facts
//!                       in and return an action / plan
//!                       ([`resolve_background_delivery_action`],
//!                       [`resolve_background_delivery_injection`],
//!                       [`build_thread_goal_resumed_delivery_plan`],
//!                       [`build_thread_goal_objective_updated_delivery_plan`],
//!                       [`resolve_turn_outcome_lifecycle_plan`],
//!                       [`resolve_agent_session_reply_action`],
//!                       [`resolve_dialog_steering_action`]).
//!
//! All public items from the three siblings are re-exported below so the
//! existing `northhing_agent_runtime::scheduler::Item` import paths keep
//! working unchanged for downstream consumers (events, tests, assembly/core).
//!
//! Behaviour, public type surface, and module path are unchanged; only
//! the file layout was reorganised.

mod sched_filter;
mod sched_state;
mod sched_types;

pub use sched_filter::{
    build_thread_goal_objective_updated_delivery_plan, build_thread_goal_resumed_delivery_plan,
    resolve_agent_session_reply_action, resolve_background_delivery_action, resolve_background_delivery_injection,
    resolve_dialog_steering_action, resolve_turn_outcome_lifecycle_plan,
};
pub use sched_state::{
    ActiveDialogTurnStore, DialogReplySuppressionSet, DialogRoundInjectionInterrupt, DialogTurnQueue,
    NoopDialogRoundInjectionSource, SessionAbortFlags, SessionRoundInjectionBuffer,
};
pub use sched_types::{
    ActiveDialogTurn, AgentSessionReplyAction, AgentSessionReplyPlan, BackgroundDeliveryAction,
    BackgroundDeliveryFacts, BackgroundInjectionKind, DialogSteeringAction, DialogTurnQueueError,
    GoalContinuationAfterTurnAction, ThreadGoalDeliveryPlan, ThreadGoalDeliveryReminder,
    ThreadGoalDeliveryReminderKind, TurnOutcome, TurnOutcomeLifecyclePlan, TurnOutcomeQueueAction, TurnOutcomeStatus,
    DEFAULT_MAX_DIALOG_QUEUE_DEPTH,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_lifecycle_dispatches_completed_turn_and_verifies_goal() {
        let outcome = TurnOutcome::Completed {
            turn_id: "turn_1".to_string(),
            final_response: "done".to_string(),
        };

        let plan = resolve_turn_outcome_lifecycle_plan(&outcome, true);

        assert_eq!(plan.status, TurnOutcomeStatus::Completed);
        assert_eq!(plan.queue_action, TurnOutcomeQueueAction::DispatchNext);
        assert!(plan.drain_finished_turn_injections);
        assert_eq!(
            plan.goal_continuation,
            GoalContinuationAfterTurnAction::Evaluate { turn_completed: true }
        );
        assert!(plan.dispatch_next());
        assert!(!plan.clear_queue());
    }

    #[test]
    fn outcome_lifecycle_aborts_goal_continuation_for_cancelled_turn() {
        let outcome = TurnOutcome::Cancelled {
            turn_id: "turn_1".to_string(),
        };

        let plan = resolve_turn_outcome_lifecycle_plan(&outcome, true);

        assert_eq!(plan.status, TurnOutcomeStatus::Cancelled);
        assert_eq!(plan.queue_action, TurnOutcomeQueueAction::DispatchNext);
        assert_eq!(
            plan.goal_continuation,
            GoalContinuationAfterTurnAction::AbortForCancelled
        );
        assert!(plan.dispatch_next());
        assert!(!plan.clear_queue());
    }

    #[test]
    fn outcome_lifecycle_clears_queue_for_failed_turn_and_verifies_goal() {
        let outcome = TurnOutcome::Failed {
            turn_id: "turn_1".to_string(),
            error: "boom".to_string(),
        };

        let plan = resolve_turn_outcome_lifecycle_plan(&outcome, true);

        assert_eq!(plan.status, TurnOutcomeStatus::Failed);
        assert_eq!(plan.queue_action, TurnOutcomeQueueAction::ClearQueue);
        assert_eq!(
            plan.goal_continuation,
            GoalContinuationAfterTurnAction::Evaluate { turn_completed: false }
        );
        assert!(!plan.dispatch_next());
        assert!(plan.clear_queue());
    }

    #[test]
    fn outcome_lifecycle_skips_goal_when_no_active_turn_exists() {
        let outcome = TurnOutcome::Completed {
            turn_id: "turn_1".to_string(),
            final_response: "done".to_string(),
        };

        let plan = resolve_turn_outcome_lifecycle_plan(&outcome, false);

        assert_eq!(
            plan.goal_continuation,
            GoalContinuationAfterTurnAction::SkipNoActiveTurn
        );
        assert!(plan.dispatch_next());
    }
}
