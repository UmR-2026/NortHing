# W17-1 Report: CI Windows-only Matrix, Remove CLI Unused Import Warnings, Record Tech Debt

## Changes Summary

1. **Change 1: Narrow CI `rust-build-check` Matrix to Windows-Only**
   - File: `.github/workflows/ci.yml:32-34, 43-74`
   - Modified `strategy.matrix.os` to retain only `windows-latest` with explanatory comment (`# Windows-only per user decision 2026-09-05; non-Windows builds currently broken (terminal-core E0624), see tech-debt-ledger`).
   - Removed entire `Install Linux system dependencies (Tauri)` step (dead configuration with linux runners removed).
   - Retained `Setup OpenSSL (Windows, prebuilt)` step intact.
   - Commit: `77b69df` (`ci: windows-only build matrix per user decision 2026-09-05 (W17-1)`)

2. **Change 2: Remove `northhing-cli` Unused Imports Warning**
   - File: `src/apps/cli/src/ui/question/mod.rs:15`
   - Removed unused re-exports `QuestionData` and `QuestionOption` from `pub use types::{...};`.
   - Result: `northhing-cli` compilation warnings reduced from 1 to 0.
   - Commit: `4bc3fb1` (`fix(cli): drop unused question re-exports, zero-warning baseline (W17-1)`)

3. **Change 3: Record Non-Windows Build Failure in Tech Debt Ledger**
   - File: `docs/status/tech-debt-ledger.md:246-253`
   - Added item `P2-23: 非 Windows 平台构建失败（terminal-core E0624 private deadline method）` with symptom, evidence, proposed fix, and status `deferred` per user decision 2026-09-05.
   - Commit: `77b69df` (`ci: windows-only build matrix per user decision 2026-09-05 (W17-1)`)

## Verification Evidence

### 1. `cargo check -p northhing-cli`
Command:
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check -p northhing-cli
```
Output:
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

warning: unused variable: `at_ms`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:692:85
    |
692 |     pub(crate) fn supersede_fact(&self, fact_id: &str, superseded_by: Option<&str>, at_ms: u64) -> NortHingResult<()> {
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

warning: `northhing-core` (lib) generated 16 warnings (run `cargo fix --lib -p northhing-core` to apply 15 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.85s
```
Note: `northhing-cli` produced 0 warnings (the 16 warnings above belong to library dependency `northhing-core`).

Direct binary check verification (`cargo check --bin northhing-cli`):
```text
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.76s
```
0 warnings generated by `northhing-cli`.

### 2. `node scripts/verify-rot-budget.mjs`
Command:
```powershell
node scripts/verify-rot-budget.mjs
```
Output:
```text
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=44/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=59/400], 6 god-file rules checked across 1368 files).
```

### 3. `node scripts/check-repo-hygiene.mjs`
Command:
```powershell
node scripts/check-repo-hygiene.mjs
```
Output:
```text
Repository hygiene check passed (2 content files scanned, 3831 filenames checked).
```

## Status

DONE
