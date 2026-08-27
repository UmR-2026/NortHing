# Task 2 (W5-2): F2 — 事件通道关键事件不丢（TurnState/ToolCall 与 TextChunk 分级）

来源：`.superpowers/sdd/w4-2-dioxus-shell-review.md` F2（Important）。

审计原文：`api.rs:191-193` 把 kernel 1024 广播桥到 `mpsc::channel(256)` 用 `try_send`，满载静默丢。丢 `TextChunk` = 文案缺口（可接受）；丢 `ToolCall(AwaitingConfirmation)` = 审批卡消失；丢 `TurnState::Completed/Failed/Cancelled` = 流式标志永不复位、草稿永不提交、UI 永久卡"生成中"。消费者循环在 `app.rs:158-253`。

## 编排者裁定（钉死）

- 方向：**控制事件与数据事件分级**。`TextChunk` 保持有损（try_send，满了可丢）；`TurnState` / `ToolCall`（及任何影响状态机/审批的事件）必须保证投递——实现者选择最小机制（独立 unbounded 控制通道，或满载时控制事件走不丢路径），并在 report 说明选择与代价。
- 不许用"无限加大 256 缓冲"当修复（不解决根因）。

## Spec（全部满足）

1. TextChunk 之外的事件类型不再因通道满载而丢失（给出机制与 file:line）。
2. 附自动化测试：塞满有损通道后 TurnState 事件仍到达消费者（测试设施由实现者按 crate 内现有模式选）。
3. 消费者循环（app.rs:158-253）的流式复位语义不变。

## Global Constraints（逐字遵守）

1. 分层边界：改动只在 `src/apps/desktop`；其它 crate 零改动。
2. 日志纪律：新增日志一律英文、无 emoji，带关键上下文字段。
3. 并发测试绑定（家规④）：触碰 tokio 任务生命周期/取消/关闭顺序的改动必须随附至少一个自动化测试（本任务 Spec 2 即该测试，不豁免）。
4. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/`。
5. rot-budget：不上调任何 ceiling；不新增 >800 行文件。
6. 验证最小集：`cargo check -p northhing` + 聚焦测试；命令与输出原文进 report。
7. commit 规则：恰好一个 commit，消息对齐近期 git log；不含 `.superpowers/` 产物。
8. 不新建无 owner 抽象；优先复用既有通道/设施。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
