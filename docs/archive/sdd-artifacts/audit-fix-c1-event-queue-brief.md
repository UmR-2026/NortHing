# Task Brief — Audit C1: EventQueue 容量闸与广播投递解耦

## 1. 来源与验收标准（逐字）

来源：`.superpowers/sdd/reviews/project-audit-20260826/r2-core.md` Finding 1（Critical），用户 2026-08-26 拍板修复方向：

> 修复方向二选一：容量闸与广播投递解耦（推荐）/ 桌面 bootstrap 起常驻 drain 任务。
> **用户选定：容量闸与广播投递解耦**。

r2-core.md 原文验收锚点：

> Add a regression test asserting the desktop delivery path does not reject after 10k enqueues.

验收标准（逐条可机械核对）：

1. 桌面投递路径（broadcast）在堆满/无堆消费者时不再拒绝任何事件。
2. 默认模式（CLI/server 现有路径）行为逐字节不变：`test_enqueue_queue_full_and_critical_bypass` 原样绿。
3. 新回归测试：broadcast-only 模式下超过 `max_queue_size` 的 enqueue 全部 `Ok` 且订阅者全收到。
4. `cargo check --workspace` 与聚焦测试全绿，输出原文进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

2026-08-26 @ 66f08d1 grep 实时核实：

| 事实 | 锚点 |
|---|---|
| `enqueue` 容量闸在 broadcast 之前：堆满且非 Critical → `return Err(EventQueueFull)`，broadcast 根本不执行 | `src/crates/assembly/core/src/agentic/events/queue.rs:97-109` |
| `max_queue_size: 10000`（Default） | `queue.rs:36` |
| 堆的唯一生产消费者 = CLI（`dequeue_batch`） | `src/apps/cli/src/modes/exec.rs:128-129`、`src/apps/cli/src/modes/chat/run.rs:223` |
| 桌面生产路径只消费 broadcast（pump 订阅 broadcast，从不 pop 堆） | `src/crates/assembly/core/src/agentic/system.rs:87-103` |
| 队列构造点：`EventQueue::new(Default::default())` | `system.rs:34`（`init_agentic_system()` 内） |
| 桌面唯一生产入口链：`main.rs:59` → `init_core()` → `kernel_facade/lifecycle.rs:98` → `init_agentic_system()` | grep `init_core(` 已验证：kernel_facade 的 `init_core` 生产调用者仅 desktop `main.rs:59` |
| CLI 入口：`init_agentic_system_for_cli()` → `init_agentic_system()`（签名不变，继续走 Default） | `src/apps/cli/src/agent/agentic_system.rs:8-15` |
| server `bootstrap.rs:53` 用 Default —— frozen-experimental 面，本任务不动 | `src/apps/server/src/bootstrap.rs:53` |
| `src/apps/desktop/src/bin/w4_repro.rs:68` 是桌面 debug bin，直接调 `init_agentic_system()` —— 本任务不动 | 同上 grep |
| P2b（df47924）成果：`Err(EventQueueFull)` 可观测 + Critical 旁路 + `StreamEventSink` 满队 error log | `queue.rs:14-19, 99-105, 240-247` |
| `EventQueueFull` error 类型 derive `PartialEq, Eq` | `queue.rs:14` |

## 3. 复用侦察（强制）

动手前用 codegraph_explore 或 rg 查：`EventQueue` / `EventQueueConfig` / `init_agentic_system` 是否已有可复用的配置化入口；report 必须有「复用侦察」一节：查了哪些符号、复用了什么、若新写了已有能力的等价物逐条给理由。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. **配置项**：`EventQueueConfig` 增加 `heap_enabled: bool`；`Default` impl 中 `heap_enabled: true`（所有现有构造点行为不变）。字段 doc comment 写明：false = broadcast-only 模式，用于无堆消费者的宿主（桌面）；此模式永不返回 `EventQueueFull`。
2. **broadcast-only 语义**（`heap_enabled == false` 时 `enqueue`）：跳过容量检查与堆 push；照常执行 `broadcast_tx.send(envelope)`；`stats.total_enqueued` 照常 +1；不调 `notify.notify_one()`；`pending_events` 保持 0（`len()` 恒 0）；恒返回 `Ok(event_id)`。
3. **默认模式保值**：`heap_enabled == true` 时行为与现状逐字节一致（容量闸在 broadcast 前、满队非 Critical 返回 `Err(EventQueueFull)`、Critical 旁路、`StreamEventSink` 满队 error log）。现有测试 `test_enqueue_queue_full_and_critical_bypass` 不许改且必须绿。
4. **接线**：`system.rs` 新增 `pub async fn init_agentic_system_with_queue_config(config: EventQueueConfig) -> Result<AgenticSystem>`；`init_agentic_system()` 改为以 `EventQueueConfig::default()` 委托新函数（签名不变，现有调用点零改动）。`kernel_facade/lifecycle.rs:98` 改调 `init_agentic_system_with_queue_config(EventQueueConfig { heap_enabled: false, ..Default::default() })`。CLI / server / w4_repro / 所有测试构造点保持 Default 不动。
5. **回归测试**（放 `queue.rs` tests 模块，TDD：先写测试看它失败[编译失败即 RED]，再实现）：构造 `EventQueueConfig { max_queue_size: 2, heap_enabled: false, ..Default::default() }`；`subscribe()` 先拿 receiver；enqueue 3 个非 Critical 事件 → 断言：三个全部 `Ok`；`queue.len().await == 0`；receiver 按序收到全部 3 个 envelope。

判断点（已授权，不许上报）：broadcast-only 分支内代码组织方式（早期分支 vs 提取小函数）由实现者选最小 diff；其余不许自由发挥。

## 5. Global Constraints（逐字遵守）

- 禁止改动 CLI / server / w4_repro 的构造路径与行为。
- 禁止回退 P2b 成果（见预检表最后一行）。
- 禁止给 `init_agentic_system()` 加参数或改签名（CLI 多处调用，必须零改动编译）。
- 日志只许英文、无 emoji。
- 本任务不涉 `tokio::select!` / cancellation token / timeout race —— 家规 4 不适用。
- queue.rs 现 298 行，改后远低于 800 —— 家规 3 无虞；不许借机扩文件。
- i18n frozen，不许碰 locale 资源。
- Windows 环境：写非 ASCII 内容一律用 edit 工具，禁用 PowerShell Set-Content（GBK 双重编码事故史）。
- 免费池铁律：假汇报 = 停用；编排者将 diff 逐条核对；验证输出必须贴原文进 report。

## 6. 验证（命令 + 输出原文都要进 report）

MSVC 测试通道（PATH 上 GNU cargo 遮蔽 rustup default，裸 `cargo test` 必挂 `-lshlwapi`）：

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --lib events::queue
```

report 里每条命令贴完整输出尾部（含 test result 行）。report 无输出原文 = 假汇报嫌疑。

## 7. 报告

写入 `E:\agent-project\northing\.superpowers\sdd\audit-fix-c1-event-queue-report.md`：实现内容 / 复用侦察节 / 每个编译错误最终修在哪一层（机制层/设计层，一行一个）/ 测试与输出原文 / 文件清单 / 自审发现 / 疑虑。

最终回复只含（≤15 行）：Status（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit 短 SHA + subject、一行测试摘要、疑虑、report 路径。

## 8. 派发元信息

- BASE commit：`66f08d1`（派发前 HEAD；review-package 以此为 BASE）
- 禁区文件：`src/apps/cli/**`、`src/apps/server/**`、`src/apps/desktop/src/bin/w4_repro.rs`、所有现有测试构造点
- commit 规则：conventional commits（如 `fix(core): ...`），不加 AI 署名/co-author
- 工作目录：`E:\agent-project\northing`，直接在 main 工作（本会话既定流程）

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
