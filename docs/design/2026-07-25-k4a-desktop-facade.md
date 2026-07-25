# K4a 设计稿：Slint desktop 切 kernel facade

> 状态：**定稿 v1.0**（2026-07-25；judge-lc 两轮审判 FAIL→修复→APPROVED；用户五项拍板完毕，见 §10）。
> 上游：`docs/architecture/agent-kernel-northstar.md` §5（K4a）。**重定向说明**：原 K2 目标宿主 desktop-tauri 已于 `34a2397`（2026-07-23，"superseded by Slint"）删除，K2 验收对象不复存在；K4a 提升为 K 线下一主单。本文档同时是 northstar §5 K2 条目的修订依据（同 commit 更新）。
> 前置事实：facade（`contracts/kernel-api`，53 方法 + ToolPort）在 K1/K1a 冻结，N=44 → 上限 53，实发 53 已满额；`assembly/core/src/kernel_facade/` 为纯转发实现。desktop-tauri 删除后 facade 暂无宿主消费者。

## 1. 目标与非目标

**目标**：`src/apps/desktop`（Slint）全部 `northhing_core::` 引用清零，只经 `northhing-kernel-api` facade；行为不变（P5）；facade 获得真实消费者，K3 ROI 闸门与 K4b（cli+acp）获得实证输入。

**非目标**：不改 Slint UI 代码本身（只改 import/调用面）；不动 core 内部实现；不新增 facade 公开方法（53 已满额，任何新增必须走 northstar P2 评审——本设计按零新增设计，见 §4 缺口处置）；不动 frozen 面。

## 2. 调用面机械清单（2026-07-25 实测，grep 无遗漏）

> 方法论补注（judge-lc 复审）：主清单用 `rg -o "northhing_core::[A-Za-z_:]+"` 去重得出 53 符号；该模式对花括号分组 import（`use …::{A, B, C}`）有盲区，已用 `rg "northhing_core::" | rg "MCPServerConfig|MCPServerStatus"` 补测——补测命中项（⑩ 域 MCPServerConfig/MCPServerStatus）已列入下表，无其它遗漏。

19 文件、53 个 unique 符号引用，落 11 个域：

| 域 | 符号 | 涉及文件（引用数） |
|---|---|---|
| ① coordination（turn 生命周期） | `ConversationCoordinator` `DialogScheduler::new` `DialogSubmissionPolicy::for_source` `DialogSubmitOutcome::{Queued,Started}` `DialogTriggerSource::DesktopApi` `global_coordinator` `global_scheduler` `set_global_scheduler` | callbacks_lifecycle(27)、agentic_system(5)、sessions(19)、actor(2) |
| ② agentic::core（消息 DTO） | `Message` `MessageContent::{Text,Mixed,Multimodal,ToolResult}` `MessageRole::{System,User,Assistant,Tool}` `SessionConfig` `SessionState::Processing` `SessionSummary` | sessions、callbacks_lifecycle、event_bridge(5)、state(4) |
| ③ events | `events::router::EventSubscriber` | event_bridge |
| ④ message | `MessageMetadata` | sessions |
| ⑤ system（bootstrap） | `AgenticSystem` `init_agentic_system` | agentic_system |
| ⑥ skills | `skill_registry` `resolve_skill_default_enabled_for_mode` `set_user_mode_skill_state` | skills(2)、callbacks_lifecycle |
| ⑦ infrastructure::ai | `AIClient` `AIClientFactory` | agentic_system、provider_test(2) |
| ⑧ infrastructure::debug_log | `log_event` `COMP_{ACTOR_RUNTIME,APP_LIFECYCLE,MODE_ROUTING,SESSION_LIFECYCLE,SKILL_PANEL}` | log(2)、actor、main(4)、mod(2)、skills、inspector(3) 等 8 文件 |
| ⑨ service::config | `GlobalConfig` `AIModelConfig` `AuthConfig::ApiKey` `ModelCategory::GeneralChat` `get_global_config_service` `initialize_global_config` | main、agentic_system、settings/sync(4)、settings/tests(2)、callbacks_settings/provider(1)、create_ui(4)、inspector_model_status(2) |
| ⑩ service::mcp | `MCPService::new` `set_global_mcp_service` `global_mcp_service` `MCPServerConfig` `MCPServerStatus` | main、mcp_adapter(1) |
| ⑪ util | `NortHingResult` `AIConfig` | provider_test、w4_repro(5) 等 |

## 3. facade 覆盖对照

| 域 | facade 承接 | 判定 |
|---|---|---|
| ① coordination | `submit_turn` / `stop_turn` / `get_turn_state` / `subscribe_events`（`DialogSubmitOutcomeDto`、`TriggerSourceDto`、`SubmissionPolicyDto` 齐备） | ✅ 全覆盖；`global_coordinator`/`global_scheduler`/`set_global_scheduler` 属 bootstrap 接线，随 ⑤ 收编 |
| ② 消息 DTO | `MessageDto`/`MessageContentDto`/`MessageRoleDto`/`SessionConfigDto`/`SessionStateDto`/`SessionSummaryDto`/`MessageMetadataDto`（session.rs） | ✅ 全覆盖（逐字段映射进 T2 任务书验收） |
| ③ EventSubscriber | `subscribe_events(callback)`（KernelEventSubscriber 模式已在 kernel_facade/events.rs 成形） | ✅ |
| ④ MessageMetadata | `MessageMetadataDto` | ✅ |
| ⑤ system bootstrap | `init_core()`（lifecycle） | ✅ 概念覆盖，**但需核对**：desktop 当前 bootstrap 序列 = `initialize_global_config` → `init_agentic_system` → `DialogScheduler::new` + `set_global_scheduler` + `MCPService::new` + `set_global_mcp_service`（main.rs:62-87、agentic_system.rs:28-41）。facade `init_core` 是否已内化全序列（含 MCP service 全局注册）→ T0 核对项，缺则补在 core 侧 init_core 内（不动 facade 签名） |
| ⑥ skills | `list_skills`/`get_skill`/`set_skill_enabled`/`resolve_skill_default_enabled` 等 9 方法 | ✅（`set_user_mode_skill_state` → `set_skill_enabled(scope)` 映射进任务书） |
| ⑦ infra::ai（provider 连通性测试） | `test_provider(id)` / `test_provider_config(form)` | ⚠️ 语义核对项：provider_test.rs 当前直接用 `AIClient`/`AIClientFactory` 发真实请求；facade 两方法为 F2 立项面，desktop-tauri 时代已实装（B 线）。T0 核对返回粒度（错误分类/延迟）是否满足 Slint 设置页 UI 需求，不足则在 core 侧实现内补，不动 facade 签名 |
| ⑧ debug_log | **无对应面** | ❌ 见 §4-D1 |
| ⑨ service::config | `get_global_config` / `update_global_config` + providers/models CRUD（`list_model_configs`/`upsert_model_config`/`delete_model_config`/`set_default_provider`） | ✅（`sync_providers_to_core` 模式保留在 desktop 侧，改走 facade 写入；`initialize_global_config` 收编进 init_core） |
| ⑩ service::mcp | `list_mcp_servers`/`upsert_mcp_server`/`delete_mcp_server`/`get_mcp_status` | ⚠️ 见 §4-D2（mcp_adapter 处置） |
| ⑪ util | `KernelError` / DTO | ✅ |

## 4. 缺口与决策点

- **D1 debug_log（8 文件依赖，唯一真缺口）**：`log_event` + `COMP_*` 常量是宿主机侧同步日志工具，不是 kernel 命令——按 northstar P2 不该为它加 facade 方法（且同步 fn 不适合 async trait）。**方案 A（推荐）**：把 `infrastructure/debug_log` 下沉为 leaf 微 crate（`contracts/` 或独立 `crates/shared/debug-log`，零依赖、只含 log_event + 常量），desktop 与 core 同依赖它——不违反"宿主只见 facade"（它是工具库不是 kernel 面），与 leaf 干净现状一致。**方案 B**：desktop 内复制一份（约百行）→ 双源漂移，否决。**方案 C**：为日志开 facade 方法 → 违反 P2 量化约束，否决。
- **D2 mcp_adapter.rs（170 行）处置（用户拍板 2026-07-25：MCP 接口必须保留，是 agent 拓展能力必要部分）**：它是 `MCPService → McpCatalogPort` 的读侧桥（状态映射 + probe）。**方案 A'（采纳）**：**保留 `McpCatalogPort` 接口与 mcp_adapter 文件**，将其输入从 `MCPService` 换成 facade 读面（`list_mcp_servers` + `get_mcp_status` DTO → port DTO 纯映射）；8 态折叠留在 adapter 映射层（facade `MCPServerStatusDto` 的 kind+message 已够承载，`kernel_facade/settings.rs:80-93`）；adapter 变为零 core 依赖的纯映射层，MCP 能力本身（CRUD/状态/probe）经 facade 完整保留。消费侧从单次 list_servers 改为 list+per-id status（N+1 模式，Inspector 刷新延迟影响与 `inspector.rs:44-46` 配套方案列入 T0⑤ 核实）。~~方案 B：删桥~~（否决——用户拍板保留接口）；~~方案 C：保留桥直连 core~~（否决——双读面且 core 依赖不清零）。
- **D3 w4_repro.rs（bin，5 处引用）**：W4 运行时纪律 repro 线，且是 northstar §5 K3 验收工具（`w4_repro --mode=dual`）。**影响说明**：豁免 = 这个开发调试 bin 继续直连 core，desktop crate 对 `northhing-core` 的 Cargo 依赖因此无法彻底移除（只能把产品代码路径清零）；不影响产品面解耦与编译收益主体。**决策：豁免（编排者建议默认值，用户 2026-07-25 知悉影响后同意按默认执行）**，K4a 验收后若 K3 启动再随 K3 迁移。显式豁免并写入验收标准。
- **D4 settings/tests.rs**：测试文件引用 core 类型构造 fixture → 随 T4 同批改为 facade DTO，不单列。

## 5. Ticket 切分（tracer bullets，每片独立可验收、独立 commit）

| # | 范围 | 文件 | 验收 |
|---|---|---|---|
| **T0** | 核对单（不动码）：① init_core 是否内化 desktop bootstrap 全序列（config→agentic→scheduler→mcp 全局注册）——judge-lc 已预核 lifecycle.rs:91-136 序列完整，T0 复核确认即可 ② `test_provider_config` 返回粒度 vs 设置页需求 ③ `get_mcp_status` DTO vs McpCatalogPort 消费点 ④ debug_log 下沉目标 crate 选址 ⑤ **D2 深化（judge-lc 复审新增，按用户拍板 A' 修订）**：facade `get_mcp_status` 返回的 `MCPServerStatusDto`（kind+message）是否够承载 mcp_adapter 映射层所需的 8 态折叠信息（现 mcp_adapter.rs:86-100 折叠逻辑留在 adapter，不迁移）；Inspector 消费侧从单次 list_servers 改为 list+per-id status 的 N+1 调用模式对刷新延迟的影响评估 + `inspector.rs:44-46` 配套修改方案 | — | 核对报告；发现 core 侧缺陷则先出 core 侧小单补齐（不动 facade 签名） |
| **T1** | bootstrap 收编：main.rs + agent/agentic_system.rs 全序列塌缩为 `init_core()`；删 `set_global_scheduler`/`set_global_mcp_service`/`initialize_global_config` 直连 | main.rs(4)、agentic_system.rs(5) | `cargo check -p northhing` 绿；GUI 冒烟（启动→会话列表加载） |
| **T23**（T2+T3 合并，2026-07-25 重切分） | turn+session 数据流簇（最大片）：submit/stop/事件订阅/Message DTO 迁移 + sessions 域。合并理由：sessions.rs 的转换函数签名（`build_messages_model`/`refresh_sessions_ui` 等）被 callbacks_lifecycle/event_bridge/create_ui/mod 四处调用，T2/T3 分单并行会在共享签名上互撞（实测耦合发现） | callbacks_lifecycle(27)、event_bridge(5)、actor(2)、mod(2)、state(4)、create_ui(4)、sessions(19) | check 绿 + GUI 冒烟（发消息→流式 TextChunk→ToolCall→TurnState 完成/失败/取消；会话 CRUD/分支/消息加载）+ focused desktop 测试 |
| **T4** | settings/skills/mcp/inspector：config 读写、provider CRUD+test、skills 面板、MCP 面板（mcp_adapter 改纯映射层，D2-A'）、inspector 数据面（**前置依赖 T4p**：provider_test 迁移需 ProviderFormDto.provider_type） | settings/sync(4)、settings/tests(2)、provider(1)、provider_test(2)、skills(2)、inspector(3)、inspector_model_status(2)、mcp_adapter(改造) | check 绿 + 设置页全流程冒烟（改 provider→test→保存→skills 开关→MCP 状态） |
| **T5** | 清扫验收：D1 debug-log 微 crate 落地（`contracts/debug-log`，用户拍板新 crate）+ 全仓 `northhing_core::` 在 desktop/src 清零 grep 守卫（按 §6 豁免清单：kernel_facade 手柄、shutdown_mcp_servers、w4_repro）+ K0 编译指标对比 | log(2) 等 8 文件 | `rg "northhing_core::" src/apps/desktop/src` 仅剩豁免行；`cargo tree -p northhing-kernel-api` 对 `(rmcp|git2|axum|tower-http|reqwest)` 零命中；增量 check 实测对比 K0 基线（目标 min(30s, 基线×0.5)，未达成不阻塞但记录给 K3 闸门） |

依赖：T0 → T1 →（T23 ∥ T4p → T4，文件集不相交可并行；T23 与 T4 亦不相交）→ T5。每片 judge 验收后 commit；T23 是重灾区单独 judge。

## 6. 不变量与红线

- **P2 零新增**：53 方法已满额。任何"facade 缺方法"的发现 → 停步，按 northstar P2 三要素（提出人/覆盖缺口分析/合并可行性）走评审，严禁顺手加。
- **cargo 机制约束**（northstar §4）：kernel-api 不得引入 product-full feature 传染；facade 不得 re-export kernel 内部泛型/derive 类型；T5 含 `cargo tree` 零命中守卫。
- **P5 行为不变**：每片 `cargo check --workspace` + focused 测试全绿 + GUI 冒烟。
- **feature 口径**：desktop 依赖 kernel-api 只取默认 feature。**依赖保留口径（2026-07-25 修订，遵循 K2b `ae15d22` 先例）**：desktop **保留对 `northhing-core` 的 Cargo 依赖**——composition-root 手柄 `northhing_core::kernel_facade::kernel_facade()` 住在 core 内，且 desktop 是单 crate bin，删依赖不现实也无编译收益（K2b 验收已按此口径通过）；代码面口径 = 除显式豁免清单外不得出现 `northhing_core::` 引用。**豁免清单**：① `main.rs` 的 `kernel_facade()` 手柄调用 + `shutdown_mcp_servers`（facade 无 shutdown 生命周期方法，新增须走 P2 评审）② `src/bin/w4_repro.rs`（D3）。T5 grep 守卫按此豁免清单执行。
- **并发改动带测试**（家规④）；god-file 防线：callbacks_lifecycle.rs 迁移时若超 800 行警戒，顺手拆分记 commit message（家规①）。
- **UI 线程纪律**：事件订阅 callback 写 Slint 属性必须经 `slint::invoke_from_event_loop`（沿用 error_banners.rs 既有包装）。

## 7. 回退路径

facade 与旧路径在整个 K 线期间并存（northstar §5 K2 回退条款同样适用 K4a）：任一片验收失败 → 该 piece `git revert` 即可，desktop 恢复直连 core；facade 不删。T 片间独立 commit 保证粒度化回退。回退发生 → K 线暂停复盘，不阻塞其它线。

## 8. 风险

| 风险 | 等级 | 缓解 |
|---|---|---|
| T2 callbacks_lifecycle 27 处耦合迁移引入行为漂移（消息装配/取消语义） | 高 | T0 先产逐符号映射表进任务书；T2 单独 judge + GUI 全链路冒烟 + focused 测试 |
| facade DTO 与 core 类型字段级不齐（Message 附件/混合内容） | 中 | T0 核对项④扩展为 DTO 字段 diff 清单，缺字段在 facade DTO 侧补（DTO 是 facade 内部数据，加字段≠加方法，不占 P2 额度，但需 reviewer 确认） |
| debug-log 微 crate 选址引发层级争议 | 低 | T0 裁定；倾向 `contracts/` 下新 crate 或并入 `core-types`（行为轻，合规） |
| init_core 已内化序列与 desktop 顺序有微妙差异（如 MCP 注册时机） | 中 | T0 核对①；差异在 core 侧 init_core 内对齐，不动 facade 签名 |
| desktop Cargo.toml 去 core 依赖后 w4_repro 编译断 | 低 | D3 豁免 + T5 显式处理 |

## 9. 验收总表（K4a 完成判据）

1. `rg "northhing_core::" src/apps/desktop/src` 零命中（w4_repro.rs 豁免行除外）
2. `cargo check --workspace` 0 err + desktop focused 测试全绿
3. GUI 冒烟四链路：启动/会话、发消息全事件链、设置页全流程、MCP/skills 面板
4. `cargo tree -p northhing-kernel-api` 对 `(rmcp|git2|axum|tower-http|reqwest)` + `northhing-core` 零命中
5. 增量 check 实测 vs K0 基线记录（给 K3 ROI 闸门）
6. northstar §5 K2/K4a 条目修订 + surfaces.md 同步（同 commit，家规②）

## 10. 拍板记录（用户 2026-07-25 + judge-lc 两轮审判 APPROVED）

1. D1 debug_log → **`contracts/debug-log` 新微 crate**（用户选前者）
2. D2 mcp_adapter → **保留 McpCatalogPort 接口，adapter 改 facade 纯映射层**（用户拍板"MCP 必须保留接口"，方案 A'）
3. D3 w4_repro → **豁免**（影响已说明：仅该 dev bin 保留 core 直连，desktop 对 core 的 Cargo 依赖无法彻底移除；用户知悉后按默认执行）
4. 并行 → **尽可能并行**（T2∥T3∥T4，文件集已验证不相交）
5. facade DTO → **缺字段可补**（加字段不占 P2 方法额度，reviewer 确认即可）

设计稿状态：**定稿**。judge-lc 审判记录：一轮 FAIL（3 条：T2 误含 log.rs / T0 缺 D2 深化项 / grep 花括号盲区）→ 全部修复 → 二轮 APPROVED。

## 11. T0 核对结论（2026-07-25，编排者执行，全部关闭）

| # | 核对项 | 结论 |
|---|---|---|
| ① | init_core 内化 bootstrap 全序列 | ✅ 完整（judge-lc 预核 `kernel_facade/lifecycle.rs:91-136`：config→ai_factory→agentic→scheduler(notifier+injection)→mcp(new+global+bg init)→set_coordinator，与 desktop 现行序列逐步对应） |
| ② | test_provider* 返回粒度 | ✅ `ProviderTestResultDto{success,error}` 够用（error 已是 first_line，desktop 侧自行 120 字符截断 + last_verified_* 持久化留在 desktop AppSettings，正确分层）。**但发现 1 个 DTO 缺口**：`ProviderFormDto`（kernel-api/settings.rs:105-113）缺 provider wire format 字段——desktop `provider_to_ai_model_config` 用 `provider_wire_format(&p.provider_type)` 决定 AIModelConfig.provider（影响 AIClient 端点形态），而 facade `test_provider_config` 实现误用 `provider: form.provider_id`（kernel_facade/settings.rs:362）→ **新增 T4p 小单**：ProviderFormDto 加 `provider_type: Option<String>` 可选字段 + 实现侧改用它（拍板项 5 授权，DTO 加字段不占 P2 额度；serde optional 向后兼容） |
| ③ | get_mcp_status DTO vs McpCatalogPort | ✅ 覆盖（`MCPServerStatusDto` kind+message，`MCPServerStatusKind::Failed{message}` 承载 NeedsAuth reason；judge-lc 预核 settings.rs:80-93） |
| ④ | debug-log crate 选址 | ✅ `contracts/debug-log` 新微 crate（用户拍板）；T5 落地，同 commit 更新 `docs/status/surfaces.md`（家规②） |
| ⑤ | D2 深化（N+1 / 8 态折叠 / inspector 配套） | ✅ 8 态折叠信息够用（见③），折叠逻辑留 mcp_adapter 映射层（方案 A'）；N+1 配套方案：`build_mcp_status_string`（inspector.rs:17-47）改为 facade `list_mcp_servers` + `futures::future::join_all` 并发 per-id `get_mcp_status`，`render_status` 纯函数保留；MCP 实例数典型 <10，并发后延迟与现状同阶，无预算风险 |

**T0 产出**：T4p 小单立项（ProviderFormDto.provider_type）；其余核对项无需 core 侧补单。T1 可开工。
