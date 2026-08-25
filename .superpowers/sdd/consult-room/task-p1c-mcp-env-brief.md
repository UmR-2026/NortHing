# Task P1c Brief — B2 MCP env 变量 keyring 化

> 需求唯一来源：`.superpowers/sdd/consult-room/prescription-v3-20260825.md` §B2。
> Base commit: `34f4214`（P1b 已落）。

## 范围

参照**既有 C3 provider api_key 模式**（`app_state/settings/io.rs:79-113` `keyring_migrate_providers` + `keyring.rs` `store_api_key`/`resolve_api_key`），为 MCP env 做同形迁移。**读既有实现再动手，模式逐字对齐。**

### 1. `src/apps/desktop/src/app_state/settings/keyring.rs` 追加

```rust
pub const MCP_ENV_SENTINEL: &str = "__kr_env__";

/// 整块 env JSON 存入 keyring entry "mcp-env:{server_id}"，返回 sentinel。
pub fn store_env(keyring: &dyn KeyringBackend, server_id: &str, env: &HashMap<String, String>) -> Result<String>

/// sentinel 还原；entry 缺失/解析失败 → Ok(HashMap::new())（fail-open 兼容旧数据）+ warn。
pub fn load_env(keyring: &dyn KeyringBackend, server_id: &str) -> Result<HashMap<String, String>>
```

### 2. `src/apps/desktop/src/app_state/settings/io.rs` 接入

- **保存路径**（`update_app_settings_at`/`save_app_settings_at` 链）：写盘前遍历 `settings` 里各 MCP server 配置，`env` 非空 → `store_env` → 磁盘写 `{"__kr_env__": true}` 形态或 sentinel 字符串（与 C3 的字段形态对齐——读 C3 现有 sentinel 写法再定）。
- **加载路径**（`load_app_settings_at`）：遇 sentinel → `load_env` 还原进内存结构。
- **范围**：仅 user 级 `~/.northhing/config/app.json`；project 级 Cursor 格式 mcp 配置不动。
- keyring backend 获取方式与 C3 相同（读现有调用点）。

### 3. 测试

- `store_env`/`load_env` 用 `MockKeyring` roundtrip 单测
- sentinel 写入/还原集成测试（落盘 JSON 无明文 env；加载还原）
- fail-open：keyring entry 缺失 → 空 map + 不 panic

## 禁区

- 不动 C3 provider key 路径的行为
- 不动 GlobalConfig / core 侧
- 不引入 per-variable sentinel（整块 JSON 一个 entry）
- 不动 services-integrations

## 验证（必跑并贴输出）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cd E:\agent-project\northing
cargo test -p northhing --lib settings
cargo test -p northhing --lib keyring
cargo check -p northhing
```

报告：`.superpowers/sdd/reports/task-p1c-mcp-env-report.md`（status + files + 验证输出原文 + 偏离声明）。

## 完成后义务

tech-debt-ledger.md 的 **P1-8 状态翻转 resolved**（家规 2：同 commit），commit message 里注明。
