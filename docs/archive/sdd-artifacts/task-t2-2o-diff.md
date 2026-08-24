diff --git a/AGENTS-CN.md b/AGENTS-CN.md
index c44d471..4a403a9 100644
--- a/AGENTS-CN.md
+++ b/AGENTS-CN.md
@@ -22,7 +22,7 @@ northhing 是一个 Rust 工作区加上 React 前端的组合。
 | 1 | 接口与入口 | `src/apps/*`、`src/web-ui`、`northhing-Installer`、`tests/e2e`、`src/crates/interfaces` | 产品宿主、命令、UI 入口、协议接口以及跨表面测试 | desktop、CLI、server、Web UI、installer、E2E、`acp` | 最近本地 `AGENTS.md`；[interfaces](src/crates/interfaces/AGENTS.md) |
 | 2 | 产品装配 | `src/crates/assembly` | 兼容性导出、产品能力选择、product-full 装配以及适配器/服务注册 | `core`、`product-capabilities` | [AGENTS.md](src/crates/assembly/AGENTS.md) |
 | 3 | 适配器 | `src/crates/adapters` | AI 协议适配器与外部提供方翻译 | `ai-adapters` | [AGENTS.md](src/crates/adapters/AGENTS.md) |
-| 4 | 服务 | `src/crates/services` | 可复用的 OS、文件系统、终端、MCP、远程、git、watch、进程、会话持久化原语、MiniApp 运行时 IO 以及网络实现 | `services-core`、`services-integrations`、`terminal` | [AGENTS.md](src/crates/services/AGENTS.md) |
+| 4 | 服务 | `src/crates/services` | 可复用的 OS、文件系统、终端、MCP、远程、git、watch、进程、会话持久化原语以及网络实现 | `services-core`、`services-integrations`、`terminal` | [AGENTS.md](src/crates/services/AGENTS.md) |
 | 5 | 执行原语 | `src/crates/execution` | 可移植的 agent、stream、DeepReview 策略/报告、typed-service、tool-contract 以及 tool-execution 构件 | `agent-runtime`、`agent-stream`、`tool-contracts`、`runtime-services`、`tool-execution` | [AGENTS.md](src/crates/execution/AGENTS.md) |
 | 6 | 稳定契约与产品域 | `src/crates/contracts` | 共享 DTO、事件形态、运行时端口以及产品域契约/策略 | `core-types`、`events`、`runtime-ports`、`product-domains` | [AGENTS.md](src/crates/contracts/AGENTS.md) |
 
@@ -31,7 +31,7 @@ northhing 是一个 Rust 工作区加上 React 前端的组合。
 - 接口和应用入口暴露选定的产品行为；可复用行为下移。
 - 装配层连接下层并选择产品能力事实；不得实现具体的适配器、OS 或服务细节。
 - 适配器翻译协议和外部系统；不应拥有产品能力选择或可复用 OS 服务行为。
-- 服务实现可复用的具体 OS、进程、终端、MCP、远程、git、文件系统以及 MiniApp 运行时 IO 能力。
+- 服务实现可复用的具体 OS、进程、终端、MCP、远程、git 以及文件系统能力。
 - 执行 crate 是可移植的运行时构件，而不是宿主特定或交付配置的所有者。
 - 契约保持轻行为，不得向上依赖。
 
@@ -134,10 +134,10 @@ await api.invoke('your_command', { request: { ... } });
 - **桌面包名是 `northhing`（Slint）**，不是 `northhing-desktop`。agent-dispatch flags：只剩 `USE_LIGHTWEIGHT_ACTOR = true`；Phase 3 IPC（`USE_ONESHOT_DISPATCHER` / `USE_ACTOR_IPC` / `USE_DISPATCHER_IPC` + IpcSpawnAdapter）已于 2026-07-20 descope 并删除。
 - **配置单一事实源 = core `GlobalConfig`**（`dirs::config_dir()/northhing/config/app.json`）。桌面 `AppSettings` 仍是 UI owner，经 `sync_providers_to_core` 适配推送到 core（见 `95e29ba`）。禁止再出现第二个运行时可读的配置文件。
 - **UI 线程纪律**：非事件循环线程写 Slint 属性会被静默丢弃。所有此类写入必须走 `slint::invoke_from_event_loop`（`error_banners.rs` 的 helper 已封装，直接复用，见 `ad349f9`）。
-- **Shell 安全**：`guard_command_execution` 已接入 Bash/ExecCommand 的 `validate_input` 路径并写审计日志（见 `9a1575d`）。新增 shell 类工具必须同样接入；MiniApp string 模式命令含 shell 元字符一律拒绝。
+- **Shell 安全**：`guard_command_execution` 已接入 Bash/ExecCommand 的 `validate_input` 路径并写审计日志（见 `9a1575d`）。新增 shell 类工具必须同样接入。
 - **项目运行时 slug 恒带路径哈希**（CJK 路径不得冲突，见 `c7e7218`）。
 - **安装器工具链**：`northing-installer` `[lib] crate-type = ["rlib"]`（cdylib/staticlib 会突破 GNU ld 导出 ordinal 上限）；`embed-resource` pin 3.0.5（3.0.11 在 rustc 1.96 MSVC 下编译失败）。桌面构建用 MSVC；仓库目录 override 是 GNU 且 `cargo +toolchain` 不可用——用 `rustup run <tc> cargo`。
-- **v0.1.0 面基线**：发货面仅 Slint 桌面 + `northing-installer`；server / MiniApp UI / SDLC harness 为冻结-实验面。能力 crates（tools/MCP/search/terminal/git/ssh）是 agent 工具箱，保持激活。见 `docs/tech-debt-cleanup-guide.md` §0。
+- **v0.1.0 面基线**：发货面仅 Slint 桌面 + `northing-installer`；server / SDLC harness 为冻结-实验面。能力 crates（tools/MCP/search/terminal/git/ssh）是 agent 工具箱，保持激活。见 `docs/tech-debt-cleanup-guide.md` §0。
 
 ## 架构
 
diff --git a/AGENTS.md b/AGENTS.md
index fa8a8f5..3a09826 100644
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -23,7 +23,7 @@ crate dependencies inside each layer to the smallest set needed.
 | 1 | Interfaces and entrypoints | `src/apps/*`, `northing-installer`, `tests/e2e`, `src/crates/interfaces` | Product hosts, commands, UI entrypoints, protocol interfaces, and cross-surface tests | desktop, CLI, server, installer, E2E, `acp` | nearest local `AGENTS.md`; [interfaces](src/crates/interfaces/AGENTS.md) |
 | 2 | Product assembly | `src/crates/assembly` | Compatibility exports, product capability selection, product-full wiring, and adapter/service registration | `core`, `product-capabilities` | [AGENTS.md](src/crates/assembly/AGENTS.md) |
 | 3 | Adapters | `src/crates/adapters` | AI protocol adapters and external-provider translation | `ai-adapters` | [AGENTS.md](src/crates/adapters/AGENTS.md) |
-| 4 | Services | `src/crates/services` | Reusable OS, filesystem, terminal, MCP, remote, git, watch, process, session persistence primitives, MiniApp runtime IO, and network implementations | `services-core`, `services-integrations`, `terminal` | [AGENTS.md](src/crates/services/AGENTS.md) |
+| 4 | Services | `src/crates/services` | Reusable OS, filesystem, terminal, MCP, remote, git, watch, process, session persistence primitives, and network implementations | `services-core`, `services-integrations`, `terminal` | [AGENTS.md](src/crates/services/AGENTS.md) |
 | 5 | Execution primitives | `src/crates/execution` | Portable agent, stream, DeepReview policy/report, typed-service, tool-contract, and tool-execution building blocks | `agent-runtime`, `agent-stream`, `tool-contracts`, `runtime-services`, `tool-execution` | [AGENTS.md](src/crates/execution/AGENTS.md) |
 | 6 | Stable contracts and product domains | `src/crates/contracts` | Shared DTOs, event shapes, runtime ports, and product domain contracts/policies | `core-types`, `events`, `runtime-ports`, `product-domains` | [AGENTS.md](src/crates/contracts/AGENTS.md) |
 
@@ -32,7 +32,7 @@ Boundary rules:
 - Interfaces and app entrypoints expose selected product behavior; reusable behavior moves down.
 - Assembly wires lower layers and selects product capability facts; it must not implement concrete adapter, OS, or service details.
 - Adapters translate protocols and external systems; they should not own product capability selection or reusable OS service behavior.
-- Services implement reusable concrete OS, process, terminal, MCP, remote, git, filesystem, and MiniApp runtime IO capabilities.
+- Services implement reusable concrete OS, process, terminal, MCP, remote, git, and filesystem capabilities.
 - Execution crates are portable runtime building blocks, not host-specific or delivery-profile owners.
 - Contracts stay behavior-light and must not depend upward.
 
@@ -173,10 +173,10 @@ Change these only with a flag flip + integration test, and update this section i
 - **Desktop package is `northhing` (Slint)**, not `northhing-desktop`. agent-dispatch flags: only `USE_LIGHTWEIGHT_ACTOR = true` remains; Phase 3 IPC (USE_ONESHOT_DISPATCHER / USE_ACTOR_IPC / USE_DISPATCHER_IPC + IpcSpawnAdapter) descoped and deleted 2026-07-20.
 - **Config single source of truth = core `GlobalConfig`** (`dirs::config_dir()/northhing/config/app.json`). Desktop `AppSettings` stays UI-owner and pushes providers into core via `sync_providers_to_core` (see `95e29ba`). Never add a second runtime-readable config file.
 - **UI thread discipline**: writing Slint properties from a non-event-loop thread is silently dropped. All such writes must go through `slint::invoke_from_event_loop` (helpers in `error_banners.rs` already wrap this — reuse them, see `ad349f9`).
-- **Shell safety**: `guard_command_execution` is wired into the `validate_input` path of Bash/ExecCommand and writes audit entries (see `9a1575d`). New shell-like tools must call it too; MiniApp string-mode commands containing shell metacharacters are rejected.
+- **Shell safety**: `guard_command_execution` is wired into the `validate_input` path of Bash/ExecCommand and writes audit entries (see `9a1575d`). New shell-like tools must call it too.
 - **Project runtime slug always carries a path hash** (CJK paths must not collide, see `c7e7218`).
 - **Installer toolchain**: `northing-installer` `[lib] crate-type = ["rlib"]` only (cdylib/staticlib blow past the GNU ld export-ordinal limit); `embed-resource` pinned to 3.0.5 (3.0.11 fails on rustc 1.96 MSVC). Desktop builds use MSVC; repo dir override is GNU and `cargo +toolchain` is unavailable — use `rustup run <tc> cargo`.
-- **v0.1.0 surface baseline**: only Slint desktop + `northing-installer` are shipping surfaces; server / MiniApp UI / SDLC harness are frozen-experimental. Capability crates (tools/MCP/search/terminal/git/ssh) are the agent toolbox and stay active. See `docs/tech-debt-cleanup-guide.md` §0.
+- **v0.1.0 surface baseline**: only Slint desktop + `northing-installer` are shipping surfaces; server / SDLC harness are frozen-experimental. Capability crates (tools/MCP/search/terminal/git/ssh) are the agent toolbox and stay active. See `docs/tech-debt-cleanup-guide.md` §0.
 
 ## Architecture
 
diff --git a/README.md b/README.md
index 66bf267..9cf7087 100644
--- a/README.md
+++ b/README.md
@@ -40,7 +40,7 @@ See [`AGENTS.md`](AGENTS.md) for the layered module index, backbone invariants,
 See [`docs/status/surfaces.md`](docs/status/surfaces.md) for the complete ledger of shipping vs frozen-experimental surfaces.
 
 **Shipping (v0.1.0)**: Slint desktop + installer.  
-**Frozen-experimental**: CLI, server, MiniApp UI, SDLC harness.
+**Frozen-experimental**: CLI, server, SDLC harness.
 
 ## Tech Debt
 
diff --git a/docs/architecture/backend-roadmap.md b/docs/architecture/backend-roadmap.md
index 1f81f6b..eec1f15 100644
--- a/docs/architecture/backend-roadmap.md
+++ b/docs/architecture/backend-roadmap.md
@@ -82,7 +82,7 @@ FU-1 MCP 配置写 fail-closed、FU-2 LSP uninstall 按语言键停服（`7a4bdc
 
 | review 编号 | 内容 | 台账对应 |
 |---|---|---|
-| SW1-1 | MiniApp shell/net 空 allowlist=放行（语义翻转） | 新登记 |
+| ~~SW1-1~~ | ~~MiniApp shell/net 空 allowlist=放行（语义翻转）~~ | **随 MiniApp 整删关闭（moot）**（2026-08-17，T2-2） |
 | SW1-2 | 嵌入式 relay 0.0.0.0 无鉴权（默认 loopback + fail-closed） | = P1-7 |
 | SW1-3 | 远程来源对话取消跳过确认 | 新登记 |
 | SW1-4 | ComputerUse run_script 系接入 guard | 新登记 |
@@ -93,7 +93,7 @@ FU-1 MCP 配置写 fail-closed、FU-2 LSP uninstall 按语言键停服（`7a4bdc
 | SW1-9 | bot 配对码爆破防护 | 新登记 |
 | SW1-10 | 低危批量（恒时比较/Origin/hash 校验/ACP 钉版本） | 新登记 |
 
-依赖关系（2026-08-17 终版）：remote 栈已决删除——**T1-2 / T1-3 / T1-7 / T1-9 随栈关闭**；**MiniApp 已决整删——T1-1 / T3-5 随子系统关闭**。安全清单由 10 项缩至 **5 项（T1-4/5/6/8/10）**。删除前唯一要求：先摘除所有启动入口（feature/配置/UI），确保 dormant 期间不可被意外拉起。
+依赖关系（2026-08-17 终版）：remote 栈已整删（commits fa88342..d16b037）——**T1-2 / T1-3 / T1-7 / T1-9 随栈关闭**；**MiniApp 已整删（commits a930c93..T2-2o）——T1-1 / T3-5 随子系统关闭**。安全清单由 10 项缩至 **5 项（T1-4/5/6/8/10）**。启动入口（feature/配置/UI）已全数摘除，整删已完成。
 
 ### 1.5 债线（台账 backend active 项）
 
@@ -114,7 +114,7 @@ FU-1 MCP 配置写 fail-closed、FU-2 LSP uninstall 按语言键停服（`7a4bdc
 | `apps/server` | 位腐（源码 import core 但 Cargo.toml 未声明，编译不过；内含未接线 `ai_relay.rs`/`rpc_dispatcher.rs`） | T1-8 修复（删 ai_relay、修依赖）→ **T5 升格为进程外 core 宿主**（或新建 host，T5 时定） |
 | `apps/relay-server` | 已整删（T2-2 C5, commit f6a011b, PEND-1） | 随删除关闭；原维持/解冻评估规划失效 |
 | `apps/cli` | frozen（编译产物已有 CI：cli-package.yml） | T4（= K4b CLI 半）后评估解冻 |
-| MiniApp host | frozen（沙箱语义待修） | SW1-1 修复是任何 MiniApp 开放的前置 |
+| MiniApp host | 已整删（T2-2 M1-M5, commits a930c93..T2-2o） | 随删除关闭；原 SW1-1 / 开放规划失效 |
 | mobile-web/remote_connect | 已整删（TH-4 删除已执行，T2-2 C1-C7, commits fa88342..d16b037） | P1-4/P1-7/D-2 已随删除关闭；将来移动需求 = T5 协议客户端重建 |
 
 ---
@@ -164,7 +164,7 @@ FU-1 MCP 配置写 fail-closed、FU-2 LSP uninstall 按语言键停服（`7a4bdc
 | # | 内容 | 来源线 | 量 |
 |---|---|---|---|
 | T2-1 | **CI 补齐**：check 去 exclude、test 扩面、`cargo tree -p northhing-kernel-api` 守卫已在 CI（kernel-api-clean job）、desktop check 强制门（P2-15 流程结转） | K+review | S |
-| T2-2 | 死代码删除第一批（insights / tool-provider-groups / 空 session 目录 / webdriver / enigo+screenshots / **judge_gate 适配层**（assembly/core 1,690L；**协议层 1,473L 保留**转 TH-5 词汇，2026-08-17 G15 修正）≈6.5k 行）**+ remote 栈整删（TH-4：remote_connect 11.5k + mobile-web 4.7k + embedded relay 入口先摘后删；P1-4/P1-7/D-2 随之关闭；remote 栈部分（含 relay-server/relay-core 整删、mobile-web、contracts 修剪、i18n 面）已完成 C1-C8（commits fa88342..本批），MiniApp 部分待执行）** **+ MiniApp 子系统整删（2026-08-17 拍板：内置四件套 + 宿主 host_routing/bridge/manager/契约 ≈6k 行；permission_policy 默认拒绝语义先提炼进 PCS 设计再删码；连带关闭 T1-1、T3-5）**+ relay-server + relay-core 整删（PEND-1 拍板 2026-08-17：≈4-5k 行；surfaces.md 同 commit 同步）** + plan-compliance-checker(894L) + harness(571L，或并入 test-support)**，合计 ≈35k 行 | review+论题 | M |
+| ~~T2-2~~ | **已完成**（2026-08-19）：死代码删除第一批（insights / tool-provider-groups / 空 session 目录 / webdriver / enigo+screenshots / judge_gate 适配层 ≈6.5k 行，commits 38eb04a..0fbc987）+ remote 栈整删（TH-4：remote_connect / mobile-web / relay-server / relay-core，commits fa88342..3702baf）+ MiniApp 子系统整删（内置 6 套资产 / 宿主 / 顶层 MiniApp/，M1-M5 commits a930c93..T2-2o；连带关闭 T1-1、T3-5）+ plan-compliance-checker + harness，合计删除 ≈40k+ 行 | review+论题 | **Done** |
 | T2-9 | **功能冗余合并批次**（2026-08-17 冗余扫描）：第一批 S 级——deep_research 去重（255L×2，diff 仅 10 行注释→re-export）、ndjson_log 统一（4 个追加+轮转实现 ~1,320L）、now_unix_ms 统一（3 同名函数+25 内联）、原子写收口 json_store（顺修 P2-16 save_config 裸写；删 PersistenceService FILE_LOCKS）、初始化收口（server bootstrap 手抄 + CLI 样板×4 → init_agentic_system）；第二批 M 级——app.json↔GlobalConfig 镜像拆除（写穿 kernel API）、**事件管道收敛 A7**（BackendEvent 死管道并入 EventQueue 或删除）、**desktop NullDispatcher 空转路径移除**（agent-dispatch B2，回退直连直至 dispatcher 真接线）；延期 L 级——ExecCommand↔Bash 合并（Bash/PTY 为正）、双 ToolRegistry 迁移收尾、MCP core 包装层（3,641L）收口 | 冗余扫描 | 第一批 S / 第二批 M / 延期 L |
 | T2-10 | **连续性自检测试**：自动化"杀 core → 恢复 → diff 会话/记忆/身份"（T5"agent 不死"验收的轻量前置版，0.3 即可写，依赖 fake AI backend 提供确定性） | 论题 §3 度量 | S |
 
@@ -182,7 +182,7 @@ FU-1 MCP 配置写 fail-closed、FU-2 LSP uninstall 按语言键停服（`7a4bdc
 | PCS-6 | 协议插件：ACP 客户端作为插件形态接入（随 T4-5 协议冻结）；2.0 C 选项的生态入口 | 随 T4/T5 |
 | T2-3 | i18n 生成器大小写修复 + 幽灵目录清除 | review | XS |
 | T2-4 | 债项：P2-16（save_config 原子写）、P2-7（subagent_ports fake AI backend） | 债 | S |
-| T2-5 | unwrap 定向治理（password_vault / mcp::auth / miniapp::manager / facts） | review | M |
+| T2-5 | unwrap 定向治理（password_vault / mcp::auth / facts） | review | M |
 | T2-6 | god-file 复拆 + 行数守卫（callbacks_lifecycle 1063L / theme.rs 990L） | review+台账纪律 | M |
 | T2-7 | `code-rot-scan.sh` 建实或删引用；debug-log 轮转 | review | XS |
 | T2-8 | 命名 canonical 统一（随 D-4 拍板） | review | S |
@@ -213,7 +213,7 @@ FU-1 MCP 配置写 fail-closed、FU-2 LSP uninstall 按语言键停服（`7a4bdc
 | T3-2 | WebSearch 可配置（provider 化 + 降级路径） | review | M |
 | T3-3 | Provider 目录（preset 列表 + 能力声明化，替代名字推断） | review | M |
 | T3-4 | Gemini 视觉接通（放开 gating） | review | S |
-| ~~T3-5~~ | ~~MiniApp bridge 诚实化~~ | **随 MiniApp 整删关闭**（2026-08-17） | — |
+| ~~T3-5~~ | ~~MiniApp bridge 诚实化~~ | **随 MiniApp 整删关闭**（2026-08-17，随 T2-2 M1-M5 完成） | — |
 | T3-6 | 体验洞后端部分：P2-5 失败 turn 落史、P2-6 事件丢弃策略、P2-4 CleanupService 调度 | 债 | M |
 | T3-7 | **M 线落地**（**owner = growth session**，E-08）：TH-3 记忆浏览面板（read-only + JSONL 导出）+ TH-2 演化审计（策略/判定归 growth，P2-12 CI 硬门禁接线归编排线）+ TH-6 半被动约束配置 + P2-14 去重修复 + **本地度量埋点**（P-10 边界：不离机；记忆纠正频率/审计覆盖率/工具成功率） | M 线（论题） | M |
 | T3-8 | **TH-5 身份演化机制**（**owner = growth session**，E-08；G15-b 自评审模式：触发限轮内/维护周期，评审执行器新写参考 SubagentJudgeRunner，**复用保留的 judge_gate 协议层**，证据禁取 episodes（P2-12），consume-once 凭证继承 P2-11 教训；insights 删除不复活） | M 线（论题） | L |
@@ -244,7 +244,7 @@ FU-1 MCP 配置写 fail-closed、FU-2 LSP uninstall 按语言键停服（`7a4bdc
 | 选项 | 触发条件 | 前置 |
 |---|---|---|
 | 移动/IM 远程 | **已决删除**（论题 v1.1，D-1 终值）；将来如需 = T5 协议客户端重写，旧栈不复用 | T4-5 协议冻结 |
-| MiniApp 第三方生态 | 真实第三方开发者需求 | T1-1 + T3-5（否则沙箱是假的） |
+| ~~MiniApp 第三方生态~~ | **已失效**（MiniApp 子系统已整删；将来如需 = 2.0 协议插件形态） | 前提 T1-1 + T3-5 已随整删关闭 |
 | 多宿主/被嵌入 | T4-5 协议冻结后自动获得 | ACP server 已在；论题要求协议不锁死单 agent 假设（为 C 留口） |
 | CLI 解冻 | T4-1 完 + doctor 统一（P2-1 尾款） | surfaces 协议四要件 |
 
diff --git a/docs/status/decision-register.md b/docs/status/decision-register.md
index eeaf857..2a5db0e 100644
--- a/docs/status/decision-register.md
+++ b/docs/status/decision-register.md
@@ -37,7 +37,7 @@
 | P-11 | 08-16 | 数据导出 = 阅读形式（JSONL 单文件，随 M 线）；不做加密（小圈期） | 导出是阅读不是修改，与 P-04 兼容 | 含加密的完整可携带 | — | 生效 | TH-3 |
 | P-12 | 08-16 | CLI 永久 frozen | 论题下无独立产品面 | 解冻（除非 agent 自己需要它） | — | 生效 | G13 |
 | P-13 | 08-16 | 公开发布门槛清单：小圈用满 4 周后立；首项 = 重估 R-1/2/3 三条已接受风险 | 风险接受是小圈期条件性的 | 提前公开 | — | 生效 | G14 |
-| P-14 | 08-17 | **MiniApp 子系统整删**（内置四件套 + 宿主 ≈6k 行）；permission_policy 默认拒绝语义删除前提炼进 PCS 权限框架 | MiniApp 是"壳的功能"非"同事的本体"，与 core-agent 最优先冲突；空宿主是带安全缺陷的死重 | 只删内置留宿主冻结 | — | 生效 | T2-2、T1-1/T3-5 关闭、2.0 C 形态改协议插件 |
+| P-14 | 08-17 | **MiniApp 子系统整删**（内置六套资产 + 宿主 ≈11.2k rs/test + 55.9k 资产 + 顶层 8k 行，已执行：T2-2 M1-M5，commits a930c93..T2-2o）；permission_policy 默认拒绝语义删除前提炼进 PCS 权限框架 | MiniApp 是"壳的功能"非"同事的本体"，与 core-agent 最优先冲突；空宿主是带安全缺陷的死重 | 只删内置留宿主冻结 | — | 生效（已执行：T2-2 M1-M5，commits a930c93..T2-2o） | T2-2、T1-1/T3-5 关闭、2.0 C 形态改协议插件 |
 | P-15 | 08-17 | **PCS 插件连接系统 0.3 末启动**（core-agent 最优先的解耦+热插拔落地） | 用户明示优先级；MCP/LSP 已证进程外/zip 插件可行，PCS 补注册可逆+统一面板+权限框架 | 立即并行开工（地基未稳）；维持 0.5 才动（太晚） | 修订 D-9（P0 从 T5-1 提前） | 生效 | T2.5、D-9 |
 | P-16 | 08-17 | **PCS 权限批准者 = 用户安装时批准**（默认拒绝 + 安装时显式授权，类手机 App 权限弹窗）；agent 自主装留待 1.0 后凭 G15-b 自评审再议 | 与原则 7 确认门一致；agent 自主批准自己 = 权限框架失效 | agent 自主批准（永久）/ 无权限框架 | — | 生效 | PCS-3、A1 关闭 |
 | P-17 | 08-17 | **1.0 验收环补第六步**：「用 PCS 给它装一个插件，看它用起来」 | 原第六步（手机上继续）随 G4 删除；防 1.0 与 PCS 验收脱钩，一步同验注册/面板/权限/成长故事 | 维持五步（不考 PCS） | — | 生效 | E-05、thesis §7、PCS 验收 |
diff --git a/docs/status/surfaces.md b/docs/status/surfaces.md
index 50450d7..d27108f 100644
--- a/docs/status/surfaces.md
+++ b/docs/status/surfaces.md
@@ -19,7 +19,6 @@ These compile and may have partial functionality, but are **not** shipped, not t
 |---------|-------------|--------|-------|
 | **CLI** | `src/apps/cli` (`northhing-cli`) | 🧊 Frozen | Compiles; no release artifact. `doctor` command has false positives. See tech-debt-ledger P2. |
 | **Server** | `src/apps/server` | 🧊 Frozen | HTTP API surface; no auth layer. Not deployed. |
-| **MiniApp UI** | `src/crates/contracts/product-domains/src/miniapp/` | 🧊 Frozen | Built-in mini-apps (PPT live, etc.) are experimental. String-mode shell commands rejected by `guard_command_execution`. |
 | **Tauri Desktop (candidate)** | `src/apps/desktop-tauri` | 🧊 Frozen | Tauri 2 + React candidate for the next baseline; flips at F4. src-tauri is its own cargo workspace (excluded from main). |
 
 ## Active Capability Crates (Agent Toolbox)
diff --git a/docs/status/tech-debt-ledger.md b/docs/status/tech-debt-ledger.md
index 55f2cd3..a2abaa5 100644
--- a/docs/status/tech-debt-ledger.md
+++ b/docs/status/tech-debt-ledger.md
@@ -229,6 +229,13 @@
 - **Proposed fix**: 作为独立决策项处理，在后续工作区配置清理批次中移除（来源：T2-2h review F1/M-h）。
 - **Status**: active
 
+### P2-21: MiniApp 契约层三处 serde/wire 残留（零构造零生产者，反序列化兼容悬置待决）
+
+- **Symptom**: MiniApp 子系统整删后，契约层保留了三处 serde/wire 残留：`core-types/src/surface.rs:52` `RuntimeArtifactKind::MiniApp`、`services-core/src/session/session_metadata.rs:27` `SessionRelationshipKind::Miniapp`、`services-core/src/session/lineage.rs:19` `"miniapp"` tag。当前代码中零构造、零生产者，但直接删除存在旧会话/工件数据反序列化兼容风险。
+- **Evidence**: T2-2 MiniApp recon Q7 (`.superpowers/sdd/task-t2-2-miniapp-recon.md`)；`rg` 实测全仓零业务构造。
+- **Proposed fix**: 2026-08-19 用户决策超时未拍板，默认保守路径悬置待决。后续若确认无旧数据迁移负担可整删变体，或在反序列化层增加 serde alias/fallback 后删除。
+- **Status**: active (suspended / pending user decision)
+
 ## Change Protocol
 
 - **New entry**: Add with next available ID, include evidence (file:line), proposed fix, and status.
diff --git a/docs/tech-debt-cleanup-guide.md b/docs/tech-debt-cleanup-guide.md
index d86e547..18d55bd 100644
--- a/docs/tech-debt-cleanup-guide.md
+++ b/docs/tech-debt-cleanup-guide.md
@@ -9,7 +9,7 @@
 
 - 产品本质：**隐藏 IDE 模块的通用 agent 应用**。IDE/CLI/编程能力是主 agent + subagent 的工具，不是人类 UI。
 - 用户面（v0.1.0 唯一认账的）：**Slint 桌面（src/apps/desktop）+ 安装器（northing-installer）**。
-- 冻结面（标记 experimental，不修 bug、不删除代码）：`src/mobile-web`、`src/apps/server`、`src/apps/relay-server`、`src/crates/services/relay-core`（relay 部分）、MiniApp 运行时 UI、SDLC harness 产品面。能力 crates（tools/MCP/search/terminal/ssh/git 等）**全部保留**——那是 agent 的工具箱。
+- 冻结面（标记 experimental，不修 bug、不删除代码）：`src/mobile-web`、`src/apps/server`、`src/apps/relay-server`、`src/crates/services/relay-core`（relay 部分）、SDLC harness 产品面。能力 crates（tools/MCP/search/terminal/ssh/git 等）**全部保留**——那是 agent 的工具箱。
 - 确认策略：全免确认（用户拍板）。兜底 = shell denylist（已加固）+ core 快照。回滚入口暂不做。
 - UI 语言：v0.1.0 维持硬编码中文，i18n 工程不做。
 
@@ -72,7 +72,7 @@
 
 ### 3.6 docs/ 下的 web-ui / 已死引用
 
-已实证含 `src/web-ui` 引用的文档（12+）：`docs/architecture/core-decomposition.md`、`docs/architecture/deep-review.md`、`docs/architecture/i18n.md`、`docs/development/i18n.md`、`docs/features/session-runtime-usage-report-design.md`、`MiniApp/Skills/miniapp-dev/SKILL.md` 等。
+已实证含 `src/web-ui` 引用的文档（12+）：`docs/architecture/core-decomposition.md`、`docs/architecture/deep-review.md`、`docs/architecture/i18n.md`、`docs/development/i18n.md`、`docs/features/session-runtime-usage-report-design.md` 等。
 - 历史交接（docs/handoffs/2026-06-* 及更早）：文件头加 `> Frozen historical snapshot (pre-v0.1.0). Describes surfaces that may be absent/frozen.` 一行，不改正文。
 - 现行架构文档：web-ui 引用改为现状注记或移除。
 
@@ -112,7 +112,7 @@
 
 ### 5.1 新建 `docs/status/surfaces.md`（单一事实源）
 
-表格：每个面（desktop/installer/cli/mobile-web/server/relay/MiniApp/SDLC/web-ui）× 状态（active/frozen/absent）× 入口命令 × 备注。以后任何文档描述面状态以此为准。
+表格：每个面（desktop/installer/cli/mobile-web/server/relay/SDLC/web-ui）× 状态（active/frozen/absent）× 入口命令 × 备注。以后任何文档描述面状态以此为准。
 
 ### 5.2 新建 `docs/status/tech-debt-ledger.md`
 
diff --git a/src/crates/execution/agent-stream/src/tool_call_accumulator.rs b/src/crates/execution/agent-stream/src/tool_call_accumulator.rs
index 81c7d53..3dfaffa 100644
--- a/src/crates/execution/agent-stream/src/tool_call_accumulator.rs
+++ b/src/crates/execution/agent-stream/src/tool_call_accumulator.rs
@@ -147,7 +147,6 @@ mod tests {
             ("Grep", "Arguments are invalid JSON"),
             ("WebSearch", "OpenAI Agents SDK"),
             ("WebFetch", "https://example.com"),
-            ("InitMiniApp", "Markdown Viewer"),
         ];
 
         for (tool_name, raw_arguments) in cases {
