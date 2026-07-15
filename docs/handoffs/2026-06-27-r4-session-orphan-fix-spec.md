# R4 Spec: session/ orphan-sibling 立即修复（P0 Blocker）

> **目的**：完成 commit `5250199` 失败的拆分意图（Round 3b）。
> **范围**：仅做最小修复让 orphan sibling 真正生效（不重做完整拆分设计）。
> **不修改任何源文件**——本文件是 spec，下游由 Task agent apply。

## 1. Background（Phase 1 audit 验证）

- HEAD：`9dbcb9cc`（`main`，2026-06-27 18:30 +08:00）
- 触发 commit：`52501994`（2026-06-27 03:08，`refactor(session): split 6532-line god object`）
- 文件状态（`[System.IO.File]::ReadAllLines().Count` = `wc -l` 语义，2026-06-27 实测）：

| 文件 | 行数 | 字节 | 编译状态 | 备注 |
|---|---|---|---|---|
| `session_manager.rs` | **6,532** | 247,613 | ✅ 编译 | 主体 impl block 仍保留所有原方法体（parallel copy） |
| `session_evidence.rs` | **749** | 28,036 | ❌ orphan | 未声明到 mod.rs → cargo 不感知 |
| `session_persistence.rs` | **1,272** | 47,072 | ❌ orphan | 同上 |
| `session_restore.rs` | **757** | 31,594 | ❌ orphan | 同上 |
| `mod.rs` | 26 | 771 | ✅ 编译 | 仅 8 个 `pub mod` 声明 |
| **orphan 小计** | **2,778** | 96,702 | - | dead code，0 test，0 caller |

- **重复统计**（duplicate-scanner §6.2）：**64 个 fn 名**在 4 个 agentic/session `impl SessionManager` 块中重复出现 ≥2 次
- **exact-signature 重复**（visibility-auditor §6.2）：**12 个**，加上 multi-line 严格匹配的 **16 个 session_restore** = **28 个 high-confidence 重复**
- **并行复制**事实：visibility-auditor §6.2 验证 sibling 方法体与 mgr 对应方法体**逐字一致**（commit message 也自认："a future cleanup pass could remove the duplicates from session_manager.rs to make it a true split (not a parallel copy)"）
- **测试基线**：`cargo test -p northhing-core --features product-full` 当前 **935+ 通过**（visibility-auditor 实测 `Finished 1.18s, 0 errors, 139 warnings`），sibling 三个文件 0 test，0 个 test 期望 sibling 方法的特定行为

## 2. 修复方案（3 步）

### 2.1 步骤 1：mod.rs 声明 sibling（4 → 11 个 pub mod）

**当前 mod.rs**（`agentic/session/mod.rs`，26 行）：
```rust
pub mod compression;
pub mod context_store;
pub mod evidence_ledger;
pub mod file_read_state;
pub mod prompt_cache;
pub mod session_manager;
pub mod session_store_port;
pub mod turn_skill_agent_snapshot_store;

pub use compression::*;
pub use context_store::*;
pub use evidence_ledger::*;
pub use file_read_state::*;
pub use prompt_cache::*;
pub use session_manager::*;
pub use session_store_port::*;
pub use turn_skill_agent_snapshot_store::*;

pub use northhing_runtime_ports::{
    SessionStorageKind, SessionStoragePathRequest, SessionStoragePathResolution,
    SessionTurnLoadTiming, SessionViewRestoreRequest, SessionViewRestoreTiming,
};
```

**改动**（保留字母序）：
- `pub mod session_manager;` **之后**插入（与 `session_manager`/`session_store_port` 之间）：
  - `pub mod session_evidence;`
  - `pub mod session_persistence;`
  - `pub mod session_restore;`
- 对应 `pub use` 同样插入（保持 `pub mod` ↔ `pub use` 一一对应）

**警告**（CRITICAL）：仅做此步会触发**大量编译错误**：
- E0616 `private field ... accessed from outside its module`（sibling 通过 `self.sessions`、`self.evidence_ledger` 等访问 mgr 的 10 个 private 字段）
- E0616 `private function ... cannot be accessed`（sibling 内部调用 mgr 的 private helper，如 `effective_session_workspace_path`、`build_messages_from_turns`、`sanitize_listing_diff_context_snapshot_if_needed` 等）
- E0616 `private constant ... cannot be accessed`（sibling 可能用到 `LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY`）

→ 步骤 1 必须与 §2.2 + §2.3 配套，否则 cargo check 立即爆 E06xx。

### 2.2 步骤 2：提升 SessionManager 字段可见性（10 个字段 `private` → `pub(crate)`）

`session_manager.rs:104-126` 当前 `pub struct SessionManager` 的 10 个字段都是 private。sibling 一旦声明到 mod.rs，所有访问 `self.<field>` 的代码都会因为 E0616 编译失败。

**字段可见性提升**（按 visibility-auditor §3.2 清单）：

| 字段 | 行号 | 当前 | 改为 | 依据（访问次数 / 跨 split file 范围） |
|---|---|---|---|---|
| `sessions` | 106 | private | **`pub(crate)`** | 34 次访问，跨 5+ split file |
| `session_workspace_index` | 113 | private | **`pub(crate)`** | 4 次访问 |
| `context_store` | 116 | private | **`pub(crate)`** | 16 次访问，跨 4+ split file |
| `prompt_cache_store` | 117 | private | **`pub(crate)`** | 13 次访问 |
| `turn_skill_agent_snapshot_store` | 118 | private | **`pub(crate)`** | 11 次访问 |
| `skill_agent_baseline_override_snapshot_store` | 119 | private | **`pub(crate)`** | 6 次访问 |
| `file_read_state_store` | 120 | private | **`pub(crate)`** | 8 次访问 |
| `evidence_ledger` | 121 | private | **`pub(crate)`** | 4 次访问（evidence.rs + spawn_model_reconciliation_listener） |
| `persistence_manager` | 122 | private | **`pub(crate)`** | 37 次访问，**全部** split file |
| `config` | 125 | private | **`pub(crate)`** | 25 次访问，**全部** split file |

**理由选 `pub(crate)` 而非 `pub(super)`**：
- `pub(crate)` 是 visibility-auditor 推荐的最小可见性，所有 sibling 都在 `crate::agentic::session::*` 同模块
- `pub(super)` 会把可见性泄漏给 `agentic` 父模块（不必要的更宽）
- 参考 round 3a `coordinator.rs` 拆分样板的实际选择（按 round 3b visibility-audit handoff §3.2）

### 2.3 步骤 3：提升跨文件 helper 可见性（9 个 `Self::` + N 个 instance method）

sibling 之间互调需要 `pub(crate)`。完整清单见 `docs/handoffs/2026-06-26-round3b-session-manager-visibility-audit.md` §3.3.2 + §3.4.1。**本 spec 只列 sibling 实际编译需要的最小集**（以 cargo check 报错为校准信号）。

#### 2.3.1 静态方法（`Self::` 形式，session_manager.rs 内部 → sibling 跨调）

| 方法 | 行号 | 当前 | 改为 | 跨调用方 |
|---|---|---|---|---|
| `sync_session_context_window_from_ai_config` | 201 | private | `pub(crate)` | `update_session_model_id:2142`（main）+ `restore_session_with_turns_internal:2738`（restore.rs） |
| `load_ai_config_for_model_resolution` | 144 | private | `pub(crate)` | `update_session_model_id:2119`（main）+ `refresh_session_context_window:2180`（main）+ `restore_session_with_turns_internal:2696`（restore.rs） |
| `effective_workspace_path_from_config` | 414 | private | `pub(crate)` | 9 次调用，跨 5+ split file（main + restore + persistence） |
| `should_persist_session` | 307 | private | `pub(crate)` | 6 次调用，跨 5 split file |
| `is_session_model_id_usable` | 912 | private | `pub(crate)` | `restore_session_with_turns_internal:2708`（restore.rs 跨调） |
| `build_messages_from_turns` | 517 | private | `pub(crate)` | `start_persisted_turn:621`（persistence.rs）+ `restore_session_with_turns_internal:2795,2814`（restore.rs） |
| `strip_listing_diff_internal_reminders` | 1637 | private | `pub(crate)` | `remove_listing_diff_internal_reminders:1622`（evidence.rs）+ `sanitize_listing_diff_context_snapshot_if_needed:1713`（persistence.rs 或 restore.rs） |
| `listing_baseline_rebuild_turn_index_from_metadata` | 1662 | private | `pub(crate)` | `restore_session_with_turns_internal:2676`（restore.rs）+ `rollback_context_to_turn_start:2941`（rollback）+ `sanitize_listing_diff_context_snapshot_if_needed:1765` |
| `derive_last_user_dialog_agent_type_from_turns` | 2080 | private | `pub(crate)` | `restore_session_with_turns_internal:2772`（restore.rs）+ `rollback_context_to_turn_start:2990`（rollback） |

#### 2.3.2 实例方法（`self.<method>` 形式，sibling 跨调）

按 round 3b visibility-audit handoff §3.4.1，跨文件需要的 `pub(crate)` 实例方法（按归属文件）：

**session_manager.rs**（留在主文件，但需要 `pub(crate)`）：
- `effective_session_workspace_path` (line 430) — 18 次跨 split file 调用
- `update_persisted_session_metadata` (line 3187) — 被 4 个 pub metadata method 跨调
- `update_session_metadata_at_workspace` (line 3172) — `update_persisted_session_metadata` 链调用
- `metadata_workspace_path_for_update` (line 3119) — 同上
- `load_or_persist_session_metadata` (line 3140) — 同上

**session_evidence.rs**（sibling 内部，跨 restore 调用）：
- `persist_listing_baseline_rebuild_turn_index_best_effort` (line 1733) — 被 restore + sanitize 跨调

**session_persistence.rs**（sibling 内部，跨 evidence + restore 调用）：
- `ensure_prompt_cache_loaded` (line 687) — 被 6 处跨调
- `persist_prompt_cache_best_effort` (line 766) — 被 6 处跨调
- `persist_context_snapshot_for_turn_best_effort` (line 635) — 被 3 处跨调
- `persist_context_snapshot_messages_best_effort` (line 1670) — evidence + persistence 跨调
- `persist_current_turn_context_snapshot_best_effort` (line 666) — add_message + replace_context + start_persisted_turn 跨调
- `invalidate_prompt_cache` (line 1786) — 已有 `pub` 不变
- `sanitize_listing_diff_context_snapshot_if_needed` (line 1694) — restore rollback 跨调
- `clone_prompt_cache` (line 1268) — 已有 `pub` 不变

**session_restore.rs**（sibling 内部，跨 evidence + persistence 调用）：
- `restore_session_internal` (line 2374) — restore_session + restore_internal_session 跨调
- `restore_session_with_turns_internal` (line 2634) — restore_session_with_turns + restore_internal_session_with_turns 跨调
- `restore_session_view_internal` (line 2481) — 8 个 restore_view* 公共方法跨调
- `truncate_listing_baseline_rebuild_turn_index_after_rollback` (line 1754, mgr 行号) — rollback 跨调
- `start_persisted_turn` (line 3269) — 5 个 start_dialog_turn* / start_maintenance_turn / append_completed_local_command_turn 跨调

**注**：上述 `pub(crate)` 提升，**优先复用 visibility-audit handoff §3.4.1 完整清单**（33 个实例方法）。具体行号可能因 §3 的删除顺序微调，以 cargo check 实际报错为准。

#### 2.3.3 const

| 名称 | 行号 | 当前 | 改为 | 理由 |
|---|---|---|---|---|
| `LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY` | 101 | private | `pub(super)` | restore.rs 和 evidence.rs 都需用，且 const 不影响外部 API 路径 |

### 2.4 步骤 4：删除 session_manager.rs 中的 28 个 parallel copy

visibility-auditor §6 给出 12 个 exact-signature 重复 + 16 个 multi-line session_restore 重复 = **28 个 high-confidence 重复**。本步骤删除 session_manager.rs 中这 28 个方法体（保留 sibling 版本作为 canonical），按 4 个分组顺序进行：

#### 2.4.1 group A（evidence，6 个方法）

| # | fn | session_manager.rs 行号 | canonical sibling 行号 | 可见性变化 | 外部 caller |
|---|---|---|---|---|---|
| 1 | `append_evidence_event` | 835 | session_evidence.rs:61 | 已是 `pub(crate)` 不变 | 无（仅 mod tests） |
| 2 | `invalidate_ai_clients_for_models` | 990 | session_evidence.rs:215 | private → `pub(crate)` | 无（内部调） |
| 3 | `rebuild_skill_agent_listing_baseline_to_latest` | 1576 | session_evidence.rs:512 | 已是 `pub(crate)` 不变 | `execution_engine.rs`（同 crate，安全） |
| 4 | `remove_listing_diff_internal_reminders` | 1615 | session_evidence.rs:551 | 已是 `pub(crate)` 不变 | `dialog_turn.rs`（同 crate） |
| 5 | `strip_listing_diff_internal_reminders` | 1637 | session_evidence.rs:573 | private → `pub(crate)` | 无（内部调） |
| 6 | `spawn_model_reconciliation_listener` | 1001 | session_evidence.rs:683 | private → `pub(crate)` | 无（仅 `new()` 内部） |

#### 2.4.2 group B（persistence，5 个方法）

| # | fn | session_manager.rs 行号 | canonical sibling 行号 | 可见性变化 | 外部 caller |
|---|---|---|---|---|---|
| 7 | `build_messages_from_turns` | 517 | session_persistence.rs:62 | private → `pub(crate)` | 无（restore_session_with_turns_internal 内部） |
| 8 | `ensure_prompt_cache_loaded` | 687 | session_persistence.rs:232 | private → `pub(crate)` | 无（cached_* / remember_* 内部） |
| 9 | `persist_prompt_cache_best_effort` | 766 | session_persistence.rs:311 | private → `pub(crate)` | 无（同上） |
| 10 | `reset_session_state_if_processing` | 1813 | session_persistence.rs:533 | 已是 `pub(crate)` 不变 | `dialog_turn.rs`, `coordinator.rs`（同 crate） |
| 11 | `cancel_dialog_turn` | 3784 | session_persistence.rs:1071 | 已是 `pub(crate)` 不变 | `cli`, `server`, `coordinator`, `ports`, `dialog_turn`（同 crate） |

#### 2.4.3 group C（restore，16 个方法）

| # | fn | session_manager.rs 行号 | canonical sibling 行号 | 可见性变化 | 外部 caller |
|---|---|---|---|---|---|
| 12 | `restore_session` | 2356 | session_restore.rs:67 | `pub` → `pub(crate)` | `cli/agent/*`（同 crate） |
| 13 | `restore_internal_session` | 2365 | session_restore.rs:76 | `pub` → `pub(crate)` | `dialog_turn.rs`（同 crate） |
| 14 | `restore_session_internal` | 2374 | session_restore.rs:85 | 已是 `pub(crate)` 不变 | 无（仅 `restore_session*` 链） |
| 15 | `restore_session_view` | 2389 | session_restore.rs:99 | `pub` → `pub(crate)` | `cli/modes/exec.rs`, `root_handlers.rs`（同 crate） |
| 16 | `restore_session_view_timed` | 2399 | session_restore.rs:109 | `pub` → `pub(crate)` | `dialog_turn.rs`（同 crate） |
| 17 | `restore_internal_session_view` | 2409 | session_restore.rs:119 | `pub` → `pub(crate)` | `dialog_turn.rs` |
| 18 | `restore_internal_session_view_timed` | 2419 | session_restore.rs:129 | `pub` → `pub(crate)` | `dialog_turn.rs` |
| 19 | `restore_session_view_tail` | 2429 | session_restore.rs:139 | `pub` → `pub(crate)` | `dialog_turn.rs` |
| 20 | `restore_session_view_tail_timed` | 2440 | session_restore.rs:150 | `pub` → `pub(crate)` | `dialog_turn.rs` |
| 21 | `restore_internal_session_view_tail` | 2455 | session_restore.rs:165 | `pub` → `pub(crate)` | `dialog_turn.rs` |
| 22 | `restore_internal_session_view_tail_timed` | 2466 | session_restore.rs:176 | `pub` → `pub(crate)` | `dialog_turn.rs` |
| 23 | `restore_session_view_internal` | 2481 | session_restore.rs:191 | private → `pub(crate)` | 无（仅 sibling 内部） |
| 24 | `restore_session_with_turns` | 2616 | session_restore.rs:326 | `pub` → `pub(crate)` | `dialog_turn.rs`（同 crate） |
| 25 | `restore_internal_session_with_turns` | 2625 | session_restore.rs:335 | `pub` → `pub(crate)` | `dialog_turn.rs` |
| 26 | `restore_session_with_turns_internal` | 2634 | session_restore.rs:344 | private → `pub(crate)` | 无（仅 sibling 内部） |
| 27 | `rollback_context_to_turn_start` | 2922 | session_restore.rs:632 | `pub` → `pub(crate)` | 无（仅 sibling 内部） |

#### 2.4.4 group D（无 — 任务清单中的 placeholder）

任务模板提到 "12: (None for session_restore.rs)" 是基于 visibility-auditor 的 strict first-line-sig match。**本 spec 扩展为 16 个**（用 multi-line sig match），与 visibility-audit handoff §6.2 列表完全一致。

## 3. 实施步骤（增量 cargo check 校准）

按可见性影响范围递增排列，**每个 step 后跑一次 `cargo check -p northhing-core --features product-full`**：

### Step 0: baseline 校验
```bash
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cd E:\agent-project\northing
cargo check -p northhing-core --features product-full --message-format=short
git status -s src/crates/assembly/core/src/agentic/session/
```
预期：`Finished 1.18s, 0 errors, 139 warnings`，mod.rs 8 pub mod，sibling 3 文件未声明。

### Step 1: 提升 SessionManager 字段可见性（10 个字段）
- 修改 `session_manager.rs:104-126` 的 10 个字段从 `private` 到 `pub(crate)`
- `cargo check` — 预期 **0 errors**（字段可见性提升不改变现有 mgr 内部代码，sibling 还未声明）
- 验证：`git diff session_manager.rs` 应只显示字段可见性变化

### Step 2: 提升 const + 9 个静态方法 + 关键实例方法可见性
- `session_manager.rs:101` 的 `LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY` → `pub(super)`
- `session_manager.rs` 的 9 个静态方法（§2.3.1 表格）→ `pub(crate)`
- `session_manager.rs` 的关键实例方法（§2.3.2 表格列出但留在 mgr 的）→ `pub(crate)`
- `cargo check` — 预期 **0 errors**（仍只编译 mgr 内部代码）
- 验证：visibility-audit handoff §3.4.1 列出的所有 sibling 需要的 mgr 内部方法已 pub(crate)

### Step 3: 添加 mod.rs 声明
- `mod.rs` 增加 3 个 `pub mod` + 3 个 `pub use`（§2.1 内容）
- `cargo check` — **预期可能 E06xx 编译错误**（sibling 内部访问 mgr private helper 报 E0616）→ 按 cargo check 实际报错补充提升可见性
- 关键报错模式：
  - `error[E0616]: field 'XXX' of struct 'agentic::session::SessionManager' is private`
  - `error[E0616]: function 'YYY' is private`
  - 解决：补 §2.3 表格中未列出的 `pub(crate)` 提升
- 迭代直到 `cargo check` 通过

### Step 4: 删除 evidence 组（6 个方法，group A）
- 删除 `session_manager.rs:835-836, 990-1000, 1576-1614, 1615-1636, 1637-1651, 1001-1066` 段对应方法体
- 注：删除前先 `git diff session_manager.rs:<line> session_evidence.rs:<line>` 确认 body 字符级一致
- `cargo check` — 预期 0 errors（sibling 已 canonical）
- `cargo test -p northhing-core --features product-full` — 预期 935+ 通过（sibling 0 test 不变）

### Step 5: 删除 persistence 组（5 个方法，group B）
- 删除 `session_manager.rs:517-611, 687-765, 766-803, 1813-1829, 3784-3844` 段对应方法体
- `cargo check` + `cargo test`

### Step 6: 删除 restore 组（16 个方法，group C）
- 删除 `session_manager.rs:2356-3048` 段对应方法体（restore 段）
- 注：rollback 段 2922-3048 与 restore 段连续，整段一起删
- `cargo check` + `cargo test`

### Step 7: 最终验证
```bash
cargo test -p northhing-core --features product-full
cargo fmt --check
cargo clippy -p northhing-core --features product-full -- -D warnings
```
预期：
- 935+ tests pass（不变）
- 0 fmt 差异
- 0 clippy errors

## 4. 验收标准

- [ ] `mod.rs` 包含 **11 个** `pub mod`（原 8 + session_evidence/session_persistence/session_restore）
- [ ] `session_manager.rs` 行数下降至 **~5,500-5,800**（删除 28 个方法体后）
- [ ] `session_evidence.rs` / `session_persistence.rs` / `session_restore.rs` **被 cargo 编译**（sibling 不再 orphan）
- [ ] `cargo test -p northhing-core --features product-full` 仍然 **935+ 通过**
- [ ] `cargo check --workspace` 0 errors
- [ ] `cargo fmt --check` 0 差异
- [ ] 外部 `pub` API 路径 `crate::agentic::session::SessionManager` 不变
- [ ] 外部 `use crate::agentic::session::SessionManager;` 7 个文件不需改 import
- [ ] 外部 `SessionManager::xxx()` 调用 7 个方法（get_session、get_turn_count、get_context_messages、get_messages、list_sessions、reset_session_state_if_processing、should_persist_session_id）继续工作
  - 其中 `reset_session_state_if_processing` 是本 spec 删除的 28 个之一（group B #10），sibling 已是 `pub(crate)`，同 crate 调用安全

## 5. 风险与缓解

### R1. Sibling 方法体与 mgr 不一致（**中风险**）

**事实**：visibility-auditor §6 验证 12 个 exact-signature 案例是 **IDENTICAL** 或 **pub(crate) prefix only**。但 16 个 session_restore 案例的 body 未做 byte-level 验证（visibility-auditor 只验过 first-line sig）。

**缓解**：
- Step 4-6 删除前**强制 `git diff` 校验** body 一致性：
  ```bash
  git diff src/crates/assembly/core/src/agentic/session/session_manager.rs:<line> src/crates/assembly/core/src/agentic/session/session_evidence.rs:<line>
  ```
- 如果 body 不一致，**优先保留 mgr 版本**（sibling 是 parallel copy 的产物，行为回归由 mgr 决定）

### R2. 字段可见性提升破坏 API 约束（**低风险**）

**事实**：10 个字段从 private → pub(crate) 扩大可见性。理论上允许 crate 内其他模块写这些字段，破坏封装。

**缓解**：
- `pub(crate)` 是 crate 内可见，仍小于 `pub`（外部 crate 仍看不到）
- 当前 935 tests 仍通过 = 测试覆盖足够，不会误用
- 完整 god-object 拆分（round 4/5 计划）会重新评估字段可见性，本 spec 只做"足够"的修复

### R3. `pub` → `pub(crate)` 降级破坏外部调用（**已验证零风险**）

**事实**：删除的 28 个方法中，部分是 `pub` 降级到 `pub(crate)`（group C 大多数）。

**已验证**（2026-06-27 18:35 实际外部 caller 扫描）：
- 所有外部 caller **都在 `crate::northhing-core` 同 crate 内**（无跨 crate 调用）
- 涉及文件：`apps/cli/src/agent/*`、`apps/server/src/rpc_dispatcher.rs`、`coordination/{coordinator,dialog_turn,ports}.rs`、`execution_engine.rs`
- `pub(crate)` 在同 crate 内 = 与 `pub` 等价访问，**无破坏**
- 唯一一个跨 crate 可能受影响的是 `services/terminal/src/session/manager.rs:1209` 调 `get_session` —— 但 `get_session` 不在本 spec 删除清单

### R4. mod tests 引用 sibling 内部 private helper（**低风险**）

**事实**：`session_manager.rs:4363-6532` 是 mod tests 块（35 个 test fn + 4 helper），可能通过 `super::xxx` 或 `SessionManager::xxx` 访问 sibling 内的 private 方法。

**缓解**：
- visibility-audit handoff §1.3 已列出 mod tests 用到的 `SessionManager::xxx` 静态调用：只有 7 个，其中 3 个在新 sibling 文件（`build_messages_from_turns`、`listing_baseline_rebuild_turn_index_from_metadata`、`fallback_session_title`）
- 这 3 个的访问路径仍是 `SessionManager::xxx`（impl 块内共享 self），不需要 `pub(crate)` → **不需调整**
- `cargo test` 通过即验证

### R5. cargo check 报错需多次迭代提升可见性（**中风险**）

**事实**：§2.3 表格基于 visibility-audit handoff 预测，但 sibling 实际访问的 helper 可能超出 handoff 范围（handoff 是 pre-split 分析，commit 5250199 实际写出的 sibling 可能与 handoff 预测略有差异）。

**缓解**：
- Step 3 设计为迭代校准循环（cargo check 报错 → 补 `pub(crate)` → 再 check）
- 经验估计需要 1-3 轮迭代
- 每轮只增 `pub(crate)` 标注，不改其他代码

## 6. Errata

### E1. 是否给 sibling 加 `#[cfg(feature = "experimental-session-split")]` feature gate？

**推荐**:**不加**,**保持默认 wired**。理由:
- 当前 orphan 状态浪费 2,778 行 dead code,修复是**激活**而不是**实验**
- feature gate 会让 cargo 在没有 flag 时跳过 sibling,等于维持 orphan 状态 → **未解决问题**
- 如果未来需要回退,git revert commit 比 feature flag 简单
- 与 round 3a `coordinator.rs` 拆分样板一致(默认 wired)

### E2. 是否保留 session_manager.rs 的 28 个方法作为 deprecated wrapper?

**推荐**:**不保留**,**直接删除**。理由:
- 28 个 wrapper 会增加 ~1500 行 boilerplate(session_manager.rs 已 6,532 行)
- mgr 现状的 138 fn 中已经有 ~64 个 fn 是同 mgr 内的 internal helper 链,再加 28 个 wrapper 会让文件更长
- 所有外部 caller 都在同 crate 内,通过 `impl SessionManager` 路径调用,`pub(crate)` 足够
- 完整 god-object 拆分(round 4/5 计划)会处理任何 deprecated API 路径,本 spec 只做最小修复
- Round 3a 拆分(coordinator.rs)也用直接删除方式,无 wrapper 兼容层

### E3. 是否需要在 mod.rs 顶部加 doc 注释说明拆分来源?

**推荐**:**加**(参考 visibility-audit handoff §4.4 草稿)。约 11 行 doc,记录:
- Round 3b 是 2026-06-27 03:08 commit 5250199 的拆分尝试
- session_manager.rs 是 god-object 修复中的 R4 步骤
- 后续 round 5/6 会拆 review_platform/mod.rs 和 chat.rs

这不是强需求,但保持与 round 3a coordinator 拆分样板一致,便于代码考古。

## 7. 不在范围

- **不重做完整拆分设计**（visibility-auditor handoff + round 3b visibility-audit handoff 已设计完整 4 文件拆分,本 spec 只做"让 orphan 生效"）
- **不删除 mod tests 块**(35 个 test + 4 helper 保留在 session_manager.rs,参考 round 3a 策略)
- **不动 dialog turn 子模块**(已正确归属 session_persistence.rs)
- **不动 model reconciliation listener**(`spawn_model_reconciliation_listener` 移 evidence 但仍由 `new()` 在 session_manager.rs 启动,跨调已 pub(crate) 化)
- **不动 public API 路径**(`crate::agentic::session::SessionManager` 等外部 import 全部不变)
- **不解决 review_platform/mod.rs god-object**(round 6 spec 单独处理)
- **不解决 chat.rs god-object**(round 5 spec 单独处理)

## 8. 引用文档

- `docs/handoffs/2026-06-26-round3b-session-manager-split-plan.md` — 完整 4 文件拆分设计(563 行,Task agent 可参考)
- `docs/handoffs/2026-06-26-round3b-session-manager-visibility-audit.md` — 完整 visibility 改动清单(第 3 节 §3.1-§3.4 是字段/const/方法可见性权威清单,本 spec §2.3 是其精简版)
- `~/.mavis/plans/plan_8b640472/outputs/visibility-auditor/deliverable.md` — Phase 1 audit B1 blocker + 12 exact-signature 重复表
- `~/.mavis/plans/plan_8b640472/outputs/duplicate-scanner/deliverable.md` — 64 个 fn-name 重复 + 16 个 multi-line session_restore 重复

## 9. 状态

- [x] Spec 草稿(本文档)
- [ ] 用户 review
- [ ] Task agent apply(mod.rs + 字段可见性 + 28 个方法删除)
- [ ] 内部自审(cargo check + cargo test 935+ pass)
- [ ] 外部 review

**Owner**: Mavis(orchestrator)
**External reviewer**: TBD(用户安排)
**Target**: 1 commit(完成 session/ orphan 修复)+ 1 review handoff,单次 review pass
