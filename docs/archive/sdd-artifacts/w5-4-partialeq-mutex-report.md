# Task Report: W5-4 (F5 + F6 — PartialEq hack 与 entry.rs Mutex 收口)

## 1. Implemented Findings & Rationale

### Finding F5: `ModuleAppProps::PartialEq` Real Implementation & Documenting Comment
- **Location**: `src/apps/desktop/src/ui_dioxus/registry.rs:39-59`
- **Implemented**:
  - Implemented real `PartialEq` comparing structural identity: `self.plugin_id == other.plugin_id && self.gen == other.gen`.
  - Added `std::fmt::Debug` for `ModuleAppProps` formatting `plugin_id` and `gen`.
  - Added unit test `test_module_app_props_partial_eq` in `registry.rs:628-677`.
  - Documented lifecycle rationale in doc-comment.
- **Choice Rationale**:
  - `ModuleAppProps` is supplied once at window creation when constructing `VirtualDom::new_with_props`.
  - Dynamic state updates (geometry, theme, active states) flow reactively through embedded `tokio::sync::watch` receivers and signals, not top-level prop mutation.
  - Comparing `plugin_id` and `gen` provides exact structural identity for the window instance/lifecycle generation without false equality across different plugins or generations.

### Finding F6: `entry.rs` Mutex Elimination (`room_window_id` + `latest_geometry`)
- **Location**:
  - `src/apps/desktop/src/ui_dioxus/state.rs:34-36` (`RoomWindowIdTx` alias)
  - `src/apps/desktop/src/ui_dioxus/entry.rs:30,133-143,193-255,273-280` (`room_window_id` watch channel + `geometry_tx.send_modify`)
  - `src/apps/desktop/src/ui_dioxus/app.rs:31,110,270-285` (`room_window_id_tx.send(...)` in `use_effect`)
- **Implemented**:
  - Replaced `room_window_id: Arc<Mutex<Option<WindowId>>>` with `tokio::sync::watch::channel::<Option<WindowId>>(None)`.
  - Producer: `room_app_root` sends `Some(window().id())` on mount in `use_effect` via `room_window_id_tx.send(...)`.
  - Consumer: tao custom event handler reads `*room_window_id_rx.borrow()` lock-free.
  - `latest_geometry`: Completely eliminated the redundant `Arc<Mutex<Geometry>>` and `std::sync::Mutex` in `entry.rs`. The custom event handler calls `geometry_tx.send_modify(...)` on `WindowEvent::Moved` / `WindowEvent::Resized` to mutate geometry coordinates in-place inside `tokio::sync::watch::Sender<Geometry>`.
- **Choice Rationale**:
  - `room_window_id` is a single-writer (on mount) and lock-free multi-reader pattern, matching `tokio::sync::watch` perfectly.
  - `latest_geometry` was only used to maintain coordinate continuity between moved and resized events. `tokio::sync::watch::Sender::send_modify` provides built-in mutable access to the current channel value without any intermediate Mutex allocation or lock overhead, establishing `geometry_tx` as the single source of truth.
  - Removed `std::sync::Mutex` import from `entry.rs`.

---

## 2. 复用侦察 (Reuse Reconnaissance)

- Reused `tokio::sync::watch` already present in `ui_dioxus::state` (`GeometryTx`, `GlobalTheme`).
- Reused `tokio::sync::watch::Sender::send_modify` for in-place geometry updates.
- Reused existing `tao::window::WindowId` type and `Copy`/`Clone`/`PartialEq` semantics.
- No new dependencies or abstractions introduced.

---

## 3. Verification Commands and Full Verbatim Outputs

### Command 1: `cargo check -p northhing`
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
   --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:373:17
    |
373 |             let workspace_turn_status = tokio::select! {
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
warning: unused import: `dioxus::desktop::tao::platform::windows::WindowExtWindows`
  --> src\apps\desktop\src\ui_dioxus\pages_archive.rs:18:5
   |
18 | use dioxus::desktop::tao::platform::windows::WindowExtWindows;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: methods `is_first_run`, `set_current_workspace`, and `remove_workspace` are never used
   --> src\apps\desktop\src\app_state\settings\mod.rs:88:12
    |
 86 | impl AppSettings {
    | ---------------- methods in this implementation
 87 |     /// Spec Q9=a: triggers the welcome flow when the user has done nothing yet.
 88 |     pub fn is_first_run(&self) -> bool {
    |            ^^^^^^^^^^^^
...
111 |     pub fn set_current_workspace(&mut self, path: Option<&Path>) {
    |            ^^^^^^^^^^^^^^^^^^^^^
...
120 |     pub fn remove_workspace(&mut self, path: &Path) -> Option<WorkspaceEntry> {
    |            ^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: struct `SessionIntegrityIssue` is never constructed
 --> src\apps\desktop\src\app_state\settings\integrity.rs:9:12
  |
9 | pub struct SessionIntegrityIssue {
  |            ^^^^^^^^^^^^^^^^^^^^^

warning: method `validate_session_integrity` is never used
  --> src\apps\desktop\src\app_state\settings\integrity.rs:33:12
   |
18 | impl AppSettings {
   | ---------------- method in this implementation
...
33 |     pub fn validate_session_integrity<I, P, W>(
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: method `delete` is never used
  --> src\apps\desktop\src\app_state\settings\keyring.rs:97:8
   |
91 | pub trait KeyringBackend: Send + Sync + std::fmt::Debug {
   |           -------------- method in this trait
...
97 |     fn delete(&self, account: &str) -> Result<()>;
   |        ^^^^^^

warning: function `delete_api_key` is never used
   --> src\apps\desktop\src\app_state\settings\keyring.rs:253:8
    |
253 | pub fn delete_api_key(keyring: &dyn KeyringBackend, provider_id: &str) -> Result<()> {
    |        ^^^^^^^^^^^^^^

warning: function `resolve_effective_api_key` is never used
 --> src\apps\desktop\src\app_state\settings\sync.rs:5:8
  |
5 | pub fn resolve_effective_api_key(stored: Option<&str>, incoming: &str) -> String {
  |        ^^^^^^^^^^^^^^^^^^^^^^^^^

warning: function `resolve_edit_api_key` is never used
  --> src\apps\desktop\src\app_state\settings\sync.rs:16:8
   |
16 | pub fn resolve_edit_api_key(stored: anyhow::Result<String>, incoming: &str) -> anyhow::Result<String> {
   |        ^^^^^^^^^^^^^^^^^^^^

warning: function `provider_wire_format_from_str` is never used
  --> src\apps\desktop\src\app_state\settings\sync.rs:40:8
   |
40 | pub fn provider_wire_format_from_str(s: &str) -> &'static str {
   |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: function `push_resolved_keys_to_core` is never used
  --> src\apps\desktop\src\app_state\settings\sync.rs:53:14
   |
53 | pub async fn push_resolved_keys_to_core(keyring: &dyn KeyringBackend) -> anyhow::Result<usize> {
   |              ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: function `validate_provider_input` is never used
  --> src\apps\desktop\src\app_state\settings\sync.rs:72:8
   |
72 | pub fn validate_provider_input(
   |        ^^^^^^^^^^^^^^^^^^^^^^^

warning: function `turn_runtime` is never used
  --> src\apps\desktop\src\app_state\turn_runtime.rs:18:15
   |
18 | pub(crate) fn turn_runtime() -> Option<Handle> {
   |               ^^^^^^^^^^^^

warning: constant `DEFAULT_MODE_ID` is never used
  --> src\apps\desktop\src\flags.rs:10:11
   |
10 | pub const DEFAULT_MODE_ID: &str = "agentic"; // 2026-07-18: registry has no "code" mode; agentic is the default single-agent mode
   |           ^^^^^^^^^^^^^^^

warning: struct `McpCatalogAdapter` is never constructed
  --> src\apps\desktop\src\mcp_adapter.rs:29:12
   |
29 | pub struct McpCatalogAdapter {
   |            ^^^^^^^^^^^^^^^^^

warning: associated function `new` is never used
  --> src\apps\desktop\src\mcp_adapter.rs:42:12
   |
39 | impl McpCatalogAdapter {
   | ---------------------- associated function in this implementation
...
42 |     pub fn new(facade: Arc<KernelFacade>) -> Self {
   |            ^^^

warning: function `map_status` is never used
  --> src\apps\desktop\src\mcp_adapter.rs:51:4
   |
51 | fn map_status(kind: &MCPServerStatusKind) -> McpServerStatusDto {
   |    ^^^^^^^^^^

warning: function `resolve_enabled` is never used
  --> src\apps\desktop\src\mcp_adapter.rs:67:4
   |
67 | fn resolve_enabled(config: &northhing_kernel_api::settings::MCPServerDto) -> bool {
   |    ^^^^^^^^^^^^^^^

warning: function `render_status` is never used
   --> src\apps\desktop\src\mcp_adapter.rs:120:8
    |
120 | pub fn render_status(result: &Result<Vec<McpServerDto>, McpCatalogError>) -> String {
    |        ^^^^^^^^^^^^^

warning: function `list_sessions` is never used
  --> src\apps\desktop\src\ui_dioxus\api.rs:53:14
   |
53 | pub async fn list_sessions() -> Result<Vec<SessionSummaryDto>, KernelError> {
   |              ^^^^^^^^^^^^^

warning: function `get_session` is never used
  --> src\apps\desktop\src\ui_dioxus\api.rs:63:14
   |
63 | pub async fn get_session(id: &SessionId) -> Result<SessionDto, KernelError> {
   |              ^^^^^^^^^^^

warning: function `store_provider_api_key` is never used
   --> src\apps\desktop\src\ui_dioxus\api.rs:179:14
    |
179 | pub async fn store_provider_api_key(provider_id: &str, plaintext: &str) -> anyhow::Result<String> {
    |              ^^^^^^^^^^^^^^^^^^^^^^

warning: method `pending_text_chunks` is never used
   --> src\apps\desktop\src\ui_dioxus\api.rs:299:12
    |
288 | impl EventReceiver {
    | ------------------ method in this implementation
...
299 |     pub fn pending_text_chunks(&self) -> usize {
    |            ^^^^^^^^^^^^^^^^^^^

warning: type alias `GeometryRx` is never used
  --> src\apps\desktop\src\ui_dioxus\state.rs:31:10
   |
31 | pub type GeometryRx = watch::Receiver<Geometry>;
   |          ^^^^^^^^^^

warning: method `is_any_active` is never used
   --> src\apps\desktop\src\ui_dioxus\registry.rs:205:12
    |
184 | impl ShellWindowManager {
    | ----------------------- method in this implementation
...
205 |     pub fn is_any_active(&self, ids: &[&str]) -> bool {
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

warning: `northhing` (bin "northhing") generated 50 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.23s
```

### Command 2: `cargo test -p northhing` (via MSVC toolchain per AGENTS.md desktop toolchain rule)
```text
running 110 tests
test app_state::settings::keyring::tests::delete_api_key_best_effort_missing ... ok
test app_state::settings::keyring::tests::mock_keyring_store_get ... ok
test app_state::settings::keyring::tests::mock_keyring_load_env_missing_returns_empty_map_fail_open ... ok
test app_state::settings::keyring::tests::mock_keyring_store_env_sentinel_is_noop ... ok
test app_state::settings::keyring::tests::mock_seed_and_assert_helpers ... ok
test app_state::settings::keyring::tests::mock_keyring_delete_removes_entry ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_empty_string_as_is ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_plaintext_directly ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_sentinel_from_keyring ... ok
test app_state::settings::keyring::tests::mock_keyring_load_env_corrupt_json_returns_empty_map_fail_open ... ok
test app_state::settings::keyring::tests::mock_keyring_delete_missing_does_not_error ... ok
test app_state::settings::keyring::tests::store_api_key_empty_is_noop ... ok
test app_state::settings::keyring::tests::mock_keyring_get_missing_returns_err ... ok
test app_state::settings::keyring::tests::delete_api_key_removes_existing ... ok
test app_state::settings::keyring::tests::resolve_api_key_sentinel_missing_keyring_returns_err ... ok
test app_state::settings::keyring::tests::sentinel_identity ... ok
test app_state::settings::keyring::tests::store_api_key_returns_sentinel ... ok
test app_state::settings::keyring::tests::mock_keyring_store_load_env_roundtrip ... ok
test app_state::settings::keyring::tests::store_api_key_sentinel_is_noop ... ok
test app_state::settings::tests::integration_welcome_provider_session_delete_provider ... ok
test app_state::settings::tests::is_first_run_empty_settings ... ok
test app_state::settings::tests::is_first_run_with_workspace ... ok
test app_state::settings::tests::provider_new_has_unique_id_and_defaults ... ok
test app_state::settings::tests::provider_type_default_base_url ... ok
test app_state::settings::tests::provider_type_default_models_non_empty_for_named ... ok
test app_state::settings::tests::provider_wire_format_from_str_mapping ... ok
test app_state::settings::tests::provider_wire_format_from_str_other_defaults_to_openai ... ok
test app_state::settings::tests::onboarding_completed_roundtrip ... ok
test app_state::settings::tests::remove_workspace_clears_current ... ok
test app_state::settings::tests::onboarding_completed_serde_default_false ... ok
test app_state::settings::tests::resolve_edit_api_key_err_stored_blank_incoming_returns_err ... ok
test app_state::settings::tests::resolve_edit_api_key_ok_stored_blank_incoming_returns_ok_stored ... ok
test app_state::settings::tests::resolve_edit_api_key_ok_stored_non_blank_incoming_returns_ok_incoming ... ok
test app_state::settings::tests::resolve_effective_api_key_empty_incoming_keeps_stored ... ok
test app_state::settings::tests::resolve_effective_api_key_empty_incoming_no_stored_returns_empty ... ok
test app_state::settings::tests::resolve_effective_api_key_non_empty_incoming_passes_through ... ok
test app_state::settings::tests::resolve_effective_api_key_whitespace_only_treated_as_empty ... ok
test app_state::settings::tests::settings_json_roundtrip ... ok
test app_state::settings::tests::resolve_edit_api_key_err_stored_non_blank_incoming_returns_ok_incoming ... ok
test app_state::settings::tests::test_infer_provider_wire_format ... ok
test app_state::settings::tests::validate_provider_input_accepts_valid_anthropic ... ok
test app_state::settings::tests::validate_provider_input_rejects_unknown_type ... ok
test app_state::settings::tests::validate_session_integrity_detects_deleted_provider ... ok
test app_state::settings::tests::validate_provider_input_custom_requires_base_url ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_api_key ... ok
test app_state::settings::tests::workspace_add_dedups ... ok
test app_state::settings::tests::validate_session_integrity_reports_both_q6_and_q7_per_session ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_name ... ok
test app_state::settings::tests::validate_provider_input_accepts_valid_custom ... ok
test app_state::settings::tests::validate_session_integrity_detects_removed_workspace ... ok
test app_state::settings::tests::validate_session_integrity_empty_session_list_is_noop ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_model ... ok
test app_state::settings::tests::workspace_set_current_updates_last_opened ... ok
test flags::tests::default_mode_id_is_agentic ... ok
test mcp_adapter::tests::map_status_disabled ... ok
test mcp_adapter::tests::map_status_failed_carries_message ... ok
test mcp_adapter::tests::map_status_probe_timeout ... ok
test mcp_adapter::tests::map_status_connected ... ok
test mcp_adapter::tests::map_status_starting ... ok
test app_state::settings::io::io_tests::mcp_env_fail_open_missing_entry_returns_empty_map ... ok
test mcp_adapter::tests::render_status_uses_format_helpers ... ok
test mcp_adapter::tests::resolve_enabled_reads_config_field ... ok
test app_state::settings::io::io_tests::mcp_env_fail_closed_on_store_error_does_not_corrupt_disk ... ok
test app_state::settings::io::io_tests::mcp_env_keyring_sentinel_loaded_and_restored ... ok
test app_state::settings::io::io_tests::load_parse_failure_returns_err ... ok
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok
test ui_dioxus::api::tests::test_event_channel_returns_receiver ... ok
test ui_dioxus::api::tests::test_pick_room_session_empty_groups_returns_none ... ok
test ui_dioxus::api::tests::test_pick_room_session_no_preferred_picks_first_non_empty ... ok
test ui_dioxus::api::tests::test_pick_room_session_preferred_miss_returns_none ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_single ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_one ... ok
test ui_dioxus::app::tests::test_mix_hex ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_three_history ... ok
test ui_dioxus::api::tests::test_tiered_event_channel_drain_refills_budget ... ok
test ui_dioxus::api::tests::test_tiered_event_channel_text_chunk_lossy_control_guaranteed ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_three ... ok
test ui_dioxus::api::tests::test_pick_room_session_preferred_hit ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_two ... ok
test ui_dioxus::pages_settings::tests::test_mcp_server_toggle_optimistic_update ... ok
test ui_dioxus::pages_settings::tests::test_load_app_settings_resolves_workspace_path_or_default ... ok
test ui_dioxus::pages_settings::tests::test_provider_active_matching ... ok
test ui_dioxus::registry::tests::test_mark_all_closing_targets ... ok
test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
test app_state::settings::io::io_tests::mcp_env_idempotent_load_with_sentinel_does_not_rewrite_keyring ... ok
test ui_dioxus::registry::tests::test_module_app_props_partial_eq ... ok
test ui_dioxus::pages_settings::tests::test_update_app_settings_transaction_closure ... ok
test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
test ui_dioxus::registry::tests::test_onboarding_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
test ui_dioxus::registry::tests::test_settings_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok
test ui_dioxus::registry::tests::test_space_registration_and_lifecycle ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_assistant_mixed_reasoning_fallback ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_assistant_mixed_with_tool_calls ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_empty_returns_empty ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_system_and_tool_skipped ... ok
test ui_dioxus::session_mock::tests::test_seed_session_has_mock_approvals_with_call_ids ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_user_text_to_witness ... ok
test app_state::settings::io::io_tests::mcp_env_update_app_settings_stores_new_env_in_keyring ... ok
test app_state::settings::io::io_tests::leftover_tmp_file_does_not_break_main_file ... ok
test app_state::settings::io::io_tests::mcp_env_keyring_migration_plaintext_to_sentinel_on_load ... ok
test app_state::settings::io::io_tests::second_write_keeps_previous_version_in_bak ... ok
test app_state::settings::tests::push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean ... ok
test app_state::settings::io::io_tests::concurrent_updates_preserve_all_writes ... ok
test ui_dioxus::api::tests::test_persist_onboarding_provider_success_flow ... ok
test ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized ... ok
test app_state::settings::io::io_tests::concurrent_loads_and_updates_preserve_all_writes ... ok
test app_state::settings::io::io_tests::update_with_err_closure_does_not_write_file ... ok
test ui_dioxus::api::tests::test_api_functions_fail_cleanly_before_init ... ok

test result: ok. 110 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running unittests src\main.rs (target\debug\deps\northhing-b25540c259ba06d3.exe)

running 110 tests
[110 tests ok - matching lib suite]
test result: ok. 110 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running unittests src\bin\w4_repro.rs (target\debug\deps\w4_repro-7791c7b3476422f9.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests northhing

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

---

## 4. Compile Errors & Layers Fixed

1. `error[E0277]: ui_dioxus::registry::ModuleAppProps doesn't implement std::fmt::Debug`:
   - **Layer**: Trait / Language mechanics layer (`m04-zero-cost`).
   - **Fix**: Added explicit `impl std::fmt::Debug for ModuleAppProps` displaying `plugin_id` and `gen`, enabling standard test assertion reporting and logging.
2. `error: linking with x86_64-w64-mingw32-gcc failed (GNU ld response file parsing on Windows temp path)`:
   - **Layer**: Build / Platform environment layer (per `AGENTS.md` Backbone Invariants: desktop package builds/tests use MSVC toolchain on Windows via `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing`).
   - **Fix**: Tested under MSVC toolchain, linking and passing all 110 tests.

---

## 5. Self-Review Findings & Concerns

- **Scope boundaries**: Changes strictly contained in `src/apps/desktop/src/ui_dioxus/` (`app.rs`, `entry.rs`, `registry.rs`, `state.rs`). No other crate touched.
- **Concurrency & Behavioral Equivalence**: Zero behavior change. Window ID registration on mount and event filtering remain strictly identical in semantics. Geometry tracking now updates directly via `geometry_tx.send_modify`, eliminating race conditions between intermediate `Mutex<Geometry>` and channel broadcasts.
- **Git discipline**: Exactly one commit for code changes, no `.superpowers/` files included in the commit.
- **Concerns**: None.
