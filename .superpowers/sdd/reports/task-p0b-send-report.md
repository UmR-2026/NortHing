# Task P0b Report — F2 消息发送/stop/streaming 接线

- **Status**: DONE
- **Brief**: `.superpowers/sdd/consult-room/task-p0b-send-brief.md`
- **Base commit**: `8703901`

## Files Changed

1. `src/apps/desktop/src/ui_dioxus/api.rs`
   - Added `ensure_room_session() -> Result<SessionId, KernelError>`: returns existing session from `list_sessions()` or creates a new one via `create_session(SessionConfigDto { workspace_path: None, agent_type: "agentic".into(), model_name: "default".into(), name: Some("诊室".into()) })`.
   - Added `test_ensure_room_session_fails_cleanly_when_uninitialized` unit test.
2. `src/apps/desktop/src/ui_dioxus/app.rs`
   - Replaced placeholder `div.input-box` and `span.cursor` with interactive Dioxus `input` element (`value`, `oninput`, `onkeydown` with `!e.is_composing() && e.key() == Key::Enter` guard).
   - Wired send / stop combined button and action handlers: non-streaming submits turn via `api::ensure_room_session()` + `api::submit_turn(&sid, text)` and adds `MockEntry::Witness` bubble; streaming invokes `api::stop_turn(&turn_id)`.
   - Added `use_future` consuming `api::event_channel()`: accumulates `KernelEventDto::TextChunk` into `assistant_draft`, settles terminal `TurnState` (`Completed`, `Failed`, `Cancelled`) into `MockEntry::Entity` (clearing draft and setting `streaming` to false).
   - Rendered real-time `assistant_draft` as an entity message in `chat-flow` while streaming.
   - Preserved all forbidden zones: no changes to `session_mock.rs`, `entry.rs`, other module pages, or i18n keys.

## Deviations from Brief

None.

## Verification Command Outputs

### 1. `cargo check -p northhing --features ui-dioxus`

```
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cd E:\agent-project\northing
cargo check -p northhing --features ui-dioxus

warning: private item shadows public glob re-export
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: the name `prompt_cache` in the type namespace is supposed to be publicly re-exported here
  --> src\crates\assembly\core\src\agentic\session\mod.rs:34:9
   |
34 | pub use facade::*;
   |         ^^^^^^^^^
note: but the private item here shadows it
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: `#[warn(hidden_glob_reexports)]` on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:295:9
    |
295 |     let mut command_started_after_ms: Option<u64> = None;
    |         ----^^^^^^^^^^^^^^^^^^^^^^^^
    |         |
    |         help: remove this `mut`
    |
    = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_input.rs:191:9
    |
191 |     let mut timeout_seconds = match input.get("timeout_seconds") {
    |         ----^^^^^^^^^^^^^^^
    |         |
    |         help: remove this `mut`

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:63:13
   |
63 |         let mut turn_id = ctx.final_turn_id.clone();
   |             ----^^^^^^^
   |             |
   |             help: remove this `mut`

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:35:13
   |
35 |         let mut extra_user_message_metadata = ctx.extra_user_message_metadata.clone();
   |             ----^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |             |
   |             help: remove this `mut`

warning: unused variable: `port`
   --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser.rs:137:13
    |
137 |         let port = params
    |             ^^^^ help: if this is intentional, prefix it with an underscore: `_port`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `actions`
  --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser_telemetry.rs:26:13
   |
26 |         let actions = BrowserActions::new(session.client.as_ref());
   |             ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_actions`

warning: unused variable: `deep_review_subagent_role`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:80:5
   |
80 |     deep_review_subagent_role: Option<crate::agentic::deep_review_policy::DeepReviewSubagentRole>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_deep_review_subagent_role`

warning: unused variable: `is_retry`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:84:5
   |
84 |     is_retry: bool,
   |     ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_is_retry`

warning: unused variable: `suppress_session_title_generation`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:34:13
   |
34 |         let suppress_session_title_generation = ctx.suppress_session_title_generation;
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_suppress_session_title_generation`

warning: unused variable: `turn_index`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:39:13
   |
39 |         let turn_index = ctx.turn_index;
   |             ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_turn_index`

warning: unused variable: `workspace_turn_status`
   --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:372:17
    |
372 |             let workspace_turn_status = tokio::select! {
    |                 ^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_workspace_turn_status`

warning: unused variable: `active_counter`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:67:13
   |
67 |         let active_counter = Arc::new(AtomicUsize::new(0));
   |             ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_active_counter`

warning: unused variable: `ws`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:236:36
    |
236 |         let mut stmt = if let Some(ws) = workspace_key {
    |                                    ^^ help: if this is intentional, prefix it with an underscore: `_ws`

warning: unused variable: `last_mentioned_at`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:291:80
    |
291 |             let (id, text, scope, confidence, session_id, turn_id, created_at, last_mentioned_at, fact_type) =
    |                                                                                ^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_last_mentioned_at`

warning: unused variable: `at_ms`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:743:85
    |
743 |     pub(crate) fn supersede_fact(&self, fact_id: &str, superseded_by: Option<&str>, at_ms: u64) -> NortHingResult<()> {
    |                                                                                     ^^^^^ help: if this is intentional, prefix it with an underscore: `_at_ms`

warning: unused variable: `ws`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db\dream.rs:17:36
   |
17 |         let mut stmt = if let Some(ws) = workspace_key {
   |                                    ^^ help: if this is intentional, prefix it with an underscore: `_ws`

warning: unused variable: `params`
   --> src\crates\assembly\core\src\service\mcp\server\manager\interaction.rs:104:9
    |
104 |         params: Option<Value>,
    |         ^^^^^^ help: if this is intentional, prefix it with an underscore: `_params`

warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
warning: unused imports: `PhysicalPosition`, `PhysicalSize`, and `Window`
 --> src\apps\desktop\src\app_state\block_registry.rs:1:30
  |
1 | use slint::{ComponentHandle, PhysicalPosition, PhysicalSize, Window};
  |                              ^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^  ^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `Mutex`
 --> src\apps\desktop\src\app_state\block_registry.rs:3:22
  |
3 | use std::sync::{Arc, Mutex};
  |                      ^^^^^

warning: unused import: `tokio::time::interval`
 --> src\apps\desktop\src\app_state\block_registry.rs:4:5
  |
4 | use tokio::time::interval;
  |     ^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `dioxus::desktop::tao::platform::windows::WindowExtWindows`
  --> src\apps\desktop\src\ui_dioxus\pages_archive.rs:18:5
   |
18 | use dioxus::desktop::tao::platform::windows::WindowExtWindows;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `slint::platform::WindowAdapter`
 --> src\apps\desktop\src\app_state\block_registry.rs:2:5
  |
2 | use slint::platform::WindowAdapter;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `HasWindowHandle`
 --> src\apps\desktop\src\app_state\block_registry.rs:5:25
  |
5 | use raw_window_handle::{HasWindowHandle, RawWindowHandle};
  |                         ^^^^^^^^^^^^^^^

warning: function `get_session` is never used
  --> src\apps\desktop\src\ui_dioxus\api.rs:54:14
   |
54 | pub async fn get_session(id: &SessionId) -> Result<SessionDto, KernelError> {
   |              ^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: function `respond_to_tool_confirmation` is never used
  --> src\apps\desktop\src\ui_dioxus\api.rs:74:14
   |
74 | pub async fn respond_to_tool_confirmation(
   |              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: type alias `GeometryRx` is never used
  --> src\apps\desktop\src\ui_dioxus\state.rs:30:10
   |
30 | pub type GeometryRx = watch::Receiver<Geometry>;
   |          ^^^^^^^^^^

warning: method `is_any_active` is never used
   --> src\apps\desktop\src\ui_dioxus\registry.rs:188:12
    |
167 | impl ShellWindowManager {
    | ----------------------- method in this implementation
...
188 |     pub fn is_any_active(&self, ids: &[&str]) -> bool {
    |            ^^^^^^^^^^^^^

warning: function `inject_stylesheet_html` is never used
   --> src\apps\desktop\src\ui_dioxus\css.rs:754:8
    |
754 | pub fn inject_stylesheet_html() -> String {
    |        ^^^^^^^^^^^^^^^^^^^^^^

warning: field `locale` is never read
  --> src\apps\desktop\src\ui_dioxus\i18n.rs:29:5
   |
27 | pub struct LocalePack {
   |            ---------- field in this struct
28 |     by_key: HashMap<String, String>,
29 |     locale: String,
   |     ^^^^^^
   |
   = note: `LocalePack` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis

warning: method `locale` is never used
  --> src\apps\desktop\src\ui_dioxus\i18n.rs:81:12
   |
32 | impl LocalePack {
   | --------------- method in this implementation
...
81 |     pub fn locale(&self) -> &str {
   |            ^^^^^^

warning: constant `WINDOW_TITLE_INNER` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:142:15
    |
142 |     pub const WINDOW_TITLE_INNER: &str = "dioxus-room-inner-window-title";
    |               ^^^^^^^^^^^^^^^^^^

warning: constant `WINDOW_TITLE_OUTER` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:143:15
    |
143 |     pub const WINDOW_TITLE_OUTER: &str = "dioxus-room-outer-window-title";
    |               ^^^^^^^^^^^^^^^^^^

warning: constant `STATE_PILL_DRIVE` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:144:15
    |
144 |     pub const STATE_PILL_DRIVE: &str = "dioxus-room-state-drive";
    |               ^^^^^^^^^^^^^^^^

warning: constant `STATUS_IDENTITY` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:147:15
    |
147 |     pub const STATUS_IDENTITY: &str = "dioxus-room-status-identity";
    |               ^^^^^^^^^^^^^^^

warning: constant `STATUS_CONTEXT` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:148:15
    |
148 |     pub const STATUS_CONTEXT: &str = "dioxus-room-status-context";
    |               ^^^^^^^^^^^^^^

warning: constant `AGENT_WHO` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:158:15
    |
158 |     pub const AGENT_WHO: &str = "dioxus-room-agent-who";
    |               ^^^^^^^^^

warning: constant `AGENT_BODY` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:159:15
    |
159 |     pub const AGENT_BODY: &str = "dioxus-room-agent-body";
    |               ^^^^^^^^^^

warning: constant `AGENT_TOOL_LOG` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:160:15
    |
160 |     pub const AGENT_TOOL_LOG: &str = "dioxus-room-agent-tool-log";
    |               ^^^^^^^^^^^^^^

warning: constant `AGENT_ARTIFACT_CHIP` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:161:15
    |
161 |     pub const AGENT_ARTIFACT_CHIP: &str = "dioxus-room-agent-artifact-chip";
    |               ^^^^^^^^^^^^^^^^^^^

warning: constant `AGENT_BODY_2` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:162:15
    |
162 |     pub const AGENT_BODY_2: &str = "dioxus-room-agent-body-2";
    |               ^^^^^^^^^^^^

warning: constant `WITNESS_WHO` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:163:15
    |
163 |     pub const WITNESS_WHO: &str = "dioxus-room-witness-who";
    |               ^^^^^^^^^^^

warning: constant `WITNESS_BODY` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:164:15
    |
164 |     pub const WITNESS_BODY: &str = "dioxus-room-witness-body";
    |               ^^^^^^^^^^^^

warning: constant `APPROVAL_HEAD` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:165:15
    |
165 |     pub const APPROVAL_HEAD: &str = "dioxus-room-approval-head";
    |               ^^^^^^^^^^^^^

warning: constant `APPROVAL_MAIN` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:166:15
    |
166 |     pub const APPROVAL_MAIN: &str = "dioxus-room-approval-main";
    |               ^^^^^^^^^^^^^

warning: constant `APPROVAL_RISK` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:167:15
    |
167 |     pub const APPROVAL_RISK: &str = "dioxus-room-approval-risk";
    |               ^^^^^^^^^^^^^

warning: constant `APPROVAL_HEAD_2` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:170:15
    |
170 |     pub const APPROVAL_HEAD_2: &str = "dioxus-room-approval-head-2";
    |               ^^^^^^^^^^^^^^^

warning: constant `APPROVAL_MAIN_2` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:171:15
    |
171 |     pub const APPROVAL_MAIN_2: &str = "dioxus-room-approval-main-2";
    |               ^^^^^^^^^^^^^^^

warning: constant `APPROVAL_STATE` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:172:15
    |
172 |     pub const APPROVAL_STATE: &str = "dioxus-room-approval-state";
    |               ^^^^^^^^^^^^^^

warning: constant `OUTER_TERMINAL_PROMPT` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:231:15
    |
231 |     pub const OUTER_TERMINAL_PROMPT: &str = "dioxus-room-outer-terminal-prompt";
    |               ^^^^^^^^^^^^^^^^^^^^^

warning: constant `EMPTY_CHAT_FLOW` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:234:15
    |
234 |     pub const EMPTY_CHAT_FLOW: &str = "dioxus-room-empty-chat-flow";
    |               ^^^^^^^^^^^^^^^

warning: constant `EMPTY_STREAMING_INTERRUPT` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:235:15
    |
235 |     pub const EMPTY_STREAMING_INTERRUPT: &str = "dioxus-room-empty-streaming-interrupt";
    |               ^^^^^^^^^^^^^^^^^^^^^^^^^

warning: constant `EMPTY_PROVIDER_TEST_FAILED` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:236:15
    |
236 |     pub const EMPTY_PROVIDER_TEST_FAILED: &str = "dioxus-room-empty-provider-test-failed";
    |               ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: constant `EMPTY_APPROVAL_TIMEOUT` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:237:15
    |
237 |     pub const EMPTY_APPROVAL_TIMEOUT: &str = "dioxus-room-empty-approval-timeout";
    |               ^^^^^^^^^^^^^^^^^^^^^^

warning: `northhing` (bin "northhing") generated 36 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 36.08s
```

### 2. `cargo test -p northhing --features ui-dioxus --lib ui_dioxus`

```
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
$env:PATH = "C:\msys64\mingw64\bin;$env:PATH"
cd E:\agent-project\northing
cargo test -p northhing --features ui-dioxus --lib ui_dioxus

warning: private item shadows public glob re-export
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: the name `prompt_cache` in the type namespace is supposed to be publicly re-exported here
  --> src\crates\assembly\core\src\agentic\session\mod.rs:34:9
   |
34 | pub use facade::*;
   |         ^^^^^^^^^
note: but the private item here shadows it
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: `#[warn(hidden_glob_reexports)]` on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:295:9
    |
295 |     let mut command_started_after_ms: Option<u64> = None;
    |         ----^^^^^^^^^^^^^^^^^^^^^^^^
    |         |
    |         help: remove this `mut`
    |
    = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_input.rs:191:9
    |
191 |     let mut timeout_seconds = match input.get("timeout_seconds") {
    |         ----^^^^^^^^^^^^^^^
    |         |
    |         help: remove this `mut`

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:63:13
   |
63 |         let mut turn_id = ctx.final_turn_id.clone();
   |             ----^^^^^^^
   |             |
   |             help: remove this `mut`

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:35:13
   |
35 |         let mut extra_user_message_metadata = ctx.extra_user_message_metadata.clone();
   |             ----^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |             |
   |             help: remove this `mut`

warning: unused variable: `port`
   --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser.rs:137:13
    |
137 |         let port = params
    |             ^^^^ help: if this is intentional, prefix it with an underscore: `_port`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `actions`
  --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser_telemetry.rs:26:13
   |
26 |         let actions = BrowserActions::new(session.client.as_ref());
   |             ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_actions`

warning: unused variable: `deep_review_subagent_role`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:80:5
   |
80 |     deep_review_subagent_role: Option<crate::agentic::deep_review_policy::DeepReviewSubagentRole>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_deep_review_subagent_role`

warning: unused variable: `is_retry`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:84:5
   |
84 |     is_retry: bool,
   |     ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_is_retry`

warning: unused variable: `suppress_session_title_generation`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:34:13
   |
34 |         let suppress_session_title_generation = ctx.suppress_session_title_generation;
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_suppress_session_title_generation`

warning: unused variable: `turn_index`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:39:13
   |
39 |         let turn_index = ctx.turn_index;
   |             ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_turn_index`

warning: unused variable: `workspace_turn_status`
   --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:372:17
    |
372 |             let workspace_turn_status = tokio::select! {
    |                 ^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_workspace_turn_status`

warning: unused variable: `active_counter`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:67:13
   |
67 |         let active_counter = Arc::new(AtomicUsize::new(0));
   |             ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_active_counter`

warning: unused variable: `ws`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:236:36
    |
236 |         let mut stmt = if let Some(ws) = workspace_key {
    |                                    ^^ help: if this is intentional, prefix it with an underscore: `_ws`

warning: unused variable: `last_mentioned_at`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:291:80
    |
291 |             let (id, text, scope, confidence, session_id, turn_id, created_at, last_mentioned_at, fact_type) =
    |                                                                                ^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_last_mentioned_at`

warning: unused variable: `at_ms`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:743:85
    |
743 |     pub(crate) fn supersede_fact(&self, fact_id: &str, superseded_by: Option<&str>, at_ms: u64) -> NortHingResult<()> {
    |                                                                                     ^^^^^ help: if this is intentional, prefix it with an underscore: `_at_ms`

warning: unused variable: `ws`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db\dream.rs:17:36
   |
17 |         let mut stmt = if let Some(ws) = workspace_key {
   |                                    ^^ help: if this is intentional, prefix it with an underscore: `_ws`

warning: unused variable: `params`
   --> src\crates\assembly\core\src\service\mcp\server\manager\interaction.rs:104:9
    |
104 |         params: Option<Value>,
    |         ^^^^^^ help: if this is intentional, prefix it with an underscore: `_params`

warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
   Compiling northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 45.18s
     Running unittests src\lib.rs (target\debug\deps\northhing-998dc8915b11e80b.exe)

running 11 tests
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok
test ui_dioxus::api::tests::test_event_channel_returns_receiver ... ok
test ui_dioxus::registry::tests::test_settings_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_onboarding_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
test ui_dioxus::api::tests::test_api_functions_fail_cleanly_before_init ... ok
test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
test ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized ... ok
test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok
test ui_dioxus::registry::tests::test_space_registration_and_lifecycle ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 110 filtered out; finished in 0.00s
```
