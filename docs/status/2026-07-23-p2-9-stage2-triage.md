# P2-9 Stage 2 — Core Boundary Violation Triage

Date: 2026-07-23
Checker: `node scripts/check-core-boundaries.mjs`
Scope: triage the ~230 boundary violations reported after Stage 1 (`3a2b170`); fix only
grep-verifiable stale rules; report real violations and architecture decisions without
touching source.

## Summary

| Metric | Count |
|---|---|
| Violations before | 230 |
| Violations after (this session) | 205 |
| Fixed via stale-rule repair | 25 |
| Self-test (`northhing_BOUNDARY_CHECK_SELF_TEST=1`) | PASS (before and after) |
| ENOENT / file-not-found in checker run | none |

Remaining 205 violations break down as:

| Classification | Count | Action this session |
|---|---|---|
| STALE-RULE — fixed | 25 | rules repointed / regex updated (grep-verified) |
| STALE-RULE — blocked by self-test anchor | 181 | reported only (cannot fix inside `rules/`) |
| NEEDS-DECISION | 13 | reported only (architecture decision required) |
| REAL-VIOLATION (symbol absent everywhere) | 4 | reported only (needs source fix) |
| Needs source-side verification | 7 | reported only (regression tests / registry symbols absent from expected file) |
| **Total accounted** | **230** | |

Changed files: `scripts/core-boundaries/rules/source/required-rules.mjs` only
(plus this report). No source code, no `checker.mjs`, no `self-test.mjs` modified.

## Methodology

1. Ran the checker, captured all 230 violations, grouped by `path :: rule-reason`
   (42 groups initially; 39 after the scheduler fix split one group into three).
2. Read `checker.mjs` to confirm the mechanism: the bulk of violations come from
   `requiredContentRules` → `checkRequiredContent(rule.path, …)`, which reads **one**
   file (`rule.path`) and reports `${reason}; ${pattern.message}` for every pattern
   regex that does **not** match that file's text.
3. Read `self-test.mjs`. Its `requiredContentContracts` loop (lines 2758–2775) pins
   each owner rule to a path: for `{path, contracts}` it requires
   `requiredContentRules` to contain a rule whose `path === path` **or**
   `path.startsWith(prefix + '/')` (prefix = path without extension), and whose
   combined pattern-regex sources contain every pinned contract string. **This anchor
   is the m27hs safeguard against silently loosening rules — and it is the key
   constraint on what can be fixed inside `rules/`.**
4. For each large group, inspected the target file and grep-verified (ripgrep) where
   the contract symbols actually live now.

## Root-cause finding: R25–R39 god-splits vs. the self-test anchor

Almost every source-content violation has the same structural cause. Owner files were
god-split (the in-source comments cite R25–R39) into smaller files, leaving the
original path as a thin re-export facade. The boundary rules still point at the facade
path and look for `pub struct …` / `pub fn …` **definitions**, which now live in sibling
files. Two split shapes occur:

- **Same-named subdirectory** (Rust 2018 child modules): `scheduler.rs` +
  `scheduler/sched_types.rs`, `scheduler/sched_state.rs`, `scheduler/sched_filter.rs`.
  The self-test anchor prefix `…/scheduler` **matches** the child files
  (`…/scheduler/sched_types.rs`.startsWith(`…/scheduler/`)), so the rule can be
  repointed to the child files without breaking the anchor. **Fixable inside `rules/`.**
- **Flat siblings** (via `#[path = "…"] mod …`, `pub use super::…`, or
  `<name>_<subdomain>.rs`): e.g. `runtime.rs` + `runtime_builder.rs`; `task_execution.rs`
  + `provider_capacity_queue.rs`; `storage.rs` + `storage_port.rs`; `manager.rs` +
  `service_impl.rs`. The self-test anchor prefix `…/runtime` (etc.) does **not** match
  `…/runtime_builder.rs`, so repointing the rule to the sibling **breaks the self-test
  anchor** (`missing owner content anchor rule for …`). Because `self-test.mjs` is
  outside the allowed change scope (`scripts/core-boundaries/rules/`), these stale rules
  **cannot be repaired here** without a coordinated self-test anchor update.

This is why only a small subset of stale rules is fixable in this task, exactly the
PARTIAL outcome the brief anticipates. The blocked stale rules are genuine rule debt,
but clearing them safely requires a follow-up task that updates `self-test.mjs`
anchors and the rules together (still grep-verified, still no loosening).

A second, smaller stale shape is **tightened feature gating**: a few `#[cfg]` rules
expected `#[cfg(feature = "service-integrations")]` but the gating was tightened to
`#[cfg(all(feature = "service-integrations", feature = "product-full"))]`. The boundary
intent (surface stays behind `service-integrations`, so no-default builds skip it) is
still satisfied — the regex was simply stale. These were updated to recognize the
stricter `all(…)` form (not a loosening).

## STALE-RULE — fixed this session (25)

### 1. agent-runtime scheduler owner (22 violations cleared)

Rule pointed at `src/crates/execution/agent-runtime/src/scheduler.rs`, now a 131-line
facade that re-exports three child modules. Split the single rule entry into three,
repointed to the child files; regexes and messages unchanged (no loosening).

Grep evidence (each symbol confirmed in its new owner file under `scheduler/`):

| Symbol (regex) | New location |
|---|---|
| `pub const DEFAULT_MAX_DIALOG_QUEUE_DEPTH`, `pub struct ActiveDialogTurn`, `pub enum AgentSessionReplyAction`, `pub struct AgentSessionReplyPlan`, `pub struct BackgroundDeliveryFacts`, `pub enum BackgroundDeliveryAction`, `pub enum BackgroundInjectionKind`, `pub enum DialogSteeringAction`, `follow_up_submission_policy`, `SubmitAgentSessionFollowUp`, `InjectIntoRunningTurn`, `pub enum TurnOutcome`, `pub enum TurnOutcomeQueueAction` | `scheduler/sched_types.rs` |
| `pub struct ActiveDialogTurnStore`, `pub struct DialogReplySuppressionSet`, `pub struct DialogTurnQueue`, `pub struct SessionAbortFlags`, `pub struct SessionRoundInjectionBuffer` | `scheduler/sched_state.rs` |
| `pub fn resolve_agent_session_reply_action`, `pub const fn resolve_background_delivery_action`, `pub fn resolve_background_delivery_injection`, `pub fn resolve_dialog_steering_action` | `scheduler/sched_filter.rs` |

Self-test stays green: the anchor entry for `…/src/scheduler.rs` matches rules under
`…/src/scheduler/`, and the combined regex sources still contain all 22 pinned contracts.

### 2. Tightened `#[cfg]` gating (3 violations cleared)

| Rule path | Pattern | Actual gating (grep evidence) |
|---|---|---|
| `assembly/core/src/lib.rs` | `pub(crate) mod service_agent_runtime` | `lib.rs:28` `#[cfg(all(feature = "service-integrations", feature = "product-full"))]` |
| `assembly/core/src/service/mod.rs` | `pub mod mcp` | `service/mod.rs:21` `#[cfg(all(feature = "service-integrations", feature = "product-full"))]` |
| `assembly/core/src/service/mod.rs` | `pub mod remote_connect` | `service/mod.rs:23` `#[cfg(all(feature = "service-integrations", feature = "product-full"))]` |

Each regex was changed from `#\[cfg\(feature = "service-integrations"\)\]` to
`#\[cfg\(all\(feature = "service-integrations", feature = "product-full"\)\)\]`. The
surface remains gated behind `service-integrations` (stricter than before), so the
boundary contract is preserved; the regex source still contains the pinned contract
strings (`feature = "service-integrations"`, `pub mod mcp`, etc.), so the self-test
passes.

## NEEDS-DECISION (13) — report only

These match the brief's explicit decision list (crate layout not on an approved layered
path; product-full coverage; optional-dependency ownership; default feature). They are
 Cargo/feature-graph decisions, not stale paths, and cannot be resolved by editing
content rules.

| # | Violation | Decision needed |
|---|---|---|
| 1–4 | `Cargo.toml`: workspace member must use an approved layered path — `services/relay-core`, `execution/agent-dispatch`, `test-support`, `cli-internal` | Approve these crate locations in the layered index or relocate the crates (`crate-layout.mjs` allowlist + `docs/status/surfaces.md`). |
| 5–6 | `cli-internal`, `test-support` must live under a layer directory, not directly under `src/crates` | Same layout decision as above. |
| 7 | `services-integrations` default profile forbids non-optional `async-trait` | Decide whether `async-trait` becomes optional/feature-owned or the default-profile rule changes. |
| 8 | `services-integrations` optional runtime deps must be feature-owned — `async-trait` must be optional; missing optional dep `northhing-agent-runtime` | Decide optional-dep/feature ownership for these integrations deps. |
| 9 | `northhing-core` missing optional dependency `northhing-relay-server` (product/runtime optional deps must be feature-owned) | Decide relay-server feature ownership in core. |
| 10 | `product-domains` missing optional dependency `log` | Decide `log` feature ownership in product-domains. |
| 11 | `desktop-tauri/src-tauri` depends on `northhing-core` but is not covered by product-full assembly rules | Decide whether desktop-tauri joins the product-full assembly rule set (product matrix). |
| 12 | `northhing-core` default feature must remain `product-full` | Product matrix review (explicitly called out in the rule message). |

## REAL-VIOLATION (4) — report only, needs source fix

Contract symbols that the rule (and the self-test anchor) require, but which exist
**nowhere** in `src` (ripgrep over all `*.rs`):

| Rule path | Missing symbol (regex) | Evidence |
|---|---|---|
| `execution/tool-contracts/src/framework/manifest.rs` | `pub fn get_tool_spec_input_schema` | not found anywhere in `src` |
| `execution/tool-contracts/src/framework/manifest.rs` | `pub fn get_tool_spec_short_description` | not found anywhere in `src` |
| `execution/tool-contracts/src/framework/manifest.rs` | `pub fn get_tool_spec_is_readonly` | not found anywhere in `src` |
| `execution/tool-contracts/src/framework/registry.rs` | `pub fn get_collapsed_tool_names` | not found anywhere in `src` (the registry exposes `collapsed_tool_names(&self)` / `is_tool_collapsed(&self)` instead) |

These rules are correct to flag; the pure GetToolSpec schema/description/readonly
contract helpers and the generic collapsed-tool catalog query are genuinely absent.
The self-test pins all four contract names, so the rules must keep them — the fix is a
source implementation (out of scope), not a rule edit.

## STALE-RULE — blocked by self-test anchor (181) — report only

God-split facades whose contract symbols were grep-verified to exist in sibling files,
but whose rule cannot be repointed inside `rules/` because the self-test anchor prefix
does not cover the flat siblings (see root-cause finding). Clearing these requires a
follow-up task that updates `self-test.mjs` anchors + rules together (grep-verified, no
loosening). Large groups (symbols verified present elsewhere):

| # | Rule path (facade) | Count | Symbols verified moved to (flat siblings) |
|---|---|---|---|
| 1 | `contracts/runtime-ports/src/lib.rs` (wildcard facade, R26) | 79 | `agent/agent_types.rs`, `agent/agent_thread_goal.rs`, `agent/mod.rs`, `session_workspace.rs`, `remote.rs`, `port_core.rs` (e.g. `pub trait AgentTurnCancellationPort`/`RemoteControlStatePort`/`RuntimeEventSink`, `pub struct CompressionContract`/`RelatedPath`, `pub struct DelegationPolicy`, `pub enum SubagentContextMode`, `pub trait WorkspaceFileSystem`, `pub struct ThreadGoal` all confirmed in submodules) |
| 2 | `execution/agent-runtime/src/deep_review/task_execution.rs` (wildcard facade over `super::`) | 20 | `deep_review/provider_capacity_queue.rs`, `retry_runtime.rs`, `reviewer_admission_queue.rs`, `task_completion_and_cache.rs`, `types.rs` |
| 3 | `execution/agent-runtime/src/runtime.rs` (`#[path]` siblings) | 13 | `runtime_builder.rs` (`AgentRuntimeBuilder`, `with_event_stream`), `runtime_event_stream.rs` (`AgentEventStream`), `runtime_types.rs` (`SessionSelector`, `AgentRunRequest`, `AgentRunHandle`), `tests.rs` (regression fns) |
| 4 | `assembly/core/src/service/cron/service.rs` | 10 | delegation moved to `cron/service_impl.rs` (`apply_due_scheduled_trigger`, `mark_enqueued`, `ScheduledJobEnqueueFailureAction`) |
| 5 | `assembly/core/src/agentic/persistence/manager.rs` | 9 | `persistence/metadata_subhandlers.rs`, `session_subhandlers.rs`, `paths_utilities.rs`, `turn_io.rs` (`session_metadata_store`, `ensure_runtime_for_write`) |
| 6 | `assembly/core/src/agentic/session/session_manager.rs` | 8 | `session/session_persistence/prompt_cache.rs` (`clone_prompt_cache`), `session/session_persistence/turn_lifecycle.rs` (`start_dialog_turn_with_existing_context`) |
| 7 | `assembly/core/src/agentic/coordination/coordinator.rs` | 8 | `coordination/session_ports.rs`, `ports.rs`, `subagent_ports.rs` (`SessionTranscriptReader`, `runtime_session_summary`); `AgentSubmissionPort` also in `service_agent_runtime/*` |
| 8 | `services/services-integrations/src/miniapp/storage.rs` | 6 | `miniapp/storage_imports_io.rs`, `storage_app_io.rs`, `storage_port.rs` (`MiniAppImportBundleWriteRequest`, `read_import_meta_json`, `write_import_bundle`, `impl MiniAppStoragePort`) |
| 9 | `assembly/core/src/agentic/execution/execution_engine.rs` | 4 | `execution/turn_tick.rs`, `turn_main_loop.rs`, `turn_lifecycle.rs` (`collect_product_unlocked_collapsed_tools`, `citation_renumber`) |
| 10 | `services/services-integrations/src/remote_ssh/workspace_search/service.rs` | 4 | `workspace_search/service_helpers.rs`, `repo_session.rs` (`RemoteWorkspaceSearchProvider`) |
| 11 | `assembly/core/src/service/remote_connect/bot/command_router.rs` | 3 | `bot/command_router_session.rs` (`build_remote_session_create_request`) |
| 12 | `assembly/core/src/service/workspace/service.rs` | 3 | `workspace/service_init.rs`, `admin.rs` (`prepare_startup_restored_workspaces`) |
| 13 | `services/services-integrations/src/remote_ssh/manager.rs` | 2 | `remote_ssh/manager_handler.rs`, `mgr_lifecycle_handlers.rs` (`russh::client::connect_stream`), `manager_tests.rs` (`prunes_password_connection_without_vault_entry`) |
| 14 | `interfaces/acp/src/client/manager.rs` | 2 | `client/manager_errors.rs`, `manager_transport.rs` (`startup_timeout_error_message`, `formats_startup_timeout_error_message`) |
| 15 | `assembly/core/src/service/workspace/manager.rs` (git-worktree gating + related-path re-export) | 3 | git worktree enrichment moved to `workspace/manager_lifecycle.rs`, `workspace_info_impl.rs` |
| 16 | singletons: `service/filesystem/service.rs`, `service_agent_runtime/mod.rs`, `tools/registry/registry_lookup.rs`, `tools/tool_context_runtime/context_format.rs`, `workspace_search/service.rs` (local flashgrep), `coordination/scheduler/scheduler_lifecycle.rs`, `coordination/scheduler/scheduler_turn/turn_submit.rs` (legacy import path) | 7 | same flat-sibling god-split pattern (inferred from the verified groups above; per-symbol grep recommended in the follow-up) |

Note on verification depth: groups 1–15 were grep-verified symbol-by-symbol (or via
representative symbols spanning the group). Group 16 singletons are classified by the
same structural pattern; a follow-up should confirm each symbol's new location before
repointing.

## Needs source-side verification (7) — report only

Rules whose target is a regression-test name / registry symbol that was **not** found in
the expected crate location; could be a real regression (test/symbol dropped) or a
completed migration to `agent-runtime`. Classify before fixing:

| Rule path | Count | Missing target | Note |
|---|---|---|---|
| `assembly/core/src/agentic/coordination/scheduler/scheduler_turn/turn_submit.rs` ("core scheduler keeps remote queue policy semantics until agent-runtime migration is reviewed") | 5 | `remote_queue_policy_preserves_confirmation_boundary` and four dialog-lifecycle attachment/reminder regression tests | not found in `assembly/core`; decide whether remote queue policy already migrated to `agent-runtime` (then update/remove the rule with review) or the regressions were dropped (real violation) |
| `assembly/core/src/agentic/tools/product_runtime/catalog.rs` | 2 | `get_global_tool_registry` (core product registry snapshot access), core agent policy source | `get_global_tool_registry` not found under `agentic/tools`; likely renamed/moved or real — verify before any rule change |

## Recommendations (follow-up tasks)

1. **Self-test-anchor refactor task** (largest payoff, ~181 violations): for each
   god-split owner, update the `self-test.mjs` `requiredContentContracts` anchor path
   and the matching `requiredContentRules` paths **together**, repointing to the
   sibling files where the definitions live. Keep every regex/contract identical
   (grep-verified at the new location) — no loosening. Suggest doing this crate by
   crate, starting with `runtime-ports` (79) and `agent-runtime` (33).
2. **Source fix task** for the 4 REAL-VIOLATIONs: implement the pure GetToolSpec
   schema/description/readonly helpers and the generic collapsed-tool catalog query in
   `tool-contracts` (or update both rule and self-test if the contract was intentionally
   retired).
3. **Architecture decisions** for the 13 NEEDS-DECISION items (crate layout allowlist +
   `docs/status/surfaces.md`, optional-dep/feature ownership, desktop-tauri product-full
   coverage, `northhing-core` default feature product-matrix review).
4. **Verification task** for the 7 "needs source-side verification" items: confirm
   whether remote queue policy / catalog snapshot migrated to `agent-runtime` (update
   rules with review) or regressed (source fix).

## Verification log

- Before: `node scripts/check-core-boundaries.mjs` → exit 1, 230 violations, no ENOENT.
- After scheduler fix: 208 violations; self-test PASS.
- After `#[cfg]` fixes: 205 violations; self-test PASS.
- Final: `node scripts/check-core-boundaries.mjs` → exit 1, 205 violations, no ENOENT.
- Final: `$env:northhing_BOUNDARY_CHECK_SELF_TEST='1'; node scripts/check-core-boundaries.mjs`
  → `Core boundary check self-test passed.`, exit 0.
- `git status --short scripts/core-boundaries/rules/` → only
  `M scripts/core-boundaries/rules/source/required-rules.mjs`.
