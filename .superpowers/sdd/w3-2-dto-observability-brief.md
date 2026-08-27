# Task 2 (W3-2): r2#7 + r2#8 — kernel_facade/dto.rs 观测性收口

来源与验收标准（逐字，r2-core.md Finding 7 / 8）：

> Finding 7: `src/crates/assembly/core/src/kernel_facade/dto.rs:72-74` — `serde_json::to_value(p).unwrap_or(serde_json::Value::Null)`. If `compression_payload` fails to serialize, the DTO carries `Null` with no log. **fix direction**: Log a warning on the `Err` arm instead of a bare `unwrap_or(Null)`.

> Finding 8: `src/crates/assembly/core/src/kernel_facade/dto.rs:23-26` — `images.iter().filter_map(|img| img.image_path.clone())`. Multimodal images that carry only a `data_url` (no `image_path`) are filtered out of the DTO silently. The frozen `MessageContentDto::Multimodal.images` is a `Vec<String>` of paths, so data-URL-only images can't be represented. **fix direction**: Document the path-only contract at the filter site, or surface a marker for path-less images rather than dropping them silently.

编排者预检结论（直接采信，不重复侦察）：

- 两 finding 同文件 `kernel_facade/dto.rs`，合并一任务；DTO schema 冻结（frozen minimal `KernelEventDto` 体系），**不改任何 DTO 字段形状**。
- Finding 8 裁定：采用"文档化契约 + 可观测丢弃"，不引入 marker 字段（marker 会改 DTO 形状，越界）。

Spec（全部满足）：

1. dto.rs:72-74：序列化失败臂改为 `warn!`（英文，含错误详情）后回落 `Value::Null`；成功臂与 DTO 形状不变。
2. dto.rs:23-26：filter 站点加注释，写明 path-only 契约及 data_url-only 图片无法表示的事实；每次调用若有图片因此被丢弃，记一条 `debug!`（英文，含本次丢弃计数）——不逐张刷日志。
3. 不改函数签名；其它映射函数零改动。

验证：`cargo check --workspace`；dto 相关现有测试全绿（无行为变更，不强制新测试）。

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
