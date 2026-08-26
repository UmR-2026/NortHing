### Spec Compliance

- ❌ Issues found (file:line)

Implementation matches the brief's structural and behavioral contract (落点 / 签名 / sentinel / fail-open / 整块 JSON / 不动禁区 / ledger 状态), but the brief's explicit verification gate was not satisfied — `cargo test -p northhing --lib settings|keyring` did not run on the implementer's machine; only the linker error is documented. Per brief 验证段："必跑并贴输出"。

Detail verdict by brief item:

| Brief item | Status | Evidence |
|---|---|---|
| `keyring.rs` 追加 `MCP_ENV_SENTINEL = "__kr_env__"` | PASS | `keyring.rs:62` |
| `store_env(&dyn KeyringBackend, server_id: &str, env: &HashMap<String,String>) -> Result<String>` 签名 | PASS | `keyring.rs:278-290`（签名一致；serializes env to JSON → `mcp-env:{server_id}` → returns sentinel） |
| `load_env(&dyn KeyringBackend, server_id: &str) -> Result<HashMap<String,String>>` 签名 | PASS | `keyring.rs:298-319`（fail-open：warn + 空 map，兼容旧数据） |
| `io.rs` load 迁移：遇 sentinel → `load_env` 还原 | PASS | `io.rs:68-77` `keyring_migrate_mcp_servers` |
| `io.rs` save 迁移：写盘前遍历 → `store_env` → 磁盘 sentinel | PASS | `io.rs:89-114` `prepare_settings_for_save`（含 `Err` 复原 plaintext 路径） |
| 不动 C3 provider key 路径 | PASS | `keyring.rs:57 API_KEY_SENTINEL`、`:226-265 resolve_api_key/store_api_key/delete_api_key`、`:354-465` 既有测试 — diff 仅追加新内容，0 删除 0 改 |
| 不动 GlobalConfig / core 侧 | PASS | `git diff --name-only d4f8779..0b14f8a` 8 文件全部在 `apps/desktop/src/app_state/settings/` 与 `.superpowers/sdd/`、 `docs/status/` |
| 不引入 per-variable sentinel（整块 JSON 一个 entry） | PASS | `keyring.rs:287` `serde_json::to_string(env)` 整块序列化；账户名 `mcp-env:{server_id}` 单条 entry |
| 不动 services-integrations | PASS | diff 文件清单无 services-integrations 路径 |
| 范围仅 user 级 `~/.northhing/config/app.json` | PASS | `io.rs:18-21` `app_settings_path()` 仅写 user 级；Cursor 格式 `mcp_servers`（`services-integrations/src/mcp/config/cursor_format.rs:66`）未触碰 |
| MockKeyring roundtrip 单测 | PASS | `keyring.rs:474-486` |
| sentinel 落盘 / 还原 集成测试 | PASS | `io_tests.rs:230-274` (plaintext→sentinel on load) + `:277-311` (sentinel→keyring restore) |
| fail-open：keyring 缺失 → 空 map + 不 panic | PASS | `keyring.rs:498-503` 单测 + `io_tests.rs:347-365` 集成 |
| store fail-closed | PASS | `keyring.rs` `store_env` `keyring.store(&account, &json)?` 失败冒泡；`io_tests.rs:401-441` `FailingKeyring` 集成 |
| 验证（必跑并贴输出）`cargo test -p northhing --lib settings` / `keyring` | **FAIL** | report §验证 仅贴 `ld: cannot find -lshlwapi` 失败；未贴任何 test pass 输出。`cargo check -p northhing` 通过（这是 AGENTS.md 家规 6 的硬门，不是 brief 验证段的最小集） |
| ledger P1-8 翻 resolved（家规 2 同 commit） | OVERRIDDEN | 用户 2026-08-26 拍板："P1-8 保持 active；ledger 仅标注证据路径 stale"。`tech-debt-ledger.md:78` 现状：evidence 段已加 "Stale after K4a"，status 仍 `active`（tech-debt-ledger.md:80）；未 claim resolved ✓ 符合用户裁定 |
| `cargo check -p northhing` 通过（AGENTS.md 家规 6） | PASS | report 末段：Finished dev profile 2m14s, exit 0 |

⚠️ Cannot verify from diff

- 集成测试在 MSVC 工具链下是否实际通过。源码已写、check 已过；但 GNU 链接器环境无法产出测试二进制，MSVC 工具链的实测 pass 集缺失。本机复跑需求待 CI / 后续 reviewer 在 MSVC 环境补做。
- Diff 包含的 `keyring.rs` 在最终 commit 是 547 行（diff +192 / pre 355），仍 < 800（无 `allow-god-file` 触发条件；不在 `scripts/rot-budget.json` 7 个 manifest 名单内）。但 `keyring.rs` 行数靠近 600 警戒线（800 是 review pressure，> 1000 必须拆或带 `allow-god-file`），后续若再 append 可能撞线。

### Strengths

- **模式与 C3 同形**：`store_env`/`load_env` 的签名、sentinel 命名 (`__kr_env__` 对齐 `__kr__`)、`is_*_sentinel` + `make_*_sentinel` 镜像、`PRODUCTION_KEYRING` 复用 — diff inspection 与 `task-c3-diff.patch:160-219` 逐字同结构。
- **fail-closed 一致**：`prepare_settings_for_save`（`io.rs:89-114`）与 C3 `keyring_migrate_providers` 一致用 `std::mem::take` + 失败 `Err(e).context(...)` 复原 plaintext，绝不写 sentinel 进内存，绝不触发后续 save。
- **迁移幂等**：`is_env_sentinel` 短路（`keyring.rs:283-285`）+ `is_empty()` 短路（`io.rs:95-96`）+ `MockKeyring` 第二次 load 不重写 keyring（`io_tests.rs:369-397`）三处独立护栏。
- **idempotent save-side 注入**：每次 `update_app_settings_at` 走 `prepare_settings_for_save`，新加的 MCP env 自动迁移到 keyring，与 C3 「新加 key 自动迁移」语义对齐。
- **loader 写盘**（`io.rs:51-55`）：迁移 count > 0 时同步把 sentinel 形态写盘，避免每次启动都重跑 migrate — 与 C3 当时行为一致。
- **test 覆盖面**：6 个新集成测试覆盖 plaintext→sentinel / sentinel→keyring / 新建 env / fail-open / idempotent / store-fail-closed；7 个新单测覆盖 sentinel identity / roundtrip / sentinel-noop / fail-open / corrupt-json / resolve / delete。
- **ledger 诚实**：`MCPServerConfig` 复活但 `AppSettings.mcp_servers` 是 K4a 后 dead 字段（`callbacks_settings/refresh.rs:25-26` 注释确认）；P1-8 evidence 段加 "Stale after K4a" + 不 claim resolved，与用户裁定一致。
- **不越层**：8 文件 diff 全在 desktop app_state/settings 子树，无 core / services-integrations / GlobalConfig / Cursor 格式 触碰。

### Issues

#### Critical

None.

#### Important

1. **未执行 brief 验证段**（file:line: brief.md:42-50）。`cargo test -p northhing --lib settings` 与 `cargo test -p northhing --lib keyring` 均未跑通：报告贴的输出是 GNU ld `-lshlwapi` 链接错误。Brief 验证段明示"必跑并贴输出"。本批 source 静态可见、check 门过；但 roundtrip 单测（`keyring.rs:474-486`）与 6 个新集成测试（`io_tests.rs:230-441`）在本机零实测证据。AGENTS.md 家规 6 仅要求 `cargo check -p northhing` 通过；brief 验证段比家规 6 更严，但两者并不冲突 — 本批两者缺一。建议：(a) 后续 reviewer 在 MSVC 工具链复跑这两条 `cargo test`；(b) brief 验证段应在下次 v4 处方里写明 "check 门即可，MSVC CI 兜底"，避免单子被同一环境问题反复堵。

2. **scope 外加 dead helper**（`keyring.rs:71-74`、`325-336`、`339-346`）。Brief 仅要求 `store_env` + `load_env`（task-p1c-mcp-env-brief.md:16-20），未要求 `is_mcp_env_sentinel` / `resolve_env` / `delete_env`。三函数全部 `#[allow(dead_code)]`，无 production caller（grep `resolve_env|delete_env|is_mcp_env_sentinel` 在 `src/apps/desktop/src/` 内零命中；tests 内零调用 — `io_tests.rs` 走 `is_env_sentinel`（map 版）不走 `is_mcp_env_sentinel`（str 版））。按家规 0 「无 owner 抽象：投机性抽象 = Important 起评」，三函数属投机扩展。C3 `delete_api_key` 有 production caller (`sync.rs` + `provider_test.rs` per ledger P1-2 resolution notes)，与此处的 dead state 不同。建议：删除 `is_mcp_env_sentinel` + `resolve_env` + `delete_env`；或在下次 B2 真实写入路径出现前以 `// ponytail: dead, remove if next B2 wave doesn't wire` 标记，并加进 ledger 提醒。

#### Minor

1. **loader 内 save 路径**（`io.rs:51-55`）：`load_app_settings_at` 检测到 plaintext 后立即 `save_app_settings_at`。`update_app_settings_at`（`io.rs:134`）再调 `load_app_settings_at`，`load` 完后再走 `prepare_settings_for_save` + `save_app_settings_at`（`io.rs:137-145`）。同一事务内对同一份 settings 做了两次「迁移 + 写盘」，第二次的迁移永远是 no-op（已经在 load 时变成 sentinel 了），但每次更新都会触发一份 `settings.clone()`（`io.rs:137`）+ 一次 sentinel 检查 + 一次 save。功能正确，仅冗余。建议下次若触发性能压测再拆；现阶段 Ponytail "走通先"。

2. **ledger evidence path 措辞**：`tech-debt-ledger.md:78` "originally `src/apps/desktop/src/app_state/settings/types.rs` `MCPServerConfig.env` in desktop `app.json`" — `MCPServerConfig` 已在本次 diff 中重新加回（`types.rs:127-151`），evidence 措辞 "originally ... in" 现在需读为"现状也 in"，但更准的写法是 "stale after K4a; MCPServerConfig has been re-introduced in `types.rs:127` by P1c for the keyring integration path, but has no production writer after K4a — see `callbacks_settings/refresh.rs:25-26`"。建议后续 ledger 卫生轮一并整句改写。

3. **`MCPServerConfig` / `mcp_servers` 复活 vs production dead**：diff 引入 `MCPServerConfig`（types.rs:127-151）、`AppSettings.mcp_servers: Vec<MCPServerConfig>`（mod.rs:70-71,81）、`upsert_mcp`/`remove_mcp`（mod.rs:130-142）、`MCPTransport`（types.rs:117-124）— 全部 `#[allow(dead_code)]`。功能正确，但 dead code 量约 +60 行（types.rs ~30L、mod.rs ~15L）。本次为打通 io.rs 处方路径所必需；下次 B2 真正接入 facade 时应去除 `#[allow(dead_code)]` 并补 1-2 个 production-caller 单测。

4. **god-file 健康度**：`keyring.rs` post-diff 547 行；`io.rs` 219 行；`io_tests.rs` 441 行；`mod.rs` 153 行；`types.rs` 160 行。全部 < 800，无 `allow-god-file` 触发条件；不在 `scripts/rot-budget.json` 7 个 god-file manifest 名录（`callbacks_lifecycle.rs` / `theme.rs` / `memory_db.rs` / `selectors.rs` / `manager.rs` / `chat/input.rs` + `dir_entries:*` 3 项）— 预算闸无影响。本批也未触碰 `scripts/rot-budget.json`（`git diff -- scripts/rot-budget.json` 零行），house rule 7（rot-budget 只降不升）未触发。

### Assessment

**Task quality:** Needs fixes

**Reasoning:** Diff 完整覆盖 brief 结构与行为契约（C3 模式同形 / sentinel / fail-open / 整块 JSON / 禁区 / ledger 与用户裁定一致），实现质量扎实（fail-closed 复原 / 幂等护栏 / 测试覆盖面广）；但 brief 验证段 "必跑并贴输出" 未满足（仅贴 link 错误，零 test pass 输出）属 Important；外加 `is_mcp_env_sentinel` / `resolve_env` / `delete_env` 三 dead helper 越出 brief scope，属 Important 投机抽象。两项 Important 都需处理：补 MSVC 工具链实测（或在下次处方里降级为 check 门）+ 删 dead helper（或加 ponytail 标记并入账）。Critical 无，Minor 都是卫生/冗余类。