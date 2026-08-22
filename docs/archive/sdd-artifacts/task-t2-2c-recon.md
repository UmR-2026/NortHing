# T2-2c 侦察报告：remote 栈整删（remote_connect + mobile-web + embedded relay + relay-server/relay-core）

- 日期：2026-08-19
- 范围：只侦察，未改任何文件
- 决策依据：T-01/TH-4/P-06 终值"无移动通道"（docs/status/decision-register.md:32,49,93）；roadmap T2-2 行 docs/architecture/backend-roadmap.md:118,167,259
- 重要前提：本侦察发现 **desktop 当前树里 remote 栈 UI 入口为零**（.rs/.slint 均无引用，见 Q4）——"需前端配合的 UI 面"对 remote 栈而言实际是空集；handoff 2026-08-18:33 的"前置②UI 入口摘除"主要指向 MiniApp 批，不阻塞 remote 栈。

## 规模实测

| 目标 | 实测 | roadmap 估值 |
|---|---|---|
| `src/crates/assembly/core/src/service/remote_connect/` | 48 个 .rs，10,443 行 | 11.5k |
| `src/crates/services/services-integrations/src/remote_connect/` | 14 个 .rs，4,081 行（roadmap 未单列，属同一栈） | — |
| services-integrations/tests 里 remote 专测 | 7 个文件 + tests/common/mod.rs 的 remote 段 | — |
| `src/mobile-web/src` | 40 文件，10,860 行（含 scss/assets 文本行；纯 ts/tsx 约 4.7k 与估值口径一致） | 4.7k |
| `src/apps/relay-server/` | 4 个 .rs，1,508 行 + static/ + deploy 脚本（Caddyfile/Dockerfile/deploy.sh 等 9 个非 rs 文件） | ≈4-5k（合计） |
| `src/crates/services/relay-core/` | 8 个 .rs，2,300 行 | 同上 |

---

## Q1. remote_connect 消费面

**结论：remote_connect 的全部对外 API 在两个模块之外没有任何 app 层调用点——desktop/cli/acp/server/installer 的 .rs 调用数为零；仅剩 core 内部的三处耦合（service_agent_runtime 适配器、product_runtime 注册、agentic 的 remote_file_delivery 提示词通路）需要外科手术。**

### 双模块结构
- core 侧 facade：`src/crates/assembly/core/src/service/remote_connect/mod.rs:11-49`（re-export services-integrations primitives + `RemoteConnectService`/`RemoteServer`）；gating：`src/crates/assembly/core/src/service/mod.rs:23-24`（`#[cfg(all(feature = "service-integrations", feature = "product-full"))]`）。
- 实现主体：`src/crates/services/services-integrations/src/remote_connect/mod.rs:31`（`lib.rs` 挂点 `#[cfg(feature = "remote-connect")] pub mod remote_connect;` — src/crates/services/services-integrations/src/lib.rs:31 附近）。

### 外部调用点核实（零引用证据）
grep 佐证：
```
rg -n "RemoteConnectService|RemoteConnectConfig|ConnectionMethod" --type rust src
  → 全部命中均在 crates/assembly/core/src/service/remote_connect/ 内部（connect.rs:133/38、bot_connection.rs 等）
rg -l -i "remote_connect|RemoteConnect|relay|pairing" --type rust src/apps/desktop
  → 仅 src/apps/desktop/src/app_state/settings/io/io_tests.rs:4（注释，提及 persistence_tests.rs 测试注入方案）
rg -n -i "remote_connect|relay|pairing" --type rust src/apps/cli src/apps/server
  → cli/management.rs:219 + cli/modes/chat/commands.rs:275 均为 SessionUsageReportRequest 的 `remote_connection_id: None`（SSH 工作区字段，恒 None，与 remote_connect 服务无关）
  → server/ai_relay.rs 是 AI API 本地代理（`northhing-server --ai-relay`），与 relay-server 无关
```
- ACP（src/crates/interfaces/acp）：全部 `remote_connection_id` 为 SSH 远程工作区语义（manager_connection_start.rs:99-106 走 `open_exec_channel` SSH），**不消费 remote_connect**。
- installer（northing-installer / northhing-Installer）：无 remote_connect 引用。
- kernel-api facade（src/crates/contracts/kernel-api/src/）：无 remote/relay/pair 任何 DTO（`rg -i "remote|relay|pair|mobile"` 仅 memory.rs:38 `repair` 误命中）。

### core 内部耦合点（删除时的手术清单）
1. `service_agent_runtime`（SAR，core 内）——remote host 适配器全部只为 remote_connect 服务：
   - sar_state.rs:11,27,89（RemoteExecutionDispatcher、RemoteCancelRuntimeHost、RemoteInteractionRuntimeHost impl）
   - sar_lifecycle.rs:5,119（`get_or_init_global_dispatcher().remove_tracker`）
   - sar_handler.rs:7,25 + CoreRemoteSessionTrackerHost/DialogRuntimeHost/PollRuntimeHost/WorkspaceFileRuntimeHost/WorkspaceRuntimeHost（:28,71,94,115,191）
   - sar_dispatch.rs:14-141（remote_dialog_host/remote_cancel_host/remote_session_host/remote_poll_host/remote_interaction_host、remote_image_context、load_remote_model_catalog、load_remote_chat_messages）
   - sar_types.rs:2,16-17,33-37,337-338
   - mod.rs:9-14（CoreRemote*Host re-export）+ :113-132（RemoteDialogSubmissionPolicy 测试）
   - **注意**：`CoreServiceAgentRuntime`（sar_dispatch.rs:15）本身被 agentic 各处广泛使用（coordinator.rs:48、session_control_tool.rs、cron/service.rs:14,65、bash_tool 等），**必须保留**，只摘 remote_* 方法族。
2. `product_runtime/runtime_services.rs:9,17,47-52`——注册 CoreRemoteWorkspaceRuntimeHost/CoreRemoteWorkspaceFileRuntimeHost 为 RemoteWorkspacePort/RemoteProjectionPort。
3. `agentic/remote_file_delivery.rs`（computer:// 链接提示词通路）——由 DialogTriggerSource::RemoteRelay|Bot 触发（remote_file_delivery.rs:7），消费者遍布 prompt_builder（mod.rs:100,155；system_prompt.rs:122,259）、coordinator.rs:28-29、dialog_turn/{compaction,session,thread_goal,workspace}.rs:33-34、execution/ 8 个文件:28、create_plan_tool.rs:5、tool_context_runtime/context_init.rs:214-215。**决策点**：随栈删（触发源消失后恒 false 的死通路）还是保留 contracts 变体——建议随栈删，触及 agentic 热点文件，单列小子批。
4. contracts 层残留（远程栈专用词汇）：
   - runtime-ports/src/remote.rs:109-140（RemoteWorkspaceRuntimeHost/RemoteInitialSyncRuntimeHost/RemoteWorkspaceFileRuntimeHost——注释明示 "Old remote-connect host compatibility trait"）+ RemoteWorkspacePort/RemoteProjectionPort（:120,138）
   - runtime-ports/src/session_workspace.rs:544 `RemoteConnectionPort`（marker trait）；消费方 runtime-services/src/lib.rs:45,132,193,225-227 + test_support.rs:156-166 + tests/runtime_services_contracts.rs:39-58
   - runtime-ports/src/port_core.rs:58,77 `RuntimeServiceCapability::RemoteConnection`
   - core-types/src/surface.rs:16 `SurfaceKind::Remote`、:27 `ThreadEnvironmentKind::RemoteConnect`（core-types AGENTS.md 要求"wire 兼容视为契约变更"——删除枚举变体是契约变更，需在 brief 里显式授权）
   - DialogTriggerSource::RemoteRelay/Bot：runtime-ports/src/agent/agent_dialog.rs:62、agent_facade_tests.rs 多处；core coordination subagent_ports.rs:113（`unwrap_or(Bot)` 默认值）、agentic/remote_file_delivery.rs:7。**变体在稳定 contracts 层，建议保留变体、只删 remote 栈代码**（或经契约评审后删）。

### 配置键 / 事件名 / feature flag
- **GlobalConfig（app.json）无 remote 字段**：`src/crates/assembly/core/src/service/config/app_shell.rs:41-70`（GlobalConfig 字段清单：app/theme/editor/terminal/workspace/ai/memory/project/mcp_servers/acp_clients/themes/font/version/last_modified）。
- bot 配置不入 GlobalConfig，单独持久化文件：bot/mod.rs:474-483（`~/.northhing/remote_connect_persistence.json` + legacy `bot_connections.json`）。
- 事件名：events/src/agentic.rs:78-83 的 `remote_connection_id`/`remote_ssh_host` 注释明示是 **Remote SSH** 语义，非本栈。
- feature flags：core `service-integrations` feature（Cargo.toml:208-225，含 :212 `dep:northhing-relay-core`、:223 `northhing-services-integrations/product-full`）；services-integrations `remote-connect` feature（Cargo.toml:100-121）+ `product-full` 列表 :157；core Cargo.toml:100 默认 dep 带 `features = ["remote-ssh"]`（不受影响）。
- RemoteConnectConfig 默认值（mod.rs:83-96）：lan_port 9700、northhing_server_url/web_app_url = `https://remote.openagentapp.com/relay`（外部托管中继 URL，删除后该托管服务与本仓库脱钩）。

---

## Q2. embedded relay 入口与 relay 依赖面

**结论：embedded relay 入口只有一处（embedded_relay.rs），且只被 remote_connect 自己调用；全仓没有任何进程 spawn relay-server 二进制；relay-core 的 crate 依赖方恰好两个（relay-server + assembly/core optional），摘除路径干净。**

### 摘除点清单
1. `src/crates/assembly/core/src/service/remote_connect/embedded_relay.rs:24` `start_embedded_relay(port, static_dir)`（:66 bind `0.0.0.0:{port}`，:15 `use northhing_relay_core::{build_relay_router, MemoryAssetStore, RoomManager}`——**这是 core 对 relay-core 的唯一 use**）。
2. 唯一调用方：`connect/relay_connection.rs:36,50`（Lan/Ngrok 两个 arm 起 embedded relay；:253-256 stop）。`connect.rs:141` 持有 handle 字段。
3. core Cargo.toml：:141 `northhing-relay-core = { path = ..., optional = true }`、:212 `"dep:northhing-relay-core"`（service-integrations feature 内）。
4. relay-core Cargo 依赖方证据：
   - `rg -n "relay-core|relay_core" -g Cargo.toml` → 仅 relay-server/Cargo.toml:17 与 assembly/core/Cargo.toml:141。
   - Cargo.lock：:5991（northhing-core 的 dep 列表内含 northhing-relay-core）、:6127（northhing-relay-server → northhing-relay-core）。无第三方。
5. relay-server 进程 spawn 证据（零）：`rg -n "relay-server|northhing-relay|relay_server" src scripts northing-installer` → 无 spawn/Command 调用（仅 relay-core lib.rs:169 注释提 ngrok CORS）。
6. relay-server 附带物（随 crate 整删）：Caddyfile、Dockerfile、docker-compose.yml、deploy.sh、restart.sh、start.sh、stop.sh、test_incremental_upload.py、README.md、static/（index.html + assets + homepage/i18n*.json）、tests/e2e_web_assets.rs。
7. 边界规则面见 Q5。

### relay-core 内部结构（供删除量评估）
lib.rs（build_relay_router + WebAssetStore trait + Memory/DiskAssetStore——DiskAssetStore 在 relay-server/src/lib.rs）、routes/{api.rs,websocket.rs}、relay/room.rs、validated.rs。relay-core/src/lib.rs:3-5 注释明示"Used by both the standalone relay-server binary and the embedded relay"——两个消费方都在删除范围内。

---

## Q3. mobile-web 面

**结论：mobile-web 位于 `src/mobile-web/`（pnpm workspace 成员），构建/脚本/CI/i18n/文档引用点全部列出如下；冻结标注 5 处。引用面虽多但全是"构建管道 + i18n 工程 + 文档"三类，无运行时 Rust 依赖（core 只通过 `mobile_web_dir` 配置字符串指向其产物目录，见 Q1）。**

### 位置与规模
- `src/mobile-web/`（package.json name=northhing-mobile-web v0.2.10，private；vite+react18+zustand；src/ 40 文件 ≈10.9k 行含样式）。
- 自身 AGENTS.md：`src/mobile-web/AGENTS.md`（35 行，整篇随目录删除）。

### 构建/脚本引用
- 根 package.json:12-15（dev:mobile-web / dev:mobile-web:host / preview:mobile-web / type-check:mobile-web）、:23（build:mobile-web）、:24（prepare:mobile-web → scripts/mobile-web-build.cjs）。
- pnpm-workspace.yaml:5（`- "src/mobile-web"` 成员）。
- scripts/mobile-web-build.cjs 整文件（:82-90 清理 target profile 下 stale mobile-web 产物；:104-131 pnpm install + build）。
- scripts/dev.cjs:22（require）+ :657-667（Step 3 "Build mobile-web (desktop only)"，`desktopMode` 门控，失败 exit 1）——**desktop dev 流程当前强制构建 mobile-web**，删除后 dev.cjs 少一步。
- CI：.github/workflows/ci.yml:44-50（"Create mobile-web dist directory (placeholder)"——v0.1.0 起只建空 dist 占位防 embed 路径报错；删后该 step 整段移除）。其余 workflows（cli-package/desktop-package/nightly/release-please）无 mobile-web/relay 引用。
- installer payload：northing-installer/scripts/build-installer.cjs:256-257（`runtimeDirs = ["resources","locales","swiftshader","mobile-web"]`——no-bundle 构建时把 mobile-web 产物作为 sibling 目录纳入）。tauri.conf.json / installer Cargo.toml 无引用。

### i18n 工程引用（frozen i18n，但删除必须同步）
- src/shared/i18n/contract/locales.json:11（surfaceDefaults["mobile-web"]）、:21-25（surfaceOrders）、:42-45（surfaces["mobile-web"].resourceRoot）；另有 :54-58 `relay-static-homepage` surface（resourceRoot=src/apps/relay-server/static/homepage）——**relay 删除时一并摘**。
- scripts/generate-i18n-contract.mjs:15（mobile-web generatedLocaleContract 路径）、:592（relay-static-homepage locale 序）。
- scripts/i18n-audit.mjs:28（mobileWebSourceDir）、:1127-1132、:1602-1607、:2286-2287（relay-static-homepage 审计块）等。
- scripts/i18n-contract.test.mjs:15、:181-203（messages/I18nProvider 断言）、:339、:360、:633、:660-698、:1160。
- 基线 JSON：scripts/i18n-governance-baseline.json:13、:49（bySurface 含 "mobile-web" 与 "relay-static-homepage" 键）；scripts/i18n-hardcoded-baseline.json:9（"mobile-web-source" 条目，附近另有 relay-static-homepage 条目）。

### 文档引用
- docs/status/surfaces.md:23（Frozen 行 "Mobile Web | src/mobile-web/ | 🧊 Frozen | PWA shell; re-pairing flow unguided, i18n has mojibake."）。
- 根 AGENTS.md:23（分层表 `src/mobile-web` *(frozen)*）、:60（`[frozen: mobile-web]` 命令标注）、:74（`[frozen: build:mobile-web]`）、:116（i18n 规则提及）、:181（v0.1.0 面基线）、:227（Verification 表 mobile-web 冻结行）。
- 根 AGENTS-CN.md:22、:56、:70、:86、:142、:172（对应 CN 镜像行）。
- docs/architecture/i18n.md:12、:36、:50；CONTRIBUTING.md:146；README.md:43（"Frozen-experimental: CLI, server, relay, mobile-web..."）；docs/tech-debt-cleanup-guide.md:12、:56、:115。
- docs/architecture/core-decomposition.md:56、:238（入口层枚举含 src/mobile-web）。

### 冻结状态标注位置（5 处）
surfaces.md:22-23 表格行；AGENTS.md:23/60/74/181/227 [frozen] 标注；tech-debt-ledger.md P1-4 条目 "active (mobile-web: frozen surface)"；tech-debt-cleanup-guide.md:12 冻结面清单；README.md:43。

---

## Q4. desktop Rust 侧边界

**结论：desktop（Slint）当前树对 remote/relay/pairing 的引用为零——.rs 仅 1 行注释，.slint 全空。remote 栈对 desktop 的唯一实际耦合是 Cargo feature 传递（northhing-core product-full → service-integrations → remote-connect + relay-core）和 dev.cjs/installer 的构建管道。不存在"等前端 session 摘 UI"的 remote 前置。**

### .rs 清单（全量）
```
rg -l -i "remote_connect|RemoteConnect|relay|pairing" --type rust src/apps/desktop
→ src/apps/desktop/src/app_state/settings/io/io_tests.rs:4  // 注释："injection scheme as Task 5 remote_connect/bot/persistence_tests.rs"
```
其余命中均为误命中：app_state/log.rs:87-92（`pairs` 局部变量）、flags.rs:33（注释里 "paired"）、callbacks_settings/refresh.rs:443-444（MCP 测试数据 id:"remote"）、app_state/sessions.rs:116（注释 "pair (D2b fix)"）。
- desktop/Cargo.toml：无直接 remote/relay dep；仅 `northhing-core features=["product-full"]`（第 14 行）传递拉入 remote 栈。删除后 desktop 无需改 Cargo.toml（core feature 内部消化），但 `cargo check -p northhing`（MSVC）是既定门禁。

### .slint 清单（全量）
```
rg -l -i "remote|relay|pair|mobile|phone" -g "*.slint" src/apps/desktop     → 零命中
rg -n "远程|手机|扫码|配对|移动|二维码" src/apps/desktop/src/ui              → 零命中
```
SettingsView.slint / AccessSettingsPanel.slint / main.slint / strings.slint 均无 remote 入口（strings.slint:40,42,99,100 的"测试连接"是 provider/MCP 连通性测试，无关）。
git 佐证：`git log --oneline -S "remote_connect" -- src/apps/desktop` 仅 9be74ec（settings 统一写入重构，非删除入口）——Slint desktop 从未接线 remote UI（v0.1.0 重写后不存在）。

### 划分
- 编排线可动：全部（io_tests.rs:4 注释顺手清；core feature 摘除；dev.cjs/mobile-web-build.cjs/build-installer.cjs 构建管道属 scripts 非 UI）。
- 归前端 session：remote 栈无。⚠️ 仅当前端 session 正在改 dev.cjs/dev 流程时需文件级对齐（dev.cjs 是三 session 潜在撞点，T2-2a 的 Minor M1/M3 也在 dev.cjs）。

---

## Q5. boundary 检查器与文档面

**结论：boundary 规则里 remote/relay 规则集中在 feature-rules.mjs、crate-layout.mjs、required-rules.mjs 三个文件 + self-test.mjs 锚点；删除批必须同 commit 同步（house rule 2），否则 checker 立刻红。**

### scripts/core-boundaries/
- rules/feature-rules.mjs:
  - :46-88 services-integrations optional dep 归属表——remote-connect 独占 owner 的 dep：hostname(:63)、image(:64)、mac_address(:65)、qrcode(:67)、rustls(:74)、rustls-native-certs(:75)、schannel(:76)、tokio-tungstenite(:83)、urlencoding(:85)、x25519-dalek(:88)、northhing-runtime-ports(:53)；共享 owner 的若干（aes-gcm/anyhow/base64/chrono/futures/rand/sha2/uuid 等含 remote-connect 项需逐个摘名）。**这些独占 dep 大概率随删除成为 orphan，需同批清 services-integrations/Cargo.toml。**
  - :153 `'remote-connect'` ∈ services-integrations requiredProductFullFeatures（:141-158 块）。
- rules/crate-layout.mjs:29 `{ crateName: 'relay-core', layer: 'services', path: 'src/crates/services/relay-core' }`。
- rules/source/required-rules.mjs（remote-connect 专区，行号带）：
  - :2554-2555 service/mod.rs facade cfg 规则（`pub mod remote_connect` 必须 gating——删除后规则删）。
  - :3823-4246 SAR 规则组（reason 含 "remote-connect and agent runtime port bindings"，:4011 CoreRemoteCancelRuntimeHost impl 锚等）——SAR 摘 remote 适配器后这些规则要改写为只锚 agent-runtime 部分。
  - :4246-5009 services-integrations remote_connect 全组规则（mod.rs/remote_session_state/remote_request_builders/remote_workspace_resolver/remote_cancel_handlers/remote_dialog_handlers/remote_file_io/remote_session_handlers/remote_session_response_builders）。
  - :4704-4890 测试名锚点组（pairing_qr_relay/session_wire_and_responses/model_catalog_tracker_poll/submission_images/dialog_cancel_contracts/file_transfer/command_runtime 的 remote_connect_* 测试名）。
  - :4893-4914 core remote_connect/mod.rs re-export 锚；:4920 remote_server.rs 锚；:5007-5009 command_router_session.rs 锚。
- self-test.mjs 锚点：:1734-1747、:1820、:2330、:2542、:3224-3298（含 "missing remote-connect remote_server boundary rule" 自检——规则删除时自检同步删）。
- 已知预先存在失败：self-test.mjs:2941 tool-contracts anchor（T2-2a Minor M5，非本批引入）。

### 文档面（需同 commit 同步）
- docs/status/surfaces.md:22（Relay Server 行）、:23（Mobile Web 行）、:52（relay-core capability 行）。
- 根 AGENTS.md:23、:60、:74、:116、:181、:227；AGENTS-CN.md:22、:56、:70、:86、:142、:172。
- src/crates/services/services-integrations/AGENTS.md:20-23（Remote-connect primitives 归属段 + "Remote workspace facts... belong in runtime-ports" 段）。
- src/crates/services/AGENTS.md:15 / AGENTS-CN.md:5,12（services-integrations 职责描述含 Remote Connect primitives）。
- src/crates/assembly/core/AGENTS.md:20 / AGENTS-CN.md:16（`src/service/` 枚举含 remote connect）。
- src/crates/interfaces/AGENTS.md:7、:21（入口层枚举含 src/mobile-web）。
- docs/architecture/core-decomposition.md:56、:106、:238、:318、:328、:355（remote/relay 词汇）；docs/architecture/agent-runtime-services-design.md:221、:235、:245-249、:739（RemoteConnectionPort/Remote mobile 行）。
- docs/tech-debt-cleanup-guide.md:12、:56、:96-99（relay 双实现 dedupe 冻结约定）、:115、:121、:166。
- README.md:43；CONTRIBUTING.md:146。
- docs/architecture/backend-roadmap.md:118、:151-152、:167、:259（T2-2 行/D-1 行——完成后标 done）。
- docs/status/decision-register.md:32、:49、:93（决策已生效，无需改，但可在 T2-2c 完成后补执行回链）。

---

## Q6. 配置与持久化迁移语义

**结论：app.json 侧零迁移风险——GlobalConfig 从未有过 remote 键，且 serde 无 deny_unknown_fields，历史遗留未知键静默忽略；用户机器上会遗留 3 类 remote 数据文件，需要处置说明（建议留盘 + 文档注明，不做运行时清理代码）。**

### app.json 加载行为
- GlobalConfig 定义：`src/crates/assembly/core/src/service/config/app_shell.rs:41-70`，`#[serde(default)]`，**无 `deny_unknown_fields`**（config/ 目录全目录 grep 零命中）。serde 默认行为 = 未知键静默忽略。
- 加载管线：mgr_load.rs:32-85 `load_existing_config`（from_value 失败走 :88-109 smart merge 兜底）——即便有意外键也不会启动失败。
- 结论：历史上 app.json 从未写过 remote 键（GlobalConfig 无此字段），即便用户手工塞入也无副作用。**无需迁移代码。**

### 遗留数据文件（用户机器）
| 文件/目录 | 来源 | 证据 |
|---|---|---|
| `~/.northhing/remote_connect_persistence.json`（+ legacy `~/.northhing/bot_connections.json`） | bot 连接持久化（单写者事务，H-6） | bot/mod.rs:474-483 |
| `~/.northhing/relay/api_key` | relay-server 首跑自动生成的 API key（P1-5） | relay-server/src/config.rs:56-67 |
| `/tmp/northhing-room-web`（默认 room_web_dir，env RELAY_ROOM_WEB_DIR 可改） | relay-server DiskAssetStore per-room mobile-web 文件 | relay-server/src/config.rs:147、:34 |
- mobile 配对身份（TrustedMobileIdentity）仅内存态，不落盘：connect/mobile_identity.rs:55-60。
- DeviceIdentity 为每次从 hostname+MAC 现算，不落盘：services-integrations/src/remote_connect/device.rs:19-36。
- 处置建议：发布说明/handoff 注明"以上文件为废弃 remote 栈残留，可手工删除"；**不在新产品代码里写清理逻辑**（YAGNI，且 northhing 无权默认清用户 home 目录）。

---

## Q7. 连带债关闭点

**结论：P1-4、P1-7 为 tech-debt-ledger active 条目，随删除翻 resolved；D-2 是 full-review 决策表行（非 ledger 条目），roadmap:118 已声明三者随 T2-2 关闭。另 T1-2（roadmap:152）已声明随栈删除关闭。**

| 条目 | 位置 | 当前状态 | 删除后动作 |
|---|---|---|---|
| P1-4 Mobile-web re-pairing 无引导 | docs/status/tech-debt-ledger.md:47-52 | active（mobile-web: frozen surface） | 翻 resolved（随 mobile-web 删除，house rule 2 同 commit） |
| P1-7 Embedded relay open mode 0.0.0.0 无鉴权 | tech-debt-ledger.md:68-73 | active（2026-08-04 注册，warn! 已加 embedded_relay.rs:45-49） | 翻 resolved（随 embedded_relay 删除） |
| D-2 weixin QR 登录接线还是删除 | docs/status/full-review-2026-08-16.md:231（决策表 "D-2 | weixin QR 登录：接线还是删除 | 无 IM 产品规划则删（448 行零调用点）"） | 待执行决策 | 随 bot/weixin* 删除落地；在 full-review/roadmap 标已执行。证据：bot/weixin_qr_login.rs 等在 core remote_connect 48 文件内 |
| T1-2 嵌入式 relay 鉴权 | backend-roadmap.md:152 | 已声明"随栈删除关闭" | 无需额外动作，T2-2c 完成后保持 |
| P1-5 relay-server 默认 0.0.0.0 无鉴权 | tech-debt-ledger.md:63-66 | 已 resolved（2026-08-04） | 无需动作（crate 整删后条目保留历史记录即可） |

---

## 建议删除批次划分

**总判断：remote 栈全链路删除在当前树里不需要前端 session 配合（Q4 零 UI 入口）。唯一跨 session 撞点风险是 dev.cjs（构建管道，非 UI）与 locales.json（i18n 契约，frozen），按文件锁协调即可。**

### 编排线可独立删（建议顺序，"先摘后删"）

- **子批 C1 — core 摘除（先摘）**：core `service/remote_connect/` 整目录（10,443 行）+ SAR remote 适配器（sar_state/sar_lifecycle/sar_handler/sar_dispatch/sar_types 的 remote_* 段，保留 CoreServiceAgentRuntime 本体）+ product_runtime/runtime_services.rs:47-52 注册摘除 + core Cargo.toml（:141、:212，及 service-integrations feature 里 relay-core 项）+ service/mod.rs:23-24。验证：`cargo check --workspace` + `cargo check -p northhing`。
- **子批 C2 — agentic 残留通路**：remote_file_delivery.rs + prompt_builder/coordinator/dialog_turn/execution 的 `computer://`/remote_file_delivery_channel 通路 + DialogTriggerSource::RemoteRelay/Bot 去留决策（建议 contracts 变体保留、core 死分支删除）。触及 agentic 热点文件多，独立小批降风险。⚠️ 需 brief 显式授权契约语义。
- **子批 C3 — services-integrations remote_connect**：src/remote_connect/（4,081 行）+ tests 7 文件 + tests/common/mod.rs remote 段 + Cargo.toml `remote-connect` feature(:100-121) 与 product-full 引用(:157) + orphan optional deps 清理。
- **子批 C4 — contracts 修剪**：runtime-ports/src/remote.rs 整文件、session_workspace.rs:544 RemoteConnectionPort、port_core.rs:58 RemoteConnection 变体、runtime-services registry 对应字段（lib.rs:45,132,193,225 + test_support + tests）、core-types surface.rs:16,27 变体。⚠️ 契约层删除=wire 变更，需 brief 授权；可与 C3 合并。
- **子批 C5 — relay 双 crate 整删（后删）**：src/apps/relay-server/（含 static/、deploy 脚本、Dockerfile 等全部附带物）+ src/crates/services/relay-core/ + 根 Cargo.toml workspace members(:6-7) + crate-layout.mjs:29 + Cargo.lock 同步。前置=C1 已摘 core 对 relay-core 的 dep。
- **子批 C6 — mobile-web + 构建管道**：src/mobile-web/ 整目录 + 根 package.json:12-15,23,24 + pnpm-workspace.yaml:5 + scripts/mobile-web-build.cjs 整删 + dev.cjs:22,657-667 摘除 + build-installer.cjs:256-257 + ci.yml:44-50 step 删除。
- **子批 C7 — i18n 契约面**：locales.json 的 mobile-web(:11,21-25,42-45) 与 relay-static-homepage(:54-58) surface 摘除 + generate-i18n-contract.mjs:15,592 + i18n-audit.mjs mobile/relay 块 + i18n-contract.test.mjs 相关断言 + 两个 baseline JSON 键。i18n engineering frozen——改动限"删除 surface 注册"，不触碰存活 surface 逻辑。
- **子批 C8 — boundary 规则 + 文档 + ledger 收口**（可与各子批同 commit 拆带，或最后统一）：feature-rules.mjs:46-88,153、required-rules.mjs remote 组、self-test.mjs 锚点、surfaces.md:22-23,52、AGENTS.md/CN 各行、services-integrations/AGENTS.md、core-decomposition.md、README/CONTRIBUTING、tech-debt-ledger P1-4/P1-7 翻 resolved、roadmap:118/167 标 done。

### 必须等前端 session 的
- **无（对 remote 栈）**。handoff 2026-08-18:33 的"前置②UI 入口摘除"经本侦察核实不适用于 remote 栈（desktop 无 remote UI）；该前置仅约束 MiniApp 批（T2-2 另一半）。
- 协调项（非阻塞）：dev.cjs 文件锁（前端 session 若改 dev 流程）；locales.json（若前端 session 动 i18n 契约）。

### 风险标注
1. C2/C4 触碰 contracts 与 agentic 热点——建议 reviewer 档位上调，且 brief 里写明"保留 SSH 语义 remote_connection_id 字段一律不动"（session_workspace.rs:18 等 SSH 字段与 remote 栈同名不同义，误删会炸 SSH 工作区）。
2. services-integrations orphan dep 清理（rustls/qrcode/x25519-dalek 等）要对照 feature-rules.mjs owner 表逐个核，sha2/tokio-util 等被 remote-ssh 共享的**不能删**。
3. `cargo check -p northhing`（MSVC 门禁，P2-15 教训）每子批必跑。