# R4 Spec: extract session LIFECYCLE methods to NEW `session_lifecycle.rs`

> 状态：草案 v1（2026-06-27 18:29 +08:00）  
> 范围：`northing` repo（Rust workspace + 前端），HEAD `9dbcb9c`（`main` 分支）  
> 前置依赖：**孤儿修复 spec**（`mod.rs` 添加 `pub mod session_evidence / session_persistence / session_restore` + 删除 session_manager.rs 的 64 个重复方法体）必须先落地  
> 目标：把 session_manager.rs 中 **LIFECYCLE** 域方法移到一个新的 `session_lifecycle.rs` 文件  
> 用户指示：尽可能分块到单个文件最简，可以增大文件量（`docs/handoffs/2026-06-27-r4-comprehensive-cleanup-plan.md`）

---

## 0. 背景与前置

### 0.1 当前状态（2026-06-27 18:11 测量，详见 `~/.mavis/plans/plan_8b640472/outputs/visibility-auditor/deliverable.md`）

| 文件 | 行数 | 编译状态 | 备注 |
|---|---|---|---|
| `session_manager.rs` | **6,532** | ✅ 是当前唯一可执行 impl | 138 个 `impl SessionManager` 方法 |
| `session_evidence.rs` | 749 | ❌ 孤儿（`mod.rs` 未声明） | 25 个 `impl SessionManager` 方法，与 manager 重复 |
| `session_persistence.rs` | 1,272 | ❌ 孤儿 | 28 个 `impl SessionManager` 方法，与 manager 重复 |
| `session_restore.rs` | 757 | ❌ 孤儿 | 16 个 `impl SessionManager` 方法，与 manager 重复 |
| `mod.rs` | 26 | ✅ | 只声明 8 个模块，3 个 sibling 不在 |

`session_manager.rs` impl 块范围：L143–L4361（4,218 行）。138 个 fn 签名详见 `outputs/duplicate-scanner/session_impl_analysis.md`。

### 0.2 依赖关系

R3/R4 系列拆分的前提：**Rust 允许同一类型在多个 sibling 模块中各开 `impl` 块**。本 spec 严格遵守此约束 — 字段访问、关联函数调用、私有 helper 调用通过以下机制实现：

- 字段访问：`self.sessions`、`self.session_workspace_index` 等是 `pub(self)` 字段，从 sibling 模块访问需 `pub(super)` 或 `pub(crate)`（当前字段已是 private 但 Rust 同 crate 可见） — **当前字段未标注可见性**（line 104–126 全是 `pub struct SessionManager { <field>: Type }`，没 `pub` 也没 `pub(super)`）。详细见 §5 风险点。
- `Self::xxx()` 关联函数（如 `Self::session_workspace_from_config`）跨 impl 块天然可见，无需改可见性。
- `use` 导入：需要在新文件中重新导入同样的 crate-level 符号。
- `crate::service::cron::get_global_cron_service`、`crate::service::terminal::TerminalApi`：是 crate-level 绝对路径，跨 impl 块无差异。

### 0.3 本 spec 目标（重述）

- 把 8 个 LIFECYCLE 方法（create / get / touch / delete / list）+ 2 个对应的单元测试 移到新的 `session_lifecycle.rs` 文件
- 保留 `session_manager.rs` 中所有其它方法 + 所有 helper 关联函数（`session_workspace_from_config`、`should_persist_session` 等）+ 所有不属于 LIFECYCLE 域的测试
- session_manager.rs 的 impl SessionManager 块从 138 个方法减到 130 个
- mod.rs 添加 `pub mod session_lifecycle;` + `pub use session_lifecycle::*;`
- 测试套件保持 935+ 通过

---

## 1. 识别的 LIFECYCLE 方法

### 1.1 LIFECYCLE 域分类（基于 `git grep` 与 `session_impl_analysis.md`）

| # | 函数签名 | 当前行号 | 行数 | 跨文件依赖 |
|---|---|---|---|---|
| 1 | `pub(crate) async fn create_session(&self, session_name: String, agent_type: String, config: SessionConfig) -> NortHingResult<Session>` | L1071–L1086 | **16** | 调用 fn #4 |
| 2 | `pub(crate) async fn create_session_with_id(&self, session_id: Option<String>, session_name: String, agent_type: String, config: SessionConfig) -> NortHingResult<Session>` | L1089–L1105 | **17** | 调用 fn #4 |
| 3 | `pub(crate) async fn create_session_with_id_and_creator(&self, session_id: Option<String>, session_name: String, agent_type: String, config: SessionConfig, created_by: Option<String>) -> NortHingResult<Session>` | L1108–L1125 | **18** | 调用 fn #4 |
| 4 | `pub(crate) async fn create_session_with_id_and_details(&self, session_id: Option<String>, session_name: String, agent_type: String, config: SessionConfig, created_by: Option<String>, kind: SessionKind) -> NortHingResult<Session>` | L1128–L1187 | **60** | 自包含；调用 `Self::session_workspace_from_config`、`Self::effective_workspace_path_from_config`、`Self::should_persist_session`、`self.persistence_manager.save_session`、`self.context_store.create_session`、`self.turn_skill_agent_snapshot_store.create_session`、`self.file_read_state_store.create_session` |
| 5 | `pub fn get_session(&self, session_id: &str) -> Option<Session>` | L1190–L1192 | **3** | 仅访问 `self.sessions` |
| 6 | `pub(crate) fn touch_session(&self, session_id: &str)` | L2197–L2201 | **5** | 仅访问 `self.sessions` + `SystemTime::now()` |
| 7 | `pub(crate) async fn delete_session(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<()>` | L2204–L2353 | **150** | 调用 `ensure_snapshot_manager_for_workspace`、`self.context_store.delete_session`、`self.prompt_cache_store.delete_session`、`self.turn_skill_agent_snapshot_store.delete_session`、`self.skill_agent_baseline_override_snapshot_store.remove`、`self.file_read_state_store.delete_session`、`self.persistence_manager.delete_session`、`crate::service::cron::get_global_cron_service`、`crate::service::terminal::TerminalApi::from_singleton` |
| 8 | `pub(crate) async fn list_sessions(&self, workspace_path: &Path) -> NortHingResult<Vec<SessionSummary>>` | L3049–L3097 | **49** | 调用 `self.persistence_manager.list_sessions` 或遍历 `self.sessions` |
| | **fn body 合计** | | **318** | |

**全部 8 个 fn body 占用 ~318 行**（含 4 行 `// ============ Session CRUD ============` 分隔注释）。

### 1.2 非 LIFECYCLE（留在 `session_manager.rs`）

#### STATE 域
- L1813 `reset_session_state_if_processing`
- L1834 `update_session_state`
- L1876 `update_session_state_for_turn_if_processing`
- L1930 `update_session_title`
- L1977 `update_session_title_if_current`
- L2006 `update_session_agent_type`
- L2046 `update_last_submitted_agent_type`
- L2114 `update_session_model_id`
- L2179 `refresh_session_context_window`
- L3099 `load_session_metadata`
- L3109 `save_session_metadata`
- L3201 `merge_session_custom_metadata`
- L3212 `merge_session_relationship`
- L3223 `persist_session_lineage`
- L3234 `collect_hidden_subagent_cascade_for_parent_turns`
- L3255 `set_session_deep_review_run_manifest`

#### PERSISTENCE 域（将被 `session_persistence.rs` 拆分，**不在本 spec 范围**）
- `ensure_prompt_cache_loaded`、`persist_prompt_cache_best_effort`、`cached_system_prompt`、`remember_system_prompt`、`cached_user_context`、`remember_user_context`、`clone_prompt_cache`、`persist_context_snapshot_messages_best_effort`、`invalidate_prompt_cache`、`reset_session_state_if_processing`、`start_persisted_turn`、`start_dialog_turn`、`start_dialog_turn_with_prepended_messages`、`start_dialog_turn_with_existing_context`、`start_maintenance_turn`、`append_completed_local_command_turn`、`complete_dialog_turn`、`fail_dialog_turn`、`cancel_dialog_turn`、`complete_maintenance_turn`、`fail_maintenance_turn`、`build_messages_from_turns`、`rebuild_messages_from_turns`、`persist_context_snapshot_for_turn_best_effort`、`persist_current_turn_context_snapshot_best_effort`、`load_turn_skill_agent_snapshot_from_persistence`、`load_prompt_cache_from_persistence`、`sanitize_listing_diff_context_snapshot_if_needed`

#### EVIDENCE 域（将被 `session_evidence.rs` 拆分，**不在本 spec 范围**）
- `append_evidence_event`、`record_checkpoint_created`、`evidence_events_for_turn`、`evidence_summary_for_session`、`compression_contract_for_session`、`record_subagent_partial_timeout`、`is_session_model_id_usable`、`migrate_sessions_off_invalidated_models`、`invalidate_ai_clients_for_models`、`turn_skill_agent_snapshot`、`latest_turn_skill_agent_snapshot_at_or_before`、`remember_turn_skill_agent_snapshot`、`recover_first_turn_skill_agent_snapshot`、`remember_skill_agent_baseline_override_snapshot`、`skill_agent_baseline_override_snapshot`、`seed_forked_skill_agent_listing_baselines`、`rebuild_skill_agent_listing_baseline_to_latest`、`remove_listing_diff_internal_reminders`、`strip_listing_diff_internal_reminders`、`listing_baseline_rebuild_turn_index_from_custom_metadata`、`listing_baseline_rebuild_turn_index_from_metadata`、`persist_context_snapshot_messages_best_effort`（3-way 重叠）、`persist_listing_baseline_rebuild_turn_index_best_effort`、`truncate_listing_baseline_rebuild_turn_index_after_rollback`、`spawn_model_reconciliation_listener`

#### RESTORE 域（将被 `session_restore.rs` 拆分，**不在本 spec 范围**）
- 14 个 `restore_session*` 变体 + `rollback_context_to_turn_start`

#### 基础设施（保持）
- `new` (L805)、`spawn_auto_save_task` (L4244)、`spawn_cleanup_task` (L4283)
- 所有 helper 关联函数：`session_workspace_from_config`、`should_persist_session_kind`、`should_persist_session`、`same_session_version`、`collect_auto_save_snapshots`、`auto_save_snapshot_is_current`、`auto_save_interval`、`is_session_expired`、`collect_expired_session_candidates`、`cleanup_candidate_matches_session`、`cleanup_snapshot_for_candidate`、`should_persist_session_id`、`effective_workspace_path_from_config`、`session_workspace_path`、`effective_session_workspace_path`、`resolve_session_workspace_path`、`paginate_messages`、`load_ai_config_for_model_resolution`、`is_auto_model_selector`、`context_window_for_model_selection`、`sync_session_context_window_from_ai_config`、`normalize_session_title_input`、`normalize_whitespace`、`truncate_chars`、`fallback_session_title`、`derive_last_user_dialog_agent_type_from_turns`、`try_generate_session_title_with_ai`、`resolve_session_title`、`generate_session_title`
- 消息 / 上下文辅助：`get_messages`、`get_messages_paginated`、`get_context_messages`、`add_message`、`replace_context_messages`、`set_file_read_state`、`get_file_read_state`、`get_turn_count`、`get_compression_state`、`update_compression_state`

### 1.3 对应的测试（将移动）

| 测试名 | 行号 | 行数 | 测什么 | 是否 LIFECYCLE 主测 |
|---|---|---|---|---|
| `ephemeral_child_session_is_kept_in_memory_without_persisting` | L4799–L4829 | **31** | 创建 EphemeralChild session + 验证不持久化 | ✅ 是（create + get_session + persistence_check） |
| `delete_session_removes_workspace_cache_entry` | L5935–L5970 | **36** | 创建 session → 删除 → 验证 `session_workspace_index` 已清空 | ✅ 是（create + delete + cache_state） |
| **小计** | | **67** | | |

#### 不移动的测试（虽然它们调用 create_session 作为 setup）

下列 19 个测试虽然 `body` 包含 `.create_session(...)` 调用，但**主要测试目标不是 LIFECYCLE**：

| 测试名 | 行号 | 主要测试目标 |
|---|---|---|
| `auto_save_snapshot_collection_releases_session_map_guards` | L4549 | auto-save 任务 |
| `reset_session_state_if_processing_ignores_a_newer_turn` | L4577 | state machine |
| `reset_session_state_if_processing_resets_the_matching_turn` | L4607 | state machine |
| `update_session_state_for_turn_if_processing_ignores_a_newer_turn` | L4631 | state machine |
| `update_session_state_for_turn_if_processing_updates_matching_turn` | L4665 | state machine |
| `append_completed_local_command_turn_persists_without_model_context` | L4693 | turn 持久化 |
| `restore_session_view_loads_turns_without_restoring_runtime_context` | L5054 | restore |
| `start_dialog_turn_with_existing_context_persists_turn_and_snapshot` | L5117 | turn 生命周期 |
| `restore_session_view_preserves_full_visible_tool_result_payload` | L5195 | restore |
| `rollback_context_deletes_persisted_turns_from_target` | L5320 | rollback |
| `latest_skill_agent_snapshot_scans_persistence_beyond_stale_cache_hit` | L5440 | evidence snapshot |
| `rebuild_skill_agent_listing_baseline_to_latest_removes_listing_diff_reminders` | L5532 | evidence listing baseline |
| `rollback_sanitizes_pre_cutoff_snapshot_and_truncates_cutoff` | L5750 | rollback |
| `rollback_to_empty_history_clears_last_user_dialog_agent_type` | L5878 | rollback |
| `prompt_cache_persists_across_session_restore` | L6075 | prompt cache |
| `skill_agent_baseline_override_snapshot_persists_across_session_restore` | L6133 | evidence baseline |
| `seed_forked_skill_agent_listing_baselines_splits_prompt_and_diff_baselines` | L6191 | evidence |
| `prompt_cache_invalidation_removes_persisted_entries` | L6290 | prompt cache |
| `clone_prompt_cache_copies_runtime_and_persisted_entries` | L6359 | prompt cache |

这些测试**保留在 session_manager.rs 的 `#[cfg(test)] mod tests`** 模块中。它们通过 sibling `impl SessionManager` 块继续可以调用 `manager.create_session(...)`（Rust 允许从同 crate 的其它位置调用同类型方法）。

---

## 2. 迁移步骤

### Step 0：前置 — 落地孤儿修复 spec（不属于本 spec 范围）

必须先完成下列动作（来自 `~/.mavis/plans/plan_8b640472/outputs/visibility-auditor/deliverable.md` Block B1 + `duplicate-scanner/deliverable.md` § 7.3）：

1. 在 `mod.rs` 添加：
   ```rust
   pub mod session_evidence;
   pub mod session_persistence;
   pub mod session_restore;
   pub use session_evidence::*;
   pub use session_persistence::*;
   pub use session_restore::*;
   ```
2. 删除 `session_manager.rs` 中 64 个与方法 sibling 重名的方法体（参见 `outputs/duplicate-scanner/deliverable.md` § 6.2 的 64 行表）。
3. 删除孤儿兄弟文件里与 manager 语义不一致的方法体（保证唯一真值源）。
4. 跑 `cargo check -p northhing-core --features product-full` 确认 0 errors。

孤儿修复完成后：`session_manager.rs ≈ 6532 - 300 = 6200 行`。

### Step 1：创建 `session_lifecycle.rs` 骨架

文件路径：`northing/src/crates/assembly/core/src/agentic/session/session_lifecycle.rs`

骨架结构（先只放 header + 一个空 impl 块）：

```rust
//! Session lifecycle: create / get / list / touch / delete
//!
//! Sibling impl block for `SessionManager` — keeps the god object's CRUD surface in
//! one focused file. Helpers (`session_workspace_from_config`, `should_persist_session`,
//! etc.) and other domains (evidence / persistence / restore) remain in
//! `session_manager.rs` and the other sibling modules.

use crate::agentic::core::{Session, SessionConfig, SessionKind, SessionSummary};
use crate::agentic::persistence::PersistenceManager;
use crate::service::snapshot::ensure_snapshot_manager_for_workspace;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::timing::elapsed_ms_u64;
use std::path::Path;
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use tracing::{debug, info, warn};

use super::session_manager::SessionManager;

impl SessionManager {
    // (lifecycle methods will move here one-by-one)
}

#[cfg(test)]
mod tests {
    // (lifecycle tests will move here)
}
```

确认 `cargo check -p northhing-core --features product-full` 编译通过（空 impl 块不影响）。

### Step 2：迁移方法 — 逐个搬（增量 cargo check）

每移动一个方法就运行 `cargo check -p northhing-core --features product-full`：

**顺序**（按行号从大到小，避免行号漂移）：

1. **delete_session** (L2204–L2353, 150 rows) — 最大，先移它
2. **touch_session** (L2197–L2201, 5 rows)
3. **create_session_with_id_and_details** (L1128–L1187, 60 rows) — 核心实现
4. **create_session_with_id_and_creator** (L1108–L1125, 18 rows)
5. **create_session_with_id** (L1089–L1105, 17 rows)
6. **create_session** (L1071–L1086, 16 rows)
7. **get_session** (L1190–L1192, 3 rows)
8. **list_sessions** (L3049–L3097, 49 rows)

每步：

1. 把 fn 体从 session_manager.rs 剪切到 session_lifecycle.rs 同名 impl 块
2. 移除 `// ============ Session CRUD ============` 分隔注释（最后一个 fn 移完后）
3. 运行 `cargo check -p northhing-core --features product-full`
4. 错误处理：
   - "找不到字段 X" → 检查字段可见性（见 §5 风险点）
   - "找不到函数 X" → 检查 `Self::xxx()` 关联函数是否仍存在于 session_manager.rs（本 spec 不移它们）
   - "找不到类型 X" → 补 import

### Step 3：迁移测试

**只移 2 个 LIFECYCLE 主测**：

1. `ephemeral_child_session_is_kept_in_memory_without_persisting` (L4799–L4829)
2. `delete_session_removes_workspace_cache_entry` (L5935–L5970)

每个测试需要：

- `use super::*;` 拿 SessionManager 本文件符号
- `use crate::agentic::session::test_helpers::*;` 或保留 `TestWorkspace`、`test_manager`、`in_memory_test_manager`、`PersistenceManager` 等在 session_manager.rs 中（测试 helper 不随被测方法走 — 详见 §5.6）
- 用 `use crate::agentic::session::session_manager::SessionManagerConfig;` 等显式 import 路径访问 session_manager.rs 中的 helper

每移一个跑 `cargo test -p northhing-core ephemeral_child_session_is_kept_in_memory_without_persisting -- --nocapture`，确认通过。

### Step 4：更新 `mod.rs`

在 `northing/src/crates/assembly/core/src/agentic/session/mod.rs` 添加：

```rust
pub mod session_lifecycle;
pub mod session_evidence;
pub mod session_persistence;
pub mod session_restore;
// ... existing 8 modules stay ...

pub use session_lifecycle::*;
// ... existing pub use stays ...
```

### Step 5：验收

```bash
cargo check -p northhing-core --features product-full
cargo test --workspace
```

期望：

- `cargo check` 0 errors
- `cargo test --workspace` 935+ pass（与孤儿修复后等价）

---

## 3. 验收标准（Acceptance Criteria）

### 3.1 行数验收

| 文件 | 孤儿修复后 | 本 spec 后 | 变化 |
|---|---|---|---|
| `session_manager.rs` | ~6,200 | **~5,800** | −400 行 |
| `session_lifecycle.rs` | — | **~440** | 新增 |
| `mod.rs` | 26 → ~31 | 31 → **~33** | +2 行 |

**关于用户预期目标 (800–1500 行 drop / 1000–1500 行新文件)**：

实际测量结果（约 400 行 drop / 440 行新文件）**显著低于**用户预期的 800–1500。原因：

- LIFECYCLE 域的方法体本来就精炼（8 个 fn = 318 行），不像 EVIDENCE/PERSISTENCE/RESTORE 那样有大量重复体
- 仅 2 个测试是 LIFECYCLE 主测；其它 19 个测试虽然用 create_session，但主测目标是 state/persistence/evidence/restore
- 19 行 helper 注释、5 行 header 等装饰性内容不到 50 行

**如希望达到 800–1500 行 drop，可选扩展**（不在本 spec 范围，需用户决策）：

- **(a) 移动所有调用 create_session 的测试**：把 19 个非主测但调用 create 的测试一并搬到 session_lifecycle.rs 的 tests 子模块。技术可行 — 测试 helper（TestWorkspace、test_manager）需要 `pub(super)` 可见性。预估增加 1100+ 行。
- **(b) 移动 STATE 域方法**（`update_session_state` / `update_session_title` / `update_session_agent_type` 等 14 个）：用户没明确指明，但属于 session lifecycle 范畴。预估增加 700+ 行。
- **(c) 拆分多个文件**：进一步拆成 `session_lifecycle_create.rs`、`session_lifecycle_delete.rs`、`session_lifecycle_list.rs`。但用户倾向"最小单文件"，不推荐过度拆分。

### 3.2 测试验收

- `cargo test -p northhing-core` 935+ pass
- 迁移的 2 个 lifecycle 测试在 session_lifecycle.rs 中通过
- 没有新增测试失败
- `cargo build --workspace` 0 errors / 0 new warnings（孤儿修复已经触发的 ~139 warnings 不增加）

### 3.3 可见性 / 编译验收

- `SessionManager` struct 字段从 sibling impl 块可访问 — 当前字段未标可见性，详见 §5 风险点
- 公开 API 路径不变 — `pub use session_lifecycle::*;` 保证 `crate::agentic::session::SessionManager::create_session` 等方法仍可从外部访问

### 3.4 行为等价验收

- `cargo test --workspace` 全过即证明行为等价（无新功能）
- `git diff 5250199 HEAD -- session_manager.rs` 应显示 net 删除约 400 行（无功能改动）

---

## 4. 风险点

### 4.1 ⚠️ 字段可见性是阻塞项

**当前 `session_manager.rs:104-126`**：

```rust
pub struct SessionManager {
    sessions: Arc<DashMap<String, Session>>,           // private（无 pub）
    session_workspace_index: Arc<DashMap<String, PathBuf>>,
    context_store: Arc<SessionContextStore>,
    // ...
}
```

字段未标注可见性（默认 private）。Rust 中**同 crate 的 sibling 模块**可以访问 private 字段 — 这是 Rust 的"同 crate 可见"规则，不是字段可见性。**实测**：在 sibling 模块中写 `self.sessions.get(...)` 是允许的，因为 sibling 模块与原始模块位于同一 crate 内。

**风险等级**：低。但需要 Task agent 实测验证（写第一个 fn `delete_session` 后 cargo check 通过即证明）。

### 4.2 关联函数 helper 不动

`session_workspace_from_config`、`should_persist_session`、`effective_workspace_path_from_config`、`effective_session_workspace_path`、`should_persist_session_id`、`resolve_session_workspace_path` 等 helper 在 session_manager.rs 中以 `fn` (private) 定义，跨 impl 块**不能直接调用** — 必须用 `Self::` 前缀。

实测验证：现有 `session_evidence.rs` / `session_persistence.rs` / `session_restore.rs` 中的 `impl SessionManager` 块**不调用任何 `Self::xxx()` helper**（因为孤儿状态下没人 review 这点）。本 spec 是 session_manager.rs 第一次真正拆出 `impl` 块。

**风险等级**：中。Task agent 必须**实测** `Self::session_workspace_from_config(...)` 在 sibling impl 中可调用。如果不行，需把 helper 提升为 `pub(super)` 或 `pub(crate)`。

### 4.3 import 重新声明

新文件需要：

```rust
use crate::agentic::core::{Session, SessionConfig, SessionKind, SessionSummary};
use crate::agentic::persistence::PersistenceManager;
use crate::service::snapshot::ensure_snapshot_manager_for_workspace;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::timing::elapsed_ms_u64;
use std::path::Path;
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use tracing::{debug, info, warn};
```

**风险等级**：低。逐 fn 移动时缺什么补什么。

### 4.4 测试 helper 留在 session_manager.rs

`TestWorkspace`、`test_manager`、`in_memory_test_manager`、`PersistenceManager` 等测试 helper 全部保留在 session_manager.rs 的 `#[cfg(test)] mod tests` 中。新文件的 tests 模块需要：

```rust
use super::super::session_manager::tests::{
    test_manager, in_memory_test_manager, TestWorkspace,
};
```

但 `tests` 模块默认 private。需要把 session_manager.rs 中的 `mod tests` 改成 `pub(super) mod tests` 或者更精确地用 `pub(crate) mod tests`。

**风险等级**：中。需要明确可见性策略。

### 4.5 mod.rs 8 个模块声明顺序

当前 mod.rs 已经有 8 个 `pub mod xxx;` 声明 + 8 个 `pub use xxx::*;`。新增 `session_lifecycle` 应该在 `session_manager` 之后（语义相关）。

**风险等级**：极低。

### 4.6 cargo fmt / cargo clippy 噪音

迁移会引入新文件的格式问题。Task agent 完成后**不**应跑 `cargo fmt`（项目当前有 156 个 pre-existing 改动，agent memory 2026-06-24 指出不要碰）。如必须 fmt，限定到新文件：

```bash
cargo fmt -- src/crates/assembly/core/src/agentic/session/session_lifecycle.rs
```

**风险等级**：低。

### 4.7 孤儿修复必须先做

如果跳过 Step 0 直接做本 spec：

- mod.rs 还没有 `pub mod session_evidence / persistence / restore;`
- session_manager.rs 仍保留全部 138 个方法（与孤儿文件重复）
- 加 `pub mod session_lifecycle;` 后，session_manager.rs 与 session_lifecycle.rs 中同名方法会**重复定义** → Rust 编译错 E0599

**风险等级**：高。**必须先做孤儿修复**。

---

## 5. 与其他 spec 的衔接

### 5.1 上游

- **孤儿修复 spec**（`docs/handoffs/2026-06-27-r4-comprehensive-cleanup-plan.md` 或独立 spec）：必须先完成。本 spec 是其后续动作。

### 5.2 下游

本 spec 完成后，`session_manager.rs` 仍有 ~5,800 行，遗留方法分组为：

| 域 | 方法数估算 | 下一步拆分目标 |
|---|---|---|
| STATE (state / title / model_id) | ~16 | `session_state.rs`（未来 spec） |
| PERSISTENCE (turn lifecycle) | ~28 | 已存在 `session_persistence.rs`（孤儿修复后合并） |
| EVIDENCE (evidence / skill agent snapshot) | ~25 | 已存在 `session_evidence.rs`（孤儿修复后合并） |
| RESTORE (restore / rollback) | ~16 | 已存在 `session_restore.rs`（孤儿修复后合并） |
| INFRASTRUCTURE (helpers + spawn tasks) | ~30 | 留在 `session_manager.rs` |
| MESSAGE / CONTEXT | ~10 | 留在 `session_manager.rs` |

**最终目标**（多轮拆分后）：`session_manager.rs` 仅剩 struct 定义 + Config + dispatcher + helpers，~1,000–1,500 行。

### 5.3 与 v3-restructure 分支的关系

本 spec 工作在 `main` 分支（HEAD `9dbcb9c`）。当前用户的 MVP 路径分支是 `v3-restructure`（agent memory 2026-06-24 记录）。本 spec 的代码修改**不应直接合并到 v3-restructure** — 应该是先在 main 落地，由用户决定 cherry-pick 或 rebase。

---

## 6. Errata

### 6.1 测量方法

- 行数：`[System.IO.File]::ReadAllLines().Count` (= `wc -l` 语义)
- 测试边界扫描：Python 脚本解析 `{` `}` 嵌套（参考 `outputs/duplicate-scanner/extract_session_impl.py`）
- LIFECYCLE 域归类：基于 fn 名的 `_session` 后缀 + 文档注释（"Create / Get / Delete / List / Update session activity time"）

### 6.2 不确定的判断

- **`touch_session` 是否属于 LIFECYCLE**：仅 5 行，更像 activity-tracker helper。但 fn 名带 `_session` 后缀，按用户"create/delete/list/get/switch"的扩展解释 "etc."，归入 LIFECYCLE 域。Task agent 如有疑虑可与用户确认。
- **`list_sessions` 的非持久化分支**：当前实现遍历 `self.sessions` 直接构造 SessionSummary。如果未来改为完全依赖 `persistence_manager.list_sessions`，可能合并到 PERSISTENCE 域。但当前 fn 主要意图是暴露"会话清单"接口，归 LIFECYCLE。
- **2 个 lifecycle 测试**：其它 19 个测试虽然用 create，但主测目标是 state/persistence/evidence/restore。如希望更彻底拆分，把它们一起搬过来，需 user 决策（见 §3.1 (a) 选项）。

### 6.3 与用户预期目标的差距

如 §3.1 所述：实际 400 行 drop 显著低于用户预期 800–1500 行。三种可选扩展方案详见 §3.1 (a)/(b)/(c)。**Task agent 应先按本 spec 执行**，完成后报告实际数字与差距，**不要自行扩大 scope**。

### 6.4 已知的语义改动

本 spec 不引入任何行为改动。仅做物理文件重组。所有方法体逐字移动（除 `Self::` 前缀规范化外）。