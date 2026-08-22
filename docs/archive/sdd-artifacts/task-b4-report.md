# Task B4 报告 — FU-5 `AIClientFactory::initialize_global` TOCTOU [concurrency]

- 分支：`fix/backend-followups-0804`；派发时 HEAD `6868377`；本任务 commit **`50b0f44`**（`fix(core): serialize AIClientFactory global init with double-checked locking (FU-5)`）。
- 提交文件清单（`git show --stat HEAD` 实核）：仅 2 个文件，209 insertions / 39 deletions：
  - `src/crates/assembly/core/src/infrastructure/ai/client_factory.rs`
  - `.superpowers/sdd/tech-debt-followups.md`
- 工作区最终状态：仅剩未追踪的派发物 `task-b4-brief.md`（按 brief §6 不入 commit）；`generated_locale_contract.rs` 为 gitignore 生成物（`src/crates/assembly/core/src/service/i18n/`，不入库）。

## 1. 改动清单（file:line，基于 commit `50b0f44`）

| 位置 | 内容 |
|---|---|
| `client_factory.rs:222-232` | 新增 `static AI_CLIENT_FACTORY_INIT_MUTEX: std::sync::OnceLock<tokio::sync::Mutex<()>>`，附与 `global.rs:20-30` 同质英文 doc（为何 `tokio::sync::Mutex`、为何 `std::sync::OnceLock` 包裹、fast path 免锁说明）。 |
| `client_factory.rs:240-271` | 新增可测 helper `async fn init_once_with<F, Fut>`（双检锁骨架：fast path → 取锁 → 锁内 double-check → 调用方 closure；double-check 命中打 `debug!("{} already initialized, skipping", init_name)`）。 |
| `client_factory.rs:273-288` | `initialize_global` doc 注释：并发保证 + "double-check 之后 set 必成功" 理由（对齐 `global.rs:90-99` 写法），引用 `6574b01`。 |
| `client_factory.rs:286-331` | `initialize_global` 改为调用 `init_once_with`；原 fast path 保留（现由 helper fast path 承担）；fallible work（`get_global_config_service`、factory 构造）顺序不变、全部在 `OnceLock::set` 之前；`set` 的 `map_err` 防御保留；**全部 P0-E 计时日志逐字保留**。 |
| `client_factory.rs:418` | tests 模块 import 增加 `init_once_with`。 |
| `client_factory.rs:490-496` | 注释说明为何抽 helper 测试（进程级 OnceLock 与 lib 测试二进制共享）。 |
| `client_factory.rs:499-543` | 新测试 `init_once_with_concurrent_callers_run_build_exactly_once`（`#[tokio::test(flavor="multi_thread", worker_threads=4)]`，8 个并发 caller，断言全 `Ok`、cell 恰 set 一次、build 恰执行一次）。 |
| `client_factory.rs:546-585` | 新测试 `init_once_with_failed_build_leaves_no_half_initialized_state`（build 返回 `Err` → cell 保持空；后续重试成功 → 无半初始化态）。 |
| `tech-debt-followups.md:5` | 状态汇总行：FU-5 翻为 `resolved`，`全部完成`。 |
| `tech-debt-followups.md:55-56` | FU-5 标题下加 `> **状态**：resolved — Task B4 ...` + 修复摘要（照 FU-1..FU-4 格式，含验证结论）。 |

## 2. 修复前后逻辑对照

**修复前**（`6868377`）：
```
fast path is_global_initialized() → (漏空窗) → get_global_config_service().await
→ 构造 factory → GLOBAL_AI_CLIENT_FACTORY.set(wrapper)  ← 后到者 set 失败
→ map_err → 返回 Err("Failed to initialize global AIClientFactory")
```
check 与 set 之间无互斥：N 个并发 caller 全部通过 fast path → 全部重复做 config service 往返 + 构造 → 后到者 `set` 失败向调用方返回伪错误（尽管单例已就绪）。

**修复后**（`50b0f44`，套用 `6574b01` `GlobalConfigManager::initialize` 模式）：
```
fast path is_global_initialized()（免锁，稳态零等待）
→ AI_CLIENT_FACTORY_INIT_MUTEX.get_or_init(|| Mutex::new(())).lock().await
→ 锁内 double-check is_global_initialized() → 命中则 debug! 并 Ok(())
→ fallible work（get_global_config_service + 构造）顺序不变，全部在 set 之前
→ GLOBAL_AI_CLIENT_FACTORY.set(wrapper)（双检后必成功；map_err 防御保留）
```
不变量：a) 只有一个 caller 会执行 fallible work + set；b) 并发 caller 全部 `Ok`，无伪 `Err`；c) fallible work 失败 → 无任何 set → 无半初始化态 → 重试干净起步；d) `get_global`/`update_global`/`get_or_create_client` 语义未动。

日志语义：初代 caller 的 P0-E 日志（`enter` / `before get_global_config_service` / `after ...` / `after ...set` / `done total`）逐字保留；fast path 命中保持静默（原行为）；锁内 double-check 命中新增一条英文 `debug!("AIClientFactory already initialized, skipping")`（brief §2 第 3 点要求，风格对齐 `global.rs:115-116`）。

## 3. 测试方案选择：B（抽 helper + hermetic 并发测试）

按 brief §3 优先级，先评估 A，判定不可 hermetic，转 B。**A 不可行的具体证据**：

1. **本机配置含真实凭据**：`C:\Users\UmR\AppData\Roaming\northhing\config\app.json` 存在且 `ai.models` 有 2 个 enabled `auth=api_key` 模型（longcat / sensenova）→ `get_global_config_service()` 在本机必成功，`initialize_global` 走真实路径。
2. **进程级 OnceLock 跨测试共享**：`--lib` 测试二进制单进程运行全部 `#[cfg(test)]` 模块；`GLOBAL_AI_CLIENT_FACTORY` 一旦被 A 测试 set，同二进制的 `subagent_ports` 测试（`src/crates/assembly/core/src/agentic/coordination/tests/subagent_ports/mod.rs:131-146`）spawn 的执行任务在 `init_turn` 处 `get_global_ai_client_factory()` 不再 fail-fast，而会**用本机真实 API key 发起真实 LLM 网络请求**（`turn_init.rs:101-109` 的 W4-P 路径）。
3. **作者曾实测该组合导致测试失败**：`mod.rs:143-146` 明言 "Previously, on machines with real LLM credentials the spawned task would block on a network chat request for ~0.8s, far exceeding the 50ms cancel window in tests_cancel, so the cancel arm won instead of join_result and the tests failed"——这正是 A 会复现的非 hermetic 失败面；且 `tests_concurrent.rs:9-12` 说明 Cancel/超时臂在 `SubagentPhase2Output` 构建前返回 `Err`（字段只在 Completed 臂，`coordinator.rs:4767-4771`），网络阻塞 >1s 时 `tests_timeout` 结果会翻转。
4. **结论**：A 依赖真实用户配置/网络凭据且会改变同进程其它测试的可观测行为（真实出网、烧真实额度、时序翻转），不属于 hermetic 测试；OnceLock 无法在测试内 reset，顺序无法保证。

**B 落地**（按 brief §3 示例签名 + "抽取必须保持 `initialize_global` 外部行为与日志不变"的硬约束）：`init_once_with` 只负责"fast path → 锁 → double-check → 调 closure"，closure 内保留原 fallible-work + set + P0-E 日志 → 外部行为与日志零变化（对比 `client_factory.rs:286-333` 与旧 `224-263`，P0-E 五条日志逐字一致）。测试用 test-local `Arc<OnceLock<()>>` + `Arc<OnceLock<Mutex<()>>>`，完全无全局/网络/磁盘依赖：
- `init_once_with_concurrent_callers_run_build_exactly_once`：8 并发 → 全 `Ok`、cell 恰 set 一次、`build` 原子计数器 = 1（幂等）。
- `init_once_with_failed_build_leaves_no_half_initialized_state`：build `Err` → 错误传播、cell 保持空；重试成功 → 无半初始化态。

brief §3 目标断言逐条覆盖：并发幂等（全 Ok 无伪 Err）✓；无半初始化态（失败不留已 set 态，重试干净）✓。

## 4. 验证命令原文输出

> cargo 前缀统一：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`（本机必需，已执行）。

### 4.1 `cargo check -p northhing-core --features product-full`
```
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 38.24s
```
- 通过；19 条 warning 全部为既有（agent_memory/memory_db.rs 等），`client_factory.rs` 零 warning（专项 grep 无输出）。
- 前置：worktree 缺 gitignore 生成物 `generated_locale_contract.rs`，按 brief §6 跑 `node scripts/generate-i18n-contract.mjs`（输出 `[i18n:generate] Wrote 6 generated i18n contract file(s).`），其副产物 `src/apps/relay-server/static/homepage/i18n.shared.json` 已 `git checkout --` 还原，不入 commit。

### 4.2 `cargo test -p northhing-core --features product-full --lib client_factory`
```
running 6 tests
test infrastructure::ai::client_factory::tests::auto_model_selectors_normalize_to_primary_for_client_lookup ... ok
test infrastructure::ai::client_factory::tests::init_once_with_failed_build_leaves_no_half_initialized_state ... ok
test infrastructure::ai::client_factory::tests::resolve_fast_selection_falls_back_to_primary_when_fast_missing ... ok
test infrastructure::ai::client_factory::tests::resolve_model_reference_supports_id_name_and_model_name ... ok
test infrastructure::ai::client_factory::tests::resolve_fast_selection_falls_back_to_primary_when_fast_is_stale ... ok
test infrastructure::ai::client_factory::tests::init_once_with_concurrent_callers_run_build_exactly_once ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1135 filtered out; finished in 0.00s
```
- 新增 2 条测试均出现在结果里并通过。

### 4.3 `cargo test -p northhing-core --features product-full --lib`（基线核对，做了 stash 对照）
提交后实跑：
```
test agentic::persistence::metadata_subhandlers::tests::bench_session_metadata_page_vs_full_list ... ignored, local performance benchmark; prints timing data only
test result: ok. 1140 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.62s
```
基线核对：`git stash` 掉本改动后实跑 B2 状态：
```
test result: ok. 1138 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.52s
```
- 基线 = 1138 passed + 1 ignored（`bench_session_metadata_page_vs_full_list`，`agentic::persistence` 既有 `#[ignore]` 性能基准，与本次无关）= 1139 总（对应 brief 的 "1139/1139"）。
- 本任务后 = 1140 passed + 1 ignored = 1141 总 = 基线 1139 + **新增 2 条**，0 fail。`--list` 计数 1141 交叉确认。
- 无失败、无可归因项。

### 4.4 其它
- `pnpm run fmt:rs`（只碰改动文件）：`[format-changed-rust] Formatting 1 Rust file(s).`；格式化后 `git diff --check` 无空白错误，`cargo check` / `client_factory` 测试复跑仍绿。
- 文件行数：`client_factory.rs` 422 → **592 行**（<800，brief §6 约束满足）。
- `git status --short` 提交后：仅 `?? .superpowers/sdd/task-b4-brief.md`（派发物，不入 commit）。

## 5. 范围外未动的确认

- 未改 `get_global` / `update_global` / `get_or_create_client`（`client_factory.rs:335-367 / 140-217` 语义零变化，diff 中无相关 hunk）。
- 未碰 MCP config（B1）、LSP manager（B2）、desktop settings（B3）任何文件。
- 未跑 `cargo check --workspace`（brief §5：被上游 embed-resource 3.0.11 阻断，交 CI）。
- 未跑裸 `cargo fmt`（仅 `pnpm run fmt:rs`）。
- 未 commit `task-b4-brief.md` / 报告文件 / `generated_locale_contract.rs` / `i18n.shared.json` 副产物。

## 6. 遗留观察项

1. **`initialize_global` 双检命中日志级别为 `debug!`**（`"AIClientFactory already initialized, skipping"`）：与 `global.rs:115-116` 同款，生产默认日志级别下不可见；如需审计并发 init 命中，可后续上调 `info!`——本次按 brief 要求选现有风格。
2. **P0-E `enter` 日志语义微调**：修复前凡过 fast path 的 caller 都会打印 `enter`（含随后被 set 失败打回者）；修复后 `enter` 仅由真正执行初始化的首个 caller 打印，其余 caller 在锁内 double-check 命中走 `debug!`。冷启动挂起定位能力不变（真正慢的初始化路径日志逐字保留），见 §2。
3. **`GLOBAL_AI_CLIENT_FACTORY` 与 `subagent_ports` 的耦合**：本修复未改变"测试二进制内初始化工厂会让 subagent_ports 走向真实 LLM 出网"这一既有事实（A 方案不可行的根源）；若未来想让 A 方案可行，需先给 subagent_ports 的执行任务注入 fail-fast 语义（例如测试专用 runtime handle），属范围外课题。
4. **`init_once_with` 未设泛型上限**（`Fut` 无 `Send`/`'static` bound）：与调用点匹配（`initialize_global` 为 `async` 非 `'static` 上下文）；若未来移到需要 spawn 的宿主，需加 bound。

## 7. 偏离 brief 的显式声明

- **测试方案由 A 降级为 B**：brief §3 允许（A 不可 hermetic 时转 B），并在 §0 要求"两条都不可行时上报，不得静默降级为无测试"——本任务**有测试**（B 方案两条并发测试），未静默无测试。A 不可行的证据链见 §3。
- **helper 签名**：brief §3 示例为 `init_once_with<T, F, Fut>(cell, mutex, build)`；实现改为 `init_once_with(is_initialized, init_mutex, init_name, initialize)`（不带 `T`，由调用方 closure 负责 set）。理由：brief §2 硬性要求"`initialize_global` 外部行为与 P0-E 日志不变"，`set` 及其前后两条 P0-E 计时日志必须留在 `initialize_global` 内；若按示例让 helper 执行 `cell.set`，`"P0-E: after GLOBAL_AI_CLIENT_FACTORY.set, took {:?}ms"` 将无法从调用方发出，违反日志保持约束。测试仍完整覆盖 brief 要求的两类断言（build 恰一次 / 失败无半初始化态）。
- **doc sync 的 commit 引用**：`tech-debt-followups.md` 中 FU-5 状态行以 commit message 引用（`fix(core): serialize AIClientFactory global init with double-checked locking (FU-5)`），与 FU-1..FU-4 现有写法一致（它们也只引 message 不引 hash），且 doc 与代码同 commit，hash 不可前置知晓。
