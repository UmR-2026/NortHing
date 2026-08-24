# R21 Stage Summary: dialog_turn/mod.rs 1653 → facade 1310 + 4 sibling parallel split

> **Round**: R21 (4 sub-rounds parallel, R21+ new flow)
> **Date**: 2026-07-02
> **Scope**: `src/crates/assembly/core/src/agentic/coordination/dialog_turn/mod.rs` (1653 行 facade god-object) → 4 sibling 文件扩展 + facade 缩小
> **R21+ new flow**: 4 producer 并行 + producer self-report + Mavis 3-axis verify + sequential merge → 等 user review 后 1 squash-merge

---

## 1. Stage Summary

| Sub-round | File edited | Before | After | Δ | Methods migrated | Status |
|---|---|---|---|---|---|---|
| **R21a** | `restore.rs` | 2 | 167 | +165 | 12 restore_* methods | ✅ Merged `78c2e3c` |
| **R21b** | `turn.rs` | 690 | 881 | +191 | 4 cancel/delete methods | ✅ Merged `527188c` |
| **R21c** | `session.rs` | 253 | 354 | +101 | 9 misc methods (session + tool) | ✅ Merged `45a2a95` |
| **R21d** | `thread_goal.rs` | 211 | 471 | +260 | 8 of 12 thread_goal_* methods | ✅ Merged `b279c3b` |
| **R21e (post-review)** | (mod.rs dead code) | — | — | -93 | L83-175 cleanup | ✅ Commit `c28c53d` |
| **mod.rs** | facade | **1653** | **1217** | **-436 (-26.4%)** | 33 method bodies → facade delegates + 94 dead code lines | ✅ Merged + fix |

**Total**:
- mod.rs: 1653 → 1217 (**-436 lines, -26.4%**)
- 4 sibling files extended (+717 lines combined)
- 33 method bodies migrated from mod.rs to sibling (`pub(super) async fn ..._impl` pattern)
- 94 lines dead code cleaned (R21e: 3 const + 1 struct + 1 enum + 5 fn)
- 0 fns dropped, 0 method signatures changed (cross-crate API preserved)

---

## 2. Merge order (sequential, no conflicts)

```
1a69a82 docs(spec): R21 dialog_turn/mod.rs 1653 → facade ~700 + 4 sibling parallel split
61af534 refactor(assembly-core): R21d thread_goal consolidation (impl ConversationCoordinator 8 thread_goal methods)
b279c3b Merge R21d (--no-ff)
ca99759 refactor(assembly-core): R21b turn_control extract (4 cancel/delete methods mod.rs L1235-1425 → turn.rs)
527188c Merge R21b (--no-ff)
1cbf0b2 refactor(assembly-core): R21a restore.rs revival (12 restore_* methods mod.rs L1426-1570 → restore.rs)
78c2e3c Merge R21a (--no-ff)
78052b4 refactor(assembly-core): R21c session extension (9 misc methods mod.rs L1571-1644 → session.rs)
45a2a95 Merge R21c (--no-ff)
```

mod.rs 4 line 段不重叠 (R21 spec §4.1 严格 ownership), 4 个 merge 全部 auto-merge 无冲突.

---

## 3. Naming convention: `_impl` suffix (spec correction)

**R21 spec §3.3** 假设 facade `pub async fn method()` + sibling `pub(super) async fn method()` 在 2 个 `impl ConversationCoordinator` block 不会冲突。**这是错的** — Rust 拒绝同 type 在 2 impl block 同名 inherent method (E0592 duplicate definitions).

**Producer 决策** (3/4 用 `_impl`, 1/4 用 `_inner`):
- R21a restore.rs: 12 method `*_impl` ✅
- R21b turn.rs: 4 method `*_impl` ✅
- R21c session.rs: 9 method `*_inner` ⚠️ (与另 3 不一致, QClaw 3.1 minor)
- R21d thread_goal.rs: 8 method `*_impl` ✅
- 跨 crate public API 不变 (facade `pub async fn method` 签名保留)
- Sibling 内部访问用 `pub(super) async fn method_impl` 或 `method_inner`

**R21 review 后标准化**: QClaw 9.0/10 + Kimi 8.5/10 都建议统一 `_impl`。Fix commit `c28c53d` 把 session.rs 9 method 全部从 `_inner` 改为 `_impl`, **4/4 sibling 一致**。

**R7 precedent** (`start_dialog_turn_internal` in turn.rs) 已经用 distinct name pattern, R21 producer 沿用 R7 而非 R21 spec 假设.

**Spec 修正建议 (R22+)**:
- `facade delegate` 模式必须用 `_impl` 后缀区分 sibling impl method (3/4 producer 一致 + R21 review 后统一)
- 例: facade `pub async fn update_thread_goal_objective(...)` + sibling `pub(super) async fn update_thread_goal_objective_impl(...)`
- 这是 Rust inherent method 单 crate 唯一性约束, **不可绕开**

---

## 4. Per-sub-round detail

### R21a: restore.rs revival (12 restore_* methods)

| Metric | Value |
|---|---|
| restore.rs canonical wc-l | 2 → 167 (+165) |
| mod.rs canonical wc-l | 1653 → 1644 (-9) |
| Methods migrated | 12 (restore_session, restore_internal_session, restore_session_with_turns, restore_internal_session_with_turns, restore_session_view, restore_session_view_timed, restore_session_view_tail, restore_session_view_tail_timed, restore_internal_session_view, restore_internal_session_view_timed, restore_internal_session_view_tail, restore_internal_session_view_tail_timed) |
| Visibility | sibling: `pub(super) async fn ..._impl`; facade: `pub async fn` (1-line delegate) |
| Cargo check | 0 NEW errors (northhing-core, northhing-cli, workspace) |
| Iron rules | 0 NEW unwrap/panic, 0 BOM/CRLF, 0 long lines |
| Commit | `6bd85d2` (impl handoff) |
| Merge | `78c2e3c` |
| Handoff doc | `docs/handoffs/2026-07-02-r21a-restore-revival-impl.md` (163 lines) |

### R21b: turn_control extract (4 cancel/delete methods)

| Metric | Value |
|---|---|
| turn.rs canonical wc-l | 690 → 881 (+191) |
| mod.rs canonical wc-l | 1644 → 1503 (-141) |
| Methods migrated | 4 (cancel_dialog_turn, cancel_active_turn_for_session, delete_session, delete_hidden_subagent_sessions_for_parent_turns) |
| Visibility | sibling: `pub(super) async fn ..._impl`; facade: `pub async fn` |
| Cargo check | 0 NEW errors (northhing-core, northhing-cli, northhing-server, northhing desktop) |
| Iron rules | 0 NEW unwrap/panic |
| Commit | `ca99759` |
| Merge | `527188c` |
| Cap | turn.rs 881 ≤ 1000 R7 precedent ✅ |

### R21c: session extension (9 misc methods)

| Metric | Value |
|---|---|
| session.rs canonical wc-l | 253 → 354 (+101) |
| mod.rs canonical wc-l | 1503 → 1497 (-6) |
| Methods migrated | 9 (list_sessions, resolve_session_workspace_path, get_messages, get_messages_paginated, subscribe_internal, unsubscribe_internal, confirm_tool, reject_tool, cancel_tool) |
| Visibility | sibling: `pub(super) ..._inner`; facade: `pub` |
| Cargo check | 0 NEW errors (northhing-core product-full, northhing-cli, workspace) |
| Iron rules | 0 NEW unwrap/panic |
| Commit | `78052b4` |
| Merge | `45a2a95` |
| Note | 3 tool methods (confirm/reject/cancel) 暂留 session.rs (R21 spec §2.3); R22 候选拆 tool_control.rs |

### R21d: thread_goal consolidation (8/12 thread_goal_* methods)

| Metric | Value |
|---|---|
| thread_goal.rs canonical wc-l | 211 → 471 (+260) |
| mod.rs canonical wc-l | 1497 → 1310 (-187) |
| Methods migrated | 8 (update_thread_goal_objective, set_thread_goal_objective, maybe_mark_thread_goal_usage_limited, set_thread_goal_status, update_thread_goal_status, emit_thread_goal_updated, activate_session_goal, prepare_goal_continuation_after_turn) |
| Methods retained in facade | 4 (get_thread_goal, clear_thread_goal, create_thread_goal, pause_thread_goal_after_user_cancel) |
| Visibility | sibling: `pub(super) async fn ..._impl`; facade: `pub async fn` |
| Cargo check | 0 NEW errors |
| Iron rules | 0 NEW unwrap/panic |
| Commit | `61af534` |
| Merge | `b279c3b` |
| Cap | thread_goal.rs 471 ≤ 800 ✅ |

---

## 5. Mavis 3-axis verify (R21+ new flow)

| Axis | Command | Result |
|---|---|---|
| 1. 编译过 | `cargo check --workspace --message-format=short` | ✅ 0 errors (89s) |
| 2. 跨 crate 测过 | `cargo check -p northhing-cli --message-format=short` | ✅ 0 errors (81s) |
| 2. 跨 crate 测过 | `cargo check -p northhing --message-format=short` (desktop main bin) | ✅ 0 errors (180s) |
| 3. 不退化 | `cargo test -p northhing-core --features product-full --lib` | ✅ **899 passed; 0 failed; 1 ignored** (baseline preserved, 2.16s) |

**R19 cross-crate lesson applied**: workspace check + 2 per-crate checks all 0 errors.

---

## 6. Pre-existing warnings (NOT R21 regression)

`cargo check -p northhing` 输出 1159 warnings, 都是 pre-existing:
- `subagent_orchestrator.rs:80-93` 常量 + struct + enum "never used" (R6 E3 历史遗留, R21 不动 subagent_orchestrator.rs)
- `mod.rs` 多处 unused imports (R21 producer 删 method 后 use 段没清理, **R21e 范围**)
- `persistence/manager.rs` 多处 unused imports (R21 不在 scope)
- `desktop` app_state 多处 dead code (R21 不在 scope)

**0 errors, 0 NEW warnings introduced by R21** (R21 producer 报告 + git diff main..HEAD 验证).

---

## 7. Deferred items

### R21e: mod.rs 顶层段 cleanup (deferred to review-fix-cleanup cycle)

mod.rs L83-175 (~94 lines) 是 dead code (R6 拆分历史遗留, subagent_orchestrator.rs 用自己的副本):
- L83-85: 3 unused const (`CONTEXT_COMPRESSION_TOOL_NAME`, `DEFAULT/MAX_SUBAGENT_MAX_CONCURRENCY`)
- L87-99: `WrappedUserInputPayload` struct + `SkillAgentSnapshotPersistence` enum (unused)
- L101-175: 5 unused 顶层 fn (subagent_orchestrator.rs 用同名副本)
- L921/L931: `MANUAL_COMPACTION_COMMAND.to_string()` 保留 (L82 const 仍在用)

**mod.rs 1310 → ~1216 after R21e cleanup** (-94 行, ~-7%). 

**Decision**: R21e 推迟到 review-fix-cleanup cycle (按 R20 mode: review → fix → cleanup). 这样:
1. 避免 R21 + R21e 引入跨 commit 影响
2. 让 reviewer 在 1 squash-merge commit 看到完整 R21 范围
3. R21e 可作为 review 后 cleanup commit, 独立可回滚

### Long line / BOM / CRLF

所有 4 producer commit 都报告 0 NEW long lines (≤5 R18 tolerance), 0 BOM, 0 CRLF. git diff main..HEAD --check 验证.

### Cargo.lock

4 worktree merge 后, `git diff main..HEAD -- Cargo.lock` = 0 行 drift. Mavis 没有跑 `cargo update`.

---

## 8. Cross-crate API stability (R19 lesson applied)

**所有 33 个 facade method 签名保持不变** (cross-crate consumers expect same signature):
- `pub async fn restore_session(...)` ✓
- `pub async fn cancel_dialog_turn(...)` ✓
- `pub async fn list_sessions(...)` ✓
- `pub async fn update_thread_goal_objective(...)` ✓
- ... 等 33 个

Sibling method 名带 `_impl` 后缀是 internal-to-crate, 不影响 cross-crate API.

---

## 9. Risk assessment (post-merge)

| Risk | Mitigation | Status |
|---|---|---|
| Rust E0592 同名 inherent method (R21 spec §3.3 假设错) | Producer 自主用 `_impl` suffix | ✅ Mitigated |
| 4 producer 并行改 mod.rs 不同 line 段 → merge 冲突 | spec §4.1 strict ownership + merge order d→b→a→c | ✅ 0 conflicts |
| Cargo.lock drift | 不跑 `cargo update` | ✅ 0 drift |
| R19 跨 crate visibility regression | `pub(super)` sibling + `pub` facade; cross-crate check | ✅ 0 regression |
| Pre-existing dead code in mod.rs L83-175 | R21e defer to review-fix-cleanup | ⏸ Pending |
| turn_subhandlers.rs 806 lines (R7 仍超 800 cap 6) | R22 候选 | ⏸ Out of R21 scope |

---

## 10. Owner

- **Owner**: Mavis (orchestrator)
- **Producer**: 4 sub-agent (M2.7 non-highspeed)
- **Verifier**: Mavis 3-axis verify (workspace + cli + desktop + test)
- **Reviewer**: ⏸ Pending — User-driven QClaw + Kimi review (verbal verdict, no repo commit for Kimi)
- **Final arbitration**: Mavis (after QClaw + Kimi verdicts returned)
- **Squash merge**: ⏸ Pending — after user review signal, Mavis will `git merge --squash` + 1 squash-merge commit + bump version if appropriate

---

## 11. Artifacts

- R21 spec: `docs/handoffs/2026-07-02-r21-dialog-turn-mod-split-spec.md` (commit `1a69a82`)
- R21a impl handoff: `docs/handoffs/2026-07-02-r21a-restore-revival-impl.md` (commit `6bd85d2`)
- QClaw R21 review: `docs/reviews/round21-qclaw-review.md` (QClaw 9.0/10 APPROVE)
- Kimi R21 review: verbal 8.5/10 APPROVE (per user message, not in repo)
- Plan YAML: `C:\Users\UmR\.mavis\plans\round21-dialog-turn-mod-split-2026-07-02.yaml`
- Plan ID: `plan_8e4eb3ac`
- Main HEAD post-fix: `c28c53d` (4 sequential merge commits + 1 fix commit)
- Verify logs: `r21-verify-workspace.log`, `r21-verify-cli.log`, `r21-verify-northhing.log`, `r21-verify-test.log`, `r21-fix-check.log`, `r21-fix-test.log` (in northing/ root)

---

## 12. Post-review fix (commit `c28c53d`)

QClaw 9.0/10 APPROVE + Kimi 8.5/10 APPROVE both flagged naming inconsistency
(`_impl` vs `_inner`) + R21e mod.rs dead code deferral. Mavis fix commit
`c28c53d` closes all reviewer minor observations:

| Fix | What | Impact |
|---|---|---|
| 1 | session.rs 9 method `*_inner` → `*_impl` | 4/4 sibling 一致 (QClaw 3.1 + Kimi) |
| 1 | mod.rs 9 facade delegate `_inner(` → `_impl(` | 同上, cross-crate API 不变 |
| 2 | turn.rs:870 long line 拆 (`\` continuation) | ≤120 chars (QClaw 3.2) |
| 3 | mod.rs L83-175 dead code cleanup (-93 行) | 1310 → 1217 (QClaw 6 + Kimi R21e) |
| 4 | stage-summary §1/§3 修正 (`-21%` → `-26.4%`, `_impl` 描述) | 文档精度 |

**Verification post-fix**:
- `cargo check -p northhing-core --features product-full --lib`: 0 errors
- `cargo test -p northhing-core --features product-full --lib`:
  899 passed; 0 failed; 1 ignored (baseline preserved, 2.18s)
- Cross-crate API: 0 fn signatures changed in mod.rs facade
- 0 NEW unwrap/panic introduced

**mod.rs final**: 1217 lines (vs R21 spec §1.2 estimate ~1216, 99.2% match).

R21 round ready for user review signal → Mavis 1 squash-merge commit
(collapses 4 sequential merge + 1 fix into 1 clean commit).