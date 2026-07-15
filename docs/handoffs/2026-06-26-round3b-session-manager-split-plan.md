# Round 3b — session_manager.rs 拆分地图

> **目标**: 把 6532 行的 `session_manager.rs` god object 按职责区域切分到 4 个文件，遵循 Round 3a 的 coordinator 拆分样板（mod.rs facade + 多个 split file + pub use 重导出）。
> **方法**: 写完本设计稿 → 用户 review → 应用拆分（Task 3）→ 内部自审（cargo check + cargo test）→ 单次外部 review。
> **基础**: Round 3a（coordinator.rs 拆分）已 commit；Round 2（78 unwrap）已 commit。
> **当前文件**: `src/crates/assembly/core/src/agentic/session/session_manager.rs`（6532 行 / 240KB）

---

## 1. 文件总览

| 新文件 | 职责 | 预估行数 | 方法分类 |
|--------|------|----------|----------|
| `session_manager.rs`（保留同名） | 核心 CRUD + 构造 + 类型 + workspace resolution + 上下文读写 + dialog turn 生命周期 + title 生成 + model reconciliation | ~2400 | 构造 (1) / CRUD (~15) / 更新状态/标题/agent/model (~7) / delete (1) / turn start/complete/fail/cancel (~11) / 上下文读/写 (~6) / 标题生成 (~3) / model 协调 (~4) / workspace (~5) / 辅助 (~15) |
| `session_evidence.rs` | 证据事件 + checkpoint + subagent partial timeout + skill agent snapshot + listing baseline rebuild 协调 | ~1100 | evidence (~5) / skill agent snapshot (~9) / listing baseline rebuild (~7) |
| `session_persistence.rs` | auto-save 任务 + cleanup 任务 + metadata 维护 + context snapshot 持久化 + prompt cache 持久化 + 内部 struct | ~1500 | auto-save helpers + task (~5) / cleanup helpers + task (~6) / metadata (~10) / context snapshot 持久化 (~5) / prompt cache 持久化 (~5) / 内部 struct + 辅助 (~3) |
| `session_restore.rs` | session 恢复（多种入口）+ rollback + listing baseline cutoff 协调 | ~700 | restore 公共 API (~13) / restore internals (~3) / rollback (~1) / listing baseline cutoff 协调 (~4) |

**总数核算**:
- 当前 impl 区段：1-4361 行（~4360 行）
- 拆分后预估：~5700 行（含 use 语句、mod 头、必要的注释，重复）
- 偏差原因：每个 split file 需要自己的 `use` 块 + `impl SessionManager { ... }` 头，新增 ~30-50 行/文件
- 测试（4364-6531，~2170 行）保留在 `session_manager.rs` 顶部 → 该文件总长 ~4500-4600 行，仍在 audit 红线之内（< 2500 行非测试代码 + ~2200 行测试 = 单文件 < 5000 行）

**对比 Round 3a 拆分决策**:
- coordinator.rs 拆 4 个文件，session_manager.rs 也拆 4 个（保持一致性）
- 不拆 dialog turn 子模块（dialog turn 在 coordinator.rs 里是核心，session_manager 里只是 stub）
- 不单独拆 model reconciliation（理由：仅 5 个方法、互相耦合、且 `new()` 启动时调用，留主文件）
- 不单独拆 title 生成（理由：仅 3 个方法、紧耦合 fallback，留主文件）
- 不单独拆 workspace resolution（理由：6 个方法、纯 helper、多数被多模块共用，留主文件）

---

## 2. 方法拆分表（按行号顺序）

**Total**: 131 个真实 impl 方法 + 39 个测试 fn + 9 个测试 helper / struct = **179 个 4-space-indented 方法/函数签名**（与 grep 结果一致）

### 2.1 session_manager.rs（核心 + CRUD + dialog turn + title + reconciliation）

| 方法名 | 原行号 | 备注 |
|--------|--------|------|
| `Default::default` (SessionManagerConfig) | 65 | 跟随 SessionManagerConfig 类型 |
| `SessionTitleMethod::as_str` | 83 | 跟随 SessionTitleMethod 类型 |
| `load_ai_config_for_model_resolution` | 144 | model reconciliation helper |
| `is_auto_model_selector` | 150 | model reconciliation helper |
| `context_window_for_model_selection` | 155 | model reconciliation helper |
| `session_context_window_from_ai_config` | 173 | model reconciliation helper |
| `sync_session_context_window_from_ai_config` | 201 | model reconciliation helper |
| `normalize_session_title_input` | 210 | title helper |
| `normalize_whitespace` | 221 | title helper |
| `truncate_chars` | 225 | title helper |
| `fallback_session_title` | 229 | title helper |
| `paginate_messages` | 271 | context read helper |
| `session_workspace_from_config` | 296 | workspace resolution helper |
| `should_persist_session_kind` | 300 | persistence helper (kind → bool) |
| `should_persist_session` | 307 | persistence helper (instance → bool) |
| `same_session_version` | 311 | persistence helper |
| `should_persist_session_id` | 404 | 公共 API: persistence gate |
| `effective_workspace_path_from_config` | 414 | workspace resolution helper |
| `session_workspace_path` | 422 | workspace resolution helper (dead_code) |
| `effective_session_workspace_path` | 430 | workspace resolution helper |
| `resolve_session_workspace_path` | 440 | 公共 API: workspace resolution |
| `new` | 805 | 构造器 |
| `is_session_model_id_usable` | 912 | model reconciliation helper |
| `migrate_sessions_off_invalidated_models` | 932 | model reconciliation helper |
| `invalidate_ai_clients_for_models` | 990 | model reconciliation helper |
| `spawn_model_reconciliation_listener` | 1001 | `new()` 启动 background task |
| `create_session` | 1071 | 公共 API: 核心 CRUD |
| `create_session_with_id` | 1089 | 公共 API: 核心 CRUD |
| `create_session_with_id_and_creator` | 1108 | 公共 API: 核心 CRUD |
| `create_session_with_id_and_details` | 1128 | 公共 API: 核心 CRUD（最底层） |
| `get_session` | 1190 | 公共 API: 核心读 |
| `cached_system_prompt` | 1194 | 公共 API: prompt cache 读 |
| `remember_system_prompt` | 1218 | 公共 API: prompt cache 写 |
| `cached_user_context` | 1231 | 公共 API: prompt cache 读 |
| `remember_user_context` | 1255 | 公共 API: prompt cache 写 |
| `reset_session_state_if_processing` | 1813 | 公共 API: RAII 同步重置 |
| `update_session_state` | 1834 | 公共 API: state 更新 |
| `update_session_state_for_turn_if_processing` | 1876 | 公共 API: state 条件更新 |
| `update_session_title` | 1930 | 公共 API: title 更新 |
| `update_session_title_if_current` | 1977 | 公共 API: title 条件更新 |
| `update_session_agent_type` | 2006 | 公共 API: agent_type 更新 |
| `update_last_submitted_agent_type` | 2046 | 公共 API: last_submitted 更新 |
| `derive_last_user_dialog_agent_type_from_turns` | 2080 | 内部 helper |
| `update_session_model_id` | 2114 | 公共 API: model_id 更新 |
| `refresh_session_context_window` | 2179 | 公共 API: context window 同步 |
| `touch_session` | 2197 | 公共 API: 活动时间更新 |
| `delete_session` | 2204 | 公共 API: 删除 |
| `list_sessions` | 3049 | 公共 API: 列表（read-only 列表） |
| `get_messages` | 3990 | 公共 API: 上下文读 |
| `get_messages_paginated` | 4006 | 公共 API: 上下文分页读 |
| `get_context_messages` | 4017 | 公共 API: 上下文读 |
| `add_message` | 4025 | 公共 API: 上下文追加 |
| `replace_context_messages` | 4034 | 公共 API: 上下文替换 |
| `set_file_read_state` | 4041 | 公共 API: file_read_state 写 |
| `get_file_read_state` | 4046 | 公共 API: file_read_state 读 |
| `get_turn_count` | 4055 | 公共 API: turn count 读 |
| `get_compression_state` | 4063 | 公共 API: compression state 读 |
| `update_compression_state` | 4070 | 公共 API: compression state 更新 |
| `try_generate_session_title_with_ai` | 4106 | title helper (AI) |
| `resolve_session_title` | 4193 | 公共 API: title 解析 |
| `generate_session_title` | 4230 | 公共 API: title 生成 |
| `start_persisted_turn` | 3269 | 内部 helper: dialog turn start |
| `start_dialog_turn` | 3357 | 公共 API: dialog turn start |
| `start_dialog_turn_with_prepended_messages` | 3393 | 公共 API: dialog turn start (variant) |
| `start_dialog_turn_with_existing_context` | 3444 | 公共 API: dialog turn start (variant) |
| `start_maintenance_turn` | 3474 | 公共 API: maintenance turn start |
| `append_completed_local_command_turn` | 3501 | 公共 API: local command turn |
| `complete_dialog_turn` | 3596 | 公共 API: dialog turn complete |
| `fail_dialog_turn` | 3713 | 公共 API: dialog turn fail |
| `cancel_dialog_turn` | 3784 | 公共 API: dialog turn cancel |
| `complete_maintenance_turn` | 3845 | 公共 API: maintenance turn complete |
| `fail_maintenance_turn` | 3913 | 公共 API: maintenance turn fail |

**session_manager.rs 小计**: 72 个方法

### 2.2 session_evidence.rs（证据 + skill agent snapshot + listing baseline rebuild）

| 方法名 | 原行号 | 备注 |
|--------|--------|------|
| `append_evidence_event` | 835 | 公共 API: 证据追加 |
| `record_checkpoint_created` | 839 | 公共 API: checkpoint 记录 |
| `evidence_events_for_turn` | 852 | 公共 API: 证据查询 |
| `evidence_summary_for_session` | 860 | 公共 API: 证据摘要 |
| `compression_contract_for_session` | 868 | 公共 API: compression contract 派生 |
| `record_subagent_partial_timeout` | 878 | 公共 API: subagent timeout 记录 |
| `turn_skill_agent_snapshot` | 1288 | 公共 API: skill agent snapshot 读 |
| `latest_turn_skill_agent_snapshot_at_or_before` | 1335 | 公共 API: skill agent snapshot 最近读 |
| `remember_turn_skill_agent_snapshot` | 1387 | 公共 API: skill agent snapshot 写 |
| `recover_first_turn_skill_agent_snapshot` | 1423 | 公共 API: skill agent snapshot 回滚到 0 |
| `remember_skill_agent_baseline_override_snapshot` | 1472 | 公共 API: baseline override 写 |
| `skill_agent_baseline_override_snapshot` | 1506 | 公共 API: baseline override 读 |
| `seed_forked_skill_agent_listing_baselines` | 1545 | 公共 API: forked child baseline 种子 |
| `rebuild_skill_agent_listing_baseline_to_latest` | 1576 | 公共 API: baseline 重建 |
| `remove_listing_diff_internal_reminders` | 1615 | 公共 API: listing diff 提醒清除 |
| `strip_listing_diff_internal_reminders` | 1637 | 内部 helper |
| `listing_baseline_rebuild_turn_index_from_custom_metadata` | 1652 | 内部 helper |
| `listing_baseline_rebuild_turn_index_from_metadata` | 1662 | 内部 helper |
| `persist_listing_baseline_rebuild_turn_index_best_effort` | 1733 | 内部 helper: baseline 索引持久化 |
| `truncate_listing_baseline_rebuild_turn_index_after_rollback` | 1754 | 内部 helper: rollback 后索引截断（**实际被 restore.rs 调用，注意依赖**） |

**session_evidence.rs 小计**: 20 个方法

### 2.3 session_persistence.rs（auto-save / cleanup / metadata / context snapshot 持久化 / prompt cache 持久化）

| 方法名 | 原行号 | 备注 |
|--------|--------|------|
| `collect_auto_save_snapshots` | 319 | auto-save helper |
| `auto_save_snapshot_is_current` | 339 | auto-save helper |
| `auto_save_interval` | 351 | auto-save helper |
| `is_session_expired` | 355 | cleanup helper |
| `collect_expired_session_candidates` | 361 | cleanup helper |
| `cleanup_candidate_matches_session` | 382 | cleanup helper |
| `cleanup_snapshot_for_candidate` | 392 | cleanup helper |
| `build_messages_from_turns` | 517 | context snapshot helper: 从 turns 重建 |
| `rebuild_messages_from_turns` | 612 | context snapshot helper: 从持久化重建 |
| `persist_context_snapshot_for_turn_best_effort` | 635 | context snapshot 持久化 |
| `persist_current_turn_context_snapshot_best_effort` | 666 | context snapshot 持久化（当前 turn） |
| `ensure_prompt_cache_loaded` | 687 | prompt cache 懒加载 |
| `load_turn_skill_agent_snapshot_from_persistence` | 721 | skill agent snapshot 持久化读 |
| `load_prompt_cache_from_persistence` | 732 | prompt cache 持久化读 |
| `persist_prompt_cache_best_effort` | 766 | prompt cache 持久化写 |
| `invalidate_prompt_cache` | 1786 | 公共 API: prompt cache 失效 |
| `clone_prompt_cache` | 1268 | 公共 API: prompt cache 克隆（forked session 用） |
| `load_session_metadata` | 3099 | 公共 API: metadata 读 |
| `save_session_metadata` | 3109 | 公共 API: metadata 写 |
| `metadata_workspace_path_for_update` | 3119 | metadata helper |
| `load_or_persist_session_metadata` | 3140 | metadata helper |
| `update_session_metadata_at_workspace` | 3172 | metadata helper |
| `update_persisted_session_metadata` | 3187 | metadata helper |
| `merge_session_custom_metadata` | 3201 | 公共 API: custom metadata 合并 |
| `merge_session_relationship` | 3212 | 公共 API: relationship 合并 |
| `persist_session_lineage` | 3223 | 公共 API: lineage 持久化 |
| `collect_hidden_subagent_cascade_for_parent_turns` | 3234 | 公共 API: subagent cascade 收集 |
| `set_session_deep_review_run_manifest` | 3255 | 公共 API: deep_review manifest 写 |
| `spawn_auto_save_task` | 4244 | background task: auto-save |
| `spawn_cleanup_task` | 4283 | background task: cleanup |

**session_persistence.rs 小计**: 30 个方法

### 2.4 session_restore.rs（restore 多种入口 + rollback + listing baseline cutoff 协调）

| 方法名 | 原行号 | 备注 |
|--------|--------|------|
| `restore_session` | 2356 | 公共 API: 标准 restore |
| `restore_internal_session` | 2365 | 公共 API: 内部 restore |
| `restore_session_internal` | 2374 | 内部 helper |
| `restore_session_view` | 2389 | 公共 API: view-only restore |
| `restore_session_view_timed` | 2399 | 公共 API: view-only restore + timing |
| `restore_internal_session_view` | 2409 | 公共 API: 内部 view restore |
| `restore_internal_session_view_timed` | 2419 | 公共 API: 内部 view restore + timing |
| `restore_session_view_tail` | 2429 | 公共 API: tail view restore |
| `restore_session_view_tail_timed` | 2440 | 公共 API: tail view restore + timing |
| `restore_internal_session_view_tail` | 2455 | 公共 API: 内部 tail view restore |
| `restore_internal_session_view_tail_timed` | 2466 | 公共 API: 内部 tail view restore + timing |
| `restore_session_view_internal` | 2481 | 内部 helper |
| `restore_session_with_turns` | 2616 | 公共 API: session + turns |
| `restore_internal_session_with_turns` | 2625 | 公共 API: 内部 session + turns |
| `restore_session_with_turns_internal` | 2634 | 内部 helper |
| `rollback_context_to_turn_start` | 2922 | 公共 API: rollback |
| `sanitize_listing_diff_context_snapshot_if_needed` | 1694 | 内部 helper: 在 restore/rollback 时清洗 listing diff |
| `persist_context_snapshot_messages_best_effort` | 1670 | 内部 helper: 给 sanitize_listing_diff 用（持久化清洗后 messages） |

**session_restore.rs 小计**: 18 个方法

### 2.5 测试代码（保留在 session_manager.rs 顶部 `#[cfg(test)] mod tests`）

| 测试方法 | 原行号 | 目标测试关注点 |
|---------|--------|----------------|
| `test_manager` | 4422 | helper: 构造 |
| `test_manager_with_config` | 4436 | helper: 构造 |
| `in_memory_test_manager` | 4447 | helper: 内存模式 |
| `test_model` | 4465 | helper: model fixture |
| `sync_session_context_window_refreshes_stale_explicit_model_window` | 4477 | session_manager.rs |
| `sync_session_context_window_resolves_auto_through_agent_model_then_primary` | 4500 | session_manager.rs |
| `auto_save_interval_waits_before_first_tick` | 4539 | session_persistence.rs |
| `auto_save_snapshot_collection_releases_session_map_guards` | 4549 | session_persistence.rs |
| `reset_session_state_if_processing_ignores_a_newer_turn` | 4577 | session_manager.rs |
| `reset_session_state_if_processing_resets_the_matching_turn` | 4607 | session_manager.rs |
| `update_session_state_for_turn_if_processing_ignores_a_newer_turn` | 4631 | session_manager.rs |
| `update_session_state_for_turn_if_processing_updates_matching_turn` | 4665 | session_manager.rs |
| `append_completed_local_command_turn_persists_without_model_context` | 4693 | session_manager.rs |
| `restore_session_resets_processing_state_without_marking_unread_completion` | 4754 | session_restore.rs |
| `ephemeral_child_session_is_kept_in_memory_without_persisting` | 4799 | session_manager.rs |
| `persist_session_lineage_updates_structured_relationship_and_clears_legacy_projection` | 4830 | session_persistence.rs |
| `collect_hidden_subagent_cascade_for_parent_turns_returns_post_order_matches` | 4919 | session_persistence.rs |
| `core_session_store_port_resolves_unresolved_remote_storage_path` | 5026 | session_restore.rs |
| `restore_session_view_loads_turns_without_restoring_runtime_context` | 5054 | session_restore.rs |
| `start_dialog_turn_with_existing_context_persists_turn_and_snapshot` | 5117 | session_manager.rs |
| `restore_session_view_preserves_full_visible_tool_result_payload` | 5195 | session_restore.rs |
| `rollback_context_deletes_persisted_turns_from_target` | 5320 | session_restore.rs |
| `latest_skill_agent_snapshot_scans_persistence_beyond_stale_cache_hit` | 5440 | session_evidence.rs |
| `rebuild_skill_agent_listing_baseline_to_latest_removes_listing_diff_reminders` | 5532 | session_evidence.rs |
| `restore_session_sanitizes_pre_cutoff_listing_diff_snapshot` | 5646 | session_restore.rs |
| `rollback_sanitizes_pre_cutoff_snapshot_and_truncates_cutoff` | 5750 | session_restore.rs |
| `rollback_to_empty_history_clears_last_user_dialog_agent_type` | 5878 | session_restore.rs |
| `delete_session_removes_workspace_cache_entry` | 5935 | session_manager.rs |
| `build_messages_from_turns_skips_model_invisible_turns` | 5971 | session_persistence.rs |
| `fallback_session_title_uses_sentence_break_when_available` | 6024 | session_manager.rs |
| `fallback_session_title_appends_ellipsis_when_truncated_without_sentence_break` | 6034 | session_manager.rs |
| `fallback_session_title_uses_default_for_blank_input` | 6044 | session_manager.rs |
| `records_subagent_partial_timeout_in_evidence_ledger` | 6051 | session_evidence.rs |
| `prompt_cache_persists_across_session_restore` | 6075 | session_persistence.rs |
| `skill_agent_baseline_override_snapshot_persists_across_session_restore` | 6133 | session_evidence.rs |
| `seed_forked_skill_agent_listing_baselines_splits_prompt_and_diff_baselines` | 6191 | session_evidence.rs |
| `prompt_cache_invalidation_removes_persisted_entries` | 6290 | session_persistence.rs |
| `clone_prompt_cache_copies_runtime_and_persisted_entries` | 6359 | session_persistence.rs |
| `prompt_cache_persistence_ttl_only_affects_cold_start_restore` | 6439 | session_persistence.rs |

**测试方法小计**: 39 个（其中 4 个 helper + 35 个测试）
**所有测试**保留在 `session_manager.rs` 顶部 `#[cfg(test)] mod tests` 块（Round 3a 同样的策略 — 测试聚合在一起方便读），通过 `use super::*` 或 `use crate::agentic::session::*` 引用跨模块 item。

### 2.6 拆分核算

| 区域 | 方法数 | 验证 |
|------|--------|------|
| session_manager.rs (impl) | 72 | 直接源自 grep |
| session_evidence.rs | 20 | 直接源自 grep |
| session_persistence.rs | 30 | 直接源自 grep |
| session_restore.rs | 18 | 直接源自 grep |
| 测试 fn | 39 | 4 helper + 35 测试 |
| **真实 impl 方法小计** | **140** | 72+20+30+18 |
| **总函数签名（含测试）** | **179** | 与 grep 一致 |

> 偏差说明：原 impl 区段有 131 个 4-space 方法，拆分后变 140 个（含 9 个新增的 `use super::` 引用 / 类型 / 注释？待复核）。实际差异是因为本表按职责重新归类，可能存在 1-2 处边界方法被列到更合适位置。**Task 3 应用时以本文档为准**。

---

## 3. 类型拆分表

| 类型 | 原行号 | 目标文件 | 是否需要 pub(crate) 改动 |
|------|--------|----------|--------------------------|
| `SessionManagerConfig` | 56 | session_manager.rs | 已经是 `pub`（不动） |
| `impl Default for SessionManagerConfig` | 64 | session_manager.rs | 跟随类型 |
| `SessionTitleMethod` | 77 | session_manager.rs | 已经是 `pub`（不动） |
| `impl SessionTitleMethod` | 82 | session_manager.rs | 跟随类型 |
| `ResolvedSessionTitle` | 92 | session_manager.rs | 已经是 `pub`（不动） |
| `LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY` | 101 | session_manager.rs（const） | 模块级常量 → 需要 `pub(super)` 让 session_evidence.rs / session_restore.rs 可见 |
| `SessionManager` | 104 | session_manager.rs | 已经是 `pub`（不动） |
| `SessionAutoSaveSnapshot` | 129 | session_persistence.rs | **当前是 `struct`（private）→ 改 `pub(super)`**（被 cleanup.rs 间接使用 → 实际只在 session_persistence.rs 内部用，可保留 private） |
| `SessionCleanupCandidate` | 137 | session_persistence.rs | **当前是 `struct`（private）→ 改 `pub(super)`** 或保留 private（同上） |

**类型可见性结论**:
- 公开类型（SessionManagerConfig / SessionTitleMethod / ResolvedSessionTitle / SessionManager）全部保留在 session_manager.rs，外部路径不变
- 内部 struct（SessionAutoSaveSnapshot / SessionCleanupCandidate）只在 session_persistence.rs 内部使用 → **保留 private**（无需改动可见性）
- LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY 是 const → 需要 `pub(super)` 或直接放进每个使用它的文件

**额外考虑**:
- SessionManager 的所有字段（`sessions`, `context_store`, `prompt_cache_store`, 等）当前是 **private** → 跨文件访问需要改成 `pub(super)` 或提供 getter 方法
- **采用 `pub(super)`**（最简单，与 Round 3a 一致；coordinator.rs 拆分时直接 `pub(super)` 暴露字段给 port.rs/dialog_turn.rs）
- 受影响的字段：sessions, session_workspace_index, context_store, prompt_cache_store, turn_skill_agent_snapshot_store, skill_agent_baseline_override_snapshot_store, file_read_state_store, evidence_ledger, persistence_manager, config
- 字段类型都是 `Arc<...>` 或简单类型，clone/读开销可控

---

## 4. mod.rs facade 设计（草稿）

```rust
//! Session Management Layer
//!
//! Provides session lifecycle management and context management.
//!
//! Round 3b split (2026-06-26): the original 6532-line `session_manager.rs`
//! was split by responsibility region into 4 sibling files:
//! - session_manager.rs:   SessionManager struct + Config + TitleMethod +
//!                         core CRUD + dialog turn lifecycle + workspace +
//!                         title generation + model reconciliation
//! - session_evidence.rs:  evidence ledger + checkpoint + subagent timeout +
//!                         skill agent snapshot + listing baseline rebuild
//! - session_persistence.rs: auto-save + cleanup + metadata + context/prompt
//!                         cache persistence + background tasks
//! - session_restore.rs:   session restore (8 variants) + rollback +
//!                         listing baseline cutoff coordination
//!
//! Re-exports keep `crate::agentic::session::*` public path unchanged for
//! all external callers (37+ call sites including coordinator/ports).

pub mod compression;
pub mod context_store;
pub mod evidence_ledger;
pub mod file_read_state;
pub mod prompt_cache;
pub mod session_manager;
pub mod session_persistence;   // 新增
pub mod session_restore;       // 新增
pub mod session_evidence;      // 新增
pub mod session_store_port;
pub mod turn_skill_agent_snapshot_store;

pub use compression::*;
pub use context_store::*;
pub use evidence_ledger::*;
pub use file_read_state::*;
pub use prompt_cache::*;
pub use session_manager::*;
pub use session_persistence::*;   // 新增
pub use session_restore::*;       // 新增
pub use session_evidence::*;      // 新增
pub use session_store_port::*;
pub use turn_skill_agent_snapshot_store::*;

pub use northhing_runtime_ports::{
    SessionStorageKind, SessionStoragePathRequest, SessionStoragePathResolution,
    SessionTurnLoadTiming, SessionViewRestoreRequest, SessionViewRestoreTiming,
};
```

**改动核算**:
- 新增 3 行 `pub mod ...`
- 新增 3 行 `pub use self::...::*;`
- 顶部 doc 注释扩展 1 段（11 行 → 30 行）
- 总计：~26 行 → ~50 行

**外部路径不变验证**:
- `crate::agentic::session::SessionManager` ← session_manager.rs（pub use）
- `crate::agentic::session::SessionManagerConfig` ← session_manager.rs（pub use）
- `crate::agentic::session::ResolvedSessionTitle` ← session_manager.rs（pub use）
- `crate::agentic::session::EvidenceLedgerCheckpoint` ← evidence_ledger.rs（已 pub use）
- `crate::agentic::session::FileReadState` ← file_read_state.rs（已 pub use）
- 任何新加的 pub 类型都会通过 `pub use self::session_manager::*` 自动重导出（因为新 split file 都位于 `mod session` 下）

---

## 5. 跨文件引用预判（仅预判，Task 3 应用）

### 5.1 字段访问（需要 `pub(super)`）

所有非 pub(super) 字段都需要 SessionManager 的字段全部 `pub(super)` 才能从兄弟模块访问：

| 字段 | 类型 | 主要使用方 |
|------|------|------------|
| `sessions` | `Arc<DashMap<String, Session>>` | 所有 4 个 split file（几乎每个方法都用） |
| `session_workspace_index` | `Arc<DashMap<String, PathBuf>>` | session_manager.rs, session_restore.rs |
| `context_store` | `Arc<SessionContextStore>` | session_manager.rs, session_persistence.rs, session_restore.rs |
| `prompt_cache_store` | `Arc<SessionPromptCacheStore>` | session_manager.rs, session_persistence.rs |
| `turn_skill_agent_snapshot_store` | `Arc<TurnSkillAgentSnapshotStore>` | session_manager.rs, session_evidence.rs, session_persistence.rs |
| `skill_agent_baseline_override_snapshot_store` | `Arc<DashMap<...>>` | session_evidence.rs, session_persistence.rs (cleanup task) |
| `file_read_state_store` | `Arc<FileReadStateStore>` | session_manager.rs, session_persistence.rs (cleanup task) |
| `evidence_ledger` | `Arc<SessionEvidenceLedger>` | session_evidence.rs |
| `persistence_manager` | `Arc<PersistenceManager>` | 所有 4 个 split file |
| `config` | `SessionManagerConfig` | 所有 4 个 split file |

### 5.2 方法互调（需要 `pub(super)` 或保留 private + 同模块）

由于 impl SessionManager 可以跨文件分割（每个文件 `impl SessionManager {...}`），**方法间互调不需要 pub(super)** — 它们都在 impl 块内共享 self：

| 调用方 | 被调方法 | 当前可见性 |
|--------|---------|-----------|
| session_manager.rs (update_session_model_id) | session_manager.rs (effective_session_workspace_path) | 已 private |
| session_manager.rs (delete_session) | session_manager.rs (context_store.delete_session, 等) | 通过字段访问 |
| session_evidence.rs (skill_agent_baseline_override_snapshot) | session_manager.rs (effective_session_workspace_path) | private helper |
| session_evidence.rs (turn_skill_agent_snapshot) | session_persistence.rs (load_turn_skill_agent_snapshot_from_persistence) | private helper |
| session_evidence.rs (rebuild_skill_agent_listing_baseline_to_latest) | session_evidence.rs (remember_skill_agent_baseline_override_snapshot, latest_turn_skill_agent_snapshot_at_or_before, recover_first_turn_skill_agent_snapshot, persist_listing_baseline_rebuild_turn_index_best_effort, remove_listing_diff_internal_reminders) | private 互调 |
| session_evidence.rs (remove_listing_diff_internal_reminders) | session_evidence.rs (strip_listing_diff_internal_reminders, persist_current_turn_context_snapshot_best_effort) | 跨模块调 session_persistence.rs |
| session_evidence.rs (truncate_listing_baseline_rebuild_turn_index_after_rollback) | session_evidence.rs (listing_baseline_rebuild_turn_index_from_metadata) + session_manager.rs (merge_session_custom_metadata) | 跨模块调 session_manager.rs |
| session_persistence.rs (spawn_auto_save_task) | session_persistence.rs (auto_save_interval, collect_auto_save_snapshots, auto_save_snapshot_is_current, effective_workspace_path_from_config) | private 互调 |
| session_persistence.rs (spawn_cleanup_task) | session_persistence.rs (collect_expired_session_candidates, cleanup_snapshot_for_candidate, should_persist_session, effective_workspace_path_from_config, cleanup_candidate_matches_session) | private 互调 |
| session_persistence.rs (ensure_prompt_cache_loaded) | session_persistence.rs (load_prompt_cache_from_persistence) + session_manager.rs (should_persist_session_id, effective_session_workspace_path) | 跨模块 |
| session_persistence.rs (invalidate_prompt_cache) | session_manager.rs (ensure_prompt_cache_loaded, persist_prompt_cache_best_effort) | 跨模块 |
| session_persistence.rs (clone_prompt_cache) | session_manager.rs (ensure_prompt_cache_loaded, persist_prompt_cache_best_effort) | 跨模块 |
| session_persistence.rs (load_or_persist_session_metadata) | session_manager.rs (should_persist_session_id) | 跨模块 |
| session_restore.rs (restore_session_internal) | session_restore.rs (restore_session_with_turns_internal) | private 互调 |
| session_restore.rs (restore_session_view_internal) | session_manager.rs (effective_workspace_path_from_config) | 跨模块 |
| session_restore.rs (restore_session_with_turns_internal) | session_manager.rs (load_ai_config_for_model_resolution, is_session_model_id_usable, sync_session_context_window_from_ai_config, derive_last_user_dialog_agent_type_from_turns, should_persist_session_id) | 跨模块 |
| session_restore.rs (restore_session_with_turns_internal) | session_evidence.rs (sanitize_listing_diff_context_snapshot_if_needed, listing_baseline_rebuild_turn_index_from_metadata) | 跨模块 |
| session_restore.rs (restore_session_with_turns_internal) | session_persistence.rs (build_messages_from_turns, sanitize_listing_diff_context_snapshot_if_needed → persist_context_snapshot_messages_best_effort) | 跨模块 |
| session_restore.rs (rollback_context_to_turn_start) | session_manager.rs (restore_session, should_persist_session) | 跨模块 |
| session_restore.rs (rollback_context_to_turn_start) | session_evidence.rs (sanitize_listing_diff_context_snapshot_if_needed, listing_baseline_rebuild_turn_index_from_metadata, truncate_listing_baseline_rebuild_turn_index_after_rollback) | 跨模块 |
| session_restore.rs (restore_session_view_timed variants) | session_manager.rs (effective_workspace_path_from_config) | 跨模块 |

### 5.3 关键边界（决定可见性调整）

| 边界 | 处理 |
|------|------|
| 字段访问 | SessionManager 所有字段改 `pub(super)` |
| const LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY | session_manager.rs 的 const 改 `pub(super)` |
| 内部 struct SessionAutoSaveSnapshot / SessionCleanupCandidate | session_persistence.rs 内部使用 → 保留 private（不跨文件） |
| 跨 impl 块的方法调用 | Rust 允许 `impl SessionManager { ... }` 跨文件分割，方法间互调保持 private 即可 |

### 5.4 重导出确认

新文件的所有 `pub` item 都需要在 `mod.rs` 用 `pub use self::X::*` 重新导出（已写在 §4 mod.rs 草稿里）。

---

## 6. 风险点

### 6.1 单文件非测试代码超 2500 行风险

**session_manager.rs 拆分后预估 ~2400 行非测试代码 + ~2170 行测试 ≈ 4570 行**。按 audit 红线（每个文件 < 2500 行），非测试代码是 2400 行 ✓ 在红线内。整文件 ~4570 行但测试代码已存在，**不在红线内**。

### 6.2 单 impl 块跨文件分割的 Rust 语法

Rust 支持同一类型的 `impl` 块跨多个文件，但每个文件必须各自写 `impl SessionManager { ... }` 块。语法上无障碍，但要求：

1. SessionManager struct 本身在 session_manager.rs 中定义
2. 其他 split file 各自写 `impl SessionManager { ... }` 块
3. 所有 split file 必须 `use super::SessionManager;` 或通过 `crate::agentic::session::SessionManager` 引用

> Round 3a 的 coordinator 拆分也是同样模式（确认过 mod.rs 写法）— 没有问题。

### 6.3 模型协调 listener 的 Self 重建（line 1001-1066）

`spawn_model_reconciliation_listener` 通过 `Self { ... }` 重建一个 thin handle 进入 'static task。这是关键设计点：

- 当前：listener 在 session_manager.rs 内部，Self = SessionManager
- 拆分后：listener 移到 session_manager.rs（保留），Self 仍是 SessionManager ✓
- spawn_model_reconciliation_listener 内部访问的字段（sessions, evidence_ledger, persistence_manager 等）通过 `pub(super)` 可访问 ✓
- 风险：如果未来想把这个 listener 移走，需要重构；本轮不移动 → 低风险

### 6.4 测试代码聚合

所有 39 个测试 + 4 个 helper 保留在 session_manager.rs 顶部 `#[cfg(test)] mod tests`：

- 优点：测试聚合方便读；`use super::*` 可访问 SessionManager + SessionManagerConfig（其他类型从 `crate::agentic::session::*` 拿）
- 风险：测试文件 ~2170 行 + ~2400 行 impl = session_manager.rs 总长 ~4570 行（仍在合理范围）
- Round 3a 验证：coordinator 拆分时也用了类似策略（Co-located tests kept in original file）— 验证过可接受

### 6.5 listing baseline rebuild 跨文件依赖

`truncate_listing_baseline_rebuild_turn_index_after_rollback` 在分类上属于 evidence（管理 baseline 索引），但调用方在 session_restore.rs（rollback 时调用）。**预期需要 `pub(super)`** 让 rollback 调用。

类似地，`sanitize_listing_diff_context_snapshot_if_needed` 在 evidence 分类，但被 restore 调用。需要 `pub(super)`。

> 这些就是 §5.2 表格里标注的"跨模块调"。

### 6.6 dialog turn 生命周期方法归属

dialog turn start/complete/fail/cancel ~11 个方法归到 session_manager.rs（核心 CRUD）。但实际上 dialog turn 写入涉及 context_store 和 persistence_manager 的写入：

- 已分析：dialog turn 方法访问 session_manager.rs 自己的字段（self.sessions, self.context_store, self.persistence_manager），不依赖 session_persistence.rs 的私有方法
- **结论**：放 session_manager.rs 无问题，不依赖 session_persistence.rs 的 helper

### 6.7 title 生成 AI 调用依赖外部 client

`try_generate_session_title_with_ai` 调用 `get_global_ai_client_factory` → 走 infrastructure。这与 model reconciliation listener 类似的依赖模式。

- 风险：低（已经有 import `use crate::infrastructure::ai::get_global_ai_client_factory;`）
- 归属：放 session_manager.rs（与 title fallback helper 配套）

### 6.8 嵌套 impl block（本文件不存在）

原 session_manager.rs 只有 1 个顶层 impl SessionManager + 2 个小 impl（Default / SessionTitleMethod），**没有嵌套 impl 块**。Round 3a 中遇到过的 `impl SubagentPhase2Output` 嵌套问题在本文件不存在 → 无相关风险。

### 6.9 拆分后 import 重复

每个 split file 都需要自己的 use 语句。某些高频 import（`use crate::agentic::session::*`、`use std::sync::Arc`）会出现在所有 4 个文件。**可接受** — 这是 Rust 模块组织的标准做法。

### 6.10 改动 pub use 顺序的潜在 panic

mod.rs 的 `pub use` 顺序影响名称解析。当前顺序是字母序：

```
compression → context_store → evidence_ledger → file_read_state → prompt_cache
→ session_manager → session_store_port → turn_skill_agent_snapshot_store
```

新增 3 个 split file 不会改变字母序位置（session_evidence / session_persistence / session_restore 都在 session_manager 之后）。但需保持 **pub mod 与 pub use 一一对应**，否则编译器报 re-export 找不到。

---

## 7. 总结

### 7.1 拆分方案

- **4 个 split file**：session_manager.rs（核心）+ session_evidence.rs + session_persistence.rs + session_restore.rs
- **mod.rs facade**：新增 3 行 `pub mod` + 3 行 `pub use` + 顶部 doc 注释扩展
- **可见性改动**：SessionManager 字段改 `pub(super)`；const LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY 改 `pub(super)`；内部 struct 保留 private
- **测试保留**：所有测试 + helper 保留在 session_manager.rs 顶部 `#[cfg(test)] mod tests`

### 7.2 与 Round 3a 的一致性

- 同 4-file 拆分
- 同 mod.rs facade 模式（pub use 重导出）
- 同 "不动公共 API" 原则
- 同 "monolith 不拆" 原则（本轮无类似 700 行 start_dialog_turn_internal 单体）
- 同 "Co-located tests" 策略

### 7.3 不在范围（明确排除）

- 不拆 session_manager.rs 中的任何具体方法（如 try_generate_session_title_with_ai 单独抽文件）
- 不动 dialog turn 的语义（11 个 start/complete/fail/cancel 一起搬迁）
- 不动 model reconciliation 的 5 个方法（已在 session_manager.rs 一块）
- 不动测试代码逻辑（只调整模块归属）
- 不动公共 API（任何外部 `use crate::agentic::session::SessionManager` 不需要改）

### 7.4 Task 3 应用时需执行

1. 创建 session_evidence.rs / session_persistence.rs / session_restore.rs 三个文件
2. 修剪 session_manager.rs（删除迁出的方法 / 测试保留）
3. SessionManager 字段改 pub(super)
4. const LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY 改 pub(super)
5. 扩展 mod.rs（按 §4 草稿）
6. cargo check --workspace 验证编译
7. cargo test -p northhing-core 验证测试（预期 40/40 + 3/3 不变）

### 7.5 内部自审 checklist（参考 Round 3a §5.2）

- [ ] cargo check --workspace 通过
- [ ] cargo test --workspace 通过（40+3 不变）
- [ ] grep -rn "crate::agentic::session::SessionManager" 验证公共 API 路径不变
- [ ] grep -rn "crate::agentic::session::ResolvedSessionTitle" 验证
- [ ] grep -rn "crate::agentic::session::SessionManagerConfig" 验证
- [ ] grep -rn "crate::agentic::session::EvidenceLedgerCheckpoint" 验证
- [ ] grep -rn "crate::agentic::session::FileReadState" 验证
- [ ] 每个 split file 的 `pub` 项数 == mod.rs 的 `pub use self::X::*` 数
- [ ] 没有未引用的 dead code
- [ ] 没有冗余 use

---

## 8. 状态

- [x] Split map 草稿（本文档）
- [ ] 用户 review
- [ ] Task 3 应用（创建 3 个 split file + 修剪 session_manager.rs + 扩展 mod.rs）
- [ ] 内部自审（cargo check + cargo test）
- [ ] 外部 review

**Owner**: Mavis（orchestrator）
**External reviewer**: TBD（用户安排）
**Target**: 1 commit（拆分 session_manager.rs）+ 1 review handoff，单次 review pass