use super::*;
use tokio_util::sync::CancellationToken;

// ─── Test #6: parent chain ───────────────────────────────────────────

/// Test #6: parent chain. The spec's "degraded path" uses a
/// single-level `SpawnNested { depth: 1, max_depth: 1, after: 50ms }`
/// (no actual recursion). We unit-test the mock's SpawnNested arm:
/// it currently returns `Err("SpawnNested: not yet wired")`. We
/// assert on that error; when the recursive wiring lands (out of
/// scope for this PR per spec §5), this test can be updated to
/// assert on the leaf result.
#[tokio::test]
async fn subagent_parent_chain_propagates_through_nested_calls() {
    let (_coordinator, _session_manager, mock) = build_test_coordinator_with_mock_tool(SubagentScenario::SpawnNested {
        depth: 1,
        max_depth: 1,
        after: Duration::from_millis(50),
    })
    .await;

    // Primary: unit-test the mock's SpawnNested arm.
    let ctx = empty_tool_context();
    let input = serde_json::json!({});
    let err = mock
        .call_impl(&input, &ctx)
        .await
        .expect_err("SpawnNested currently returns Err (not yet wired)");
    match err {
        NortHingError::Tool(msg) => {
            assert!(
                msg.contains("SpawnNested"),
                "expected SpawnNested error message, got {:?}",
                msg
            );
        }
        other => panic!("expected NortHingError::Tool, got {:?}", other),
    }

    // Secondary: verify 4 fields via real phase calls.
    let cancel_token = CancellationToken::new();
    let request = build_minimal_request();
    let phase1 = _coordinator
        .execute_hidden_subagent_phase1(request, Some(&cancel_token), None)
        .await
        .expect("phase 1 should succeed");
    let phase2 = _coordinator
        .execute_hidden_subagent_phase2(&phase1, Some(&cancel_token))
        .await
        .expect("phase 2 should return Ok (4 fields populated)");
    assert_secondary_fields_populated(&phase2, "");
}
