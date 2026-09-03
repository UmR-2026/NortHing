# Task W15-1f: services-integrations dev-dependencies anyhow 违规修复（core boundary 红治理）

## 来源与验收标准（逐字）

handoff `docs/handoffs/2026-09-03-w15-1-done-screenshot-pending.md` §3：
> ❌ core boundary 红（预存）——本地复现根因：`services-integrations/Cargo.toml:50` anyhow 非 optional 违反边界规则，**可直接修**（optional 化 + feature 挂接）。

本地复现命令 `node scripts/check-core-boundaries.mjs` 输出（编排者已跑，逐字）：
```
src/crates/services/services-integrations/Cargo.toml:50: services-integrations default profile must not compile feature-gated integrations; default integrations profile forbids non-optional dependency: anyhow
src/crates/services/services-integrations/Cargo.toml:50: services-integrations optional runtime dependencies must stay owned by explicit integration features; dependency must be optional: anyhow
```

验收标准（逐条可机械核对）：
1. `node scripts/check-core-boundaries.mjs` 退出码 0，输出无上述两条。
2. `cargo test -p northhing-services-integrations` 全绿（命令+输出贴 report）。
3. diff 只触及允许文件集。

## 编排者预检结论（直接采信，不重复侦察）

- 违规行 = `[dev-dependencies]` 段的 `anyhow = { workspace = true }`（Cargo.toml:50）。主 `[dependencies]` 的 anyhow（:21）已是 optional，不违规。
- dev-dependencies **不能标 optional**（Cargo 语义），所以"optional 化"在本案不可行；真实修法 = **让测试代码不再依赖 anyhow，然后从 dev-dependencies 删除该行**。
- anyhow 在该 crate 的唯一 dev 用途（编排者已 rg 全 crate 核实）：
  `src/crates/services/services-integrations/tests/file_watch_contracts.rs:15`：
  ```rust
  async fn emit(&self, event_name: &str, payload: serde_json::Value) -> anyhow::Result<()> {
  ```
- 同段 `async-trait = { workspace = true }`（:51）也非 optional，但边界规则的 forbidden 名单不含 async-trait（checker 未报），**不动它**。
- blast radius：该 test 文件只被 cargo test 编译；checker 解析整个 Cargo.toml 所有段，dev-deps 非 optional 即触发。
- 判断点（授权）：`anyhow::Result<()>` 的替代类型由实现者选最小改动——函数体当前应是 `Ok(())` 返回，可改成 `Result<(), Box<dyn std::error::Error + Send + Sync>>` 或 `std::io::Result<()>` 或任何满足 trait 约束的具体类型；以能编译过且 diff 最小为准。若 emit 的 trait 签名要求精确 `anyhow::Result`（不太可能，trait 来自 northhing-events），则上报 BLOCKED 而非绕规则。

## 复用侦察（强制）

- 动手前 rg 确认该 tests 目录下其它文件是否也用 anyhow（编排者已查：仅 file_watch_contracts.rs 一处）。
- report 必须有「复用侦察」一节：查了哪些符号、复用了什么、若新写了已有能力的等价物逐条给理由。无此节 = 未完成。

## Spec（必须全部满足）

1. `services-integrations/Cargo.toml` 的 `[dev-dependencies]` 删除 `anyhow = { workspace = true }` 行。
2. `tests/file_watch_contracts.rs` 不再引用 anyhow（改 :15 的返回类型及函数体相应适配）。
3. 不改 checker 脚本、不改规则数据、不改 `[dependencies]` 段。
4. `cargo test -p northhing-services-integrations` 绿。

## Global Constraints（逐字遵守）

- 禁止改 `scripts/core-boundaries/` 下任何文件（改规则=治理变更，需仲裁，不在本任务授权内）。
- 禁止"顺手"改 async-trait 行或任何其它依赖行。
- 仓库货：cargo 一律 `C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo ...`；长命令用 run_detached 或 cmd 重定向，不要让 shell 假死。

## 验证（命令 + 输出原文进 report）

```powershell
node scripts/check-core-boundaries.mjs
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-services-integrations
```

## 报告

- 路径：`.superpowers/sdd/reports/W15-1f-report.md`
- 内容：改动摘要 / 复用侦察节 / 验证命令+输出原文 / 编译错误处置（本任务预期无）/ 结尾状态词 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 派发元信息

- BASE commit：`65a44e286b9474dcfdd8cf6a021206797dbdb986`
- **允许文件集**（diff 越出即 Critical）：
  - `src/crates/services/services-integrations/Cargo.toml`
  - `src/crates/services/services-integrations/tests/file_watch_contracts.rs`
  - `.superpowers/sdd/reports/W15-1f-report.md`（新建）
- 禁区：`scripts/core-boundaries/**`、其它一切 crate。
- 不 commit（编排者收口）。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
