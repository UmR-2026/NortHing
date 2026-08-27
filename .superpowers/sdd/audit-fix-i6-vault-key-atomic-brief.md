# Task Brief — Audit I6：vault 钥匙文件原子写（password_vault + mcp auth）

## 1. 来源与验收标准（逐字）

来源：`.superpowers/sdd/reviews/project-audit-20260826/r3-services.md` F3（Important）：

> Replace the key-write block in both vaults with a `JsonFileStore.write_atomic` call on the key file (or extract a tiny `write_secret_bytes_atomic` helper), and `fsync` the directory for durability. Chmod can be tightened via `set_permissions` BEFORE the rename.

**编排者裁定（钉死）**：

- 钥匙文件**格式不变**（raw 32 bytes；读路径 `bytes.len() != 32` fail-closed，改格式 = 已有 vault 永久不可解，正是本任务要防的事故）。
- 采用 helper 路线：`JsonFileStore` 加 `write_bytes_atomic`（JSON 序列化器不能写裸字节）。
- **不加 fsync**（参照系 `write_atomic` 本体也不 fsync，保持一致；残余掉电窗口与硬化后的 vault 内容写相同）。
- chmod 维持 rename 后现有块不动（与 `write_vault` :129+ 同模式；unix 上 rename→chmod 的 ms 级权限窗口记 ponytail 声明，不修）。

验收标准（逐条可机械核对）：

1. 两个 vault 的钥匙写入走 tmp+rename 原子路径（崩溃不会再留半截钥匙文件）。
2. `write_atomic` 对外行为逐字节不变（现有 contract 测试全绿）。
3. 钥匙文件内容与权限语义不变（raw 32 bytes；unix 0o600）。
4. `cargo check --workspace` 与聚焦测试全绿，输出原文进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

2026-08-26 @ 0b195bc 实时核实：

| 事实 | 锚点 |
|---|---|
| 非原子写 1：`tokio::fs::write(&self.key_path, key.as_slice())` + 之后 chmod 0o600 | `src/crates/services/services-integrations/src/remote_ssh/password_vault.rs:57-66` |
| 非原子写 2：同模式 | `src/crates/services/services-integrations/src/mcp/auth.rs:114-124` |
| 两个读路径均 raw-bytes + `len() != 32 → bail`（fail-closed） | `password_vault.rs:42-50`、`auth.rs:97-105` |
| `JsonFileStore` 已在两处 scope 内（unit struct，用法 `JsonFileStore.write_atomic(...)`） | `password_vault.rs:10,125`、`auth.rs:184-198` |
| `write_atomic` 可复用的私有机制：`get_file_write_lock` / `build_temp_json_path`（nonce+pid+attempt）/ `replace_file_from_temp`（rename→删目标→rename）/ `is_retryable_write_error` / `retry_delay` / PermissionDenied fallback | `services-core/src/json_store.rs:136-259` |
| `JsonFileStoreError` 实现 `std::error::Error`（anyhow `.context()` 可直接包） | `json_store.rs:23-83` |
| contract 测试在 `services-core/tests/json_store_contracts.rs`（集成测试，tempdir 风格） | 现有 4 个测试 |
| vault 测试现成：mcp::auth::tests 8 个（I4+I5 轮实证 10/10 含之）；password_vault 测试在同 crate lib 内 | `--features product-full --lib` |
| 分层：services-integrations → services-core 依赖合法（现状即如此） | services-integrations/AGENTS.md |

## 3. 复用侦察（强制）

本任务核心就是复用：`write_atomic` 的全部重试/rename/fallback 机制必须被新 helper 共享（同模块私有 fn 直接调用），禁止复制粘贴循环体。report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. **`json_store.rs`**：新增

```rust
pub async fn write_bytes_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), JsonFileStoreError>
```

   方法体 = 现 `write_atomic` 的写循环（create_dir_all parent → 写锁 → tmp 写 → replace_file_from_temp 重试 → PermissionDenied fallback）。`write_atomic` 改为：serialize 成 bytes 后**委托** `write_bytes_atomic`（对外行为不变）。
2. **`password_vault.rs:57-59`**：替换为 `JsonFileStore.write_bytes_atomic(&self.key_path, key.as_slice()).await.context("write ssh password vault key")?;`。其后 `#[cfg(unix)]` chmod 块原样保留。
3. **`auth.rs:114-116`**：同款替换（`.context("write MCP OAuth vault key")?`），chmod 块保留。
4. **测试**（`services-core/tests/json_store_contracts.rs`，TDD 风格随文件现状）：
   - `write_bytes_atomic` roundtrip：写 `[u8]` → `tokio::fs::read` 逐字节相等；
   - 覆盖写：先写 A 再写 B → 读出 B；成功后目录无 `.tmp` 残留。
5. report 声明两个 ponytail 残余（不加 fsync / chmod 在 rename 后），各一句。

判断点（已授权）：新测试命名随 `json_store_contracts.rs` 惯例；错误 context 文案如上钉死。

## 5. Global Constraints（逐字遵守）

- 禁止改两个 vault 的读路径、钥匙文件格式、加密/解密函数。
- 禁止改 `write_atomic` 的签名与对外语义（序列化 pretty、锁、重试、fallback 全部保持）。
- 禁止给 helper 加 mode/fsync 等参数（speculative generality）。
- 日志只许英文、无 emoji。
- 不涉并发原语改动 —— 家规 4 不适用（文件写锁沿用现有机制，非新增）。
- json_store.rs 现 260 行 / password_vault.rs 353 / auth.rs 523，改后均须远低于 800。
- Windows 环境：写非 ASCII 一律用 edit 工具，禁用 PowerShell Set-Content（GBK 双重编码事故史）。
- 免费池铁律：假汇报 = 停用；编排者将 diff 逐条核对；验证输出必须贴原文进 report。

## 6. 验证（命令 + 输出原文都要进 report）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-core --test json_store_contracts
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-integrations --features product-full --lib password_vault
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-integrations --features product-full --lib mcp::auth
```

report 里每条命令贴完整输出尾部（含 test result 行）。report 无输出原文 = 假汇报嫌疑。

## 7. 报告

写入 `E:\agent-project\northing\.superpowers\sdd\audit-fix-i6-vault-key-atomic-report.md`：实现内容 / 复用侦察节 / 每个编译错误最终修在哪一层（机制层/设计层，一行一个）/ 测试与输出原文 / 文件清单 / 自审发现 / 疑虑。

最终回复只含（≤15 行）：Status（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit 短 SHA + subject、一行测试摘要、疑虑、report 路径。

## 8. 派发元信息

- BASE commit：`0b195bc`（派发前 HEAD）
- 禁区文件：`json_store.rs` 中除 `write_atomic` 委托化与新增 `write_bytes_atomic` 外的代码、两个 vault 的读/加解密路径、`remote_ssh/manager.rs`
- commit 规则：conventional commits（如 `fix(services): ...`），不加 AI 署名/co-author
- 工作目录：`E:\agent-project\northing`，直接在 main 工作（本会话既定流程）

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
