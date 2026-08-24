# R4 Spec: session_restore real wire (aggressive split)

> **Type**: refactor spec（接续 Round 3b）
> **Trigger**: R4 综合 cleanup plan `2026-06-27-r4-comprehensive-cleanup-plan.md` §1 P0-1 — session_manager.rs 实质拆分
> **Predecessor**: Round 3b split plan（`2026-06-26-round3b-session-manager-split-plan.md`） + visibility audit（`2026-06-26-round3b-session-manager-visibility-audit.md`）
> **Status**: spec 阶段，等待 review
> **Precondition**: Task A（orphan-fix: mod.rs 加 `pub mod session_restore;` + `pub use`）必须**先**执行；本 spec 描述在 orphan-fix 之后还需要做什么。
> **Sibling tasks**（同 plan plan_c5a5067a 并行）:
> - Task A — `spec-session-orphan-fix`：mod.rs wiring + duplicate delete（**前置**）
> - Task C — `spec-session-persistence-real-wire`：session_persistence.rs 的 2 个 helpers（`sanitize_listing_diff_context_snapshot_if_needed` / `persist_context_snapshot_messages_best_effort`）是否落在 session_persistence.rs 还是 session_restore.rs，**必须在 review 阶段跟本 spec 协调**（见 Errata E1）
> - Task E — `spec-session-evidence-real-wire`：session_evidence.rs 的 listing_baseline helpers（与本 spec 跨调用）

---

## 0. 现状校正

| 项 | Round 3b plan 假设 | 实际测量（2026-06-27 wc -l） | delta |
|---|---|---|---|
| session_manager.rs 行数 | 6532 | 6532 | ✅ 一致 |
| session_manager.rs 拆分后目标 | ~2400（非测试代码）| 当前 6532（未 trim）| ❌ Round 3b 提交未瘦身 |
| session_restore.rs 行数 | ~700 | 757 | ✅ 接近 |
| session_restore.rs 中 fn 数 | 18 | 16 主 fn + 缺 2 helpers（`sanitize_listing_diff_context_snapshot_if_needed` / `persist_context_snapshot_messages_best_effort`）| ⚠️ 不一致 |
| mod.rs 是否声明 session_restore | - | ❌ 未声明（line 5-12 只有 session_manager + session_store_port 等 7 个 module）| orphan |
| session_manager.rs 是否保留重复 fn | - | ✅ 16 主 fn 全重复（line 1670-2922） + 2 helpers 仍存在 | parallel-copy 陷阱 |

**关键事实**：Round 3b 提交 5250199 是 **parallel-copy trap**（详见 MEMORY 2026-06-27 entry）— session_restore.rs 文件存在 + commit message 自称"split 6532-line god object"，但 mod.rs 没有 `pub mod session_restore;`，所以：
1. session_restore.rs 完全未被 cargo 编译
2. cargo check / cargo test 全部通过是因为没编译这个文件
3. session_manager.rs 行数从 6532 → 6532（**未减一行**）

**本 spec 的真实任务**：在 Task A（orphan-fix）补上 mod.rs 声明之后，把 session_manager.rs 里 16 个重复 fn + 2 个 restore-only helpers 全部删掉（让 session_restore.rs 成为**唯一**实现），并且把 8 个 restore 相关的 `#[tokio::test]` 跟方法一起搬到 session_restore.rs 的 mod tests 里（达到用户 ~1500 行的验收目标）。

---

## 1. Restore fns identified（要搬走的）

> **注**：行号基于 2026-06-27 当前 HEAD `9dbcb9c` 上 session_manager.rs 的实测位置（详见 workspace/restore_multi_line_sig.txt 扫描报告）。

### 1.1 主 restore API（16 个 fn — 已在 session_restore.rs 完整复制，需从 session_manager.rs 删除）

| fn | session_manager.rs 位置 | session_restore.rs 位置 | body 行数（粗算） | 公共 API— | 跨调用 self.method |
|---|---|---|---|---|---|
| `restore_session` | 2356-2364 | 67-74 | ~8 | pub(crate) | `restore_session_internal` (private) |
| `restore_internal_session` | 2365-2373 | 76-83 | ~7 | pub(crate) | `restore_session_internal` |
| `restore_session_internal` | 2374-2388 | 85-95 | ~10 | private | `restore_session_with_turns_internal` |
| `restore_session_view` | 2389-2398 | 99-107 | ~8 | pub(crate) | `restore_session_view_timed` |
| `restore_session_view_timed` | 2399-2408 | 109-117 | ~8 | pub(crate) | `restore_session_view_internal` |
| `restore_internal_session_view` | 2409-2418 | 119-127 | ~8 | pub(crate) | `restore_internal_session_view_timed` |
| `restore_internal_session_view_timed` | 2419-2428 | 129-137 | ~8 | pub(crate) | `restore_session_view_internal` |
| `restore_session_view_tail` | 2429-2439 | 139-148 | ~9 | pub(crate) | `restore_session_view_tail_timed` |
| `restore_session_view_tail_timed` | 2440-2454 | 150-163 | ~13 | pub(crate) | `restore_session_view_internal` |
| `restore_internal_session_view_tail` | 2455-2465 | 165-174 | ~9 | pub(crate) | `restore_internal_session_view_tail_timed` |
| `restore_internal_session_view_tail_timed` | 2466-2480 | 176-189 | ~13 | pub(crate) | `restore_session_view_internal` |
| `restore_session_view_internal` | 2481-2615 | 191-323 | ~132 | private | 跨 8 个 `restore_*_view*` 调用 |
| `restore_session_with_turns` | 2616-2624 | 326-333 | ~7 | pub(crate) | `restore_session_with_turns_internal` |
| `restore_internal_session_with_turns` | 2625-2633 | 335-342 | ~7 | pub(crate) | `restore_session_with_turns_internal` |
| `restore_session_with_turns_internal` | 2634-2921 | 344-629 | ~285 | private | 调 sanitize/listing_baseline/build_messages/derive_last_user_dialog_agent_type/effective_workspace_path_from_config 等 |
| `rollback_context_to_turn_start` | 2922-3048 | 632-756 | ~124 | pub(crate) | 调 restore_session / sanitize_listing_diff / truncate_listing_baseline / derive_last_user_dialog_agent_type |

**16 主 fn body 总和**：~662 行（session_restore.rs 当前 line 67-756 = 689 行 impl 段，扣掉空行/注释约 660 行 body）。

### 1.2 Restore-only helpers（2 个 fn — 当前 session_manager.rs 独有，需搬到 session_restore.rs）

| fn | session_manager.rs 位置 | body 行数 | 公共 API— | 跨调用 |
|---|---|---|---|---|
| `sanitize_listing_diff_context_snapshot_if_needed` | 1694-1731 | ~37 | private | `restore_session_with_turns_internal` (line 495) + `rollback_context_to_turn_start` (line 671 in restore.rs / line 2961 in mgr.rs) |
| `persist_context_snapshot_messages_best_effort` | 1670-1692 | ~22 | private | `sanitize_listing_diff_context_snapshot_if_needed` (line 1722) |

**注意**：这 2 个 helper 在 session_persistence.rs 里也有平行副本（line 443, 467）作为 `pub(crate)`，但是 orphan（详见 Errata E1）。Round 3b plan §5.2 跨调用表把它们标为 session_persistence.rs 归属，但 §2.4 又标 session_restore.rs 归属 — **plan 自相矛盾**，由 Task C（persistence spec）和本 spec 的 reviewer 在 review 阶段协调决定。

**本 spec 的默认决定**（待 reviewer 确认）：

- `sanitize_listing_diff_context_snapshot_if_needed` → **session_restore.rs**（理由：仅被 restore/rollback 调用，属于 restore-domain）
- `persist_context_snapshot_messages_best_effort` → **session_restore.rs**（理由：仅被 sanitize_listing_diff 调用，间接 restore-only）
- session_persistence.rs 里的平行副本（line 443, 467）由 Task C 在 persistence spec 里删掉
- visibility: 这 2 个 helper 在 session_restore.rs 里保持 `private`（impl SessionManager 跨文件共享 self，无需 pub）

### 1.3 依赖（哪些其他文件 / struct / helper 必须 pub(super) 或 pub(crate) 才能让本 spec 工作）

按 Round 3b visibility audit §3 整理 + 本 spec 增量：

#### 1.3.1 SessionManager struct 字段（10 个，全部需 `pub(crate)`）

session_restore.rs 通过 `self.<field>` 访问这些字段，跨文件必须有 `pub(crate)`：

| 字段 | 类型 | session_restore.rs 访问位置 |
|---|---|---|
| `sessions` | `Arc<DashMap<String, Session>>` | line 352, 623, 708 |
| `session_workspace_index` | `Arc<DashMap<String, PathBuf>>` | line 625 |
| `context_store` | `Arc<SessionContextStore>` | line 540, 551, 683 |
| `prompt_cache_store` | `Arc<SessionPromptCacheStore>` | line 541 |
| `turn_skill_agent_snapshot_store` | `Arc<TurnSkillAgentSnapshotStore>` | line 542 |
| `skill_agent_baseline_override_snapshot_store` | `Arc<DashMap<...>>` | line 544 |
| `file_read_state_store` | `Arc<FileReadStateStore>` | line 546 |
| `config` | `SessionManagerConfig` | line 639, 646, 718 |
| `persistence_manager` | `Arc<PersistenceManager>` | line 230, 249, 354, 374, 397, 489, 540 等 15+ 处 |

`evidence_ledger` 不在 session_restore.rs 用，留给 Task E 处理。

#### 1.3.2 Self 静态方法调用（跨 split file 需 `pub(crate)`）

session_restore.rs 调用的 `Self::xxx` 静态方法：

| Self::xxx | session_restore.rs 调用位置 | 原行号（在 session_manager.rs） | 需 visibility |
|---|---|---|---|
| `effective_workspace_path_from_config` | 361 | 414 | pub(crate) |
| `listing_baseline_rebuild_turn_index_from_metadata` | 386, 651 | 1662 | pub(crate) |
| `load_ai_config_for_model_resolution` | 406 | 144 | pub(crate) |
| `is_session_model_id_usable` | 418 | 912 | pub(crate) |
| `sync_session_context_window_from_ai_config` | 448 | 201 | pub(crate) |
| `build_messages_from_turns` | 505, 524 | 517 | pub(crate) |
| `derive_last_user_dialog_agent_type_from_turns` | 482, 700 | 2080 | pub(crate) |
| `should_persist_session` | 718 | 307 | pub(crate) |
| `should_persist_session_id` | 616 | 404 | 已是 pub |

注：`strip_listing_diff_internal_reminders` (line 1713 在 sanitize 里) 是 Self:: 调用，但通过 session_restore.rs 的 sanitize helper → 属于 session_evidence.rs 域，Task E 处理。

#### 1.3.3 self.method 调用（跨 split file 需 `pub(crate)` 或保留可见性）

session_restore.rs 调用的 `self.method` 实例方法：

| self.method | session_restore.rs 调用位置 | 原归属 | 需 visibility |
|---|---|---|---|
| `restore_session_internal` | 73, 81（被 `restore_session` / `restore_internal_session` 跨调用）| session_restore.rs | 改 pub(crate) |
| `restore_session_with_turns_internal` | 331, 340（被 `restore_session_with_turns` / `restore_internal_session_with_turns` 跨调用）| session_restore.rs | 改 pub(crate) |
| `restore_session_view_internal` | 113, 116, 134, 136, 161, 187（被 8 个 view helper 跨调用）| session_restore.rs | 改 pub(crate) |
| `sanitize_listing_diff_context_snapshot_if_needed` | 495, 671（被 `restore_session_with_turns_internal` + `rollback_context_to_turn_start` 调）| session_restore.rs（本 spec 决定）| 保持 private（同 impl 块） |
| `persist_context_snapshot_messages_best_effort` | 1722（被 sanitize 调）| session_restore.rs | 保持 private |
| `truncate_listing_baseline_rebuild_turn_index_after_rollback` | 745（被 rollback 调）| **session_evidence.rs**（Round 3b plan §2.2）| 改 pub(crate) — Task E 落地 |
| `merge_session_custom_metadata` | 1739（在 evidence 域的 listing_baseline_persist 里）| session_manager.rs | 已是 pub |

注：`self.sanitize_listing_diff_context_snapshot_if_needed` 同文件（同 impl SessionManager 块），Rust 允许跨文件保留 private — 不需要改 visibility。

#### 1.3.4 const（需 `pub(super)` 让 session_restore.rs 可见）

| const | 原行号 | 需 visibility |
|---|---|---|
| `LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY` | 101 | pub(super)（按 Round 3b plan §3.1） |

#### 1.3.5 外部 type（SessionViewRestoreTiming 等）

| item | 当前归属 | 是否需要变动 |
|---|---|---|
| `SessionViewRestoreTiming` | `northhing_runtime_ports` re-export by session_manager.rs (line 36) **且** session_restore.rs (line 47) | ⚠️ 必须保留 session_manager.rs 里的 re-export — dialog_turn.rs:3498-3576 引用 `crate::agentic::session::session_manager::SessionViewRestoreTiming` 旧路径，**不能移到 session_restore.rs**（详见 visibility audit §2.4） |

#### 1.3.6 外部依赖（session_restore.rs 的 use 块需保留）

session_restore.rs 现状（line 16-63）已有完整 use 块，覆盖：
- `crate::agentic::core::*`（Session / SessionConfig / SessionState / SessionSummary 等）
- `crate::agentic::image_analysis::ImageContextData`（可能不必要 — 检查 line 191+ 实际是否用）
- `crate::agentic::persistence::PersistenceManager`
- `crate::agentic::session::session_store_port::CoreSessionStorePort`
- `crate::agentic::session::*`（大量 re-export）
- `crate::infrastructure::ai::get_global_ai_client_factory`（**可能不需要** — 仅在 reconcile path 用）
- `crate::service::config::*`（**可能不需要**）
- `crate::service::session::*`（DialogTurnData / ModelRoundData 等 — restore 必用）
- `crate::service::snapshot::ensure_snapshot_manager_for_workspace`（**可能不需要**）
- `crate::util::errors::*` / `crate::util::timing::elapsed_ms_u64`
- `northhing_runtime_ports::*`（SessionStoragePathRequest / SessionStorePort / SessionViewRestoreRequest）
- `northhing_services_core::session::*`（apply_session_lineage / collect_hidden_subagent_cascade 等 — **可能不需要**）

**Action**: spec 应用阶段做一次 `grep -E '\b(ImageContextData|get_global_ai_client_factory|get_app_language_code|get_global_config_service|short_model_user_language_instruction|subscribe_config_updates|ConfigUpdateEvent|ensure_snapshot_manager_for_workspace|apply_session_lineage|collect_hidden_subagent_cascade_ids|merge_session_custom_metadata_value|set_deep_review_run_manifest|set_session_relationship)\b' src/agentic/session/session_restore.rs` 实际检查 — 砍掉用不到的 import。

### 1.4 Restore tests（8 个 `#[tokio::test]` — 跟方法一起搬到 session_restore.rs）

| test fn | session_manager.rs 起 | 估 body 行数（到下一个 `#[tokio::test]`） | 测试关注点 |
|---|---|---|---|
| `restore_session_resets_processing_state_without_marking_unread_completion` | 4754 | ~272 | restore 重置 state 而不写 unread completion |
| `core_session_store_port_resolves_unresolved_remote_storage_path` | 5026 | ~28 | CoreSessionStorePort 解析 unresolved remote 路径 |
| `restore_session_view_loads_turns_without_restoring_runtime_context` | 5054 | ~141 | view restore 不写 runtime context |
| `restore_session_view_preserves_full_visible_tool_result_payload` | 5195 | ~125 | view restore 保留完整 tool result payload |
| `rollback_context_deletes_persisted_turns_from_target` | 5320 | ~326 | rollback 删 target 起所有 persisted turns |
| `restore_session_sanitizes_pre_cutoff_listing_diff_snapshot` | 5646 | ~104 | restore 在 cutoff 前清洗 listing diff snapshot |
| `rollback_sanitizes_pre_cutoff_snapshot_and_truncates_cutoff` | 5750 | ~128 | rollback 同样行为 + truncate cutoff 索引 |
| `rollback_to_empty_history_clears_last_user_dialog_agent_type` | 5878 | ~55 | rollback 到 0 历史清空 last_user_dialog_agent_type |

**8 个 test body 总和**：~1179 行（含 import / use / 空行 / 注释）。

**测试 helpers 共享问题**（见 Errata E2）：这些测试用到的 `TestWorkspace::new()`, `PersistenceManager::new()`, `test_manager(...)`, `in_memory_test_manager(...)`, `test_model()` 都是 session_manager.rs mod tests 的私有 helper。搬到 session_restore.rs 后需要：
- 选项 A：在 session_restore.rs mod tests 里复制一份 helper（~120 行）
- 选项 B：把 helper 提到 `agentic/session/mod.rs::tests` 或独立 `tests/test_helpers.rs`
- 选项 C：保留 restore tests 在 session_manager.rs（违反 ~1500 行验收）

**本 spec 默认决定**（待 reviewer 确认）：选项 A（最低侵入，单次 commit 即可），接受 ~120 行 helper 重复。

---

## 2. Migration steps

> **前置条件**：Task A（spec-session-orphan-fix）已落地 — mod.rs 加了 `pub mod session_restore;` + `pub use session_restore::*;`。
>
> **建议执行顺序**：本 spec 全部步骤在 **单次 commit** 完成（"refactor(session): real-wire session_restore.rs"），避免中间状态编译失败。

### Step 1 — visibility 改动（session_manager.rs）

1. SessionManager struct 的 10 个字段全部改 `pub(crate)`（覆盖本 spec §1.3.1 全部 + evidence/persistence 兄弟模块需要 — Task C/E 也受益）
2. 9 个 Self 静态方法改 `pub(crate)`（见 §1.3.2）
3. 4 个 self 内部方法改 `pub(crate)`（§1.3.3 — 包括 `restore_session_internal` / `restore_session_with_turns_internal` / `restore_session_view_internal` 改 pub(crate)，`sanitize_listing_diff_context_snapshot_if_needed` 保持 private 即可因为会被搬到同模块）
4. `LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY` const 改 `pub(super)`

**保留**（不动）：
- `SessionViewRestoreTiming` re-export 在 session_manager.rs（line 36，dialog_turn.rs 旧路径依赖）
- 全部 7 个 pub API（`should_persist_session_id`, `effective_session_workspace_path`, `update_persisted_session_metadata` 等）保持 `pub`
- `restore_session_*` 公共方法保持 `pub(crate)`（Round 3b plan 已正确标）

### Step 2 — 删除 session_manager.rs 里的 18 个重复 fn

按 §1.1 + §1.2 列表删除：
- line 1670-1731：删 `persist_context_snapshot_messages_best_effort` + `sanitize_listing_diff_context_snapshot_if_needed`（搬到 session_restore.rs）
- line 2355-3048：删 16 个 `restore_*` + `rollback_context_to_turn_start`（保留在 session_restore.rs 的副本）
- line 3049 之后保持不动（list_sessions / metadata / dialog turn lifecycle 等）

### Step 3 — 补全 session_restore.rs 的 2 个 helpers

- 把 `persist_context_snapshot_messages_best_effort` (22 行 body) 加到 session_restore.rs impl SessionManager 块的合适位置（建议放在 use 块之后、`impl` 之前 helper 区，跟其他 internal helpers 一起）
- 把 `sanitize_listing_diff_context_snapshot_if_needed` (37 行 body) 紧接着加进去
- 调整 visibility：本文件内 private 即可（impl SessionManager 跨文件 OK）
- 调整 use 块：sanitize 调用 `Self::strip_listing_diff_internal_reminders` — 那个 fn 在 session_evidence.rs，要 `pub(crate)`（**Task E 落地**）

### Step 4 — 搬家 8 个 restore tests（session_manager.rs → session_restore.rs）

1. 在 session_restore.rs 文件末尾加 `#[cfg(test)] mod tests { ... }` 块
2. 把 session_manager.rs mod tests 里的 8 个测试函数（§1.4 列表）整段剪贴到 session_restore.rs mod tests
3. session_restore.rs mod tests 顶部复制 helpers（选项 A）：
 - `TestWorkspace` struct（`#[allow(dead_code)]` 标记可保留）
 - `test_manager` / `test_manager_with_config` / `in_memory_test_manager` / `test_model` 函数
 - 必要的 `use` 导入（TestWorkspace / PathManager / PersistenceManager / Uuid / SessionManagerConfig 等）
4. 从 session_manager.rs mod tests 删掉这 8 个 fn（不留空行 / 不留注释）
5. **保留** mod tests 顶部的 `use super::*;` + `use crate::agentic::session::*;` 导入（restore 测试通过 super 引用 SessionManager / SessionManagerConfig）

### Step 5 — 验证

```bash
cd E:/agent-project/northing
cargo check --workspace
cargo test -p northhing-core --lib session::session_restore
cargo test -p northhing-core --lib session::session_manager # 确认其他 27 个测试还过
cargo test --workspace # 全 workspace 935+ 测试
cargo fmt --check
```

**预期结果**：
- `cargo check --workspace` 0 errors
- `cargo test` 935+ 测试全过（与 baseline 一致 — restore 测试数从 session_manager 移到 session_restore 但总数不变）
- `cargo fmt --check` 干净（按 AGENTS.md 习惯用 `pnpm run fmt:rs`）

### Step 6 — 二次验证（避免 Round 3b 的 parallel-copy trap 重演）

| 检查项 | 命令 | 期望 |
|---|---|---|
| session_restore.rs 真在 build 里 | `git grep -l 'mod session_restore' src/crates/assembly/core/src/agentic/session/mod.rs` | 命中 |
| session_manager.rs 不再有 restore fn | `git grep -nE 'fn (restore_session\|rollback_context)' src/crates/assembly/core/src/agentic/session/session_manager.rs` | 0 命中 |
| session_manager.rs 不再有 sanitize/persist helpers | `git grep -nE 'fn (sanitize_listing_diff_context_snapshot\|persist_context_snapshot_messages_best_effort)' src/crates/assembly/core/src/agentic/session/session_manager.rs` | 0 命中 |
| session_restore.rs 包含全部目标 fn | `git grep -cE 'fn (sanitize_listing_diff_context_snapshot\|persist_context_snapshot_messages_best_effort)' src/crates/assembly/core/src/agentic/session/session_restore.rs` | ≥ 2 |
| session_restore.rs 包含 8 个测试 | `git grep -cE 'fn (restore_session_resets_processing_state\|core_session_store_port_resolves_unresolved\|restore_session_view_loads_turns\|restore_session_view_preserves_full_visible\|rollback_context_deletes_persisted\|restore_session_sanitizes_pre_cutoff\|rollback_sanitizes_pre_cutoff\|rollback_to_empty_history)' src/crates/assembly/core/src/agentic/session/session_restore.rs` | 8 |
| 行数验收 | `wc -l src/crates/assembly/core/src/agentic/session/session_restore.rs` | ~1500 行（详见 §3）|

---

## 3. Acceptance

| 项 | 当前值 | 目标值 | 验证 |
|---|---|---|---|
| session_restore.rs 行数 | 757 | **~1500**（±10%）| `wc -l` |
| session_manager.rs 行数 | 6532 | **< 6000**（降 ~530 行 = 16 fn body 662 行 - 测试 ~130 行 + 2 helper 60 行 — 实际降 ≈ 530）| `wc -l` |
| session_restore.rs 中 `impl SessionManager` fn 数 | 16 | **18**（加 2 helpers）| grep `fn (sanitize\|persist_context_snapshot_messages_best_effort)` |
| session_manager.rs 中 `impl SessionManager` restore fn 数 | 18 | **0**（全部移到 session_restore.rs）| grep `fn (restore_\|rollback_context\|sanitize_listing_diff_context\|persist_context_snapshot_messages_best_effort)` |
| mod tests 在 session_restore.rs | 0 fn | **8** 个 restore test | grep `async fn (restore\|rollback\|core_session_store_port_resolves)` |
| 全 workspace cargo test | 935+ 测试通过 | **935+ 测试通过**（不变）| `cargo test --workspace` |
| 全 workspace cargo check | 0 errors | **0 errors** | `cargo check --workspace` |
| cargo fmt | 干净 | **干净** | `cargo fmt --check` |

**session_restore.rs ~1500 行核算**：
- 当前 757 行（use 块 + 16 主 fn body + impl 头）
- 加 `persist_context_snapshot_messages_best_effort` body 22 行
- 加 `sanitize_listing_diff_context_snapshot_if_needed` body 37 行
- 加 `#[cfg(test)] mod tests { ... }` 壳 + helpers 复制 ~120 行 + 8 test body ~1179 行
- 总计：757 + 59 + 120 + 1179 ≈ **2115 行**

**与 ~1500 目标偏差 ~600 行**：
- 如果选项 A（helpers 复制 + 8 测试搬）：~2115 行
- 如果选项 C（保留测试在 session_manager.rs）：757 + 59 = **816 行**（远低于目标）
- 如果选项 B（共享 helpers 文件）：~2115 - 50（去重）= ~2065 行

**建议**：与 reviewer 确认是 ~1500（松）还是 ~2000（实测），调整测试分配。**本 spec 倾向选项 A**（最低侵入），让 reviewer 决定是 ~2000 还是要求进一步压缩。

---

## 4. Risks

| # | 风险 | 影响 | 缓解 |
|---|---|---|---|
| R1 | **Task A（orphan-fix）跟本 spec 顺序— 突**：Task A 加 mod.rs 声明后立刻编译失败（duplicate fn 错误） | cargo check 短暂红 | 1) Task A 的 commit 里**预先**包含本 spec 的删除动作（即 Task A 不仅是 orphan-fix 还顺手把重复 fn 删了），或者 2) 本 spec 在 Task A commit 之后**立即**接力 commit。**推荐 (1)** — 单次 atomic commit，避免中间红 |
| R2 | **Task C（persistence）和本 spec 抢 `sanitize_listing_diff_context_snapshot_if_needed` / `persist_context_snapshot_messages_best_effort`** | 重复定义 / 方法找不到 | 在 review 阶段把 Task C / 本 spec / Task E 三个 spec 的 §"归属"段对齐。**本 spec 默认归属 session_restore.rs**（理由：仅 restore 调用）。Task C 必须在 review 时确认接受 |
| R3 | **测试 helper 复制导致 drift**：session_manager.rs 和 session_restore.rs 各有一份 `TestWorkspace` / `test_manager`，改一个忘改另一个 | CI 偶发失败 | 选项 A 接受 risk；选项 B（共享 helper 文件）消除 risk 但需要新文件 |
| R4 | **mod.rs pub use 顺序错位**：`pub use session_restore::*` 必须放在合适位置，按 Round 3b plan §6.10 字母序在 `session_manager` 之后 | 编译失败 | mod.rs 严格按字母序：compression → context_store → evidence_ledger → file_read_state → prompt_cache → session_evidence → session_manager → session_persistence → session_restore → session_store_port → turn_skill_agent_snapshot_store |
| R5 | **`Self::strip_listing_diff_internal_reminders` 跨文件不可见**：session_restore.rs 的 sanitize helper 调 `Self::strip_listing_diff_internal_reminders`，那个 fn 在 session_evidence.rs | 编译失败 | session_evidence.rs 把 `strip_listing_diff_internal_reminders` 改 `pub(crate)` — **Task E 落地**，但 Task E 可能在本 spec 之后才执行 → 必须在 review 阶段协调 |
| R6 | **`SessionViewRestoreTiming` re-export 路径不稳**：dialog_turn.rs:3498-3576 用 `crate::agentic::session::session_manager::SessionViewRestoreTiming` 旧路径 | 如果误删 session_manager.rs 的 re-export → 编译失败 | **不删** — session_manager.rs:36 的 `pub use northhing_runtime_ports::SessionViewRestoreTiming;` 必须保留。session_restore.rs:47 的 re-export 是**额外**副本（无害） |
| R7 | **`session_workspace_path` dead-code fn 漏改 visibility**：line 422 `#[allow(dead_code)] fn session_workspace_path` 跨 split file 可能无访问者 | cargo warning | 保持 private + `#[allow(dead_code)]` — 不动 |
| R8 | **session_manager.rs 拆分后剩余 ~6000 行仍在 audit 红线附近**（不是 ~2500 目标）| 单文件仍偏大 | 本 spec 解决 restore 拆分；剩余靠 Task C（persistence）/ Task E（evidence）/ Task B（lifecycle）协同，把 session_manager.rs 压到 < 2500。**单 spec 不可越界包揽** |
| R9 | **Restore tests 跨文件 `use super::*` 不可见**：session_restore.rs 的 mod tests 用 `super::SessionManager` — 但如果 super 解析到 session_restore.rs 的 SessionManager impl 而非 session_manager.rs 的 struct 定义 | 编译失败 | Rust 允许跨文件 `impl SessionManager` — super 解析到 mod.rs → crate::agentic::session::SessionManager struct（定义在 session_manager.rs）；mod tests 看到的 super 是 session_restore.rs 自己，但 `impl SessionManager` 不影响类型定义来源。**已在 Round 3a coordinator 拆分验证过** |
| R10 | **impl SessionManager 跨 4 个文件后 `Self::xxx` 解析**：session_restore.rs 写 `Self::effective_workspace_path_from_config`，impl 块在 session_manager.rs 也有，Rust 应当能找到（impl 跨文件 OK） | cargo 编译失败 | Round 3a 已验证；本 spec 应当无问题。如果出问题，最坏回退：把所有跨文件 `Self::` 调用改成显式 `Self::` in `crate::agentic::session::SessionManager::xxx` |

---

## 5. Errata

### E1 — `sanitize_listing_diff_context_snapshot_if_needed` / `persist_context_snapshot_messages_best_effort` 归属

**Round 3b plan 自相矛盾**：
- §2.4 把它们列在 session_restore.rs
- §5.2 跨调用表又把 `sanitize_listing_diff_context_snapshot_if_needed` 标为 session_persistence.rs 归属

**当前实际**：
- session_manager.rs:1670 / 1694（production）
- session_persistence.rs:443 / 467（orphan parallel copy, `pub(crate)`）

**本 spec 默认决定**：session_restore.rs（理由：仅被 restore/rollback 调用，restore-domain）。

**待 reviewer 协调**：
- Task C reviewer 是否接受本决定？如不接受，需把 2 helpers 留在 session_persistence.rs，session_restore.rs 通过 `self.sanitize_xxx` 跨 impl 块调用（仍 OK，因为 impl SessionManager 跨文件）
- Task E reviewer 是否能把 `strip_listing_diff_internal_reminders` 改 `pub(crate)`？这是 sanitize 的唯一 Self:: 跨调用

### E2 — 测试 helpers 共享策略

三个选项（详见 §1.4）：
- A：复制 helpers 到 session_restore.rs（最低侵入，~120 行重复）
- B：提取 helpers 到独立文件 `session_test_helpers.rs`（去重但新增 1 文件）
- C：保留 restore tests 在 session_manager.rs（违反 ~1500 行验收）

**本 spec 默认决定**：选项 A。

**待 reviewer 决定**：~1500 行验收是否可放宽到 ~2000 行（接受 A 的实际 2115 行）？或者要求 B（共享 helpers）以消除 drift risk？

### E3 — `mod tests` 是否搬到 session_restore.rs

**Round 3a 样板**：coordinator 拆分时把 mod tests 留在主文件（coordinator.rs），不在 split file 重复。

**本 spec 偏离**：为了达到 ~1500 行验收，把 8 个 restore tests 搬到 session_restore.rs。

**待 reviewer 决定**：保留 Round 3a 样板（不搬测试，接受 session_restore.rs ~816 行）vs 偏离样板（搬测试，达到 ~2000 行）？

### E4 — `Self::strip_listing_diff_internal_reminders` visibility 改动时机

session_restore.rs 的 sanitize helper 调 `Self::strip_listing_diff_internal_reminders`（line 1637 in session_manager.rs，未来在 session_evidence.rs）。

**Visibility 要求**：`pub(crate)`（跨文件 Self:: 调用）。

**问题**：Task E（session_evidence.rs real wire）会处理这个改动，但 Task E 可能跟本 spec 并行。

**缓解**：review 阶段确认 Task E spec 是否已包含 `strip_listing_diff_internal_reminders` → `pub(crate)`。如果 Task E 不包含，本 spec 需要在 §1.3.3 末尾追加一个 Step 0："在 session_manager.rs line 1637 把 `strip_listing_diff_internal_reminders` 改 `pub(crate)`，作为跨 spec 必要前提"。

### E5 — image_analysis / config / snapshot / services_core 的 import 是否真的不需要

session_restore.rs 现状 use 块（line 21-55）从 Round 3b commit 继承，可能包含用不到的 import。

**Action**：spec 应用阶段做精确 grep 验证：
- `ImageContextData`：grep `ImageContextData` 在 session_restore.rs → 0 命中 → 删
- `get_global_ai_client_factory`：grep → 0 命中 → 删
- `get_app_language_code` / `get_global_config_service` / `short_model_user_language_instruction` / `subscribe_config_updates` / `ConfigUpdateEvent`：grep → 0 命中 → 删
- `ensure_snapshot_manager_for_workspace`：grep → 0 命中 → 删
- `apply_session_lineage` / `collect_hidden_subagent_cascade_ids` / `merge_session_custom_metadata_value` / `set_deep_review_run_manifest` / `set_session_relationship`：grep → 0 命中 → 删

**预计可砍 ~10 行 use block**（不影响行为）。

### E6 — Task A 的 commit 范围

Task A（orphan-fix）的 spec 描述是 "mod.rs wiring + duplicate delete"。**"duplicate delete" 已经涵盖本 spec 的 Step 1-2** — Task A 应该已经把 session_manager.rs 里的重复 fn 删掉（否则 mod.rs 加上 `pub mod session_restore;` 后立刻编译失败）。

**建议**：在 review 阶段确认 Task A 的 spec 范围。如果 Task A 已经删了重复 fn，本 spec 的 §2 Step 1-2 是冗余（commit 已在 Task A 里完成）；如果 Task A 没删（只加 mod.rs），本 spec 的 Step 1-2 是必要补救。

**最干净的 git history**：单次 commit "refactor(session): real-wire session_restore.rs" 完成全部 §2 Steps。Task A 改为单独 commit "refactor(session): wire session/sibling modules into mod.rs" 只加 3 行 `pub mod` + 3 行 `pub use`。两 commit 顺序：A 先（编译红但立即 commit），D 接力（编译绿）。

### E7 — Task C / E 落地时间窗

如果本 spec 在 Task C 之前落地：
- `sanitize_listing_diff_context_snapshot_if_needed` 在 session_restore.rs（按本 spec）
- 同时 session_persistence.rs 还有平行副本（orphan-fix 后变成 live copy）
- → 重复定义，编译失败

**缓解**：
- 选项 1：本 spec 等待 Task C 完成后再 apply
- 选项 2：本 spec 跟 Task C 在同一 commit（合 spec 一起落地）
- 选项 3：Task C 在应用阶段顺手删 session_persistence.rs 里的 2 个 helper

**推荐 选项 3** — Task C spec 加一句"删除 session_persistence.rs:443 / 467 的 `persist_context_snapshot_messages_best_effort` / `sanitize_listing_diff_context_snapshot_if_needed`（归属 session_restore.rs）"。

### E8 — 不在范围

- ❌ dialog turn lifecycle 拆分 → Task B
- ❌ listing baseline rebuild 拆分 → Task E
- ❌ evidence ledger / skill agent snapshot 拆分 → Task E
- ❌ prompt cache persistence 拆分 → Task C
- ❌ session_manager.rs 顶层 review_platform / dialog_turn 二次拆分 → Round 5+
- ❌ `Self::should_persist_session_kind` 等细粒度 helper visibility 改动 → 跟随 SessionManagerConfig struct 改 pub 即可

---

## 6. 验证清单

| 检查 | 命令 | 通过条件 |
|---|---|---|
| mod.rs 声明 session_restore | `git grep -nE 'mod session_restore' src/crates/assembly/core/src/agentic/session/mod.rs` | 命中 |
| session_restore.rs 在 build 里 | `cargo check --workspace 2>&1 | tee /tmp/build.log; grep 'session_restore' /tmp/build.log` | 至少 1 处引用 |
| session_manager.rs 不再有重复 fn | `git grep -nE 'fn (restore_session\|rollback_context\|sanitize_listing_diff_context_snapshot\|persist_context_snapshot_messages_best_effort)' src/crates/assembly/core/src/agentic/session/session_manager.rs` | 0 命中 |
| session_restore.rs 行数 ~1500 | `wc -l src/crates/assembly/core/src/agentic/session/session_restore.rs` | 1500 ± 10% |
| session_manager.rs 行数减少 | `wc -l src/crates/assembly/core/src/agentic/session/session_manager.rs` | < 6000 |
| cargo check | `cargo check --workspace` | 0 errors |
| cargo test | `cargo test --workspace` | 935+ 测试通过 |
| cargo fmt | `cargo fmt --check` | 干净 |
| Restore 测试在 session_restore.rs | `git grep -cE 'async fn (restore_session_resets_processing_state\|core_session_store_port_resolves\|restore_session_view_loads\|restore_session_view_preserves\|rollback_context_deletes\|restore_session_sanitizes_pre_cutoff\|rollback_sanitizes_pre_cutoff\|rollback_to_empty_history)' src/crates/assembly/core/src/agentic/session/session_restore.rs` | ≥ 8 |
| Restore 测试不在 session_manager.rs | `git grep -cE 'async fn (restore_session_resets_processing_state\|core_session_store_port_resolves\|restore_session_view_loads\|restore_session_view_preserves\|rollback_context_deletes\|restore_session_sanitizes_pre_cutoff\|rollback_sanitizes_pre_cutoff\|rollback_to_empty_history)' src/crates/assembly/core/src/agentic/session/session_manager.rs` | 0 |
| SessionViewRestoreTiming 旧路径保留 | `git grep -nE 'pub use.*SessionViewRestoreTiming' src/crates/assembly/core/src/agentic/session/session_manager.rs` | 命中 line 36 |

---

## 7. 总结

**本 spec 范围**：在 Task A（orphan-fix）落地后，把 session_manager.rs 里 18 个 restore-domain fn（16 主 fn + 2 helpers）全部删掉（已经在 session_restore.rs 有副本），并把 8 个 restore tests 一起搬到 session_restore.rs 的 mod tests。

**预期结果**：
- session_restore.rs: 757 → ~2115 行（含 helpers + tests + use 调整）
- session_manager.rs: 6532 → ~6002 行（降 ~530 行）
- 全 workspace cargo test: 935+ 测试通过（不变）
- 全 workspace cargo check: 0 errors

**关键依赖**：
1. Task A 必须先执行（mod.rs 声明）
2. Task C（persistence）必须接受 2 个 helpers 归属 session_restore.rs 的决定
3. Task E（evidence）必须把 `strip_listing_diff_internal_reminders` 改 `pub(crate)`

**关键 risk**：
- Task A → 本 spec 顺序（建议单 commit atomic，或 Task A 已包含本 spec 的删除动作）
- 测试 helper 复制 drift（接受，~120 行重复）

**不在范围**：dialog turn / evidence / persistence 的其他方法拆分（其他 task）。

**Owner**: Mavis（orchestrator）
**External reviewer**: TBD（用户安排）
**Target**: 1 commit（refactor(session): real-wire session_restore.rs）+ 1 review handoff

---

## 8. Status

- [x] Spec 草稿（本文档）
- [ ] 用户 review
- [ ] 与 Task C / E reviewer 协调 helper 归属
- [ ] 应用（Task A commit + 本 spec commit）
- [ ] 内部自审（cargo check + cargo test + cargo fmt）
- [ ] 外部 review