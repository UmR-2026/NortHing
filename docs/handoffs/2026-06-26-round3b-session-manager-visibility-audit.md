# Round 3b · session_manager.rs 拆分后 visibility 改动审计

> 目的：在 Round 3b 拆分 session_manager.rs 之前，预先识别**所有跨 split file 引用**，
> 让执行阶段（task `execute-split`）能一次性 apply visibility 改动，
> 避免手动 debug 多次。
>
> 输入文件：
> - `E:/agent-project/northing/src/crates/assembly/core/src/agentic/session/session_manager.rs`（6505 行）
> - `E:/agent-project/northing/src/crates/assembly/core/src/agentic/session/mod.rs`（现有 facade）
> - `E:/agent-project/northing/src/crates/assembly/core/src/agentic/coordination/`（Round 3a 样板）
>
> 扫描工具：ripgrep（`rg`）跨文件搜索 + Select-String 局部定位 + Read 按段浏览。
>
> 日期：2026-06-27
> 作者：Mavis · general agent · mvs_c555b6029df641ae9dd1ba849ed0fd58

---

## 1. session_manager.rs 内部 cross-reference 扫描

### 1.1 文件结构（line ranges，按 `// ============` 段落注释 + fn 定义位置）

| 段 | 起 | 止 | 内容 | 行数 |
|----|-----|-----|------|------|
| 头 | 1 | 142 | module doc + use 块 + `SessionManagerConfig`/`SessionTitleMethod`/`ResolvedSessionTitle` + `SessionManager` struct + `SessionAutoSaveSnapshot` + `SessionCleanupCandidate` | 142 |
| helpers-A | 143 | 403 | model resolution + title helpers + persistence predicates | 261 |
| workspace-helpers | 404 | 515 | workspace resolution 系列 | 112 |
| message-builders | 517 | 609 | `build_messages_from_turns` | 93 |
| persist-context | 612 | 680 | `persist_context_snapshot_for_turn_best_effort` / `persist_current_turn_context_snapshot_best_effort` | 69 |
| prompt-cache-load | 687 | 803 | `ensure_prompt_cache_loaded` / `load_turn_skill_agent_snapshot_from_persistence` / `load_prompt_cache_from_persistence` / `persist_prompt_cache_best_effort` | 117 |
| constructor | 805 | 833 | `new` | 29 |
| evidence-methods | 835 | 903 | `append_evidence_event` / `record_checkpoint_created` / `evidence_events_for_turn` / `evidence_summary_for_session` / `compression_contract_for_session` / `record_subagent_partial_timeout` | 69 |
| reconciliation | 905 | 1066 | `is_session_model_id_usable` / `migrate_sessions_off_invalidated_models` / `invalidate_ai_clients_for_models` / `spawn_model_reconciliation_listener` | 162 |
| create-sessions | 1068 | 1187 | `create_session` / `create_session_with_id` / `create_session_with_id_and_creator` / `create_session_with_id_and_details` | 120 |
| accessors | 1189 | 1287 | `get_session` / `cached_system_prompt` / `remember_system_prompt` / `cached_user_context` / `remember_user_context` / `clone_prompt_cache` | 99 |
| skill-agent | 1288 | 1636 | `turn_skill_agent_snapshot` / `latest_turn_skill_agent_snapshot_at_or_before` / `remember_turn_skill_agent_snapshot` / `recover_first_turn_skill_agent_snapshot` / `remember_skill_agent_baseline_override_snapshot` / `skill_agent_baseline_override_snapshot` / `seed_forked_skill_agent_listing_baselines` / `rebuild_skill_agent_listing_baseline_to_latest` / `remove_listing_diff_internal_reminders` | 349 |
| listing-diff-helpers | 1637 | 1785 | `strip_listing_diff_internal_reminders` / `listing_baseline_rebuild_turn_index_from_custom_metadata` / `listing_baseline_rebuild_turn_index_from_metadata` / `persist_context_snapshot_messages_best_effort` / `sanitize_listing_diff_context_snapshot_if_needed` / `persist_listing_baseline_rebuild_turn_index_best_effort` / `truncate_listing_baseline_rebuild_turn_index_after_rollback` | 149 |
| invalidate-cache | 1786 | 1829 | `invalidate_prompt_cache` | 44 |
| update-state | 1830 | 1976 | `reset_session_state_if_processing` / `update_session_state` / `update_session_state_for_turn_if_processing` | 147 |
| update-title-agent | 1930 | 2111 | `update_session_title` / `update_session_title_if_current` / `update_session_agent_type` / `update_last_submitted_agent_type` / `derive_last_user_dialog_agent_type_from_turns` | 182 |
| update-model | 2113 | 2194 | `update_session_model_id` / `refresh_session_context_window` | 82 |
| touch-delete | 2196 | 2354 | `touch_session` / `delete_session` | 159 |
| restore-top | 2355 | 2479 | `restore_session` / `restore_internal_session` / `restore_session_internal` / `restore_session_view` / `restore_session_view_timed` / `restore_internal_session_view` / `restore_internal_session_view_timed` / `restore_session_view_tail` / `restore_session_view_tail_timed` | 125 |
| restore-view-internal | 2481 | 2921 | `restore_session_view_internal` (440 行 monolith) | 441 |
| rollback | 2922 | 3048 | `rollback_context_to_turn_start` | 127 |
| list-sessions | 3049 | 3097 | `list_sessions` | 49 |
| metadata-persistence | 3099 | 3264 | `load_session_metadata` / `save_session_metadata` / `metadata_workspace_path_for_update` / `load_or_persist_session_metadata` / `update_session_metadata_at_workspace` / `update_persisted_session_metadata` / `merge_session_custom_metadata` / `merge_session_relationship` / `persist_session_lineage` / `collect_hidden_subagent_cascade_for_parent_turns` / `set_session_deep_review_run_manifest` | 166 |
| start-persisted-turn | 3266 | 3353 | `start_persisted_turn` | 88 |
| start-dialog-turn | 3354 | 3595 | `start_dialog_turn` / `start_dialog_turn_with_prepended_messages` / `start_dialog_turn_with_existing_context` / `start_maintenance_turn` / `append_completed_local_command_turn` | 242 |
| complete-fail-cancel-turn | 3596 | 3984 | `complete_dialog_turn` / `fail_dialog_turn` / `cancel_dialog_turn` / `complete_maintenance_turn` / `fail_maintenance_turn` | 389 |
| helpers-B | 3985 | 4240 | `get_messages` / `get_messages_paginated` / `get_context_messages` / `add_message` / `replace_context_messages` / `set_file_read_state` / `get_file_read_state` / `get_turn_count` / `get_compression_state` / `update_compression_state` / `try_generate_session_title_with_ai` / `resolve_session_title` / `generate_session_title` / `paginate_messages` | 256 |
| background-tasks | 4241 | 4361 | `spawn_auto_save_task` / `spawn_cleanup_task` | 121 |
| mod tests | 4363 | 6532 | `#[cfg(test)] mod tests { ... }`（含 `TestWorkspace`、3 个 test manager helpers、`test_model` helper、22 个 `#[test]` 函数） | 2170 |

> 关键观察：impl SessionManager block 从 line 143 一直延续到 line 4361，**没有任何嵌套 impl**（不像 coordinator.rs 有 `impl SubagentPhase2Output` 这种嵌套），所以 `mod tests` 是唯一第二个 impl-bearing block。
>
> 所有方法都是单一 `impl SessionManager { ... }` 内的 fn，**没有第二个 impl 块**。这意味着拆分后所有方法都可以直接 `impl SessionManager { ... }` 形式迁移到 split file（参考 Round 3a dialog_turn.rs 顶部 `impl ConversationCoordinator { ... }` 重开的模式）。

### 1.2 内部 `Self::` 静态方法调用矩阵（66 次）

| `Self::method` | 调用次数 | 调用方（按 line 排序） | 拆分后归属 |
|---|---|---|---|
| `is_auto_model_selector` | 3 | 160, 185, 194 | session_manager.rs（model resolution） |
| `context_window_for_model_selection` | 4 | 186, 197, 198, 155 | session_manager.rs |
| `session_context_window_from_ai_config` | 2 | 193, 205 | session_manager.rs |
| `sync_session_context_window_from_ai_config` | 4 | 205, 2142, 2183, 2738 | session_manager.rs（**pub(crate)**: 2738 在 restore.rs） |
| `load_ai_config_for_model_resolution` | 3 | 2119, 2180, 2696 | session_manager.rs（**pub(crate)**: 2696 在 restore.rs） |
| `normalize_session_title_input` | 1 | 1931 | session_manager.rs（title） |
| `normalize_whitespace` | 1 | 231 | session_manager.rs（title helper） |
| `truncate_chars` | 1 | 234 | session_manager.rs（title helper） |
| `fallback_session_title` | 2 | 4222, 4222 | session_manager.rs（title helper） |
| `should_persist_session_kind` | 1 | 308 | session_manager.rs（persistence predicate） |
| `should_persist_session` | 6 | 308, 326, 1178, 1848, 1905, 3008, 4082, 4320 | session_manager.rs（**pub(crate)**: 1178/1848/1905/3008/4082/4320 在不同 split file） |
| `same_session_version` | 2 | 346, 388 | session_manager.rs（cleanup helper，仅 background task 用） |
| `is_session_expired` | 2 | 370, 389 | session_manager.rs（cleanup helper，仅 background task 用） |
| `collect_expired_session_candidates` | 1 | 4302 | session_manager.rs（cleanup helper） |
| `cleanup_candidate_matches_session` | 2 | 399, 4340 | session_manager.rs |
| `cleanup_snapshot_for_candidate` | 3 | 4311, 4324 | session_manager.rs |
| `collect_auto_save_snapshots` | 1 | 4255 | session_manager.rs |
| `auto_save_snapshot_is_current` | 2 | 4256, 4262 | session_manager.rs |
| `auto_save_interval` | 1 | 4250 | session_manager.rs |
| `session_workspace_from_config` | 2 | 425, 1137 | session_manager.rs |
| `effective_workspace_path_from_config` | 9 | 432, 1137, 1141, 2651, 3283, 3512, 4260, 4322 | session_manager.rs（**pub(crate)**: 跨 5 个 split file） |
| `paginate_messages` | 1 | 4013 | session_manager.rs |
| `build_messages_from_turns` | 4 | 621, 2795, 2814 | **pub(crate)**（621 在 start_persisted_turn / 2795+2814 在 restore.rs） |
| `invalidate_ai_clients_for_models` | 1 | 1044 | session_evidence.rs（reconciliation） |
| `is_session_model_id_usable` | 2 | 2708 | **pub(crate)**（2708 在 restore.rs / 912 在 evidence.rs） |
| `strip_listing_diff_internal_reminders` | 2 | 1622, 1713 | **pub(crate)**（1622 在 evidence.rs / 1713 在 persistence.rs） |
| `listing_baseline_rebuild_turn_index_from_custom_metadata` | 1 | 1665 | session_evidence.rs |
| `listing_baseline_rebuild_turn_index_from_metadata` | 3 | 2676, 2941, 1765 | **pub(crate)**（2676 restore.rs / 2941 rollback / 1765 sanitize） |
| `derive_last_user_dialog_agent_type_from_turns` | 2 | 2772, 2990 | **pub(crate)**（2772 restore.rs / 2990 rollback） |
| `effective_workspace_path_from_config` (重复列出) | - | - | - |

> 总结：真正需要 `pub(crate)` 的内部静态方法只有 6 个：
> - `sync_session_context_window_from_ai_config`（line 201, fn 签名 `fn(&mut Session, &AIConfig) -> Option<usize>`）
> - `load_ai_config_for_model_resolution`（line 144, `async fn() -> Option<AIConfig>`）
> - `effective_workspace_path_from_config`（line 414, `async fn(&SessionConfig) -> Option<PathBuf>`）
> - `should_persist_session`（line 307, `fn(&Session) -> bool`）
> - `build_messages_from_turns`（line 517, `fn(&[DialogTurnData]) -> Vec<Message>`）
> - `is_session_model_id_usable`（line 912, `fn(&AIConfig, &str) -> bool`）
> - `strip_listing_diff_internal_reminders`（line 1637, `fn(Vec<Message>) -> (Vec<Message>, bool)`）
> - `listing_baseline_rebuild_turn_index_from_metadata`（line 1662, `fn(Option<&SessionMetadata>) -> Option<usize>`）
> - `derive_last_user_dialog_agent_type_from_turns`（line 2080, `fn(&[DialogTurnData], Option<&str>) -> Option<String>`）

### 1.3 `SessionManager::` 静态调用（mod tests 内部，line 4363+）

| `SessionManager::xxx` | 调用次数 | line | 拆分后归属 | 是否需要 pub(crate) |
|---|---|---|---|---|
| `new` | 3 | 4423, 4440, 4452 | session_manager.rs | 已 pub |
| `sync_session_context_window_from_ai_config` | 3 | 4493, 4523, 4532 | session_manager.rs | 已 pub(crate) 上面已标 |
| `auto_save_interval` | 1 | 4540 | session_manager.rs | private 保留（仅 mod tests 用） |
| `collect_auto_save_snapshots` | 1 | 4564 | session_manager.rs | private 保留 |
| `build_messages_from_turns` | 2 | 4743, 6017 | session_manager.rs 或 persistence.rs | private 保留（mod tests 跟 method 同文件） |
| `listing_baseline_rebuild_turn_index_from_metadata` | 2 | 5640, 5872 | session_evidence.rs | private 保留（同上） |
| `fallback_session_title` | 3 | 6025, 6035, 6045 | session_manager.rs | private 保留 |

> mod tests 跟随 `build_messages_from_turns` 归属文件走就行。如果 `build_messages_from_turns` 移到 session_persistence.rs，那 22 个测试函数（line 4477-6500）也应该跟着移动——但这样不现实，因为测试覆盖了所有 sections。
>
> **推荐方案**：把 `mod tests` 块整体留在 session_manager.rs，跨 split file 的 internal calls（`SessionManager::build_messages_from_turns` 等）需要从 `pub(crate)` 改成 `pub(super)` 或者让 mod tests 跟随 method 走。
>
> **更现实方案**：把 mod tests 内部用到的 `SessionManager::xxx` 调用都改成 `super::xxx` 通过父模块引用（session_manager.rs 的 mod tests 用 `super::SessionManager` 就行），不影响 method visibility。

### 1.4 `self.<field>` 字段访问矩阵（100+ 次）

| 字段 | 次数 | 跨 split file? | 拆分后归属 + visibility |
|---|---|---|---|
| `self.sessions` | 34 | main | session_manager.rs（必须留主文件，**pub(crate)** 给 background tasks clone） |
| `self.config` | 25 | main + 全部 | session_manager.rs（**pub(crate)**） |
| `self.persistence_manager` | 37 | main + 全部 | session_manager.rs（**pub(crate)**） |
| `self.context_store` | 16 | main + evidence + restore + persistence + background | session_manager.rs（**pub(crate)**） |
| `self.prompt_cache_store` | 13 | main + persistence + restore + background | session_manager.rs（**pub(crate)**） |
| `self.turn_skill_agent_snapshot_store` | 11 | main + evidence + restore + background | session_manager.rs（**pub(crate)**） |
| `self.skill_agent_baseline_override_snapshot_store` | 6 | main + evidence + restore + background | session_manager.rs（**pub(crate)**） |
| `self.file_read_state_store` | 8 | main + persistence + restore + background + helpers | session_manager.rs（**pub(crate)**） |
| `self.evidence_ledger` | 4 | evidence + spawn_model_reconciliation_listener | session_manager.rs（**pub(crate)**） |
| `self.session_workspace_index` | 4 | main | session_manager.rs（**pub(crate)**） |

> 关键：**所有 10 个字段都需要 `pub(crate)`**（除 `evidence_ledger` 只跨 2 个 split file 也需要）。impl 块在 split file 之间共享 struct 字段，必须 `pub(crate)`。

### 1.5 `self.<method>` 实例方法调用矩阵（关键跨 file）

| `self.method` | 次数 | 跨 split file? | 拆分后归属 + visibility |
|---|---|---|---|
| `self.should_persist_session_id` | 28 | 跨 main + evidence + persistence + restore | session_manager.rs（已 pub，无需改） |
| `self.effective_session_workspace_path` | 18 | 跨多个 split file | session_manager.rs（已 private，**需 pub(crate)**） |
| `self.context_store.replace_context` / `add_message` / `get_context_messages` / `delete_session` / `create_session` | 16 | 跨 persistence + restore | 直接通过 `self.context_store` 字段，不需要改 method visibility |
| `self.prompt_cache_store.xxx` | 13 | 跨 persistence + restore | 直接通过字段 |
| `self.turn_skill_agent_snapshot_store.xxx` | 11 | 跨 evidence + restore | 直接通过字段 |
| `self.append_evidence_event` | 2 | evidence | session_evidence.rs（已 pub） |
| `self.create_session_with_id_and_details` | 3 | create_session 链 | session_manager.rs（已 pub） |
| `self.ensure_prompt_cache_loaded` | 6 | 跨 persistence | session_persistence.rs 或 session_manager.rs（**需 pub(crate)** 取决于归属） |
| `self.persist_prompt_cache_best_effort` | 6 | 跨 persistence | session_persistence.rs（同上） |
| `self.remember_turn_skill_agent_snapshot` | 1 | evidence | session_evidence.rs（已 pub） |
| `self.remember_skill_agent_baseline_override_snapshot` | 2 | evidence | session_evidence.rs（已 pub） |
| `self.turn_skill_agent_snapshot` | 1 | evidence | session_evidence.rs（已 pub） |
| `self.get_turn_count` | 1 | evidence | session_manager.rs（已 pub） |
| `self.get_messages` | 1 | helpers | session_manager.rs（已 pub） |
| `self.merge_session_custom_metadata` | 1 | metadata helpers | session_manager.rs（已 pub） |
| `self.update_session_metadata_at_workspace` | 1 | metadata | session_manager.rs（private，**需 pub(crate)** 给 merge_session_custom_metadata 等链调用） |
| `self.update_persisted_session_metadata` | 4 | metadata + merge_session_relationship | session_manager.rs（同上） |
| `self.metadata_workspace_path_for_update` | 1 | metadata | session_manager.rs（private） |
| `self.recover_first_turn_skill_agent_snapshot` | 1 | evidence | session_evidence.rs（已 pub） |
| `self.sanitize_listing_diff_context_snapshot_if_needed` | 2 | restore + rollback | session_restore.rs 或 persistence.rs（**需 pub(crate)**） |
| `self.persist_listing_baseline_rebuild_turn_index_best_effort` | 1 | evidence | session_evidence.rs（**需 pub(crate)** 因为 persistence.rs 的 `sanitize_listing_diff_context_snapshot_if_needed` 也有用） |
| `self.persist_context_snapshot_for_turn_best_effort` | 8 | 跨 persistence | session_persistence.rs（**需 pub(crate)**） |
| `self.persist_context_snapshot_messages_best_effort` | 1 | evidence / persistence | session_persistence.rs（**需 pub(crate)**） |
| `self.persist_current_turn_context_snapshot_best_effort` | 3 | 跨 persistence | session_persistence.rs（**需 pub(crate)**） |
| `self.truncate_listing_baseline_rebuild_turn_index_after_rollback` | 1 | rollback | session_restore.rs（**需 pub(crate)**） |
| `self.restore_session` | 2 | update_session_model_id + rollback | session_restore.rs（已 pub） |
| `self.restore_session_internal` | 2 | restore 链 | session_restore.rs（private，**需 pub(crate)** 给 restore_session / restore_internal_session） |
| `self.restore_session_with_turns_internal` | 2 | restore 链 | session_restore.rs（private，**需 pub(crate)**） |
| `self.restore_session_view_internal` | 4 | restore 链 | session_restore.rs（private，**需 pub(crate)**） |
| `self.restore_session_view_timed` | 1 | restore 链 | session_restore.rs（已 pub） |
| `self.restore_session_view_tail_timed` | 1 | restore 链 | session_restore.rs（已 pub） |
| `self.restore_internal_session_view_timed` | 1 | restore 链 | session_restore.rs（已 pub） |
| `self.restore_internal_session_view_tail_timed` | 1 | restore 链 | session_restore.rs（已 pub） |
| `self.evidence_summary_for_session` | 1 | evidence | session_evidence.rs（已 pub） |
| `self.update_session_title` | 1 | title | session_manager.rs（已 pub） |
| `self.update_session_model_id` | 1 | update_model | session_manager.rs（已 pub） |

> 总结：跨 split file 需要 `pub(crate)` 的实例方法（按归属）：
>
> - **session_manager.rs** 留 main：`should_persist_session_id`（已 pub）、`effective_session_workspace_path`（private → pub(crate)）、`update_persisted_session_metadata`（private → pub(crate)）、`update_session_metadata_at_workspace`（private → pub(crate)）
> - **session_evidence.rs**：`persist_listing_baseline_rebuild_turn_index_best_effort`（private → pub(crate)）— 因为 persistence.rs 的 `sanitize_listing_diff_context_snapshot_if_needed` 也调用
> - **session_persistence.rs**：`ensure_prompt_cache_loaded`（private → pub(crate)）、`persist_prompt_cache_best_effort`（private → pub(crate)）、`persist_context_snapshot_for_turn_best_effort`（private → pub(crate)）、`persist_context_snapshot_messages_best_effort`（private → pub(crate)）、`persist_current_turn_context_snapshot_best_effort`（private → pub(crate)）、`sanitize_listing_diff_context_snapshot_if_needed`（private → pub(crate)）
> - **session_restore.rs**：`restore_session_internal`（private → pub(crate)）、`restore_session_with_turns_internal`（private → pub(crate)）、`restore_session_view_internal`（private → pub(crate)）、`truncate_listing_baseline_rebuild_turn_index_after_rollback`（private → pub(crate)）、`sanitize_listing_diff_context_snapshot_if_needed`（如果不在 persistence.rs）

### 1.6 `crate::agentic::session::xxx` 内部 import（line 5-45）

| use 行 | 内容 | 跨 split file? |
|---|---|---|
| 5-9 | `crate::agentic::core::{...}` (new_turn_id, CompressionContract, CompressionState, InternalReminderKind, Message, MessageSemanticKind, ProcessingPhase, Session, SessionConfig, SessionKind, SessionState, SessionSummary, TurnStats) | 全用，全部 split file 都需要 |
| 10 | `crate::agentic::image_analysis::ImageContextData` | session_manager.rs（仅 start_dialog_turn 用） |
| 11 | `crate::agentic::persistence::PersistenceManager` | 全用 |
| 12 | `crate::agentic::session::session_store_port::CoreSessionStorePort` | session_manager.rs（workspace + restore_view 用） |
| 13-19 | `crate::agentic::session::{CachedSystemPrompt, CachedUserContext, EvidenceLedgerCheckpoint, EvidenceLedgerEvent, EvidenceLedgerEventStatus, EvidenceLedgerSummary, EvidenceLedgerTargetKind, FileReadState, FileReadStateStore, PromptCacheLookup, PromptCachePolicy, PromptCacheScope, SessionContextStore, SessionEvidenceLedger, SessionPromptCache, SessionPromptCacheStore, SystemPromptCacheIdentity, TurnSkillAgentSnapshotStore, UserContextCacheIdentity}` | 全部 split file 都需要 |
| 20 | `crate::agentic::skill_agent_snapshot::TurnSkillAgentSnapshot` | session_evidence.rs |
| 21 | `crate::infrastructure::ai::get_global_ai_client_factory` | session_evidence.rs（reconciliation） |
| 22-25 | `crate::service::config::{get_app_language_code, get_global_config_service, short_model_user_language_instruction, subscribe_config_updates, ConfigUpdateEvent}` | session_manager.rs + session_evidence.rs（subscribe） |
| 26-29 | `crate::service::session::{DialogTurnData, DialogTurnKind, ModelRoundData, SessionMetadata, SessionRelationship, TextItemData, TurnStatus, UserMessageData}` | 全用 |
| 30 | `crate::service::snapshot::ensure_snapshot_manager_for_workspace` | session_persistence.rs |
| 31 | `crate::service::workspace::get_global_workspace_service` | session_manager.rs |
| 32 | `crate::util::errors::{NortHingError, NortHingResult}` | 全用 |
| 33 | `crate::util::sanitize_plain_model_output` | session_manager.rs（title gen 用） |
| 34 | `crate::util::timing::elapsed_ms_u64` | session_restore.rs + session_manager.rs |
| 35 | `dashmap::DashMap` | session_manager.rs（field type） |
| 36 | `pub use northhing_runtime_ports::SessionViewRestoreTiming;` | session_restore.rs（保留为 re-export） |
| 37-39 | `northhing_runtime_ports::{SessionStoragePathRequest, SessionStorePort, SessionViewRestoreRequest}` | session_restore.rs |
| 40-44 | `northhing_services_core::session::{apply_session_lineage, collect_hidden_subagent_cascade, merge_session_custom_metadata, set_deep_review_run_manifest, set_session_relationship}` | session_manager.rs（metadata persistence） |
| 45 | `serde_json::json` | session_manager.rs（title AI gen） |
| 46-52 | std / tokio / tracing 标准 use | 各 split file 按需 |

---

## 2. session_manager.rs 外部引用扫描

扫描范围：`E:/agent-project/northing/src/crates/assembly/core/src` 全树。

### 2.1 直接 import `SessionManager` 的调用方（7 个文件）

| 文件 | line | import |
|---|---|---|
| `agentic/coordination/coordinator.rs` | 37 | `use crate::agentic::session::SessionManager;` |
| `agentic/coordination/dialog_turn.rs` | 41 | `use crate::agentic::session::SessionManager;` |
| `agentic/coordination/subagent_orchestrator.rs` | 39 | `use crate::agentic::session::SessionManager;` |
| `agentic/coordination/ports.rs` | 35 | `use crate::agentic::session::SessionManager;` |
| `agentic/coordination/scheduler.rs` | 23 | `use crate::agentic::session::SessionManager;` |
| `agentic/execution/execution_engine.rs` | 31 | `use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};` |
| `agentic/goal_mode/mod.rs` | 12 | `use crate::agentic::session::SessionManager;` |

### 2.2 直接构造 `SessionManager` 的位置（4 个文件）

| 文件 | line | 代码 |
|---|---|---|
| `agentic/system.rs` | 50 | `let session_manager = Arc::new(session::SessionManager::new(...))` |
| `agentic/coordination/ports.rs` | 584, 1139 | `let session_manager = Arc::new(SessionManager::new(...))` |
| `agentic/session/session_manager.rs` | 4423, 4440, 4452 | `mod tests` 内部 3 处 |

### 2.3 `session_manager.<method>()` 实际调用矩阵（7 个不同 method）

| method | 调用方（line 排序） |
|---|---|
| `get_session` | `service_agent_runtime.rs:362`, `coordination/subagent_orchestrator.rs:398, 1210`, `coordination/scheduler.rs:490`, `coordination/dialog_turn.rs:1540, 1920, 2076, 2451, 3356` |
| `get_turn_count` | `coordination/subagent_orchestrator.rs:689`, `coordination/dialog_turn.rs:1549, 2293, 2704` |
| `get_context_messages` | `coordination/dialog_turn.rs:2847` |
| `get_messages` | `coordination/dialog_turn.rs:3602` |
| `list_sessions` | `coordination/dialog_turn.rs:3588` |
| `reset_session_state_if_processing` | `coordination/dialog_turn.rs:3237` |
| `should_persist_session_id` | `coordination/dialog_turn.rs:1342` |

> 这 7 个 method 全部已 `pub`，**拆分不需要改 visibility**。

### 2.4 `SessionViewRestoreTiming` 的引用（pub use re-export）

| 文件 | line | 引用形式 |
|---|---|---|
| `session_manager.rs:36` | - | `pub use northhing_runtime_ports::SessionViewRestoreTiming;` |
| `agentic/coordination/dialog_turn.rs` | 3498, 3525, 3549, 3576 | `crate::agentic::session::session_manager::SessionViewRestoreTiming` |

> **关键**：外部代码用 `crate::agentic::session::session_manager::SessionViewRestoreTiming`（不是 `crate::agentic::session::SessionViewRestoreTiming`）。
>
> 这意味着 `SessionViewRestoreTiming` 的 re-export 必须**留在 session_manager.rs**（主文件），不能移到 session_restore.rs。
>
> 否则需要协调外部 4 个引用点全部改成 `crate::agentic::session::SessionViewRestoreTiming`（这个路径在 mod.rs line 25 已经有 `pub use`，但外部仍然用旧路径）。

### 2.5 外部 import 统计

- 直接 import `SessionManager` 类型：**7 个文件**（见 2.1）
- 构造 `SessionManager::new()`：**4 个文件**（见 2.2）
- 调用 `session_manager.xxx()`：**5 个文件**（dialog_turn.rs 是主要调用方）
- 使用 `SessionViewRestoreTiming`：**1 个文件**（dialog_turn.rs 的 4 个位置）
- **总外部调用方**：7 个 import 文件 + 1 个 test-only 引用 = 8 个文件

---

## 3. 预期 visibility 改动清单（**核心**）

### 3.1 Type 改动

| Item | 当前行号 | 当前可见性 | 建议改为 | 依据 |
|---|---|---|---|---|
| `SessionManager` struct | 104 | `pub` | `pub`（不动） | 7 个外部调用方 |
| `SessionManagerConfig` struct | 56 | `pub` | `pub`（不动） | `coordination/ports.rs:590, 1145` 构造时用 |
| `SessionTitleMethod` enum | 77 | `pub` | `pub`（不动） | 外部不直接用，留 main file 即可 |
| `ResolvedSessionTitle` struct | 92 | `pub` | `pub`（不动） | `resolve_session_title` 返回类型，外部可能用 |
| `SessionAutoSaveSnapshot` struct | 129 | private | **private 保留** | 仅 `collect_auto_save_snapshots` + `auto_save_snapshot_is_current` 用，留在 main file |
| `SessionCleanupCandidate` struct | 137 | private | **private 保留** | 仅 `collect_expired_session_candidates` + `cleanup_*` 用，留在 main file |

### 3.2 Struct 字段改动（**所有 10 个字段需要 pub(crate)**）

| Item | 当前行号 | 当前可见性 | 建议改为 | 依据 |
|---|---|---|---|---|
| `sessions` | 105 | private | **pub(crate)** | 34 次访问，跨 5+ split file（main + background tasks + evidence + restore） |
| `session_workspace_index` | 110 | private | **pub(crate)** | 4 次访问，跨 2+ split file（main + update_session_model_id） |
| `context_store` | 113 | private | **pub(crate)** | 16 次访问，跨 4+ split file（main + evidence + persistence + restore） |
| `prompt_cache_store` | 114 | private | **pub(crate)** | 13 次访问，跨 3+ split file（main + persistence + restore） |
| `turn_skill_agent_snapshot_store` | 115 | private | **pub(crate)** | 11 次访问，跨 3+ split file（main + evidence + restore + background） |
| `skill_agent_baseline_override_snapshot_store` | 116 | private | **pub(crate)** | 6 次访问，跨 3+ split file（main + evidence + restore + background） |
| `file_read_state_store` | 117 | private | **pub(crate)** | 8 次访问，跨 4+ split file（main + create + restore + replace_context + background） |
| `evidence_ledger` | 118 | private | **pub(crate)** | 4 次访问，跨 2 split file（evidence + spawn_model_reconciliation_listener） |
| `persistence_manager` | 119 | private | **pub(crate)** | 37 次访问，**全部 split file** 都用 |
| `config` | 122 | private | **pub(crate)** | 25 次访问，**全部 split file** 都用 |

### 3.3 静态方法（`Self::`）改动清单

#### 3.3.1 保持 private（不需跨 split file）

| Item | 当前行号 | 当前可见性 | 决策依据 |
|---|---|---|---|
| `is_auto_model_selector` | 150 | private | 仅 model resolution helpers 用，4 处 self call，归属 session_manager.rs |
| `context_window_for_model_selection` | 155 | private | 同上 |
| `session_context_window_from_ai_config` | 173 | private | 同上 |
| `normalize_session_title_input` | 210 | private | 仅 title helper，归属 session_manager.rs |
| `normalize_whitespace` | 221 | private | 同上 |
| `truncate_chars` | 225 | private | 同上 |
| `fallback_session_title` | 229 | private | 同上 + mod tests（line 6025-6045） |
| `paginate_messages` | 271 | private | 仅 get_messages_paginated 用，归属 session_manager.rs |
| `session_workspace_from_config` | 296 | private | 仅 workspace helpers，归属 session_manager.rs |
| `should_persist_session_kind` | 300 | private | 仅 should_persist_session 用，归属 session_manager.rs |
| `same_session_version` | 311 | private | 仅 background tasks 用，归属 session_manager.rs |
| `is_session_expired` | 355 | private | 同上 |
| `collect_expired_session_candidates` | 361 | private | 同上 |
| `cleanup_candidate_matches_session` | 382 | private | 同上 |
| `cleanup_snapshot_for_candidate` | 392 | private | 同上 |
| `collect_auto_save_snapshots` | 319 | private | 同上 |
| `auto_save_snapshot_is_current` | 339 | private | 同上 |
| `auto_save_interval` | 351 | private | 同上 |
| `session_workspace_path` | 422 | private | `#[allow(dead_code)]` 保留，归属 session_manager.rs |
| `invalidate_ai_clients_for_models` | 990 | private | 仅 spawn_model_reconciliation_listener（line 1044）用，归属 session_evidence.rs |
| `listing_baseline_rebuild_turn_index_from_custom_metadata` | 1652 | private | 仅 listing_baseline_rebuild_turn_index_from_metadata 用，归属 session_evidence.rs |
| `is_session_model_id_usable` | 912 | private | 实际在 line 2708（restore.rs）也被调用 → **改为 pub(crate)**（见下） |
| `strip_listing_diff_internal_reminders` | 1637 | private | line 1622（evidence）和 line 1713（persistence）都用 → **改为 pub(crate)**（见下） |

#### 3.3.2 改为 `pub(crate)`（跨 split file）

| Item | 当前行号 | 当前可见性 | 建议改为 | 依据 |
|---|---|---|---|---|
| `sync_session_context_window_from_ai_config` | 201 | private | **pub(crate)** | `update_session_model_id:2142` + `refresh_session_context_window:2183`（main）+ `restore_session_with_turns_internal:2738`（restore.rs） |
| `load_ai_config_for_model_resolution` | 144 | private | **pub(crate)** | `update_session_model_id:2119` + `refresh_session_context_window:2180`（main）+ `restore_session_with_turns_internal:2696`（restore.rs） |
| `effective_workspace_path_from_config` | 414 | private | **pub(crate)** | 9 次调用，跨 5+ split file（main + restore + start_persisted_turn + append_completed_local_command_turn + background tasks） |
| `should_persist_session` | 307 | private | **pub(crate)** | 6 次调用，跨 5 split file（spawn_cleanup_task + create_session + update_session_state + rollback + update_compression_state） |
| `is_session_model_id_usable` | 912 | private | **pub(crate)** | `migrate_sessions_off_invalidated_models`（evidence.rs 内部 line 912 不用 + restore.rs line 2708 跨调用） |
| `build_messages_from_turns` | 517 | private | **pub(crate)** | `start_persisted_turn:621`（persistence.rs）+ `restore_session_with_turns_internal:2795, 2814`（restore.rs） |
| `strip_listing_diff_internal_reminders` | 1637 | private | **pub(crate)** | `remove_listing_diff_internal_reminders:1622`（evidence.rs）+ `sanitize_listing_diff_context_snapshot_if_needed:1713`（persistence.rs 或 restore.rs） |
| `listing_baseline_rebuild_turn_index_from_metadata` | 1662 | private | **pub(crate)** | `restore_session_with_turns_internal:2676`（restore.rs）+ `rollback_context_to_turn_start:2941`（rollback）+ `sanitize_listing_diff_context_snapshot_if_needed:1765`（persistence.rs 或 restore.rs） |
| `derive_last_user_dialog_agent_type_from_turns` | 2080 | private | **pub(crate)** | `restore_session_with_turns_internal:2772`（restore.rs）+ `rollback_context_to_turn_start:2990`（rollback） |

### 3.4 实例方法改动清单

#### 3.4.1 保持 private（仅同 split file 内部用）

| Item | 当前行号 | 当前可见性 | 决策依据 |
|---|---|---|---|
| `effective_workspace_path_from_config`（已列在 3.3.2） | - | - | - |
| `load_turn_skill_agent_snapshot_from_persistence` | 721 | private | 仅 `ensure_prompt_cache_loaded` 用，归属 session_persistence.rs |
| `load_prompt_cache_from_persistence` | 732 | private | 同上 |
| `migrate_sessions_off_invalidated_models` | 932 | private | 仅 `spawn_model_reconciliation_listener` 用，归属 session_evidence.rs |
| `restore_session_internal` | 2374 | private | 实际被 `restore_session:2361` 和 `restore_internal_session:2370` 跨调用 → **改 pub(crate)** |
| `restore_session_with_turns_internal` | 2634 | private | 实际被 `restore_session_with_turns:2621` 和 `restore_internal_session_with_turns:2630` 跨调用 → **改 pub(crate)** |
| `restore_session_view_internal` | 2481 | private | 被 8 个 `restore_*_view*` 公共方法跨调用 → **改 pub(crate)** |
| `start_persisted_turn` | 3269 | private | 被 5 个 `start_dialog_turn*` / `start_maintenance_turn` / `append_completed_local_command_turn` 跨调用 → **改 pub(crate)** |
| `try_generate_session_title_with_ai` | 4106 | private | 仅 `resolve_session_title` 用，归属 session_manager.rs |
| `metadata_workspace_path_for_update` | 3119 | private | 仅 `update_persisted_session_metadata:3187-3197` 链调用，归属 session_manager.rs |
| `load_or_persist_session_metadata` | 3140 | private | 仅 `update_session_metadata_at_workspace` 用 |
| `update_session_metadata_at_workspace` | 3172 | private | 仅 `update_persisted_session_metadata` 用 |
| `update_persisted_session_metadata` | 3187 | private | 被 `merge_session_custom_metadata` + `merge_session_relationship` + `persist_session_lineage` + `set_session_deep_review_run_manifest` 4 个 pub 方法调用 → **改 pub(crate)** |
| `recover_first_turn_skill_agent_snapshot` | 1423 | private | 仅 `rebuild_skill_agent_listing_baseline_to_latest:1604` 用，归属 session_evidence.rs |
| `sanitize_listing_diff_context_snapshot_if_needed` | 1694 | private | 被 `rollback_context_to_turn_start:2961`（rollback 跨调用）→ **改 pub(crate)** |
| `truncate_listing_baseline_rebuild_turn_index_after_rollback` | 1754 | private | 被 `rollback_context_to_turn_start:3019-3040` 跨调用 → **改 pub(crate)** |
| `persist_listing_baseline_rebuild_turn_index_best_effort` | 1733 | private | 被 `rebuild_skill_agent_listing_baseline_to_latest:1606`（evidence）+ `sanitize_listing_diff_context_snapshot_if_needed`（persistence）跨调用 → **改 pub(crate)** |
| `persist_context_snapshot_messages_best_effort` | 1670 | private | 仅 mod tests + `persist_listing_baseline_rebuild_turn_index_best_effort` 用，归属 session_persistence.rs |
| `persist_context_snapshot_for_turn_best_effort` | 635 | private | 被 `complete_dialog_turn:3689` + `fail_dialog_turn:3759` + `start_persisted_turn:3309` 跨调用 → **改 pub(crate)** |
| `persist_current_turn_context_snapshot_best_effort` | 666 | private | 被 `add_message:4027` + `replace_context_messages:4037` + `start_persisted_turn:3320-3335` 跨调用 → **改 pub(crate)** |
| `ensure_prompt_cache_loaded` | 687 | private | 被 `cached_system_prompt:1199` + `cached_user_context:1236` + `clone_prompt_cache:1273` + `remember_*_prompt:1224,1261` 跨调用 → **改 pub(crate)** |
| `persist_prompt_cache_best_effort` | 766 | private | 被 `cached_*_prompt` 4 个 + `clone_prompt_cache` + `invalidate_prompt_cache:1813` 跨调用 → **改 pub(crate)** |
| `spawn_auto_save_task` | 4244 | private | 仅 `new:827` 用，归属 session_manager.rs |
| `spawn_cleanup_task` | 4283 | private | 同上 |
| `spawn_model_reconciliation_listener` | 1001 | private | 同上 |
| `load_or_persist_session_metadata`（已列） | - | - | - |
| `update_session_metadata_at_workspace`（已列） | - | - | - |
| `metadata_workspace_path_for_update`（已列） | - | - | - |
| `reset_session_state_if_processing` | 1813 | private | **已 pub**（line 1813） |
| `effective_session_workspace_path` | 430 | private | 被 18 个 split file 跨调用 → **改 pub(crate)** |

#### 3.4.2 保持 `pub`（外部或 mod tests 用）

| Item | 当前行号 | 决策依据 |
|---|---|---|
| `new` | 805 | 4 个外部构造点 |
| `append_evidence_event` | 835 | pub，未被外部直接调，但 mod tests 用 |
| `record_checkpoint_created` | 839 | pub |
| `evidence_events_for_turn` | 852 | pub |
| `evidence_summary_for_session` | 860 | pub（被 compression_contract_for_session + restore_session_view 用） |
| `compression_contract_for_session` | 868 | pub |
| `record_subagent_partial_timeout` | 878 | pub |
| `create_session` 系列 4 个 | 1071-1187 | pub，外部主要构造入口 |
| `get_session` | 1190 | pub，7 个外部调用 |
| `cached_system_prompt` / `remember_system_prompt` | 1194, 1218 | pub |
| `cached_user_context` / `remember_user_context` | 1231, 1255 | pub |
| `clone_prompt_cache` | 1268 | pub |
| `turn_skill_agent_snapshot` | 1288 | pub |
| `latest_turn_skill_agent_snapshot_at_or_before` | 1335 | pub |
| `remember_turn_skill_agent_snapshot` | 1387 | pub |
| `recover_first_turn_skill_agent_snapshot` | 1423 | pub（已 pub） |
| `remember_skill_agent_baseline_override_snapshot` | 1472 | pub |
| `skill_agent_baseline_override_snapshot` | 1506 | pub |
| `seed_forked_skill_agent_listing_baselines` | 1545 | pub |
| `rebuild_skill_agent_listing_baseline_to_latest` | 1576 | pub |
| `remove_listing_diff_internal_reminders` | 1615 | pub |
| `invalidate_prompt_cache` | 1786 | pub |
| `reset_session_state_if_processing` | 1813 | pub，外部 1 处调用（dialog_turn.rs:3237） |
| `update_session_state` 系列 | 1834, 1876 | pub |
| `update_session_title` 系列 | 1930, 1977 | pub |
| `update_session_agent_type` | 2006 | pub |
| `update_last_submitted_agent_type` | 2046 | pub |
| `update_session_model_id` | 2114 | pub |
| `refresh_session_context_window` | 2179 | pub |
| `touch_session` | 2197 | pub |
| `delete_session` | 2204 | pub |
| `restore_session` 系列 8 个 | 2356-2478 | pub（外部 dialog_turn.rs 可能用） |
| `rollback_context_to_turn_start` | 2922 | pub |
| `list_sessions` | 3049 | pub，外部 1 处（dialog_turn.rs:3588） |
| `load_session_metadata` / `save_session_metadata` | 3099, 3109 | pub |
| `merge_session_custom_metadata` 系列 5 个 | 3201-3263 | pub |
| `start_dialog_turn` 系列 5 个 | 3357-3500 | pub |
| `append_completed_local_command_turn` | 3501 | pub |
| `complete_dialog_turn` / `fail_dialog_turn` / `cancel_dialog_turn` | 3596-3849 | pub |
| `start_maintenance_turn` / `complete_maintenance_turn` / `fail_maintenance_turn` | 3474-3984 | pub |
| `get_messages` 系列 4 个 | 3990-4034 | pub |
| `set_file_read_state` / `get_file_read_state` | 4041, 4046 | pub |
| `get_turn_count` | 4055 | pub，4 个外部调用 |
| `get_compression_state` / `update_compression_state` | 4063, 4070 | pub |
| `resolve_session_title` / `generate_session_title` | 4193, 4230 | pub |

---

## 4. 拆分后子模块结构建议

### 4.1 4 文件拆分（推荐）

基于 cross-reference 矩阵（重点关注 self.<field> 字段访问和 self.<method> 跨 split file 调用），最优拆法是按"职责区域"切分，让共享字段（`self.sessions`、`self.persistence_manager`、`self.context_store` 等）只通过 `pub(crate)` 字段暴露。

```
src/crates/assembly/core/src/agentic/session/
├── mod.rs                      (facade + pub use 重导出，~30 行)
├── session_manager.rs          (struct + Config + 核心 CRUD + 状态更新 + title + background tasks + mod tests, ~3700 行)
├── session_evidence.rs         (evidence ledger + skill agent snapshot + listing diff + reconciliation, ~1000 行)
├── session_persistence.rs      (prompt cache + load/save + start_persisted_turn + dialog turn lifecycle + maintenance, ~1500 行)
└── session_restore.rs          (restore_session 系列 + rollback + view restore, ~900 行)
```

### 4.2 文件职责分配

#### session_manager.rs（**主文件，含 struct + mod tests**）

**保留**（按 line 排序）：

| line 段 | 内容 | 备注 |
|---|---|---|
| 1-142 | module doc + use 块 + 3 个 pub type + `SessionManager` struct + 2 个 private helper struct | 头 |
| 56-90 | `SessionManagerConfig` / `SessionTitleMethod` | config |
| 92-102 | `ResolvedSessionTitle` | title |
| 104-127 | `SessionManager` struct (10 fields 全 `pub(crate)`) | struct |
| 129-141 | `SessionAutoSaveSnapshot` / `SessionCleanupCandidate` | private helper struct |
| 144-208 | `load_ai_config_for_model_resolution` + `is_auto_model_selector` + `context_window_for_model_selection` + `session_context_window_from_ai_config` + `sync_session_context_window_from_ai_config` | model resolution helpers（**pub(crate)**） |
| 210-268 | `normalize_session_title_input` + `normalize_whitespace` + `truncate_chars` + `fallback_session_title` | title helpers（private） |
| 271-294 | `paginate_messages` | helper（private） |
| 296-318 | `session_workspace_from_config` + `should_persist_session_kind` + `should_persist_session` | persistence predicates（**should_persist_session pub(crate)**） |
| 319-403 | `collect_auto_save_snapshots` + `auto_save_snapshot_is_current` + `auto_save_interval` + `is_session_expired` + `collect_expired_session_candidates` + `cleanup_candidate_matches_session` + `cleanup_snapshot_for_candidate` | background task helpers（private） |
| 404-515 | workspace resolution 系列 | workspace helpers |
| 805-833 | `new` | constructor |
| 1067-1187 | `create_session` 4 个 | create session |
| 1189-1192 | `get_session` | accessor（pub） |
| 1830-1976 | `update_session_state` 系列 | update state |
| 1977-2111 | `update_session_title` 系列 + `derive_last_user_dialog_agent_type_from_turns`（**pub(crate)**） | title + agent |
| 2113-2194 | `update_session_model_id` + `refresh_session_context_window` | update model |
| 2196-2354 | `touch_session` + `delete_session` | touch + delete |
| 3049-3097 | `list_sessions` | list（pub） |
| 3099-3264 | `load_session_metadata` / `save_session_metadata` + 4 个 private helpers + 5 个 `merge_session_*` pub methods | metadata |
| 3985-4060 | `get_messages` / `get_messages_paginated` / `get_context_messages` / `add_message` / `replace_context_messages` / `set_file_read_state` / `get_file_read_state` / `get_turn_count` / `get_compression_state` | helpers + accessors |
| 4062-4104 | `update_compression_state` | compression |
| 4106-4238 | `try_generate_session_title_with_ai` + `resolve_session_title` + `generate_session_title` | title gen |
| 4241-4361 | `spawn_auto_save_task` + `spawn_cleanup_task` | background tasks |
| 4363-6532 | `mod tests` | tests |

#### session_evidence.rs

**搬迁**（line 段）：

| line 段 | 内容 | 备注 |
|---|---|---|
| 835-903 | `append_evidence_event` + `record_checkpoint_created` + `evidence_events_for_turn` + `evidence_summary_for_session` + `compression_contract_for_session` + `record_subagent_partial_timeout` | evidence methods（pub） |
| 905-1000 | `is_session_model_id_usable` + `migrate_sessions_off_invalidated_models` + `invalidate_ai_clients_for_models` | reconciliation（**is_session_model_id_usable pub(crate)**） |
| 1288-1422 | `turn_skill_agent_snapshot` + `latest_turn_skill_agent_snapshot_at_or_before` + `remember_turn_skill_agent_snapshot` | skill agent snapshot（pub） |
| 1423-1471 | `recover_first_turn_skill_agent_snapshot` | skill agent snapshot（pub） |
| 1472-1543 | `remember_skill_agent_baseline_override_snapshot` + `skill_agent_baseline_override_snapshot` | skill agent override（pub） |
| 1545-1574 | `seed_forked_skill_agent_listing_baselines` | fork seed（pub） |
| 1576-1636 | `rebuild_skill_agent_listing_baseline_to_latest` + `remove_listing_diff_internal_reminders` | rebuild（pub） |
| 1637-1651 | `strip_listing_diff_internal_reminders` | listing diff helper（**pub(crate)**） |
| 1652-1661 | `listing_baseline_rebuild_turn_index_from_custom_metadata` | listing diff helper（private） |
| 1662-1670 | `listing_baseline_rebuild_turn_index_from_metadata` | listing diff helper（**pub(crate)**） |
| 1733-1753 | `persist_listing_baseline_rebuild_turn_index_best_effort` | listing diff persist（**pub(crate)**） |
| 1754-1785 | `truncate_listing_baseline_rebuild_turn_index_after_rollback` | listing diff truncate（**pub(crate)**） |
| 1001-1066 | `spawn_model_reconciliation_listener` | background task |

#### session_persistence.rs

**搬迁**（line 段）：

| line 段 | 内容 | 备注 |
|---|---|---|
| 517-609 | `build_messages_from_turns` | message builder（**pub(crate)**） |
| 612-680 | `persist_context_snapshot_for_turn_best_effort` + `persist_current_turn_context_snapshot_best_effort` | persist context（**pub(crate)**） |
| 687-803 | `ensure_prompt_cache_loaded` + `load_turn_skill_agent_snapshot_from_persistence` + `load_prompt_cache_from_persistence` + `persist_prompt_cache_best_effort` | prompt cache load/persist（**pub(crate)**） |
| 1194-1267 | `cached_system_prompt` + `remember_system_prompt` + `cached_user_context` + `remember_user_context` | prompt cache accessors（pub） |
| 1268-1287 | `clone_prompt_cache` | prompt cache clone（pub） |
| 1670-1693 | `persist_context_snapshot_messages_best_effort` | persist messages（private） |
| 1694-1732 | `sanitize_listing_diff_context_snapshot_if_needed` | sanitize（**pub(crate)**） |
| 1786-1829 | `invalidate_prompt_cache` | invalidate（pub） |
| 3266-3353 | `start_persisted_turn` | turn start（**pub(crate)**） |
| 3354-3500 | `start_dialog_turn` + `start_dialog_turn_with_prepended_messages` + `start_dialog_turn_with_existing_context` | dialog turn（pub） |
| 3474-3500 | `start_maintenance_turn` | maintenance（pub） |
| 3501-3595 | `append_completed_local_command_turn` | local command（pub） |
| 3596-3712 | `complete_dialog_turn` | complete turn（pub） |
| 3713-3783 | `fail_dialog_turn` | fail turn（pub） |
| 3784-3849 | `cancel_dialog_turn` | cancel turn（pub） |
| 3845-3912 | `complete_maintenance_turn` | complete maintenance（pub） |
| 3913-3984 | `fail_maintenance_turn` | fail maintenance（pub） |

#### session_restore.rs

**搬迁**（line 段）：

| line 段 | 内容 | 备注 |
|---|---|---|
| 2355-2384 | `restore_session` + `restore_internal_session` + `restore_session_internal` | restore（**restore_session_internal pub(crate)**） |
| 2386-2480 | `restore_session_view` + `restore_session_view_timed` + `restore_internal_session_view` + `restore_internal_session_view_timed` + `restore_session_view_tail` + `restore_session_view_tail_timed` + `restore_internal_session_view_tail` + `restore_internal_session_view_tail_timed` | view restore（pub） |
| 2481-2616 | `restore_session_view_internal` 前半 | view restore internal（**pub(crate)**） |
| 2616-2633 | `restore_session_with_turns` + `restore_internal_session_with_turns` | restore with turns（pub） |
| 2634-2921 | `restore_session_with_turns_internal` + `restore_session_view_internal` 后半 | restore internal（**pub(crate)**） |
| 2922-3048 | `rollback_context_to_turn_start` | rollback（pub） |

### 4.3 哪个文件持有最多 pub(crate) item

**session_manager.rs 是 boundary file**：
- 持有 10 个 `pub(crate)` struct 字段（必须）
- 持有 `Self::effective_workspace_path_from_config`（被 5+ split file 调用，最热）
- 持有 `Self::should_persist_session`（被 5+ split file 调用）
- 持有 `Self::load_ai_config_for_model_resolution` / `Self::sync_session_context_window_from_ai_config`（被 2 split file 调用）
- 持有 `update_persisted_session_metadata`（被 4 个 pub metadata methods 跨调用）

**session_persistence.rs 是次要 boundary**：
- 持有 `Self::build_messages_from_turns`（被 2 split file 调用）
- 持有 `Self::effective_workspace_path_from_config` 调用方（line 3283, 3512）
- 持有 `self.ensure_prompt_cache_loaded` / `self.persist_prompt_cache_best_effort`（被多个 pub prompt cache methods 跨调用）
- 持有 `self.persist_context_snapshot_for_turn_best_effort`（被 3 个 dialog turn methods 跨调用）

**session_evidence.rs 是 self-contained**：
- 主要依赖 `Self::*` 在 evidence 内部（line 1622, 1665, 1044）
- 只有 `is_session_model_id_usable`（line 912）被 restore.rs 跨调用 → `pub(crate)`
- 只有 `listing_baseline_rebuild_turn_index_from_metadata`（line 1662）被 restore.rs + persistence.rs 跨调用 → `pub(crate)`
- 只有 `strip_listing_diff_internal_reminders`（line 1637）被 persistence.rs 跨调用 → `pub(crate)`

**session_restore.rs 跨调用最复杂**：
- 跨调用 `session_evidence.rs` 的 `is_session_model_id_usable` / `listing_baseline_rebuild_turn_index_from_metadata` / `strip_listing_diff_internal_reminders`
- 跨调用 `session_manager.rs` 的 `load_ai_config_for_model_resolution` / `sync_session_context_window_from_ai_config` / `effective_workspace_path_from_config` / `should_persist_session` / `effective_session_workspace_path`
- 跨调用 `session_persistence.rs` 的 `build_messages_from_turns` / `sanitize_listing_diff_context_snapshot_if_needed` / `persist_listing_baseline_rebuild_turn_index_best_effort`
- 内部多个 `restore_*_internal` 方法相互跨调用 → 都需要 `pub(crate)`

### 4.4 mod.rs facade 设计

参考 Round 3a 样板的 `coordination/mod.rs` 模式：

```rust
//! Session Management Layer
//!
//! Provides session lifecycle management and context management.
//!
//! Round 3b of the debt-reduction plan split session_manager.rs (6505 lines)
//! into 4 files by responsibility region:
//! - session_manager.rs: struct + accessors + Config + title + model resolution
//!   + background tasks + mod tests
//! - session_evidence.rs: evidence ledger + skill agent snapshot + listing diff
//!   + reconciliation
//! - session_persistence.rs: prompt cache + load/save + dialog turn lifecycle
//!   + maintenance
//! - session_restore.rs: restore_session family + rollback + view restore
//!
//! Re-exports keep the `crate::agentic::session::*` public path unchanged for
//! all 7+ external callers.

mod session_manager;
mod session_evidence;
mod session_persistence;
mod session_restore;

pub use self::session_manager::*;
pub use self::session_evidence::*;
pub use self::session_persistence::*;
pub use self::session_restore::*;

// Pre-existing sibling modules unchanged
pub mod compression;
pub mod context_store;
pub mod evidence_ledger;
pub mod file_read_state;
pub mod prompt_cache;
pub mod session_store_port;
pub mod turn_skill_agent_snapshot_store;

pub use compression::*;
pub use context_store::*;
pub use evidence_ledger::*;
pub use file_read_state::*;
pub use prompt_cache::*;
pub use session_store_port::*;
pub use turn_skill_agent_snapshot_store::*;

pub use northhing_runtime_ports::{
    SessionStorageKind, SessionStoragePathRequest, SessionStoragePathResolution,
    SessionTurnLoadTiming, SessionViewRestoreRequest, SessionViewRestoreTiming,
};
```

### 4.5 不变项

- `SessionViewRestoreTiming` 的 re-export **必须留在 session_manager.rs**（外部 dialog_turn.rs 用 `crate::agentic::session::session_manager::SessionViewRestoreTiming` 引用）
- `mod tests` 整体留在 session_manager.rs（避免 22 个测试函数跨文件搬运）
- `SessionManager` struct definition 留在 session_manager.rs（Rust 单一 struct 只能在一个地方定义）

---

## 5. 拆分后的 use 块预判

参考 Round 3a 样板的 use 块（dialog_turn.rs / subagent_orchestrator.rs 头部 use 完全相同，每个 split file 顶部重复 use crate 的东西；`use super::coordinator::*;` 在文件中部 line 184+ 出现）。

### 5.1 session_manager.rs 的 use 块

```rust
//! Session Manager: struct + Config + accessors + state updates + title +
//! model resolution + background tasks + mod tests
//!
//! (lines 1-142, 144-403, 805-833, 1067-1187, 1189-1192, 1830-2354,
//!  3049-3264, 3985-4361, 4363-6532 of original session_manager.rs)

use crate::agentic::core::{
    CompressionState, ProcessingPhase, Session, SessionConfig, SessionKind, SessionState,
    SessionSummary,
};
use crate::agentic::persistence::PersistenceManager;
use crate::agentic::session::{
    CachedSystemPrompt, CachedUserContext, EvidenceLedgerEvent, EvidenceLedgerEventStatus,
    EvidenceLedgerTargetKind, FileReadState, FileReadStateStore, PromptCacheLookup,
    PromptCachePolicy, PromptCacheScope, SessionContextStore, SessionEvidenceLedger,
    SessionPromptCacheStore, SystemPromptCacheIdentity, UserContextCacheIdentity,
};
use crate::infrastructure::ai::get_global_ai_client_factory;
use crate::service::config::{
    get_app_language_code, get_global_config_service,
    short_model_user_language_instruction, ConfigUpdateEvent,
};
use crate::service::session::{
    DialogTurnData, DialogTurnKind, ModelRoundData, SessionMetadata, SessionRelationship,
    TextItemData, TurnStatus, UserMessageData,
};
use crate::service::workspace::get_global_workspace_service;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::sanitize_plain_model_output;
use crate::util::timing::elapsed_ms_u64;
use dashmap::DashMap;
pub use northhing_runtime_ports::SessionViewRestoreTiming;
use northhing_runtime_ports::{SessionStoragePathRequest, SessionViewRestoreRequest};
use northhing_services_core::session::{
    apply_session_lineage, collect_hidden_subagent_cascade as collect_hidden_subagent_cascade_ids,
    merge_session_custom_metadata as merge_session_custom_metadata_value,
    set_deep_review_run_manifest, set_session_relationship,
};
use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use std::time::{Duration, SystemTime};
use tokio::time;
use tracing::{debug, error, info, warn};

/// 10 个 struct 字段全 pub(crate)
pub struct SessionManager {
    pub(crate) sessions: Arc<DashMap<String, Session>>,
    pub(crate) session_workspace_index: Arc<DashMap<String, PathBuf>>,
    pub(crate) context_store: Arc<SessionContextStore>,
    pub(crate) prompt_cache_store: Arc<SessionPromptCacheStore>,
    pub(crate) turn_skill_agent_snapshot_store: Arc<TurnSkillAgentSnapshotStore>,
    pub(crate) skill_agent_baseline_override_snapshot_store: Arc<DashMap<String, TurnSkillAgentSnapshot>>,
    pub(crate) file_read_state_store: Arc<FileReadStateStore>,
    pub(crate) evidence_ledger: Arc<SessionEvidenceLedger>,
    pub(crate) persistence_manager: Arc<PersistenceManager>,
    pub(crate) config: SessionManagerConfig,
}

impl SessionManager {
    // ... 所有上述 line 段的方法
}

// mod tests 整体跟在后面
#[cfg(test)]
mod tests { ... }
```

> 注意：`SessionViewRestoreTiming` 的 `pub use` **必须保留**（外部引用）。

### 5.2 session_evidence.rs 的 use 块

```rust
//! Session evidence ledger + skill agent snapshot + listing diff + reconciliation
//!
//! (lines 835-903, 905-1066, 1288-1785 of original session_manager.rs)

use super::session_manager::SessionManager;  // 拿 pub(crate) 字段
use crate::agentic::core::{Session, SessionConfig};
use crate::agentic::session::{
    EvidenceLedgerCheckpoint, EvidenceLedgerEvent, EvidenceLedgerEventStatus,
    EvidenceLedgerSummary, EvidenceLedgerTargetKind, SessionEvidenceLedger,
};
use crate::agentic::skill_agent_snapshot::TurnSkillAgentSnapshot;
use crate::service::config::subscribe_config_updates;
use crate::util::errors::NortHingResult;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{debug, info, warn};

impl SessionManager {
    // 835-903 evidence methods
    // 905-1066 reconciliation
    // 1288-1785 skill agent snapshot + listing diff
    // 1001-1066 spawn_model_reconciliation_listener
}
```

> 关键：跨 split file 引用用 `use super::session_manager::SessionManager;`（因为是 impl SessionManager 块的一部分，SessionManager 必须在 super 模块可见）。

### 5.3 session_persistence.rs 的 use 块

```rust
//! Session persistence: prompt cache + load/save + start_persisted_turn +
//! dialog turn lifecycle + maintenance
//!
//! (lines 517-803, 1194-1287, 1670-1829, 3266-3984 of original session_manager.rs)

use super::session_manager::SessionManager;
use crate::agentic::core::{
    new_turn_id, CompressionState, InternalReminderKind, Message, MessageSemanticKind,
    ProcessingPhase, Session, SessionConfig, SessionKind, SessionState, TurnStats,
};
use crate::agentic::image_analysis::ImageContextData;
use crate::agentic::session::{
    CachedSystemPrompt, CachedUserContext, FileReadState, PromptCacheLookup,
    PromptCacheScope, SessionPromptCache, SessionPromptCacheStore,
};
use crate::agentic::persistence::PersistenceManager;
use crate::service::session::{
    DialogTurnData, DialogTurnKind, ModelRoundData, TextItemData, TurnStatus,
    UserMessageData,
};
use crate::service::snapshot::ensure_snapshot_manager_for_workspace;
use crate::util::errors::{NortHingError, NortHingResult};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time;
use tracing::{debug, info, warn};

impl SessionManager {
    // 517-609 build_messages_from_turns
    // 612-680 persist_context_snapshot
    // 687-803 prompt cache load/persist
    // 1194-1287 prompt cache accessors
    // 1670-1829 sanitize + invalidate
    // 3266-3984 dialog turn lifecycle
}
```

### 5.4 session_restore.rs 的 use 块

```rust
//! Session restore: restore_session family + rollback + view restore
//!
//! (lines 2355-2921, 2922-3048 of original session_manager.rs)

use super::session_manager::SessionManager;
use super::session_evidence::{
    is_session_model_id_usable, listing_baseline_rebuild_turn_index_from_metadata,
    strip_listing_diff_internal_reminders,
};
use super::session_persistence::{
    build_messages_from_turns, sanitize_listing_diff_context_snapshot_if_needed,
};
use crate::agentic::core::{Message, MessageContent, MessageRole, ProcessingPhase, Session};
use crate::agentic::session::{
    session_store_port::CoreSessionStorePort, SessionEvidenceLedger,
};
use crate::agentic::persistence::PersistenceManager;
use crate::service::session::{DialogTurnData, DialogTurnKind, SessionMetadata, TextItemData};
pub use northhing_runtime_ports::SessionViewRestoreTiming;
use northhing_runtime_ports::{SessionStoragePathRequest, SessionViewRestoreRequest};
use std::path::Path;
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use tokio::time;
use tracing::{debug, error, info, warn};

impl SessionManager {
    // 2355-2384 restore_session + restore_session_internal
    // 2386-2480 restore_session_view family
    // 2481-2616 restore_session_view_internal 前半
    // 2616-2633 restore_session_with_turns
    // 2634-2921 restore_session_with_turns_internal + restore_session_view_internal 后半
    // 2922-3048 rollback_context_to_turn_start
}
```

> 关键：`pub use northhing_runtime_ports::SessionViewRestoreTiming;` 在 session_restore.rs 重新导出（不删除 session_manager.rs 的同名 re-export，保持外部引用兼容）。

---

## 6. 总结报告

### 6.1 统计

- **session_manager.rs 总行数**：6505
- **impl SessionManager 块方法总数**：~111（pub + private）
- **mod tests 块测试函数**：22
- **struct 字段数**：10
- **顶级 struct/enum**：3 pub + 2 private
- **`Self::` 内部静态方法调用**：66 次
- **`SessionManager::` 静态调用**（mod tests 内部）：15 次
- **`self.<field>` 字段访问**：100+ 次
- **`self.<method>` 实例方法调用**：~150 次

### 6.2 外部引用方统计

- **直接 import `SessionManager` 的文件**：7 个
- **构造 `SessionManager::new()` 的文件**：4 个（含 mod tests）
- **调用 `session_manager.xxx()` 的文件**：5 个（dialog_turn.rs 是主调用方）
- **跨文件访问的不同 method 数**：7 个（全部已 pub）
- **`SessionViewRestoreTiming` 引用点**：4 个（全部在 dialog_turn.rs）

### 6.3 visibility 改动总数

| 类别 | 改动总数 |
|---|---|
| Struct 字段（10 个全改） | **10** |
| 静态方法 pub(crate) | **9** |
| 实例方法 pub(crate) | **~12**（effective_session_workspace_path + restore_*_internal + start_persisted_turn + persist_*_best_effort + ensure_prompt_cache_loaded + sanitize + truncate + recover + update_persisted_session_metadata + load_or_persist + update_session_metadata_at_workspace） |
| **总 visibility 改动** | **~31** |

### 6.4 拆分后文件数

- **4 个**（session_manager.rs + session_evidence.rs + session_persistence.rs + session_restore.rs）

### 6.5 风险点（给执行阶段 flag）

1. **`SessionViewRestoreTiming` 路径兼容性**：外部 dialog_turn.rs 用 `crate::agentic::session::session_manager::SessionViewRestoreTiming` 直接引用，必须保留 session_manager.rs 内的 re-export。
2. **mod tests 整体留在主文件**：22 个测试函数覆盖了所有 sections，跨文件搬运会破坏大量 test helper。mod tests 内的 `SessionManager::xxx` 调用通过 `super::SessionManager` 即可，不需要改 visibility。
3. **嵌套字段访问**：例如 `self.sessions.iter().filter_map(...)` 在 line 944、361 等地方密集使用，**所有 10 个字段都需 pub(crate)**，不能只改"用得多的"几个。
4. **impl block 必须在每个 split file 重新 `impl SessionManager { ... }`**（参考 Round 3a dialog_turn.rs / subagent_orchestrator.rs 模式），但 SessionManager struct 本身只在 session_manager.rs 定义。
5. **保持 private 不动的方法不要改 visibility**：上面 3.3.1 和 3.4.1 列出~40 个保留 private 的方法，不要误改。

### 6.6 不需要改的（验证依据）

- 所有 `pub` 方法（除 `SessionViewRestoreTiming` 的 `pub use` 外）保持 `pub`
- 所有 `Self::*` 在同 split file 内部用的静态方法（~25 个）保持 private
- 所有 `self.<field>` 在 main file 内部用的字段不需要 pub(crate)，但**统一改 pub(crate) 更安全**（避免后面 round 漏掉）
- `mod tests` 不需要 visibility 改动

---

## 7. 给执行阶段（task `execute-split`）的 checklist

执行阶段拿到这个 audit 后，应该按以下顺序 apply visibility 改动：

1. **先改 struct 字段**（10 个全 `pub(crate)`）— 一次到位
2. **再改静态方法**（9 个 `pub(crate)`）— 按上表 3.3.2
3. **再改实例方法**（~12 个 `pub(crate)`）— 按上表 3.4.1 标 "改 pub(crate)" 的项
4. **最后拆分方法到 split file**（按上表 4.2）
5. **写 4 个文件的 use 块**（按上表 5）
6. **更新 mod.rs**（按上表 4.4）
7. **跑 cargo check --workspace 验证 0 error**

### 7.1 验证命令

```bash
# 最小验证
cd E:/agent-project/northing
export PATH="/c/msys64/mingw64/bin:$PATH"
cargo check -p northhing-core --lib 2>&1 | head -50

# 完整验证
cargo check --workspace 2>&1 | head -50
cargo test -p northhing-core --lib 2>&1 | tail -30
```

### 7.2 回退方案

如果 visibility 改动后 cargo check 失败，最常见原因是：
1. 漏改某个字段（缺 pub(crate)）
2. 静态方法归属文件错（应该在 session_manager.rs 但被搬到 evidence.rs）
3. impl SessionManager block 内的方法签名改了（不应该改）

回退：先 `git stash` 当前改动，按 audit 报告逐项对照 visibility 清单，fix 后再 apply。

---

> 报告完成时间：2026-06-27 00:22
> 写入文件：`E:/agent-project/northing/docs/handoffs/2026-06-26-round3b-session-manager-visibility-audit.md`
> 行数：~580 行
