# Task Brief — Audit 进程批（I4+I5）：LSP/MCP 子进程孤儿治理

## 1. 来源与验收标准（逐字）

来源：`.superpowers/sdd/reviews/project-audit-20260826/r3-services.md` F1（I4，Important）+ F2（I5，Important）。

F1 fix direction 原文：

> (1) Add `.kill_on_drop(true)` and `process_manager::configure_process_group(&mut cmd)` to `process_spawn.rs:43-52`. (2) Replace the trivial `Drop` in `process.rs:67-71` with a `start_kill()` on the child if the handle is still live. (3) Track JoinHandles for the three spawned tasks and abort them in Drop or on `stop_server`.

F2 fix direction 原文：

> (1) Apply `process_manager::configure_process_group` in `start()` (line 85). (2) Replace `child.kill().await` (line 245) with `process_manager::terminate_child_process_tree(&mut child, graceful_timeout)`, matching the pattern at `flashgrep/client.rs:539`.

**编排者裁定（钉死，不许自由发挥）**：

- F1 只做 (1)+(2)，**不做 (3)**。依据：read task 在 stdout EOF 时自终止（`process_runtime.rs:77-84` break），stderr/notification task 同理随通道关闭退出；(1)+(2) 保证孤儿 PID 被回收、EOF 触发任务退出。残余天花板：孙进程继承 stdout 管道时 read task 以 30s 节奏空转（`MAX_CONSECUTIVE_TIMEOUTS` 块只重置计数不退出，:85-97）——记 ponytail 注释 + report 声明，不修。
- F5/F9 不随批（保持 Minor 台账项，终审 triage）。本批只有 F1+F2 四个改动点。

验收标准（逐条可机械核对）：

1. LSP spawn 路径带 `kill_on_drop(true)` + `configure_process_group`；`LspServerProcess::drop` 在 child handle 存活时 `start_kill()`。
2. MCP `start()` 带 `kill_on_drop(true)` + `configure_process_group`；`stop()` 用 `terminate_child_process_tree(&mut child, Duration::from_millis(750))`；`Drop` 用 `spawn_child_process_tree_cleanup(child, Duration::from_millis(750))` 替代裸 `start_kill()`。
3. `cargo check --workspace` 与聚焦测试全绿，输出原文进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

2026-08-26 @ 37a71f4 实时核实：

| 事实 | 锚点 |
|---|---|
| LSP spawn：`cmd.spawn()` 前只有 stdin/stdout/stderr 设置，无 kill_on_drop/process_group | `src/crates/assembly/core/src/service/lsp/process_spawn.rs:43-52` |
| LSP `Drop` 只打 debug 日志 | `src/crates/assembly/core/src/service/lsp/process.rs:67-71` |
| LSP child 为 `Arc<RwLock<Child>>`，字段 `pub(super)`，同模块 impl 可触 | `process.rs:46` |
| 三个后台 task **不持有** child Arc（grep `process_runtime.rs` 无 child clone）→ 结构体 Drop 即最后引用 | grep 实证 |
| read task EOF 自终止（`Ok(Err(_))` → break）；超时块只重置计数 | `process_runtime.rs:77-84, 85-97` |
| MCP spawn：`process_manager::create_tokio_command(&final_command)` 已有 process_manager 导入，无 process_group/kill_on_drop | `src/crates/services/services-integrations/src/mcp/server/process.rs:85-91` |
| MCP `stop()`：`self.child.take()` 后 `child.kill().await`（只杀直接子进程） | `process.rs:244-251` |
| MCP `Drop`：`self.child.take()` + `child.start_kill()`（owned Child，可直接移交） | `process.rs:396-401` |
| 参考范式：flashgrep `kill_on_drop(true)` + `configure_process_group` + Drop `spawn_child_process_tree_cleanup(child, DROP_CLEANUP_TIMEOUT)` + stop 路径 `terminate_child_process_tree(child, Duration::from_millis(750))` | `services-integrations/src/workspace_search/flashgrep/client.rs:430-431, 539, 667-674` |
| `process_manager` API：`configure_process_group(&mut TokioCommand)`（unix: process_group(0)；windows: no-op，由 Job 对象兜底）；`terminate_child_process_tree(&mut Child, Duration)`（unix TERM/KILL 组，windows taskkill /T /F）；`spawn_child_process_tree_cleanup(Child, Duration)`（接管所有权，独立线程+runtime） | `services-core/src/process_manager.rs:170-245` |
| assembly/core 已依赖 services-core 的 process_manager（`lsp/process_command.rs` 用 `create_tokio_command`） | r3 F5 清单 |
| Windows 应用退出时孙进程由全局 Job 对象兜底（kill-on-close，父进程已 assign） | `process_manager.rs:52-75` |

## 3. 复用侦察（强制）

动手前查 `process_manager` 全部公共 API 与 flashgrep 参考用法；本任务**零新抽象**，全部复用现有 helper。report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. **LSP spawn**（`process_spawn.rs`，`:43` `build_command` 之后、`:49` `cmd.spawn()` 之前）：加
   `cmd.kill_on_drop(true);` 和 `northhing_services_core::process_manager::configure_process_group(&mut cmd);`（按该文件现有 import 风格组织 use）。
2. **LSP Drop**（`process.rs:67-71`）：保留 debug 日志，追加：`-if let Ok(mut child) = self.child.try_write() { let _ = child.start_kill(); }`（try_write 失败 = 他处持锁，放弃不阻塞 Drop）。字段旁加一行 ponytail 注释：孙进程持 stdout 管道的残余空转窗口不修（见 report）。
3. **MCP start**（`process.rs:85-91`）：`create_tokio_command` 之后、`spawn()` 之前加 `process_manager::configure_process_group(&mut cmd);` + `cmd.kill_on_drop(true);`。
4. **MCP stop**（`process.rs:244-251`）：`child.kill().await` 替换为 `process_manager::terminate_child_process_tree(&mut child, Duration::from_millis(750)).await`，Err 臂维持现有 warn! 文案语义（英文、含 name/id/error）。注意 `Duration` 导入。
5. **MCP Drop**（`process.rs:396-401`）：`child.start_kill()` 替换为 `process_manager::spawn_child_process_tree_cleanup(child, Duration::from_millis(750))`。

不加新测试（编排者裁定：进程树杀回收需真实进程，无单测面；家规 4 不适用——不涉 select!/cancellation/timeout race 改动）。现有测试套件即回归网。

## 5. Global Constraints（逐字遵守）

- 禁止新抽象/新 helper/新配置项——全部复用 process_manager 现有 API。
- 禁止改 `process_manager.rs`、`flashgrep/client.rs`、LSP `process_runtime.rs` 任务体、`manager.rs`。
- 禁止给三个后台 task 加 JoinHandle 跟踪（见 §1 裁定）。
- 日志只许英文、无 emoji。
- 不涉并发原语改动 —— 家规 4 不适用。
- 三个触碰文件均远低于 800 行，保持。
- Windows 环境：写非 ASCII 一律用 edit 工具，禁用 PowerShell Set-Content（GBK 双重编码事故史）。
- 免费池铁律：假汇报 = 停用；编排者将 diff 逐条核对；验证输出必须贴原文进 report。

## 6. 验证（命令 + 输出原文都要进 report）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --lib lsp
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-integrations --features product-full --lib mcp
```

report 里每条命令贴完整输出尾部（含 test result 行）。report 无输出原文 = 假汇报嫌疑。

## 7. 报告

写入 `E:\agent-project\northing\.superpowers\sdd\audit-fix-i4i5-process-tree-report.md`：实现内容 / 复用侦察节 / 每个编译错误最终修在哪一层（机制层/设计层，一行一个）/ 测试与输出原文 / 文件清单 / 自审发现 / 疑虑。

最终回复只含（≤15 行）：Status（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit 短 SHA + subject、一行测试摘要、疑虑、report 路径。

## 8. 派发元信息

- BASE commit：`37a71f4`（派发前 HEAD）
- 禁区文件：`process_manager.rs`、`flashgrep/client.rs`、`process_runtime.rs`、`manager.rs`（LSP）、`mcp/server/` 下除 `process.rs` 外文件
- commit 规则：conventional commits（如 `fix(lsp,mcp): ...`），不加 AI 署名/co-author
- 工作目录：`E:\agent-project\northing`，直接在 main 工作（本会话既定流程）

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
