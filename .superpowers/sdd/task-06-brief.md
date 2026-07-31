# Task 6 Brief: MCP 项目配置 + MiniApp sync fail-closed（H-7/H-8）

仓库：`E:\agent-project\.worktrees\northing-backend-debug`（分支 `fix/backend-debug-0731`，基线 a53711e）
来源：审计报告 H-7（项目级 MCP 配置读失败时保存单项覆盖全部）、H-8（MiniApp 源码读取失败被保存为空）

## 已核实现状（编排者亲验）

### H-7 `services-integrations/src/mcp/config/service.rs`
- `save_project_config` L212-226：L213 `self.load_project_configs().await.unwrap_or_default()` —— load 失败（config store 读错误或格式不明）→ 空 Vec → upsert 单项 → `set_config_value("project.mcp_servers", ...)` 覆盖全部项目级配置。
- `load_project_configs` L103-120：`get_config_value(...).await?` 已传播 store 错误（好）；但 L112-117「config_value 存在但既非 cursor 格式也非数组」分支静默 `Ok(Vec::new())`——写路径调用时等同 fail-open。

### H-8 `services-integrations/src/miniapp/storage_app_io.rs`
- `load_source_from_dirs` L101-133：L107-110 `index.html/style.css/ui.js/worker.js` 四个 `read_to_string(...).await.unwrap_or_default()`——任何读取错误（权限/IO/截断）都变空串；上游 `sync_from_fs`（assembly/core `miniapp/manager/mgr_lifecycle.rs:346-374`）会把这些「空源码」编译并经 `persist_sync_from_fs_result_for_app` 覆盖当前 app 全部 source。
- L112-119 `esm_deps.json`：存在但读/解析失败同样 fail-open。
- 对照：`load_npm_dependencies_from_package` L137-145 已是 fail-closed 好榜样（读失败 io Err、解析失败 parse Err）。

## 需求

### 1. H-7 修复

- `save_project_config` L213：删 `unwrap_or_default()`，改 `?` 传播。
- `load_project_configs` 拆分语义：
  - 读路径（`load_all_configs`/展示用）保持宽容现状不改（避免列表页因单条脏数据崩溃）。
  - 写路径（`save_project_config` 内部）使用严格变体：config_value 存在但形状不可识别（既非 cursor 对象也非数组）→ `MCPRuntimeError::configuration` Err（"refusing to overwrite project MCP configs with unrecognized existing format" 类信息）。
- `delete_server_config`（L228-）如存在同款 read-modify-write fail-open，一并核对修复（user 级 L233 `unwrap_or_else(json!({...}))` 仅限 key 缺失合法初始态；get_config_value 错误已 `?` 传播，确认即可）。

### 2. H-8 修复

`load_source_from_dirs` 改造：
- `index.html`（必需）：NotFound → `MiniAppStorageError::not_found`（sync 流程据此中止，不覆盖）；其他 IO 错误 → io Err。
- `style.css`/`ui.js`/`worker.js`：NotFound → 空串合法（可选文件）；其他 IO 错误 → io Err。
- `esm_deps.json`：exists 但读失败 → io Err；解析失败 → parse Err（不再静默空 Vec）。
- 先 grep `load_source`/`load_source_from_dirs` 全部调用方（storage.rs/mgr_lifecycle/runtime_facade 等），确认 not_found 语义变化对各流程的影响：sync 中止是目标行为；普通 load（编辑器打开 app）遇到缺 index.html 从「空白页」变「显式错误」——评估并在 report 披露。
- 空文件语义不变：真实存在且内容为空的文件仍返回空串。

### 3. 测试（必须）

H-7：
- config store 注入读错误 → save_project_config → Err 且 set_config_value 未被调用（spy/mock store 断言）。
- 现有值形状不可识别（如 `42`）→ save → Err 且未写。
- 正常 upsert（新增/覆盖同 id）行为不变。
H-8：
- index.html 缺失 → not_found Err；权限/IO 错误注入 → io Err。
- style.css 缺失 → Ok 且 css 为空串；worker.js 缺失 → Ok。
- esm_deps.json 损坏 JSON → parse Err。
- 真实空文件 → Ok 空串（不误报）。
- sync 层（如可行）损坏源目录 → persist 未被调用。

## 明确不做

- 不改 config store 接口/schema、不改 MiniAppSource 结构。
- 不动 sync_from_fs 的编译逻辑本身。
- H-9（desktop settings）归 Task 7。
- 不 git commit。

## 约束（逐字）

- Logs must be English-only, with no emojis.
- 严禁裸 `cargo fmt` 与 `cargo fmt -p <大crate>`；只允许 `cargo fmt -p northhing-services-integrations`（该 crate 已由 Task 4 格式化过，安全）。对 assembly/core 若必须格式化，只允许 `pnpm run fmt:rs` 前先确认它不会卷无关文件——不确定就不格式化，保持原样。
- 若需动 assembly/core 或 product-domains 的调用点（评估后确有必要），改动最小化并逐处披露。

## 验证命令

```
cargo check -p northhing-services-integrations --features product-full
cargo test -p northhing-services-integrations --features product-full mcp
cargo test -p northhing-services-integrations --features product-full miniapp
node scripts/check-core-boundaries.mjs
```
（若动了 assembly/core/product-domains，加跑对应 crate check/test）

## Report

写 `.superpowers/sdd/task-06-report.md`：改动 file:line、load_source 调用点影响评估、测试与输出、语义变化披露、状态。
