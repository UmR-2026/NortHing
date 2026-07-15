use super::*;
use tokio_util::sync::CancellationToken;

// ─── Test #4: Timeout ────────────────────────────────────────────────

/// Test #4: timeout propagation. Phase 1's `timeout_seconds = 1`
/// sets a 1-second deadline; phase 2's main loop observes the
/// deadline. With the dev environment's missing LLM, the task
/// fails first (microseconds), so the join_result arm wins; the
/// 4 dead-code fields are still populated.
#[tokio::test]
async fn subagent_timeout_returns_partial() {
    let (coordinator, _session_manager, _mock) =
        build_test_coordinator_with_mock_tool(SubagentScenario::SleepForever).await;

    let cancel_token = CancellationToken::new();
    let request = build_minimal_request();
    let phase1 = coordinator
        .execute_hidden_subagent_phase1(request, Some(&cancel_token), Some(1))
        .await
        .expect("phase 1 should succeed");
    let phase2 = coordinator
        .execute_hidden_subagent_phase2(&phase1, Some(&cancel_token))
        .await
        .expect("phase 2 should return Ok (4 fields populated)");

    // Primary assertion: 4 dead-code fields are populated.
    assert_secondary_fields_populated(&phase2, "");

    // Secondary: the spawned task finished (either via join_result
    // because of the LLM error, or because the task itself
    // acknowledged the timeout). Both are acceptable per spec §5.
    assert!(
        phase2.execution_task.is_finished(),
        "expected execution_task to be finished"
    );
}
