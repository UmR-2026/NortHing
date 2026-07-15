use super::*;
use tokio_util::sync::CancellationToken;

// ─── Test #1: Succeed scenario ───────────────────────────────────────

/// Test #1: `Succeed` scenario. The mock should return a
/// `ToolResult::Result` whose `result_for_assistant` carries the
/// configured `text` (Errata v2.B: not just `data["text"]`). This
/// validates the mock's wiring; the 4 dead-code fields are
/// verified via the phase 1 + phase 2 calls below.
#[tokio::test]
async fn subagent_success_completes_with_text() {
    let (_coordinator, _session_manager, mock) = build_test_coordinator_with_mock_tool(SubagentScenario::Succeed {
        after: Duration::from_millis(50),
        text: "ok".to_string(),
    })
    .await;

    // ── Primary: unit-test the mock's call_impl ─────────────────
    let ctx = empty_tool_context();
    let input = serde_json::json!({});
    let mock_results = mock
        .call_impl(&input, &ctx)
        .await
        .expect("Succeed scenario should return Ok from call_impl");
    assert_eq!(mock_results.len(), 1, "expected exactly 1 ToolResult");
    match &mock_results[0] {
        ToolResult::Result {
            result_for_assistant, ..
        } => {
            assert_eq!(
                result_for_assistant.as_deref(),
                Some("ok"),
                "Errata v2.B: result_for_assistant must carry the configured text"
            );
        }
        other => panic!("expected ToolResult::Result, got {:?}", other),
    }

    // ── Secondary: verify the 4 dead-code fields via real phase calls ──
    let cancel_token = CancellationToken::new();
    let request = build_minimal_request();
    let phase1 = _coordinator
        .execute_hidden_subagent_phase1(request, Some(&cancel_token), None)
        .await
        .expect("phase 1 should succeed (no LLM needed)");
    let phase2 = _coordinator
        .execute_hidden_subagent_phase2(&phase1, Some(&cancel_token))
        .await
        .expect("phase 2 should succeed (4 fields populated even on LLM error path)");
    assert_secondary_fields_populated(&phase2, "ok");
}

// ─── Test #2: Succeed with 5000-char text ────────────────────────────

/// Test #2: `Succeed` scenario with a 5000-char payload. Verifies
/// the mock can transport large strings (Errata v2.B: text must be
/// in `result_for_assistant`, not just `data["text"]`, so the
/// downstream path picks it up).
#[tokio::test]
async fn subagent_success_transmits_large_payload() {
    let payload = "x".repeat(5_000);
    let (_coordinator, _session_manager, mock) = build_test_coordinator_with_mock_tool(SubagentScenario::Succeed {
        after: Duration::from_millis(50),
        text: payload.clone(),
    })
    .await;

    // ── Primary: unit-test the mock with the 5000-char text ─────
    let ctx = empty_tool_context();
    let input = serde_json::json!({});
    let mock_results = mock
        .call_impl(&input, &ctx)
        .await
        .expect("Succeed scenario should return Ok");
    assert_eq!(mock_results.len(), 1);
    match &mock_results[0] {
        ToolResult::Result {
            result_for_assistant, ..
        } => {
            let got = result_for_assistant
                .as_deref()
                .expect("result_for_assistant must be Some");
            assert_eq!(got.len(), 5_000);
            assert_eq!(got, payload);
        }
        other => panic!("expected ToolResult::Result, got {:?}", other),
    }

    // ── Secondary: verify 4 fields ──
    let cancel_token = CancellationToken::new();
    let request = build_minimal_request();
    let phase1 = _coordinator
        .execute_hidden_subagent_phase1(request, Some(&cancel_token), None)
        .await
        .expect("phase 1 should succeed");
    let phase2 = _coordinator
        .execute_hidden_subagent_phase2(&phase1, Some(&cancel_token))
        .await
        .expect("phase 2 should succeed (4 fields populated even on LLM error)");
    assert_secondary_fields_populated(&phase2, &payload);
}
