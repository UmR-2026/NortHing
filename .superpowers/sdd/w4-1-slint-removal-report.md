# W4-1 Report: 物理删除 Slint 壳，Dioxus 成为唯一壳

## 1. Deletion Inventory

- **Files Deleted**: 69 files (19 Rust modules/files under `src/apps/desktop/src/app_state/` + 50 files under `src/apps/desktop/src/ui/` including 41 `.slint` files, 6 font binaries, and 3 font license/markdown files).
- **Files Modified**: 10 files (`AGENTS-CN.md`, `AGENTS.md`, `Cargo.lock`, `src/apps/desktop/Cargo.toml`, `src/apps/desktop/build.rs`, `src/apps/desktop/src/app_state/mod.rs`, `src/apps/desktop/src/flags.rs`, `src/apps/desktop/src/lib.rs`, `src/apps/desktop/src/main.rs`, `src/apps/desktop/src/ui_dioxus/entry.rs`).
- **Total Changes**: 79 files changed, 164 insertions(+), 16783 deletions(-).

### `git show --stat HEAD` Summary

```
commit 707e414bf20960a8e99580c8034771d6daa11b25
Author: Mavis <mavis@northhing.local>
Date:   Fri Aug 28 00:41:02 2026 +0800

    refactor(desktop): physically remove Slint UI shell, Dioxus consult-room shell is sole frontend

 AGENTS-CN.md                                       |    2 +-
 AGENTS.md                                          |    4 +-
 Cargo.lock                                         | 3182 +-------------------
 src/apps/desktop/Cargo.toml                        |   62 +-
 src/apps/desktop/build.rs                          |   16 +-
 src/apps/desktop/src/app_state/block_registry.rs   |  158 -
 .../desktop/src/app_state/callbacks_lifecycle.rs   | 1010 -------
 .../src/app_state/callbacks_settings/misc.rs       |  210 --
 .../src/app_state/callbacks_settings/mod.rs        |   61 -
 .../src/app_state/callbacks_settings/provider.rs   |  201 --
 .../app_state/callbacks_settings/provider_test.rs  |  185 --
 .../src/app_state/callbacks_settings/refresh.rs    |  834 -----
 .../app_state/callbacks_settings/skill_filter.rs   |   85 -
 .../src/app_state/callbacks_settings/workspace.rs  |  200 --
 src/apps/desktop/src/app_state/create_ui.rs        |  514 ----
 src/apps/desktop/src/app_state/error_banners.rs    |  129 -
 src/apps/desktop/src/app_state/event_bridge.rs     |  606 ----
 src/apps/desktop/src/app_state/inspector.rs        |   19 -
 .../src/app_state/inspector_model_status.rs        |   46 -
 src/apps/desktop/src/app_state/mod.rs              |  504 +---
 src/apps/desktop/src/app_state/sessions.rs         |  344 ---
 src/apps/desktop/src/app_state/skills.rs           |  342 ---
 src/apps/desktop/src/app_state/slint_glue.rs       |   31 -
 src/apps/desktop/src/app_state/state.rs            |  285 --
 .../desktop/src/app_state/streaming_lifecycle.rs   |  618 ----
 src/apps/desktop/src/flags.rs                      |   53 -
 src/apps/desktop/src/lib.rs                        |    8 -
 src/apps/desktop/src/main.rs                       |  127 +-
 src/apps/desktop/src/ui/components/AirTint.slint   |   33 -
 .../desktop/src/ui/components/AvatarWrap.slint     |   62 -
 .../src/ui/components/ChatMessageBubble.slint      |   88 -
 .../desktop/src/ui/components/ChronicleBar.slint   |   71 -
 src/apps/desktop/src/ui/components/CodeBlock.slint |   29 -
 src/apps/desktop/src/ui/components/DeckBar.slint   |  149 -
 .../desktop/src/ui/components/DoorbellGem.slint    |   42 -
 .../desktop/src/ui/components/InnerDrawer.slint    |    6 -
 .../desktop/src/ui/components/MarkdownText.slint   |   14 -
 .../desktop/src/ui/components/MaterialBadge.slint  |   21 -
 .../desktop/src/ui/components/MaterialBanner.slint |  104 -
 .../desktop/src/ui/components/MaterialButton.slint |   47 -
 .../desktop/src/ui/components/MaterialCard.slint   |   16 -
 .../src/ui/components/MaterialIconButton.slint     |   40 -
 .../desktop/src/ui/components/MaterialList.slint   |   46 -
 .../src/ui/components/MaterialTextField.slint      |   52 -
 src/apps/desktop/src/ui/components/MindMod.slint   |  338 ---
 src/apps/desktop/src/ui/components/MoodText.slint  |   24 -
 .../desktop/src/ui/components/OuterDrawer.slint    |   11 -
 src/apps/desktop/src/ui/components/Pill.slint      |   43 -
 .../desktop/src/ui/components/PresenceZone.slint   |  148 -
 src/apps/desktop/src/ui/components/RoomHead.slint  |  228 --
 .../src/ui/components/SegmentedControl.slint       |   80 -
 .../desktop/src/ui/components/ThinkBlock.slint     |   79 -
 .../desktop/src/ui/components/ToggleSwitch.slint   |   46 -
 .../desktop/src/ui/components/ToolCallCard.slint   |   81 -
 src/apps/desktop/src/ui/components/ToolChip.slint  |   46 -
 .../desktop/src/ui/components/TurnContainer.slint  |   41 -
 .../desktop/src/ui/components/WindowChrome.slint   |  181 --
 src/apps/desktop/src/ui/components/WorkMod.slint   |   93 -
 src/apps/desktop/src/ui/fonts/FONTS.md             |   60 -
 src/apps/desktop/src/ui/fonts/Fraunces-Display.ttf |  Bin 72780 -> 0 bytes
 src/apps/desktop/src/ui/fonts/Fraunces-Italic.ttf  |  Bin 88968 -> 0 bytes
 src/apps/desktop/src/ui/fonts/Fraunces-Regular.ttf |  Bin 72788 -> 0 bytes
 src/apps/desktop/src/ui/fonts/JetBrainsMono.ttf    |  Bin 300144 -> 0 bytes
 src/apps/desktop/src/ui/fonts/NotoSansSC.ttf       |  Bin 1777952 -> 0 bytes
 src/apps/desktop/src/ui/fonts/OFL-Fraunces.txt     |   93 -
 .../desktop/src/ui/fonts/OFL-JetBrainsMono.txt     |   93 -
 src/apps/desktop/src/ui/fonts/OFL-NotoSansSC.txt   |   93 -
 src/apps/desktop/src/ui/main.slint                 |  514 ----
 src/apps/desktop/src/ui/redesign_palette.slint     |  291 --
 src/apps/desktop/src/ui/strings.slint              |  309 --
 src/apps/desktop/src/ui/system_constants.slint     |   17 -
 src/apps/desktop/src/ui/theme.slint                |  173 --
 src/apps/desktop/src/ui/views/ArchiveView.slint    |  298 --
 src/apps/desktop/src/ui/views/ChatPaneView.slint   |  449 ---
 .../desktop/src/ui/views/IdentityCreatorView.slint |  218 --
 src/apps/desktop/src/ui/views/SettingsView.slint   | 1201 --------
 src/apps/desktop/src/ui/views/SpaceView.slint      |  103 -
 src/apps/desktop/src/ui/views/WelcomeView.slint    |  923 ------
 src/apps/desktop/src/ui_dioxus/entry.rs            |   17 +-
 79 files changed, 164 insertions(+), 16783 deletions(-)
```

---

## 2. 复用侦察 (Recon)

- **Extra items kept outside prescribed keep-list**: None.
- Kept strictly per keep-list: `src/apps/desktop/src/app_state/settings/**`, `src/apps/desktop/src/app_state/log.rs`, `src/apps/desktop/src/app_state/turn_runtime.rs`, `src/apps/desktop/src/ui_dioxus/**`.
- `build.rs`: Preserved the Windows manifest embedding logic (`northhing.rc` / `northhing.exe.manifest`) required for ComCtl32 v6 linking while dropping the `slint_build` call and `src/ui/*` rebuild triggers.
- `src/apps/desktop/src/bin/w4_repro.rs`: Kept intact as diagnostic binary.

---

## 3. `rg -i slint` Residue Analysis

Command run: `rg -i -n "slint" src/ src/apps/desktop/Cargo.toml Cargo.toml`

### Residue Items and Justifications:

1. `Cargo.toml:151`: `# Native file dialogs (cross-platform; used by desktop Slint shell)` — Root workspace Cargo.toml comment.
2. `Cargo.toml:153`: `# installer crate, but desktop uses pure Slint + winit` — Root workspace Cargo.toml comment.
3. `Cargo.toml:220`: `# Slint UI framework (desktop shell)` — Root workspace Cargo.toml comment.
4. `Cargo.toml:221-222`: `slint = "1.16"`, `slint-build = "1.16"` — Root workspace dependencies table (unchanged per Global Constraint 2: scope limited to desktop crate).
5. `src/apps/desktop/northhing.exe.manifest:6`: Doc comment explaining why `muda` requires ComCtl32 v6.
6. `src/apps/desktop/src/ui_dioxus/entry.rs:61,153`: Doc comments referencing design history.
7. `src/apps/desktop/src/ui_dioxus/state.rs:35`: Doc comment referencing theme state history.
8. `src/apps/desktop/src/mcp_adapter.rs:6,119`: Doc comments referencing status mapping history.
9. `src/apps/desktop/src/ui_dioxus/pages_onboarding.rs:318`: UI mock text `div { class: "row active", span { class: "sq-toggle" } "Slint 规格架构" }` (present in truth HTML mock content).
10. `src/apps/desktop/src/ui_dioxus/mod.rs:8,11`: Doc comments referencing migration history.
11. `src/apps/desktop/src/ui_dioxus/i18n.rs:10,18`: Doc comments referencing string global design history.
12. `src/crates/contracts/runtime-ports/src/mcp.rs:108`: Doc comment referencing port contract history.
13. `src/apps/desktop/src/app_state/settings/mod.rs:23,33`: Doc comments in settings data layer.
14. `src/apps/desktop/src/app_state/log.rs:62`: Doc comment explaining background logging channel history.
15. `src/apps/desktop/README.md`: Historical desktop README documentation.
16. `src/crates/contracts/product-domains/...`: False positive substring match for `"ESLint"`.
17. `src/crates/assembly/core/...`: False positive substring match for `".eslintrc.json"`.

**Conclusion**: Active Slint code (imports, structs, types, functions, build calls, crate dependencies) is completely 0 in `src/apps/desktop`.

---

## 4. Compile Errors and Fixes

1. **Build script MinGW GNU response file path syntax issue**:
   - *Error*: `ld: cannot find @C:\WINDOWS\TEMP\cc...: Invalid argument` when compiling `build.rs`.
   - *Layer*: Toolchain environment layer.
   - *Fix*: Point `$env:TEMP` / `$env:TMP` to standard `$env:LOCALAPPDATA\Temp` and include `C:\msys64\mingw64\bin` on `$env:PATH` so `windres` and MinGW GNU linker operate properly.
2. **`ui_dioxus::entry::launch` remaining `flags::DIOXUS_SHELL` check**:
   - *Error*: After deleting `DIOXUS_SHELL` from `flags.rs`, `entry.rs` imported `crate::flags::DIOXUS_SHELL`.
   - *Layer*: Design/Wiring layer.
   - *Fix*: Removed `use crate::flags::DIOXUS_SHELL;` and the redundant `if !DIOXUS_SHELL` check from `ui_dioxus/entry.rs` since Dioxus is now the sole shell.

---

## 5. Verification Commands and Outputs

### (1) `cargo check --workspace`
```
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
warning: unused import: `sync::*`
  --> src\apps\desktop\src\app_state\settings\mod.rs:50:9
   |
50 | pub use sync::*;
   |         ^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `dioxus::desktop::tao::platform::windows::WindowExtWindows`
  --> src\apps\desktop\src\ui_dioxus\pages_archive.rs:18:5
   |
18 | use dioxus::desktop::tao::platform::windows::WindowExtWindows;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

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
  --> src\apps\desktop\src\app_state\settings\sync.rs:27:8
   |
27 | pub fn provider_wire_format_from_str(s: &str) -> &'static str {
   |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: function `push_resolved_keys_to_core` is never used
  --> src\apps\desktop\src\app_state\settings\sync.rs:40:14
   |
40 | pub async fn push_resolved_keys_to_core(keyring: &dyn KeyringBackend) -> anyhow::Result<usize> {
   |              ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: function `validate_provider_input` is never used
  --> src\apps\desktop\src\app_state\settings\sync.rs:59:8
   |
59 | pub fn validate_provider_input(
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
  --> src\apps\desktop\src\ui_dioxus\api.rs:54:14
   |
54 | pub async fn list_sessions() -> Result<Vec<SessionSummaryDto>, KernelError> {
   |              ^^^^^^^^^^^^^

warning: function `get_session` is never used
  --> src\apps\desktop\src\ui_dioxus\api.rs:64:14
   |
64 | pub async fn get_session(id: &SessionId) -> Result<SessionDto, KernelError> {
   |              ^^^^^^^^^^^

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

warning: `northhing` (bin "northhing") generated 49 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 2 suggestions)
warning: unused imports: `QuestionData` and `QuestionOption`
  --> src\apps\cli\src\ui\question\mod.rs:15:33
   |
15 | pub use types::{QuestionAction, QuestionData, QuestionOption, QuestionPrompt};
   |                                 ^^^^^^^^^^^^  ^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.93s
```

### (2) `cargo check -p northhing`
```
warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
warning: unused import: `sync::*`
  --> src\apps\desktop\src\app_state\settings\mod.rs:50:9
   |
50 | pub use sync::*;
   |         ^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `dioxus::desktop::tao::platform::windows::WindowExtWindows`
  --> src\apps\desktop\src\ui_dioxus\pages_archive.rs:18:5
   |
18 | use dioxus::desktop::tao::platform::windows::WindowExtWindows;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

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
  --> src\apps\desktop\src\app_state\settings\sync.rs:27:8
   |
27 | pub fn provider_wire_format_from_str(s: &str) -> &'static str {
   |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: function `push_resolved_keys_to_core` is never used
  --> src\apps\desktop\src\app_state\settings\sync.rs:40:14
   |
40 | pub async fn push_resolved_keys_to_core(keyring: &dyn KeyringBackend) -> anyhow::Result<usize> {
   |              ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: function `validate_provider_input` is never used
  --> src\apps\desktop\src\app_state\settings\sync.rs:59:8
   |
59 | pub fn validate_provider_input(
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
  --> src\apps\desktop\src\ui_dioxus\api.rs:54:14
   |
54 | pub async fn list_sessions() -> Result<Vec<SessionSummaryDto>, KernelError> {
   |              ^^^^^^^^^^^^^

warning: function `get_session` is never used
  --> src\apps\desktop\src\ui_dioxus\api.rs:64:14
   |
64 | pub async fn get_session(id: &SessionId) -> Result<SessionDto, KernelError> {
   |              ^^^^^^^^^^^

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

warning: `northhing` (bin "northhing") generated 49 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 2 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.60s
```

### (3) `cargo test -p northhing`
```
running 105 tests
test app_state::settings::keyring::tests::delete_api_key_removes_existing ... ok
test app_state::settings::keyring::tests::mock_keyring_delete_missing_does_not_error ... ok
test app_state::settings::keyring::tests::mock_keyring_store_env_sentinel_is_noop ... ok
test app_state::settings::keyring::tests::mock_keyring_load_env_corrupt_json_returns_empty_map_fail_open ... ok
test app_state::settings::keyring::tests::mock_keyring_load_env_missing_returns_empty_map_fail_open ... ok
test app_state::settings::keyring::tests::mock_seed_and_assert_helpers ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_plaintext_directly ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_sentinel_from_keyring ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_empty_string_as_is ... ok
test app_state::settings::keyring::tests::sentinel_identity ... ok
test app_state::settings::keyring::tests::resolve_api_key_sentinel_missing_keyring_returns_err ... ok
test app_state::settings::keyring::tests::mock_keyring_delete_removes_entry ... ok
test app_state::settings::keyring::tests::mock_keyring_get_missing_returns_err ... ok
test app_state::settings::tests::is_first_run_with_workspace ... ok
test app_state::settings::keyring::tests::mock_keyring_store_get ... ok
test app_state::settings::keyring::tests::store_api_key_sentinel_is_noop ... ok
test app_state::settings::keyring::tests::store_api_key_returns_sentinel ... ok
test app_state::settings::tests::is_first_run_empty_settings ... ok
test app_state::settings::keyring::tests::mock_keyring_store_load_env_roundtrip ... ok
test app_state::settings::keyring::tests::delete_api_key_best_effort_missing ... ok
test app_state::settings::tests::provider_new_has_unique_id_and_defaults ... ok
test app_state::settings::tests::integration_welcome_provider_session_delete_provider ... ok
test app_state::settings::keyring::tests::store_api_key_empty_is_noop ... ok
test app_state::settings::tests::onboarding_completed_roundtrip ... ok
test app_state::settings::tests::provider_wire_format_from_str_other_defaults_to_openai ... ok
test app_state::settings::tests::onboarding_completed_serde_default_false ... ok
test app_state::settings::tests::provider_type_default_base_url ... ok
test app_state::settings::tests::resolve_effective_api_key_empty_incoming_no_stored_returns_empty ... ok
test app_state::settings::tests::provider_type_default_models_non_empty_for_named ... ok
test app_state::settings::tests::provider_wire_format_from_str_mapping ... ok
test app_state::settings::tests::remove_workspace_clears_current ... ok
test app_state::settings::tests::resolve_edit_api_key_err_stored_blank_incoming_returns_err ... ok
test app_state::settings::tests::resolve_edit_api_key_err_stored_non_blank_incoming_returns_ok_incoming ... ok
test app_state::settings::tests::resolve_edit_api_key_ok_stored_blank_incoming_returns_ok_stored ... ok
test app_state::settings::tests::resolve_effective_api_key_empty_incoming_keeps_stored ... ok
test app_state::settings::tests::resolve_effective_api_key_non_empty_incoming_passes_through ... ok
test app_state::settings::tests::resolve_effective_api_key_whitespace_only_treated_as_empty ... ok
test app_state::settings::tests::validate_provider_input_accepts_valid_anthropic ... ok
test app_state::settings::tests::settings_json_roundtrip ... ok
test app_state::settings::tests::validate_provider_input_accepts_valid_custom ... ok
test app_state::settings::tests::validate_provider_input_custom_requires_base_url ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_api_key ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_model ... ok
test app_state::settings::tests::validate_session_integrity_reports_both_q6_and_q7_per_session ... ok
test app_state::settings::tests::resolve_edit_api_key_ok_stored_non_blank_incoming_returns_ok_incoming ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_name ... ok
test app_state::settings::tests::validate_provider_input_rejects_unknown_type ... ok
test app_state::settings::tests::validate_session_integrity_detects_deleted_provider ... ok
test app_state::settings::tests::validate_session_integrity_detects_removed_workspace ... ok
test app_state::settings::tests::validate_session_integrity_empty_session_list_is_noop ... ok
test app_state::settings::tests::workspace_add_dedups ... ok
test app_state::settings::tests::workspace_set_current_updates_last_opened ... ok
test flags::tests::default_mode_id_is_agentic ... ok
test mcp_adapter::tests::resolve_enabled_reads_config_field ... ok
test mcp_adapter::tests::map_status_connected ... ok
test mcp_adapter::tests::map_status_disabled ... ok
test mcp_adapter::tests::map_status_failed_carries_message ... ok
test mcp_adapter::tests::map_status_probe_timeout ... ok
test mcp_adapter::tests::map_status_starting ... ok
test mcp_adapter::tests::render_status_uses_format_helpers ... ok
test ui_dioxus::api::tests::test_event_channel_returns_receiver ... ok
test app_state::settings::io::io_tests::mcp_env_fail_open_missing_entry_returns_empty_map ... ok
test ui_dioxus::api::tests::test_pick_room_session_empty_groups_returns_none ... ok
test ui_dioxus::api::tests::test_pick_room_session_no_preferred_picks_first_non_empty ... ok
test ui_dioxus::api::tests::test_pick_room_session_preferred_hit ... ok
test ui_dioxus::api::tests::test_pick_room_session_preferred_miss_returns_none ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_single ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_three_history ... ok
test ui_dioxus::app::tests::test_mix_hex_base ... ok
test app_state::settings::io::io_tests::mcp_env_keyring_sentinel_loaded_and_restored ... ok
test app_state::settings::io::io_tests::mcp_env_idempotent_load_with_sentinel_does_not_rewrite_keyring ... ok
test ui_dioxus::app::tests::test_mix_hex_target ... ok
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_one ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_three ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_two ... ok
test ui_dioxus::pages_settings::tests::test_mcp_server_toggle_optimistic_update ... ok
test ui_dioxus::pages_settings::tests::test_provider_active_matching ... ok
test app_state::settings::io::io_tests::load_parse_failure_returns_err ... ok
test ui_dioxus::registry::tests::test_onboarding_registration_and_lifecycle ... ok
test ui_dioxus::pages_settings::tests::test_update_app_settings_transaction_closure ... ok
test ui_dioxus::registry::tests::test_space_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
test ui_dioxus::pages_settings::tests::test_load_app_settings_resolves_workspace_path_or_default ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_system_and_tool_skipped ... ok
test app_state::settings::io::io_tests::mcp_env_fail_closed_on_store_error_does_not_corrupt_disk ... ok
test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok
test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_assistant_mixed_reasoning_fallback ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_empty_returns_empty ... ok
test ui_dioxus::registry::tests::test_settings_registration_and_lifecycle ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_user_text_to_witness ... ok
test ui_dioxus::session_mock::tests::test_seed_session_has_mock_approvals_with_call_ids ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_assistant_mixed_with_tool_calls ... ok
test app_state::settings::io::io_tests::leftover_tmp_file_does_not_break_main_file ... ok
test app_state::settings::io::io_tests::mcp_env_keyring_migration_plaintext_to_sentinel_on_load ... ok
test app_state::settings::io::io_tests::second_write_keeps_previous_version_in_bak ... ok
test app_state::settings::tests::push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean ... ok
test app_state::settings::io::io_tests::concurrent_updates_preserve_all_writes ... ok
test ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized ... ok
test app_state::settings::io::io_tests::concurrent_loads_and_updates_preserve_all_writes ... ok
test app_state::settings::io::io_tests::update_with_err_closure_does_not_write_file ... ok
test app_state::settings::io::io_tests::mcp_env_update_app_settings_stores_new_env_in_keyring ... ok
test ui_dioxus::api::tests::test_api_functions_fail_cleanly_before_init ... ok

test result: ok. 105 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
```

### (4) `pnpm run check:repo-hygiene`
```
> northhing@0.2.10 check:repo-hygiene E:\agent-project\northing
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (13 content files scanned, 3595 filenames checked).
```

---

## 6. Self-Review Findings

- **Spec Compliance**: All 8 delete items completed exactly. Keep-list preserved without drift.
- **Single Commit**: Exactly 1 commit `707e414` created, containing code deletions + wiring + AGENTS.md update, with zero `.superpowers/` artifacts tracked.
- **Cargo / Tests**: Workspace checks pass; desktop crate check passes; all 105 desktop tests pass.
- **Rot Budget**: No ceilings altered.

---

## 7. Concerns / Notes

- None. Slint shell physically eliminated; Dioxus consult-room shell is now the sole shell for `northhing`.

---

## 8. W4-1 Review Fixes (Commit 0c95aa6)

### Changes Per Finding

1. **Root `Cargo.toml` Workspace Cleanup**:
   - Removed `slint = "1.16"` and `slint-build = "1.16"` workspace declarations along with the `# Slint UI framework (desktop shell)` comment.
   - Removed `rfd = "0.14"` workspace declaration along with its comment block.
   - **`rfd` Residue Verification**:
     - `rg -n "rfd" --type rust src/`: 0 matches (confirmed no active Rust callers).
     - `rg -n "rfd" -g "Cargo.toml"`: 0 matches (confirmed no member crate dependencies).
2. **`pages_onboarding.rs:318` Rendered Text Fix**:
   - Changed string literal `"Slint 规格架构"` to `"Dioxus 规格架构"`.
3. **`AGENTS.md` & `AGENTS-CN.md` Quick-Start Item 2**:
   - Updated quick-start item 2 in both files to refer to the Dioxus consult-room desktop app.

### `rg -i slint src/ Cargo.toml` Post-Fix Residue List

Command: `rg -i -n "slint" src/ Cargo.toml`

Output:
```
src/apps\desktop\src\ui_dioxus\state.rs:35:/// Slint `RedesignTheme` global was per-instance, which broke
src/crates\contracts\runtime-ports\src\mcp.rs:108:/// `set_mcp_status` Slint property contract.
src/apps\desktop\src\ui_dioxus\mod.rs:8:// completely so the Slint shell remains byte-identical.
src/apps\desktop\src\ui_dioxus\mod.rs:11:// default), `main.rs` keeps launching the Slint shell. When `true` and the
src/apps\desktop\src\ui_dioxus\i18n.rs:10:// The locale selection mirrors the existing Slint shell behavior — read
src/apps\desktop\src\ui_dioxus\i18n.rs:18:/// the default in the Slint shell's `AppStrings` global.
src/apps\desktop\src\ui_dioxus\entry.rs:61:/// constant as the Slint `block_registry.rs` to keep both stacks
src/apps\desktop\src\ui_dioxus\entry.rs:153:        // §4, D = 方案一) — the old "Slint shell keeps decorations" matching
src/apps\desktop\src\mcp_adapter.rs:6://! refreshing the `mcp_status` Slint property.
src/apps\desktop\src\mcp_adapter.rs:119:/// `set_mcp_status` Slint callback (Phase G.2).
src/crates\contracts\product-domains\src\function_agents\git_func_agent\context_analyzer.rs:187:        if repo_path.join(".eslintrc.js").exists()
src/crates\contracts\product-domains\src\function_agents\git_func_agent\context_analyzer.rs:188:            || repo_path.join(".eslintrc.json").exists()
src/crates\contracts\product-domains\src\function_agents\git_func_agent\context_analyzer.rs:189:            || repo_path.join("eslint.config.js").exists()
src/crates\contracts\product-domains\src\function_agents\git_func_agent\context_analyzer.rs:191:            standards.push("ESLint");
src/apps\desktop\src\app_state\settings\mod.rs:23://! UI settings there would couple the shared core to the desktop Slint shell.
src/apps\desktop\src\app_state\settings\mod.rs:33://! wrapper layers debounced save + Mutex on top so the Slint UI can mutate
src/apps\desktop\src\app_state\log.rs:62:/// `mpsc::unbounded_channel` so the sync Slint callbacks can record
src/apps\desktop\README.md:3:Slint + Material GUI application - the primary human-facing entry point for northhing.
src/apps\desktop\README.md:11:│  northhing (Slint GUI)                  │
src/apps\desktop\README.md:37:- **Slint reactive UI**: Declarative `.slint` markup with Rust backend binding
src/apps\desktop\README.md:44:const USE_SLINT_SHELL: bool = true;        // Disable to compile stub
src/apps\desktop\README.md:52:- `slint` 1.16+ (UI framework)
src/apps\desktop\README.md:63:│   ├── app_state.rs     # Slint UI creation + callbacks
src/apps\desktop\README.md:66:    ├── main.slint       # Root window + theme
src/apps\desktop\northhing.exe.manifest:6:    Why this file exists (2026-08-07): `muda` (pulled in by the Slint /
src/crates\assembly\core\src\service\lsp\config_watcher.rs:40:            ".eslintrc.json",
src/crates\assembly\core\src\service\lsp\config_watcher.rs:41:            ".eslintrc.js",
src/crates\assembly\core\src\service\lsp\config_watcher.rs:105:            ".eslintrc.json" | ".eslintrc.js" => Some("javascript"),
```

### Post-Fix Verification Outputs

#### (1) `cargo check --workspace`
```
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
warning: unused imports: `QuestionData` and `QuestionOption`
  --> src\apps\cli\src\ui\question\mod.rs:15:33
   |
15 | pub use types::{QuestionAction, QuestionData, QuestionOption, QuestionPrompt};
   |                                 ^^^^^^^^^^^^  ^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
warning: unused import: `sync::*`
  --> src\apps\desktop\src\app_state\settings\mod.rs:50:9
   |
50 | pub use sync::*;
   |         ^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `dioxus::desktop::tao::platform::windows::WindowExtWindows`
  --> src\apps\desktop\src\ui_dioxus\pages_archive.rs:18:5
   |
18 | use dioxus::desktop::tao::platform::windows::WindowExtWindows;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

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
  --> src\apps\desktop\src\app_state\settings\sync.rs:27:8
   |
27 | pub fn provider_wire_format_from_str(s: &str) -> &'static str {
   |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: function `push_resolved_keys_to_core` is never used
  --> src\apps\desktop\src\app_state\settings\sync.rs:40:14
   |
40 | pub async fn push_resolved_keys_to_core(keyring: &dyn KeyringBackend) -> anyhow::Result<usize> {
   |              ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: function `validate_provider_input` is never used
  --> src\apps\desktop\src\app_state\settings\sync.rs:59:8
   |
59 | pub fn validate_provider_input(
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
  --> src\apps\desktop\src\ui_dioxus\api.rs:54:14
   |
54 | pub async fn list_sessions() -> Result<Vec<SessionSummaryDto>, KernelError> {
   |              ^^^^^^^^^^^^^

warning: function `get_session` is never used
  --> src\apps\desktop\src\ui_dioxus\api.rs:64:14
   |
64 | pub async fn get_session(id: &SessionId) -> Result<SessionDto, KernelError> {
   |              ^^^^^^^^^^^

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

warning: `northhing` (bin "northhing") generated 49 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 2 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.53s
```

#### (2) `cargo check -p northhing`
```
warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
warning: unused import: `sync::*`
  --> src\apps\desktop\src\app_state\settings\mod.rs:50:9
   |
50 | pub use sync::*;
   |         ^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `dioxus::desktop::tao::platform::windows::WindowExtWindows`
  --> src\apps\desktop\src\ui_dioxus\pages_archive.rs:18:5
   |
18 | use dioxus::desktop::tao::platform::windows::WindowExtWindows;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

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
  --> src\apps\desktop\src\app_state\settings\sync.rs:27:8
   |
27 | pub fn provider_wire_format_from_str(s: &str) -> &'static str {
   |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: function `push_resolved_keys_to_core` is never used
  --> src\apps\desktop\src\app_state\settings\sync.rs:40:14
   |
40 | pub async fn push_resolved_keys_to_core(keyring: &dyn KeyringBackend) -> anyhow::Result<usize> {
   |              ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: function `validate_provider_input` is never used
  --> src\apps\desktop\src\app_state\settings\sync.rs:59:8
   |
59 | pub fn validate_provider_input(
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
  --> src\apps\desktop\src\ui_dioxus\api.rs:54:14
   |
54 | pub async fn list_sessions() -> Result<Vec<SessionSummaryDto>, KernelError> {
   |              ^^^^^^^^^^^^^

warning: function `get_session` is never used
  --> src\apps\desktop\src\ui_dioxus\api.rs:64:14
   |
64 | pub async fn get_session(id: &SessionId) -> Result<SessionDto, KernelError> {
   |              ^^^^^^^^^^^

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

warning: `northhing` (bin "northhing") generated 49 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 2 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.88s
```

#### (3) `cargo test -p northhing`
```
running 105 tests
test app_state::settings::keyring::tests::mock_keyring_delete_removes_entry ... ok
test app_state::settings::keyring::tests::mock_keyring_load_env_missing_returns_empty_map_fail_open ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_empty_string_as_is ... ok
test app_state::settings::keyring::tests::mock_keyring_delete_missing_does_not_error ... ok
test app_state::settings::keyring::tests::delete_api_key_removes_existing ... ok
test app_state::settings::keyring::tests::mock_keyring_get_missing_returns_err ... ok
test app_state::settings::keyring::tests::mock_keyring_load_env_corrupt_json_returns_empty_map_fail_open ... ok
test app_state::settings::keyring::tests::mock_keyring_store_env_sentinel_is_noop ... ok
test app_state::settings::keyring::tests::mock_keyring_store_get ... ok
test app_state::settings::keyring::tests::mock_seed_and_assert_helpers ... ok
test app_state::settings::keyring::tests::resolve_api_key_sentinel_missing_keyring_returns_err ... ok
test app_state::settings::keyring::tests::mock_keyring_store_load_env_roundtrip ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_plaintext_directly ... ok
test app_state::settings::keyring::tests::store_api_key_empty_is_noop ... ok
test app_state::settings::keyring::tests::sentinel_identity ... ok
test app_state::settings::keyring::tests::store_api_key_returns_sentinel ... ok
test app_state::settings::keyring::tests::store_api_key_sentinel_is_noop ... ok
test app_state::settings::keyring::tests::delete_api_key_best_effort_missing ... ok
test app_state::settings::tests::integration_welcome_provider_session_delete_provider ... ok
test app_state::settings::keyring::tests::resolve_api_key_returns_sentinel_from_keyring ... ok
test app_state::settings::tests::remove_workspace_clears_current ... ok
test app_state::settings::tests::is_first_run_empty_settings ... ok
test app_state::settings::tests::is_first_run_with_workspace ... ok
test app_state::settings::tests::onboarding_completed_roundtrip ... ok
test app_state::settings::tests::provider_new_has_unique_id_and_defaults ... ok
test app_state::settings::tests::onboarding_completed_serde_default_false ... ok
test app_state::settings::tests::provider_type_default_base_url ... ok
test app_state::settings::tests::provider_type_default_models_non_empty_for_named ... ok
test app_state::settings::tests::provider_wire_format_from_str_other_defaults_to_openai ... ok
test app_state::settings::tests::resolve_edit_api_key_err_stored_blank_incoming_returns_err ... ok
test app_state::settings::tests::resolve_edit_api_key_err_stored_non_blank_incoming_returns_ok_incoming ... ok
test app_state::settings::tests::resolve_edit_api_key_ok_stored_blank_incoming_returns_ok_stored ... ok
test app_state::settings::tests::resolve_edit_api_key_ok_stored_non_blank_incoming_returns_ok_incoming ... ok
test app_state::settings::tests::resolve_effective_api_key_empty_incoming_keeps_stored ... ok
test app_state::settings::tests::resolve_effective_api_key_empty_incoming_no_stored_returns_empty ... ok
test app_state::settings::tests::provider_wire_format_from_str_mapping ... ok
test app_state::settings::tests::settings_json_roundtrip ... ok
test app_state::settings::tests::resolve_effective_api_key_whitespace_only_treated_as_empty ... ok
test app_state::settings::tests::validate_provider_input_custom_requires_base_url ... ok
test app_state::settings::tests::validate_provider_input_accepts_valid_anthropic ... ok
test app_state::settings::tests::resolve_effective_api_key_non_empty_incoming_passes_through ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_api_key ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_name ... ok
test mcp_adapter::tests::map_status_probe_timeout ... ok
test app_state::settings::tests::validate_provider_input_rejects_unknown_type ... ok
test app_state::settings::tests::validate_session_integrity_detects_deleted_provider ... ok
test app_state::settings::tests::validate_session_integrity_detects_removed_workspace ... ok
test app_state::settings::tests::validate_session_integrity_empty_session_list_is_noop ... ok
test app_state::settings::tests::validate_session_integrity_reports_both_q6_and_q7_per_session ... ok
test app_state::settings::tests::workspace_add_dedups ... ok
test flags::tests::default_mode_id_is_agentic ... ok
test mcp_adapter::tests::map_status_connected ... ok
test app_state::settings::tests::validate_provider_input_rejects_empty_model ... ok
test app_state::settings::tests::validate_provider_input_accepts_valid_custom ... ok
test mcp_adapter::tests::map_status_starting ... ok
test mcp_adapter::tests::resolve_enabled_reads_config_field ... ok
test app_state::settings::tests::workspace_set_current_updates_last_opened ... ok
test mcp_adapter::tests::map_status_disabled ... ok
test mcp_adapter::tests::map_status_failed_carries_message ... ok
test mcp_adapter::tests::render_status_uses_format_helpers ... ok
test ui_dioxus::api::tests::test_pick_room_session_empty_groups_returns_none ... ok
test app_state::settings::io::io_tests::load_parse_failure_returns_err ... ok
test app_state::settings::io::io_tests::mcp_env_fail_closed_on_store_error_does_not_corrupt_disk ... ok
test app_state::settings::io::io_tests::mcp_env_keyring_sentinel_loaded_and_restored ... ok
test ui_dioxus::api::tests::test_pick_room_session_no_preferred_picks_first_non_empty ... ok
test ui_dioxus::api::tests::test_pick_room_session_preferred_hit ... ok
test ui_dioxus::api::tests::test_event_channel_returns_receiver ... ok
test ui_dioxus::api::tests::test_pick_room_session_preferred_miss_returns_none ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_single ... ok
test ui_dioxus::app::tests::test_mix_hex_base ... ok
test ui_dioxus::app::tests::test_mix_hex_target ... ok
test app_state::settings::io::io_tests::mcp_env_fail_open_missing_entry_returns_empty_map ... ok
test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_one ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_three ... ok
test ui_dioxus::pages_onboarding::tests::test_step_gate_step_two ... ok
test app_state::settings::io::io_tests::mcp_env_idempotent_load_with_sentinel_does_not_rewrite_keyring ... ok
test ui_dioxus::pages_settings::tests::test_mcp_server_toggle_optimistic_update ... ok
test ui_dioxus::pages_settings::tests::test_load_app_settings_resolves_workspace_path_or_default ... ok
test ui_dioxus::pages_settings::tests::test_provider_active_matching ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_three_history ... ok
test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok
test ui_dioxus::pages_settings::tests::test_update_app_settings_transaction_closure ... ok
test ui_dioxus::registry::tests::test_onboarding_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok
test ui_dioxus::registry::tests::test_settings_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_space_registration_and_lifecycle ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_assistant_mixed_reasoning_fallback ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_assistant_mixed_with_tool_calls ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_empty_returns_empty ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_system_and_tool_skipped ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_user_text_to_witness ... ok
test ui_dioxus::session_mock::tests::test_seed_session_has_mock_approvals_with_call_ids ... ok
test app_state::settings::io::io_tests::leftover_tmp_file_does_not_break_main_file ... ok
test app_state::settings::io::io_tests::mcp_env_keyring_migration_plaintext_to_sentinel_on_load ... ok
test app_state::settings::io::io_tests::second_write_keeps_previous_version_in_bak ... ok
test app_state::settings::tests::push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean ... ok
test app_state::settings::io::io_tests::concurrent_updates_preserve_all_writes ... ok
test app_state::settings::io::io_tests::mcp_env_update_app_settings_stores_new_env_in_keyring ... ok
test ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized ... ok
test app_state::settings::io::io_tests::update_with_err_closure_does_not_write_file ... ok
test app_state::settings::io::io_tests::concurrent_loads_and_updates_preserve_all_writes ... ok
test ui_dioxus::api::tests::test_api_functions_fail_cleanly_before_init ... ok

test result: ok. 105 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
```

#### (4) `pnpm run check:repo-hygiene`
```
> northhing@0.2.10 check:repo-hygiene E:\agent-project\northing
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (8 content files scanned, 3527 filenames checked).
```

