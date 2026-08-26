# Task Brief — W2-1：F5 进程组/kill_on_drop 模式根治（spawn 向构造器 + 真实 spawn 点迁移）

## 1. 来源与验收标准（逐字）

来源：`.superpowers/sdd/reviews/project-audit-20260826/r3-services.md` Finding F5（Minor，模式问题）：

> Either roll `configure_process_group` into `create_tokio_command` for the `spawn`-oriented callers (with a separate `create_command_one_shot` for `output().await`-style callers), or audit each spawn site and apply the helper. The flashgrep pattern (`kill_on_drop(true)` + `configure_process_group` + Drop-cleanup) should be the default.

**编排者裁定（钉死）**：走"新构造器 + 迁移真实 spawn 点"路线——**不改 `create_tokio_command` 现有语义**（15+ 调用点含 git/terminal 的 output() 短任务，进程组隔离会改变 Ctrl-C 信号路由，行为边界不动）。

验收标准（逐条可机械核对）：

1. `services-core/src/process_manager.rs` 新增 `pub fn create_tokio_command_for_spawn<S: AsRef<std::ffi::OsStr>>(program: S) -> TokioCommand` = `create_tokio_command` + `kill_on_drop(true)` + `configure_process_group`，带英文 doc comment 说明何时用哪个（长命 spawn 子进程 vs 一次性 output()）。
2. F5 列出的每个候选点逐个分类 spawn()/output()（分类表进 report）：
   - `src/crates/assembly/core/src/service/lsp/process_command.rs` 5 处（审计行号 125,142,178,187,211——先 grep 核实，可能漂移）
   - `src/crates/services/services-core/src/system/command.rs:280`
   - `src/crates/services/services-integrations/src/git/utils.rs:201`
   - `src/crates/assembly/core/src/service/workspace/workspace_info_impl.rs:380`
   - `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions/utilities.rs` 4 处（227,239,274,295）+ 同族 `app_control.rs:425`
3. 真实 spawn() 点迁移到新构造器；output() 点**原样不动**并在 report 列出+理由。
4. 三处已手工正确的点迁移到构造器统一模式（可 grep 单一真相）：`lsp/process_spawn.rs:50-51`、`mcp/server/process.rs:85-92`、`flashgrep/client.rs:423-431`（各自的 kill_on_drop/configure_process_group 手工两行由构造器替代）。
5. `cargo check --workspace` + 聚焦测试全绿，输出原文进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

2026-08-27 @ 5a90e04 实时核实：

| 事实 | 锚点 |
|---|---|
| `create_tokio_command` 仅置 Windows `CREATE_NO_WINDOW`；`configure_process_group` 仅 unix 生效（`process_group(0)`），Windows no-op | `services-core/src/process_manager.rs:108-127, 171-176` |
| tree-kill 在 Windows 走 `taskkill /PID /T /F`（不依赖进程组），unix 走 `kill -TERM -pgid`（依赖 configure_process_group 先设组） | `process_manager.rs:178-226` |
| 已合规三点：process_spawn.rs:50-51 / mcp process.rs:91-92 / flashgrep client.rs:430-431 | grep 实证 |
| process_manager.rs 现 249 行，远低于 800 | — |
| F9（同文件 :228-245 helper 改造）是紧随的下一任务 W2-2，本任务**不碰** `spawn_child_process_tree_cleanup` | 波次排序 |

## 3. 复用侦察（强制）

grep 全仓 `create_tokio_command` 全部调用点（不止 F5 清单——terminal/exec/output.rs:422 有自己的管道进程组变体，确认不需动）；确认无第三方已有同义构造器。report 必须有「复用侦察」一节（调了什么、为什么 F5 清单外的点不动）。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. 新构造器签名如上；实现 = 三行组合，doc comment 英文两句内（when-to-use 对比）。
2. 分类表：每个候选点一行（file:line → spawn/output → 迁移/不动+一句理由）。分类方法：读该点上下文，追 `cmd.spawn()` vs `cmd.output()`/`.status()` 消费方式。
3. 迁移 = 把 `create_tokio_command(x)` 换成 `create_tokio_command_for_spawn(x)` 并删除该点手工的 `kill_on_drop(true)`/`configure_process_group` 行（如有）。
4. 不动：F5 清单外的 output() 点、terminal 自有变体、`spawn_child_process_tree_cleanup`（W2-2 领土）。
5. 测试：构造器为平凡组合，不强制新单测；但若迁移点所在模块有现成 spawn 路径测试（lsp/mcp），必须跑绿。

## 5. Global Constraints（逐字遵守）

- 日志只许英文、无 emoji。
- services-core  guardrails：不向上依赖（core/app/Tauri），helper 留在 services-core。
- 只 commit 代码文件——**禁止 commit `.superpowers/sdd/progress.md`（编排者台账）**；report 文件可 commit。
- Windows 环境：写非 ASCII 一律用 edit 工具，禁用 PowerShell Set-Content（GBK 双重编码事故史）。
- 免费池铁律：假汇报 = 停用；编排者将 diff 逐条核对；验证输出必须贴原文进 report。

## 6. 验证（命令 + 输出原文都要进 report）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --lib lsp
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-core
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-integrations --features mcp
```

（feature 名以仓库实际为准；若某条命令组合不对，用 `cargo test -p <crate> --help`/Cargo.toml 自查并在 report 说明替代命令。）

## 7. 报告

写入 `E:\agent-project\northing\.superpowers\sdd\w2-1-f5-spawn-constructor-report.md`：实现内容 / 复用侦察节 / **站点分类表**（file:line → spawn/output → 处置+理由）/ 每个编译错误最终修在哪一层（机制层/设计层，一行一个）/ 测试与输出原文 / 文件清单 / 自审发现 / 疑虑。

最终回复只含（≤15 行）：Status（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit 短 SHA + subject、一行测试摘要、疑虑、report 路径。

## 8. 派发元信息

- BASE commit：`5a90e04`（派发前 HEAD）
- 禁区文件：`spawn_child_process_tree_cleanup` 函数体（W2-2 领土）、`.superpowers/sdd/progress.md`
- commit 规则：conventional commits（如 `refactor(services): ...`），不加 AI 署名/co-author
- 工作目录：`E:\agent-project\northing`，直接在 main 工作（本会话既定流程）

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
