# Task 1 (W5-1): F1 — quit_shell 走优雅退出，禁 process::exit

来源：`.superpowers/sdd/w4-2-dioxus-shell-review.md` F1（Critical）。

审计原文：`app.rs:763-765` `quit_shell()` 调 `std::process::exit(0)`，由 room chrome ✕ 按钮（app.rs:433-436）触发。后果：WindowDropGuard 不跑、几何跟随线程被 OS 强杀、worker 线程（tokio runtime + MCP servers + cleanup scheduler）永远收不到 main.rs 的 shutdown 信号。优雅退出路径存在且正确（`shutdown_tx.send(())` → worker 退出 → `shutdown_mcp_servers()`），但从 ✕ 不可达。修复方向：用关窗信号替代 process::exit，让控制流回到 `launch()` 再回 main.rs。

## 编排者裁定（钉死）

- 目标语义：✕ → 关闭 room + 全部 module 窗（走 `ShellWindowManager` 现有关闭路径）→ `ui_dioxus::launch()` 返回 → main.rs 的 `shutdown_tx.send(())` + `shutdown_mcp_servers()` 正常执行。
- 实现路径由实现者按现有代码选（关 room 窗触发既有退出链，或经 manager 广播关闭），但必须满足：不再出现 `std::process::exit` 于正常退出路径（init 失败的 exit(1) 保留）；退出后 MCP 子进程被清理。
- 测试豁免说明：窗口关闭链路难自动化；若实现中抽出了可测的信号/状态函数，为其附一个单测；纯 wiring 部分豁免（编排者事后真机实测兜底，对应实测清单 6/7 项）。

## Spec（全部满足）

1. `quit_shell` 不再调 `process::exit(0)`；✕ 触发完整优雅退出链（room + module 窗关闭 → launch 返回 → main shutdown 路径）。
2. `rg "process::exit" src/apps/desktop/src` 仅剩 init 失败路径。
3. 验证集全绿；report 附退出链路的 file:line 走查说明（每个环节如何接力）。

## Global Constraints（逐字遵守）

1. 分层边界：改动只在 `src/apps/desktop`；其它 crate 零改动。
2. 日志纪律：新增日志一律英文、无 emoji，带关键上下文字段。
3. 并发测试绑定（家规④）：触碰 tokio 任务生命周期/取消/关闭顺序的改动必须随附至少一个自动化测试；无法自动化处由编排者在 brief 里显式豁免并说明理由。
4. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/`。
5. rot-budget：不上调任何 ceiling；不新增 >800 行文件。
6. 验证最小集：`cargo check -p northhing` + 本任务指定的聚焦测试；命令与输出原文进 report。
7. commit 规则：每任务恰好一个 commit，消息对齐近期 git log；不含 `.superpowers/` 产物。
8. 不新建无 owner 抽象；优先复用既有通道/设施（brief 里已点名）。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
