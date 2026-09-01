# Task 1 (W3-1): r2#5 — 会话创建持久化失败回滚内存插入

来源与验收标准（逐字，r2-core.md Finding 5）：

> - **file:line**: `src/crates/assembly/core/src/agentic/session/session_manager_lifecycle.rs:166-182` — `sessions.insert(...)` and index insert happen, then `save_session(...).await?` can return `Err` with no rollback of the in-memory inserts.
> - **what**: If the initial persist fails, the caller gets `Err` but the session remains in the in-memory map and `session_workspace_index`, consuming one of `max_active_sessions` slots.
> - **fix direction**: On persist failure, remove the just-inserted in-memory entries before returning `Err` (or persist first, then insert).
> - **effort**: S

编排者预检结论（直接采信，不重复侦察）：

- 目标函数：`session_manager_lifecycle.rs` 创建路径（约 :149-182），含 `max_active_sessions` 守卫（:149-154）、`sessions.insert` 与 `session_workspace_index` 插入、`save_session(...).await?`。
- 回滚方案裁定：采用"失败时回滚刚插入的内存项"，**不重排** insert/persist 顺序（重排会改变成功路径语义，超出 Minor 收口范围）。

Spec（全部满足）：

1. `save_session(...).await` 返回 Err 时，函数返回前撤销本次调用刚插入的 `sessions` 项与 `session_workspace_index` 项。
2. 回滚只移除本次插入的键；实现者须先在代码中确认该键在插入前不可能已有同名项（session id 为新生成），并在 report 中给出确认依据。
3. 新增一个聚焦测试：模拟持久化失败 → 断言返回 Err 且 `sessions` map 与 `session_workspace_index` 均无残留。模拟方式由实现者按 crate 内现有测试设施选择。
4. 不改函数签名；成功路径行为零变化。

验证：`cargo check --workspace`；`cargo test -p <assembly/core 实际包名>` 中会话管理相关聚焦测试（含新测试）。

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
