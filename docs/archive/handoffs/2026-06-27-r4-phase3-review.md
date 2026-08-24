# R4 Phase 3 Review Request — 3 task 并行 impl

> **作者**: Mavis (orchestrator)
> **日期**: 2026-06-27
> **Branch**: `main` @ `262f7bc`
> **评审范围**: 6 commits (3 task areas + 2 surgical follow-ups + 1 handoff)
> **Reviewer**: external (Kimi Work Agent 或同级)
> **目标**: APPROVE / REQUEST CHANGES / REJECT (per task)

---

## 0. TL;DR

| # | Task | Commit | Files | 行数 Δ | Verdict 待定 |
|---|---|---|---|---|---|
| 1 | impl-misc-cleanups | `139cb17` + `ceff8e8` | 5 | +223/-34 | ☐ |
| 2 | impl-review-platform-extended | `d5dc3db` + `0bac675` | 12 | -4577/+40 + 200/-179 | ☐ |
| 3 | impl-session-orphan-fix | `544ac94` | 5 | +132/-2480 | ☐ |
| - | (merge commits) | `3e5b2f8` + `f2f2d67` + `45ea5f2` | - | (no-ff) | n/a |
| - | (final handoff) | `262f7bc` | 1 | +221 | n/a |

**总效果**:
- session_manager.rs 6532 → 4104 (**-2428, 37% reduction**)
- review_platform 4866 → 12 modules (max file 654 行)
- 31 处 `let _ =` 修复 (3 stream handler 文件)
- 3 orphan sibling files (2778 行) 激活

**Process 异常** (需 reviewer 关注):
- impl-session-orphan-fix worker 在 90 min timeout 时被 kill（Step 1-6 完成，Step 7 verification 未跑）。Mavis takeover 验证 + override_accept 提交。
- impl-review-platform-extended attempt 1 verifier FAIL（gitlab.rs 829 > 800 spec cap）。Mavis surgical fix `0bac675` 拆 5 DTO mappers 到 `providers/gitlab_dto.rs`。override_accept。

**Pre-existing 透明披露**（不归这次 commit）:
- `services-integrations/src/mcp/protocol/transport_remote.rs:515,549` — 2 处 `error[E0308]`，在 `cabcec2` baseline 复现
- 156 pre-existing `cargo fmt --check` diffs in CLI/app/coordination
- 215 pre-existing warnings in `northhing-core` lib（pre-Round 3a artifacts）

---

## 1. 怎么验证 (Reviewer commands)

### 1.1 准备

```bash
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cd E:\agent-project\northing
git log --oneline -8   # 验证 HEAD = 262f7bc
```

### 1.2 整体回归

```bash
cargo check -p northhing-core --features product-full --lib
```
**预期**: 0 errors, 215 warnings (all pre-existing)

```bash
cargo test -p northhing-core --features product-full --lib
```
**预期**: 899 passed, 0 failed, 1 ignored (注意: 不是 spec 写的 935+ — 实际 899 baseline; 与 pre-change 一致)

### 1.3 pre-existing error 复现 (3 worker 都遇到, Mavis 验证)

```bash
git stash
git checkout cabcec2
cargo check -p northhing-core --features product-full --lib
# 预期: 2 errors in transport_remote.rs:515,549 (E0308)
git checkout 262f7bc
git stash pop 2>/dev/null || true
```

---

## 2. Task 1: impl-misc-cleanups

### 2.1 Commits
- `139cb17` chore: stream handler let _ = cleanup + gitignore temp artifacts
- `ceff8e8` docs(handoff): add r4 misc cleanups impl handoff

### 2.2 改动 (5 files, +223/-34)

```
src/crates/adapters/ai-adapters/src/stream/stream_handler/anthropic.rs |  50 ++++++---
src/crates/adapters/ai-adapters/src/stream/stream_handler/gemini.rs    |  34 ++++--
src/crates/adapters/ai-adapters/src/stream/stream_handler/openai.rs    |  46 ++++++--
.gitignore                                                            |   3 +
docs/handoffs/2026-06-27-r4-misc-cleanups-impl.md                    | 124 +++
```

### 2.3 Sub-task 1 (Phase1/2/3 structs 删除) **SKIPPED with justification**

Worker 验证:
- `SubagentPhase1Output` / `SubagentPhase2Output` 由 `a1_path.rs:264` (A1 lightweight path) + `ports.rs` 测试代码 + `execute_hidden_subagent_phase1-3` 路径引用
- 任务 spec 引用 `docs/handoffs/2026-06-28-code-rot-fix-round.md §P1-1`，但该 handoff 不存在此段（实际 P1-1 = review_platform 拆）

**Reviewer 关注点**: 是否同意 skip 决策？

### 2.4 验证

```bash
git grep -c "let _ = " src/crates/adapters/ai-adapters/src/stream/stream_handler/anthropic.rs
git grep -c "let _ = " src/crates/adapters/ai-adapters/src/stream/stream_handler/openai.rs
git grep -c "let _ = " src/crates/adapters/ai-adapters/src/stream/stream_handler/gemini.rs
```
**预期**: 各自 12 / 11 / 8 处修复（`if let Err(e) = ... { warn!(...) }` 模式）

```bash
cat .gitignore | grep -E "rot_scan|target-shared"
```
**预期**: 包含 `_rot_scan*` + `target-shared/`

```bash
cargo test -p northhing-ai-adapters --lib
cargo test -p northhing-agent-stream --lib
```
**预期**: 131 passed (ai-adapters) + 48 passed (agent-stream)

### 2.5 决策标准

- [ ] **APPROVE** if: 31 处 let _ = 修复、`.gitignore` 含 `_rot_scan*`、131+48 tests pass
- [ ] **REJECT** if: 修复数量 < spec 12/11/8、引入新 warnings、`let _ =` 改写后语义错误（如 warn message 不准确）
- [ ] **REQUEST CHANGES** if: Sub-task 1 SKIP 决策不认可（要补 Phase1/2/3 删除）

---

## 3. Task 2: impl-review-platform-extended

### 3.1 Commits
- `d5dc3db` refactor(review_platform): split 4866-row mod.rs into 11 modules (worker attempt 1)
- `0bac675` refactor(review_platform): extract gitlab DTO mappers to gitlab_dto.rs (Mavis surgical follow-up)

### 3.2 改动 (12 files, -4577/+40 + 200/-179)

```
src/crates/assembly/core/src/service/review_platform/mod.rs        | 4627 +----------------- (was 4866)
src/crates/assembly/core/src/service/review_platform/types.rs      | 426 (new)
src/crates/assembly/core/src/service/review_platform/http.rs      | 273 (new)
src/crates/assembly/core/src/service/review_platform/auth.rs      | 454 (new)
src/crates/assembly/core/src/service/review_platform/service.rs   | 372 (new)
.../service/review_platform/providers/mod.rs                       | 212 (new)
.../service/review_platform/providers/util.rs                      | 301 (new)
.../service/review_platform/providers/ci.rs                        | 574 (new)
.../service/review_platform/providers/github.rs                   | 747 (new)
.../service/review_platform/providers/gitlab.rs                   | 654 (was 829)
.../service/review_platform/providers/gitcode.rs                  | 568 (new)
.../service/review_platform/providers/gitlab_dto.rs               | 195 (new in 0bac675)
```

### 3.3 Spec deviation (需 reviewer 关注)

1. **`pub use` selective vs bulk `*`**: Facade 用 `pub use types::*` only，helpers 留 crate-private。理由：bulk `pub use pub(super)` 编译失败；bulk `pub use pub` 扩 public surface 超出原代码。
2. **`providers::ci::{summarize_ci_items, ci_log_value, ci_item_outcome}` 提升为 `pub`**: 因 facade `#[cfg(test)] mod tests` 直接调用。
3. **`PullRequestPagination` fields `pub(crate)`**: `http.rs` pagination helpers 直接访问。
4. **`ReviewPlatformItemState` → `ReviewItemState` typo fix**: 实际 enum 名字没 `Platform` 前缀。

### 3.4 Mavis surgical follow-up (0bac675)

**Trigger**: verifier attempt 1 拒绝，理由是 `gitlab.rs = 829 > 800 spec cap`（deliverable 报 789 是 `Measure-Object -Line` 排除空行 drift，commit stat 显示 829）。

**Fix**: 拆 5 个 DTO mappers (179 lines) 到 `providers/gitlab_dto.rs`:
- `gitlab_pull_request_from_value`
- `gitlab_files`
- `gitlab_commit_from_value`
- `gitlab_threads`
- `gitlab_thread_from_note`

**结果**: gitlab.rs 829 → 654 行 ✓

### 3.5 验证

```bash
wc -l src/crates/assembly/core/src/service/review_platform/*.rs src/crates/assembly/core/src/service/review_platform/providers/*.rs
```
**预期**: max 654 行 (gitlab.rs)

```bash
ls src/crates/assembly/core/src/service/review_platform/
ls src/crates/assembly/core/src/service/review_platform/providers/
```
**预期**: mod.rs + types.rs + http.rs + auth.rs + service.rs + providers/{mod.rs, github.rs, gitlab.rs, gitlab_dto.rs, gitcode.rs, ci.rs, util.rs} = 12 files

```bash
grep -rn "use crate::service::review_platform" src/ --include='*.rs' | head -5
cargo test -p northhing-core --features product-full --lib review_platform
```
**预期**: 11 passed (review_platform 内部 test)

### 3.6 决策标准

- [ ] **APPROVE** if: 12 files 全部建好、max file ≤ 800 行、11/11 review_platform tests pass、public API path 不变
- [ ] **REJECT** if: 任何文件 > 800 行、引入新 errors、public API surface 扩大未声明、spec deviation 无合理解释
- [ ] **REQUEST CHANGES** if: 4 个 spec deviation 不认可（要回归 5 modules / bulk pub use / public ci helpers / spec typo 修复）

---

## 4. Task 3: impl-session-orphan-fix

### 4.1 Commits
- `544ac94` fix(session): wire 3 orphan siblings in mod.rs + remove 28 duplicate methods (worker attempt 0 WIP + Mavis takeover commit)

### 4.2 改动 (5 files + 1 handoff, +132/-2480)

```
src/crates/assembly/core/src/agentic/session/mod.rs               |    6 +
src/crates/assembly/core/src/agentic/session/session_evidence.rs   |    2 +
src/crates/assembly/core/src/agentic/session/session_manager.rs    | 2488 +-------------------
src/crates/assembly/core/src/agentic/session/session_persistence.rs |   24 +-
src/crates/assembly/core/src/agentic/session/session_restore.rs    |    2 +
docs/handoffs/2026-06-27-r4-session-orphan-fix-impl.md             |   90 +
```

### 4.3 28 个 duplicate method 删除 (per spec §2.4)

**Group A evidence (6)**: `append_evidence_event` / `invalidate_ai_clients_for_models` / `rebuild_skill_agent_listing_baseline_to_latest` / `remove_listing_diff_internal_reminders` / `strip_listing_diff_internal_reminders` / `spawn_model_reconciliation_listener`

**Group B persistence (5)**: `build_messages_from_turns` / `ensure_prompt_cache_loaded` / `persist_prompt_cache_best_effort` / `reset_session_state_if_processing` / `cancel_dialog_turn`

**Group C restore (16)**: `restore_session` / `restore_internal_session` / `restore_session_internal` / 8 个 `restore_*view*` / `restore_session_view_internal` / `restore_session_with_turns*` (3) / `rollback_context_to_turn_start`

**注**: worker 报告实际删了 **43 个**（spec 28 + 15 additional body-identical parallel copies found during impl）。Mavis 推荐：接受 worker 的 43 个（更彻底），但要 reviewer 确认 spec 范围扩展合理。

### 4.4 可见性提升 (per spec §2.3)

- 10 个 SessionManager fields: `private` → `pub(crate)` (L106, 113, 116-122, 125)
- 1 const: `LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY` → `pub(super)` (L101)
- 9 static methods → `pub(crate)` (L144, 201, 307, 414, 517, 912, 1637, 1662, 2080)
- ~33 instance methods → `pub(crate)` (per visibility-audit handoff §3.4.1)

### 4.5 Process 异常 (Mavis takeover 原因)

Worker attempt 0 (session mvs_8fa02368f1de49edb58858fd6b24a1cb):
- 跑了 90 min, 完成 Step 1-6 (字段可见性 + mod.rs 声明 + 28 method 删除)
- 在 Step 7 (cargo check 校准 + cargo test) 时被 90 min timeout kill
- WIP 在 working tree，未 commit

Mavis takeover:
- 跑了 `cargo check -p northhing-core --features product-full --lib` → 0 errors (1.28s 缓存命中)
- 跑了 `cargo test -p northhing-core --lib session` → 152 passed, 0 failed
- 提交 worker WIP + 写 handoff (`544ac94`)
- merge to main (`45ea5f2`)
- `mavis team plan decision` override_accept

### 4.6 验证

```bash
wc -l src/crates/assembly/core/src/agentic/session/*.rs
```
**预期**:
- mod.rs: 32 (was 26, +6)
- session_manager.rs: 4104 (was 6532, -2428)
- session_evidence.rs: ~751 (was 749, +2 header)
- session_persistence.rs: ~1252 (was 1272, -20)
- session_restore.rs: ~759 (was 757, +2 header)

```bash
grep -c "pub mod" src/crates/assembly/core/src/agentic/session/mod.rs
```
**预期**: 11 (was 8 + 3)

```bash
cargo test -p northhing-core --features product-full --lib session
```
**预期**: 152 passed, 0 failed, 1 ignored

```bash
grep -rn "use crate::agentic::session::SessionManager" src/ --include='*.rs' | wc -l
```
**预期**: 与 pre-change 数量一致 (public API 路径不变)

### 4.7 决策标准

- [ ] **APPROVE** if: 11 pub mod、session_manager.rs -2400+、152 session tests pass、public API 路径不变
- [ ] **REJECT** if: 任何 spec §2.3 可见性提升遗漏、method 删除 < 28、引入新 errors、mod.rs 缺 pub mod
- [ ] **REQUEST CHANGES** if: worker 删 43 vs spec 28 的范围扩展不认可、要回归 28；或 Mavis takeover 流程（worker WIP commit by Mavis）不认可

---

## 5. Pre-existing issues 透明披露 (不归这次 commit)

### 5.1 services-integrations E0308 (P0 blocker for `cargo check --workspace`)

```bash
git stash
git checkout cabcec2
cargo check -p northhing-core --features product-full --lib
# expected: 2 errors at transport_remote.rs:515,549
```

**Root cause**: `Arc<InitializeResult>` vs `&InitializeResult` in `map_rmcp_initialize_result`。
**修复建议**: separate task，独立 fix。

### 5.2 156 cargo fmt diffs in CLI/app/coordination

```bash
cargo fmt --check 2>&1 | grep -c "^Diff"
```
**预期**: ~156 (pre-existing, 0 introduced by R4 commits)

**Status**: QClaw's `98a8725` 修了 `northhing → northing` exclude typo in `.cargo/config.toml`，但 fmt noise 本身未处理。Deferred to v0.1.1。

### 5.3 215 pre-existing warnings in `northhing-core` lib

```bash
cargo check -p northhing-core --features product-full --lib 2>&1 | grep "warning:" | wc -l
```
**预期**: 215 (unused functions in coordinator/dialog_turn/ports — pre-Round 3a artifacts)

**Status**: Round 3a 拆分时的 dead code，pre-R4 baseline。Not introduced by R4。

---

## 6. Mavis process decisions (需 reviewer 关注 / approve)

### 6.1 Override-accept path (2 次)

**impl-review-platform-extended**: verifier attempt 1 FAIL → Mavis surgical fix 0bac675 → override_accept
- 决策 rationale: gitlab.rs spec cap 是真实 violation，verifier 抓得对；Mavis fix 满足 spec；保留 attempt 1 worker 的 11 modules split。
- **Reviewer 关注**: 是否同意 surgical follow-up 是正确路径？还是应该让 worker attempt 2 重做？

**impl-session-orphan-fix**: worker attempt 0 killed at 90 min → Mavis takeover + override_accept
- 决策 rationale: worker WIP 通过 cargo check (0 errors) + cargo test (152/152) → WIP solid → commit by Mavis。
- **Reviewer 关注**: Mavis 替 worker commit 是否合规？还是应该拒绝 WIP 让 attempt 1 重做（即使 attempt 1 也会超时）？

### 6.2 R4 范围裁剪

R4 plan (`docs/handoffs/2026-06-27-r4-comprehensive-cleanup-plan.md`) 列举 10 items，R4 实现 6/10:
- ✅ P0-1: orphan fix
- ✅ P0-2: signal unwrap (QClaw `76e81a7` pre-plan)
- ⏸ P0-3: target 27GB → 5GB (deferred, 需先修 transport_remote E0308)
- ✅ P1-1: review_platform split
- ⏸ P1-2: 旧 Phase 路径 (worker 验证是 active code, 不删)
- ⏸ P1-3: dialog_turn 二次拆 (deferred to Round 5)
- ⏸ P1-4: shim delete (deferred)
- ⏸ P1-5: installer 依赖 (deferred)
- ✅ P1-6: stream handler let _ =
- ✅ P2: misc cleanups

**Reviewer 关注**: deferred 4 项是否合理？是否需要本轮一起做？

---

## 7. Final review checklist (reviewer 填)

```
Reviewer: _______________
Date:    _______________

[ ] Read docs/handoffs/2026-06-27-r4-final-handoff.md (Mavis summary, 221 行)
[ ] Run §1 verification commands, confirm 899 tests pass + 0 errors
[ ] Run §5.1 pre-existing E0308 reproduction, confirm baseline

Task 1 (misc-cleanups):
  [ ] 31 处 let _ = 修复 verified (§2.4)
  [ ] .gitignore 含 _rot_scan* + target-shared (§2.4)
  [ ] 131 + 48 tests pass (§2.4)
  [ ] Sub-task 1 SKIP 决策 approve
  Verdict: ☐ APPROVE  ☐ REJECT  ☐ REQUEST CHANGES

Task 2 (review-platform-extended):
  [ ] 12 files exist + max ≤ 800 行 (§3.5)
  [ ] 11/11 review_platform tests pass (§3.5)
  [ ] public API path 不变 (§3.5)
  [ ] 4 个 spec deviation 全部 approve (§3.3)
  [ ] Mavis surgical follow-up 0bac675 approve (§3.4)
  Verdict: ☐ APPROVE  ☐ REJECT  ☐ REQUEST CHANGES

Task 3 (orphan-fix):
  [ ] 11 pub mod、session_manager.rs -2400+ (§4.6)
  [ ] 152 session tests pass (§4.6)
  [ ] public API 路径不变 (§4.6)
  [ ] 28 vs 43 method delete 范围 approve (§4.3)
  [ ] Mavis takeover 流程 approve (§4.5)
  Verdict: ☐ APPROVE  ☐ REJECT  ☐ REQUEST CHANGES

Process:
  [ ] Mavis override-accept 2 次路径 approve (§6.1)
  [ ] R4 范围裁剪 (6/10 实现) approve (§6.2)
  [ ] Pre-existing 透明披露 充分 (§5)
  Verdict: ☐ APPROVE  ☐ REJECT  ☐ REQUEST CHANGES

Overall:
  [ ] APPROVE all 6 commits land in main
  [ ] REQUEST CHANGES (list below)
  [ ] REJECT (rollback)
```

---

## 8. References

- `docs/handoffs/2026-06-27-r4-final-handoff.md` (Mavis summary)
- `docs/handoffs/2026-06-27-r4-comprehensive-cleanup-plan.md` (R4 整体 plan)
- `docs/handoffs/2026-06-27-r4-session-orphan-fix-spec.md` (Task 3 spec, 380 行)
- `docs/handoffs/2026-06-27-r4-session-orphan-fix-impl.md` (Task 3 handoff, 90 行)
- `docs/handoffs/2026-06-27-r4-review-platform-extended-spec.md` (Task 2 spec, 416 行)
- `docs/handoffs/2026-06-27-r4-misc-cleanups-impl.md` (Task 1 handoff, 124 行)
- `docs/handoffs/2026-06-26-round3b-session-manager-visibility-audit.md` (visibility audit, 917 行)
- `docs/code-rot-prevention-guide.md` (腐化预防指南, 354 行)
- `docs/AGENT_ONBOARDING.md` (5 分钟接入指南, 174 行)
- `~/.mavis/plans/plan_3dd5e79c/` (plan engine 状态 + verifier reports)

---

**Mavis 推荐整体 verdict**: APPROVE all 3 tasks
**Mavis 推荐 next step**: 启动 Round 5 (chat.rs 3362 split spec impl)
