# Task P0c Implementation Report — F3-UI approval 卡接线（含事件方向缺口补齐）

## Status
DONE

## Files Changed
- `src/crates/contracts/kernel-api/src/events.rs`:
  - `ToolCallPhase` 枚举追加 `AwaitingConfirmation` 变体（derive 追加 `Copy, PartialEq, Eq`）
  - 增加 `ToolCallPhase::AwaitingConfirmation` 序列化/反序列化单测
- `src/crates/assembly/core/src/kernel_facade/events.rs`:
  - 在 `agentic_event_to_dtos` 中为 `ToolEventData::ConfirmationNeeded` 增加映射 arm，产出 `phase = ToolCallPhase::AwaitingConfirmation` 的 `ToolCallDto`（不发 `TurnPhase`）
- `src/crates/assembly/core/src/kernel_facade/tests.rs`:
  - 增加单测 `test_agentic_event_to_dtos_confirmation_needed_maps_to_awaiting_confirmation`
- `src/apps/desktop/src/ui_dioxus/session_mock.rs`:
  - `MockEntry::Approval` 增加 `call_id: String` 字段
  - `seed_session()` 填充 `"mock-call-1"` / `"mock-call-2"` 占位 `call_id`
  - 增加单测 `test_seed_session_has_mock_approvals_with_call_ids`
- `src/apps/desktop/src/ui_dioxus/app.rs`:
  - `use_future` 事件监听增加 `KernelEventDto::ToolCall(tc) if tc.phase == ToolCallPhase::AwaitingConfirmation` 处理分支，按 `call_id` 去重推入未决 approval 卡
  - `render_entries` / `render_entry` 传入 `entries: Signal<Vec<MockEntry>>`
  - 为未决 approval 卡的 Approve / Reject 按钮接线：点击触发 `spawn(api::respond_to_tool_confirmation(&cid, approved))`，成功后按 `call_id` 乐观定位更新卡片 `resolved = true` 与对应状态文本

## Deviations
无偏离。完全遵循 brief §0 - §4 要求。

## Verification Outputs (Verbatim)

### 1. `cargo check --workspace`
```text
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
warning: unused imports: `QuestionData` and `QuestionOption`
  --> src\apps\cli\src\ui\question\mod.rs:15:33
   |
15 | pub use types::{QuestionAction, QuestionData, QuestionOption, QuestionPrompt};
   |                                 ^^^^^^^^^^^^  ^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
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

warning: `northhing` (bin "northhing") generated 35 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 51.28s
```

### 2. `cargo check -p northhing --features ui-dioxus`
```text
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

warning: `northhing` (bin "northhing") generated 35 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 08s
```

### 3. `cargo test -p northhing-core --features product-full kernel_facade`
```text
running 37 tests
test kernel_facade::settings::tests::test_form_to_model_config_uses_provider_type_when_present ... ok
test kernel_facade::settings::tests::test_form_to_model_config_falls_back_to_provider_id_when_none ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_text_chunk_produces_text_and_phase ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_thinking_chunk_produces_phase_only ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_dialog_turn_started_produces_state_and_phase ... ok
test kernel_facade::tests::test_dialog_turn_failed_auth_is_fatal ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_completed_result_fallback ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_completed_summary_and_detail ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_failed_maps_to_completed_phase ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_started_summary_from_command ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_cancelled_summary_with_prefix_truncated_to_120 ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_tool_started_carries_tool_name ... ok
test kernel_facade::tests::test_dialog_turn_failed_no_category_is_fatal ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_completed_truncation_at_120 ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_started_summary_fallback ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_confirmation_needed_maps_to_awaiting_confirmation ... ok
test kernel_facade::tests::test_facade_construction_no_panic ... ok
test kernel_facade::tests::test_first_line_truncated ... ok
test kernel_facade::tests::test_backward_compat_deserialization_missing_new_fields ... ok
test kernel_facade::tests::test_session_config_dto_name_round_trip ... ok
test kernel_facade::tests::test_dialog_turn_failed_network_is_recoverable ... ok
test kernel_facade::tests::test_outcome_to_dto_started_and_queued ... ok
test kernel_facade::tests::test_message_to_dto_carries_timestamp ... ok
test kernel_facade::tests::test_tool_completed_result_count_object_is_none ... ok
test kernel_facade::tests::test_result_methods_return_error_before_init ... ok
test kernel_facade::tests::test_summary_to_dto_carries_parent_and_state ... ok
test kernel_facade::tests::test_tool_completed_result_count_array ... ok
test kernel_facade::tests::test_truncate_4000 ... ok
test kernel_facade::turn::tests::turn_lookup_matches_active_and_queued_turn_ids ... ok
test kernel_facade::tools::tests::test_respond_to_tool_confirmation_returns_runtime_err_before_init ... ok
test kernel_facade::tests::test_list_tools_returns_err_before_init ... ok
test kernel_facade::tests::test_subscribe_events_returns_err_before_init ... ok
test kernel_facade::tests::test_list_episodes_nonexistent_slug_returns_empty_vec ... ok
test kernel_facade::tests::test_list_episodes_dto_fields_are_correct ... ok
test kernel_facade::tests::test_list_tools_ordering_and_degraded_description ... ok
test kernel_facade::tests::test_list_tools_single_tool_field_mapping ... ok
test kernel_facade::tests::test_init_gate_lifecycle_all_scenarios ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 1012 filtered out; finished in 0.09s
```

### 4. `cargo test -p northhing --features ui-dioxus --lib ui_dioxus`
```text
running 12 tests
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok
test ui_dioxus::api::tests::test_event_channel_returns_receiver ... ok
test ui_dioxus::session_mock::tests::test_seed_session_has_mock_approvals_with_call_ids ... ok
test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
test ui_dioxus::registry::tests::test_space_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_onboarding_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
test ui_dioxus::registry::tests::test_settings_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok
test ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized ... ok
test ui_dioxus::api::tests::test_api_functions_fail_cleanly_before_init ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 110 filtered out; finished in 0.00s
```
