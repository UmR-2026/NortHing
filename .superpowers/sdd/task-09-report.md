# Task 9 Report: 修复 6 个 pre-existing 测试失败

分支 `fix/backend-debug-0731`，基线 `1a65fc1`。不 git commit。

## 1. 根因确认与修正

### 组 A：auto_memory prompt_injection x3 -- 测试非 hermetic

**brief 根因：确认正确。**

`build_workspace_agent_memory_prompt`（auto_memory.rs:243-286）的 facts 来源：
1. 开**全局** `MemoryDb::open(&default_memory_db_path())`，`default_memory_db_path()`（memory_db.rs:792）直接用 `dirs::config_dir().join("northhing")...`，**不走 PathManager**，所以 PathManager 的 env override / `with_user_root_for_tests` 对它无效。
2. `db.get_facts(Some(&workspace_key))`（memory_db.rs:231-287）的 SQL 是 `WHERE status='active' AND (scope='global' OR (scope='workspace' AND workspace_key=?1))`。**无论 workspace_key 是什么随机 UUID，global scope 的 facts 都会被返回**（global 优先是产品设计，见 `select_facts_respects_scope_global_first`，不改此语义）。
3. DB 结果非空时不 fallback 到 jsonl（auto_memory.rs:253-255）。

本机真实全局 DB 含 global facts -> 三个测试的 prompt 混入真实用户 facts：
- `prompt_injection_with_facts_includes...`：section 存在但不含测试写入的 "I prefer pnpm"（DB 有 global facts，jsonl fallback 被跳过）。
- `prompt_injection_without_facts_excludes...` / `prompt_injection_degrades_when_facts_file_unreadable`：期望空 facts 却拿到真实 global facts。

`prompt_injection_with_select_facts_budget_limit` 侥幸通过，因为它只断言 section 存在。

### 组 B：subagent_ports cancel/timeout x3

**brief 根因：部分正确，需修正。**

#### B-1. GlobalConfigManager::initialize TOCTOU（真产品 bug，brief 定位正确）

`global.rs:79-95`（修复前）：
```rust
if Self::is_initialized() { return Ok(()); }   // L80 检查（无锁）
let (sender, _) = broadcast::channel(100);
CONFIG_UPDATE_SENDER.set(sender)                // L86 OnceLock set
    .map_err(|_| ... "Failed to initialize config update sender")?;
let config_service = Arc::new(ConfigService::new().await?);  // L90 可能失败
GLOBAL_CONFIG_SERVICE.set(service_wrapper)...   // L93
```

TOCTOU：并行测试同时通过 L80 `is_initialized()=false`，一个 `CONFIG_UPDATE_SENDER.set` 成功、另一个失败。若 `ConfigService::new` 也失败，sender 已 set 但 service 未 set，`is_initialized()` 仍 false（检查 `GLOBAL_CONFIG_SERVICE`），后续 initialize 永远在 L86 失败 -- 进程内不可逆。

这是真 bug，桌面运行时多入口并发 initialize 同样会踩。**已修复。**

#### B-2. cancel 测试失败的直接根因（brief 未覆盖，需修正）

**独立验证发现：cancel 测试失败与 initialize TOCTOU 无关。**

在 main 基线（stash 我的修改后）**单独跑** `subagent_cancel_propagates_to_result`（无并行，无 TOCTOU）仍失败（0.08s），panic 为 `Cancelled("Subagent task has been cancelled")`，**无** initialize eprintln。

通过临时插桩日志定位：
- phase2 的步骤 1-5（start_dialog_turn / register / spawn 等）仅耗时 **659us**，select loop 几乎立即开始。
- `init_turn` 仅耗时 **4.9ms**（agent_registry 1.7ms + resolve_model 1.1ms + get_factory 0.06ms + get_client_resolved 2ms）。
- 但 `execution_task`（`execute_dialog_turn` -> `init_turn` + `tick()` chat request）**未在 50ms 内完成**。从 `tests_timeout`（单独跑 0.84s 通过）推断 execution_task 完整耗时约 **0.84s**（LLM chat request 网络往返）。

`tests_cancel` 在 50ms 后 `cancel_token.cancel()`。由于 0.84s >> 50ms，select loop 的 `cancel` arm 先于 `join_result` arm 触发，phase2 返回 `Err(Cancelled)`。测试 `.expect("phase 2 should return Ok")` panic。

**为什么其他 subagent_ports 测试不失败：**
- `tests_success` / `tests_error` / `tests_timeout` / `tests_parent_chain`：**直接 `await` phase2**（不 spawn），execution_task 有充足时间完成 -> `join_result` arm 先 -> `Ok`。
- `tests_concurrent`：spawn 但**不 cancel**（注释明确："No cancel - let phase 2 run to completion"）。
- `tests_abort_exit`：不调 phase2，直接调 `persist_aborted_subagent_exit`。
- 只有 `tests_cancel` **spawn + 50ms cancel**，触发竞争。

测试注释（tests_cancel.rs:9 / tests_concurrent.rs:33）均假设 "dev environment's missing LLM" 使 `init_turn` 微秒级失败。但在本机（有真实 LLM 凭证）`AIClientFactory` 已初始化，`init_turn` 成功获取 client，`tick()` 发出真实网络 chat request，耗时远超 50ms。

**`tests_timeout` 的 flaky 与 TOCTOU 相关**：单独跑通过（0.84s < 1s timeout），但并行跑时多个测试同时调 `ensure_global_config_for_tests` -> `initialize` TOCTOU -> eprintln + 竞态，可能影响时序。修复 initialize TOCTOU 后并行不再有竞态。

## 2. 改动文件与行号

| 文件 | 改动 | 行号 |
|---|---|---|
| `service/config/global.rs` | 新增 `INIT_MUTEX` static | L20-31 |
| `service/config/global.rs` | 重写 `initialize`：double-checked locking + fallible-work-first | L86-148 |
| `service/agent_memory/memory_db.rs` | `default_memory_db_path` 加 `#[cfg(test)]` thread-local override | L792-798 |
| `service/agent_memory/memory_db.rs` | 新增 thread-local seam：`TEST_MEMORY_DB_PATH` / `MemoryDbPathGuard` / `with_test_memory_db_path` / `unique_test_memory_db_path` / `Drop` | L800-874 |
| `service/agent_memory/mod.rs` | test-only re-export | L19-20 |
| `service/agent_memory/auto_memory.rs` | mod tests 加 import + 4 个测试加 `MemoryDbPathGuard` | L406-411, L429/458/476/495 |
| `agentic/coordination/tests/subagent_ports/mod.rs` | 移除 `AIClientFactory` import | L32 删除 |
| `agentic/coordination/tests/subagent_ports/mod.rs` | `ensure_global_config_for_tests` 不再初始化 `AIClientFactory` | L132-156 |

## 3. Seam 设计说明

### 组 A：thread-local DB 路径 override

**机制：** `default_memory_db_path()` 在 `#[cfg(test)]` 下先查 thread-local `TEST_MEMORY_DB_PATH`。`with_test_memory_db_path(path)` 返回 RAII `MemoryDbPathGuard`，构造时把 path 存入 thread-local 并保存旧值，`Drop` 时恢复旧值并 best-effort 删除隔离 DB 文件（含 `-wal` / `-shm` sidecar）。

**为什么用 thread-local 而非进程级 Mutex：**
- `#[tokio::test]` 默认 current-thread runtime。`default_memory_db_path()` 是同步函数，在测试线程内调用（`MemoryDb::open` / `get_facts` 均同步）。thread-local 对该线程全程可见。
- `cargo test` 默认多线程（每测试一线程）。thread-local 天然 per-thread 隔离：组 A 测试线程设 override，其他 1128 个测试线程的 `default_memory_db_path()` 返回 `None` -> 走真实路径，**不受影响，不被阻塞**。
- 进程级 Mutex 会让所有用 `default_memory_db_path()` 的测试串行（含 `turn_persist.rs` / `dream.rs` 产品代码路径及 `query_aware_tests`），引入不必要的串行化和潜在死锁风险。

**还原机制：** `Drop` 恢复 thread-local 旧值（支持嵌套 guard）。`let _db_guard = ...` 绑定确保 guard 活到测试函数结束（不能用 `let _ = ...`，那会立即 drop）。

**并行安全性论证：**
- 每个测试用 `unique_test_memory_db_path()`（UUID）生成独立路径，并发测试不共享 DB 文件。
- thread-local 不跨线程，无需同步。
- `query_aware_tests`（auto_memory.rs:560-588）不设 guard，仍用真实 DB -- 这是 pre-existing 行为，不在本任务 6 个失败测试范围内，且其 fact 是 `Workspace` scope + 随机 workspace_key，不影响组 A 测试的 `get_facts`。
- 产品代码（`dream.rs` / `turn_persist.rs`）在 `#[cfg(test)]` 下才查 thread-local；非 test 构建完全不感知 override。

**验证无连带破坏：** 全量 1134 测试两次通过（见下）。

### 组 B-1：initialize double-checked locking + fallible-work-first

**机制：**
1. Fast path：`is_initialized()` 在锁外检查（已初始化时无锁、无 await）。
2. 获取 `INIT_MUTEX`（`std::sync::OnceLock<tokio::sync::Mutex<()>>`，因为 `tokio::sync::Mutex::new` 非 const fn）。
3. Double-check：持锁后再查 `is_initialized()`，防止等待期间已被另一个 caller 完成初始化。
4. **所有 fallible work（`ConfigService::new`）在 `OnceLock::set` 之前完成。** 若失败，Nothing 被 set，后续 retry 从干净状态开始，不留半初始化态。
5. `CONFIG_UPDATE_SENDER.set` + `GLOBAL_CONFIG_SERVICE.set`：double-check 保证此时两个 OnceLock 均为空，首次 set 必成功。

**锁作用域：** `_guard` 持续到 `initialize` 函数结束（含 canonicalize）。canonicalize 在锁内执行，但它是幂等的，且 initialize 是低频调用（启动 / 测试 setup），锁开销可忽略。

**保持的语义：**
- `is_initialized()`：不变（检查 `GLOBAL_CONFIG_SERVICE`）。
- `subscribe_updates()` / `broadcast_update()`：不变（检查 `CONFIG_UPDATE_SENDER`）。
- 公开 API 签名：不变。
- 订阅语义：initialize 成功后 sender 已 set，可 subscribe；initialize 失败时 sender 未 set，subscribe 返回 `None`（合理：未成功初始化）。修复前半初始化态下 subscribe 返回 `Some`（旧 sender）但 `service()` 失败 -- 这是 bug 态，修复后不再出现。

### 组 B-2：subagent_ports 测试 hermetic 化

**机制：** `ensure_global_config_for_tests` 不再调用 `AIClientFactory::initialize_global`。`AIClientFactory` 保持未初始化，`init_turn` 的 `get_global_ai_client_factory()`（turn_init.rs:105-107）快速返回 `Err`（微秒级），`execution_task` 立即完成，select loop 的 `join_result` arm 先触发，phase2 返回 `Ok`（4 fields populated）。

**为什么这正确：**
- 所有 subagent_ports 测试的注释（tests_cancel.rs:9, tests_concurrent.rs:33, tests_success.rs:50, tests_error.rs:41）均声明测试走 "LLM error path" / "no LLM is reachable"。不初始化 AIClientFactory 恰好实现这一假设，使测试真正 hermetic。
- 测试断言均只检查 `phase2` 返回 `Ok` + 4 dead-code fields populated（`assert_secondary_fields_populated`），不检查 execution_task 的 `result` 是否成功。`SubagentExecutionOutcome::Completed(join_result)` 的 `Ok(result)` 分支（lifecycle.rs:321-387）无论 `result` 是 `Ok` 还是 `Err` 都返回 `Ok(SubagentPhase2Output)`。
- `build_test_coordinator_with_mock_tool` / `ensure_global_config_for_tests` 仅被 subagent_ports 测试使用（grep 确认），不影响其他 1128 个测试。
- `GlobalConfigManager::initialize` 仍调用（config service 初始化），因为 phase1/phase2 依赖 config service（如 `ConfigService` 用于 resolve_model 等）。

## 4. 验证命令与输出

### 命令 1：6 个目标测试

```
> cargo test -p northhing-core --features product-full --lib -- agentic::coordination::tests::subagent_ports service::agent_memory::auto_memory::tests

running 14 tests
test agentic::coordination::tests::subagent_ports::tests_abort_exit::aborted_timeout_exit_persists_failed_and_returns_timeout ... ok
test agentic::coordination::tests::subagent_ports::tests_abort_exit::aborted_cancelled_exit_persists_and_clears_registry ... ok
test agentic::coordination::tests::subagent_ports::tests_timeout::subagent_timeout_returns_partial ... ok
test agentic::coordination::tests::subagent_ports::tests_parent_chain::subagent_parent_chain_propagates_through_nested_calls ... ok
test agentic::coordination::tests::subagent_ports::tests_error::subagent_error_propagates_to_result ... ok
test agentic::coordination::tests::subagent_ports::tests_concurrent::subagent_concurrent_cancellations_are_independent ... ok
test agentic::coordination::tests::subagent_ports::tests_success::subagent_success_completes_with_text ... ok
test agentic::coordination::tests::subagent_ports::tests_success::subagent_success_transmits_large_payload ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_degrades_when_facts_file_unreadable ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_select_facts_budget_limit ... ok
test agentic::coordination::tests::subagent_ports::tests_cancel::subagent_cancel_propagates_to_result ... ok
test agentic::coordination::tests::subagent_ports::tests_cancel::subagent_cancel_takes_precedence_over_timeout ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_facts_includes_remembered_facts_section ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_without_facts_excludes_remembered_facts_section ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
```

### 命令 2：core 全量回归（跑两次）

**第一次：**
```
> cargo test -p northhing-core --features product-full --lib
test result: ok. 1134 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.41s
```

**第二次：**
```
> cargo test -p northhing-core --features product-full --lib
test result: ok. 1134 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.41s
```

两次均 1134 passed（原 1128 + 修复的 6），0 failed，无 flaky。

### 命令 3：桌面面不破

```
> cargo check -p northhing
warning: `northhing` (bin "northhing") generated 1 warning
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 19s
```

（1 个 warning 是 pre-existing `dead_code` for `save_app_settings`，非本次引入。）

## 5. 遗留观察

1. **`AIClientFactory::initialize_global` 也有 TOCTOU**（client_factory.rs:224-263）：同样的 check-then-set 模式（`is_global_initialized` -> `GLOBAL_AI_CLIENT_FACTORY.set`）。在 subagent_ports 测试中不再触发（因为不再调用它），但桌面运行时多入口并发仍可能踩。这不是本任务 6 个失败的根因，未修复，建议后续单独处理（同样的 double-checked locking 模式可适用）。

2. **`query_aware_tests::build_query_aware_facts_reminder_returns_some_with_matching_fact`**（auto_memory.rs:560-588）仍向真实全局 DB 写入 fact（非 hermetic），但它是 pre-existing 通过测试，不在本任务范围。其 fact 是 `Workspace` scope + 随机 workspace_key，不污染组 A 测试。若后续要全量 hermetic 化，可给它也加 `MemoryDbPathGuard`。

3. **`ensure_global_config_for_tests` 的 `DONE: OnceLock<()>`** 仍有 check-then-set TOCTOU（mod.rs:135-145），但修复 `initialize` 的并发安全后，两个并行 caller 都调 `initialize`（都返回 `Ok`，幂等），`DONE` 的 TOCTOU 只导致 `initialize` 被多调一次（无害）。

4. **组 B 根因修正**：brief 将 cancel 测试失败归因于 initialize TOCTOU。独立验证（main 基线单独跑 cancel 仍失败 + 临时插桩日志）表明 cancel 测试的直接根因是 execution_task 网络耗时 > 50ms cancel 窗口，与 TOCTOU 无关。initialize TOCTOU 是独立的真 bug（影响 timeout 测试并行 flaky），已一并修复。
