# Task 3 (W5-3): F4 — onboarding 持久化 provider 配置

来源：`.superpowers/sdd/w4-2-dioxus-shell-review.md` F4（Important）。

审计原文：`pages_onboarding.rs:672-705` 测连通性（test_provider_config）→ key 存 keyring（account "onboarding"）→ `create_session(model_name: "default")`，但从不创建/持久化 `ProviderConfigDto`（无 upsert_provider_config / 无 set_default_provider）。后果：引导完成后全局配置无 provider，会话创建空转，用户面对空设置页。

## 编排者裁定（钉死）

- 修复：`test_provider_config` 成功后，从表单字段构造 ProviderConfigDto → `kernel_facade().upsert_provider_config(...)` → 设为默认 provider（核实 facade 上的真实 API 名，report 引用）→ 再 create_session。keyring account "onboarding" 的 key 要与持久化的 provider 关联（或改存到 provider 对应 account，按 keyring 既有约定——先读 `app_state/settings/keyring.rs` 的 account 命名规则再定）。
- 失败语义：persist 失败 → 不推进到下一步，错误展示在 onboarding UI（不静默）。

## Spec（全部满足）

1. 引导完成后 `list_providers` 能看到新 provider 且为默认；create_session 不再因缺 default provider 空转。
2. 各失败臂（测试失败/persist 失败/设默认失败）有明确 UI 错误，不静默吞。
3. 附聚焦测试或注明无法自动化的理由（UI spawn 块可豁免，但持久化序列若抽成函数则必须测）。

## Global Constraints（逐字遵守）

1. 分层边界：改动只在 `src/apps/desktop`；其它 crate 零改动。
2. 日志纪律：新增日志一律英文、无 emoji，带关键上下文字段。
3. 并发测试绑定（家规④）：触碰 tokio 任务生命周期/取消/关闭顺序的改动必须随附至少一个自动化测试。
4. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/`。
5. rot-budget：不上调任何 ceiling；不新增 >800 行文件。
6. 验证最小集：`cargo check -p northhing` + 聚焦测试；命令与输出原文进 report。
7. commit 规则：恰好一个 commit，消息对齐近期 git log；不含 `.superpowers/` 产物。
8. 不新建无 owner 抽象；优先复用既有通道/设施（kernel facade 的 settings API 是第一选择）。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
