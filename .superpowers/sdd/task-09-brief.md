# Task 9 Brief: 修复 6 个 pre-existing 测试失败（回归健康）

仓库：`E:\agent-project\.worktrees\northing-backend-debug`（分支 `fix/backend-debug-0731`，基线 1a65fc1）。不 git commit。

## 背景

回归扫描发现 `cargo test -p northhing-core --features product-full --lib` 有 6 个失败，已在 main 基线仓库复现确认 **pre-existing**（非本分支引入）。分两组，根因已定位，你需要验证根因并修复。

## 组 A：auto_memory prompt_injection ×3（测试非 hermetic）

失败测试（`src/crates/assembly/core/src/service/agent_memory/auto_memory.rs`）：
- `prompt_injection_with_facts_includes_remembered_facts_section`（panic L447）
- `prompt_injection_without_facts_excludes_remembered_facts_section`（panic L465）
- `prompt_injection_degrades_when_facts_file_unreadable`（panic L506）

### 已定位根因

`build_workspace_agent_memory_prompt`（auto_memory.rs:248-285）的 facts 来源：
1. 先开**全局** `MemoryDb::open(&default_memory_db_path())`（`~/.northhing/` 下真实用户 sqlite）
2. `db.get_facts(Some(&workspace_key))` 返回该 workspace + **global scope** facts（global 优先是产品设计，见 `select_facts_respects_scope_global_first`，**不得改变此产品语义**）
3. DB 结果非空时**不** fallback 到 jsonl

本机真实全局 DB 含 facts → 三个测试的 prompt 混入真实用户 facts：
- L428 测试：section 存在但不含测试写入的 "I prefer pnpm"（DB 有数据，jsonl fallback 被跳过）
- L457/L494 测试：期望空 facts 却拿到真实全局 facts

佐证：`prompt_injection_with_select_facts_budget_limit`（L475，同逻辑）通过，因为它只断言 section 存在，真实 facts 恰好满足。

### 修复要求

测试必须 hermetic（不依赖本机环境），但不能破坏并行执行的其他 1128 个测试。可用机制：
- `PathManager` 支持 `northhing_HOME` / `northhing_E2E_HOME` env override（path_manager.rs:119-121），但全局 `GLOBAL_PATH_MANAGER` 是 `OnceLock`（path_manager.rs:162-183），进程内首次使用后 set_var 无效——直接 set_var 方案不可靠，不要采用。
- `PathManager::with_user_root_for_tests`（path_manager.rs:148）已存在 test-only 构造器，但 `path_manager_arc()` 返回全局单例无法注入。
- `default_memory_db_path()`（memory_db.rs:792）是另一个注入点。

建议方向（你可改进）：为测试增加注入 seam，例如 `#[cfg(test)]` 的全局 PathManager override（`path_manager_arc()` 先查 override）+ 测试间互斥守卫（全局 `std::sync::Mutex` 或 serial 机制），使受影响测试在隔离 home 下运行；或对 `default_memory_db_path` 加 test override。注意：override 生命周期结束后必须还原，且与并行测试共存——仔细设计锁的作用域，避免引入新 flaky。

## 组 B：subagent_ports cancel/timeout ×3（产品 initialize TOCTOU bug）

失败测试（`src/crates/assembly/core/src/agentic/coordination/tests/subagent_ports/`）：
- `tests_cancel::subagent_cancel_propagates_to_result`（tests_cancel.rs:46）
- `tests_cancel::subagent_cancel_takes_precedence_over_timeout`（tests_cancel.rs:101）
- `tests_timeout::subagent_timeout_returns_partial`（tests_timeout.rs:25）

panic 信息：
```
GlobalConfigManager::initialize failed in test setup: Configuration error: Failed to initialize config update sender
AIClientFactory::initialize_global failed: ... Global config service not initialized
```

### 已定位根因

`GlobalConfigManager::initialize`（`src/crates/assembly/core/src/service/config/global.rs:79-95`）：
```rust
if Self::is_initialized() { return Ok(()); }        // L80 检查
let (sender, _) = broadcast::channel(100);
CONFIG_UPDATE_SENDER.set(sender)                     // L86 OnceLock set
    .map_err(|_| ... "Failed to initialize config update sender")?;
let config_service = Arc::new(ConfigService::new().await?);   // L90 可能失败
GLOBAL_CONFIG_SERVICE.set(service_wrapper)...
```

TOCTOU：并行测试同时调 `initialize`，双双通过 L80 `is_initialized()=false` → 一个 `CONFIG_UPDATE_SENDER.set` 成功、另一个失败。且失败后处半初始化态（sender 已 set、service 未 set），`is_initialized()` 仍 false，后续 initialize 永远在 L86 失败——测试进程内不可逆。

同模块 `tests_abort_exit` 等其它测试通过，说明它们要么串行时序幸运、要么不触发并行 initialize。

### 修复要求

修产品代码（这是真 bug，桌面运行时多入口并发 initialize 同样会踩）：
- 使 `initialize` 并发安全：建议 `tokio::sync::OnceCell::get_or_try_init` 语义或全局初始化 `Mutex` 包住"检查+设置"全程
- 失败不得留下不可逆半初始化态（可重入重试）
- 保持现有公开 API 签名与行为（is_initialized、订阅 sender 的语义）不变
- 若 `ConfigService::new` 失败路径也有半初始化问题，一并处理

## 约束

- Logs must be English-only, with no emojis.
- 生产 .rs 单文件 <800 行（超出需 `// allow-god-file` 头注）。
- 不得改变 global facts 优先于 workspace facts 的产品语义。
- 不得改变 `GlobalConfigManager` / `AIClientFactory` 公开 API。
- 不修改其他 1128 个通过测试的行为；若你的 seam 影响共享单例，必须验证无连带破坏。
- 禁止运行裸 `cargo fmt` / `cargo fmt -p northhing-core`（会污染无关文件）。格式自查手工对齐。
- 遵循就近 AGENTS.md。

## 验证（必须全部通过并附输出）

```powershell
# 1. 六个目标测试（本机有真实用户数据的环境）
cargo test -p northhing-core --features product-full --lib -- agentic::coordination::tests::subagent_ports service::agent_memory::auto_memory::tests

# 2. core 全量回归（必须 1134 全过，即原 1128 + 修复的 6）
cargo test -p northhing-core --features product-full --lib

# 3. 桌面面不破
cargo check -p northhing
```

注意：#2 全量跑时并行默认线程数，正是触发组 B 竞态的条件——必须通过。多跑两次确认无 flaky。

## Report

写 `.superpowers/sdd/task-09-report.md`：根因确认/修正、改动文件+行号、seam 设计说明（锁作用域、还原机制、并行安全性论证）、验证命令+完整输出、遗留观察。
