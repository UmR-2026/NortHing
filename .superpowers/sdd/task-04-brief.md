# Task 4 Brief: SSH/MCP OAuth vault fail-closed + 原子写

仓库：`E:\agent-project\.worktrees\northing-backend-debug`（分支 `fix/backend-debug-0731`，基线 ab6a91a）
来源：审计报告 H-5（vault 损坏/读取失败时按空 vault 写回，导致旧凭证全丢 + 非原子写截断风险）

## 已核实现状（编排者亲验）

- `services-integrations/src/remote_ssh/password_vault.rs`：
  - `store` L92-116：L96-97 `read_to_string(...).unwrap_or_default()` + `from_str(...).unwrap_or_default()`；L107 直接 `tokio::fs::write`。
  - `load` L118-146：L130-131 同 fail-open（load 路径，见下方需求区分）。
  - `remove` L148-162：L153-154 fail-open；entries 空时直接 remove_file。
  - `migrate_entry` L164-184：L172-173 fail-open；L178 直接 write。
- `services-integrations/src/mcp/auth.rs`：
  - `load` L153-187：L168-169 fail-open。
  - `store` L189-219：L194-195 fail-open；L208 直接 write。
  - `clear` L221-235+：L227-228 fail-open；entries 空 remove_file / 否则直接 write。
- **可复用设施**：`services-core/src/json_store.rs` `JsonFileStore::write_atomic`（L136-241）已实现 tmp（pid+nonce+attempt 命名）+ rename + Windows share-handle 重试。**优先复用，不重复造轮子**。services-integrations 已依赖 services-core（分层规则）。
- 两 vault 均在写后对 unix 设 0o600 权限——原子写替换后该行为必须保留（rename 后重新 set_permissions）。

## 需求

### 1. fail-closed：读/解析错误传播，禁止写回

所有**写路径**（password_vault 的 store/remove/migrate_entry；mcp/auth 的 store/clear）：
- vault 文件不存在 → 按空 `VaultFile::default()` 继续（合法初始态）。
- 文件存在但 `read_to_string` 失败 → `Err` 传播（带 context），**不得写**。
- 文件存在但 JSON 解析失败 → `Err` 传播（带 "vault corrupted, refusing to overwrite" 类 context），**不得写**。

**读路径**（两者的 load）：
- JSON 损坏 → 同样返回 `Err`（调用方需要知道 vault 坏了，而不是当成"无凭证"再去触发覆盖写）。注意调用点兼容：grep load 调用方确认 Err 传播不会破坏正常流程（调用方本来就处理 Result）。
- 单个 entry 解密失败 → 保持现状 `warn + Ok(None)`（单条损坏不拖死整个 vault，属合理设计，不改）。

### 2. 原子写 + 备份

- 所有 vault 写入改经 `JsonFileStore::write_atomic`（或其实例方法等价调用）；若 json_store API 与 vault 需求不完全匹配（如需要 pretty 格式、自定义后缀），可在 vault 模块内复刻其 tmp+nonce+rename 模式并注明来源，但优先真复用。
- rename 前若目标已存在：先 `tokio::fs::copy(target, target.with_extension("bak"))` 留一份备份（失败仅 warn，不阻塞写）。
- unix 0o600 权限在 rename 后补设（保持现状行为）。

### 3. 测试（必须）

每个 vault 各覆盖（可共享 test helper）：
- 损坏 JSON（手写 garbage 进 vault 文件）+ store → Err 且**原文件字节不变**。
- 损坏 JSON + remove/clear/migrate → Err 且原文件不变。
- 截断文件（合法 JSON 前缀一半）→ 同上 fail-closed。
- 正常 store → 文件可读回、`.bak` 在第二次写后存在且为上一版内容。
- load 在损坏时返回 Err（非 Ok(None)）。
- entries 清空后的 remove_file 行为保持（clear/remove 到空的语义不变，但前提是解析成功）。

## 明确不做

- 不改 VaultFile schema、加密结构、key 管理。
- 不动 desktop 调用点 UI 流程。
- 不动 H-6/H-7/H-8 范围（bot persistence / mcp config service / miniapp，后续任务）。
- 不 git commit。

## 约束（逐字）

- Logs must be English-only, with no emojis.
- 严禁裸 `cargo fmt`；只许 `cargo fmt -p northhing-services-integrations`。
- 顺手清配额可行，但 commit message 可追溯（report 中列出）。
- 若发现 load 调用点对新增的 Err 传播有行为敏感（如把 Err 当致命弹窗），在 report 披露而非静默改语义。

## 验证命令

```
cargo check -p northhing-services-integrations
cargo test -p northhing-services-integrations vault
cargo test -p northhing-services-integrations password_vault
cargo test -p northhing-services-integrations mcp
```
（focused 过滤器按实际测试名调整；report 贴实际命令与输出）

## Report

写 `.superpowers/sdd/task-04-report.md`：改动 file:line、复用 json_store 的方式选择及理由、测试清单与输出、load Err 传播的调用点影响评估、状态。
