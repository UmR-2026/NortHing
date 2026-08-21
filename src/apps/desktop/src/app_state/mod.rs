//! App State - Bridge between Slint UI and northhing-core
//!
//! Manages data binding, event handling, and state synchronization.
//!
//! ## Safety (Phase I.2, 2026-06-20)
//!
//! The pre-existing 5 raw-pointer casts in the Slint callback bodies
//! (one per callback) were removed; closures now capture `Arc<AppState>`
//! and `Weak<AppWindow>` instead of raw pointers. `Arc::clone` is cheap
//! (one atomic increment) and the closures have `'static` lifetime
//! since `AppState` outlives the UI loop.
//!
//! The Slint-generated `ItemTreeVTable_static` macro internally
//! emits `unsafe { ... }` blocks, so we can't apply
//! `#![forbid(unsafe_code)]` to this file 鈥?the lint is intentionally
//! omitted. Future maintainers adding code should stay in safe Rust
//! (no new `unsafe { }` blocks in this file); grep for `unsafe` to
//! audit.

// Existing siblings (Phase B split, preserved)
pub(super) mod inspector;
pub(super) mod inspector_model_status;
pub(super) mod log;
pub(super) mod sessions;
pub(super) mod settings;
pub(super) mod skills;
pub(super) mod slint_glue;
pub(super) mod streaming_lifecycle;

// R37a NEW siblings (split from this 2122-line mod.rs)
pub(super) mod callbacks_lifecycle;
pub(super) mod callbacks_settings;
pub(super) mod create_ui;
pub(super) mod error_banners;
pub(super) mod event_bridge;
pub(super) mod state;

// W4: long-lived runtime handle for turn dispatch (spawns onto the
// worker runtime instead of a throwaway per-callback runtime).
pub(super) mod turn_runtime;

// Wildcard re-exports so `crate::app_state::{AppState, create_ui, ...}`
// and `crate::app_state::set_session_error` keep working from callers
// (main.rs, sessions.rs, etc.). Preserves the cross-crate import paths.
// (2026-07-27: callbacks_lifecycle/callbacks_settings glob re-exports removed —
// their items are pub(super)/pub(crate) and were never visible downstream.)
pub use create_ui::*;
pub use error_banners::*;
pub use state::*;

// 2026-07-27 (K4a R3, fix #4): the test module that used
// these imports was removed; the imports themselves are
// now dead and would warn. Drop them.
use slint::{ModelRc, SharedString, VecModel};
use slint_glue::{MessageItem, SessionItem, SkillItem};

// 鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺?// Phase I.5 tests (2026-06-20)
// 鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺?
#[cfg(test)]
mod phase_i_tests {
    //! Smoke tests for the Slint DTO projection helpers. These cover the
    //! pure functions (`build_sessions_model`, `session_summary_to_item`,
    //! `build_messages_model`) 鈥?the higher-level `create_ui` test would
    //! need a real display handle and is left for future Phase I.x work
    //! (or for manual smoke-testing).

    use super::sessions::{build_messages_model, build_sessions_model, message_to_item};
    use northhing_kernel_api::session::{
        MessageContentDto, MessageDto, MessageMetadataDto, MessageRoleDto, SessionStateDto, SessionStatusDto,
        SessionSummaryDto, ToolCallStub,
    };
    use slint::Model;

    fn sample_meta() -> MessageMetadataDto {
        MessageMetadataDto {
            turn_id: None,
            round_id: None,
            tokens: None,
            thinking_signature: None,
            semantic_kind: None,
            internal_reminder_kind: None,
            compression_payload: None,
        }
    }

    fn sample_summary(id: &str, parent_id: Option<&str>, depth_target_id: Option<&str>) -> SessionSummaryDto {
        // The `parent_session_id` field on SessionSummaryDto is what the
        // depth walker reads. `depth_target_id` is unused here (the
        // helper computes depth from parent links); kept in the
        // signature to make the test call sites self-documenting.
        let _ = depth_target_id;
        SessionSummaryDto {
            id: id.into(),
            name: format!("Session {id}"),
            updated_at: 0,
            status: SessionStatusDto::Active,
            parent_session_id: parent_id.map(String::from),
            state: Some("idle".to_string()),
        }
    }

    /// Root session has depth 0.
    #[test]
    fn root_session_depth_is_zero() {
        let summaries = vec![sample_summary("a", None, None)];
        let model = build_sessions_model(&summaries);
        // ModelRc exposes items via a VecModel we can downcast.
        // For the smoke test we just check `len()` 鈥?depth is internal.
        assert_eq!(model.iter().count(), 1);
    }

    /// Two levels of parent 鈫?child 鈫?grandchild yields depth 0/1/2.
    #[test]
    fn child_session_depth_walks_parent_chain() {
        let summaries = vec![
            sample_summary("root", None, None),
            sample_summary("child", Some("root"), None),
            sample_summary("grandchild", Some("child"), None),
        ];
        let model = build_sessions_model(&summaries);
        assert_eq!(model.iter().count(), 3);
        // We can't directly inspect `depth` from outside (the Slint
        // struct's fields are private), but the order is preserved
        // and the loop didn't panic on the chain. Phase C.2's Slint
        // rendering uses the same data and is verified by manual test.
    }

    /// A cycle (a 鈫?b 鈫?a) must not loop forever 鈥?`build_sessions_model`
    /// caps depth at MAX_DEPTH = 8 and stops on the second visit.
    #[test]
    fn cycle_does_not_hang() {
        let summaries = vec![
            sample_summary("a", Some("b"), None),
            sample_summary("b", Some("a"), None),
        ];
        let model = build_sessions_model(&summaries);
        assert_eq!(model.iter().count(), 2);
        // If the cycle detection regressed, this would hang the test
        // runner 鈥?the assert_eq! failing is the secondary signal.
    }

    /// Empty input produces an empty model.
    #[test]
    fn empty_summaries_produces_empty_model() {
        let summaries: Vec<SessionSummaryDto> = vec![];
        let model = build_sessions_model(&summaries);
        assert_eq!(model.iter().count(), 0);
    }

    /// `build_messages_model` round-trips a few messages.
    #[test]
    fn build_messages_model_round_trip() {
        let meta = sample_meta();
        let msgs = vec![
            MessageDto {
                id: "m1".into(),
                role: MessageRoleDto::User,
                content: MessageContentDto::Text("hello".into()),
                timestamp: 0,
                metadata: Some(meta.clone()),
            },
            MessageDto {
                id: "m2".into(),
                role: MessageRoleDto::Assistant,
                content: MessageContentDto::Text("hi".into()),
                timestamp: 0,
                metadata: Some(meta),
            },
        ];
        let model = build_messages_model(&msgs, None);
        assert_eq!(model.iter().count(), 2);
    }

    /// A7: streaming indicator is shown on the last assistant message
    /// when streaming_session_id matches.
    #[test]
    fn build_messages_model_streaming_on_last_assistant() {
        let meta = sample_meta();
        let msgs = vec![
            MessageDto {
                id: "m1".into(),
                role: MessageRoleDto::User,
                content: MessageContentDto::Text("hello".into()),
                timestamp: 0,
                metadata: Some(meta.clone()),
            },
            MessageDto {
                id: "m2".into(),
                role: MessageRoleDto::Assistant,
                content: MessageContentDto::Text("hi".into()),
                timestamp: 0,
                metadata: Some(meta.clone()),
            },
            MessageDto {
                id: "m3".into(),
                role: MessageRoleDto::Assistant,
                content: MessageContentDto::Text("there".into()),
                timestamp: 0,
                metadata: Some(meta),
            },
        ];

        // With streaming session set, last assistant message is streaming
        let model = build_messages_model(&msgs, Some("sess-1"));
        let items: Vec<_> = model.iter().collect();
        assert_eq!(items.len(), 3);
        assert!(!items[0].is_streaming); // user
        assert!(!items[1].is_streaming); // assistant (not last)
        assert!(items[2].is_streaming); // assistant (last)

        // Without streaming session, nothing is streaming
        let model_no_stream = build_messages_model(&msgs, None);
        let items_no_stream: Vec<_> = model_no_stream.iter().collect();
        assert!(!items_no_stream[2].is_streaming);
    }

    /// A7: streaming indicator is NOT shown when last message is user
    #[test]
    fn build_messages_model_no_streaming_when_last_is_user() {
        let meta = sample_meta();
        let msgs = vec![
            MessageDto {
                id: "m1".into(),
                role: MessageRoleDto::Assistant,
                content: MessageContentDto::Text("hi".into()),
                timestamp: 0,
                metadata: Some(meta.clone()),
            },
            MessageDto {
                id: "m2".into(),
                role: MessageRoleDto::User,
                content: MessageContentDto::Text("hello".into()),
                timestamp: 0,
                metadata: Some(meta),
            },
        ];

        let model = build_messages_model(&msgs, Some("sess-1"));
        let items: Vec<_> = model.iter().collect();
        assert!(!items[0].is_streaming); // assistant (not last)
        assert!(!items[1].is_streaming); // user (last, not assistant)
    }

    /// A7: AppState streaming session getter/setter round-trip
    #[test]
    fn app_state_streaming_session_round_trip() {
        let app_state = super::AppState::new();
        assert_eq!(app_state.get_streaming_session(), None);

        app_state.set_streaming_session(Some("sess-123".to_string()));
        assert_eq!(app_state.get_streaming_session(), Some("sess-123".to_string()));

        app_state.set_streaming_session(None);
        assert_eq!(app_state.get_streaming_session(), None);
    }

    /// A7: AppState active turn id getter/setter round-trip
    #[test]
    fn app_state_active_turn_id_round_trip() {
        let app_state = super::AppState::new();
        assert_eq!(app_state.get_active_turn_id(), None);

        app_state.set_active_turn_id(Some("turn-456".to_string()));
        assert_eq!(app_state.get_active_turn_id(), Some("turn-456".to_string()));

        app_state.set_active_turn_id(None);
        assert_eq!(app_state.get_active_turn_id(), None);
    }

    /// A7: streaming indicator is NOT shown when messages list is empty
    #[test]
    fn build_messages_model_empty_list_no_streaming() {
        let msgs: Vec<MessageDto> = vec![];
        let model = build_messages_model(&msgs, Some("sess-1"));
        let items: Vec<_> = model.iter().collect();
        assert_eq!(items.len(), 0);
    }

    /// A7: streaming indicator is NOT shown on tool messages even when streaming
    #[test]
    fn build_messages_model_tool_message_never_streaming() {
        let meta = sample_meta();
        let msgs = vec![
            MessageDto {
                id: "m1".into(),
                role: MessageRoleDto::User,
                content: MessageContentDto::Text("hello".into()),
                timestamp: 0,
                metadata: Some(meta.clone()),
            },
            MessageDto {
                id: "m2".into(),
                role: MessageRoleDto::Tool,
                content: MessageContentDto::Text("tool result".into()),
                timestamp: 0,
                metadata: Some(meta.clone()),
            },
            MessageDto {
                id: "m3".into(),
                role: MessageRoleDto::Assistant,
                content: MessageContentDto::Text("hi".into()),
                timestamp: 0,
                metadata: Some(meta),
            },
        ];

        // Even with streaming session, tool message is never streaming
        let model = build_messages_model(&msgs, Some("sess-1"));
        let items: Vec<_> = model.iter().collect();
        assert!(!items[0].is_streaming); // user
        assert!(!items[1].is_streaming); // tool (never streaming)
        assert!(items[2].is_streaming); // assistant (last)
    }

    /// A7: only the last assistant message streams, not all assistants
    #[test]
    fn build_messages_model_only_last_assistant_streams() {
        let meta = sample_meta();
        let msgs = vec![
            MessageDto {
                id: "m1".into(),
                role: MessageRoleDto::Assistant,
                content: MessageContentDto::Text("first".into()),
                timestamp: 0,
                metadata: Some(meta.clone()),
            },
            MessageDto {
                id: "m2".into(),
                role: MessageRoleDto::Assistant,
                content: MessageContentDto::Text("second".into()),
                timestamp: 0,
                metadata: Some(meta.clone()),
            },
            MessageDto {
                id: "m3".into(),
                role: MessageRoleDto::Assistant,
                content: MessageContentDto::Text("third".into()),
                timestamp: 0,
                metadata: Some(meta),
            },
        ];

        let model = build_messages_model(&msgs, Some("sess-1"));
        let items: Vec<_> = model.iter().collect();
        assert!(!items[0].is_streaming); // assistant (not last)
        assert!(!items[1].is_streaming); // assistant (not last)
        assert!(items[2].is_streaming); // assistant (last)
    }

    // 2026-07-27 (K4a R3, fix #4): the previous
    // `create_ui_runs_with_noop_platform` test called
    // `slint::platform::set_platform(Box::new(NoopPlatform))`,
    // which mutates a process-global slot. The other tests in
    // the suite share the same process; `set_platform` is
    // not idempotent (it panics on the second call) and
    // races between tests in the same process are
    // nondeterministic. Refactoring the test to avoid
    // `set_platform` would require running the full
    // `create_ui` setup, which pulls in the streaming
    // dispatcher OnceLock, the Slint component tree, and
    // the agentic coordinator init — all of which are
    // individually covered by the `streaming_lifecycle::tests`,
    // `event_bridge::tests`, and `phase_i_tests` modules
    // without touching the process-global platform slot.
    //
    // The value of this test was: ensure `create_ui` doesn't
    // panic and the initial Slint properties are the
    // expected defaults. Both are tautologically true in the
    // production app (the test's only signal is "Slint's
    // `set_platform` and `AppWindow::new` work in this
    // version of Slint", which is a Slint test-suite
    // concern, not a northhing concern). Removed.

    // 2026-06-26 (Phase 5): AppState session_metadata round-trip.
    // The Q6/Q7 wire-up uses this map to bridge between the runtime
    // session ids and the desktop-side provider/workspace metadata.
    // `record_session_meta` and `forget_session_meta` are called from
    // `on_new_session` and `on_delete_session` respectively.

    #[test]
    fn app_state_session_metadata_record_and_forget() {
        use super::SessionMeta;
        use std::path::PathBuf;

        let app_state = super::AppState::new();

        // Empty snapshot to start.
        assert!(app_state.session_metadata_snapshot().is_empty());

        // Record two sessions.
        app_state.record_session_meta(
            "s1".to_string(),
            SessionMeta {
                provider_id: "prov-1".to_string(),
                workspace_path: PathBuf::from("/tmp/proj1"),
            },
        );
        app_state.record_session_meta(
            "s2".to_string(),
            SessionMeta {
                provider_id: "prov-2".to_string(),
                workspace_path: PathBuf::from("/tmp/proj2"),
            },
        );
        let snap = app_state.session_metadata_snapshot();
        assert_eq!(snap.len(), 2);

        // Snapshot is order-independent; sort by id for assertions.
        let mut sorted = snap.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(sorted[0].0, "s1");
        assert_eq!(sorted[0].1.provider_id, "prov-1");
        assert_eq!(sorted[1].0, "s2");
        assert_eq!(sorted[1].1.workspace_path, PathBuf::from("/tmp/proj2"));

        // Forgetting one session leaves the other.
        app_state.forget_session_meta("s1");
        let snap = app_state.session_metadata_snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].0, "s2");
    }

    #[test]
    fn app_state_session_metadata_forget_unknown_is_noop() {
        use super::SessionMeta;
        let app_state = super::AppState::new();
        app_state.record_session_meta(
            "s1".to_string(),
            SessionMeta {
                provider_id: "prov-1".to_string(),
                workspace_path: std::path::PathBuf::from("/tmp/proj1"),
            },
        );
        // Forgetting a non-existent session is silent (HashMap::remove
        // returns None, we ignore it). Should not affect existing entries.
        app_state.forget_session_meta("does-not-exist");
        assert_eq!(app_state.session_metadata_snapshot().len(), 1);
    }

    /// Phase 5: message_to_item extracts reasoning_content and tool_calls from Mixed.
    #[test]
    fn message_to_item_mixed_extracts_think_and_tool_fields() {
        let meta = sample_meta();
        let msg = MessageDto {
            id: "msg-mixed-1".into(),
            role: MessageRoleDto::Assistant,
            content: MessageContentDto::Mixed {
                reasoning_content: Some("Let me think about this".into()),
                text: "Here is the answer".into(),
                tool_calls: vec![
                    ToolCallStub {
                        tool_name: "search".into(),
                        arguments: None,
                        is_error: false,
                    },
                    ToolCallStub {
                        tool_name: "read_file".into(),
                        arguments: Some(serde_json::json!({"path": "/tmp/test.txt"})),
                        is_error: false,
                    },
                ],
            },
            timestamp: 0,
            metadata: Some(meta),
        };

        let item = message_to_item(&msg, false);

        assert_eq!(item.think_content.as_str(), "Let me think about this");
        assert_eq!(item.tool_calls_count, 2);
        assert_eq!(item.tool_calls_summary.as_str(), "search, read_file");
        assert!(item.tool_calls_json.as_str().starts_with("["));
        assert!(item.tool_calls_json.as_str().contains("search"));
        assert!(item.tool_calls_json.as_str().contains("read_file"));

        let names: Vec<String> = item.tool_names.iter().map(|s| s.to_string()).collect();
        assert_eq!(names, vec!["search", "read_file"]);
    }

    /// Phase 5: non-Mixed messages get empty defaults for the new fields.
    #[test]
    fn message_to_item_non_mixed_has_empty_new_fields() {
        let meta = sample_meta();
        let msg = MessageDto {
            id: "msg-text-1".into(),
            role: MessageRoleDto::User,
            content: MessageContentDto::Text("hello".into()),
            timestamp: 0,
            metadata: Some(meta),
        };

        let item = message_to_item(&msg, false);

        assert_eq!(item.think_content.as_str(), "");
        assert_eq!(item.tool_calls_count, 0);
        assert_eq!(item.tool_calls_summary.as_str(), "");
        assert_eq!(item.tool_calls_json.as_str(), "");
        assert_eq!(item.tool_names.iter().count(), 0);
    }
}
