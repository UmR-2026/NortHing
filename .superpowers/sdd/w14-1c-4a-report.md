# W14-1c-4a Report — init gate 测试局部重写

状态：**DONE**（BASE `b7675d1`，工作树验证）

## 改动清单

1. `src/crates/assembly/core/src/kernel_facade/lifecycle.rs`（修改）
   - `run_init_gate` 瘦身为委托：`run_init_gate_with(&FACADE_READY, &INIT_STATE, &INIT_NOTIFY, init).await`。
   - 新增 inner 函数 `run_init_gate_with`（`pub(super)`，无新可见性扩张），接收 `ready: &AtomicBool, state: &AsyncMutex<InitState>, notify: &Notify`。状态机迁移、错误文案（"init_core timed out waiting for concurrent initialization"）、Notify 唤醒、ready 置位时机、`info!` 日志逐行不变（diff 为纯参数化改名）。
2. `src/crates/assembly/core/src/kernel_facade/tests.rs`（修改）
   - `test_init_gate_lifecycle_all_scenarios` 重写：每个 scenario 用局部 fresh `AtomicBool` / `tokio::sync::Mutex<InitState>` / `Notify` 调 `run_init_gate_with`。**测试体内 `FACADE_READY` / `INIT_STATE` / `INIT_NOTIFY` 三个名字的读写全部消失**（grep 证实，仅剩 `run_init_gate_with` 调用）。
   - Scenario 1（并发只跑一次）/ 2（Ready 幂等不重跑）/ 3（失败回退 NotStarted 可重试）断言语义全保留，并顺手加局部 `ready` 翻转断言（成功置位 / 失败不置位），覆盖 FACADE_READY 置位时机的局部门禁行为。
   - Scenario 4（fresh facade `list_sessions()` 未 init 返回 `KernelError::Internal` 不 panic）：删除原全局重置前奏——该断言只依赖新 facade 无 coordinator（`mod.rs` `coordinator()`），与进程级门禁无关，天然并发安全。
   - mojibake 顺手清（in-scope 配额）：原 :401/:433/:468 场景注释（`鈥?`/`涔嬪悗鍐嶈皟`/`鈫?`）随重写改为英文；:650 分隔线 `鈹€` 串改为与其它分隔线一致的 `──`。
   - 导入：`run_init_gate/FACADE_READY/INIT_STATE` → `run_init_gate_with`；新增 `use tokio::sync::{Mutex as AsyncMutex, Notify};` 与 `AtomicBool`。
3. `.superpowers/sdd/w14-1c-4a-report.md`（本文件，新增）

`kernel_facade/mod.rs` 未动（附带条件 #9 / C1）。rot 闸：diff 新增行中 `let _ =` 计数 = 0（rg 证实），371/388 基线不涨。S4：未引入有已知上限的简化，无新增 `ponytail:` 注释需求。

## inner 函数签名

```rust
pub(super) async fn run_init_gate_with<Fut>(
    ready: &AtomicBool,
    state: &AsyncMutex<InitState>,
    notify: &Notify,
    init: Fut,
) -> Result<(), KernelError>
where
    Fut: std::future::Future<Output = Result<(), KernelError>>,
```

## 验证输出摘录

全部经 `rustup run stable-x86_64-pc-windows-msvc cargo ...` + `cmd /c` 重定向（skill `long-running-shell`）。

1. `cargo check --workspace` → `Finished dev profile in 1m 03s`，日志 542 行无任何 `error`（并行 coder 的 `path_manager_uninit.rs` 未破坏 lib 编译）。
2. ⚠️ **偏差（C3 记录）**：brief 原命令 `cargo test -p northhing-core --lib kernel_facade` **在 BASE 上即无法裸编译**——`default = []`（core Cargo.toml:172），`lib.rs:11` 把 `pub mod agentic` 门控在 `product-full` 后，而本单未触碰的干净文件 `service/skill_watch.rs:12-13`、`service/config/service.rs:359` 无条件引用之 → 3×E0433。`cargo check --workspace` 因 desktop 消费方 feature 合并掩盖了该问题。**与本 diff 无关**（基线预存，属 P2-15 类"desktop 能编 ≠ 单 crate 能编"形态，建议后续单跟踪）。故按 crate 的组装边界补 `--features product-full` 执行：
   - `cargo test -p northhing-core --lib --features product-full kernel_facade` → `CARGO_TEST_OK`
   - 构建一次后直接跑测试二进制（CWD=包根）：`test result: ok. 53 passed; 0 failed; 0 ignored`（含 `test_init_gate_lifecycle_all_scenarios ... ok`），本模块零警告。
3. `--test-threads=1` 串行复跑同一二进制 → `test result: ok. 53 passed; 0 failed; 0 ignored`。并行/串行双绿（本测试原为全局污染点，现已解耦）。
4. `git diff --check` → exit 0（仅 lifecycle.rs 的 CRLF→LF 换行提示，非 whitespace error）。

## 遗留

- 裸 `cargo test -p northhing-core`（不带 features）的基线编译缺口不在本单范围，未修。
- 并发计时余量（50ms/10ms/5ms sleep）沿用原测试数值，未改语义。
