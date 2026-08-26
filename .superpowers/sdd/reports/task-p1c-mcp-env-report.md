# Task P1c Report — B2 MCP env keyring

## Status
DONE_WITH_CONCERNS

用户 2026-08-26 裁定：留 WIP、**不翻 P1-8**。本单按处方 v3 把 keyring 接到 `AppSettings.mcp_servers`（`~/.northhing/config/app.json`）。K4a 之后该字段无生产写入（Settings/MCP 走 facade `list_mcp_servers`）；真实 env 明文在 core Cursor 格式 `mcp_servers`（`cursor_format.rs:66`）。

## Files
- `src/apps/desktop/src/app_state/settings/keyring.rs` — `MCP_ENV_SENTINEL` / `store_env` / `load_env` / `resolve_env` / `delete_env` + MockKeyring 单测
- `src/apps/desktop/src/app_state/settings/io.rs` — load 迁移+还原；save 前 `prepare_settings_for_save`
- `src/apps/desktop/src/app_state/settings/io/io_tests.rs` — sentinel 落盘/还原、fail-open、idempotent、store fail-closed
- `src/apps/desktop/src/app_state/settings/mod.rs` — 恢复 `mcp_servers` + `upsert_mcp`/`remove_mcp`
- `src/apps/desktop/src/app_state/settings/types.rs` — 恢复 `MCPServerConfig`（含 `env`）
- `docs/status/tech-debt-ledger.md` — P1-8 仍 active；证据路径标 stale（家规 2：未 resolved 故不翻转）

## 复用侦察
- 复用 C3 `KeyringBackend` / `MockKeyring` / `store_api_key` 形态；sentinel 用 map `{"__kr_env__":"true"}` 对齐 HashMap 字段（非 api_key 的字符串 sentinel）。
- 未新写第二套 keyring。未动 services-integrations / GlobalConfig（brief 禁区）。

## 偏离
- **P1-8 不翻转**：用户裁定。处方/brief 写「同 commit 翻 resolved」被覆盖。
- `MCPServerConfig` 从 AppSettings 消失后又加回——仅为打通 io.rs 处方路径；无 UI/facade 接线。
- GNU `cargo test -p northhing --lib` 链接失败（已知 `-lshlwapi`）；未跑通 settings/keyring 测试。`cargo check -p northhing` 通过。

## 验证

### cargo test settings / keyring
```
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo test -p northhing --lib settings
cargo test -p northhing --lib keyring
```
两次均为：`error: linking with x86_64-w64-mingw32-gcc failed` → `ld: cannot find -lshlwapi`（archived HANDOFF 已知 GNU 测试二进制问题，非本 diff 引入）。测试源码已写，本机 GNU 链未执行。

### cargo check -p northhing
`Finished dev profile [unoptimized + debuginfo] target(s) in 2m 14s`；pwsh 链 exit 0（末条 check 成功）。既有 warning，无本文件 error。
