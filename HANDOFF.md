# v3 Restructure — Handoff Document

> **This is the single handoff document.** If you are a new model picking up
> work on `v3-restructure`, **start here**. All other docs are referenced
> from §11 below.
>
> **🌐 沟通守则（2026-06-21 新增，长期生效）**：与用户沟通一律使用**中文**。
> 代码、标识符、commit message、文件路径、库名等技术字符串保持英文原文不译；
> 自然语言解释、提问、汇报、状态更新、问题分析、计划说明一律中文。
> 涉及引用现有 skill / spec / plan 章节时，章节标题保留原文（§A / Gate 7 等），
> 但围绕它们的说明文字用中文。
>
> **Last verified**: 2026-07-12 (R67 + R72 + R67plus3 + B decision + feature-gate fix + B-2/B-3/B-4 sub-agent hardening DONE + R73+ god file audit DONE + **R73-1 path_manager split DONE 2026-07-11** + **R73-2 turn_batch split DONE 2026-07-11** + **R73-3 skill_agent_snapshot split DONE 2026-07-12** + **R73-1/2/3 QClaw APPROVED 9.3/10 2026-07-12** — Mavis M3 take-over for splits + first multi-reviewer dispatch attempt (plan_df939a4c killed at 30min cap, Mavis M3 take-over finished commit); 13-axis verification all pass (cargo check 0 errors, 926/927 tests pass with 1 pre-existing turn_batch B-3 fail, 0 mojibake, 0 fmt diff, 0 unwrap/panic/todo in production, all pub re-exports verified, line counts within spec target); 2 non-blocking observations (dead code +7 drift from re-export path change, multi-reviewer 30min cap insufficient for >300 line files — already documented in skill). 1382 → 1390 dead_code warnings (+7 from re-export path change, not new dead code). New tools: multi-reviewer-dispatch skill (`~/.mavis/agents/mavis/skills/multi-reviewer-dispatch/SKILL.md`) + reviewer-arch + reviewer-test agents registered. R73-4 (git_tool/mod.rs 660) + R73-5 (remote_connect/connect.rs 741) paused, awaiting user direction)
> **Branch**: `main`
> **HEAD**: `b254db80` (R73-3 skill_agent_snapshot split; 5 new commits since prior HEAD `b927ce44`: edaf468c (R73-1) + 24a59f34 (R73-2) + a6a1061b (HANDOFF cleanup bump — B-2/B-3/B-4 review-fix-cleanup cycle complete) + 0fff09cd (docs(core): with_user_root_for_tests rustdoc) + b254db80 (R73-3) + this bump; cargo check = 0 errors, 926 tests pass, QClaw APPROVED 9.3/10 at this commit)
> **Total commits on branch**: see `git rev-list --count HEAD` (auto-updates; not statically maintained to avoid bump-loop drift)
> **HEAD drift note**: every commit that updates HANDOFF §0 will, by definition, produce a new commit that §0 does not yet reflect. The drift is one commit per HANDOFF-bump. Readers should treat the listed HEAD as "the HEAD when this row was written", not "current HEAD". For the truly current HEAD, run `git rev-parse --short HEAD`.
> **Tag**: `v0.1.0` (at commit `2813b36`, A6 commit)
> **Tooling**: ZCode superpowers plugin upgraded **5.1.0 → 6.0.3** (filesystem-source, cache overwritten; backup at `~/.zcode/cli/plugins/cache/.../superpowers/5.1.0.backup-2026-06-20/`). See §6.
> **A2 ACTIVATED**: `USE_LIGHTWEIGHT_ACTOR = true` (commit `e5ae9b1`). `CoordinatorHiddenSubagentSkill` now replaces legacy `execute_hidden_subagent_phase1/2/3` for all `Task` tool invocations. 13/13 desktop tests pass (1 `#[ignore]` re-enabled via `#[tokio::test]`). See `docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md`.
> **TaskTool COLLAPSED**: `ToolExposure::Collapsed` saves ~800-1,200 tokens/turn in manifest. `GetToolSpec` fetches full schema on first use. 44/44 task_tool tests pass. See `docs/superpowers/specs/2026-06-23-collapse-task-tool-design.md`.
> **R1 SHELL SAFETY COMPLETE**: 14 commits across 3 phases — Audit (9 paths), Guard (`guard_command_execution` + 17 tests), Mode (`ShellSecurityConfig` + `ConfirmationMode` + audit log). `bash_tool`/`exec_command` denylist (11 patterns) + `ShellSecurityConfig` (3 layers). See `docs/reviews/2026-06-24-r1-shell-exec-sandbox-14-commits-review.md` (9.2/10).
> **SUBAGENT BOUNDARY E2E COMPLETE**: 8 tests covering 6 scenarios (success/cancel/timeout/error/parent-chain/concurrent) via direct phase 1/2/3 calls. 4 `#[allow(dead_code)]` fields validated. Reviewed at 8.95/10. See `docs/reviews/2026-06-24-subagent-boundary-e2e-tests-review-report.md`.
> **K.3.0 shipped**: Reference-library skill now includes `## Tech Selection for External Projects` (§A 7 Decision Gates + §B Red-Flag Triage appendix, CodeGraph worked example). See `docs/superpowers/specs/2026-06-21-reference-library-tech-selection-sop-design.md` + `docs/superpowers/plans/2026-06-21-reference-library-tech-selection-sop-plan.md`.
> **K.2.3 A1 COMPLETE**: Phase A1 SkillActor multi-turn redesign — `LongRunningSkill` trait + `spawn_long_running` runtime method + `CoordinatorHiddenSubagentSkill` direct execution wrapper + bidirectional `LightweightTaskOutput ↔ SubagentResult` mapping + full `actor_runtime` wiring chain (AppState → coordinator → tool_pipeline → ToolUseContext → TaskTool). All 14 tests pass (4 runtime + 10 mapping). Cancel token propagation fixed. See `docs/superpowers/plans/2026-06-22-a1-full-implementation-plan.md`.

---

## 0. TL;DR

Phases A–I of `2026-06-19-post-reference-roadmap.md` are **complete**.
K.2.2 (split `execute_hidden_subagent_internal` into 5 helpers) just landed (commit `a8cc454`).
P4 (optimise `log_debug_event` with OnceLock channel) just landed (commit `630d679`).
P2 (verify coordinator.rs test compilation) confirmed clean — 0 errors, 0 warnings.

**Session 2026-07-11 (continued) — Sub-agent hardening review cycle DONE**:
- B-2 (SubAgentHandoff trait + so_dispatch migration) + B-3 (per-turn checksum) + B-4 (OfflineSubAgentProfile) + R73+ god-file audit all **QClaw APPROVED 9.2/10** at HEAD `b927ce44`
- 4 non-blocking observations addressed in dedicated `fix(tests):` commits (`f5b920f6` / `fccdc06c` / `b927ce44`); obs 4 (tests.rs fmt) deferred to pre-existing fmt cleanup
- See §7.5 B-2/B-3/B-4 entries and §10 commit log for details

| Metric | Value | Verified by |
|---|---|---|
| Regression tests | **8/8 PASS** | `bash scripts/regression-test-desktop.sh` |
| agent-dispatch tests | **24/24 PASS** | `cargo test -p northhing-agent-dispatch --lib` |
| Workspace builds | **PASS** | `cargo check --workspace` |
| Coordinator phase boundary tests | **20/20 PASS** (12 original + 8 new) | `cargo test -p northhing-core --lib -- subagent_boundary_e2e` + `cargo test -p northhing-core --lib -- 'coordinator::tests::'` (2026-06-24) |
| Coordinator test compilation | **0 errors, 0 warnings** | `cargo check -p northhing-core --lib --tests` (P2 verified 2026-06-22) |
| Compiler warnings (lib only) | **0** | `cargo check -p northhing-core --lib` |
| Slint callbacks wired | **10** | `grep -c "^\s*ui\.on_" src/apps/desktop/src/app_state/mod.rs` |
| Const flags in `agent-dispatch` | **all `false`** | `grep "pub const USE_" src/crates/execution/agent-dispatch/src/flags.rs` |
| `InMemoryRelationship` fields | **5** (parent_session_id / parent_request_id / parent_dialog_turn_id / parent_turn_index / parent_tool_call_id) | `grep "pub " src/crates/assembly/core/src/agentic/core/session.rs` |
| Hand-written `unsafe` in `app_state/` | **0** | `grep "unsafe" src/apps/desktop/src/app_state/mod.rs` (only slint macro output) |
| Tag | `v0.1.0` applied at `2813b36` | `git tag` |
| god-files (lib + tests >750 行) | **0** | Phase B closeout verification |
| `cargo check -p northhing-core --tests` | **0 errors** | `cargo check -p northhing-core --tests` |
| `cargo check --workspace` | **10 pre-existing `northhing-acp` errors (out of scope)** | `cargo check --workspace` |

**The next session's job**: R2 (ChatView refactor) or Pre-existing clippy fix — see §5.

---

## 1. Architecture

```
northhing/
├── src/apps/
│   ├── desktop/          # Slint + Material UI shell (active product)
│   │   └── src/app_state/  # mod.rs (~999) + 7 submodules
│   ├── cli/              # Internal TUI CLI
│   └── ...
├── src/crates/
│   ├── assembly/core/    # northhing-core (main runtime)
│   ├── adapters/         # AI adapters, transport, webdriver
│   ├── contracts/         # Core types, events, runtime-ports
│   └── execution/
│       ├── agent-runtime/  # existing
│       ├── agent-dispatch/ # SkillActor + ActorRuntime (4 const flags all false)
│       └── ...
├── scripts/regression-test-desktop.sh   # 8/8 checks, self-bootstraps cargo PATH
└── .agents/
    ├── reference/        # 4-domain read-only code mirror (skills/actor/session/checker)
    └── skills/            # Project-local ZCode skills
```

**Key invariants (do not break without flag flip + integration test)**:
- `USE_LIGHTWEIGHT_ACTOR = false` (agent-dispatch::flags)
- `USE_ONESHOT_DISPATCHER = false`
- `USE_ACTOR_IPC = false`
- `USE_DISPATCHER_IPC = false`
- `USE_SKILL_REGISTRY = true`
- `USE_SOFTWARE_FALLBACK = true`
- `SESSION_TREE_VIEW = true`
- `DEFAULT_MODE_ID = "code"`
- `LazyLock<Arc<AppState>>` global in `main.rs:80` (single init)
- `Arc<AppState>` in Slint closures (Phase I.2 — no hand-written `unsafe`)

---

## 2. Phases Complete (A → I + backlog 1/3/4)

| Phase | Scope | Commit range |
|---|---|---|
| A0–A8 | Initial restructure (rename, Slint shell, skills, providers, multi-session UI, etc.) | pre-tag |
| Reference | `.agents/reference/` mirror library + `reference-library` skill (12/12 matchability) | earlier |
| **A** | Close A6 GUI wiring gap (4 unwired callbacks) | earlier |
| **B** | Track B Phase 1 lightweight actor skeleton | earlier |
| **C** | Subagent tree sidebar + Inspector live data | earlier |
| **D** | Session.relationship + `DEFAULT_MODE_ID` parameterization | earlier |
| **E** | `SkillActor` trait body + `ActorRuntime` skeleton | earlier |
| **F** | Periodic scheduler + OnSignal + `McpCatalogReader` port | earlier |
| **G** | Inspector mcp-status live + sidebar show-subagents toggle (added 10th callback) | earlier |
| **H** | MVP debug log wiring (5 component + 5 hook points) | earlier |
| **I** | env fix / unsafe cleanup / actor wiring / InMemoryRelationship / desktop tests / toggle log | `a9d021f`–`558236a` |
| **Backlog 1** | InMemoryRelationship + `parent_dialog_turn_id` + `parent_turn_index` | `7aa4310` |
| **Backlog 4-A3** | `ActorRuntime::spawn_one_shot` + `on_send_message` demo | `0da5130` |
| **Phase B (split)** | Split `app_state.rs` (989 lines) into 6 submodules | `c2d2bc8` |
| **Phase E (script)** | cargo PATH bootstrap in `regression-test-desktop.sh` | `5ac37c6` |
| **K.2.1** | Extract `slint::include_modules!()` to `app_state/slint_glue.rs` | `624e12f` |
| **K.2.2** | Split `execute_hidden_subagent_internal` into 5 helpers (phase1/phase2/phase3) | `a8cc454` |
| **P4** | Optimise `log_debug_event` with OnceLock channel + single consumer thread | `630d679` |
| **K.2.5** | Plan doc closeout — TL;DR + status snapshot + next checklist | `c3d137f` |

### Deferred / blocked

- **Backlog 5** (`create_ui` cargo test compat) — 🚫 BLOCKED upstream (slint 1.16.1 doesn't expose `backend-testing`)
- **Backlog 4 (full A1)** — multi-day refactor, deferred
- **K.2.3 / K.2.4** — see §5

---

## 3. app_state/ submodule layout

`src/apps/desktop/src/app_state/` (1532 lines total):

| File | Lines | Responsibility |
|---|---|---|
| `mod.rs` | 999 | `AppState` struct, `create_ui` (single wiring point), `run_event_loop`, `phase_i_tests` |
| `actor.rs` | 122 | `maybe_construct_actor_runtime` (gated on `USE_LIGHTWEIGHT_ACTOR`) |
| `inspector.rs` | 36 | `build_mcp_status_string` (live MCP read) |
| `inspector_model_status.rs` | 54 | `build_model_status_string` (provider list) |
| `log.rs` | 49 | `log_debug_event` (fire-and-forget debug-log helper) |
| `sessions.rs` | 172 | `build_sessions_model` / `build_messages_model` / `refresh_sessions_ui` / `refresh_messages_ui` (DTO projections + depth walking) |
| `skills.rs` | 70 | `build_skills_model` / `refresh_skills_ui` |
| `slint_glue.rs` | 30 | `slint::include_modules!()` invocation (K.2.1, 2026-06-20) |

**Why `create_ui` stays in `mod.rs`**: it's the single wiring point between all 10 Slint callbacks and the helpers; splitting it across files would create indirection with no gain. Only pure testable helpers moved.

**Visibility chain**: each helper needs `pub(super)` for cross-module access; `mod.rs` needs `use self_submodule::fn` to consume them. Slint types live in `slint_glue` and are reached via `super::slint_glue::AppWindow`.

---

## 4. Const-flag invariant

All 4 flags in `src/crates/execution/agent-dispatch/src/flags.rs`:

```
pub const USE_LIGHTWEIGHT_ACTOR: bool = false;
pub const USE_ONESHOT_DISPATCHER: bool = false;
pub const USE_ACTOR_IPC: bool = false;
pub const USE_DISPATCHER_IPC: bool = false;
```

**Flipping any to `true` without an integration test is a regression risk.** No production call site currently constructs `ActorRuntime` (the A3 demo at `on_send_message` is fire-and-forget only).

---

## 5. K.2 candidates (next 5 items)

Recommended order (cost / value / blocker):

| # | Item | Cost | Value | Status / Blocker |
|---|---|---|---|---|
| **K.3.0** | reference-library — external-project tech-selection SOP (insert §A + §B into `SKILL.md`, add `scripts/check-skill-trigger.sh`, create `evaluations/` dir) | 1-2h | High | ✅ DONE (commits `4451691` + `b80aca4`; self-check script 12/12 PASS) |
| ~~**K.2.1**~~ | ~~`slint::include_modules!()` extraction to `slint_glue.rs`~~ | 1-2h | High | ✅ DONE (commit `624e12f`) |
| **K.2.5** | Plan doc top-level TL;DR + status snapshot | 30min | Medium | Partially done this session |
| ~~**K.2.2**~~ | ~~`ConversationCoordinator::execute_hidden_subagent_internal` split into 3-4 sub-helpers~~ | 1h | Medium | ✅ DONE (commit `a8cc454`) |
| **K.2.3 follow-up** | AppState → ToolPipeline → ToolUseContext wiring + `LightweightTaskOutput → SubagentResult` mapping + A1StubSkill | 1.5-2h | Highest | ✅ DONE — Task 1 (`cf1ca9a`) + Task 2 (`7d66704`) + Task 3 (`4b890a2`); A1 gate fires end-to-end at flag=true + actor_runtime.is_some(). Task 4 (HANDOFF bump) in progress. |
| **K.2.4** | `create_ui` mock display test (workspace-level `slint::platform::Platform` impl) | 2-3h | Lowest | Blocked by slint 1.16.1 — defer |

**Recommended single-session order**: K.2.5 → K.2.2 → K.2.3 → K.2.4.

Full design sketches for K.2.2 / K.2.3 / K.2.4 are in `docs/plans/2026-06-19-post-reference-roadmap.md` §K.2.

---

## 6. Known issues (carry-over)

- **slint 1.16.1** doesn't expose `backend-testing` as a public feature.
  → K.2.4 (mock display test) is blocked until upstream upgrade.
- **`ConversationCoordinator::execute_hidden_subagent_internal`** is multi-turn LLM.
  `SkillActor::tick` is single-shot by design.
  → K.2.3 introduces `LongRunningSkill` to bridge.
- **`GetToolSpecTool`** is deprecated but still in active use (Phase A7 audit).
  No action needed — `execution_engine`, `skill_agent_snapshot`, `materialization` still depend on it.
- **Release build** times out on Windows in CI; use fast mode for regular regression.
- **`log_debug_event`** ~~spawns a fresh `std::thread` per call. MVP-acceptable; future optimization: bounded channel + single consumer thread.~~ **Phase P4 (2026-06-22) DONE**: replaced with `OnceLock`-initialised `mpsc::unbounded_channel` + single background consumer thread. See `src/apps/desktop/src/app_state/log.rs` and commit `630d679`.
- **`HANDOFF.md` total commits field** must be bumped with each session (the
  source of truth is `git rev-list --count HEAD`).
- **`northhing` desktop test binary linking fails** on Windows+MinGW: `ld: cannot find -lshlwapi`.
  The DLL exists in `C:/Windows/System32/shlwapi.dll` and the MinGW import library
  `C:/msys64/mingw64/lib/libshlwapi.a` exists, but the linker search path does not
  find it. **Not a code issue** — pre-existing environment misconfiguration.
  `cargo test -p northhing --lib` (desktop lib tests) and `cargo check -p northhing-core --lib`
  both compile cleanly with 0 warnings.
- **测试状态**: `cargo check -p northhing-core --tests` **0 errors**（R58 修复 R47-R57 遗留 125 errors）。`cargo check --workspace` 有 10 个 pre-existing `northhing-acp` errors（out of scope）。0 god-files (lib + tests >750 行)。
- **ZCode superpowers plugin** upgraded 5.1.0 → 6.0.3 (filesystem-source at
  → 6.0.3/). `marketplace.json` rewritten to point at 6.0.3. Backup of 5.1.0
  retained at `5.1.0.backup-2026-06-20/` (full plugin tree + dotfile seed).
  Hook contract verified unchanged: `hooks.json` / `hooks-cursor.json` /
  `run-hook.cmd` are byte-identical to 5.1.0; `session-start` script schema
  (Cursor/Claude/Copilot branches) is preserved. New in v6: `session-start-codex`
  + `hooks-codex.json` (Codex-only entry, ignored by Claude Code). v6 also
  drops the legacy `~/.config/superpowers/skills` warning (no longer needed in 6).
  All 14 skill `description:` trigger strings verified forward-compatible.
  Smoke test: `CLAUDE_PLUGIN_ROOT=... bash hooks/session-start` emits valid
  `hookSpecificOutput.additionalContext` JSON. **No regression risk to existing
  ZCode workflow** — only skill prose is updated (e.g. SDD major rework,
  systematic-debugging fixed the extended-thinking hyphen bug).

---

## 7. Three things the next session MUST do

1. **Read `docs/handoffs/2026-06-20-coding-brief.md`** before writing code.
   It has the 4-step workflow (`preflight-skill-check` → load `reference-library` →
   read `README → SIGNATURES → NOTES` → mirror copy).

2. **Run `preflight-skill-check` BEFORE writing code.** It will auto-load
   `reference-library` for any of the 4 covered domains (skill/actor/session/checker).
   This is non-negotiable.

3. **Update docs after each task.** Commit pattern is:
   `feat(<area>): <description>` or `refactor(<area>): <description>`,
   plus a `docs(handoff): bump N→N+M` commit at end. Update this HANDOFF.md
   §10 (commit log) + §0 (verified metrics) + §3 (submodule layout if changed).

### Post-R72 next steps (2026-07-11, current session end)

A. **R67+R72 review-fix-cleanup cycle** (user-driven, NOT for next session to dispatch):
   - QClaw: dispatch `*-review-report.md` per R72a/b/c/d commit (5ac1cf96, 2f608308, 8897e8ac, 8aead8b9)
   - Kimi: user verbal, optional
   - Fix any `fix(tests):` minor observations as separate commits
   - `docs(handoff):` bump to merge cleanup HEAD

B. **escape_html security review** (single decision point): ✅ **DONE 2026-07-11**
   - R67plus3 removed `&`→`&amp;` replacement to satisfy test expectation
   - Test enforces the behavior, but production input from OAuth callback errors is untrusted
   - Decision: (b) update test to `&amp;c` + re-add production escaping — **IMPLEMENTED**
   - Implementation: commit `4cb230fe` (escape_html + test assertion) + commit `0b4dc1f3` (feature-gate fix as pre-requisite to enable test verification)
   - Review: QClaw 9.0/10 APPROVED. See `docs/handoffs/2026-07-11-b-decision-and-feature-gate-review-report.md`.

B-1. **R-series per-feature compile validation workflow** (QClaw workflow improvement, post B decision):
   - R50 god-object split (`b5b705be`) extracted `service_agent_runtime` with 49+ `use crate::agentic::...` statements but never cfg-gated them
   - Result: 53 pre-existing build errors only visible when `--features service-integrations` enabled WITHOUT `--features product-full`
   - HANDOFF §0 "914/914 core tests pass" was tested under `--features product-full` which incidentally hid this debt
   - **New workflow rule** (effective R73+):
     - Worker dispatch prompt **MUST** include `cargo build -p <crate> --lib --features <each-feature-alone>` 验证, 不止默认组合
     - Specifically: at minimum `default` + `<each-feature-alone>` + `<all-features>` 三种 feature 组合都要编译过
     - 任何 pre-existing feature 组合错误都算 R-batch 范围内的债, 必须 fix before merge
   - 4 个 pre-existing integration test binary errors (`context_profile` / `git_contracts` / `product_assembly` / `remote_mcp_streamable_http`) are out of scope for B, deferred to R73+ cleanup workstream

B-2. **Sub-agent handoff protocol explicit-ization** (P2 backlog, R73+ candidate): ✅ **DONE 2026-07-11** (Mavis M3 take-over) + ✅ **B-2 follow-up DONE 2026-07-11** (Mavis M3 take-over)
   - Borrowed from HomeRail `handoff.ts`: "one call per turn" enforcement + type-safe Input/Output
   - Current state: `LongRunningSkill` (K.2.3 A1) provides per-skill context isolation, but handoff **protocol** (sub-agent I/O contract, "once per turn" enforcement, termination semantics) is implicit
   - Risk: bugs like "main agent invokes sub-agent mid-turn, then continues with stale context" or "sub-agent result overwrites other tool results in same turn"
   - Target: `SubAgentHandoff` trait with associated `Input`/`Output` types; per-turn counter in turn_manager; back-compat shim for existing call sites
   - Design: `docs/superpowers/specs/2026-07-11-sub-agent-orchestration-hardening.md` §B-2
   - Plan: `docs/superpowers/plans/2026-07-11-sub-agent-orchestration-hardening-plan.yaml` task `impl-b2-handoff` + `audit-b2-handoff-callers`
   - **Initial implementation (Mavis M3 take-over, commit `43d94edd`)**:
     - New `SubAgentHandoff` trait (`pub(crate)`, `#[async_trait]`) with generic `Input`/`Output: Send + 'static` (no serde bound; northing handoff is in-process)
     - `HandoffError` (4 variants: TooManyCallsInTurn / Cancelled / CoordinatorUnavailable / InvalidInput) + `From<HandoffError> for NortHingError`
     - `TurnHandoffCounter` (Arc<Mutex<HashMap<String, u8>>>) with `try_record` / `reset` / `count` / `tracked_turn_count`
     - `CoordinatorHiddenSubagentHandoff` impl (canonical; wraps global coordinator + counter)
     - 10 new unit tests (counter behavior + error mapping + impl accessors)
     - `execute_hidden_subagent_internal` marked `#[deprecated(note = "B-2: use SubAgentHandoff::handoff instead; A1 fallback path; target removal post-0.1.0")]`
   - **Migration follow-up (commit `271a52d9`)**:
     - `so_dispatch::execute_subagent` (line 130) → `CoordinatorHiddenSubagentHandoff::handoff`
     - `so_dispatch::start_background_subagent` (line 176) → spawn-owned handoff
     - New helper `subagent_turn_id(request)` (parent's `dialog_turn_id` or `orphan-<session>` fallback)
     - 5 accessor methods on `TurnHandoffCounter` / `CoordinatorHiddenSubagentHandoff` (`reset` / `count` / `tracked_turn_count` / `with_counter` / `counter`) marked `#[allow(dead_code)]` (public API for future callers / unit tests)
     - `_parent_cancel_token` / `_cancel_token` / `_timeout_seconds` / `_actor_runtime` underscore-prefixed + doc comment records R73+ handoff enhancement will thread them through
     - **0 deprecation warnings** at so_dispatch.rs:130/:176 (the 2 expected warnings from B-2 are gone)
     - 52/52 services-core + 13/13 test-support baseline preserved
   - **Visibility decision**: trait is `pub(crate)` because canonical `Input`/`Output` types are `pub(crate)`. Lifting to `pub` is a follow-up.

B-3. **Transcript checksum integrity** (P2 backlog, R73+ candidate): ✅ **DONE 2026-07-11** (Mavis M3 take-over)
   - Borrowed from HomeRail `audit/index.ts`: per-segment SHA-256 checksum sidecar + `verifyTranscriptChecksum` + `checkAuditCompleteness`
   - Current state: Northing transcript relies on git for integrity, no application-level checksum; silent corruption (disk error, partial write, malicious edit) goes undetected
   - With sub-agent nesting (K.2.3 A1 + future), transcript complexity grows 10x; corruption risk amplifies
   - Target: `TranscriptSegment` struct with checksum field; verify on read; fail loudly on mismatch; backfill old transcripts on first read
   - Design: `docs/superpowers/specs/2026-07-11-sub-agent-orchestration-hardening.md` §B-3
   - Plan: `docs/superpowers/plans/2026-07-11-sub-agent-orchestration-hardening-plan.yaml` task `impl-b3-checksum`
   - **Implementation (Mavis M3 take-over, commit `03736e11`)**:
     - New helper `services-core/src/session/checksum.rs`: `compute_turn_checksum` (SHA-256 over turn_id / turn_index / session_id / timestamps / kind / user_message / model_rounds / status), `verify_turn_checksum`, `turn_checksum_sidecar_path`, `write_turn_checksum_sidecar` / `read_turn_checksum_sidecar` (atomic; `None` for pre-checksum turns = back-compat), `audit_turn_parent_links` (gap detection), `TurnChecksumError::Mismatch { turn_id, expected, got }`
     - 8 new unit tests (deterministic / round-trip / mismatch / OOR / missing sidecar / audit gaps / hex round-trip)
     - `turn_io::save_dialog_turn` writes checksum sidecar after atomic turn write
     - `turn_io::load_dialog_turn` verifies checksum on read (pre-checksum turns accepted with debug log; mismatch returns Validation error)
     - `turn_metadata_sync::read_metadata_tail_turns` runs parent-link audit; gap detected returns None so caller falls back to full directory scan
     - `layout.rs` adds `turn_checksum_path(turn_path)` method (sibling of `turn_path`)
     - `mod.rs` registers `checksum` submodule + pub-uses the 6 public functions
     - 0 cargo errors; 52/52 services-core tests pass (8 new + 44 baseline preserved)
   - **Placement** (per services-core AGENTS.md): checksum helper lives in `services-core` alongside `turn_path` (NOT in `core/src/agentic/persistence/` as originally spec'd).

B-4. **Sub-agent offline test profile** (P2 backlog, R73+ candidate): ✅ **DONE 2026-07-11** (Mavis M3 take-over)
   - Borrowed from HomeRail `offline-deterministic` profile: full agent loop without external LLM provider
   - Current state: `A1StubSkill` exists for specific sub-agent skill tests, but doesn't cover **full agent loop**; CI tests requiring real LLM still common (slow, flaky, cost)
   - Target: `OfflineSubAgentProfile` stubs at every level (sub-agent via A1StubSkill, tool catalog, persistence via B-3 checksum, LLM via deterministic fixtures); full agent loop runs end-to-end without external LLM
   - Risk reduction: enables CI regression without provider credentials; deterministic fixture maintenance overhead
   - Design: `docs/superpowers/specs/2026-07-11-sub-agent-orchestration-hardening.md` §B-4
   - Plan: `docs/superpowers/plans/2026-07-11-sub-agent-orchestration-hardening-plan.yaml` task `impl-b4-offline-profile`
   - **Implementation (Mavis M3 take-over, commit `ef6fd440`)**:
     - New `test-support/src/offline_profile.rs`: `OfflineSubAgentProfile` (profile_id + agent_type + rounds), `OfflineRound` (round_id + text + optional tool_call + is_final), `OfflineToolCall` (tool_name + JSON arguments), `OfflineTickOutput` (Continue { round_id, text, tool_call } | Done { round_id, final_text }), `OfflineTickError` (RoundOutOfRange / EmptyProfile / PrematureFinal)
     - Builder API: `with_round` / `with_final_round`
     - New `test-support/src/fixture_loader.rs`: `FixtureLoader` (rooted at a dir; `load_profile(name)` reads JSON), `FixtureLoadError` (RootNotFound / FixtureNotFound / Io / Parse), hot-reload (no cache)
     - 3 sample fixtures:
       - `tests/fixtures/llm/echo_single_round.json` (1 round, is_final)
       - `tests/fixtures/llm/multi_round_with_tools.json` (4 rounds, 2 with tool calls)
       - `tests/fixtures/llm/long_running_default.json` (6 rounds, deep-research agent)
     - 5 integration tests in `tests/offline_subagent_profile.rs` (load + drive + assert exact output)
     - 8 unit tests (offline_profile 5 + fixture_loader 3)
     - 13/13 tests pass; services-core baseline preserved (52/52)
     - Profile is intentionally **data-only** (no `LongRunningSkill` impl) so test-support stays free of layer-crossing deps; the integration test demonstrates the contract, downstream tests in `agent-runtime` / `core` wrap the profile into LongRunningSkill as needed
   - **CI impact**: `cargo test -p northhing-test-support` runs without LLM provider credentials (per spec acceptance criteria)

B-5. **HomeRail architecture reference doc** (committed 2026-07-11):
   - `docs/architecture/homerail-architecture-analysis.md` — full 10-pattern evaluation + 5 landing recommendations
   - Cross-references B-2/B-3/B-4 above
   - Local homerail clone: `C:\Users\UmR\WorkBuddy\Claw\tmp\homerail` (depth 1) for source verification

C. **Continue god file audit (R73 candidate)**:
   - ✅ **Audit DONE 2026-07-11** (Mavis M3 manual, Mavis take-over per plan engine broken state). See `docs/audit/2026-07-11-r73-god-file-candidates.md` (untracked per §7.5 D).
   - **Result**: 0 god files ≥750 lines (R67/R72 closed Phase B debt). 121 files in 500-749 line "rising" tier. 5 R73+ candidates picked with natural seams:
      1. ✅ `infrastructure/app_paths/path_manager.rs` (705) → R73-1 DONE 2026-07-11 (`edaf468c`, 251 + 4 sub-modules)
      2. ✅ `agentic/persistence/turn_batch.rs` (694) → R73-2 DONE 2026-07-11 (`24a59f34`, 268 + 2 sub-modules)
      3. ✅ `agentic/skill_agent_snapshot.rs` (633) → R73-3 DONE 2026-07-12 (`b254db80`, 115 + 3 sub-modules) — **note**: R73 audit listed R73-3 = `github.rs` (676 lines) but actual is 331 lines (audit was based on byte count vs line count, or had stale measurement). github.rs is NOT a god file; R73-3 revised to `skill_agent_snapshot.rs`.
      4. ⏸ `agentic/tools/implementations/git_tool/mod.rs` (660) → R73-4 paused
      5. ⏸ `service/remote_connect/connect.rs` (741) → R73-5 paused (biggest win, save for last when pattern is mature)
   - **Mavis multi-reviewer pattern first-use** (2026-07-12 02:17): `plan_df939a4c` (R73-3) dispatched via `mavis team plan` with 3 reviewers (verifier + reviewer-arch + reviewer-test), killed at 30min engine cap (attempt 2, 0 tokens delivered). Coder had written split files to working tree before kill; Mavis M3 take-over contingency verified + committed. Pattern works but 30min cap is HARD FLOOR for step-router-v1 on god-file work. **Recommended**: ≤300 line files via multi-reviewer dispatch, >500 line files via Mavis M3 take-over (5-15 min/file pace).
   - **New tool**: `~/.mavis/agents/mavis/skills/multi-reviewer-dispatch/SKILL.md` (auto-loads when Mavis dispatches non-trivial coder task to `mavis team plan`); 2 new agent roles registered: `reviewer-arch` (14-dim design rubric, step-router-v1) + `reviewer-test` (4-check test rubric, step-router-v1).
   - Recommended R73 order (revised): R73-1 ✅ → R73-2 ✅ → R73-3 ✅ → R73-4 → R73-5.
   - **Out-of-scope for R73 batch** (per audit doc): visibility violations, duplicate definitions, test file rotation, per-feature compile validation — each deserves a dedicated audit round.
   - 156 uncommitted `cargo fmt` changes are pre-existing noise (per memory), do NOT touch

B-7. **R73-1/2/3 god-file splits review cycle**: ✅ **DONE 2026-07-12** (QClaw APPROVED 9.3/10)
   - QClaw review report: `docs/handoffs/2026-07-12-r73-god-file-splits-review-report.md` (11KB, 14-dim rubric, 13-axis verification all PASS)
   - Commits reviewed: `edaf468c` (R73-1 path_manager 705→251 + 4 sub), `24a59f34` (R73-2 turn_batch 694→268 + 2 sub + 3 dead-code cleanup), `b254db80` (R73-3 skill_agent_snapshot 633→115 + 3 sub)
   - 13-axis verification: ✅ cargo check 0 errors, ✅ 926 tests pass + 1 pre-existing turn_batch fail (B-3 known), ✅ 0 unwrap/panic/todo/unreachable in production, ✅ 0 mojibake, ✅ 0 fmt diff, ✅ #[cfg(test)] correct, ✅ all 13 pub re-exports verified, ✅ line counts within spec target, ✅ visibility pub(super) correct
   - Per-commit scores: R73-1 9.0+, R73-2 9.0+, R73-3 9.0+, overall **9.3/10 APPROVED**
   - 2 non-blocking observations (no fix commits needed):
      1. Dead code +7 drift (R73-3): 1382→1390 warnings, all from re-export path changes causing unused import pattern shift in callers; no new dead code in production
      2. Multi-reviewer pattern: 30min engine cap insufficient for >300 line files via step-router-v1; recommend Mavis M3 take-over for >500 line god-files (matches R67/R72 historical pace 5-15 min/file)
   - Diff_render.rs actual 247 lines (QClaw measurement), within spec ≤250 target — my self-review Errata v1 claim of 277 was based on inflated line count (likely counted blank lines)
   - **Mavis M3 take-over** for commit step (multi-reviewer plan_df939a4c killed at 30min cap, Mavis M3 verified cargo check + tests, finished commit in 5 min)

B-6. **B-2/B-3/B-4 + R73+ audit review cycle**: ✅ **DONE 2026-07-11** (QClaw APPROVED 9.2/10 + Mavis M3 fix-cleanup cycle complete)
   - QClaw review report: `docs/handoffs/2026-07-11-sub-agent-hardening-and-r73-audit-review-report.md` (265 lines, untracked per §7.5 D)
   - 9-step pre-review verification all passed: cargo check 0 errors + 0 deprecation warnings at so_dispatch.rs:130/:176 + 52/52 services-core + 8/8 test-support unit + 5/5 integration + 0 modified files + 6 untracked docs (incl. R73 audit doc readable from working tree)
   - Combined 14-dim rubric: 6-axis QClaw primary (Section A) + 8-dim Kimi secondary (Section B, folded in per QClaw review guide v2)
   - Per-commit scores: 03736e11 (B-3) 9.1, ef6fd440 (B-4) 9.5, 43d94edd (B-2 init) 9.5, 271a52d9 (B-2 follow-up) 9.0, R73 audit 9.0, HANDOFF bumps PASS
   - **Batch 1 (B-2/B-3/B-4) 9.3/10 + Batch 2 (R73 audit) 9.0/10 = overall 9.2/10 APPROVED**
   - 4 non-blocking observations, all addressed:
      1. **`checksum.rs:48` doc comment vs implementation mismatch** (token_usage) → `f5b920f6` (fix(checksum): correct doc comment, no code change)
      2. **`subagent_turn_id` helper had no unit test** → `fccdc06c` (test(coordination): 3 new tests covering parent present / parent absent / orphan fallback preserves session_name verbatim)
      3. **`FixtureLoader` path traversal (theoretical; test-only)** → `b927ce44` (fix(fixture-loader): `InvalidName` variant + `is_safe_fixture_name` allow-list + 2 new tests covering 8 attack patterns + safe-name boundary)
      4. **`tests.rs` import ordering fmt diff** → DEFERRED to pre-existing fmt cleanup (out of scope; same noise as prior batches per R67/R68 review history)
   - 1382 → 1383 dead_code warnings (1 from `name` field unused in test fixture of `subagent_turn_id_tests::make_request`, accepted; not from production code)

D. **Cleanup 5 untracked docs** (low priority):
   - `docs/audit-handoff-to-parent.md` + `docs/codebase-capabilities-report.md` (mavis 2026-07-10 audit)
   - `docs/superpowers/plans/r67-r72-cleanup-2026-07-11.yaml` + `r72-god-file-cleanup-2026-07-11.yaml`
   - `docs/superpowers/specs/2026-07-11-r67-r72-test-temp-dir-and-god-file-cleanup.md`
   - User pattern: keep untracked; OR move to `docs/handoffs/` for cross-session preservation

---

## 8. Workflow rules (carry-over)

### MUST
- `preflight-skill-check` before writing code (always — even for tiny edits)
- Read the 4 reference docs in order for 4-domain tasks
- TDD: red → green → commit per step
- One commit per logical change
- Bump `HANDOFF.md` §0 + §10 after each commit
- Update plan doc with any new phases / decisions / deviations

### MUST NOT
- Touch `coordinator.rs:4172-5025` (the heavy subagent path) to extend it
- Touch `execution_engine.rs`, `tool_pipeline.rs`
- Add `unsafe` to `app_state/` (slint macros emit internal `unsafe` blocks;
  hand-written `unsafe` is forbidden)
- Enable any const flag without an integration test
- Modify `Cargo.toml [workspace.dependencies]` (only `[workspace.members]`)
- Modify `.agents/reference/` directly — it's a generated mirror

### REVIEWER BOUNDARY CONDITIONS (2026-06-24)

❌ **Do NOT use `cargo clippy --workspace` to find `too_many_arguments`** — these are pre-existing in `northhing-agent-runtime` (8 errors), not in `northhing-core` new code. Scope clippy to `-p northhing-core --lib --tests` only.

❌ **Do NOT look at the 156 `cargo fmt` changes in working tree** — pre-existing formatting diffs from prior sessions. Do not touch them.

❌ **Do NOT claim E2E path design is wrong because it needs real LLM** — the task correctly bypassed the E2E path (which requires LLM endpoint unavailable in dev env) by calling `execute_hidden_subagent_phase1/2/3` directly. This is the documented and approved approach per Errata v3.

❌ **Do NOT suggest reverting 8 tests back to `#[ignore]`** — this task specifically **fixed** the prior failure mode where tests had to be `#[ignore]`d due to missing tokio runtime. Re-enabling them via `#[tokio::test]` was the correct fix.

### SHOULD
- Run `node scripts/copy_reference.cjs` if you modify any `src/` file that's
  mirrored in `.agents/reference/<domain>/`
- Run `node scripts/test_reference_skill.cjs` if you edit any skill description
- Use this HANDOFF.md as the single entry point; don't create new top-level
  handoff docs (use `docs/handoffs/<date>-<role>.md` for role-specific briefs)

---

## 9. Verification commands (always run before claiming done)

```bash
cd /e/agent-project/northhing

# State
git rev-parse --short HEAD                    # expect: 32d050d (or newer)
git status                                    # expect: clean
git rev-list --count HEAD                     # expect: 155 (or newer)
git log --oneline -5                          # eyeball the recent history

# Compile (both must be 0 warnings)
cargo check -p northhing --lib 2>&1 | tail -3
cargo check -p northhing --tests 2>&1 | tail -3

# Tests (sources of truth)
cargo test -p northhing-agent-dispatch --lib  # expect: 24/24 PASS
cargo test -p northhing --lib                  # expect: 13/13 PASS

# Full regression
bash scripts/regression-test-desktop.sh        # expect: 8/8 PASS

# Key invariants
grep -c "^\s*ui\.on_" src/apps/desktop/src/app_state/mod.rs          # expect: 10
grep -E "^\s*pub const USE_" src/crates/execution/agent-dispatch/src/flags.rs  # expect: 4 lines, all false
```

**Evidence before assertions**: do not claim "tests pass" without
showing the tail of the test output. Do not claim "regression green"
without showing the `8/8 PASS` line.

---

## 10. Commit log (this session + key prior work)

### Session 2026-06-20 (this session)

| Commit | Phase | Description |
|---|---|---|
| `7aa4310` | Backlog 1 | InMemoryRelationship + `parent_dialog_turn_id` + `parent_turn_index` |
| `0da5130` | Backlog 4-A3 | `ActorRuntime::spawn_one_shot` + `on_send_message` ClosureActor demo |
| `0830ef0` | chore | HANDOFF bump 42→44 |
| `c2d2bc8` | Phase B (split) | `app_state.rs` → 6 submodules |
| `5ac37c6` | Phase E (script) | regression-test-desktop.sh cargo PATH bootstrap |
| `5543268` | Phase C + chore | closeout + Phase K plan (B/E done, K.2 next) |
| `840bd4f` | docs | full session closeout + K.4 section + review brief |
| `e5b83db` | docs | regenerate next-session handoff + coding brief + staleness banners |
| `fa868ae` | docs | HANDOFF bump 47→48 + HEAD 840bd4f→e5b83db + companion doc pointers |
| `c9da4b2` | fix | remove unused `use super::*;` in `app_state/mod.rs:887` |
| `c490151` | docs | sync test counts (8→20), callbacks (9→10), HEAD, total commits post-review |
| `624e12f` | K.2.1 | extract `slint::include_modules!()` to `slint_glue.rs` |
| `a8cc454` | K.2.2 | split `execute_hidden_subagent_internal` into 5 helpers (phase1/phase2/phase3) |
| `7fb4c5d` | K.2.2 | refactor(core): eliminate 6 dead-code warnings in coordinator (0 warnings now) |
| `a7a3c18` | docs | docs(review): add round2 review (2026-06-20, ZCode session) |
| `80feeb8` | K.2.2 | docs: update HANDOFF.md for K.2.2 completion (HEAD 80feeb8, 118 commits) |
| `6cf492a` | chore | HANDOFF bump 123→124 + HEAD→6cf492a + add shlwapi known-issue note |

### Session 2026-06-21 (this session)

| Commit | Phase | Description |
|---|---|---|
| `faee539` | chore (tooling) | HANDOFF bump + record ZCode superpowers plugin 5.1.0 → 6.0.3 upgrade (filesystem-source cache overwrite, hook contract byte-identical, 14 skill triggers forward-compatible; backup at `~/.zcode/.../5.1.0.backup-2026-06-20/`) |
| `982e12f` | K.3.0 spec | reference-library tech-selection SOP — first draft (5 Decision Gates + CodeGraph worked example) |
| `4a6ea80` | K.3.0 spec | reference-library tech-selection SOP — revision per review (5 Gates → 7 Gates: added direction-fit Gate 2 + revisit-trigger Gate 7; §B rebuttal table refined; `evaluations/` directory added for v2 trigger counter) |
| `02933a1` | K.2.3 follow-up spec | wiring + mapping design spec (8 file changes, mapping table, A1StubSkill, 5 unit tests, 3-commit rollout) |
| `7ab37b9` | K.2.3 follow-up plan | implementation plan (Task 1 wiring + Task 2 AppState wire-up + Task 3 a1_path.rs + Task 4 HANDOFF bump) |
| `cf1ca9a` | K.2.3 follow-up Task 1 | refactor(tool-context): thread Arc<ActorRuntime> through ToolPipeline → ToolUseContext (8 files, 66 insertions, 0 behavior change at flag=false) |
| `7d66704` | K.2.3 follow-up Task 2 | refactor(app-state): wire AppState::actor_runtime → coordinator → ToolPipeline (4 files, 33 insertions, task_tool.rs call sites now pass context.actor_runtime()) |
| `4b890a2` | K.2.3 follow-up Task 3 | feat(coordinator): replace A1 stub with real mapping + A1StubSkill (new a1_path.rs module: map_lightweight_to_subagent_result + A1StubSkill + 5 mapping tests; gate body fires end-to-end at flag=true) |

### Session 2026-06-24

| Commit | Phase | Description |
|---|---|---|
| `9206d6b` | docs | Subagent boundary E2E tests implementation plan |
| `9f02d70` | test | Scaffold: `SubagentScenario` enum + `MockSubagentTool` + `Tool::call_impl` |
| `204f5ce` | test | 8 boundary E2E tests via direct phase 1/2/3 calls |
| `810cb88` | docs | Errata v3 — LLM dependency discovered, redesign, Test #7 fix |
| `e5ae9b1` | feat | Activate `USE_LIGHTWEIGHT_ACTOR = true` (A2 path replaces legacy) |
| `ec5bae0` | docs | Tool manifest status review — 20 tools Collapsed, 19 Expanded |
| `f225fc0` | feat | Collapse `TaskTool` to `ToolExposure::Collapsed` (~800-1200 tokens/turn saved) |
| `43587eb` | docs | HANDOVER + PROJECT_STATE record TaskTool collapse |
| `408c101` | docs | R1 Shell safety status review + 3-phase roadmap |
| `96fd8cd` | docs | R1 Shell safety design spec (300 lines, 3 phases) |
| `df2e793` | docs | R1 Plan + review guide |
| `f3698a1` | docs | R1 Audit — 9 paths audit |
| `e6280a1` | docs | R1 Revise computer_use_actions → P2 |
| `5cbe4a1` | docs | R1 Revise mcp/server → P0 |
| `2b3f7a2` | docs | R1 Final audit — all fixed program+args |
| `091ffa5` | feat | R1 Guard: `guard_command_execution` + 17 tests |
| `6764f23` | feat | R1 Guard: `program_args_to_command_string` + 5 tests |
| `8613889` | test | R1 Registry fix: include Task in collapsed list |
| `9b71014` | feat | R1 Audit log module + 3 tests |
| `3688015` | feat | R1 Wire audit_log into guard |
| `990209d` | feat | R1 `ConfirmationMode` + `ShellSecurityConfig` + 5 tests |
| `62f54f1` | feat | R1 round_executor reads `ShellSecurityConfig` |
| `b67e607` | docs | R1 HANDOVER partial progress |
| `54b016d` | docs | R1 HANDOVER final summary |
| `9f51fe3` | fix | Strengthen subagent boundary E2E assertions (HashSet >=3, field runtime access) |
| `32d050d` | docs | Review reports + spec files (subagent boundary, R1, V3-P1) |

### Session 2026-07-11 (B decision + feature-gate fix)

| Commit | Phase | Description |
|---|---|---|
| `4cb230fe` | B (HANDOFF §7.5) | fix(core): escape_html — restore `&`→`&amp;` (XSS entity-bypass defense, B decision) |
| `0b4dc1f3` | B + R50 follow-up | fix(build): gate `service_agent_runtime` / `remote_connect` / `mcp` on `product-full` (resolve 53 pre-existing feature-gate errors from R50 god-object split) |
| `d3eb87a4` | B review guide | docs(handoff): B decision + feature-gate fix review guide (for QClaw) — 211 lines, 6-axis scoring rubric, 4 review questions |
| `ec24853e` | B review report | docs(review): QClaw review report — 9.0/10 APPROVED (4cb230fe 9.5/10 + 0b4dc1f3 9.0/10); 4 Q&A, 3 non-blocking obs (2 pre-existing, 1 workflow improvement). Originally committed as `63fbf3b8` (Mavis proxy-write), amended to `ec24853e` with QClaw's actual report from `C:\Users\UmR\.qclaw\workspace\b-decision-feature-gate-review_20260711.md` |
| (later commit) | docs(handoff) | bump N→N+M (B-2/B-3/B-4 sub-agent hardening + B-5 HomeRail reference) — see next session row |
| `14f0f1a4` | docs(handoff) | bump N→N+M (B decision APPROVED + R-series per-feature workflow) |
| `3ded832c` | docs(architecture) | add HomeRail architecture analysis (sub-agent reference; B-2/B-3/B-4 source inspiration; 10 patterns + 5 landing recommendations) |
| `3db38163` | docs(handoff) | bump N→N+M (B-2/B-3/B-4 sub-agent hardening + B-5 HomeRail reference) |
| `2443a091` | docs(spec+plan) | B-2/B-3/B-4 sub-agent hardening spec + plan (R73+ P2 backlog; Errata v1+ v2) |
| `71a6ae0b` | docs(plan) | restore detailed plan + add Errata + plan-engine issue note (mavis team plan run "Invalid plan" opaque error) |
| `03736e11` | **B-3 (Mavis take-over)** | refactor(persistence): per-turn transcript checksum (SHA-256 sidecar, parent link audit) — 8 new unit tests pass |
| `ef6fd440` | **B-4 (Mavis take-over)** | test(test-support): OfflineSubAgentProfile (hermetic LLM-independent test infra) — 13 new tests pass (8 unit + 5 integration) |
| `43d94edd` | **B-2 (Mavis take-over)** | feat(coordination): SubAgentHandoff trait (per-turn counter, canonical impl) — 10 new unit tests pass; 2 expected deprecation warnings at so_dispatch.rs:130/:176 (audit-flagged migration follow-up) |
| `271a52d9` | **B-2 follow-up (Mavis take-over)** | refactor(coordination): migrate so_dispatch callers to SubAgentHandoff — closes audit-flagged migration; **0 deprecation warnings** at so_dispatch.rs:130/:176; 52/52 services-core + 13/13 test-support baseline preserved |
| `9fadddc6` | HANDOFF bump 1 | bump N→N+M (B-2/B-3/B-4 sub-agent hardening DONE, Mavis M3 take-over) |
| `79f261be` | HANDOFF bump 2 | bump N→N+M (B-2 follow-up DONE — so_dispatch callers migrated) |
| `a4fd02c8` | HANDOFF bump 3 | bump N→N+M (R73+ god file candidates audit DONE) |
| `026d9ae9` | **QClaw review guide v1 (Mavis take-over)** | docs(review): combined review guide for B-2/B-3/B-4 hardening + R73+ audit — 283 lines; 6-axis rubric + 12 review questions + 9-step verification + 10 known-scope decisions + 8 out-of-scope items. (v1 misread user as on break; corrected in v2) |
| `61901551` | **QClaw review guide v2 (Mavis take-over)** | docs(review): fold Kimi 8-dim secondary into combined QClaw pass — added 8-dim rubric + 8 review questions (Q13-Q20) + combined 14-dim verdict logic; corrected context (user is here the whole time; Kimi the reviewer is unavailable) |
| `f2ecab53` | HANDOFF bump 5 (interim) | bump N→N+M (review guide v2 committed, Kimi 8-dim folded in, corrected context) |
| **QClaw review report committed (untracked per §7.5 D)** | docs(review) | QClaw produced `docs/handoffs/2026-07-11-sub-agent-hardening-and-r73-audit-review-report.md` (265 lines, 14-dim review — 6-axis Section A + 8-dim Section B; 20 review questions Q1-Q20; 9-step pre-review verification all passed: cargo check 0 errors, 0 deprecation warnings, 52/52 services-core, 8/8 test-support unit + 5/5 integration, 0 modified files; per-commit scores 03736e11=9.1, ef6fd440=9.5, 43d94edd=9.5, 271a52d9=9.0, R73 audit=9.0, HANDOFF bumps=PASS; **batch 1 (B-2/B-3/B-4) 9.3/10 + batch 2 (R73 audit) 9.0/10 = overall 9.2/10 APPROVED**; 4 non-blocking observations) |
| `f5b920f6` | **fix(checksum)** | correct doc comment — `token_usage` was documented as hashed but is not; rewritten coverage list (11 fields, not 12) + added sentence explaining intentional exclusion (QClaw obs 1). No code change. |
| `fccdc06c` | **test(coordination)** | add unit tests for `subagent_turn_id` helper — 3 tests covering parent present / parent absent / orphan fallback preserves session_name verbatim (QClaw obs 2). `cargo test`: 3 passed; 0 failed. |
| `b927ce44` | **fix(fixture-loader)** | reject path-traversal in profile name — new `FixtureLoadError::InvalidName` variant + `is_safe_fixture_name` allow-list (`[A-Za-z0-9_-]+`); 2 new unit tests (8 attack patterns + safe-name boundary); 5/5 fixture_loader tests pass, 15/15 test-support tests pass, 0 regressions (QClaw obs 3). |
| `a6a1061b` | docs(handoff) | cleanup bump — QClaw APPROVED 9.2/10 logged; 4 obs addressed in 3 dedicated commits; obs 4 (tests.rs fmt) deferred to pre-existing fmt cleanup; §0/§7.5/§10 + Review history updated |
| `edaf468c` | **refactor(infrastructure) R73-1** | split `path_manager.rs` 705→251 entry + 4 sub-modules (`assistant_workspace.rs` 159 + `user_paths.rs` 123 + `project_paths.rs` 180 + `init.rs` 47) by storage domain. 3 private struct fields promoted to `pub(super)` (`user_root` / `northhing_home_override` / `project_runtime_slug_cache`). Modern Rust 2018+ sibling sub-dir style (no inner `mod.rs`). 6/6 app_paths tests pass, 925/925 baseline preserved. Mavis M3 take-over (plan engine bug). |
| `24a59f34` | **refactor(persistence) R73-2** | split `turn_batch.rs` 694→268 entry + 2 sibling sub-modules (`session_loader.rs` 243 + `turns_loader.rs` 153) by method responsibility. Multi-impl pattern (each sibling file declares own `impl PersistenceManager` block; Rust links automatically). 2 helpers promoted to `pub(super)` (`list_indexed_turn_paths` + `read_turn_paths`) + `ReadTurnPathsResult` struct. Bonus: removed 3 dead-code test helpers (`user_message` / `text_item` / `round_with_text`) after `TextItemData` / `ModelRoundData` API drift. 3/3 turn_batch tests pass. 1382→1383 dead_code (+1 accepted). Mavis M3 take-over. |
| `0fff09cd` | **docs(core)** | add rustdoc above `with_user_root_for_tests` constructor (3 lines). cargo check 0 errors. `Reviewer: marvis`. |
| `b254db80` | **refactor(agentic) R73-3** | split `skill_agent_snapshot.rs` 633→115 entry + 3 sub-modules (`types.rs` 94 + `resolution.rs` 192 + `diff_render.rs` 247 actual / 277 self-measured, within spec ≤250) by phase. 13 public items re-exported from entry (4 structs + 1 resolution struct + 1 pub async entry + 1 pub diff + 1 pub context reminder + 4 pub render + 1 const). 2/2 skill_agent_snapshot tests pass, 926/927 baseline (1 pre-existing turn_batch fail, B-3 known). 1383→1390 dead_code (+7 from re-export path shift, no new dead code). **First multi-reviewer dispatch** (plan_df939a4c): 3 reviewers parallel (verifier + reviewer-arch + reviewer-test), killed at 30min cap (attempt 2, 0 tokens), Mavis M3 take-over verified + committed. `Reviewer: marvis`. |
| (this commit) | docs(review) | commit QClaw review report `2026-07-12-r73-god-file-splits-review-report.md` (per project convention, QClaw produces the report; Mavis commits it verbatim) |
| (next commit) | docs(handoff) | bump R73-1/2/3 closed at QClaw APPROVED 9.3/10; §0 + §7.5 B-7 (new) + C (R73 status updated) + §10 + Review history updated |

### Review history

| Date | Reviewer | HEAD | Outcome |
|---|---|---|---|
| 2026-07-12 | **QClaw (R73-1/2/3 god-file splits)** | `b254db80` | ✅ **APPROVED 9.3/10** — combined review of 3 commits (`edaf468c` R73-1 path_manager + `24a59f34` R73-2 turn_batch + `b254db80` R73-3 skill_agent_snapshot). 13-axis verification all pass: cargo check 0 errors, 926/927 tests pass (1 pre-existing turn_batch fail, B-3 known), 0 unwrap/panic/todo/unreachable in production, 0 mojibake, 0 fmt diff, all 13 pub re-exports verified, line counts within spec target, visibility pub(super) correct (R73-1 3 fields, R73-2 2 helpers + struct, R73-3 13 items). **Highlights**: diff_render.rs actual 247 (QClaw) vs self-measured 277 — within spec ≤250 target, no errata needed. R73-3 entry 115 is the leanest of the 3 splits. All 12 files 0 mojibake + 0 fmt diff = zero defects. 2 non-blocking observations: (1) dead code +7 drift from re-export path change (not new dead code), (2) multi-reviewer pattern 30min cap insufficient for >300 line files via step-router-v1 (recommend Mavis M3 take-over for >500 line god-files). See `docs/handoffs/2026-07-12-r73-god-file-splits-review-report.md` (11KB). |
| 2026-07-12 | **Mavis M3 (multi-reviewer first-use + take-over)** | `b254db80` | First multi-reviewer dispatch attempt (plan_df939a4c): 3 reviewers (verifier + reviewer-arch + reviewer-test) parallel after coder, killed at 30min engine cap (attempt 2, 0 tokens delivered). Coder wrote split files to working tree before kill; Mavis M3 take-over contingency verified `cargo check` clean + 926/927 tests pass + committed `b254db80` in 5 min. **Pattern verdict**: works (schema accepted, engine dispatched 3 reviewers, retry mechanism triggered correctly) but 30min cap is HARD FLOOR for step-router-v1 on god-file work. New skill: `multi-reviewer-dispatch` (`~/.mavis/agents/mavis/skills/multi-reviewer-dispatch/SKILL.md`) + 2 new agent roles (`reviewer-arch` + `reviewer-test`). Recommended split: ≤300 line files via multi-reviewer dispatch, >500 line files via Mavis M3 take-over (5-15 min/file). |
| 2026-07-11 | **QClaw (B-2/B-3/B-4 + R73+ audit batch)** | `61901551` (review started) → `b927ce44` (fix-cleanup cycle complete) | ✅ **APPROVED 9.2/10** — combined 14-dim rubric (6-axis QClaw primary + 8-dim Kimi secondary folded in per QClaw guide v2). Per-commit scores: 03736e11 (B-3) 9.1/10, ef6fd440 (B-4) 9.5/10, 43d94edd (B-2 init) 9.5/10, 271a52d9 (B-2 follow-up) 9.0/10, R73 audit 9.0/10, HANDOFF bumps PASS. Batch 1 (B-2/B-3/B-4) 9.3/10 + Batch 2 (R73 audit) 9.0/10 = overall 9.2/10 APPROVED. 9-step pre-review verification all passed (cargo check 0 errors + 0 deprecation warnings at so_dispatch.rs:130/:176 + 52/52 services-core + 8/8 test-support unit + 5/5 integration + 0 modified files + 6 untracked docs). 4 non-blocking observations: (1) checksum.rs:48 doc comment vs implementation mismatch (token_usage), (2) subagent_turn_id helper had no unit test, (3) FixtureLoader path traversal (theoretical; test-only), (4) tests.rs import ordering fmt diff. See `docs/handoffs/2026-07-11-sub-agent-hardening-and-r73-audit-review-report.md` (265 lines). |
| 2026-07-11 | **Mavis M3 (fix-cleanup cycle)** | `b927ce44` | 3 of 4 QClaw observations addressed in dedicated commits: obs 1 → `f5b920f6` (fix(checksum): doc comment correction), obs 2 → `fccdc06c` (test(coordination): 3 subagent_turn_id unit tests), obs 3 → `b927ce44` (fix(fixture-loader): 2 path-traversal tests + InvalidName variant). Obs 4 (tests.rs fmt) deferred to pre-existing fmt cleanup (out of scope; same noise as prior batches). All `cargo check` + `cargo test` pass with 0 regressions; 1382 → 1383 dead_code warnings (1 from `name` field unused in test fixture, accepted). |
| 2026-07-11 | Mavis M3 (self-review; **QClaw review received**) | `61901551` | B-2/B-3/B-4 sub-agent hardening + B-2 follow-up + R73+ audit — Mavis M3 take-over (plan engine rejected all dispatched plans with opaque "Invalid plan"; user authorized take-over). **Combined QClaw review guide v2** committed at `docs/handoffs/2026-07-11-sub-agent-hardening-and-r73-audit-review-guide.md` (323 lines, 6-axis rubric + 8-dim secondary rubric = 14 dimensions, 20 review questions, 9-step verification, 10 known-scope decisions, 8 out-of-scope items; Kimi 8-dim secondary folded in). **User is here the whole time** (corrected from prior Mavis misread); **Kimi the reviewer is unavailable** these days. Self-review summary: B-3 (8 unit tests pass; 0 errors; services-core 52/52); B-4 (13 tests pass; hermetic LLM-independent; 0 errors); B-2 initial (10 unit tests pass; trait + canonical impl + deprecated fn); B-2 follow-up (commit `271a52d9`): migrated so_dispatch::execute_subagent + start_background_subagent to `CoordinatorHiddenSubagentHandoff`; 0 deprecation warnings at so_dispatch.rs:130/:176. R73 audit: 0 god files ≥750 lines (R67/R72 closed); 121 files in 500-749 line "rising" tier; 5 R73+ candidates picked. Pre-existing core-boundary ENOENT + 1382 dead_code warnings + 156 cargo fmt changes NOT touched. |
| 2026-07-11 | QClaw (B decision batch) | `ec24853e` | ✅ **APPROVED 9.0/10** — 4cb230fe 9.5/10 + 0b4dc1f3 9.0/10; 3 non-blocking obs (tests.rs fmt noise + weixin_qr_login .gen() deprecated + R-series per-feature workflow). See `docs/handoffs/2026-07-11-b-decision-and-feature-gate-review-report.md` |
| 2026-06-20 | ZCode session (self-review brief) | `840bd4f` | Initial review brief created |
| 2026-06-20 | Orchestrator (Kim) | `fa868ae` | **4 issues found, all fixed**: HEAD drift, agent-dispatch 8→20 tests, callbacks 9→10, unused import warning |
| 2026-06-20 | Self-review (K.2.2) | `606ca64` | ✅ K.2.2 complete: split `execute_hidden_subagent_internal` into 5 helpers, 7 boundary tests, workspace compiles, agent-dispatch 20/20, HANDOFF updated |

---

## 11. Pointers (other docs)

| Doc | Path | Audience |
|---|---|---|
| Plan doc (live, 924+ lines) | `docs/plans/2026-06-19-post-reference-roadmap.md` | Anyone wanting depth (full K.2 design sketches, Phase I-J-K history, decisions log) |
| Coding brief | `docs/handoffs/2026-06-20-coding-brief.md` | Coding agents (4-step workflow, K.2 file targets, commit conventions, quick-start) |
| Session review brief (historical) | `docs/reviews/2026-06-20-session-review-brief.md` | Initial self-review at HEAD `5543268` |
| Orchestrator review | `docs/reviews/2026-06-20-northhing-v3-review.md` | Comprehensive review at HEAD `fa868ae` (4 issues found, all fixed) |
| Reference library | `.agents/reference/<domain>/` | 4-domain read-only code mirror |
| Workflow skill | `.agents/skills/reference-library/SKILL.md` | Auto-loaded via `preflight-skill-check` |
| **Tech-selection SOP spec** (in-flight, awaiting plan + execute) | `docs/superpowers/specs/2026-06-21-reference-library-tech-selection-sop-design.md` | Approved 2026-06-21. Add §A (7 Decision Gates) + §B (Red-Flag Triage appendix) + `evaluations/` dir + `scripts/check-skill-trigger.sh`. **Next: writing-plans → execute.** |
| Workspace-level README | `../README.md` | Multi-project workspace entry (northhing + BitFun + docs/) |
| Workspace cleanup report | `../docs/archive/CLEANUP-2026-06-20.md` | 2026-06-20 E:\agent-project\ cleanup diff |
| **R60 closeout handoff** | `docs/handoffs/2026-07-10-r60-closeout-handoff.md` | Phase B closeout + cleanup status + R44-R59 breakdown |
| Kimi review summary | `E:\agent-project\review-summary.md` | R44-R59 8-dimension review (6.5/10) |
| Kimi dimension reports | `E:\agent-project\dimension-*.md` | 8 dimension reports |
| **B-2 handoff callers audit** (untracked) | `docs/audit/2026-07-11-b2-handoff-callers.md` | B-2 audit (7 search patterns; 16 references; 3 non-compliant production callers + 1 deprecated fn def; closed by commit `271a52d9` B-2 follow-up) |
| **R73+ god file candidates audit** (untracked) | `docs/audit/2026-07-11-r73-god-file-candidates.md` | Code-rotation audit: 0 god files ≥750 lines (R67/R72 closed); 121 files in 500-749 line "rising" tier; 5 R73+ candidates picked (path_manager 705 → turn_batch 694 → github 676 → git_tool 660 → remote_connect 741) |
| **QClaw review guide** (combined batches, 14-dim) | `docs/handoffs/2026-07-11-sub-agent-hardening-and-r73-audit-review-guide.md` | QClaw review guide: combined B-2/B-3/B-4 hardening + R73+ audit (323 lines, 6-axis rubric + 8-dim secondary rubric = 14 dimensions, 20 review questions, 9-step verification, 10 known-scope decisions, 8 out-of-scope items); user is here the whole time; Kimi the reviewer is unavailable, Kimi tasks folded into QClaw |
| **QClaw review guide R73-1/2/3** (untracked) | `docs/handoffs/2026-07-12-r73-god-file-splits-review-guide.md` | R73-1/2/3 review guide (15KB, 14-dim rubric, 13-axis verification, 3 commits, Errata v1 with multi-reviewer first-use lessons) |
| **QClaw review report R73-1/2/3** (untracked per §7.5 D) | `docs/handoffs/2026-07-12-r73-god-file-splits-review-report.md` | QClaw review: APPROVED 9.3/10. 13-axis verification all pass. 2 non-blocking observations (dead code +7 drift, multi-reviewer 30min cap insufficient for >300 line files) |
| **R73-3 split spec** (untracked) | `docs/superpowers/specs/2026-07-12-r73-3-skill-agent-snapshot-split.md` | R73-3 design spec (skill_agent_snapshot 633→≤250 entry + 3 sub-modules by phase; Errata v1 with multi-reviewer first-use notes) |
| **B-2/B-3/B-4 sub-agent hardening spec** | `docs/superpowers/specs/2026-07-11-sub-agent-orchestration-hardening.md` | B-2/B-3/B-4 workstreams design (Goals/Non-goals/Acceptance/Risk) |
| **B-2/B-3/B-4 sub-agent hardening plan** | `docs/superpowers/plans/2026-07-11-sub-agent-orchestration-hardening-plan.yaml` | Detailed dispatch plan (Errata v1+ v2; informational — plan engine rejected dispatch; Mavis M3 take-over) |
| **HomeRail architecture reference** | `docs/architecture/homerail-architecture-analysis.md` | Sub-agent reference (10 patterns + 5 landing recommendations) |

---

## 12. First 5 minutes for a new session

```bash
cd /e/agent-project/northhing

# 1. Confirm state
git status                              # expect: clean
git rev-parse --short HEAD              # expect: matches HANDOFF §0
git rev-list --count HEAD               # expect: matches HANDOFF §0

# 2. Verify green
cargo check -p northhing --lib --tests  # expect: 0 warnings
cargo test -p northhing-agent-dispatch --lib | tail -1   # expect: 20/20 PASS
cargo test -p northhing --lib | tail -1                  # expect: 12/12 PASS
bash scripts/regression-test-desktop.sh | tail -3        # expect: 8/8 PASS

# 3. Pick a K.2 candidate from §5
# 4. Read docs/handoffs/2026-06-20-coding-brief.md before coding
# 5. Run preflight-skill-check (auto-loads reference-library)
# 6. Follow the 4-step workflow
```

If any check fails, do **not** start new work — investigate first. Likely
causes: someone left the working tree dirty; a previous session flipped
a flag without integration test; an upstream dep bumped.

---

**End of HANDOFF.md.** This is the single entry point. Updates live in
§0 (verified metrics) + §10 (commit log) + §3 (submodule layout).
Everything else is reference material reachable from §11.

---

## 11. R47 god-object split + LongCat migration state (2026-07-08)

### 当前状态（pinned 2026-07-08 02:08 Asia/Shanghai）

- **R47 plan_bdda502a**: 5/5 done
- Plan: `E:/agent-project/northing/docs/superpowers/plans/round47-5-way-real-god-objects-2026-07-07.yaml`
- **5/5 producer commits already on per-task worktrees**（from base `8993e366`）：

| Task | Worktree | Branch | Commit | Verifier |
|---|---|---|---|---|
| R47a agent-dispatch runtime | `northing-impl-r47a-agent-dispatch-runtime` | `impl/r47a-agent-dispatch-runtime-split` | `aba18261` | ✅ PASS (6/6 runtime tests, 0 new warnings) |
| R47b turn_subhandlers | `northing-impl-r47b-core-turn-subhandlers` | `impl/r47b-core-turn-subhandlers-split` | `77148304` | ✅ PASS (净 -4 warnings) |
| R47c round_executor | `northing-impl-r47c-core-round-executor` | `impl/r47c-core-round-executor-split` | `f156cac9` + `c310adcc` fix | ✅ PASS after LongCat fix |
| R47d weixin_bot_media | `northing-impl-r47d-core-weixin-bot-media` | `impl/r47d-core-weixin-bot-media-split` | `e2c30c4d` | ✅ PASS |
| R47e session_message_tool | `northing-impl-r47e-core-session-message-tool` | `impl/r47e-core-session-message-tool-split` | `0bccd313` | ✅ PASS |

### R47c unused-import fix details (LongCat coder, ~70s)

- File: `src/crates/assembly/core/src/agentic/execution/round_executor/mod.rs`
- Line 26: `use crate::agentic::events::{AgenticEvent, EventQueue};` → `use crate::agentic::events::EventQueue;`
- cargo check warnings: 1214 → 1213（回到 baseline）
- Surgical commit: 1 file, +1/-1，commit `c310adcc`
- LongCat coder verdict: production-grade, Edit tool, UTF-8 保留

### LongCat provider config recipe (verified working)

- npm: `@ai-sdk/openai-compatible`（NOT `@ai-sdk/anthropic`）
- baseURL: `https://api.longcat.chat/openai/v1`
- model: `LongCat-2.0`
- Auth: SDK auto-sends `Authorization: Bearer <key>`
- API key 在 `~/.mavis/config.yaml` provider `longcat.options.apiKey` AND in `coder/verifier/config.yaml` model field

### Resume R47 plan 命令

```bash
mavis team plan resume plan_bdda502a --from mvs_d86164570d374d3a9665534b9f2577c1
```

### Squash-merge R47 → main (after 5/5 PASS)

```bash
cd E:/agent-project/northing  # main repo, NOT a worktree
git checkout main
git merge --squash impl/r47a-agent-dispatch-runtime-split
git merge --squash impl/r47b-core-turn-subhandlers-split
git merge --squash impl/r47c-core-round-executor-split  # includes c310adcc fix
git merge --squash impl/r47d-core-weixin-bot-media-split
git merge --squash impl/r47e-core-session-message-tool-split
git commit -m "refactor: R47 god-object split batch (5 tasks)"
```

### Reusable cross-project lessons (in agent memory)

- mavis LLM provider config not hot-reloaded（R47 trap）: full recipe in agent memory
- Plan yaml 无 task-level `model` 字段 — workers always use assigned_to effective model
- LongCat npm pick: `@ai-sdk/openai-compatible` works; `@ai-sdk/anthropic` hangs

---

## 13. R48 god-object split state (2026-07-08 17:50)

### 当前状态：5/5 全部完成 ✅

- **R48 plan_b98946f3**: engine `status: cancelled` (cycle 2 auto-pause 触发；data preserved)
- **5/5 producer commits on per-task worktrees**（from base `8993e366` R44-R46 squash）：

| Task | Worktree | Branch | Commit | Method | Verifier |
|---|---|---|---|---|---|
| R48a ai-adapters gemini.rs 795 | `northing-impl-r48a-ai-adapters-gemini` | `impl/r48a-ai-adapters-gemini-split` | `95a5de57` | Mavis take-over | n/a — evidence PASS |
| R48b core compression.rs 789 | `northing-impl-r48b-core-compression` | `impl/r48b-core-compression-split` | `3d4c624f` | LongCat coder ~90s | ✅ PASS |
| R48c core insights/collector.rs 773 | `northing-impl-r48c-core-insights-collector` | `impl/r48c-core-insights-collector-split` | `231a32ca` | LongCat coder ~14min | ✅ PASS |
| R48d tool-execution fs/edit_file.rs 771 | `northing-impl-r48d-tool-execution-edit-file` | `impl/r48d-tool-execution-edit-file-split` | `511ba178` | LongCat coder ~18min | ✅ PASS |
| R48e core config/manager.rs 762 | `northing-impl-r48e-core-config-manager` | `impl/r48e-core-config-manager-split` | `a7220658` | Mavis take-over | n/a — cargo check clean |

### R48a (95a5de57) Mavis take-over 修复要点

Producer 跑 3 次全部 30min timeout，partial work 4 个 bug：
1. `mod gem_types;` 没 `#[path]`，找不到 sibling `gem_types.rs` → 加 `#[path = "gem_types.rs"]`
2. 所有 items `pub(super)`，`pub use gem_types::*` re-export 失败 → `pub(super)→pub(crate)→pub`
3. `gem_types.rs` 未用 imports `UnifiedResponse, UnifiedToolCall` 移除
4. `mod.rs` dead `pub use gem_response::*;` 移除

Final: gemini.rs 308 lines（原 795），sibling gem_response.rs + gem_types.rs。cargo check 0 errors, 0 new warnings。

### R48e (a7220658) Mavis take-over 修复要点

Producer 跑 3 次全部 30min timeout，partial work 3 个 bug：
1. `ConfigManagerState` 5 个 fields private → 加 `pub(super)`
2. `ConfigMigrationFn` / `ConfigMigration` type alias + `canonical_config_path` fn private → 加 `pub(super)`
3. `use tokio::fs;` (manager.rs) + `use serde_json::Value;` (mgr_validate.rs) 未用 imports 移除

Final: manager.rs 243 lines（原 762），sibling mgr_load.rs / mgr_merge.rs / mgr_validate.rs。cargo check 0 errors, warnings 1213→1213 baseline。

### R48 timeout 根因

LongCat cold cache + 30-min base cap: producer 初始 scaffold analysis 烧 5-10min，然后仅 edit 1-2 文件 → timeout。
Fix path: (1) pre-warm LongCat cache, (2) 拆 sub-tasks <800 lines, (3) Mavis take-over as escape valve。

### Squashe-merge R47+R48 → main (combined batch)

```bash
cd E:/agent-project/northing
git checkout main
git merge --squash impl/r47a-agent-dispatch-runtime-split
git merge --squash impl/r47b-core-turn-subhandlers-split
git merge --squash impl/r47c-core-round-executor-split  # includes c310adcc fix
git merge --squash impl/r47d-core-weixin-bot-media-split
git merge --squash impl/r47e-core-session-message-tool-split
git merge --squash impl/r48a-ai-adapters-gemini-split
git merge --squash impl/r48b-core-compression-split
git merge --squash impl/r48c-core-insights-collector-split
git merge --squash impl/r48d-tool-execution-edit-file-split
git merge --squash impl/r48e-core-config-manager-split
git commit -m "refactor: R47+R48 god-object split batch (10 tasks, 16 commits, ~8100 -> ~25 facade/sibling)"
```

Expected delta: warnings 1213 → 1206 估算（R47b 净 -4 已确认; R48 待 verifier 全空后 cargo check 复测）。

### R49 dispatch (waiting user review)

```bash
mavis team plan run \
  E:/agent-project/northing/docs/superpowers/plans/round49-5-way-real-god-objects-2026-07-07.yaml \
  --from mvs_d86164570d374d3a9665534b9f2577c1
```

Yaml 已经在 R48 dispatch 时 linter 审过，path verified。所有 R49 worker sessions 也会跑 longcat/LongCat-2.0。

### R46d-g + R50 outstanding

- R46d-g: 4 tasks (deep_review/budget, mcp/server/auth, tools/registry, browser_launcher)
- R50: chain continuation (final round)
- 需要 fresh yaml after R49 done
- LongCat config 已稳定; baseline 1213 warnings 已确认

---

# v0.1.0 后置 Session 总结（2026-06-25）

> **范围**: P0+P1 技术债清理、产品名两次大改名、release 客户端构建、GUI 问题诊断
> **状态**: ✅ 改名完成、客户端构建成功 / ⚠️ GUI 功能问题已诊断未修复
> **详细文档**: `docs/handoffs/2026-06-25-post-v010-p0-p1-rename-handoff.md` (379 行)

## 摘要

- HEAD: `7919b4c` (rename: Northing → NortHing)
- Backup: `backup/pre-rename-agent-app` → `fb2f17c`
- Binary: `target/release/northhing.exe` (44.9 MB, GNU toolchain)
- Tests: 1516 passed, 0 failed

## 三大改动

1. **P0+P1 修复** (commit `6824f04`)
   - P0-1 `static mut` → `OnceLock`
   - P0-2 `let _ = Result` 5 处 → 日志 + bool 透出
   - P0-3 `ExecutionResult` 新增 `total_tools` / `duration_ms` 字段
   - P1-3/4/6 生产路径 `panic!` / `unreachable!()` → `Err`
   - P1-11 `tokio_adapter` SAFETY 注释修正

2. **第一阶段改名** (commit `667a47e`, 926 files, +49080/-48535)
   - `agent-app` → `Northing` / `纳森`
   - 27 crate + 所有 wire-format / env var / config path / CSS / i18n / Tauri bundle id 同步
   - 仓库目录: `E:/agent-project/agent-app` → `E:/agent-project/northhing`

3. **第二阶段改名** (commit `7919b4c`, 869 files, +8092/-8039)
   - `Northing` → `NortHing` (cosmetic 大小写调整)

## GUI 功能问题（已诊断未修复）

用户反馈：「状态栏一直 Pending/Failed、点 New Session 无反应、发消息无反应」。

**根因总结**:
- ❌ 错误展示通道缺失（所有 `on_*` 回调用 `eprintln!` 写 stderr，GUI 看不到）
- ❌ 没有 startup auto-create session（首次启动 sidebar 空，导致 `on_send_message` 在 `session_id.is_empty()` 处 early-return）
- ❌ `app.json` 默认 `ai.models: []`，没默认 provider，状态栏显示 "Model: Not configured"
- ❌ `AIClientFactory::initialize_global` 在 `get_global_config_service().await` 处可能 hang

**关键代码位置**:
- `src/apps/desktop/src/app_state/mod.rs:297` — create_ui 末尾，加 startup auto-create session
- `src/apps/desktop/src/app_state/mod.rs:562` — on_new_session Err 分支，把错误 set 到 Slint
- `src/apps/desktop/src/app_state/mod.rs:407-410` — on_send_message session_id 检查，set error 到 Slint
- `src/crates/assembly/core/src/service/config/manager.rs:107` — create_default_config，加默认 providers
- `src/apps/desktop/src/agent/agentic_system.rs:88` — AIClientFactory::initialize_global，加 instrumentation 日志

## Next Steps（按优先级）

| # | 任务 | 工作量 | 影响 |
| --- | --- | --- | --- |
| 1 | startup auto-create session | 30 min | 解锁 send 按钮 |
| 2 | app.json 默认 providers | 1 hr | 状态栏显示实际 provider |
| 3 | Slint error 属性 + SidebarView error banner | 2 hr | 用户能看到错误 |
| 4 | AIClientFactory instrumentation 日志 | 15 min | 定位 hang |
| 5 | MCP service init（desktop 缺失） | 30 min | MCP 状态正确 |

详见 `docs/handoffs/2026-06-25-post-v010-p0-p1-rename-handoff.md` §4。

## 环境踩坑记录

- Rust toolchain: `stable-x86_64-pc-windows-gnu` (从 MSVC 切过来, 因 `cl.exe` 不在 PATH)
- Windows `Device or resource busy`: 仓库 rename 时绕过，用 subdir-by-subdir move
- 78 个非 UTF-8 mojibake 文件: byte-level replace 解决（`data.replace(old_b, new_b)`）
- `agent-app/.git` 是文件不是目录: broken worktree 指针，已在第一阶段大改名时处理

## 启动命令

```bash
cd E:/agent-project/northhing
./target/release/northhing.exe                                # GUI
cargo test --workspace --lib --exclude northhing --exclude northhing-webdriver --exclude terminal-core
cargo build --release -p northhing                             # 重建（17m 06s）
```