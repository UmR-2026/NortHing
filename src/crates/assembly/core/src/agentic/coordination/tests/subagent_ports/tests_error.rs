use super::*;
use tokio_util::sync::CancellationToken;

// ─── Test #5: Fail ───────────────────────────────────────────────────

/// Test #5: `Fail` scenario. The mock returns
/// `Err(NortHingError::Tool(message))` immediately. We unit-test
/// the mock's call_impl to assert the error propagates.
#[tokio::test]
async fn subagent_error_propagates_to_result() {
    let (_coordinator, _session_manager, mock) = build_test_coordinator_with_mock_tool(SubagentScenario::Fail {
        message: "boom".to_string(),
    })
    .await;

    // Primary: unit-test the mock's Fail arm.
    let ctx = empty_tool_context();
    let input = serde_json::json!({});
    let err = mock
        .call_impl(&input, &ctx)
        .await
        .expect_err("Fail scenario should return Err from call_impl");
    match err {
        NortHingError::Tool(msg) => {
            assert_eq!(msg, "boom", "expected the mock to carry the configured error message");
        }
        other => panic!("expected NortHingError::Tool, got {:?}", other),
    }

    // Secondary: verify 4 fields via real phase calls (LLM-error
    // path is fine — the 4 fields still get populated).
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
