# Task T2-2d Review (Round 2)

## verdict
PASS

## Round 1 findings — closure status

- **F1 (formatting overflow at 3 hotspot files): CLOSED.**
  - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs`: only the 3-line `remote_file_delivery::{...}` import block removed; lines 216-220 (memory-reminder) untouched.
  - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs`: only the import line + the 6-line `if needs_computer_links_for_source(...) { context_vars.insert(...) }` block removed; lines 405-410 (`active_turn_tasks.insert(...)`) untouched.
  - `src/crates/assembly/core/src/agentic/execution/execution_engine.rs`: only the 1-line `TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY` import removed; lines 150-156 (`info!` calls) untouched.
  - `git diff --numstat -- src/` confirms 13 insertions, 173 deletions across 25 files — all changes are required deletions plus the necessary `workspace_relative_user_link` helper and two trivial `let report_link = format!(...)` collapses.

- **F2 (`computer_link` JSON-field downstream compatibility): CLOSED.**
  - `rg -n "computer_link" .` → **0 hits across the entire repo** (including src, tests, scripts, mobile-web, services-integrations, docs).
  - The 10 `computer://` matches that remain are all `computer://` URL-scheme string literals parsed by `remote_workspace_resolver` (services-integrations, in C3 scope) or link-rendering fallbacks in `src/mobile-web/src/pages/ChatPage.tsx` (frozen). None reads the JSON field name.
  - `rg -n '"computer_link"' .` → 0 hits. No serialised-payload consumer exists for the removed field; the tool's output is consumed by the AI model within the same turn (not a persistent protocol) per the brief.

## Constraints — re-checked

| # | Constraint | Status |
|---|---|---|
| 1 | Contracts layer (runtime-ports/core-types/agent-runtime) untouched | **OK** — `git diff -- src/crates/contracts src/crates/runtime-ports src/crates/core-types src/crates/agent-runtime` returns empty. |
| 2 | `DialogTriggerSource::RemoteRelay|Bot` retained; `subagent_ports.rs:113` `unwrap_or(Bot)` unchanged | **OK** — `rg "DialogTriggerSource::Bot"` shows the variant intact in `runtime-ports/src/agent/agent_dialog.rs:62` and `subagent_ports.rs:113` is byte-identical to `9c14d22`. |
| 3 | SSH semantics (`remote_connection_id`/`remote_ssh*`) untouched | **OK** — `git diff -G "remote_connection_id\|remote_ssh" -- src/` returns empty. |
| 4 | Only brief-listed deletions; no opportunistic refactor of hotspot files; memory/、.opencode/、.superpowers/sdd/ 其它 task-*、前端 untouched | **OK** — diff contains 25 src files exactly matching brief S1–S5. Pre-existing working-tree churn in `.opencode/model-capability-notes.md` and `memory/northhing.md` was authored before this task and is not part of T2-2d. |
| 5 | Verification gates green (MSVC) | **OK** — re-ran all four gates and four focused test suites; all pass (see Verification). |

## Spec compliance

- **S1 producer cleanup** (`sub_handle_state.rs:119-122` reminder injection + `sub_handle_out.rs:153-155` context_vars): both blocks deleted, `InternalReminderKind` import removed from `sub_handle_state.rs:21` (no longer needed), `submission_policy` let-binding removed because its sole consumer was the deleted block.
- **S2 context-var chain** (`turn_lifecycle.rs:104-120` + 10 execution/* files + `context_init.rs:214-217`): all 12 files cleaned; only the targeted imports / blocks removed.
- **S3 PromptBuilderContext** (`prompt_builder/mod.rs` field + builder + default; `system_prompt.rs:120-123` & `:257-260` dual-collapses; `tests.rs:222` test): all four deletions applied; two `format!(...)` collapses verified identical in both call sites.
- **S4 create_plan_tool** (import at `:6`, `use_computer_link` branch at `:217-224`, `computer_link` JSON field, helper inlining): exactly as specified — `workspace_relative_user_link` is inlined at the bottom of the impl block.
- **S5 `remote_file_delivery.rs` deletion + `mod.rs` declaration**: file removed (69 lines), module declaration removed from `agentic/mod.rs`. `rg "remote_file_delivery|computer_link|TOOL_CONTEXT_REMOTE_FILE_DELIVERY|needs_computer_links"` over `src/crates/assembly` returns 0 hits.
- **S6 boundary rules**: `scripts/core-boundaries/` contains no `remote_file_delivery` anchor (only Computer-use guidance rules which are unrelated); `node scripts/check-core-boundaries.mjs` is green.

## Quality

- Diff is minimal and on-scope: 13 insertions are (a) one-line `format!(...)` collapses (×2 in `system_prompt.rs`), (b) one-line `await?` collapse in `turn_lifecycle.rs`, (c) `let user_link = workspace_relative_user_link(...)` substitution in `create_plan_tool.rs`, and (d) the 7-line `workspace_relative_user_link` helper. No token churn in unrelated lines.
- Helper placement is correct: the brief explicitly says "workspace_relative_link 若 S3/S4 后仍有调用方则**内联保留**（可移入调用方所在模块）"; only `create_plan_tool` still needs the workspace-relative path computation, so the helper lives there. `system_prompt.rs` uses a plain `format!` because the input is already a pre-formatted workspace-relative string (`.northhing/sessions/{sid}/research/report.md`).
- No dead imports remain: every removed import was the sole consumer of the corresponding symbol; subsequent cargo check produces no `unused_imports` warnings in the touched files.
- No `tokio::select!` / cancellation / timeout code paths touched (house-rule 4 unaffected).

## Verification (independently re-run)

```
$ & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
    Checking northhing-core v0.2.10
    Checking northhing v0.2.10
    Checking northhing-acp v0.2.10
    Checking northhing-cli v0.2.10
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.07s
# exit 0; warnings are pre-existing in unrelated files

$ & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 48.32s
# exit 0

$ node scripts/check-core-boundaries.mjs
Core boundary check passed.

$ cargo test -p northhing-core --features product-full --lib prompt_builder
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 1015 filtered out

$ cargo test -p northhing-core --features product-full --lib create_plan
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1039 filtered out

$ cargo test -p northhing-core --features product-full --lib dialog_turn
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 1027 filtered out

$ cargo test -p northhing-core --features product-full --lib coordination
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 988 filtered out
```

`rg "remote_file_delivery|computer_link|TOOL_CONTEXT_REMOTE_FILE_DELIVERY|needs_computer_links" src/crates/assembly --glob "*.rs"` → 0 hits (S5 归零).

## Findings
None.

## Spec judgment
PASS. S1–S6 executed as specified; all five constraints hold; contracts/SSH/`unwrap_or(Bot)` are intact.

## Quality judgment
PASS. Mechanically coherent deletion; verification fully green; no opportunistic refactor; minimal diff surface (13 insertions, 173 deletions across 25 files, all traceable to the brief's deletion list plus the two helper/format! collapses that the brief explicitly authorises).

## Cannot-verify items — re-confirmed by independent runs

1. No frontend/desktop/serialization consumer of the removed `computer_link` JSON field — confirmed: `rg "computer_link" .` → 0 hits repo-wide.
2. Runtime deep-research link behavior — covered by `deep_research_report_link_defaults_to_workspace_relative_path` (passes) and the broader 25-test `prompt_builder` suite.
3. Feature-combination callers of the removed builder/module — confirmed: `rg "with_remote_file_delivery_channel|needs_computer_links_for_source|remote_file_delivery_channel" src` → 0 hits; the deleted module is no longer reachable from any feature gate.
4. Contracts / SSH / `subagent_ports.rs:113` working-tree diff — confirmed empty/zero-change via `git diff` against `9c14d22`.

## Evidence reviewed
- Spec: `.superpowers/sdd/task-t2-2d-brief.md`
- Implementer report: `.superpowers/sdd/task-t2-2d-report.md`
- Round 1 review: `.superpowers/sdd/task-t2-2d-review.md`
- Round 2 diff package: `.superpowers/sdd/task-t2-2d-diff.md` (BASE `9c14d22`, -U10)
- Independent re-runs: `cargo check --workspace`, `cargo check -p northhing`, `node scripts/check-core-boundaries.mjs`, four focused test suites, six `rg` sweeps over `src/`, `src/crates/assembly`, `tests/`, and the whole repo.