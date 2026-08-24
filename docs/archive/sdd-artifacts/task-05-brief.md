# Task 5 Brief: Remote bot persistence 单写者事务（H-6）

仓库：`E:\agent-project\.worktrees\northing-backend-debug`（分支 `fix/backend-debug-0731`，基线 88c719a）
来源：审计报告 H-6（损坏 JSON → `unwrap_or_default` → 多平台独立 load-modify-save → 直接写回；并发保存互相覆盖，连接配置/token/chat state/form state/verbose 可整体丢失）

## 已核实现状（编排者亲验）

- `src/crates/assembly/core/src/service/remote_connect/bot/mod.rs`：
  - `load_bot_persistence` L483-499：同步 fn；主文件读失败 fallback legacy 文件；两处均 `serde_json::from_str(...).unwrap_or_default()` fail-open。
  - `save_bot_persistence` L501-513：同步 fn；`std::fs::write` 直接写，无锁、无原子、无备份；失败仅 error 日志。
- 调用点（审计给出，派发后先逐一核对现状行号再动手）：
  - `command_router_dispatch.rs:171-174`
  - `feishu/feishu_commands.rs:290-302`
  - `telegram.rs:638-649`
  - `weixin_bot_inbound.rs:207-220`
  模式均为「load → 改字段 → save 整文件」。
- 参考设施：services-core `json_store.rs` 的原子写是 async tokio 版；本模块是 std 同步上下文，需要 std 等价物（tmp+rename），可复刻模式并注明来源。

## 需求

### 1. 新增事务式单写者 API（bot/mod.rs）

```rust
pub fn update_bot_persistence(
    f: impl FnOnce(&mut BotPersistenceData),
) -> Result<(), BotPersistenceError>
```

- 进程内单写锁：`static` `std::sync::Mutex<()>`（同步上下文，不跨 await；锁内只做 load→f→write，快进快出）。
- 锁内 fail-closed load：主文件存在但读/解析失败 → 返回 Err（"corrupted, refusing to overwrite"），**不执行 f、不写**。主文件不存在走 legacy fallback（保留现状迁移语义）；legacy 也损坏 → Err。
- f 执行后原子写：tmp（pid+nonce）→ rename；写前已存在目标 copy 为 `.bak`（失败仅 warn）。
- 错误类型含 Read/Parse/Io 分类，调用方可 match。

### 2. load 读路径语义

`load_bot_persistence` 签名保持不变（兼容只读调用方），但内部：
- 损坏时 `tracing::warn!` 明确记录（文件路径+错误），返回 default——只读不写回即无害；
- 新增 `try_load_bot_persistence() -> Result<BotPersistenceData, BotPersistenceError>` 供 update 与需要 fail-closed 的调用方使用。

### 3. 四个调用点迁移

逐一改为 `update_bot_persistence(|data| { ... })` 闭包形式；错误处理按各调用点现状语义（warn 记录或向上传播），不得静默吞 Err。

### 4. 测试（必须，并发规则 4）

- 并发事务：10 线程同时对空文件 `update_bot_persistence` 各插入不同条目 → 最终文件包含全部 10 条（无丢更新）。
- 损坏文件 + update → Err，文件字节不变，f 未被执行（用副作用标记断言）。
- 损坏 + `load_bot_persistence` → default + warn（不 panic）。
- 第二次成功写后 `.bak` 存在且内容为上一版。
- legacy fallback：主文件不存在 + legacy 存在 → 正常载入。
（测试需隔离 HOME：用环境变量/tempdir 注入路径——若 `bot_persistence_path` 依赖 `dirs::home_dir()`，加 `#[cfg(test)]` 路径覆盖机制或抽路径参数。）

## 明确不做

- 不改 BotPersistenceData schema、不改文件路径布局。
- 不改 bot 消息处理逻辑（只动持久化通道）。
- 其他 H-5/H-7/H-8 范围不动。
- 不 git commit。

## 约束（逐字）

- Logs must be English-only, with no emojis.
- 严禁裸 `cargo fmt`；只许 `cargo fmt -p northhing-core`（确认 crate 名后执行）。
- 生产 .rs 文件超 800 行注意 review pressure（bot/mod.rs 现 586 行，改动后仍须 <800 或说明）。
- 改动涉及并发 → 必须随附自动化测试（规则 4）。

## 验证命令

```
cargo check -p northhing-core
cargo test -p northhing-core bot_persistence
cargo test -p northhing-core remote_connect
```
（crate 名/过滤器按实际调整，report 贴实际命令输出）

## Report

写 `.superpowers/sdd/task-05-report.md`：改动 file:line、四调用点迁移清单、锁设计（为何 std::sync::Mutex）、测试与输出、调用点错误语义变化披露、状态。
