---
description: NortHing 项目 (E:\agent-project\northing) god-object 拆分 fundamentals — 项目基础 (QClaw/Kimi role + 已拆 god-object 历史)、sub-domain split 4 类隐式错误、Cross-crate consumer visibility rule、Cross-crate rename 5-step recipe、R26/R27/R27b/R28/R28b 各种 split pattern、golden pattern、R31 DTO schema verification、R37-R39 batch lessons + KPI、reviewer attribution。Mavis 操作层 (R21+ flow / plan YAML discipline / 验证 / take-over) 详见 `northing-split-execution.md`。Read when learning split pattern 选择 / 准备 cross-crate rename / debugging 跨 sibling visibility / 复盘 batch lessons。
---

# NortHing god-object 拆分 (项目特定)

Updated: 2026-07-14 (P2 cleanup: frontmatter date update; R21+ execution details downshifted to `northing-split-execution.md`; Windows+PowerShell downshifted; Mavis infra downshifted)

## 项目基础

- **仓库**: `E:\agent-project\northing`
- **27-crate Rust workspace** + React 前端
- **QClaw** = 主 reviewer (外部 agent, commit `*-review-report.md` 到 repo)
- **Kimi** = 用户找的次 reviewer (verbal/text 反馈, **不** commit 到 repo)
- **最大 god-object**: `bot/command_router.rs` 2614 行 (Kimi P0, R14 已拆)
- **§7 E1 cap**: 单文件 ≤ 1000 行 (mod.rs ≤ 600 行); **R20 QClaw 实际容差 = 242 canonical wc-l** (用于 client/manager_*.rs 系列)
- **已拆 god-objects**: session_manager (R9), chat (R5), dialog_turn (R6), remote_ssh_manager (R13b), control_hub_tool (R18), acp/client/manager (R19+R20), workspace/service (R23), session_usage/service (R24), runtime-ports/lib (R26), workspace/manager (R27+R27b), terminal/session/manager (R28b+R31)
- **agent-app (旧项目) 156 个 pre-existing cargo fmt 改动**: 不要碰 (in-flight, 与当前 split 无关)

## Round 历史 + commit hash

| Round | 目标 | 关键 commit | 教训 |
|---|---|---|---|
| 3b | session_manager.rs nominal split | `5250199` | mod.rs 没 `pub mod` 声明 sibling → dead code on disk |
| 4 | panic cleanup | `9dbcb9c` | audit agent 用 stale baseline |
| 5 | chat.rs sub-domain split | - | structure-verifier.py 误报 60 methods lost → 必写 custom verifier |
| 6 | dialog_turn.rs sub-domain split | `e31fda3` | M3 model 太慢 + cargo check stop-at-first + Cargo.lock drift |
| 9 | session_manager god-method split | - | `pub(super)` pattern 标准 |
| 11 | review pass | - | spec → impl → review guide → review report → fix → cleanup |
| 12/12b | Mavis take-over | - | take-over 不写 review guide → 事后改规则 |
| 13 | remote_ssh_manager facade | - | facade 2303 → 196 (fwd methods 移除) |
| 13b | remote_ssh_manager 收尾 | `1f19784` | QClaw 9/10 + Kimi 8.5/10 |
| 14 | bot/command_router god-object split | `ed35b81` | 2614 → facade 306 + 8 sub-siblings; 5-pass fix; 899/0/1 tests; QClaw 7.5/10 COND + Kimi 8.6/10 |
| 16 | control_hub_tool god-object split (R18) | `c12bb93`/`b28c645` | Kimi R16 deep review 5-bug = BLOCKING; CRLF+autocrlf fix; Mavis 误标 QClaw → 改 `marvis` |
| 18 | control_hub_tool god-object split | - | facade 244 + 6 sub-siblings; producer self-corrected R17 unwrap-count spec drift |
| 19 | acp/client/manager god-object split | `35790ad`/`230b55a` | 2519 → 12 files; QClaw caught pub(super) 跨 crate regression (11 E0624); Mavis 74-instance bulk fix |
| 20a-c | manager_session split | `8e75f3e`/`df3f878`/`320c696` | 3 + 3 + 4 sub-siblings; method-disjoint pattern 标准 |
| 20d-e | manager_transport / manager_process accept-as-is | `d925c1f`/`5aa63a4` | 276/254 lines; QClaw P2/P3 accept |
| 21-22 | parallel sub-round flow 试运行 | - | dialog_turn/mod.rs 1653 → assembly/core/coordination |
| 23 | workspace/service.rs split | - | 2339 → facade + 5; take-over after subagent timeout |
| 24 | session_usage/service.rs split | - | 2458 → facade + 5; Mavis take-over |
| 26 | runtime-ports/lib.rs (interface crate) | - | 2460 → 4 sibling; 5 sibling decision: agent_dialog + submission_events 合并入 agent (30+ cross-ref) |
| 27 | workspace/manager.rs horizontal split | - | 1505 → 2 sibling; 失败原因: types.rs(300) + manager_impl.rs(1234) 仍超 800 cap |
| 27b | manager_impl.rs sub-domain split | - | 1234 → 3 sibling; fix: 6 fields 加 `pub(super)` |
| 28 | terminal/session/manager horizontal | - | 1457 → 2 sibling; **失败** (pub(super) on Drop + 跨 sibling type visibility + 29-line use block 重复) |
| 28b | terminal/session/manager sub-domain | - | facade 170 + 4 sibling (430/231/215/474 all < 800) |
| 31 | session_*.rs DTO schema verification | - | 必先 `git show HEAD:<file>` 看原 struct, consumer code 反推不可靠 (3 of 4 fields invented) |
| 37-39 | 21 rounds 跨 3 batches | - | 详细见末尾 R37-R39 batch lessons 段 |
| 40 | 6 task (1811-1638): computer_use_host / subagent_orchestrator / ports / insights/service / service_agent_runtime / feishu | ✅ 6/6 accept (plan_92aca4a2) | - |
| 41 | 7 task (1630-1415): bash_tool / tool_pipeline / git_tool / scheduler / tool_context / code_review / workspace_manager | ✅ 7/7 accept (plan_029c030a + b22f83ae) | scheduler facade 1315 严重超 cap (R50 cleanup) |
| 42 | 5 task (1373-1257): insights/html / snapshot_core / cron_tool / review_platform / session_persistence | ✅ 5/5 accept (plan_ebf8330d) | - |
| 43 | 6 task (1255-1168): git/service / browser_control/actions / session_usage/service / remote_exec / workspace_runtime / prompt_builder | ✅ 6/6 done | session_usage/service Mavis take-over ba104020 |
| 44 | 7 task (1219-1029): dialog_turn/mod.rs, miniapp/manager.rs, exec_command/command.rs, grep_tool.rs, lsp/process.rs, skills/registry.rs, workspace/service.rs | R44a bonus done (640aa315) | R44 OLD yaml 全 phantom → cancel; user fresh session dispatch 新 plan_6379222d |
| 45-49 | 5×5-7 task | ⏳ yaml written, await user dispatch | 6 yaml written with all git ls-files verified paths |

**R40-R49 real-paths ground truth** ✅ SUPERSEDED 2026-07-14 (snapshot from 4841f3bd is stale; R73 work continued, see §R73-1/2/3 status; current HEAD = `c933e490`) (从 main 4841f3bd `git ls-files` + wc-l > 700):
- **R44 (7 task, 1219-1029)**: dialog_turn/mod.rs, miniapp/manager.rs, exec_command/command.rs (R44a bonus 已 done), grep_tool.rs, lsp/process.rs, skills/registry.rs, workspace/service.rs
- **R45 (6 task, 972-877)**: round_subhandlers, desktop_ax_actions, tool-execution/grep_search, agent-runtime/deep_review/task_execution, workspace_search/service, agent-runtime/scheduler
- **R46 (7 task, 873-815)**: agent-runtime/prompt_cache, remote_ssh/manager_session_lifecycle, snapshot/manager, agent-runtime/deep_review/budget, mcp/server/manager/auth, tools/registry, browser_launcher
- **R47 (5 task, 815-800)**: agent-dispatch/runtime, dialog_turn/turn_subhandlers, round_executor, weixin_bot_media, session_message_tool
- **R48 (5 task, 795-762)**: ai-adapters/stream/types/gemini, execution/compression, insights/collector, tool-execution/fs/edit_file, config/manager
- **R49 (5 task, 760-756)**: transcript_export, session_restore, core/message, mcp_tools, session_evidence

**Branch**: `fix/r40-r50-rework` from main `4841f3bd` (no merges yet, R40-R43 worktrees preserved with commits)

✅ R50 cleanup 待办 DONE (2026-07-14 cleanup pass; superseded by 4 days of R60+ rounds; see commit c933e490 for current main HEAD):

**Plan files** (docs/superpowers/plans/): 顶层 `round40-r50-rework-with-step37flash-2026-07-06.md`; per-round yaml `round{N}-*-2026-07-{06|07}.yaml` (R40-R43 ✅ done, R44 OLD cancelled, R44-R49 real-paths ⏳ await user dispatch); R44 review report `round44-r49-review-guide-2026-07-07.md` (APPROVE 8.5/10 + 2 P2 fixes); re-derived `round44-r49-real-god-objects-review-guide-2026-07-07.md`。

**R60 chain** (2026-07-10, god-method get_xxx → method rename):
- **R60a-d**: 5 commits (`3766ebdc` R60d Kimi P0 C4 audit, `491f2c5a` Mavis take-over 267 files fix, `5e788643` R60c-ext rename, `a6d6f71d` pre-existing PromptBuilder fix, `aff5e243` Mavis audit fix, `99969708` HANDOFF bump)
- **Net R60c getter rename scope** (across `491f2c5a` + `5e788643`): 1 safe argless `&self` rename `get_workspace_context → workspace_context`; 49 callers of `get_global_workspace_service → global_workspace_service`; 33 callers of `get_path_manager_arc → path_manager_arc` (partial); 21+ callers of `get_agent_registry → agent_registry`; 19 callers of `get_event_system → event_system`; 73 distinct `pub fn get_xxx` correctly skipped (per R60c spec — argument-taking or `get_or_*`/`get_mut_*`/`try_get_*` patterns). Audit in `r60c-ext-skip-audit.txt`。
- **R60d cross-crate regression教训** (CRITICAL, see "Cross-crate rename 5-step recipe" below)

**R61 P1 cleanup** (2026-07-10, plan_2f5016d5, 4 parallel tasks): R61a `6b8c3a3a` agent-runtime lib.rs (22 pub use re-exports, 9 modules); R61b `09895476` terminal exec mod.rs (pub mod → pub(crate) for manager/output/platform); R61c `aca3b356` apps/cli tests (15 test functions in 3 files, 34 tests pass); R61d `e6ea96d1` services-integrations boundary (moved DeepResearch to runtime-ports — REAL fix, not just audit). 4 R61 commits + 1 HANDOFF bump = 5 commits. Main HEAD = `c933e490` (as of 2026-07-14; see `git log -1 origin/main` for current). cargo check --workspace = 0 errors. R60 → R61 fully closes Kimi P0 + P1 items.

## Sub-domain split 4 类隐式错误

| 类别 | 触发条件 | 错误代码 | fix |
|---|---|---|---|
| Import paths | `mod.rs` 用 `use super::*`; sibling files 用 `use super::super::*` | E0432 | 严格按文件层级 |
| Sibling method visibility | `impl SomeStruct { fn foo() }` 默认私有, sibling 调 `self.foo()` 看不到 | E0624 | bulk `pub(super)` via Python script |
| Struct field visibility | 跨 sibling struct literal 访问字段, 字段 default 私有 | E0616 | fields `pub(crate)` 或 `pub(super)` |
| Cargo.lock drift | gitignore Cargo.lock, main HEAD 锁 rmcp X.Y, worker build 自动升 X.Y+1 | E0308 | baseline commit 上 `cargo check` 重现; worktree `--frozen` |

## Cross-crate consumer visibility rule (R19 教训, critical)

**Spec 不要 over-prescribe `pub(super)`**: 默认 `pub fn` for `impl Service { ... }`; 只 `pub(super)` 当 method 是 crate-internal cross-sibling helper。Producer 应该 push back over-prescriptive visibility specs。R18 control_hub_tool 的 `pub(super)` pattern 只适用于 no-cross-crate-consumer 的 god-object。

**Verification 必跑**: (1) `git grep '<ServiceName>' -- ':!<target_crate>/'` 找所有 cross-crate consumers; (2) 每个 consumer crate 必 `cargo check -p <consumer_crate>` — `cargo check --workspace` alone 不足; (3) 跨 crate 改动时**只**走 QClaw review (R19 教训: visibility regression)。

## Cross-crate rename 5-step recipe (R60d 教训, 2026-07-10)

1. **Identify scope**: `git grep -nE '<old_name>' -- 'src/'` 找所有 callers
2. **Skip definition site**: explicit file exclusion in Python script (e.g. `path.endswith(r'src\crates\services\workspace\service.rs')`)
3. **Skip related functions**: negative lookbehind `(?<!set_)<old_name>\b` 避免 rename set_X / with_X / etc.
4. **Bulk edit via Python**: 28 files / 49 replacements in one run (~5s) vs per-file Edit tool calls (~5min for same scope)
5. **Verify `cargo check --workspace`** (NOT just `-p <crate>`): catches cross-crate consumer breakage

**R60d specific 教训**: 2 renames missed by producer's "verified cargo check -p X" claim
- `ShellDetector::get_default_shell() → default_shell()` rename — 3 callers in other crates kept old name
- `get_global_workspace_service() → global_workspace_service()` rename — 49 callers in 28 files (all in `assembly/core`) kept old name. `set_global_workspace_service` (different fn) was correctly preserved

**Fix pattern** (Mavis take-over): 1 h commit (491f2c5a, 267 files / +1101/-1570) — bulk rename via Python + 236 file rustfmt co-pass. Verified: `cargo check -p northhing-core --features product-full --lib` = 1 pre-existing error (matches HEAD~1 baseline), `cargo fmt --check -p northhing-core` = 0 diffs。

## R26: interface crate 拆分模式 (runtime-ports/lib.rs 2460 → 4 sibling)

`northhing-runtime-ports` 是 interface crate per AGENTS.md。不同于 R23/R24 impl-block splits。

Cross-sibling use 最小集:
- port_core: base
- session_workspace: `use super::port_core::{PortError, PortResult, RuntimeServicePort};`
- remote: `use super::port_core::RuntimeServicePort; use super::session_workspace::WorkspaceFileSystem;`
- agent: `use super::port_core::{PortError, PortResult};`

5 sibling vs 4 sibling decision: agent_dialog + submission_events 合并入 agent sibling 因为 30+ cross-refs → reduced to 0 errors。

### Python f-string `{{` 陷阱 (R26 教训)

Python f-string `{{` 和 `}}` 产生 literal `{{` 和 `}}` (不是 `{` `}`)。4 sibling 全部 broken `use serde::{{Deserialize, Serialize}};`。**Fix**: 用 raw string `r'''...'''` 或 escape `{{` → `{` after writing。

### `#[serde(...)]` attribute scope (R26 教训)

`#[serde(rename_all = ...)]` attribute requires `use serde;` (bare import) in scope, OR `#[derive(Serialize, Deserialize)]` BEFORE the `#[serde(...)]` line. Without either: `error: cannot find attribute 'serde' in this scope`. R26 range off-by-one 丢 1 line, 导致首个 `#[serde(...)]` 前无 derive, manual fix。

### Range off-by-one (R26 + R27 + R27b lesson)

Python `lines[start:end+1]` = **INCLUSIVE end**. So `lines[0:296]` = items 0-295 = L1-L296 (296 items). Always: (1) `rg -n` 找实际 line numbers; (2) use `lines[actual_start:actual_end+1]` explicit; (3) after extraction, check first/last line matches expected。

## R27: horizontal split + R28 deferred + R27b fix

R27 = `workspace/manager.rs` 1505 → types.rs (300) + manager_impl.rs (1234). 1234 < 1505 but still over 800 cap → sub-domain split needed.

**R27 Key fixes**: (1) `pub(super) fn default()` rejected by Rust compiler (trait Default has implicit visibility). Strip `pub(super)` from `impl Default` blocks. (2) `pub(crate) const` items NOT re-exported via `pub use ...::*`. Change to `pub const` if cross-sibling visibility needed. (3) mod.rs re-export list needs to include new items explicitly, OR change to `pub use manager::*;` wildcard.

**R28 deferred** (terminal/session/manager 1457 → 2 sibling 失败): (1) `pub(super) fn drop()` rejected (`impl Drop` trait has implicit visibility). Strip `pub(super)`. (2) Cross-sibling types like `CommandStreamEvent` 不在 session_manager.rs, 因为 mod.rs 有 explicit `pub use types::{X, Y, Z}` list. Either change to `pub use types::*;` wildcard OR add explicit `use super::types::{X, Y, Z};` (3) Original use block (29 lines) needs to be in BOTH siblings. Copy-paste creates duplicate warnings.

R28 retry strategy: 改用 R28b sub-domain split 模式 (见下)。

### Pattern: when to use horizontal vs sub-domain split

| File structure | Recommended split |
|---|---|
| Many `impl` blocks on one struct, each on a sub-domain | Sub-domain (R23: factory/lifecycle/identity_watch) |
| One big `impl` block + many private fields | Horizontal (R27: types vs manager_impl) |
| Many `impl` blocks on different structs, sub-domain clear | Sub-domain (R24: entry/snapshot/breakdowns/utilities) |
| Trait + DTO god-file with cross-references | Interface crate pattern (R26) |
| DTO god-file (struct/enum with many impl Default) | Deferred → R28b sub-domain |

## R27b: sub-domain split — `pub(super)` on private fields

Split `impl WorkspaceManager { ... }` (736 lines, 48 methods) into 3 sibling files (lifecycle + accessors + info). Original `WorkspaceManager.workspaces` private → 100+ "field private" errors。

**Fix**: Add `pub(super)` to all 6 fields so they're visible to siblings within `workspace` module. Struct stays `pub`. Fields not exposed in public API (pub(super) = crate-private, NOT pub)。Behavior change (private → pub(super)), but acceptable for cross-sibling split goal. Document in stage summary。

```rust
pub struct WorkspaceManager {
    pub(super) workspaces: HashMap<...>,
    pub(super) opened_workspace_ids: Vec<String>,
    pub(super) current_workspace_id: Option<String>,
    pub(super) recent_workspaces: Vec<String>,
    pub(super) recent_assistant_workspaces: Vec<String>,
    pub(super) max_recent_workspaces: usize,
}
```

### R27b final result

| Sibling | Lines | Sub-domain |
|---|---|---|
| `manager.rs` (facade) | 10 | re-exports |
| `types.rs` | 300 | impl WorkspaceIdentity |
| `workspace_info_impl.rs` | 487 | impl WorkspaceInfo + WorkspaceSummary + WorkspaceManager struct + WorkspaceManagerConfig + impl Default |
| `manager_lifecycle.rs` | 439 | impl WorkspaceManager L487-L900 (new/rekey/migrate/open/close/set_active/set_current) |
| `manager_accessors.rs` | 363 | impl WorkspaceManager L900-L1233 (get/list/search/remove/cleanup/recent/statistics + WorkspaceManagerStatistics) |

All 4 sibling < 800 cap. Largest 487。

### Visibility rule for cross-sibling impl blocks

When impl X { ... } in module A, another impl X { ... } in sibling B, both need access to X's private fields. If A and B in SAME module (both `pub mod` children of `workspace`), private fields NOT visible to B. Fix: `pub(super)` on fields (visible to all `pub mod` children of parent module). Alternative: keep all impl X { ... } in same file (no split)。

### QClaw vs Kimi pattern (R27 + R27b)

| Reviewer | R27 verdict | R27b verdict (next) |
|---|---|---|
| Kimi | 9.2/10 APPROVE (P3 minor) | not yet |
| QClaw | 7.5/10 CONDITIONAL (1 blocker) | should APPROVE post-fix |

QClaw catches: line cap violation, exact file size, count drift。Kimi catches: conceptual issue, design observation。Both reviewers needed for quality gate。

## R28b: Horizontal sub-domain split pattern (same struct, multi-impl blocks)

When god-object is SINGLE struct with 1000+ line `impl X { ... }` block, split the impl across sibling files (vs R30 facade delegate pattern for DIFFERENT structs)。

Approach:
1. Keep `struct X { fields }` in facade file (session_manager.rs)
2. Convert all private fields to `pub(super)` so siblings can access
3. Create 4 sibling files, each with `impl X { ... }` opening + closing
4. Method names MUST be DISJOINT across siblings (else E0592)
5. Free helper fns + cross-sibling inherent methods need `pub(super)` too
6. Update parent module (`mod.rs`) to declare new sibling `mod`s

When to use horizontal vs facade delegate:

| File structure | Recommended split |
|---|---|
| One struct, big impl block, method sub-domains clear | **Horizontal (R28b)** — `impl X` x N sibling |
| Many distinct structs, each with own impl | **Facade delegate (R30)** — `pub(super) fn name_impl(mgr, ...)` free fns |

Cross-crate consumers: wildcard re-exports (`pub use manager::*;`) in mod.rs preserve all 67+ import paths verified by QClaw。

R28b final: facade (170 lines) + 4 sibling (430/231/215/474 all < 800 cap). Largest sibling limited by intrinsically large method (`execute_command_stream_with_options` at 297 lines, parent bucket 不能 flatten without losing cohesion)。

## R31: Never infer DTO fields from consumer code patterns

When splitting god-file with DTO structs + `From` impls, ALWAYS verify ORIGINAL field schema via:

  git show HEAD:<path-to-original-file>

...BEFORE writing sibling types.rs. Do NOT infer fields from `From<TerminalSession>` impl patterns or consumer code dot-access.

R31 misdraft: `SessionResponse` initially had `working_directory / created_at / last_activity / metadata` fields — 3 of 4 INVENTED. Real original had `cwd / status / cols / rows / source`。Discovered only when `bash_tool.rs:711/760` and `control_hub_tool_terminal.rs:48` consumer code accessed `.cwd` (not `.working_directory`) and 3 fields had no consumers。

Consumer-code-only verification is INSUFFICIENT for DTO schema. The truth is in the original struct definition, not in how callers use it。

## Reviewer line count drift (QClaw catch)

When reviewer claims "X fmt diffs" or "Y files changed", DO re-verify with `git diff --stat` + `wc -l`。QClaw claimed 11 fmt diffs; reality was 13 diff blocks in 17 files (40+/-40 lines)。Verifying avoided both over-claiming AND missing actual diffs hidden between blocks。

Also: `cargo fmt --check --message-format=short` output streams trailing newlines as separate "Diff in <file>:<line>" — count Diff blocks not diff hunks. Re-verify by:

  cargo fmt --check -p <crate> 2>&1 | rg -c 'Diff in'

## User-driven review-fix cleanup vs Mavis self-cleanup

User rule: "review-fix-cleanup cycle by user driven, Mavis 不跑" means the cycle is driven BY the user, but the actual fix actions can still go through Mavis when user explicitly delegates。QClaw flagged fmt + BOM as minor; Kimi flagged BOM only — both are sub-30-second fixes that Mavis SHOULD execute, not wait on user to do manually。

Distinction: "user 驱动" = user decides WHICH fixes to apply, NOT "Mavis does nothing until user runs git commit themselves"。When user picks scope ("all 3 together"), Mavis executes the chosen fixes in single subagent or self。

## R37-R39 batch lessons (21 rounds across 3 batches, 2026-07-05)

**整体成果**:
- R37 9-way parallel (8.7/10 APPROVE Kimi) → squash to main `61224e6c`
- R38 7-way parallel (8.8/10 APPROVE QClaw) → squash to main `c975ba9c`
- R39 5-way parallel (8.8/10 APPROVE QClaw) → squash to main `4841f3bd`

**KPI across 21 rounds**: 0 compile errors / 0 NEW unwrap/panic/unreachable / 0 cross-crate module references / 3 Mavis take-overs (R37a/d/h scratch cleanup, R39a/c M3 timeout) / 18 producer successes (86% success rate) / Thinnest facades: R39b 23 lines, R39d 35 lines。

### 黄金模式 (golden pattern, all 21 rounds converged)

```rust
// mod.rs (1-10 lines)
pub mod sibling_a;
pub mod sibling_b;
pub use sibling_a::*;  // wildcard facade re-export

// sibling files each < 800 cap
// cross-sibling private access via pub(super)
struct PrivateField { pub(super) x: T }
```

Why this works:
1. `pub use sibling::*` preserves 67+ downstream import paths (verified by QClaw)
2. `pub(super)` allows cross-sibling impl blocks to share private fields
3. mod.rs becomes 1-3 lines (negligible cognitive load)
4. Each sibling file = single sub-domain focus (rendering / IO / tests / helpers)

### M3 vs M2.7 model timeout matrix

| God-object size | M2.7 success | M3 success | Notes |
|---|---|---|---|
| < 1000 lines | high | high | Both work |
| 1000-1300 | high | high | Both work (R37d, R37f, R38a/d/e/f/g) |
| 1300-1600 | high | low (30min timeout) | M3 take-over zone |
| > 2000 | low | very low | Both need Mavis take-over |

**R39 M3 take-overs**: R39a (weixin 2157), R39c (workspace_search 1315)。
**Pattern**: M3 worker should NOT be dispatched on god-objects > 1300 lines. Pre-emptive split or use M2.7.

### Mavis take-over playbook (refined from R23/R39)

1. Worker dispatched via `mavis team plan run` (30min default timeout)
2. Monitor cron tick checks `mavis team plan status` every 30 min
3. If `status: producing` + `attempt: 0` past 30min: auto-pause imminent
4. Mavis: cancel plan, salvage from worktree (files preserved despite cancelled status)
5. Mavis: simplify split scope (extract helpers only, not full sub-domain)
6. Mavis: single commit `refactor(<crate>): Rxx split <file> N -> A + B (Mavis take-over after M3 timeout)`
7. Mavis: write `*-take-over-summary.md` handoff for reviewer
8. User/agent review continues normally

WHY: 3 take-overs in 21 rounds = 14% rate. Playbook proven. Worker-side mitigations (extend-timeout, more context) insufficient for > 1300 line god-objects.

### Reviewer attribution discipline (verified)

| Reviewer | Style | Score range | Files written by |
|---|---|---|---|
| Kimi (verbal, user-relayed) | conceptual + design obs | 8.7-9.2 | Mavis draft, user reviews |
| QClaw (autonomous) | technical + exact counts + line refs | 7.5-8.8 | Reviewer self-commits |

**Rule** (from user 2026-06-24): Verbal user-relayed review = Kimi, NEVER QClaw; Autonomous git-committing reviewer = QClaw, NEVER Kimi; Mavis draft reports MUST label `Reviewer: marvis` (not external)。

## R40+ guidance (next round)

- Target: < 1300 lines per god-object (M3-safe) OR pre-emptive sub-domain plan
- Always include Mavis take-over steps in plan YAML (worker may still timeout)
- Wildcard `pub use super::*` in mod.rs (1-3 lines facade)
- All siblings < 800 lines (acceptable: 800-970 with review note)
- `cargo check --workspace` clean before commit
- `git diff --stat` review before commit (QClaw catches drift)

## Plan dispatch pitfalls (R16 startup)

`mavis team plan run --plan-yaml <file>` 常返回 `Invalid plan` 错误: (1) **Copy known-good skeleton** from `~/.mavis/plans/<previous plan-id>/plan.yaml`, Edit 3 字段 `name`/`tasks[0].id`/`tasks[0].title`, 保留 block-scalar 缩进; (2) **Prompt body 简短** — 完整 R(N)+1 指令通过 `mavis team plan steer --message` post-dispatch; (3) **Multi-task plans 被拒** when QClaw + Kimi 双 review — solution: Mavis 写 review file based on user-relayed verdicts, 不动 plan YAML; (4) **`mavis team plan steer --abort-workers`** 强制 abort; (5) **`mavis team plan extend-timeout`** R(N) split > 2000 lines 必传 `<plan-id> <task-id>` (不是只 plan-id)。详细 provider config + 30 min cap + decision schema → `mavis-runtime.md`。

## Pre-existing error attribution 流程 (worker + verifier 都做)

```
1. worker 跑 cargo check 失败
2. git stash
3. checkout 已知干净 baseline commit (如 cabcec2)
4. 重跑 cargo check, 记录 pre-existing errors
5. checkout 回 worktree
6. git stash pop
7. 对比 baseline vs worktree error diff
8. diff 为空 → claim "out of scope" valid
9. diff 非空 → 自己引入的 regression
```

`impl-misc-cleanups` worker 这样做救了一次 review cycle (transport_remote.rs E0308 复现确认)。也见 `northing-split-execution.md` 验证 discipline 段。

## R14 P0/P1 candidates (现状, 2026-07-10)

`bot/command_router.rs` (R14) / `control_hub_tool.rs` (R16/R18) / `acp/client/manager.rs` (R19/R20) / `runtime-ports/lib.rs` (R26) — **全部 P0/P1 完成**。`terminal/exec.rs` 2488 在 R37-R39 batch 处理。`review_platform/mod.rs` 319 NOT P0 (Kimi 误报 4866, 驳回)。

## R73-1/2/3 status (2026-07-12 QClaw APPROVED 9.3/10)

### Done

| Pick | Audit claim | Actual | Status |
|---|---|---|---|
| R73-1 path_manager | 705 lines, domain split | confirmed | ✅ DONE `edaf468c` (251 entry + 4 sub) |
| R73-2 turn_batch | 694 lines, append/flush | multi-impl pattern (NOT append/flush) | ✅ DONE `24a59f34` (268 entry + 2 sub) |
| R73-3 github (audit) | 676 lines, per-operation | **331 lines (NOT god)**, audit was wrong | ❌ REVISED to skill_agent_snapshot |

### Revised R73-3 = skill_agent_snapshot

- `agentic/skill_agent_snapshot.rs` 633 lines (was NOT in audit's 5 picks, picked by
  Mavis because audit's R73-3 was wrong)
- Phase split: types / resolution / diff_render (3 sub-modules)
- ✅ DONE `b254db80` (115 entry + 3 sub)
- First multi-reviewer dispatch (plan_df939a4c) + Mavis M3 take-over contingency

### Remaining (paused)

- `agentic/tools/implementations/git_tool/mod.rs` 660 (per-operation split)
- `service/remote_connect/connect.rs` 741 (multi-protocol split, biggest win)

### Audit accuracy note

R73 audit's line counts were based on **byte size**, not line count. github.rs
claimed 676 lines but actual is 331. This means the audit's 5 picks may have
inaccuracies; the actual god files in production might differ. Before picking
the next R73 target, **always re-verify line count with `py -c "sum(1 for _ in f)"`**
on the candidate file.

Verified 2026-07-12 actual line counts for the 5 audit picks:
- path_manager 705 (correct)
- turn_batch 694 (correct)
- github 331 (audit was wrong, claimed 676)
- git_tool/mod.rs 660 (correct, per separate check)
- remote_connect/connect.rs 741 (correct)
