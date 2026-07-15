//! Integration test for `OfflineSubAgentProfile` + `FixtureLoader`.
//!
//! Exercises the full profile lifecycle:
//! 1. Load a profile from `tests/fixtures/llm/<name>.json`.
//! 2. Drive every round via `tick(round_index)`.
//! 3. Assert each round returns the canned `Continue` (or final `Done`).
//! 4. Assert the round count matches the fixture (no off-by-one).
//!
//! The test is hermetic: it reads a JSON fixture from disk, drives the
//! profile in memory, and asserts on the resulting `OfflineTickOutput`
//! values. No LLM provider, no network, no temp-dir cleanup needed
//! (the fixture files are checked in).

use northhing_test_support::{
    FixtureLoader, OfflineSubAgentProfile, OfflineTickOutput, OfflineToolCall,
};
use serde_json::json;

fn loader() -> FixtureLoader {
    // The integration test is run with the crate root as cwd
    // (cargo's default for `cargo test`), so `tests/fixtures/llm` is
    // a relative path.
    FixtureLoader::new("tests/fixtures/llm")
}

#[test]
fn echo_single_round_loads_and_dones() {
    let p = loader().load_profile("echo_single_round").expect("fixture should load");
    assert_eq!(p.profile_id, "echo_single_round");
    assert_eq!(p.agent_type, "echo-agent");
    assert_eq!(p.round_count(), 1);
    match p.tick(0).expect("round 0 exists") {
        OfflineTickOutput::Done { round_id, final_text } => {
            assert_eq!(round_id, "r0");
            assert_eq!(final_text, "Echo: hello from offline profile");
        }
        other => panic!("expected Done, got {:?}", other),
    }
}

#[test]
fn multi_round_with_tools_drives_full_sequence() {
    let p = loader().load_profile("multi_round_with_tools").expect("fixture should load");
    assert_eq!(p.round_count(), 4);

    // Round 0: continue with tool call
    match p.tick(0).expect("round 0") {
        OfflineTickOutput::Continue { round_id, text, tool_call } => {
            assert_eq!(round_id, "r0");
            assert_eq!(text, "Looking up workspace path");
            let tc = tool_call.expect("round 0 has tool call");
            assert_eq!(tc.tool_name, "list_directory");
            assert_eq!(tc.arguments, json!({"path": "."}));
        }
        other => panic!("round 0: expected Continue, got {:?}", other),
    }

    // Round 1: continue with another tool call
    match p.tick(1).expect("round 1") {
        OfflineTickOutput::Continue { round_id, text, tool_call } => {
            assert_eq!(round_id, "r1");
            assert_eq!(text, "Reading first file");
            let tc = tool_call.expect("round 1 has tool call");
            assert_eq!(tc.tool_name, "read_file");
        }
        other => panic!("round 1: expected Continue, got {:?}", other),
    }

    // Round 2: continue, no tool call (text-only)
    match p.tick(2).expect("round 2") {
        OfflineTickOutput::Continue { round_id, text, tool_call } => {
            assert_eq!(round_id, "r2");
            assert_eq!(text, "Summarizing what I read");
            assert!(tool_call.is_none());
        }
        other => panic!("round 2: expected Continue, got {:?}", other),
    }

    // Round 3: final
    match p.tick(3).expect("round 3") {
        OfflineTickOutput::Done { round_id, final_text } => {
            assert_eq!(round_id, "r3");
            assert!(final_text.contains("delivered summary"));
        }
        other => panic!("round 3: expected Done, got {:?}", other),
    }
}

#[test]
fn long_running_default_drives_six_rounds() {
    let p = loader().load_profile("long_running_default").expect("fixture should load");
    assert_eq!(p.round_count(), 6);

    // Walk the profile; assert that exactly one round is `Done` and
    // it is the last one.
    let mut continues = 0;
    let mut dones = 0;
    for i in 0..p.round_count() {
        match p.tick(i).expect("round exists") {
            OfflineTickOutput::Continue { .. } => continues += 1,
            OfflineTickOutput::Done { .. } => {
                dones += 1;
                assert_eq!(i, p.round_count() - 1, "Done must be the last round");
            }
        }
    }
    assert_eq!(continues, 5, "5 non-final rounds expected");
    assert_eq!(dones, 1, "exactly one final round expected");
}

#[test]
fn out_of_range_after_done_returns_error() {
    let p = loader().load_profile("echo_single_round").expect("fixture should load");
    // Round 0 -> Done. Round 1 -> out of range.
    assert!(matches!(p.tick(0), Ok(OfflineTickOutput::Done { .. })));
    assert!(p.tick(1).is_err());
}

#[test]
fn builder_api_produces_equivalent_profile_to_json_fixture() {
    // The builder API and JSON fixture must produce equivalent profiles.
    // If they ever diverge, the test catches the drift in fixture
    // authoring vs. typed construction.
    let from_json: OfflineSubAgentProfile = loader().load_profile("echo_single_round").expect("fixture should load");

    let tool_call = OfflineToolCall {
        tool_name: "noop".into(),
        arguments: json!({}),
    };
    let from_builder = OfflineSubAgentProfile::new("echo_single_round", "echo-agent")
        .with_final_round("r0", "Echo: hello from offline profile");
    // The profiles are equivalent on the (profile_id, agent_type, rounds)
    // axes; tool_call field is not relevant to `echo_single_round`.
    let _ = tool_call; // suppress unused warning if compiler warns
    assert_eq!(from_json.profile_id, from_builder.profile_id);
    assert_eq!(from_json.agent_type, from_builder.agent_type);
    assert_eq!(from_json.round_count(), from_builder.round_count());
    assert_eq!(from_json.rounds[0].text, from_builder.rounds[0].text);
    assert_eq!(from_json.rounds[0].is_final, from_builder.rounds[0].is_final);
}
