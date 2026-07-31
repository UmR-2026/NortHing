# Task 6 Review: MCP 项目配置 + MiniApp sync fail-closed（H-7/H-8）

**Scope:** `git diff a53711e..64c64dc`（5 files, +393/-8）

## 双判决

- **Spec verdict: PASS**
- **Quality verdict: PASS**
- **Overall: CLEAN**

## Spec 合规

### H-7

- `src/crates/services/services-integrations/src/mcp/config/service.rs:103-120` 的宽容读路径保持不变；store 错误仍由内部 `?` 返回给 `load_all_configs`，再由展示聚合路径降级为空源。
- `src/crates/services/services-integrations/src/mcp/config/service.rs:128-148` 的严格变体与宽容版在可识别 cursor 对象、legacy 数组、缺 key 三个成功分支保持相同解析/位置语义；唯一有意差异是不可识别现有形状由空 Vec 改为 Configuration Err。store 错误同样经 `?` 传播。
- `src/crates/services/services-integrations/src/mcp/config/service.rs:239-252` 的项目 save 使用严格变体并传播错误，不再从空基线覆盖。
- `src/crates/services/services-integrations/tests/config_and_server_lifecycle.rs:95-119` 对 store 读错误使用 recording spy，明确断言 `set_config_value` 零调用。
- `src/crates/services/services-integrations/tests/config_and_server_lifecycle.rs:122-151` 对不可识别形状断言 Configuration Err 且原值 `42` 保持不变；`src/crates/services/services-integrations/tests/config_and_server_lifecycle.rs:154-205` 覆盖新增和同 id 覆盖。
- `src/crates/services/services-integrations/src/mcp/config/service.rs:255-287` 的 delete 路径先传播 store 错误；仅 key 缺失使用合法空对象，形状不可识别及 id 缺失均在写入前返回 NotFound。

### H-8

- `src/crates/services/services-integrations/src/miniapp/storage_app_io.rs:107-115`：必需 `index.html` 的 NotFound → NotFound，其他读取错误 → Io。
- `src/crates/services/services-integrations/src/miniapp/storage_app_io.rs:115-117,317-324`：可选 CSS/UI/worker 的 NotFound → 空串，其他读取错误 → Io。基于 Rust `ErrorKind` 分类而非平台 raw code，Windows 文件/路径缺失映射到 NotFound；其他 Windows 错误不会被误吞。
- `src/crates/services/services-integrations/src/miniapp/storage_app_io.rs:119-127`：`esm_deps.json` 存在但读取失败返回 Io，损坏 JSON 返回 Deserialization；不再回落空 Vec。
- `src/crates/services/services-integrations/src/miniapp/storage_tests.rs:417-568` 覆盖 brief §3 的必需/可选/IO/损坏 JSON/真实空文件清单；缺失可选文件测试同时断言 css、ui_js、worker_js。
- `src/crates/assembly/core/src/miniapp/manager/mod.rs:256-299` 的 sync 测试在移除 index 后断言 sync Err、meta version 未变、version snapshots 为空；结合 `src/crates/assembly/core/src/miniapp/manager/mgr_lifecycle.rs:362-374` 的顺序，这能证明读取失败发生在 compile/persist 调用之前，持久化副作用未执行。

## 调用点影响核查

- 全仓 grep 的实际生产调用链与 report 表一致：普通 app load、core/service storage port、sync/recompile、draft load；未发现遗漏的 concrete `load_source_only` / `load_source_from_dirs` 调用点。
- `src/crates/services/services-integrations/src/miniapp/storage_port.rs:29-30,135-144` 正确保留 NotFound/Deserialization/Io kind。
- `src/crates/assembly/core/src/miniapp/storage.rs:44-45,317-325` 正确映射为 `NortHingError::NotFound`/Deserialization/Io；`src/crates/assembly/core/src/miniapp/storage.rs:211-212,341-352` 再映射到 port kind 时仍保持语义。
- `src/crates/assembly/core/src/miniapp/manager/mgr_lifecycle.rs:348-374` 的 recompile/sync 均在 persist 前传播损坏源错误，符合 fail-closed 目标。

## Triage

- `src/crates/services/services-integrations/src/mcp/config/service.rs:212-237` 的 `save_user_config` 对不可识别现有用户级形状仍会回落空 map。H-7 审计项和 Task 6 brief 明确针对 `project.mcp_servers`，因此不构成本任务 finding；建议作为独立安全加固项进入终审 triage。

## 代码质量与约束

- 未新增日志；新增错误文本和测试信息均为英文，无 emoji，满足：`Logs must be English-only, with no emojis.`
- `git diff --check a53711e..64c64dc` 干净。
- 新增 helper 边界窄，失败分类明确；无 unsafe、无新增 panic/unchecked indexing。

## Fresh verification

- `cargo test -p northhing-services-integrations --features product-full mcp`：通过；目标 MCP integration 11/11，新测试均通过。
- `cargo test -p northhing-services-integrations --features product-full miniapp`：通过；目标 MiniApp unit 29/29，新测试均通过。
- `cargo test -p northhing-core --features product-full sync_from_fs`：通过；目标测试 1/1。
- `node scripts/check-core-boundaries.mjs`：通过。

## Findings

CLEAN
