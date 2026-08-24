# Task 6 Report: MCP 项目配置 + MiniApp sync fail-closed（H-7/H-8）

- 分支：`fix/backend-debug-0731`（基线 a53711e）
- 状态：**DONE** — 实现 + 测试 + 全部验证通过；未 commit

## 改动清单（file:line）

### H-7 `src/crates/services/services-integrations/src/mcp/config/service.rs`

- `load_project_configs_strict`（新增，L123-148）：`load_project_configs`（读路径，保持宽容）的严格变体。`get_config_value` 错误经 `?` 传播；config_value 存在但既非 cursor 对象也非数组 → `MCPRuntimeError::configuration("Refusing to overwrite project MCP configs with unrecognized existing format")`；`None` → 空 Vec（合法初始态）。
- `save_project_config` L240：`self.load_project_configs().await.unwrap_or_default()` → `self.load_project_configs_strict().await?`。
- 读路径 `load_all_configs`/`load_project_configs` 未动（列表页仍宽容）。
- `delete_server_config`（L228-261）核对结论：**已是 fail-closed，无需修改**。get 错误经 `?` 传播；`unwrap_or_else(json!({"mcpServers": {}}))` 仅覆盖 key 缺失（None）的合法初始态；值存在但形状不可识别（非 `mcpServers` 对象）→ 走 `else` 分支返回 `MCPRuntimeError::not_found`，不覆盖。
- 用户级 `save_user_config` L186-190 存在**同款模式但不在本任务范围**：现有值形状不可识别（如 legacy 数组/`42`）时静默回落为空 map 并整体覆盖。H-7 审计项与 brief 均只点名项目级写路径，未授权动用户级；已在下方「披露」列出，建议终审 triage。

### H-8 `src/crates/services/services-integrations/src/miniapp/storage_app_io.rs`

- `load_source_from_dirs` L101-137 改造：
  - `index.html`（必需）：NotFound → `MiniAppStorageError::not_found`；其他 IO 错误 → `io` Err（L107-115）。
  - `style.css`/`ui.js`/`worker.js`（可选）：走新增模块级 helper `read_optional_source_file`（L317-324）——NotFound → 空串；其他 IO 错误 → `io` Err（不再静默空）。
  - `esm_deps.json`：exists 但读失败 → `io` Err；解析失败 → `parse` Err（不再静默空 Vec）。
  - 空文件语义不变：真实存在且内容为空的文件仍返回空串。

### 测试

- `tests/config_and_server_lifecycle.rs`（integration，feature `mcp`）：
  - `mcp_config_service_save_project_fails_closed_on_config_store_read_error` L95：新增 `RecordingFailingGetMCPConfigStore`（get 恒 Err、set 记录调用），save project → Err(Configuration) 且 `set_calls` 为空。
  - `mcp_config_service_save_project_fails_closed_on_unrecognized_existing_format` L122：`project.mcp_servers` 预置 `42` → save → Err(Configuration)，值未被写。
  - `mcp_config_service_save_project_preserves_upsert_contract` L154：legacy 数组预置 + 新增 id 保序追加；同 id 覆盖不改数组形状。
- `src/miniapp/storage_tests.rs`（unit）：`load_source_from_dirs_*` 8 例 L417-556：
  - missing index.html → NotFound；index.html 为目录（非 NotFound 读失败）→ Io。
  - style.css/worker.js 缺失 → Ok 且对应字段空串；style.css 为目录 → Io。
  - esm_deps.json 损坏 JSON → Deserialization；esm_deps.json 为目录 → Io。
  - 真实空 index.html/style.css → Ok 空串；全文件齐备 → 内容/依赖全部正确。
- `src/crates/assembly/core/src/miniapp/manager/mod.rs` L256：`sync_from_fs_fails_closed_and_skips_persist_when_source_is_corrupt`——save 后删除 source 目录 index.html → `sync_from_fs` Err（错误含 "index.html"）→ meta.json version 未变、`list_versions` 为空（`persist_sync_from_fs_result_for_app` 未被执行）。

## load_source 调用点影响评估

`load_source_from_dirs` 全部调用方（grep 全仓确认）：

| 调用点 | 路径 | 语义变化影响 |
|---|---|---|
| `storage_app_io.rs:65` `load()` → `load_source()` | `MiniAppStorage::load` | 缺 index.html：原先返回空 html 的 app，现为 `not_found` Err。**目标行为**：损坏 app 不再静默变成"空白 app"（空白会经 update/recompile 回写覆盖） |
| `mgr_lifecycle.rs:363` `sync_from_fs` → `storage.load_source_only` | 同步流程 | 缺 index.html → `load()` 在 L362 即失败（meta 之后先读 source）→ `persist_sync_from_fs_result_for_app` 不执行。**目标行为：sync 中止、不覆盖**（本次新增的 core 层测试直接证明） |
| `mgr_lifecycle.rs:348` `recompile` → `storage.load` | 重编译 | 缺 index.html → Err，不再用空源码编译并 persist。fail-closed 一致 |
| `storage_drafts.rs:52` `load_draft_app` → `load_source_from_dirs(draft_dir)` | 草稿打开 | 草稿由 `save_app_files` 全量写盘（index.html 必写），正常草稿不受影响；损坏草稿由空页变显式 not_found。可接受 |
| `storage_port.rs:30` / `assembly/core/storage.rs:44` port/facade `load_source_only` | 编辑器打开/展示 | `MiniAppStorageError::NotFound` → `MiniAppPortErrorKind::NotFound` / `NortHingError::NotFound`，错误类型映射已存在且正确（storage_port.rs:135-144、assembly storage.rs:317-325），**无代码改动需求** |
| `builtin/mod.rs:275` 内置种子测试 `sync_from_fs(...).unwrap()` | 测试 | 种子经 `save_app_files` 全量写盘，index.html 必在，不受影响（core miniapp 28 例全过） |

结论：not_found 语义变化只影响「损坏源目录」路径——从静默空页/空源码升级为显式错误，编辑器打开损坏 app 从"空白页"变"显式 not_found 错误"。无调用点需要改代码。

## 语义变化披露

1. **H-7 写路径**：项目级 save 在 store 读错误或现有值形状不可识别时返回 Err（Configuration），不再以空基线覆盖。读路径不变。
2. **H-8 读路径**：index.html 缺失从「空串」变「not_found Err」；css/js/worker 非 NotFound 读错误从「空串」变「io Err」；esm_deps.json 读/解析失败从「空 Vec」变「io/parse Err」。
3. **观察（未改，超范围）**：`save_user_config`（service.rs L186-190）与 H-7 同类的 read-modify-write fail-open（现有值形状不可识别时静默回落空 map 覆盖）。brief 只授权项目级；建议终审决定是否纳入后续任务。
4. 工作树存在大量**预先存在的 LF/CRLF stat 噪声**（1580 个 ` M`，内容 hash 与 HEAD 一致，`git diff` 为空，非本任务产生）；本任务实际 diff 仅 5 个文件、未触碰其他改动。

## 测试与输出

```
cargo check -p northhing-services-integrations --features product-full   # Finished, no errors
cargo test  -p northhing-services-integrations --features product-full mcp      # 11 passed (config_and_server_lifecycle, 含 3 新增)
cargo test  -p northhing-services-integrations --features product-full miniapp  # 29 passed (storage_tests, 含 8 新增)
cargo check -p northhing-core --features product-full                   # Finished, no errors（动了 assembly/core 测试）
cargo test  -p northhing-core --features product-full sync_from_fs      # 1 passed（新增 sync 中止测试）
cargo test  -p northhing-core --features product-full miniapp           # 28 passed（既有 miniapp 套件无回归）
node scripts/check-core-boundaries.mjs                                  # Core boundary check passed.
```

格式化：仅执行了 brief 允许的 `cargo fmt -p northhing-services-integrations`（只影响本任务 4 个文件）；assembly/core 未格式化（`git diff --check` 干净）。

## 约束合规

- 未 git commit；未触碰非本任务改动。
- 未改 config store 接口/schema、未改 `MiniAppSource` 结构、未动 `sync_from_fs` 编译逻辑本身（仅新增测试）。
- 日志无新增（未引入任何 log 语句）；新增错误信息为英文。
- assembly/core 改动仅 test module 内新增 1 个测试函数（+44 行），无生产代码改动。
