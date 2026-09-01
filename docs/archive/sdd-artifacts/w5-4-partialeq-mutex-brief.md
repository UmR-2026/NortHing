# Task 4 (W5-4): F5 + F6 — PartialEq hack 与 entry.rs Mutex 收口

来源：`.superpowers/sdd/w4-2-dioxus-shell-review.md` F5 / F6（均 Minor）。

审计原文：
- F5 `registry.rs:39-43`：`impl PartialEq for ModuleAppProps { fn eq(&self, _other: &Self) -> bool { true } }` 恒 true，prop 变更永不触发重渲染；当前靠 watch channel 绕行，hack 无注释。修复方向：实现真实 PartialEq（比较影响渲染的字段）或加注释说明故意为之。
- F6 `entry.rs:139-140`：`room_window_id` / `latest_geometry` 两处 `std::sync::Mutex` 跨线程共享（tao 事件处理器 + use_effect）。当前无跨 await 持锁，footgun 非 bug。修复方向：`room_window_id` 改 `tokio::sync::watch`（单写多读契合）；或等价机制。

## 编排者裁定（钉死）

- F5：实现者选"正确且懒"的那个——真实 PartialEq（若字段可比较且代价小）或注释说明。注释为下限。
- F6：`room_window_id` 改 `tokio::sync::watch`；`latest_geometry` 若 watch 化不干净则保留 Mutex 并加 `ponytail:` 注释（注明上限与升级路径）。禁止引入新框架/新依赖。

## Spec（全部满足）

1. F5：真实 PartialEq 或注释落地（report 说明选择及理由）。
2. F6：room_window_id 走 watch 或等效机制；latest_geometry 处置有明确理由（改了给 file:line，没改给 ponytail 注释位置）。
3. `cargo check -p northhing` + `cargo test -p northhing` 全绿；行为零变化（本任务是结构收口，不改任何渲染/事件语义）。

## Global Constraints（逐字遵守）

1. 分层边界：改动只在 `src/apps/desktop`；其它 crate 零改动。
2. 日志纪律：新增日志一律英文、无 emoji（本任务原则上不应新增日志）。
3. 并发测试绑定（家规④）：若改动涉及并发语义变化则带测试；纯机制替换（Mutex→watch 同语义）注明理由可豁免。
4. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/`。
5. rot-budget：不上调任何 ceiling；不新增 >800 行文件。
6. 验证最小集：`cargo check -p northhing` + `cargo test -p northhing`；命令与输出原文进 report。
7. commit 规则：恰好一个 commit，消息对齐近期 git log；不含 `.superpowers/` 产物。
8. 不新建无 owner 抽象。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
