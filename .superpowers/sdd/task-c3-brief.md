# Task C3 Brief — P1-2 API key 迁移 OS keyring

> 本文件是任务的**唯一需求来源**。按此执行，不要从会话历史或猜测中补充需求。

## 位置

- Worktree（在此工作）：`E:\agent-project\northing\.worktrees\p1-security-0804`
- 分支：`fix/p1-security-0804`（已含 C1 + C2 commits，接续工作）
- 计划上下文（只读参考）：`E:\agent-project\northing\.superpowers\sdd\plan-2026-08-04-p1-security.md`

## 问题（P1-2，tech-debt-ledger active）

`ProviderConfig.api_key` 明文存于 `~/.northhing/config/app.json`（`src/apps/desktop/src/app_state/settings/types.rs:58-59` 注释自承）。明文 in app.json，对桌面 agent 产品的密钥风险面。

- 锚点：`src/apps/desktop/src/app_state/settings/types.rs:58-59`；`src/apps/desktop/src/app_state/settings/io.rs:38-49`（`load_app_settings`）；load 路径已有迁移触发即保存的模式（dedup migration，:45-49），可参照。
- 仓库现**无** keyring 依赖（grep `keyring` 在 `src/` 零命中），需引入 workspace 依赖。

## 已解决歧义（编排者 2026-08-04 预核）

- **范围限定**：本任务只迁移 `ProviderConfig.api_key`。其它可能存在于 app.json 的字段一律不动；report 若发现其它明文敏感字段记 concern 不擅改。
- **存储定位**：keyring (service, account) = `("northhing.desktop.providers", provider.id)`（UUID 已是不可变 handle）。`api_key` 在序列化形态保留字段以便反向兼容，但值为非 secret 标记（sentinel），report 明示 sentinel 形态选型及理由。
- **迁移语义**：load 时发现明文 → 写 keyring → 改字段为 sentinel → 原子保存；**幂等**（重载不重复写、不抛错）。失败模式见下。
- **测试 seam**：`KeyringBackend` trait + `MockKeyring`（测试用）；生产实现封装 `keyring` crate 的真实 API。单测全部走 mock，不依赖真实 OS keyring。
- **keyring 不可用 = fail-closed**：store/get 抛错（Linux 无 Secret Service / macOS keychain 拒绝 / Windows Credential Manager 错）→ 整个加载/保存路径返回 Err，**禁止静默回落明文磁盘存储**。错误信息指明环境/系统层面的解决路径（如 "configure Secret Service"），English-only。
- **不 push，不 commit SDD 文档（brief/report/plan）**。

## 交付要求

1. **workspace + desktop 引入 `keyring` crate**（version 走当前 crates.io 稳定版）。desktop Cargo.toml 加依赖引用 workspace。
2. **`KeyringBackend` 抽象**（trait + production impl + mock impl），方法签名至少 `store(account, secret) → Result<()>`、`get(account) → Result<String>`、`delete(account) → Result<()>`（delete 可选，实现可借 store 覆盖）。生产实现包裹 `keyring` crate；mock 在 `#[cfg(test)]` 暴露给单测用 thread-local 或 `Arc<Mutex<_>>` 即可，**不**走 cfg 全局开关，避免污染生产二进制。
3. **ProviderConfig 序列化迁移**：
   - 加载：若 `api_key` 字段非空 → 写 keyring → 改字段为 sentinel → 触发 settings 路径的 atomic save（走既有 `SETTINGS_WRITE_LOCK` + `save_app_settings_at`，不要新写一份并行原子逻辑）。
   - 序列化：默认输出 sentinel 值；保留 `api_key` 字段名以减少 schema diff。
   - 反序列化：透明（sentinel 也合法）。
   - keyring 不可用：fail-closed Err，sentinel 不入盘。
4. **应用入口接线**：找到所有取 `provider.api_key` 的代码点（grep），改走 keyring（构建一个 `resolve_api_key(provider_id: &str) -> Result<String>` 之类的统一入口）。**报告**逐点列出改动点与原因。
5. **测试**（新增，全过）：
   - 加载：明文 → 写 keyring + sentinel 入盘 + 旧明文已抹除（断言文件不再含原明文）
   - 加载：已 sentinel → 不重复写 keyring（幂等）
   - 加载：keyring 抛错 → 整个 load 返回 Err，磁盘文件**未动**（fail-closed）
   - store/get/delete 三方法在 mock 下覆盖正常与异常路径
   - 并发加载幂等（多线程同时迁移）
6. **日志纪律**：**任何日志不得打印 key 本身**（既有注释承诺保持，并需落实——grep 验证无 `{:?}` / `{}` 打到 `api_key`）。验证代码里 `println!("{api_key}")` / `info!("provider key: {}", p.api_key)` 之类出现即 spec FAIL。
7. **ledger 翻转（同 commit）**：`docs/status/tech-debt-ledger.md`
   - P1-2 → resolved（附 commit hash 占位可在收口时补；先写修复说明）
   - 若发现其它明文敏感字段，记为新条目 concern，不擅自改。

## 范围外（勿动）

- 其它明文字段（如有发现只登记不动）
- Task 7 已落地的 settings 原子写路径（复用，不改）
- desktop 的 keyring 不可用导致 CI 测试失败的解决方案——只确保 fail-closed 在代码层正确，CI 环境由 CI 侧解决

## 全局约束（仓库硬规则，逐字生效）

- 日志 English-only，无 emoji。
- 生产 `.rs` 文件 <800 行；>1000 必须拆或加 `// allow-god-file`。
- 触及 `tokio::select!` / cancellation / timeout 竞态的改动必须带自动化测试。
- 不裸跑 `cargo fmt`；新代码手工对齐既有风格。
- 只 commit 本任务范围内文件；commit 前缀 `fix(security):`。不 commit SDD 文档。不 push。
- ledger 翻转与修复同 commit。

## 验证（最小集，必须全跑并记录输出）

```
cargo test -p northhing --lib settings
cargo check -p northhing
```

广覆盖交 CI；不跑 workspace 全量（上游 embed-resource 阻断）。

## Report

写到 `E:\agent-project\northing\.superpowers\sdd\task-c3-report.md`，必含：

- 状态行：`DONE` / `DONE_WITH_CONCERNS` / `NEEDS_CONTEXT` / `BLOCKED`
- 改动文件清单 + 每文件职责一句话
- sentinel 形态选型及理由
- 所有取 `provider.api_key` 调用点处置表（含是否改走 keyring）
- 测试命令 + 真实完整输出（通过/失败统计）
- ledger 翻转 diff 摘要 + 任何新 concern 条目
- **C1/C2 教训继承**：「机制存在/不存在」「代码包含/不包含」「日志打印/不打印」类结论必须附 file:line 证据；无法核实的写「未核实」，禁止推断成结论。