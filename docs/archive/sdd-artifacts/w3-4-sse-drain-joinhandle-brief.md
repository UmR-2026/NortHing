# Task 4 (W3-4): F10 — SSE drain task JoinHandle 跟踪 + 早退 abort

来源与验收标准（逐字，r3-services.md F10）：

> - **Where**: `src/crates/execution/agent-stream/src/stream_processor.rs:425-429`
> - **What**: `tokio::spawn(async move { while let Some(data) = rx.recv().await { ... } })` is detached (no JoinHandle stored in the surrounding scope).
> - **Fix direction**: Track JoinHandle and `abort()` in early-return paths, or rely on the bounded-by-F4 ring buffer to make the drain task cheap.
> - **Effort**: S

编排者预检结论（直接采信）：

- F4（SSE 缓冲上限）已在第一波 I7 收口，drain task 成本已有界；本任务做"JoinHandle 跟踪 + 早退 abort"这一半，使生命周期显式化。
- 属 tokio 任务生命周期改动 → 家规④ 强制带测试。

Spec（全部满足）：

1. stream_processor.rs:425-429 的 drain task 的 `JoinHandle` 被持有（存于函数局部变量或结构字段，由实现者按数据流选择并在 report 说明）。
2. 所有早退路径（含 `graceful_shutdown_from_ctx` 及其余提前返回点）返回前 `abort()` 该 handle；正常流尽路径不 abort（任务随 rx 关闭自终）。
3. 附一个自动化测试覆盖"早退 → drain 任务终止"（实现者选择 agent-stream 内最近测试设施；可用 `JoinHandle::is_finished` / abort 后 `is_aborted` 类断言）。
4. SSE 解析、ring buffer、错误分级语义零改动。

验证：`cargo check --workspace`；`cargo test -p northhing-agent-stream`（含新测试）。

## Global Constraints（逐字遵守）

1. 分层边界（根 AGENTS.md 六层）：改动只在指定 crate；不得引入向上的跨层依赖。
2. 日志纪律：新增日志一律英文、无 emoji；warn!/debug! 消息带关键上下文字段。
3. 并发测试绑定（家规④）：触碰 `tokio::select!` / cancellation token / tokio 任务生命周期的改动，必须随附至少一个自动化测试。
4. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`（不 add/commit/restore/checkout/clean）；禁止编辑 `progress.md`；自己的 report 文件用 write 工具写入 `.superpowers/sdd/`，由编排者统一入库。
5. rot-budget：不得上调任何 ceiling；不得新增 >800 行文件。
6. 验证最小集：`cargo check --workspace` + 本任务指定的就近聚焦测试；命令与输出原文进 report。
7. commit 规则：每任务恰好一个 commit，消息格式对齐近期 git log（`fix(...)` / `refactor(...)`）；commit 不含 `.superpowers/` 产物。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
