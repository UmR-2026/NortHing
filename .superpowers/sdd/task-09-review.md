# Task 9 Review — 双判决（spec 合规 + 代码质量）

**Reviewer**: judge-m3
**Scope**: commits 1a65fc1..6574b01（5 文件 +144/-11）
**Verdict**: ⚠️ **APPROVED WITH NOTES**（spec 判决 PASS / quality 判决 PASS w/ 2 Minor + 1 FYI）

---

## 1. Spec 合规判决 — **PASS**

### 组 A hermetic seam（auto_memory prompt_injection ×3）

| 要求 | 验证 |
|---|---|
| 测试 hermetic，不依赖本机 DB | ✅ 4 个测试加 `MemoryDbPathGuard` + UUID 唯一路径，`default_memory_db_path` 命中 `TEST_MEMORY_DB_PATH` thread-local |
| global facts 优先语义不变 | ✅ seam 只替换 DB 文件路径；`MemoryDb::get_facts` 的 SQL 未触碰 |
| 不破坏并行 1128 个测试 | ✅ thread-local 而非进程级 Mutex，其他测试线程的 `default_memory_db_path()` 走真实路径，无阻塞 |
| override 还原 | ✅ `Drop` 恢复 `prev` + best-effort 删除 DB/-wal/-shm |
| 嵌套支持 | ✅ `with_test_memory_db_path` 用 `replace()` 取旧值，guard 持有 `prev`，嵌套顺序还原正确 |
| `let _db_guard = ...` 生命周期 | ✅ 命名绑定 `_db_guard`（非 `let _ = ...`），活到测试函数末尾 |
| 非 test 构建零影响 | ✅ `#[cfg(test)]` 双层门控（thread-local + 函数 + 公共结构 + Drop impl） |

### 组 B-1 initialize 并发安全（TOCTOU 修复）

| 要求 | 验证 |
|---|---|
| 并发安全 | ✅ fast-path `is_initialized()` + `INIT_MUTEX` 持锁 + 持锁 double-check 三层 |
| 失败无半初始化态（可重入重试） | ✅ 所有 fallible work（`ConfigService::new`）在 `OnceLock::set` 之前完成；验证顺序：new → broadcast → set sender → set service |
| 公开 API 签名不变 | ✅ `initialize` / `is_initialized` / `subscribe_updates` / `broadcast_update` / `service` / `update_service` 签名一致 |
| `subscribe_updates` / `broadcast_update` 语义不变 | ✅ 仍查 `CONFIG_UPDATE_SENDER`；修复前半初始化态下 subscribe 返回 `Some`（旧 sender）但 service 失败的 bug 被根治 |
| 锁持有时长无死锁 | ✅ `INIT_MUTEX` 是 `tokio::Mutex`，持锁期间只 await `ConfigService::new` 与 `canonicalize_agent_profile_configs`；grep 确认 canonicalizer 不调 `initialize` 或 `subscribe_updates`（仅 `GlobalConfigManager::service()`，不取 `INIT_MUTEX`） |
| 双判决 spec 要求：INIT_MUTEX 用 `std::OnceLock` 包 `tokio::Mutex` 安全 | ✅ `OnceLock::get_or_init` 自身并发安全；`tokio::sync::Mutex::new` 非 const fn 故需懒初始化；惰性初始化只发生一次 |

### 组 B-2 测试 hermetic 化（cancel/timeout ×3）

| 要求 | 验证 |
|---|---|
| 测试不依赖本机 LLM | ✅ `ensure_global_config_for_tests` 不再调 `AIClientFactory::initialize_global`；本机有 LLM 时 init_turn 也快速失败 |
| 不破坏断言意图 | ✅ tests_cancel.rs / tests_timeout.rs / tests_concurrent.rs / tests_error.rs / tests_parent_chain.rs / tests_success.rs / tests_abort_exit.rs **零修改**；断言（`is_cancelled()` / `is_finished()` / 4 fields populated）原样保留 |
| `assert_secondary_fields_populated` 强度 | ⚠️ 见 quality Minor #1（pre-existing 弱，不在本次引入） |
| mod.rs 改动仅限 subagent_ports | ✅ grep 确认 `build_test_coordinator_with_mock_tool` / `ensure_global_config_for_tests` 仅被 subagent_ports 子测试使用；turn_init/lifecycle 等产品代码未触碰 |
| 不修改其他 1128 测试 | ✅ seam 全 `#[cfg(test)]` 门控；`#![allow(dead_code)]` 仅在 mod.rs（test module），非生产代码 |

### Brief 显式约束

- ✅ Logs English-only, no emojis（`eprintln!`、`debug!`、`info!`、`warn!` 均英语；grep 无 emoji 字符）
- ✅ 不运行 `cargo fmt`（diff 显示手工对齐）
- ✅ 不改变 global-facts 优先语义
- ✅ 不改 `GlobalConfigManager` / `AIClientFactory` 公开 API
- ✅ 不在 main 实现（`fix/backend-debug-0731` 分支）

---

## 2. 代码质量判决 — **PASS WITH NOTES**

### Critical
（无）

### Important
（无）

### Minor

#### Minor #1 — `assert_secondary_fields_populated` `_expected_text` 参数 dead-code（pre-existing, file:line 证据）

`src/crates/assembly/core/src/agentic/coordination/tests/subagent_ports/mod.rs:255`：
```rust
pub fn assert_secondary_fields_populated(phase2: &SubagentPhase2Output, _expected_text: &str) {
```
`_expected_text` 全程未使用（grep 确认仅此一处出现）。这是 **pre-existing** 签名（在 1a65fc1 已存在），非本次引入——但配合本次组 B-2 改动后效果放大：

- 修复前：本机无 LLM 凭证时，`init_turn` 微秒级失败 → `execute_dialog_turn` future 返回 `Err` → `tokio::spawn` 给出 `Ok(Err(ExecutionResult))` → phase2 `Ok` 路径走通 → 4 fields populated 满足断言（但 `_expected_text` 仍不被检查，所以测试本来就不验证 LLM 结果文本）
- 修复后：本机有 LLM 凭证时，`AIClientFactory` 不再初始化 → `init_turn` 微秒级失败 → 同上路径

`_expected_text` 应当删除以使签名诚实，或在该函数内部断言 `phase2.result` 文本与传入值匹配。后者会真正把 cancel/timeout 测试与执行结果的真实内容绑定。建议作为后续 follow-up（不在本任务范围）。

#### Minor #2 — `memory_db.rs` 突破 800 行线（god-file defense 阈值）

`src/crates/assembly/.../memory_db.rs:918` 行（修复前 743 → 修复后 918）。本次 diff 贡献 +75 行（seam 实现）。< 1000 行硬阈值，故**不**需要 `// allow-god-file` 头注；但按 god-file defense 规则"raises review pressure"。

后续若 seam 加测试用例 / WAL/SHM-Journal mode 处理 / 多 schema 迁移逻辑，会很快撞 1000 行硬阈值。建议在合适时机拆分 `memory_db` 为 `memory_db`（核心 CRUD）+ `memory_db_seam`（test seam 子模块）。不在本任务范围。

### FYI

#### FYI #1 — `AIClientFactory::initialize_global` 仍有同类 TOCTOU（report §5 已识别，非本任务）

`src/crates/.../client_factory.rs:224-263` 同模式 check-then-set；本任务未触碰，仅记录。报告 §5 观察 #1 已说明，建议下个 PR 处理。

#### FYI #2 — `DONE: OnceLock<()>` check-then-set 在 `ensure_global_config_for_tests` 仍存在

`subagent_ports/mod.rs:148-155` 的 `DONE` 仍是 check-then-set；`initialize` 自身并发安全后此 TOCTOU 只导致 `initialize` 多调一次（幂等），无功能影响。报告 §5 观察 #3 已说明。

#### FYI #3 — `query_aware_tests::build_query_aware_facts_reminder_returns_some_with_matching_fact`（auto_memory.rs:567-594）未 hermetic 化

仍写真实 DB（`db.insert_fact`），pre-existing 通过测试，未在本次 6 个失败范围。报告 §5 观察 #2 已记录；若未来全量 hermetic 化，可加 `MemoryDbPathGuard`。

#### FYI #4 — 报告的"组 B 根因修正"独立验证可信度合理

报告 §1-B-2 通过 main 基线单独跑 + 临时插桩定位 cancel 真因（execution_task 0.84s 网络往返 > 50ms cancel 窗口）；结论与 brief 的归因（TOCTOU）不同但互补。结论可信：
- `tests_cancel` 单独跑也失败 → 不是 TOCTOU 触发
- `tests_timeout` 单独跑通过 → TOCTOU 不是 cancel 失败原因，但 `tests_timeout` 并行跑 flaky 与 TOCTOU 相关（initialize eprintln 抢锁）
- 两组 fix 都必要：组 B-1 修产品 TOCTOU，组 B-2 让 cancel 测试独立可重入

#### FYI #5 — report 引用 `lifecycle.rs:321-387` "Completed(join_result) Ok 分支无论 result 是 Ok/Err 都返回 Ok" 微细但准确

代码 `lifecycle.rs:320-387`：当 `join_result`（JoinHandle 结果）为 `Ok(result)` 时，`result` 是 `ExecutionResult`（独立 Ok/Err），外层 `Ok(SubagentPhase2Output { result, ... })` 总是包装 `Ok`。只有 `join_result` 自身 `Err`（JoinError，task panic/cancel）才返回 `Err`。在 `init_turn` 失败场景下，future 返回 `Err(ExecutionResult)` → spawn 给出 `Ok(Err(...))` → 走 `Ok` 分支 → phase2 返回 `Ok`。报告论证准确。

---

## 3. 总体判决

- **Spec 合规**：PASS — 三组 seam 全部达 brief 要求；公开 API 与 global-facts 优先语义保持；B-1 正确性论证（双检 + fallible-first + canonicalize 不重入）成立。
- **代码质量**：PASS with 2 Minor + 5 FYI；无 Critical、无 Important。
- **建议**：合入 main，Minor #1 与 #2 记入 follow-up ledger（不阻塞本次合入）。

---

## 4. 验证脚本建议（可选，由合入者执行）

```bash
cargo check --workspace                  # workspace 编译
cargo test -p northhing-core --features product-full --lib -- \
  agentic::coordination::tests::subagent_ports \
  service::agent_memory::auto_memory::tests
cargo test -p northhing-core --features product-full --lib  # 全量 1134
cargo check -p northhing                # 桌面面
pnpm run fmt:rs                         # 仅改动的 .rs
```

报告已提供完整命令 + 输出，本审查不再重跑（验证前置 = implementer 已交付）。