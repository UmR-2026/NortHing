# W14-1c-4a Brief — init gate 测试局部重写（仲裁步骤 7 / B-2 最后一行）

> 来源：`.superpowers/sdd/w14-1b-arbitration.md` 步骤 7（§4 表 #7）+ §2.2 B-2 末行 + 附带条件 #8/#9 + 补遗。BASE：`b7675d1`。
> 性质：并发敏感，审慎执行。本单是 W14-1c 切片 4 的上半（另有一路并行 C/D 扫描，文件集不相交）。

## 目标（一句话）

`test_init_gate_lifecycle_all_scenarios`（`src/crates/assembly/core/src/kernel_facade/tests.rs:394`）当前直接读写真实全局 `FACADE_READY` / `INIT_STATE` / `INIT_NOTIFY`，同进程内与其它测试互相污染。重写为：**用局部 `AtomicBool` + 局部 `AsyncMutex<InitState>` + 局部 `Notify` 跑 init gate 逻辑，不再触碰任何真实全局门禁状态**。

## 现状（已磁盘核实，可直接信任）

- `src/crates/assembly/core/src/kernel_facade/lifecycle.rs`：
  - :20 `pub(super) static FACADE_READY: AtomicBool`
  - :77 `pub(super) static INIT_STATE: AsyncMutex<InitState>`
  - :78 `pub(super) static INIT_NOTIFY: Notify`
  - :24-75 `pub(super) async fn run_init_gate<Fut>(init: Fut)` —— 内部直接引用上述三个 static。
- `kernel_facade/tests.rs:393-511` `test_init_gate_lifecycle_all_scenarios`：3 个 scenario，每个开头 `FACADE_READY.store(false)` + 重置 `INIT_STATE`，断言并发只跑一次 / Ready 幂等 / 失败后状态回退可重试。
- 该测试文件内中文注释已 mojibake（如 `鈥?` `涔嬪悗鍐嶈皟`），在本单顺手修复为正常中文或英文注释（in-scope 顺手清配额）。

## Spec

- S1：**解耦方式**：把 `run_init_gate` 的门禁逻辑抽成接受状态参数的 inner 函数（形如 `async fn run_init_gate_with<Fut>(ready: &AtomicBool, state: &AsyncMutex<InitState>, notify: &Notify, init: Fut)`），生产 `run_init_gate` 改为用三个 static 委托调用 inner。生产行为一字节不变（状态机迁移、错误文案、Notify 唤醒、`FACADE_READY` 置位时机、`info!` 日志全部保持）。若你发现更小的等价形态（如泛型参数封装为小的 gate 结构体），可用，但生产委托路径必须保持。
- S2：重写 `test_init_gate_lifecycle_all_scenarios`：每个 scenario 用局部 fresh 的 `AtomicBool`/`AsyncMutex<InitState>`/`Notify` 调 inner 函数，**测试体内不再出现 `FACADE_READY` / `INIT_STATE` / `INIT_NOTIFY` 三个名字的读写**。3 个 scenario 的断言语义全部保留（并发只跑一次 / Ready 幂等不重跑 / 失败回退 NotStarted 后可重试成功）。
- S3：不动 `kernel_facade/mod.rs` 的 `FACADE` OnceLock 与核心结构（附带条件 #9）；不加任何新的 `pub` / `#[cfg(test)] pub` 可见性（inner 函数保持 `pub(super)` 即可，tests.rs 是同模块子模块可见）。
- S4：`ponytail:` 注释按附带条件 #8 要求不需要新增（本单无 OnceLock→Mutex 改造、无 test-threads 守护）；若你的实现引入了任何有已知上限的简化，补一条。

## Constraints

C1 只许动这两个文件：`src/crates/assembly/core/src/kernel_facade/lifecycle.rs`、`src/crates/assembly/core/src/kernel_facade/tests.rs`。**不许动 `kernel_facade/mod.rs`**。
C2 并行波警示：同工作树有另一路 coder 在跑 C/D 全仓扫描（可能 patch 其它测试文件），你的 diff 不感知他们；commit 时若发现别人 commit 进来，正常 commit 自己的，不 rebase 别人的。git 只点名 add 你的两个文件。
C3 以磁盘实际代码为准；brief 与磁盘冲突时以磁盘为准并把偏差写进 report。
C4 日志英文无 emoji；注释中文英文皆可但不许留 mojibake。

## 验证（report 必须含命令+输出摘录）

1. `rustup run stable-x86_64-pc-windows-msvc cargo check --workspace`（输出 `cmd /c` 重定向，禁 PS 管道；参考 skill `long-running-shell`）
2. `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --lib kernel_facade` —— 该测试模块全绿（含重写的 init gate 测试）
3. 同一测试命令加 `-- --test-threads=1` 再跑一遍，确认串行也绿（本测试原本就是全局污染点，串行/并行都要绿）
4. `git diff --check` 无 whitespace error
5. rot 闸自查：本单不得新增 `let _ =` 丢弃 Result 的位点（371/388 基线不许涨）

## 报告

写 `.superpowers/sdd/w14-1c-4a-report.md`：改动清单 / inner 函数签名 / 验证输出 / 状态词（DONE / DONE_WITH_CONCERNS / BLOCKED）。完成后自行 commit，message 含 `W14-1c-4a`。
