# Task T2-10 Report — 连续性自检测试（seed-restore-diff 形态）

## 1. 复用侦察（Mandatory Reconnaissance）

- **`restore_dialog` 先例**（`session_manager_lifecycle_tests_restore_dialog.rs:19-137`）：
  - 侦察到 `TestWorkspace` + `with_user_root_for_tests` 隔离 user_root 模式。
  - 侦察到 `PersistenceManager::new` + `test_manager` 装配 `SessionManager` 的完整范式。
  - 侦察到 `save_dialog_turn` / `save_turn_context_snapshot` / `replace_context_messages` 的上下文快照一致性保证机制。
- **`MemoryDbPathGuard` 实现**（`memory_db.rs:822-854`）：
  - 侦察到 thread-local `TEST_MEMORY_DB_PATH` + `unique_test_memory_db_path()` + RAII `Drop` 模式。
  - `#[tokio::test]` current-thread runtime 下线程内路径重定向，并在 drop 时清理临时 SQLite DB 及其 WAL/SHM 附带文件。
- **`identity.rs` 现状与隔离需求**：
  - 侦察到 `identity_path()` 直接解析 `<config_dir>/northhing/identity.md`，测试直写会导致真实用户环境污染。
  - 遵循 `MemoryDbPathGuard` 模式，在 `identity.rs` 增加 `#[cfg(test)]` 隔离缝（`IdentityPathGuard` + `with_test_identity_path` + `unique_test_identity_path`），确保零生产环境破坏与零生产运行时变化。
- **Agentic Session 测试挂载方式**：
  - `session_manager_tests.rs` 通过 `#[path = "session_manager_lifecycle_tests/mod.rs"] mod session_manager_lifecycle_tests;` 引入生命周期测试子模块目录。
  - 新建 `continuity_selfcheck.rs` 并挂入 `session_manager_lifecycle_tests/mod.rs`。

---

## 2. Spec 逐条实现（Spec Compliance）

1. **Identity 测试隔离缝**（`identity.rs`）：
   - 增加 `#[cfg(test)]` 隔离：`TEST_IDENTITY_PATH` thread-local，`with_test_identity_path` 返回 RAII guard，drop 时重置 thread-local 并删除临时 `.md` 文件。
   - `identity_path()` 仅在 `#[cfg(test)]` 且存在 override 时返回隔离路径；生产代码路径与行为零变化。
2. **连续性自检测试**（`src/crates/assembly/core/src/agentic/session/session_manager_lifecycle_tests/continuity_selfcheck.rs`）：
   - 文件头部详细声明天花板注释：当前 "kill" 范围局限于 `SessionManager` / `PersistenceManager` / `MemoryDb` / `Identity` 层 drop 与重建；完整 OS 进程级杀与守护进程恢复由 T5 负责。
   - 步骤：
     1. 初始化 `TestWorkspace`、`with_test_memory_db_path`、`with_test_identity_path`、`ensure_global_config_for_tests`。
     2. `create_session_with_id` 创建固定 session id 会话。
     3. 注入 2 个 dialog turns（包含用户消息和 assistant model round），持久化 turns 与 session 以及 context snapshot。
     4. `MemoryDb::insert_fact` 插入 2 条固定 facts（1 条 workspace scope，1 条 global scope）。
     5. `save_identity` 写入固定身份文本。
     6. `drop(manager)` / `drop(persistence_manager)` / `drop(memory_db)` 彻底销毁内存句柄。
     7. 从相同路径重建 `PersistenceManager`、`SessionManager`，调用 `restore_session_with_turns`。
     8. 三组等价断言：
        - **Session**：`restored_turns.len() == 2`、`dialog_turn_ids` 序列一致、`turn_id` 序列一致、`context_messages` (role, text) 序列严格匹配、`SessionState::Idle`。
        - **Memory**：2 条 facts 的 `text`、`scope`、`confidence`、`session_id`、`turn_id`、`fact_type` 逐字段匹配（排除 `created_at`/`last_mentioned_at`）。
        - **Identity**：`load_identity()` 恢复字符串全字等价。
3. **分层与范围收敛**：
   - 不碰 fake AI backend 建设，不碰 turn_persist，不碰 identity 生产语义。

---

## 3. 偏离声明（Authorized Deviation）

- **Fake AI backend 依赖绕过**：
  - 编排者裁定（2026-08-21）：预检实证 fake AI backend 为数据罐头半成品（故意不实现 `LongRunningSkill`），本测试采用 seed-restore-diff 形态，以固化 DTO 与 snapshot 直接复现 2 回合上下文与记忆，获得同等且更高的确定性。

---

## 4. 编译错误与解决记录（Rust Router & Error Tracing）

| 编译错误 | 现象 | 根因与分层处理 |
|---|---|---|
| E0432 (Unresolved imports) | `FactConfidence`/`FactProvenance`/`FactScope` 无法从 `service::agent_memory` 导入 | **机制层**：`service::agent_memory::mod` 未 re-export 兄弟模块 `facts.rs` 中的这几个公共类型。在 `agent_memory/mod.rs` 中补全 re-export。 |
| E0063 (Missing fields) | `TextItemData` 直接结构体字面量初始化缺少 7 个非必填/默认字段 | **机制层**：`TextItemData` 具有较多可选与默认字段。定义 `create_text_item` / `create_model_round_with_text` 测试 helper 封装构建。 |
| E0369 / E0609 (Message API) | `MessageContent` 未实现 `PartialEq`；`Message` 的 turn_id 在 `metadata` 中 | **设计层**：`Message` 结构由 `role`、`content`（枚举）和 `metadata`（含 `turn_id`、`semantic_kind`）组成。提取 `extract_role_and_text` helper，针对 `(MessageRole, String)` 及 `metadata.turn_id` 进行精确断言。 |

---

## 5. 验证证据（Verification Evidence）

### 5.1 新测试 3 连跑确定性证据

```powershell
# Run 1:
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full continuity
# Output:
running 1 test
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::continuity_selfcheck::continuity_selfcheck_seed_restore_diff ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1050 filtered out; finished in 0.17s

# Run 2:
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full continuity
# Output:
running 1 test
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::continuity_selfcheck::continuity_selfcheck_seed_restore_diff ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1050 filtered out; finished in 0.15s

# Run 3:
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full continuity
# Output:
running 1 test
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::continuity_selfcheck::continuity_selfcheck_seed_restore_diff ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1050 filtered out; finished in 0.13s
```

### 5.2 Session 就近回归测试

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib session
# Output:
test result: ok. 153 passed; 0 failed; 1 ignored; 0 measured; 897 filtered out; finished in 0.46s
```

### 5.3 编译与全工作区检查（House Rule 6）

```powershell
# Workspace Check:
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
# Output:
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 44s

# Desktop Check (House rule 6 gate):
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
# Output:
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 52s
```

### 5.4 边界、Rot 预算与格式化检查

```powershell
node scripts/check-core-boundaries.mjs && pnpm run check:rot && pnpm run fmt:rs
# Output:
Core boundary check passed.
✔ compliant fixture exits 0 and reports success (98.9726ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (86.7497ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (89.6641ms)
✔ registered god-file exceeding ceiling fails (5.2093ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (5.9399ms)
✔ actual workspace rot budget passes with current manifest (329.0193ms)
ℹ tests 6
ℹ pass 6
ℹ fail 0
Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1363 files).
[format-changed-rust] Formatting 4 Rust file(s).
[format-changed-rust] Restoring 8 collateral Rust file(s) touched through module expansion.
```

---

## 6. Global Constraints 检查清单

- [x] 日志/注释 English-only，无 emoji。
- [x] `identity.rs` 测试隔离缝严格 `#[cfg(test)]`，生产环境 0 行为变化。
- [x] 3 连跑确定性实测通过（0.17s / 0.15s / 0.13s 全部 pass）。
- [x] 断言字段与实现严格一致，无任何伪造断言。
