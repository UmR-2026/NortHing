SPEC: PASS
QUALITY: PASS

# Wave1 分支终审报告（judge-m3 终审视角，2026-08-06）

> 存档说明：终审子代理以只读角色返回本报告全文，由编排者落盘（内容逐字转录，仅修正个别断字排版）。
> 范围：merge-base `41695f5` → HEAD `e6be249`（B1-B4 + build fix + 证据链）。

## 裁决: PASS | PASS

## 终审专属检查点逐条核对

### 1. 跨任务锁交互（核心关注点）

锁拓扑完整盘点（独立调研，非采信一审）：

| 锁 | 类型 | 持有位置 | 用途范围 |
|---|---|---|---|
| `SETTINGS_WRITE_LOCK` | `tokio::Mutex<()>`（desktop） | `io.rs:17` | 仅 `~/.northhing/config/app.json` 文件 IO + keyring 写 |
| `MCPConfigService::write_lock` | `tokio::Mutex<()>`（services-integrations） | `service.rs:33` | 单实例 3 条 MCP RMW 路径串行化 |
| `ConfigService::manager` | `tokio::RwLock<ConfigManager>`（core） | `service.rs:63` | 全局 config 服务的磁盘读写 |
| `GLOBAL_CONFIG_SERVICE` | `OnceLock<Arc<RwLock<Option<Arc<ConfigService>>>>>` | `global.rs:15` | 包装 ConfigService Arc 的小 RwLock |
| `INIT_MUTEX` | `std::sync::OnceLock<tokio::sync::Mutex<()>>` | `global.rs:30` | `GlobalConfigManager::initialize` 双检锁 |
| `AI_CLIENT_FACTORY_INIT_MUTEX` | `std::sync::OnceLock<tokio::sync::Mutex<()>>` | `client_factory.rs:232` | `AIClientFactory::initialize_global` 双检锁 |

关键调用链（独立追溯）：

- **B1 write_lock → core RwLock**：`save_user_config` 持 `write_lock` 期间，`CoreMCPConfigStore::get_config_value` → `ConfigService::config()` 拿 `manager.read()`；后续 `set_config_value` → `ConfigService::set_config()` 拿 `manager.write()` + 可能再触 `reconcile_models`（`service.rs:104`）二次读写 `manager`。整条路径**没有反向环**：核心路径不会回调到 `write_lock`。
- **B3 SETTINGS_WRITE_LOCK → core RwLock**：`update_app_settings`/`load_app_settings` 的 critical section 仅含同步闭包 + 文件 IO + keyring IO（`io.rs:147-171`）。所有 `sync_providers_to_core` 调用均在锁外（`provider.rs:233`、`misc.rs:68`、`create_ui.rs:149`），零重叠。
- **SETTINGS_WRITE_LOCK ↔ MCP write_lock**：桌面 settings 模块不调用 `MCPConfigService::save_*`（grep 证实 desktop 内 0 处 `save_server_config`/`delete_server_config`），desktop `AppSettings.mcp_servers` 字段当前 `#[allow(dead_code)]`（`mod.rs:222-223`）。两条路径**完全隔离**。
- **B4 init mutex 与 core config 锁**：`initialize_global` 临界区仅做 `get_global_config_service().await`（`client_factory.rs:302`，短暂 `service_wrapper.read()`）+ 构造 factory，然后 `set`（不再触其他锁）。无嵌套持锁、无环。

**结论**：无死锁、无锁序反转、无嵌套持锁 await。MCP write_lock 会序列化所有 MCP RMW 操作（含每次 `reconcile_models` 的二次 read+write），存在轻微护航效应（lock convoy），但 MCP 写入频率极低（用户配置 UI 操作），可接受。✓

### 2. B3 行为偏离围栏

用户 2026-08-05 拍板"方案 a（锁住公共 load）"，三处声明一致：

- **commit message**（`755a503` body）：明确写 "User-approved 2026-08-05 deviation from plan literal 'load pure-read' (written pre-C3): keyring migration stays on load path, C3 security posture unchanged (task-b3-brief.md section 0)"。
- **tech-debt-followups.md FU-3 段**（line 30-36）：明写"偏离计划字面（计划写于 P1-C3 前，要求 'load 纯读'）：用户 2026-08-05 拍板方案 a（锁住公共 load）—— keyring 迁移是 C3 有意置于 load 路径的 fail-closed 安全行为（明文 key 必须 load 时立即迁出磁盘），行为与 C3 安全姿态零变化"。
- **progress.md B3 ledger**（line 47）：明写"用户拍板 (a) 锁住公共 load（计划字面 'load 纯读' 写于 C3 前，偏离已在 commit message + brief §0 声明）"。

C3 安全姿态零漂移验证（独立取证）：`keyring_migrate_providers` 函数本体（`io.rs:103-137`）逐字符与 C3 一致；只是其调用点从裸 `load_app_settings_at` 改为 `load_app_settings_locked`（外层加锁）。fail-closed 行为（`io.rs:117-120` 失败回填明文并 `Err`）完全保留。✓

### 3. `b0bfe43` build fix 安全语义

- **Cargo.toml diff**：单行 `keyring` feature `windows-native-keyring-store` → `v1`。新特征 `v1` 拉入各平台 native 后端，均仅作 backend 启用，无 API 变化。
- **keyring.rs 3 处修改**：`set_secret` → `set_password`、`get_secret` → `get_password`（keyring 4.x v1 feature 暴露的 v1 兼容 API 名）、`Lazy::new(ProductionKeyring)` → `Lazy::new(|| ProductionKeyring)`（单元结构体值非 FnOnce，闭包是显式形式）。**0 行为变化**。
- **provider_test.rs**：import path 从 `keyring::` 子模块改走 `settings::` 顶层（`pub use keyring::*` 在 `settings/mod.rs:49`），仅 import 路径调整。
- **Cargo.lock +4 包**（独立核验）：`apple-native-keyring-store 1.0.1`（macOS backend，keyring v1 依赖）+ `zbus-secret-service-keyring-store 1.0.0`（Linux backend）+ `secret-service 5.1.0`（前者依赖）+ `num 0.4.3`（secret-service 依赖）。**全部为 v1 链式依赖**，无既有条目版本漂移。✓
- **历史凭据兼容性**：Windows 后端仍解析到同一 `windows-native-keyring-store`；task-b3-report.md 与 task-b3-review.md 已逐行核定 UTF-16LE 编码细节无兼容影响（磁盘上不可能存在本应用写过的凭据），本审抽查通过。

### 4. 测试有效性总账

**B1（FU-1）**：4 个读错 fail-closed 测试 + 3 个并发 RMW 测试（`config_and_server_lifecycle.rs:154-410`），用 `RecordingFailingGetMCPConfigStore` / `InMemoryMCPConfigStore` 注入 IO 错误/未识别格式/竞态。并发测试用 `#[tokio::test(flavor = "multi_thread", worker_threads = 4)]`，spawn 10 个 saver，断言"全部 10 个服务器必须保留"，无锁基线下会丢条目（一审 r2 已实证）。✓

**B2（FU-2）**：2 个真实进程测试（`manager.rs:750-787`）：多语言插件全停服 + 未注册插件不影响其他服务。用 `cmd.exe /c exit 0`（Windows）/`sh -c exit 0`（非 Windows）即退 dummy，规避 shutdown 60s 硬超时。`register_plugin_internal` 是 `pub(crate)`（同模块测试可见），非越权。✓

**B3（FU-3/FU-4）**：`concurrent_loads_and_updates_preserve_all_writes`（`io_tests.rs:402-478`）：并发 loads × updates，duplicates 触发 dedup 写 + plaintext 触发 keyring 迁移写；30s `tokio::time::timeout` 死锁守卫。顺带修 pre-existing 的 `&MockKeyring → Arc<MockKeyring>`（`tokio::spawn` 'static 要求）。✓

**B4（FU-5）**：测试对象争议 —— 测的是 `init_once_with` helper（`client_factory.rs:499-588`），不直接测 `initialize_global` 本体。理由（task-b4-report.md §3 + review.md）：方案 A 不可 hermetic（`GLOBAL_AI_CLIENT_FACTORY` 是进程级 OnceLock，与 lib 测试二进制共享，初始化会让 `subagent_ports` spawned task 发起真实 LLM 调用）。**终审接受此等价替代**：helper 是 critical section 的逐字抽取（`client_factory.rs:240-268`），`initialize_global` 的套用方式在源码可见（`:287-330`）且与 `GlobalConfigManager::initialize`（`global.rs:100-157`）同构。2 个测试覆盖 8 并发 build 恰一次（multi_thread flavor）+ build 失败后 cell 保持空且重试成功。✓

### 5. Minor triage 裁定

| ID | 内容 | 裁定 | 理由 |
|---|---|---|---|
| **B1-M1** | `ConfigManager::save_config` 非原子写（`mgr_load.rs:158` 裸 `fs::write`） | **登记独立债项** | pre-existing；本轮范围仅 FU-1。改 `json_store::write_atomic` 涉及 core config 写路径改造，应独立任务（Wave 2/3） |
| **B1-M2** | `tech-debt-followups.md:12` FU-1 注记 "integrations +4" 应为累计 "+7" | **合并前补** | 描述性滞后；状态行正确、doc sync 硬规则已满足。简单文字增补，不需重审 |
| **B2-M1** | `stop_server` 恒 Ok 使 uninstall 新 warn 分支不可达（`manager.rs:201-213`） | **忽略（pre-existing）** | 非本轮引入；不影响 uninstall 行为 |
| **B2-M2** | commit body 未记 `plugin_ids → languages` 改名 | **忽略** | 命名修改是顺手清，与功能正确性无关 |
| **B2-M3** | 测试两 dummy 共用 plugin id（`manager.rs:756`） | **忽略** | id 仅作元数据；`processes` map 以 language 为键，逻辑不受影响 |
| **B3-M1** | `callbacks_settings/mod.rs:29` 注释仍引用已删 `save_app_settings` | **合并前修（一行注释）** | 文档卫生；FU-4 删除 wrapper 后已过期。不需重审 |
| **B4-M1** | report 自述行数 592 实为 589 | **忽略（cosmetic）** | 不影响任何断言 |
| **B4-M2** | 并发测试 `cell.get()` 断言冗余（`client_factory.rs:537`） | **忽略** | `build_count == 1` 已蕴含；冗余但无害 |
| **B4-M3** | `init_once_with` 若 `global.rs` 复用可上抽 util | **登记债项（low）** | 当前 DRY 阈值未到；记 Wave 2+ 候选 |
| 观察项 | `keyring.rs` 5 个 C3 前 test-only dead-code warning | **记台账** | pre-existing；不影响功能 |
| 观察项 | Windows keyring UTF-16LE 编码细节 | **记台账** | 无历史凭据故无兼容影响；供后续 keyring 任务参考 |
| 观察项 | `LspManager::uninstall_plugin` 全仓暂无生产调用方 | **记债项（low）** | pre-existing 死路径；B2 修复正确，实际风险面为零 |
| 观察项 | **P1-C3 合入后 desktop 从未编译** | **必须登记独立债项 + 呈报用户** | 过程性缺陷，非本轮引入，但终审发现即有义务上报。建议 handoff 单列"流程改进建议"段，并登记 tech-debt 项（如 `FU-PROC-1`："desktop `cargo check` 通过才可 merge to main"） |

### 6. 文档/台账一致性

- `tech-debt-followups.md` FU-1..FU-5 全 `resolved`，每项附 task 来源与修复说明。FU-1 注记 "+4" 描述滞后（见 B1-M2），硬规则（同 commit 翻转）满足。✓
- `progress.md` 四条 ledger 行数字与实测一致：B1 integrations +7 / core +2；B2 core lib 1139；B3 desktop 118/118；B4 core lib 1141（1139 + 2）。✓
- 未改 crate 结构（均在原 crate 内修改，无新增/移动） → `docs/status/surfaces.md` 无需同步（家规 2）。✓

### 7. 合并前风险清单

1. **回归命令**（最小集）：
   - `cargo check --workspace` 被上游 `embed-resource 3.0.11` 阻断（不动，交 CI）
   - `cargo test -p northhing-core --features product-full --lib` → 基线 1141
   - `cargo test -p northhing-services-integrations --features product-full` → 基线 212
   - `cargo check -p northhing` + `cargo test -p northhing --lib` → 118/118（仅 `b0bfe43` 之后可编译）
2. **呈报用户**：P1-C3 过程性缺陷（desktop 自 C3 合入后从未编译，本轮 `b0bfe43` 事后修复）—— 是否登记独立债项 / 流程改进项，请用户拍板。
3. **建议合并前修补**（不阻塞）：`tech-debt-followups.md:12` 补 "+7"（B1-M2）；`callbacks_settings/mod.rs:29` 注释去掉 `save_app_settings` 引用（B3-M1）。

## 范围外改动

无。`git show --stat` 抽查 6 个代码 commit，每个仅触及目标文件 + tech-debt doc。

## 副作用风险

| 级别 | 项 | 说明 |
|---|---|---|
| 中 | lock convoy | MCP `write_lock` 序列化 RMW，期间含 `reconcile_models` 二次 `manager.read/write`。高频写入时可能拖累性能；用户级 MCP 写入低频，可接受 |
| 低 | `Cargo.lock` +4 包 | 跨平台 backend 条目；Windows 构建 target-gate 不编译 |
| 低 | keyring test-only dead-code warning | pre-existing，无功能影响 |
| 低 | P1-C3 未编译即合入 | 过程性缺陷，`b0bfe43` 已修，但缺预防机制 —— 见 §7.2 |

## Findings

### Critical
无。

### Important
无。**所有跨任务锁交互风险已核实为安全。**

### Minor
本审未新发现 Minor 项；一审 + brief 已枚举的 Minor 项已在 §5 逐条裁定。

## 修复指引

无需修复即可合并。若用户同意 §7.2，建议合并前补两处文档修（§5 B1-M2 / B3-M1）。

---

## 编排者补充记录（2026-08-06）

- 终审模型：`judge-m3`（用户指定；ark/volcengine/qwen 线本时段均不可用或额度紧）。该模型此前只审过 B4 单任务，B1-B3 一审为 qwen 线，终审已按要求独立复核。
- 待用户拍板项：P1-C3 过程性缺陷是否登记独立债项 / 流程改进项（终审 §7.2）。
