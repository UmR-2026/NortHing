STATUS: DONE

# Task B1 报告 — FU-1 save_user_config / delete_server_config fail-closed [security]

- 分支：`fix/backend-followups-0804`（worktree `backend-followups-0804`，基线 main `41695f5`）
- 交付 commit：`d4b11b5` `fix(security): MCP user-level config writes fail-closed on read errors (FU-1)`（4 files, +210/-29）
- brief：`.superpowers/sdd/task-b1-brief.md`；债项：`.superpowers/sdd/tech-debt-followups.md` FU-1（已同 commit 翻转 open→resolved）

## 1. 改动文件清单

| 文件 | 类型 | 摘要 |
|---|---|---|
| `src/crates/assembly/core/src/service/mcp/config/service.rs` | 修改 | **层 A**。把 `CoreMCPConfigStore::get_config_value` 的读结果分类提取为私有纯函数 `classify_config_read`：`Ok(v)→Ok(Some(v))`、`NortHingError::NotFound→Ok(None)`（合法空态）、其它错误→`Err(MCPRuntimeError::configuration)`（中止写）。此前 `Err(_)→Ok(None)` 把一切读错误吞成空配置。import 增 `NortHingError`。`mod tests` 新增 2 个测试。 |
| `src/crates/services/services-integrations/src/mcp/config/service.rs` | 修改 | **层 B**。`save_user_config` 与 `delete_server_config` 对未识别既有格式（非 `{"mcpServers":{...}}` object）改为 fail-closed：返回 `Err(configuration("Refusing to overwrite user-level MCP configs with unrecognized existing format"))`，既有值原样保留；delete 不再把未识别格式误判为 `not_found`。镜像 Task 6 `load_project_configs_strict`（`:128-148`）语义。project 级路径未动。 |
| `src/crates/services/services-integrations/tests/config_and_server_lifecycle.rs` | 修改 | 新增 4 个 user 级测试（纯新增 +98 行），镜像 project 级既有用例：save/delete × (store 读错误 fail-closed / 未识别格式拒写)。 |
| `.superpowers/sdd/tech-debt-followups.md` | 修改 | FU-1 状态 `open`→`resolved`（全局状态行 + FU-1 段落加修复说明），满足家规 2 同 commit 翻转。 |

commit 仅含上述 4 文件；`git status` 核对无无关文件（brief 保持未追踪，未提交）。

## 2. ConfigService.config 错误语义调查结论（层 A 分类依据）

调用链：`ConfigService::config::<Value>(Some(key))`（`service/config/service.rs:74-87`）→ `ConfigManager::get`（`service/config/mgr_validate.rs:8-16`）→ `get_value_by_path_from_config`（`mgr_validate.rs:115-129`）。

- **缺 key → `NortHingError::NotFound`**。路径按 `.` 分段在序列化的 `GlobalConfig` JSON 上走查，任一段缺失即 `NotFound("Config path '{path}' not found")`。`GlobalConfig.mcp_servers` 是 `Option<serde_json::Value>` 且 `#[serde(skip_serializing_if="Option::is_none")]`（`app_shell.rs:56-57`），未写入时该键不出现在 JSON 中 → 走查失败 → NotFound。`project.mcp_servers` 同理（`ProjectConfig.mcp_servers` Option+skip，`ProjectConfig` 空则整体 skip）。这是合法空态。
- **键存在 → `Ok(value)`**。
- **真实失败 → `NortHingError::Configuration`**（非 NotFound）：仅两处——(a) 内存 `GlobalConfig` 序列化失败（`mgr_validate.rs:116-117`，对合法内存结构实际不可达）；(b) 反序列化到目标类型失败（`mgr_validate.rs:14-15`）。当目标类型为 `serde_json::Value` 时任何 JSON 值都能成功反序列化，故 (b) 对 Value 不可达。

**分类结论**：`NotFound`=合法空态→`Ok(None)`；任何其它 `Err`=真实读/解析失败→`Err(Configuration)` 中止写。这正是 `classify_config_read` 的实现。分类是防御性的：即便活的 `ConfigService` 当前对 `config::<Value>` 只会产生 Ok 或 NotFound，一旦未来引入 IO/其它错误也能正确 fail-closed。

**测试注入手段说明**（brief §3 第 4 条要求）：活的 `ConfigService` 无法对 `config::<Value>` 注入非 NotFound 读错误（序列化恒成功、Value 反序列化恒成功）。因此：
- 「真实读错误→Err」分支用纯函数 `classify_config_read` 直接喂 `NortHingError::config("...")` 构造——这是能精确命中该分支的最小真实情形；
- 「缺 key→Ok(None)」分支除纯函数外，另加一个端到端用例：用 `PathManager::with_user_root_for_tests`（隔离 temp 根）构建真实 `ConfigService::with_settings`，对从未写入的 `mcp_servers` 断言 `get_config_value` 返回 `Ok(None)`，验证 NotFound→None 的真实接线。

## 3. 写入原子性核查结论

落盘链：`set_config_value`（core 适配器 `:29-38`）→ `ConfigService::set_config`（`service.rs:94-110`）→ `ConfigManager::set`（`mgr_validate.rs:19-41`）→ `save_config`（`mgr_load.rs:146-162`）。

- **结论：当前非原子。** `save_config` 用 `fs::write(&self.config_file, content)` 直接整文件写（`mgr_load.rs:158`），无 temp+rename。
- **未按顺手原子化，原因**：`mcp_servers` 不是独立文件，而是单一 `GlobalConfig` 文档（`app.json`）里的一个字段；唯一落盘点 `save_config` 被**所有 key**（set/reset/import 全路径）共享，是骨干不变量"Config single source of truth"的落盘处。原子化它=改动整个配置文档的持久化语义，改造面远超 brief 判定的"仅影响本 key 的落盘调用点"。按 brief 明示的出口（"改造面大则记入报告'范围外观察项'，不强改"）记为观察项，不强改。
- 后续若处理，可参照 services-core `json_store::write_atomic`（`services-core/src/json_store.rs:136`）：temp 文件 + rename（`replace_file_from_temp`）+ 重试 + Windows PermissionDenied 直写兜底。

## 4. 验证命令原文输出（brief §4，按序全跑，均通过）

前置：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

### 4.1 `cargo test -p northhing-services-integrations --features product-full mcp` → EXIT=0

`config_and_server_lifecycle.rs` 二进制块（含 4 个新增 user 级用例，全绿）：

```
     Running tests\config_and_server_lifecycle.rs (target\debug\deps\config_and_server_lifecycle-65f099875b0345cd.exe)

running 15 tests
test mcp_config_location_preserves_kebab_case_wire_contract ... ok
test mcp_server_type_and_status_preserve_lowercase_wire_contract ... ok
test mcp_config_authorization_helpers_preserve_header_precedence_and_normalization ... ok
test mcp_json_config_helpers_preserve_load_format_and_save_validation_contract ... ok
test mcp_config_merge_helpers_preserve_precedence_and_dedup_contract ... ok
test mcp_config_service_delete_user_fails_closed_on_config_store_read_error ... ok
test mcp_config_service_save_user_fails_closed_on_config_store_read_error ... ok
test mcp_config_service_save_user_fails_closed_on_unrecognized_existing_format ... ok
test mcp_config_service_save_project_fails_closed_on_unrecognized_existing_format ... ok
test mcp_config_service_keeps_load_failures_as_empty_baseline ... ok
test mcp_config_service_save_project_fails_closed_on_config_store_read_error ... ok
test mcp_config_service_delete_user_fails_closed_on_unrecognized_existing_format ... ok
test mcp_server_process_owner_preserves_unsupported_remote_transport_contract ... ok
test mcp_config_service_orchestration_preserves_load_save_delete_contract ... ok
test mcp_config_service_save_project_preserves_upsert_contract ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
```

该命令 `mcp` 过滤跨全部测试二进制合计 **44 passed; 0 failed**（各二进制 `test result: ok`，无失败）。关键契约保持：`keeps_load_failures_as_empty_baseline`（trait 层读错误 user save fail-closed + load 兜底空）与 project 级三用例仍全绿。

补充全套件（确认 mcp 之外无回归）：`cargo test -p northhing-services-integrations --features product-full` → **212 passed; 0 failed**（基线 208 + 本单新增 4）。

### 4.2 `cargo test -p northhing-core --features product-full --lib mcp` → EXIT=0

```
     Running unittests src\lib.rs (target\debug\deps\northhing_core-5d037dac39717bd0.exe)

running 13 tests
test agentic::agents::registry::tests::merge_dynamic_mcp_tools_appends_registered_mcp_tools_once ... ok
test service::mcp::config::service::tests::classify_config_read_maps_missing_key_to_none_and_real_failures_to_error ... ok
test service::mcp::server::manager::auth::tests::resolve_oauth_callback_locale_defaults_to_zh_cn ... ok
test service::mcp::server::manager::auth::tests::escape_html_replaces_all_special_chars ... ok
test service::mcp::server::manager::auth::tests::resolve_oauth_callback_locale_falls_back_to_accept_language ... ok
test service::mcp::server::manager::auth::tests::resolve_oauth_callback_locale_prefers_preferred_language ... ok
test service::mcp::server::manager::tests::backoff_delay_grows_exponentially_and_caps ... ok
test service::mcp::server::manager::interaction::tests::roots_list_does_not_fallback_to_process_current_dir_without_workspace ... ok
test service::mcp::server::manager::tests::detect_list_changed_kind_supports_three_catalogs ... ok
test service::mcp::config::service::tests::remote_authorization_prefers_headers_and_normalizes_tokens ... ok
test service::config::types::tests::global_config_preserves_project_mcp_servers ... ok
test agentic::tools::registry::tests::dynamic_tool_provider_prefers_mcp_registry_metadata ... ok
test service::mcp::config::service::tests::core_mcp_config_store_returns_none_for_missing_key_on_real_config_service ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 1124 filtered out; finished in 0.01s
```

mcp 相关子集全绿；含本单新增 2 个（`classify_config_read_...`、`core_mcp_config_store_returns_none_...`）。

### 4.3 `cargo check -p northhing-core --features product-full` → EXIT=0

```
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 18s
```

19 个 warning 均为预存 `unused variable`（agentic/session 代码），**无一涉及本次改动的 mcp/config 文件**（已 grep 核对）。0 error。

## 5. 观察项（范围外，不动手）

1. **写入非原子（FU-1 建议修复的原子落盘部分）**：`ConfigManager::save_config`（`mgr_load.rs:146-162`）直写整文件，非 temp+rename。因落盘点被全 key 共享、触及"Config single source of truth"骨干，改造面大，按 brief 出口记为观察项。建议后续独立任务用 `json_store::write_atomic` 模式原子化 GlobalConfig 文档落盘。
2. **基线测试计数与 brief 不符（非回归）**：brief 记 integrations 172/172、core lib 1134/1134。实测基线 `41695f5`：integrations 全套件 208（+本单 4=212）、core lib 总 1137（mcp 子集 13，含本单 +2）。integrations 差额（172→208）与已并入基线的 P1 安全分支新增测试一致；均为基线估算偏差，非本单引入。
3. **读侧宽容语义保持**：`load_all_configs`（`:57-80`）仍对 user/project load 错误 warn+empty 兜底；层 A 收紧后真实读错误经 `load_user_configs` 的 `?` 上抛、被 `load_all_configs` 捕获为空——端到端读侧行为不变（由既有用例 `keeps_load_failures_as_empty_baseline` 验证，未改动）。

## 6. 环境/过程说明（非代码结论，供审查溯源）

- 本 worktree 缺 gitignore 生成物 `generated_locale_contract.rs`，导致 northhing-core lib 测试无法编译（预存环境问题，非本单引入）。运行 `node scripts/generate-i18n-contract.mjs` 补齐（同时生成 5 个兄弟产物）。其中被追踪的 `src/apps/relay-server/static/homepage/i18n.shared.json` 被重写为 LF（内容相同，仅 CRLF→LF），已 `git checkout` 还原，未入 commit；其余生成物均 gitignore，未提交。
- `pnpm run fmt:rs`（brief 许可，只格式化改动 .rs）曾对 `tests/common/mod.rs` 产生仅换行/stat 的幻影改动（`git diff` 无内容差异），已还原，未入 commit。最终提交 diff 仅含 4 个范围内文件。

## 7. 与计划偏离处（显式声明）

- **唯一偏离**：计划/brief"建议修复"含"对写入走原子落盘"。本单**未**原子化写入——依据 brief §1"写入原子性核查"的明示出口：唯一落盘点 `save_config` 为全 key 共享、改造面大，故记为范围外观察项（见 §5.1），不强改。层 A/层 B fail-closed、测试、台账翻转均按 brief 完整交付。
- 无其它偏离。project 级路径（Task 6）未触碰；`load_user_configs`/`load_project_configs`/`load_all_configs` 读侧宽容语义未改；config store 其它 key 未审查。
