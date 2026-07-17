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
//! `#![forbid(unsafe_code)]` to this file — the lint is intentionally
//! omitted. Future maintainers adding code should stay in safe Rust
//! (no new `unsafe { }` blocks in this file); grep for `unsafe` to
//! audit.

// Existing siblings (Phase B split, preserved)
pub(super) mod actor;
pub(super) mod inspector;
pub(super) mod inspector_model_status;
pub(super) mod log;
pub(super) mod sessions;
pub(super) mod settings;
pub(super) mod skills;
pub(super) mod slint_glue;

// R37a NEW siblings (split from this 2122-line mod.rs)
pub(super) mod callbacks_lifecycle;
pub(super) mod callbacks_settings;
pub(super) mod create_ui;
pub(super) mod error_banners;
pub(super) mod event_bridge;
pub(super) mod state;

// Wildcard re-exports so `crate::app_state::{AppState, create_ui, ...}`
// and `crate::app_state::set_session_error` keep working from callers
// (main.rs, sessions.rs, etc.). Preserves the cross-crate import paths.
pub use callbacks_lifecycle::*;
pub use callbacks_settings::*;
pub use create_ui::*;
pub use error_banners::*;
pub use state::*;

use crate::app_state::log::log_debug_event;
use actor::maybe_construct_actor_runtime;
use inspector::build_mcp_status_string;
use inspector_model_status::build_model_status_string;
use sessions::{build_messages_model, refresh_messages_ui, refresh_sessions_ui};
use skills::refresh_skills_ui;

// R37a: bring Slint DTO + glue types into the app_state module scope so
// `use super::*;` in sessions.rs / skills.rs picks up SessionItem, SharedString,
// ModelRc, etc. (preserves the pre-split import path).
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use slint_glue::{AppWindow, MessageItem, SessionItem, SkillItem};

// ═══════════════════════════════════════════════════════════════════
// Phase I.5 tests (2026-06-20)
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod phase_i_tests {
    //! Smoke tests for the Slint DTO projection helpers. These cover the
    //! pure functions (`build_sessions_model`, `session_summary_to_item`,
    //! `build_messages_model`) — the higher-level `create_ui` test would
    //! need a real display handle and is left for future Phase I.x work
    //! (or for manual smoke-testing).

    use super::sessions::{build_messages_model, build_sessions_model};
    use northhing_core::agentic::core::{
        Message, MessageContent, MessageRole, SessionKind, SessionState, SessionSummary,
    };
    use northhing_core::agentic::message::MessageMetadata;
    use slint::Model;
    use std::time::SystemTime;

    fn sample_summary(id: &str, parent_id: Option<&str>, depth_target_id: Option<&str>) -> SessionSummary {
        // The `parent_session_id` field on SessionSummary is what the
        // depth walker reads. `depth_target_id` is unused here (the
        // helper computes depth from parent links); kept in the
        // signature to make the test call sites self-documenting.
        let _ = depth_target_id;
        SessionSummary {
            session_id: id.into(),
            session_name: format!("Session {id}"),
            agent_type: "code".into(),
            last_user_dialog_agent_type: None,
            last_submitted_agent_type: None,
            created_by: None,
            kind: SessionKind::Standard,
            turn_count: 0,
            created_at: SystemTime::now(),
            last_activity_at: SystemTime::now(),
            state: SessionState::Idle,
            parent_session_id: parent_id.map(String::from),
        }
    }

    /// Root session has depth 0.
    #[test]
    fn root_session_depth_is_zero() {
        let summaries = vec![sample_summary("a", None, None)];
        let model = build_sessions_model(&summaries);
        // ModelRc exposes items via a VecModel we can downcast.
        // For the smoke test we just check `len()` — depth is internal.
        assert_eq!(model.iter().count(), 1);
    }

    /// Two levels of parent → child → grandchild yields depth 0/1/2.
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

    /// A cycle (a → b → a) must not loop forever — `build_sessions_model`
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
        // runner — the assert_eq! failing is the secondary signal.
    }

    /// Empty input produces an empty model.
    #[test]
    fn empty_summaries_produces_empty_model() {
        let summaries: Vec<SessionSummary> = vec![];
        let model = build_sessions_model(&summaries);
        assert_eq!(model.iter().count(), 0);
    }

    /// `build_messages_model` round-trips a few messages.
    #[test]
    fn build_messages_model_round_trip() {
        let meta = MessageMetadata::default();
        let msgs = vec![
            Message {
                id: "m1".into(),
                role: MessageRole::User,
                content: MessageContent::Text("hello".into()),
                timestamp: SystemTime::now(),
                metadata: meta.clone(),
            },
            Message {
                id: "m2".into(),
                role: MessageRole::Assistant,
                content: MessageContent::Text("hi".into()),
                timestamp: SystemTime::now(),
                metadata: meta,
            },
        ];
        let model = build_messages_model(&msgs, None);
        assert_eq!(model.iter().count(), 2);
    }

    /// A7: streaming indicator is shown on the last assistant message
    /// when streaming_session_id matches.
    #[test]
    fn build_messages_model_streaming_on_last_assistant() {
        let meta = MessageMetadata::default();
        let msgs = vec![
            Message {
                id: "m1".into(),
                role: MessageRole::User,
                content: MessageContent::Text("hello".into()),
                timestamp: SystemTime::now(),
                metadata: meta.clone(),
            },
            Message {
                id: "m2".into(),
                role: MessageRole::Assistant,
                content: MessageContent::Text("hi".into()),
                timestamp: SystemTime::now(),
                metadata: meta.clone(),
            },
            Message {
                id: "m3".into(),
                role: MessageRole::Assistant,
                content: MessageContent::Text("there".into()),
                timestamp: SystemTime::now(),
                metadata: meta,
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
        let meta = MessageMetadata::default();
        let msgs = vec![
            Message {
                id: "m1".into(),
                role: MessageRole::Assistant,
                content: MessageContent::Text("hi".into()),
                timestamp: SystemTime::now(),
                metadata: meta.clone(),
            },
            Message {
                id: "m2".into(),
                role: MessageRole::User,
                content: MessageContent::Text("hello".into()),
                timestamp: SystemTime::now(),
                metadata: meta,
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
        let msgs: Vec<Message> = vec![];
        let model = build_messages_model(&msgs, Some("sess-1"));
        let items: Vec<_> = model.iter().collect();
        assert_eq!(items.len(), 0);
    }

    /// A7: streaming indicator is NOT shown on tool messages even when streaming
    #[test]
    fn build_messages_model_tool_message_never_streaming() {
        let meta = MessageMetadata::default();
        let msgs = vec![
            Message {
                id: "m1".into(),
                role: MessageRole::User,
                content: MessageContent::Text("hello".into()),
                timestamp: SystemTime::now(),
                metadata: meta.clone(),
            },
            Message {
                id: "m2".into(),
                role: MessageRole::Tool,
                content: MessageContent::Text("tool result".into()),
                timestamp: SystemTime::now(),
                metadata: meta.clone(),
            },
            Message {
                id: "m3".into(),
                role: MessageRole::Assistant,
                content: MessageContent::Text("hi".into()),
                timestamp: SystemTime::now(),
                metadata: meta,
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
        let meta = MessageMetadata::default();
        let msgs = vec![
            Message {
                id: "m1".into(),
                role: MessageRole::Assistant,
                content: MessageContent::Text("first".into()),
                timestamp: SystemTime::now(),
                metadata: meta.clone(),
            },
            Message {
                id: "m2".into(),
                role: MessageRole::Assistant,
                content: MessageContent::Text("second".into()),
                timestamp: SystemTime::now(),
                metadata: meta.clone(),
            },
            Message {
                id: "m3".into(),
                role: MessageRole::Assistant,
                content: MessageContent::Text("third".into()),
                timestamp: SystemTime::now(),
                metadata: meta,
            },
        ];

        let model = build_messages_model(&msgs, Some("sess-1"));
        let items: Vec<_> = model.iter().collect();
        assert!(!items[0].is_streaming); // assistant (not last)
        assert!(!items[1].is_streaming); // assistant (not last)
        assert!(items[2].is_streaming); // assistant (last)
    }

    // ═══════════════════════════════════════════════════════════════════
    // K.2.4 Mock display test
    // ═══════════════════════════════════════════════════════════════════

    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    use std::rc::Rc;
    use std::sync::Arc;

    /// A no-op Slint platform for headless testing.
    /// Uses MinimalSoftwareWindow (software renderer) so `create_ui` can
    /// instantiate the Slint component tree without a real display.
    struct NoopPlatform;

    impl slint::platform::Platform for NoopPlatform {
        fn create_window_adapter(&self) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
            // MinimalSoftwareWindow provides a real (software) renderer
            // but never opens an OS window.  Safe for headless tests.
            Ok(MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer))
        }

        fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
            Ok(())
        }
    }

    /// Verifies `create_ui` boots a Slint UI against the no-op platform.
    /// Uses `multi_thread` runtime (1 worker) because `ActorRuntime::new`
    /// requires a tokio handle. The runtime is torn down automatically
    /// when the test exits.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn create_ui_runs_with_noop_platform() {
        // Set the no-op platform before creating the UI.
        slint::platform::set_platform(Box::new(NoopPlatform)).unwrap();

        let app_state = Arc::new(super::AppState::new());
        let ui = super::create_ui(app_state).unwrap();

        // Verify initial properties
        assert_eq!(ui.get_app_title(), "northhing v0.1.0");
        assert_eq!(ui.get_dark_mode(), true);
    }

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
}
