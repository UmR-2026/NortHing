BASE: 1fdb819 (working-tree diff, includes fix round 1)

## git log (base context)
1fdb819 docs: E-09 deletion decision (tool-packs+harness still deleted per plan) + handoff 2026-08-18
1e6926b sdd: T2-2a ledger line + recon/brief/report/review artifacts
2dfb8e4 chore: delete dead code batch 1 - insights/webdriver/plan-compliance-checker + orphan deps (T2-2a, review clean)

## git diff --stat
 AGENTS-CN.md                                       |   2 +-
 AGENTS.md                                          |   2 +-
 Cargo.lock                                         |  17 -
 Cargo.toml                                         |   2 -
 docs/architecture/agent-runtime-services-design.md |  20 +-
 docs/architecture/core-decomposition.md            | 717 ++++++++++-----------
 docs/status/surfaces.md                            |   2 -
 scripts/core-boundaries/rules/crate-layout.mjs     |   2 -
 scripts/core-boundaries/rules/crate-rules.mjs      |  38 --
 scripts/core-boundaries/rules/feature-rules.mjs    |  16 -
 .../rules/source/forbidden-rules.mjs               |  72 ---
 .../rules/source/required-rules.mjs                |  85 +--
 scripts/core-boundaries/self-test.mjs              |  39 --
 src/crates/assembly/core/AGENTS-CN.md              |   2 -
 src/crates/assembly/core/AGENTS.md                 |   2 -
 src/crates/assembly/core/Cargo.toml                |   8 -
 src/crates/assembly/core/src/agentic/harness.rs    |  68 --
 src/crates/assembly/core/src/agentic/mod.rs        |   1 -
 .../core/src/agentic/tools/product_runtime.rs      |  57 +-
 .../tools/product_runtime/materialization.rs       |  76 ++-
 .../core/src/agentic/tools/registry/tests.rs       |  12 +-
 src/crates/assembly/core/src/product_assembly.rs   |   7 +-
 .../assembly/product-capabilities/Cargo.toml       |   2 -
 .../assembly/product-capabilities/src/lib.rs       | 153 +----
 .../tests/product_capabilities.rs                  | 136 +---
 src/crates/execution/AGENTS-CN.md                  |   2 -
 src/crates/execution/AGENTS.md                     |   2 -
 src/crates/execution/harness/AGENTS.md             |  31 -
 src/crates/execution/harness/Cargo.toml            |  17 -
 src/crates/execution/harness/src/lib.rs            | 440 -------------
 src/crates/execution/harness/tests/registry.rs     | 131 ----
 src/crates/execution/tool-execution/AGENTS.md      |   4 +-
 .../execution/tool-provider-groups/AGENTS.md       |  32 -
 .../execution/tool-provider-groups/Cargo.toml      |  22 -
 .../execution/tool-provider-groups/src/lib.rs      | 402 ------------
 35 files changed, 462 insertions(+), 2159 deletions(-)

## git diff -U10
diff --git a/AGENTS-CN.md b/AGENTS-CN.md
index 6cf767c..82bc450 100644
--- a/AGENTS-CN.md
+++ b/AGENTS-CN.md
@@ -16,21 +16,21 @@ northhing 是一个 Rust 工作区加上 React 前端的组合。
 ## 分层模块索引
 
 依赖关系自上而下流动。每层只能依赖更低的层；各层内的 crate 依赖要保持到所需的最小集合。
 
 | # | 层 | 路径 | 职责 | 模块 / 入口 | 层文档 |
 |---|---|---|---|---|---|
 | 1 | 接口与入口 | `src/apps/*`、`src/web-ui`、`src/mobile-web`、`northhing-Installer`、`tests/e2e`、`src/crates/interfaces` | 产品宿主、命令、UI 入口、协议接口以及跨表面测试 | desktop、CLI、server、relay、Web UI、mobile web、installer、E2E、`acp` | 最近本地 `AGENTS.md`；[interfaces](src/crates/interfaces/AGENTS.md) |
 | 2 | 产品装配 | `src/crates/assembly` | 兼容性导出、产品能力选择、product-full 装配以及适配器/服务注册 | `core`、`product-capabilities` | [AGENTS.md](src/crates/assembly/AGENTS.md) |
 | 3 | 适配器 | `src/crates/adapters` | AI 协议适配器与外部提供方翻译 | `ai-adapters` | [AGENTS.md](src/crates/adapters/AGENTS.md) |
 | 4 | 服务 | `src/crates/services` | 可复用的 OS、文件系统、终端、MCP、远程、git、watch、进程、会话持久化原语、MiniApp 运行时 IO 以及网络实现 | `services-core`、`services-integrations`、`terminal` | [AGENTS.md](src/crates/services/AGENTS.md) |
-| 5 | 执行原语 | `src/crates/execution` | 可移植的 agent、harness、stream、DeepReview 策略/报告、typed-service、tool-contract、tool-group 以及 tool-execution 构件 | `agent-runtime`、`agent-stream`、`tool-contracts`、`harness`、`runtime-services`、`tool-provider-groups`、`tool-execution` | [AGENTS.md](src/crates/execution/AGENTS.md) |
+| 5 | 执行原语 | `src/crates/execution` | 可移植的 agent、stream、DeepReview 策略/报告、typed-service、tool-contract 以及 tool-execution 构件 | `agent-runtime`、`agent-stream`、`tool-contracts`、`runtime-services`、`tool-execution` | [AGENTS.md](src/crates/execution/AGENTS.md) |
 | 6 | 稳定契约与产品域 | `src/crates/contracts` | 共享 DTO、事件形态、运行时端口以及产品域契约/策略 | `core-types`、`events`、`runtime-ports`、`product-domains` | [AGENTS.md](src/crates/contracts/AGENTS.md) |
 
 边界规则：
 
 - 接口和应用入口暴露选定的产品行为；可复用行为下移。
 - 装配层连接下层并选择产品能力事实；不得实现具体的适配器、OS 或服务细节。
 - 适配器翻译协议和外部系统；不应拥有产品能力选择或可复用 OS 服务行为。
 - 服务实现可复用的具体 OS、进程、终端、MCP、远程、git、文件系统以及 MiniApp 运行时 IO 能力。
 - 执行 crate 是可移植的运行时构件，而不是宿主特定或交付配置的所有者。
 - 契约保持轻行为，不得向上依赖。
diff --git a/AGENTS.md b/AGENTS.md
index 26b27bb..1f81b79 100644
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -17,21 +17,21 @@ Repository rule: **keep product logic platform-agnostic, then expose it through
 
 Dependencies flow top to bottom. A layer may depend on lower layers only; keep
 crate dependencies inside each layer to the smallest set needed.
 
 | # | Layer | Path | Owns | Modules / entries | Layer doc |
 |---|---|---|---|---|---|
 | 1 | Interfaces and entrypoints | `src/apps/*`, `src/mobile-web` *(frozen)*, `northing-installer`, `tests/e2e`, `src/crates/interfaces` | Product hosts, commands, UI entrypoints, protocol interfaces, and cross-surface tests | desktop, CLI, server, relay, mobile web, installer, E2E, `acp` | nearest local `AGENTS.md`; [interfaces](src/crates/interfaces/AGENTS.md) |
 | 2 | Product assembly | `src/crates/assembly` | Compatibility exports, product capability selection, product-full wiring, and adapter/service registration | `core`, `product-capabilities` | [AGENTS.md](src/crates/assembly/AGENTS.md) |
 | 3 | Adapters | `src/crates/adapters` | AI protocol adapters and external-provider translation | `ai-adapters` | [AGENTS.md](src/crates/adapters/AGENTS.md) |
 | 4 | Services | `src/crates/services` | Reusable OS, filesystem, terminal, MCP, remote, git, watch, process, session persistence primitives, MiniApp runtime IO, and network implementations | `services-core`, `services-integrations`, `terminal` | [AGENTS.md](src/crates/services/AGENTS.md) |
-| 5 | Execution primitives | `src/crates/execution` | Portable agent, harness, stream, DeepReview policy/report, typed-service, tool-contract, tool-group, and tool-execution building blocks | `agent-runtime`, `agent-stream`, `tool-contracts`, `harness`, `runtime-services`, `tool-provider-groups`, `tool-execution` | [AGENTS.md](src/crates/execution/AGENTS.md) |
+| 5 | Execution primitives | `src/crates/execution` | Portable agent, stream, DeepReview policy/report, typed-service, tool-contract, and tool-execution building blocks | `agent-runtime`, `agent-stream`, `tool-contracts`, `runtime-services`, `tool-execution` | [AGENTS.md](src/crates/execution/AGENTS.md) |
 | 6 | Stable contracts and product domains | `src/crates/contracts` | Shared DTOs, event shapes, runtime ports, and product domain contracts/policies | `core-types`, `events`, `runtime-ports`, `product-domains` | [AGENTS.md](src/crates/contracts/AGENTS.md) |
 
 Boundary rules:
 
 - Interfaces and app entrypoints expose selected product behavior; reusable behavior moves down.
 - Assembly wires lower layers and selects product capability facts; it must not implement concrete adapter, OS, or service details.
 - Adapters translate protocols and external systems; they should not own product capability selection or reusable OS service behavior.
 - Services implement reusable concrete OS, process, terminal, MCP, remote, git, filesystem, and MiniApp runtime IO capabilities.
 - Execution crates are portable runtime building blocks, not host-specific or delivery-profile owners.
 - Contracts stay behavior-light and must not depend upward.
diff --git a/Cargo.lock b/Cargo.lock
index 8b51178..ef9602b 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -5978,31 +5978,29 @@ dependencies = [
  "local-ip-address",
  "md5",
  "northhing-agent-dispatch",
  "northhing-agent-runtime",
  "northhing-agent-stream",
  "northhing-agent-tools",
  "northhing-ai-adapters",
  "northhing-core-types",
  "northhing-debug-log",
  "northhing-events",
- "northhing-harness",
  "northhing-kernel-api",
  "northhing-product-capabilities",
  "northhing-product-domains",
  "northhing-relay-core",
  "northhing-runtime-ports",
  "northhing-runtime-services",
  "northhing-services-core",
  "northhing-services-integrations",
  "northhing-test-support",
- "northhing-tool-packs",
  "notify",
  "rand 0.8.7",
  "readability-js",
  "regex",
  "reqwest",
  "rmcp",
  "rusqlite",
  "russh",
  "rustls",
  "rustls-native-certs",
@@ -6058,49 +6056,38 @@ dependencies = [
  "anyhow",
  "async-trait",
  "chrono",
  "northhing-core-types",
  "serde",
  "serde_json",
  "tracing",
  "uuid",
 ]
 
-[[package]]
-name = "northhing-harness"
-version = "0.2.10"
-dependencies = [
- "async-trait",
- "thiserror 2.0.18",
- "tokio",
-]
-
 [[package]]
 name = "northhing-kernel-api"
 version = "0.1.0"
 dependencies = [
  "async-trait",
  "northhing-core-types",
  "northhing-events",
  "northhing-runtime-ports",
  "serde",
  "serde_json",
  "thiserror 2.0.18",
 ]
 
 [[package]]
 name = "northhing-product-capabilities"
 version = "0.2.10"
 dependencies = [
- "northhing-harness",
  "northhing-runtime-ports",
- "northhing-tool-packs",
 ]
 
 [[package]]
 name = "northhing-product-domains"
 version = "0.2.10"
 dependencies = [
  "dirs",
  "serde",
  "serde_json",
  "sha2",
@@ -6261,24 +6248,20 @@ dependencies = [
 
 [[package]]
 name = "northhing-test-support"
 version = "0.2.10"
 dependencies = [
  "serde",
  "serde_json",
  "uuid",
 ]
 
-[[package]]
-name = "northhing-tool-packs"
-version = "0.2.10"
-
 [[package]]
 name = "notify"
 version = "8.2.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "4d3d07927151ff8575b7087f245456e549fea62edf0ec4e565a5ee50c8402bc3"
 dependencies = [
  "bitflags 2.13.0",
  "fsevent-sys",
  "inotify",
  "kqueue",
diff --git a/Cargo.toml b/Cargo.toml
index 5d7067e..5333f6f 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -11,23 +11,21 @@ members = [
     "src/crates/services/services-core",
     "src/crates/services/services-integrations",
     "src/crates/services/terminal",
     "src/crates/services/debug-log",
     "src/crates/assembly/product-capabilities",
     "src/crates/contracts/product-domains",
     "src/crates/execution/agent-dispatch",
     "src/crates/execution/agent-runtime",
     "src/crates/execution/agent-stream",
     "src/crates/execution/tool-contracts",
-    "src/crates/execution/harness",
     "src/crates/execution/runtime-services",
-    "src/crates/execution/tool-provider-groups",
     "src/crates/execution/tool-execution",
     "src/crates/support/test-support",
     "src/crates/support/cli-internal",
     "src/crates/contracts/core-types",
     "src/crates/contracts/events",
     "src/crates/contracts/kernel-api",
     "src/crates/contracts/runtime-ports",
 ]
 
 exclude = [
diff --git a/docs/architecture/agent-runtime-services-design.md b/docs/architecture/agent-runtime-services-design.md
index 251b070..12c7dc6 100644
--- a/docs/architecture/agent-runtime-services-design.md
+++ b/docs/architecture/agent-runtime-services-design.md
@@ -40,21 +40,21 @@ Agent Runtime SDK çåå¸è¾¹çä»¥è°ç¨æ¹è½å
 registry æä¾èµå¨å± mutable stateï¼SDK åå¸è¾¹çå°±ä¸æç«ã?
 
 ### 1.2 crate åå
 
 ```text
 northhing-core-types
 northhing-events
 northhing-runtime-ports
 northhing-runtime-services      # typed service bundle / capability availability
 tool-contracts              # Cargo package: northhing-agent-tools
-tool-provider-groups        # Cargo package: northhing-tool-packs
+
 tool-execution              # Cargo package: tool-runtime
 northhing-agent-runtime         # agent kernel contracts and portable runtime decisions
 northhing-harness               # workflow descriptor / provider / registry contracts
 northhing-services-core
 northhing-services-integrations
 northhing-product-domains
 northhing-acp
 northhing-core
 apps/*
 ```
@@ -63,28 +63,28 @@ apps/*
 
 ```text
 apps/*
   -> northhing-core æ?Product Assembly crate
   -> æéä¾èµ northhing-acp / transport / api-layer
 
 Product Assembly
   -> product capability packs
   -> northhing-agent-runtime
   -> northhing-harness
-  -> tool-contracts / tool-provider-groups / tool-execution
+  -> tool-contracts / tool-execution
   -> northhing-runtime-services
   -> adapters / services
 
 Product Capability packs
   -> northhing-harness
   -> northhing-agent-runtime
-  -> tool-provider-groups
+
   -> northhing-product-domains
 
 northhing-agent-runtime
   -> northhing-runtime-ports
   -> northhing-events
   -> northhing-agent-stream
   -> tool-contracts
   -> northhing-runtime-services
 
 tool-execution
@@ -427,42 +427,36 @@ impl AgentRuntime {
 - `/goal` custom metadataãpost-turn verificationãcontinuation event ä¸æ¼ç§»ã?
 - `get_goal` / `create_goal` / `update_goal` ç?tool response wire shapeãblocked/complete è¯­ä¹å?token budget report ä¸æ¼ç§»ã?
 - `Task.run_in_background` delivery ä¸æ¼ç§»ã?
 - `Task.fork_context` ç¦æ­¢å­æ®µãprompt cache cloneãcontext seeding ä¸æ¼ç§»ã?
 - DeepResearch citation renumber post-turn hook ä¿æ deterministicã?
 
 ### 3.2 Tool Primitives
 
 æå±?crateï¼?
 
-- `tool-contracts`ï¼Cargo package: `northhing-agent-tools`ï¼?
-- `tool-provider-groups`ï¼Cargo package: `northhing-tool-packs`ï¼?
-- `tool-execution`ï¼Cargo package: `tool-runtime`ï¼?
+- `tool-contracts`ï¼Cargo package: `northhing-agent-tools`ï¼?- `tool-execution`ï¼Cargo package: `tool-runtime`ï¼?
 
 ç®æ èè´£ï¼?
 
-- `tool-contracts`ï¼tool DTOãmanifestãexposureãschemaãpath policyãresult policyãadmission gate å?provider-neutral registry assemblyã?
-- `tool-provider-groups`ï¼tool provider group feature metadata å?provider planã?
-- `tool-execution`ï¼ä½å±?file/search/tool IO helperï¼ä¸æ¥æäº§å registryãpermission policy æ?agent-facing tool surfaceã?
+- `tool-contracts`ï¼tool DTOãmanifestãexposureãschemaãpath policyãresult policyãadmission gate å?provider-neutral registry assemblyã?- `tool-execution`ï¼ä½å±?file/search/tool IO helperï¼ä¸æ¥æäº§å registryãpermission policy æ?agent-facing tool surfaceã?
 
 å»ºè®®æ¨¡åï¼?
 
 ```text
 tool-contracts
   framework.rs
   restrictions.rs
   file_guidance.rs
   tool_result_storage.rs
   tool_execution_presentation.rs
-
-tool-provider-groups
-  provider_groups.rs
+
 
 tool-execution
   filesystem.rs
   search.rs
   remote.rs
   result_window.rs
 ```
 
 æ ¸å¿æ¥å£ï¼?
 
@@ -962,15 +956,15 @@ Product æµè¯ï¼?
 - Remote workspace è¡ä¸ºã?
 - MCP dynamic tool catalogã?
 - MiniApp ä¸?review workflowã?
 
 ### 5.4 ç®æ æå¤å®å£å¾?
 
 - `northhing-agent-runtime` è½å¨ä¸ä¾èµ?`northhing-core` çæåµä¸æå»º runtime kernelã?
 - Agent Runtime SDK faÃ§ade è½éè¿ fake model providerãfake runtime servicesãfake tool provider å?fake
   harness provider å®ææå°?session / turn / event stream æµç¨ã?
 - `northhing-runtime-services` æä¾ typed service injectionï¼å¹¶ç?boundary check ä¿æ¤ã?
-- `tool-contracts`ã`tool-provider-groups` å?`tool-execution` åå«æ¿æ tool contractãprovider group plan åä½å±?execution helperï¼å·ä½?tool éè¿ Product Assembly æ³¨åã?
+- `tool-contracts` å?`tool-execution` åå«æ¿æ tool contract åä½å±?execution helperï¼å·ä½?tool éè¿ Product Assembly æ³¨åã?
 - `northhing-harness` æ¯æå·¥ä½æµ?provider æ©å±ã?
 - `northhing-core` åªä½ä¸ºå¼å®?facade / product-full assemblyã?
 - ææäº§åå½¢æéè¿ Product Assembly æ¾å¼å¯ç¨è½åã?
 - ææé«é£é©è¡ä¸ºæ?snapshotãfocused regression æ?product check ä¿æ¤ã?
diff --git a/docs/architecture/core-decomposition.md b/docs/architecture/core-decomposition.md
index efcd812..91b77fa 100644
--- a/docs/architecture/core-decomposition.md
+++ b/docs/architecture/core-decomposition.md
@@ -1,359 +1,358 @@
-﻿> **v0.1.0 status note**: This document has encoding issues (GBK/UTF-8 mojibake) and references src/web-ui/ which is [missing] in v0.1.0. Preserved for historical reference only.
-
-# northhing Core 忙聥聠猫搂拢忙聻露忙聻聞
-
-忙聹卢忙聳聡忙娄聜忙聥卢 northhing core runtime 忙聥聠猫搂拢莽職聞盲赂陇盲赂陋莽篓鲁氓庐職猫庐戮猫庐隆莽禄麓氓潞娄茂录職**氓聢聺氓搂聥莽聤露忙聙?*氓聮?*莽聸庐忙聽聡莽聤露忙聙?*茫聙?
-氓聢聺氓搂聥莽聤露忙聙聛忙聫聫猫驴掳猫庐戮猫庐隆氓禄潞莽芦聥忙聴露莽職聞盲潞聥氓庐聻忙聻露忙聻聞茫聙聛猫聙娄氓聬聢氓聟鲁莽鲁禄氓聮聦盲赂禄猫娄聛茅聴庐茅垄聵茂录聸莽聸庐忙聽聡莽聤露忙聙聛忙聫聫猫驴掳忙聹聼忙聹聸氓聢聠氓卤聜茫聙聛莽篓鲁氓庐職忙聨楼氓聫拢茫聙?
-氓庐聻莽聨掳氓陆聮氓卤聻茫聙聛莽禄聞猫拢聟猫戮鹿莽聲聦茫聙聛盲戮聺猫碌聳忙聳鹿氓聬聭氓聮聦茅拢聨茅聶漏莽潞娄忙聺聼茫聙?
-
-忙聹卢忙聳聡猫聛職莽聞娄猫庐戮猫庐隆莽禄聯猫庐潞茫聙聜猫炉娄莽禄聠忙聨楼氓聫拢茫聙聛crate 氓聠聟茅聝篓忙篓隆氓聺聴氓聮聦忙碌聥猫炉聲猫庐戮猫庐隆猫搂聛
-[`agent-runtime-services-design.md`](agent-runtime-services-design.md)茫聙?
-
-## 1. 猫聝聦忙聶炉盲赂聨莽聸庐忙聽?
-
-猫庐戮猫庐隆氓禄潞莽芦聥忙聴露茂录聦northhing 氓路虏莽禄聫盲禄?`northhing-core` 盲赂颅忙聤陆氓聡潞盲潞聠猫聥楼氓鹿虏 owner crate茂录聦盲陆聠 `northhing-core` 盲禄聧忙聣驴忙聥聟氓聟录氓庐?facade茫聙?
-氓庐聦忙聲麓盲潞搂氓聯聛 runtime 莽禄聞猫拢聟茫聙聛agent loop茫聙聛service 忙聨楼莽潞驴茫聙聛tool materialization 氓聮聦茅聝篓氓聢?product domain
-adapter茫聙聜猫驴聶盲赂陋氓陆垄忙聙聛氓聹篓氓聤聼猫聝陆盲赂聤氓聫炉猫驴聬猫隆聦茂录聦盲陆聠盲录職猫庐漏 runtime 忙聥聠猫搂拢忙聦聛莽禄颅茅聺垄盲赂麓盲赂聣盲赂陋茅聴庐茅垄聵茂录?
-
-- 盲潞搂氓聯聛茅聙禄猫戮聭茫聙聛氓鹿鲁氓聫掳忙聨楼氓聟楼氓聮聦氓聟路盲陆聯 service 氓庐聻莽聨掳猫戮鹿莽聲聦盲赂聧氓陇聼莽篓鲁氓庐職茫聙?
-- Desktop茫聙聛CLI茫聙聛Server茫聙聛Remote茫聙聛ACP茫聙聛Web 莽颅聣盲潞搂氓聯聛氓陆垄忙聙聛氓庐鹿忙聵聯猫垄芦氓庐聦忙聲麓 `northhing-core` 莽聣碌氓录聲茫聙?
-- Tool茫聙聛MCP茫聙聛ACP茫聙聛subagent茫聙聛skills茫聙聛harness 莽颅聣忙聣漏氓卤聲莽聜鹿莽录潞氓掳聭莽禄聼盲赂聙莽職聞氓聢聠氓卤聜氓陆聮氓卤聻茫聙?
-
-莽聸庐忙聽聡氓陆垄忙聙聛盲赂聧忙聵炉氓聹篓 `northhing-core` 氓聠聟莽禄搂莽禄颅忙聣漏氓录聽氓庐聦忙聲?`AgentRuntime`茂录聦猫聙聦忙聵炉氓陆垄忙聢聬氓聫炉莽聥卢莽芦聥氓碌聦氓聟楼莽職聞
-Agent Runtime SDK茫聙聜莽篓鲁氓庐職氓楼聭莽潞娄氓庐職盲鹿聣盲赂聤氓卤聜氓聫炉盲戮聺猫碌聳莽職聞忙聨楼氓聫拢茂录聦Product Assembly 猫麓聼猫麓拢忙鲁篓氓聠聦氓聟路盲陆聯氓庐聻莽聨掳茂录?
-Runtime Services茫聙聛Tool primitives 氓聮?Harness Layer 氓聢聠氓聢芦茅職聰莽娄禄 service茫聙聛tool茫聙聛氓路楼盲陆聹忙碌聛氓聮聦盲潞搂氓聯聛氓陆垄忙聙聛氓路庐氓录聜茫聙?
-
-Agent Runtime SDK 氓聹篓忙聹卢忙聳聡盲赂颅盲赂聧忙聵炉忙聼聬盲赂陋 crate 莽職聞莽庐聙氓聧聲茅聡聧氓聭陆氓聬聧茂录聦猫聙聦忙聵炉盲赂聙莽禄聞氓聫炉氓炉鹿氓陇聳莽篓鲁氓庐職忙聣驴猫炉潞莽職聞猫驴聬猫隆聦忙聴露猫聝陆氓聤聸猫戮鹿莽聲聦茫聙?
-莽聸庐忙聽聡莽聤露忙聙聛盲赂聥茂录聦猫掳聝莽聰篓忙聳鹿氓潞聰猫聝陆茅聙職猫驴聡莽篓鲁氓庐職 API 氓聢聸氓禄潞 runtime茫聙聛忙聫聬盲潞?turn茫聙聛忙露聢猫麓鹿盲潞聥盲禄露忙碌聛茫聙聛忙鲁篓氓聠?tool / harness / service
-provider茫聙聛氓陇聞莽聬?permission / cancellation / persistence / telemetry茂录聦猫聙聦盲赂聧茅聹聙猫娄聛盲戮聺猫碌?`northhing-core`茫聙聛app crate茫聙?
-Tauri handle 忙聢聳盲禄禄盲陆聲盲潞搂氓聯聛氓陆垄忙聙聛莽職聞 concrete manager茫聙聜氓聹篓猫炉楼莽聸庐忙聽聡猫戮戮忙聢聬氓聣聧茂录聦`execution` 氓卤聜氓聫陋猫聝陆莽搂掳盲赂潞忙聣搂猫隆聦氓聨聼猫炉颅茅聸聠氓聬聢茂录聦
-盲赂聧猫聝陆氓炉鹿氓陇聳氓庐拢莽搂掳盲赂潞氓庐聦忙聲?SDK茫聙?
-
-莽聸庐忙聽聡莽聤露忙聙聛氓驴聟茅隆禄盲驴聺忙聦聛盲潞搂氓聯聛猫隆聦盲赂潞茫聙聛茅禄聵猫庐陇猫聝陆氓聤聸茅聸聠氓聬聢茫聙聛忙聺聝茅聶聬猫炉颅盲鹿聣茫聙聛氓路楼氓聟路忙聸聺氓聟聣茫聙聛盲潞聥盲禄露猫炉颅盲鹿聣氓聮聦 release 忙聻聞氓禄潞氓陆垄忙聙聛莽颅聣盲禄路茫聙?
-
-## 2. 忙聻露忙聻聞氓聨聼氓聢聶
-
-- 盲戮聺猫碌聳氓聫陋猫聝陆盲禄聨盲潞搂氓聯聛氓聟楼氓聫?/ 盲潞搂氓聯聛莽禄聞猫拢聟忙碌聛氓聬聭盲潞搂氓聯聛猫聝陆氓聤聸茫聙聛氓聟路盲陆聯茅聙聜茅聟聧茫聙聛忙聹聧氓聤隆氓聮聦忙聣搂猫隆聦氓聨聼猫炉颅茂录聦氓聠聧忙碌聛氓聬聭莽篓鲁氓庐職氓楼聭莽潞娄茂录聸盲赂聥氓卤聜盲赂聧氓戮聴忙聞聼莽聼楼盲赂聤氓卤聜盲潞搂氓聯聛氓陆垄忙聙聛茫聙?
-- 忙聨楼氓聫拢氓聮聦氓庐聻莽聨掳氓驴聟茅隆禄氓聢聠氓录聙茂录職忙聨楼氓聫拢氓卤聻盲潞聨莽篓鲁氓庐職氓楼聭莽潞娄茫聙聛Runtime Services茫聙聛Tool primitives 忙聢?Harness contract茂录?
-  氓聟路盲陆聯氓庐聻莽聨掳氓卤聻盲潞聨 Product Assembly 莽職聞忙鲁篓氓聠聦猫戮鹿莽聲聦茫聙聛Adapters 忙聢?Services茫聙?
-- Product interface 氓聫炉盲禄楼忙聹聣氓路庐氓录聜茂录聦capability contract 氓驴聟茅隆禄忙聰露忙聲聸茫聙聜盲赂聧氓聬聦盲潞搂氓聯聛氓聟楼氓聫拢氓聫炉盲禄楼茅聙聣忙聥漏盲赂聧氓聬聦猫聝陆氓聤聸茅聸聠氓聬聢茂录?
-  盲陆聠盲赂聧猫聝陆茅聙職猫驴聡盲赂聥忙虏聣 UI茫聙聛氓聭陆盲禄陇忙聢聳氓聧聫猫庐庐茅聙禄猫戮聭忙聺楼忙聧垄氓聫聳氓陇聧莽聰篓茫聙?
-- `northhing-core` 盲驴聺莽聲聶氓聟录氓庐鹿 facade 氓聮?`product-full` 莽禄聞猫拢聟猫戮鹿莽聲聦茂录聸忙聳掳 owner crate 盲赂聧氓戮聴盲戮聺猫碌聳氓聸?
-  `northhing-core`茫聙?
-- 氓炉鹿氓陇聳 SDK API 氓驴聟茅隆禄忙聵炉莽篓鲁氓庐職茫聙聛莽陋聞氓聫拢氓戮聞茫聙聛氓聫炉莽聣聢忙聹卢氓聦聳莽職聞 fa脙搂ade茂录聦盲赂聧氓戮聴忙聤聤 `northhing-core`茫聙聛`product-full`茫聙聛氓聟篓茅聡?
-  service bundle 忙聢聳盲潞搂氓聯聛氓聠聟茅聝?manager 忙職麓茅聹虏莽禄聶猫掳聝莽聰篓忙聳鹿茫聙?
-- Hook 忙聵炉氓聫聴忙聨搂忙聣漏氓卤聲莽聜鹿茂录聦Event 忙聵炉盲潞聥氓庐聻茅聙職莽聼楼茫聙聜猫聝陆忙聰鹿氓聫聵猫隆聦盲赂潞莽職?hook 氓驴聟茅隆禄忙聹聣茅隆潞氓潞聫茫聙聛timeout茫聙聛茅聰聶猫炉炉莽颅聳莽聲楼氓聮聦莽颅聣盲禄路盲驴聺忙聤陇茫聙?
-- feature group 忙聵炉忙聻聞氓禄潞猫戮鹿莽聲聦茂录聦CapabilitySet 忙聵炉盲潞搂氓聯聛猫驴聬猫隆聦忙聴露猫聝陆氓聤聸猫戮鹿莽聲聦茂录聸盲赂陇猫聙聟氓驴聟茅隆禄莽聰卤 Product Assembly
-  忙聵戮氓录聫忙聵聽氓掳聞茫聙?
-
-## 3. 氓聢聺氓搂聥莽聤露忙聙聛茅聙禄猫戮聭猫搂聠氓聸戮
-
-氓聢聺氓搂聥莽聤露忙聙聛莽職聞忙聽赂氓驴聝盲潞聥氓庐聻忙聵炉茂录職氓陇職盲赂陋 crate 氓路虏莽禄聫忙聣驴忙聨楼盲潞聠莽篓鲁氓庐職莽卤禄氓聻聥茫聙聛盲潞聥盲禄露茫聙聛stream茫聙聛tool contract茫聙聛茅聝篓氓聢?service
-helper 氓聮?product domain 莽潞炉茅聙禄猫戮聭茂录聦盲陆聠氓庐聦忙聲麓猫驴聬猫隆聦忙聴露盲禄聧盲禄?`northhing-core` 盲赂潞盲赂颅氓驴聝茫聙?
-
-```mermaid
-flowchart TB
-  Surfaces["盲潞搂氓聯聛氓聟楼氓聫拢<br/>Desktop / CLI / Server / Relay / Remote / Web"]
-  Core["northhing-core<br/>氓聟录氓庐鹿 facade + 氓庐聦忙聲麓盲潞搂氓聯聛 runtime 莽禄聞猫拢聟"]
-  Acp["northhing-acp<br/>ACP protocol surface / client behavior"]
-  Transport["transport / api-layer<br/>API 盲赂聨盲录聽猫戮?adapter"]
-  CoreTypes["northhing-core-types<br/>莽篓鲁氓庐職 DTO 氓颅聬茅聸聠"]
-  Events["northhing-events<br/>盲潞聥盲禄露盲潞聥氓庐聻盲赂?emitter 忙聤陆猫卤隆"]
-  Ports["northhing-runtime-ports<br/>trait-only runtime 猫戮鹿莽聲聦"]
-  Stream["northhing-agent-stream<br/>stream 猫聛職氓聬聢"]
-  AgentTools["northhing-agent-tools<br/>tool contract 盲赂聨莽潞炉莽颅聳莽聲楼"]
-  ToolRuntime["tool-execution<br/>tool-runtime package / 盲陆聨氓卤聜 helper"]
-  ToolPacks["tool-provider-groups<br/>northhing-tool-packs package / provider plan"]
-  ServicesCore["northhing-services-core<br/>氓聼潞莽隆聙 service helper / filesystem facade"]
-  ServicesIntegrations["northhing-services-integrations<br/>MCP / Git / Remote helper owner"]
-  ProductDomains["northhing-product-domains<br/>MiniApp / function-agent 莽潞?domain"]
-  Terminal["terminal-core<br/>terminal domain"]
-  Ai["northhing-ai-adapters<br/>忙篓隆氓聻聥 provider adapter"]
-  External["氓陇聳茅聝篓莽鲁禄莽禄聼<br/>OS / Git / MCP / ACP / AI provider / remote host"]
-
-  Surfaces --> Core
-  Surfaces --> Transport
-  Surfaces --> Acp
-  Acp --> Core
-  Core --> CoreTypes
-  Core --> Events
-  Core --> Ports
-  Core --> Stream
-  Core --> AgentTools
-  Core --> ToolRuntime
-  Core --> ToolPacks
-  Core --> ServicesCore
-  Core --> ServicesIntegrations
-  Core --> ProductDomains
-  Core --> Terminal
-  Core --> Ai
-  Core --> Transport
-  ServicesCore --> External
-  ServicesIntegrations --> External
-  Terminal --> External
-  Ai --> External
-```
-
-氓聢聺氓搂聥莽聤露忙聙聛盲赂禄猫娄聛忙篓隆氓聺聴猫聦聝氓聸麓茂录職
-
-| 忙篓隆氓聺聴 | 氓聢聺氓搂聥氓庐職盲陆聧 | 忙聻露忙聻聞氓陆卤氓聯聧 |
-|---|---|---|
-| `northhing-core` | 氓聟录氓庐鹿 facade茫聙聛agent runtime茫聙聛tool runtime 莽禄聞猫拢聟茫聙聛service 忙聨楼莽潞驴氓聮聦氓庐聦忙聲麓盲潞搂氓聯聛猫聝陆氓聤聸茅聸聠氓聬?| 盲禄聧忙聵炉盲潞聥氓庐聻盲赂聤莽職聞 runtime owner茂录聦忙聥聠猫搂拢氓驴聟茅隆禄氓聟聢盲驴聺忙聤陇猫隆聦盲赂潞莽颅聣盲禄路 |
-| `northhing-runtime-ports` | 茅聺垄氓聬聭 runtime/service 猫戮鹿莽聲聦莽職?DTO 氓聮?trait | 氓聫陋氓庐職盲鹿?contract茂录聦盲赂聧忙聥楼忙聹聣 runtime 氓庐聻莽聨掳 |
-| `tool-contracts` / `northhing-agent-tools` | provider-neutral tool DTO茫聙聛manifest茫聙聛path/result policy茫聙聛catalog contract 氓聮?deterministic execution admission gate | 茅聙聜氓聬聢忙聣驴忙聨楼莽潞?tool contract 莽颅聳莽聲楼茂录聦盲陆聠盲赂聧氓潞聰忙聥楼忙聹聣氓聟路盲陆聯 IO tool |
-| `tool-execution` / `tool-runtime` | 忙聴垄忙聹聣盲陆聨氓卤聜氓路楼氓聟路忙聣搂猫隆聦 helper crate | 莽聸庐忙聽聡忙聵炉氓聫陋忙聣驴忙聨楼盲陆聨氓卤聜 file/search/tool execution helper茂录聦盲赂聧忙聥楼忙聹聣盲潞搂氓聯聛 registry 忙聢?permission policy |
-| `northhing-services-core` | 氓聼潞莽隆聙 service helper茫聙聛忙聹卢氓聹?filesystem facade茫聙聛茅聝篓氓聢聠茅聙職莽聰篓 service 茅聙禄猫戮聭 | 茅聙聜氓聬聢盲陆聹盲赂潞忙聹卢氓聹掳氓聼潞莽隆聙 service owner茂录聦盲陆聠盲赂聧猫聝陆氓聬赂忙聰露盲潞搂氓聯聛 runtime 猫炉颅盲鹿聣 |
-| `northhing-services-integrations` | MCP茫聙聛Git茫聙聛remote-connect茫聙聛remote-SSH 莽颅?integration helper | 茅聙聜氓聬聢忙聥楼忙聹聣氓陇聳茅聝篓氓聧聫猫庐庐氓聮聦茅聡聧盲戮聺猫碌聳 service implementation茂录聦盲赂聧氓潞聰氓聫聧氓聬聭忙聞聼莽聼楼盲潞搂氓聯?interface |
-| `northhing-product-domains` | MiniApp茫聙聛function-agent 莽颅聣莽潞炉莽聤露忙聙聛茫聙聛莽颅聳莽聲楼茫聙聛port 氓聮聦茅聝篓氓聢聠氓聠鲁莽颅聳茅聙禄猫戮聭 | 茅聙聜氓聬聢忙聣驴忙聨楼 pure domain茂录聦盲赂聧氓潞聰莽聸麓忙聨楼忙聣搂猫隆?filesystem/Git/AI concrete call |
-| `northhing-acp` | ACP protocol interface 氓聮?client behavior | 氓潞聰盲驴聺忙聦聛盲潞搂氓聯聛氓聧聫猫庐庐氓聟楼氓聫拢茂录聦盲赂聧盲赂聥忙虏聣氓聢掳 Agent Runtime |
-| `transport` / `api-layer` | surface 氓聢?runtime 莽職?API/transport adapter | 氓潞聰盲驴聺忙聦聛盲录聽猫戮聯氓卤聜茂录聦盲赂聧忙聥楼忙聹聣 runtime owner |
-
-## 4. 氓聢聺氓搂聥莽聤露忙聙聛盲赂禄猫娄聛茅聴庐茅垄?
-
-### 4.1 氓聢聠氓卤聜盲赂聧忙赂聟忙聶?
-
-氓聬聦盲赂聙猫聝陆氓聤聸莽禄聫氓赂赂氓聬聦忙聴露氓聦聟氓聬芦 UI/command茫聙聛runtime orchestration茫聙聛tool execution茫聙聛service IO 氓聮?domain
-decision茫聙聜氓聢聺氓搂聥莽聤露忙聙聛盲禄拢莽聽聛盲赂颅猫驴聶盲潞聸茅聝篓氓聢聠盲禄聧氓陇搂茅聡聫茅聙職猫驴聡 `northhing-core` 盲赂虏猫聛聰茂录聦氓炉录猫聡麓忙聥聠猫搂拢忙聴露茅職戮盲禄楼氓聢陇忙聳颅芒聙聹莽搂禄氓聤篓莽職聞忙聵炉忙聨楼氓聫拢茫聙?
-氓庐聻莽聨掳茫聙聛莽禄聞猫拢聟茅聙禄猫戮聭猫驴聵忙聵炉盲潞搂氓聯聛猫隆聦盲赂潞芒聙聺茫聙?
-
-### 4.2 忙聨楼氓聫拢盲赂聨氓庐聻莽聨掳猫戮鹿莽聲聦盲赂聧莽篓鲁氓庐職
-
-氓路虏忙聹聣 `runtime-ports` 氓聮聦猫聥楼氓鹿?contract crate茂录聦盲陆聠猫庐赂氓陇職 call site 盲禄聧盲戮聺猫碌?concrete manager茫聙?
-core-owned context 忙聢聳氓庐聦忙聲?product runtime snapshot茫聙聜忙聨楼氓聫拢忙虏隆忙聹聣莽篓鲁氓庐職氓聢掳猫露鲁盲禄楼猫庐?runtime 盲赂聨氓聟路盲陆?service
-氓庐聻莽聨掳莽聥卢莽芦聥忙录聰猫驴聸茫聙?
-
-### 4.3 盲潞搂氓聯聛氓陆垄忙聙聛猫垄芦氓庐聦忙聲麓 core 莽聣碌氓录聲
-
-Desktop茫聙聛CLI茫聙聛Server茫聙聛Remote茫聙聛ACP 氓聮?Web 莽職聞氓聟楼氓聫拢氓路庐氓录聜猫戮聝氓陇搂茂录聦盲陆聠氓聢聺氓搂聥莽聤露忙聙聛盲赂聥氓陇搂氓陇職盲禄聧茅聙職猫驴聡氓庐聦忙聲麓 `northhing-core`
-猫聨路氓戮聴猫聝陆氓聤聸茫聙聜猫驴聶盲录職猫庐漏猫陆禄茅聡聫盲潞陇盲禄聵氓陆垄忙聙聛莽禄搂忙聣驴盲赂聧氓驴聟猫娄聛莽職?tool茫聙聛service茫聙聛UI 忙聢聳氓鹿鲁氓聫掳盲戮聺猫碌聳茫聙?
-
-### 4.4 Tool contract 盲赂?tool execution 忙路路氓聬聢
-
-provider-neutral manifest茫聙聛path policy茫聙聛result policy茫聙聛`ToolUseContext` runtime handle茫聙聛collapsed unlock
-lifecycle茫聙聛runtime artifact persistence 氓聮?product registry materialization 氓聹篓氓聢聺氓搂聥莽聤露忙聙聛盲赂聥盲赂?concrete tool
-execution 盲潞陇莽禄聡氓聹?core 氓聫聤氓聟露氓聟录氓庐鹿猫路炉氓戮聞盲赂颅茫聙聜莽聸庐忙聽聡莽聤露忙聙聛盲赂聥茂录聦tool contracts 氓潞聰忙聥楼忙聹?provider-neutral manifest /
-catalog / permission / result / artifact contract茂录聦core茫聙聛services 忙聢?adapter 氓聫陋盲驴聺莽聲聶氓庐聻茅聶?IO tool adapter茫聙?
-state update茫聙聛忙聴搂猫路炉氓戮聞 facade 氓聮聦忙聹聣莽颅聣盲禄路盲驴聺忙聤陇莽職聞忙聥聠猫搂拢猫戮鹿莽聲聦茫聙聜氓路楼氓聟?owner 忙聥聠猫搂拢氓娄聜忙聻聹忙虏隆忙聹聣氓驴芦莽聟搂盲驴聺忙聤陇茂录聦氓庐鹿忙聵聯忙聰鹿氓聫?
-prompt-visible manifest茫聙聛`GetToolSpec`茫聙聛MCP/ACP catalog 忙聢?oversized result 猫隆聦盲赂潞茫聙?
-
-### 4.5 Service茫聙聛MCP茫聙聛ACP 盲赂?runtime kernel 氓庐鹿忙聵聯盲潞陇氓聫聣
-
-MCP 氓聮?ACP 忙聵炉氓陇聳茅聝篓氓聧聫猫庐?猫聝陆氓聤聸忙聨楼氓聟楼茂录聦盲赂聧氓潞聰氓聫聵忙聢?Agent Runtime SDK 莽職聞氓聠聟茅聝篓氓聧聫猫庐庐盲戮聺猫碌聳茫聙聜Runtime kernel 氓聫陋氓潞聰莽聹聥猫搂聛
-external capability茫聙聛tool provider 忙聢?service port茂录聸猫驴聻忙聨楼莽聰聼氓聭陆氓聭篓忙聹聼茫聙聛茅聣麓忙聺聝茫聙聛transport 氓聮?timeout 莽颅聳莽聲楼氓潞聰莽聰卤
-Adapters茫聙聛Services 忙聢?Product Assembly 莽庐隆莽聬聠茫聙?
-
-### 4.6 忙聣漏氓卤聲莽聜鹿莽录潞氓掳聭莽禄聼盲赂聙猫炉颅盲鹿聣
-
-agent definitions茫聙聛subagents茫聙聛skills茫聙聛prompt modules茫聙聛tool providers茫聙聛MCP providers茫聙聛hooks 氓聮?
-product commands 茅聝陆忙聵炉忙聣漏氓卤聲莽聜鹿茂录聦盲陆聠莽聸庐氓聣聧忙虏隆忙聹聣莽禄聼盲赂聙猫隆篓猫戮戮氓庐聝盲禄卢氓聢聠氓聢芦氓卤聻盲潞聨氓聯陋盲赂聙氓卤聜茫聙聛氓娄聜盲陆聲忙鲁篓氓聠聦茫聙聛忙聵炉氓聬娄氓聟聛猫庐赂忙聰鹿氓聫聵猫隆聦盲赂潞茫聙?
-盲禄楼氓聫聤氓娄聜盲陆聲氓聛職忙聺聝茅聶聬氓聮聦忙碌聥猫炉聲盲驴聺忙聤陇茫聙?
-
-### 4.7 feature graph 猫驴聵盲赂聧忙聵炉盲潞搂氓聯聛猫聝陆氓聤聸莽聼漏茅聵?
-
-氓聢聺氓搂聥莽聤露忙聙聛盲赂聥茂录聦`product-full` 忙聵炉氓庐聦忙聲麓盲潞搂氓聯聛猫聝陆氓聤聸莽職聞氓庐聣氓聟篓莽陆聭茂录聦盲赂聧忙聵炉忙聹聙莽禄聢忙聦聣盲潞搂氓聯聛忙聥聠氓聢聠莽職?feature matrix茫聙聜莽聸麓忙聨楼氓聡聫猫陆禄茅禄聵猫庐?feature
-忙聢聳忙聤聤 feature group 氓陆聯忙聢聬盲潞搂氓聯聛猫聝陆氓聤聸猫戮鹿莽聲聦茂录聦茅聝陆盲录職氓录聲氓聟楼忙聻聞氓禄潞氓陆垄忙聙聛氓聮聦氓聫聭氓赂聝猫聝陆氓聤聸忙录聜莽搂禄茫聙?
-
-### 4.8 忙聻聞氓禄潞盲赂聨忙碌聥猫炉聲莽聣碌氓录聲猫驴聡氓陇?
-
-茅聡聧盲戮聺猫碌聳氓聮聦氓庐聦忙聲麓 runtime 猫聛職氓聬聢氓聹?`northhing-core` 氓聭篓氓聸麓茂录聦氓炉录猫聡麓氓卤聙茅聝篓忙碌聥猫炉聲茫聙聛owner crate 忙碌聥猫炉聲氓聮聦猫陆禄茅聡聫盲潞搂氓聯聛氓聟楼氓聫拢氓庐鹿忙聵聯猫垄芦
-盲赂聧莽聸赂氓聟鲁盲戮聺猫碌聳忙聥聳氓聟楼莽录聳猫炉聭氓聮聦茅聯戮忙聨楼猫路炉氓戮聞茫聙聜莽聸庐忙聽聡莽聤露忙聙聛氓驴聟茅隆禄猫庐漏盲戮聺猫碌聳忙聰露莽聸聤氓聫炉氓潞娄茅聡聫茂录聦氓聬聦忙聴露盲赂聧猫聝陆盲禄楼莽聣潞莽聣虏氓聤聼猫聝陆莽颅聣盲禄路忙聧垄氓聫聳忙聻聞氓禄潞忙聰露莽聸聤茫聙?
-
-### 4.9 SDK 氓聫聭氓赂聝猫戮鹿莽聲聦盲赂聧猫露鲁
-
-氓路虏忙聹聣 `northhing-agent-runtime`茫聙聛`northhing-runtime-services`茫聙聛`tool-contracts`茫聙聛`tool-execution`茫聙聛`northhing-harness`
-氓聮?`runtime-ports` 莽颅?SDK 氓聙聶茅聙聣氓聨聼猫炉颅茂录聦盲陆聠莽录潞氓掳聭氓聫炉氓炉鹿氓陇聳忙聣驴猫炉潞莽職聞莽禄聼盲赂聙 runtime fa脙搂ade茫聙聛莽篓鲁氓庐職茅聰聶猫炉炉忙篓隆氓聻聥茫聙聛盲潞聥盲禄露忙碌聛氓聧聫猫庐庐茫聙?
-provider 忙鲁篓氓聠聦猫戮鹿莽聲聦茫聙聛忙聦聛盲鹿聟氓聦聳/忙聛垄氓陇聧氓楼聭莽潞娄氓聮聦忙聹聙氓掳聫盲戮聺猫碌聳忙聻聞氓禄潞氓陆垄忙聙聛茫聙聜氓娄聜忙聻聹氓陇聳茅聝篓猫掳聝莽聰篓忙聳鹿盲禄聧茅聹聙猫娄聛莽聸麓忙聨楼莽聬聠猫搂?`northhing-core`茫聙?
-`product-full`茫聙聛concrete service manager 忙聢聳盲潞搂氓聯聛氓聭陆盲禄陇猫路炉氓戮聞茂录聦猫炉麓忙聵聨 SDK 猫戮鹿莽聲聦氓掳職忙聹陋氓庐聦忙聢聬茫聙?
-
-## 5. 氓炉鹿莽聟搂氓聢聠忙聻聬
-
-忙聹卢猫聤聜氓聫陋忙聫聬莽聜录氓炉鹿 northhing 氓聢聠氓卤聜忙聹聣莽聰篓莽職聞忙聻露忙聻聞盲驴隆氓聫路茂录聦盲赂聧忙聤聤氓聟露盲禄聳茅隆鹿莽聸庐莽職聞氓庐聻莽聨掳氓陆垄忙聙聛莽聸麓忙聨楼氓陇聧氓聢露氓聢掳 northhing茫聙?
-
-### 5.1 Claude Code 莽聸赂氓聟鲁氓庐聻莽聨掳氓聫聜猫聙?
-
-Claude Code 莽聸赂氓聟鲁 Rust 氓庐聻莽聨掳氓聫聜猫聙聝盲赂颅茂录聦workspace 氓掳?CLI binary茫聙聛provider API茫聙聛runtime茫聙聛tools茫聙?
-commands茫聙聛plugins茫聙聛telemetry 氓聮?mock harness 忙聥聠忙聢聬盲赂聧氓聬聦 crate茫聙聜氓聟露 `runtime` 猫麓聼猫麓拢 session茫聙聛config茫聙?
-permission茫聙聛MCP茫聙聛prompt 氓聮?runtime loop茂录聸`tools` 猫麓聼猫麓拢 tool specs 盲赂聨忙聣搂猫隆聦茂录聸`commands` 猫麓聼猫麓拢 slash command
-registry茂录聸`plugins` 猫麓聼猫麓拢 plugin metadata茫聙聛hook 氓聮?install/enable/disable surfaces茫聙聜猫炉楼莽禄聯忙聻聞猫炉麓忙聵聨茂录?
-
-- 氓路楼氓聟路猫搂聞忙聽录茫聙聛氓聭陆盲禄?surface茫聙聛plugin/hook 氓聮?runtime loop 氓聫炉盲禄楼氓聢聠氓录聙忙录聰猫驴聸茫聙?
-- permission茫聙聛MCP lifecycle茫聙聛task registry茫聙聛LSP registry 莽颅聣氓聫炉盲陆聹盲赂潞 runtime/service owner 莽庐隆莽聬聠茂录聦猫聙聦盲赂聧忙聵炉忙聲拢猫聬陆氓聹篓 UI茫聙?
-- 氓娄聜忙聻聹 runtime crate 氓聬聦忙聴露氓聬赂忙聰露 session茫聙聛MCP茫聙聛permission茫聙聛prompt 氓聮?tool bridge茂录聦盲鹿聼盲录職氓聫聵忙聢聬忙聳掳莽職聞茅聡聧猫聛職氓聬聢莽聜鹿茫聙?
-
-忙聙禄莽禄聯茂录職忙聥聠氓聢?crate 盲赂聧忙聵炉莽聸庐忙聽聡忙聹卢猫潞芦茂录聦氓聟鲁茅聰庐忙聵炉猫庐?CLI/TUI茫聙聛commands茫聙聛tools茫聙聛plugins茫聙聛runtime 氓聮?
-service integrations 茅聙職猫驴聡莽篓鲁氓庐職 contract 莽禄聞氓聬聢茂录聦茅聛驴氓聟聧忙聤聤 `northhing-core` 莽職聞猫聛職氓聬聢茅聴庐茅垄聵忙聬卢氓聢掳忙聳掳莽職?runtime crate茫聙?
-
-### 5.2 Opencode
-
-Opencode 氓庐聵忙聳鹿忙聳聡忙隆拢氓卤聲莽陇潞盲潞聠忙聸麓氓聛聫盲潞搂氓聯聛氓聦聳莽職聞忙聣漏氓卤聲忙篓隆氓聻聥茂录職氓聬聦盲赂聙盲赂?agent 氓聫炉盲禄楼猫驴聬猫隆聦氓聹?terminal茫聙聛desktop 忙聢?IDE茂录?
-agents 氓聢聠盲赂潞 primary agents 氓聮?subagents茂录聦氓聫炉茅聟聧莽陆庐 prompt茫聙聛model 盲赂?tool access茂录聸tools 茅聙職猫驴聡 permission 忙聨搂氓聢露茂录?
-氓鹿露氓聫炉茅聙職猫驴聡 custom tools 忙聢?MCP servers 忙聣漏氓卤聲茂录聸plugins 猫庐垄茅聵聟 command茫聙聛file茫聙聛permission茫聙聛session茫聙聛tool茫聙聛TUI
-莽颅聣盲潞聥盲禄露茂录聸skills 茅聙職猫驴聡莽聥卢莽芦聥莽聸庐氓陆聲忙聦聣茅聹聙氓聫聭莽聨掳氓聮聦氓聤聽猫陆陆茫聙?
-
-忙聙禄莽禄聯茂录?
-
-- Agent茫聙聛Tool茫聙聛MCP茫聙聛Plugin/Hook茫聙聛Skill 氓聮?Product Surface 氓潞聰猫炉楼忙聵炉盲潞聮莽聸赂猫驴聻忙聨楼莽職聞忙聣漏氓卤聲茅聺垄茂录聦猫聙聦盲赂聧忙聵炉氓聬聦盲赂聙盲赂陋忙篓隆氓聺聴氓聠聟茅聝篓莽職聞氓聢聠忙聰炉茫聙?
-- 忙聺聝茅聶聬氓聮聦氓路楼氓聟路氓聫炉猫搂聛忙聙搂氓驴聟茅隆禄忙聵炉 runtime 氓聫炉猫搂聜忙碌聥莽職聞 contract茂录聦盲赂聧猫聝陆氓聫陋氓颅聵氓聹篓盲潞?UI 忙聢?prompt 忙聥录忙聨楼盲赂颅茫聙?
-- 氓陇職盲潞搂氓聯聛氓陆垄忙聙聛茅聹聙猫娄?Product Assembly 氓聛?capability/provider 茅聙聣忙聥漏茂录聦猫聙聦盲赂聧忙聵炉猫庐漏 Agent Runtime SDK 氓聢陇忙聳颅猫掳聝莽聰篓忙聺楼猫聡陋
-  Desktop茫聙聛CLI茫聙聛Remote 猫驴聵忙聵炉 ACP茫聙?
-
-## 6. 莽聸庐忙聽聡茅聙禄猫戮聭猫搂聠氓聸戮
-
-莽聸庐忙聽聡忙聻露忙聻聞盲禄楼氓聟颅盲赂陋莽聣漏莽聬?owner 氓聢聠氓聦潞猫隆篓猫戮戮盲戮聺猫碌聳忙聳鹿氓聬聭茫聙聜`interfaces` 氓聫陋忙聣驴猫陆陆氓聧聫猫庐庐氓聮聦氓庐驴盲赂禄氓聟楼氓聫拢茂录聸`assembly` 猫麓聼猫麓拢盲潞搂氓聯聛猫聝陆氓聤聸茅聙聣忙聥漏盲赂聨忙鲁篓氓聠聦茂录聸`adapters` 猫麓聼猫麓拢氓聧聫猫庐庐茫聙聛transport 氓聮聦氓陇聳茅聝?provider 猫陆卢忙聧垄茂录聸`services` 猫麓聼猫麓拢忙聹卢氓聹掳莽鲁禄莽禄聼盲赂?runtime infrastructure 莽職聞氓聫炉氓陇聧莽聰篓氓聟路盲陆聯氓庐聻莽聨掳茂录聸`execution` 氓聫陋忙聰戮氓聫炉莽搂禄忙陇聧忙聣搂猫隆聦氓聨聼猫炉颅茂录聸`contracts` 忙聫聬盲戮聸莽篓鲁氓庐職盲潞聥氓庐聻茫聙聛port 氓聮聦盲潞搂氓聯聛茅垄聠氓聼聼猫搂聞氓聢聶茫聙聜猫驴聶忙聽路氓聫炉盲禄楼氓聬聦忙聴露氓聦潞氓聢聠芒聙聹氓聧聫猫庐庐茅聙聜茅聟聧芒聙聺氓聮聦芒聙聹忙聹聧氓聤隆氓庐聻莽聨掳芒聙聺茂录聦盲鹿聼茅聛驴氓聟聧忙聤聤 execution 猫炉炉猫搂拢盲赂潞氓庐聦忙聲麓猫驴聬猫隆聦忙聴露氓庐聻莽聨掳氓卤聜茫聙?
-
-```mermaid
-flowchart TB
-  Interfaces["忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录?br/>UI / command / protocol interface / delivery profile"]
-  Assembly["盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录?br/>compatibility facade / capability selection / adapter and service registration"]
-  Adapters["茅聙聜茅聟聧氓卤聜茂录聢Adapters茂录?br/>AI / API / transport / WebDriver / external provider translation"]
-  Services["忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录?br/>filesystem / git / terminal / MCP / remote / process / OS integration"]
-  Execution["忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录?br/>agent / harness / stream / typed-service / tool primitives"]
-  Contracts["莽篓鲁氓庐職氓楼聭莽潞娄盲赂聨盲潞搂氓聯聛茅垄聠氓聼聼氓卤聜茂录聢Stable Contracts and Product Domains茂录?br/>DTO / event / runtime port / product domain policy"]
-  External["氓陇聳茅聝篓莽鲁禄莽禄聼茂录聢External Systems茂录?br/>OS / Git / MCP server / ACP client / AI provider / remote host"]
-
-  Interfaces --> Assembly
-  Interfaces --> Adapters
-  Assembly --> Adapters
-  Assembly --> Services
-  Assembly --> Execution
-  Assembly --> Contracts
-  Adapters --> Services
-  Adapters --> Execution
-  Adapters --> Contracts
-  Services --> Execution
-  Services --> Contracts
-  Execution --> Contracts
-  Adapters --> External
-  Services --> External
-```
-
-盲戮聺猫碌聳忙聳鹿氓聬聭氓聫陋氓聟聛猫庐赂盲禄聨盲赂聤氓聢掳盲赂聥茫聙聜忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜忙職麓茅聹虏盲潞搂氓聯聛氓陆垄忙聙聛茂录聸莽禄聞猫拢聟氓卤聜茅聙聣忙聥漏猫聝陆氓聤聸茅聸聠氓聬聢氓鹿露忙鲁篓氓聠?adapter/service茂录聸茅聙聜茅聟聧氓卤聜莽驴禄猫炉聭氓聧聫猫庐庐氓聮聦氓陇聳茅聝篓 provider茂录聸忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜忙聨楼猫搂娄 OS茫聙聛process茫聙聛filesystem茫聙聛git茫聙聛terminal茫聙聛MCP 氓聮?remote茂录聸忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜忙聫聬盲戮聸氓聫炉氓陇聧莽聰?runtime building blocks茂录聸氓楼聭莽潞娄氓卤聜忙聫聬盲戮聸莽篓鲁氓庐職盲潞聥氓庐聻茫聙聛port 氓聮聦盲潞搂氓聯聛茅垄聠氓聼聼猫搂聞氓聢聶茫聙聜盲禄禄盲陆聲盲赂聥氓卤?crate 氓聫聧氓聬聭猫炉禄氓聫聳盲潞搂氓聯聛氓聟楼氓聫拢茫聙聛莽禄聞猫拢聟茅聟聧莽陆庐忙聢聳 host state 茅聝陆猫搂聠盲赂潞猫戮鹿莽聲聦猫驴聺猫搂聞茫聙?
-
-## 7. 莽聸庐忙聽聡氓卤聜莽潞搂
-
-莽聸庐忙聽聡氓卤聜莽潞搂盲禄楼莽聣漏莽聬?owner 氓聢聠氓聦潞盲赂潞氓聟楼氓聫拢茫聙聜忙炉聫盲赂陋氓聢聠氓聦潞氓聫炉盲禄楼氓聦聟氓聬芦氓陇職盲赂?crate茂录聦盲陆聠 crate 氓聠聟茅聝篓猫聛聦猫麓拢氓驴聟茅隆禄猫聝陆氓陇聼茅聙職猫驴聡盲戮聺猫碌聳茫聙聛忙碌聥猫炉聲氓聮聦猫戮鹿莽聲聦猫聞職忙聹卢莽聥卢莽芦聥茅陋聦猫炉聛茫聙?
-
-### 7.1 忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录?
-
-忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜忙聵炉莽聰篓忙聢路茫聙聛氓聧聫猫庐庐忙聢聳氓陇聳茅聝篓莽鲁禄莽禄聼猫驴聸氓聟楼 northhing 莽職聞氓聟楼氓聫拢茂录聦猫麓聼猫麓拢 UI茫聙聛氓聭陆盲禄陇茫聙聛猫路炉莽聰卤茫聙聛氓聧聫猫庐庐忙聨楼氓聫拢茫聙聛盲潞陇盲禄聵氓陆垄忙聙聛茅聙聣忙聥漏氓聮?host integration茫聙聜氓炉鹿氓潞聰猫聦聝氓聸麓氓聦聟忙聥?`src/apps/*`茫聙聛`src/web-ui`茫聙聛`src/mobile-web`茫聙聛`northhing-Installer`茫聙聛`tests/e2e` 氓聮?`src/crates/interfaces`茫聙聜氓聟楼氓聫拢氓卤聜氓聫炉盲禄楼茅聙聣忙聥漏 `DeliveryProfile` 氓鹿露猫掳聝莽聰?assembly 忙聢?adapter API茂录聦盲陆聠盲赂聧忙聥楼忙聹聣氓聟卤盲潞?runtime 猫隆聦盲赂潞茫聙?
-
-### 7.2 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录?
-
-盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜猫麓聼猫麓拢氓聟录氓庐鹿氓炉录氓聡潞茫聙聛氓庐聦忙聲麓盲潞搂氓聯聛猫聝陆氓聤聸茅聙聣忙聥漏茫聙聛feature group 氓聢?capability set 莽職聞忙聵聽氓掳聞茫聙聛adapter/service 忙鲁篓氓聠聦氓聮?product-full 忙聨楼莽潞驴茫聙聜莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/assembly`茂录聦氓陆聯氓聣聧氓聦聟氓聬?`northhing-core` 氓聟录氓庐鹿茅聴篓茅聺垄氓聮?`northhing-product-capabilities` 猫聝陆氓聤聸忙篓隆氓聻聥茫聙聜`product-capabilities` 氓聫陋忙聫聫猫驴?capability id茫聙聛tool group茫聙聛service requirement 氓聮?harness selection茂录聦盲赂聧忙聣搂猫隆聦 IO茂录聦盲鹿聼盲赂聧忙聣驴猫陆陆盲潞搂氓聯聛茅垄聠氓聼聼莽聤露忙聙聛忙聹潞茫聙?
-
-### 7.3 茅聙聜茅聟聧氓卤聜茂录聢Adapters茂录?
-
-茅聙聜茅聟聧氓卤聜猫麓聼猫麓拢氓聧聫猫庐庐茫聙聛transport茫聙聛氓陇聳茅聝?provider 氓聮聦氓庐驴盲赂禄茅聙職盲驴隆猫陆卢忙聧垄茂录聦莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/adapters`茫聙聜氓聟露盲赂?`ai-adapters` 猫麓聼猫麓拢 AI provider 猫炉路忙卤聜/氓聯聧氓潞聰忙聵聽氓掳聞氓聮?provider stream 氓聧聫猫庐庐猫搂拢忙聻聬茂录聦猫搂拢忙聻聬莽禄聯忙聻聹氓潞聰猫陆卢忙聧垄盲赂?execution 氓卤聜忙聥楼忙聹聣莽職聞莽禄聼盲赂聙 stream 氓楼聭莽潞娄茂录聸`api-layer` 猫麓聼猫麓拢盲潞搂氓聯聛氓庐驴盲赂禄氓聟卤莽聰篓莽職聞氓聬聨莽芦?API adapter茂录聦`transport` 猫麓聼猫麓拢盲潞聥盲禄露忙聤聲茅聙聮氓聮聦 host transport adapter茂录聦`webdriver` 猫麓聼猫麓拢 WebDriver 氓聧聫猫庐庐氓聮聦忙碌聫猫搂聢氓聶篓猫聡陋氓聤篓氓聦?adapter茫聙聜茅聙聜茅聟聧氓卤聜盲赂聧忙聥楼忙聹聣盲潞搂氓聯聛猫聝陆氓聤聸茅聙聣忙聥漏茂录聦盲鹿聼盲赂聧忙聣驴猫陆陆氓聫炉氓陇聧莽聰篓 OS service 氓庐聻莽聨掳茫聙?
-
-### 7.4 忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录?
-
-忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜猫麓聼猫麓拢忙聨楼猫搂娄忙聹卢氓聹掳莽鲁禄莽禄聼氓聮聦 runtime infrastructure 莽職聞氓聫炉氓陇聧莽聰篓氓聟路盲陆聯氓庐聻莽聨掳茂录聦莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/services`茫聙聜氓聟露盲赂?`services-core` 忙聣驴猫陆陆猫陆禄茅聡聫 service primitive茂录聦`services-integrations` 忙聣驴猫陆陆 MCP茫聙聛Git茫聙聛remote茫聙聛file watch 氓聮聦盲潞搂氓聯聛茅垄聠氓聼?port 莽職聞氓聟路盲陆聯氓庐聻莽聨掳茂录聦`terminal` 忙聣驴猫陆陆 PTY茫聙聛shell integration 氓聮?terminal session infrastructure茫聙聜忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜氓聫炉盲禄楼氓庐聻莽聨掳 `contracts`茫聙聛`execution` 忙聢?`product-domains` 氓庐職盲鹿聣莽職?port茂录聦盲陆聠盲赂聧茅聙聣忙聥漏盲潞搂氓聯聛 profile茂录聦盲鹿聼盲赂聧莽聸麓忙聨楼忙職麓茅聹?UI/氓聧聫猫庐庐氓聟楼氓聫拢茫聙?
-
-### 7.5 忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录?
-
-忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜忙聫聬盲戮?provider-neutral 莽職?runtime building blocks茂录聦莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/execution`茫聙聜`agent-runtime`茫聙聛`agent-stream`茫聙聛`harness`茫聙聛`runtime-services`茫聙聛`tool-contracts`茫聙聛`tool-provider-groups` 氓聮?`tool-execution` 氓聢聠氓聢芦氓庐職盲鹿聣 agent loop facts茫聙聛莽禄聼盲赂聙 stream DTO / tool-call 莽麓炉莽搂炉 / replay 氓楼聭莽潞娄茫聙聛workflow descriptor茫聙聛typed service bundle茫聙聛tool manifest / permission / result policy茫聙聛tool group facts 氓聮聦盲陆聨氓卤?tool execution helper茫聙聜氓陆聯氓聣?Cargo package / lib 氓聬聧盲驴聺忙聦聛氓聟录氓庐鹿茂录聦盲陆聠莽聣漏莽聬聠莽聸庐氓陆聲忙聦聣猫聛聦猫麓拢氓聭陆氓聬聧茫聙聜氓庐聝盲禄卢氓聫陋猫聝陆盲戮聺猫碌聳莽篓鲁氓庐職氓楼聭莽潞娄忙聢聳忙聵聨莽隆庐莽職?provider-neutral DTO茂录聦盲赂聧莽聸麓忙聨楼氓聢聸氓禄潞 Tauri handle茫聙聛filesystem manager茫聙聛Git provider茫聙聛MCP client茫聙聛AI client 忙聢?host process茫聙?
-
-### 7.6 莽篓鲁氓庐職氓楼聭莽潞娄盲赂聨盲潞搂氓聯聛茅垄聠氓聼聼氓卤聜茂录聢Stable Contracts and Product Domains茂录?
-
-莽篓鲁氓庐職氓楼聭莽潞娄盲赂聨盲潞搂氓聯聛茅垄聠氓聼聼氓卤聜忙聵炉忙聹聙盲陆聨氓卤聜茂录聦莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/contracts`茫聙聜氓庐聝氓聦聟氓聬芦 `core-types`茫聙聛`events`茫聙聛`runtime-ports` 氓聮?`product-domains`茫聙聜`product-domains` 忙聵?Product Domain Model茂录聦猫麓聼猫麓?MiniApp茫聙聛function-agent 莽颅聣茅垄聠氓聼?DTO茫聙聛莽潞炉莽颅聳莽聲楼茫聙聛莽聤露忙聙聛猫搂聞氓聢聶氓聮聦莽陋?port茂录聸氓聟路盲陆?Git茫聙聛filesystem茫聙聛AI 忙聢?worker execution 氓庐聻莽聨掳氓聹?services茫聙聛adapters 忙聢?assembly/core 莽職聞氓聟录氓庐鹿猫路炉氓戮聞盲赂颅茂录聦盲赂聧氓戮聴氓聸聻忙碌聛氓聢掳 contracts茫聙?
-
-### 7.7 忙聣漏氓卤聲莽聜鹿氓陆聮氓卤?
-
-- AI茫聙聛API茫聙聛transport 氓聮?WebDriver 莽職聞氓聧聫猫庐庐猫陆卢忙聧垄氓卤聻盲潞?Adapters茫聙?
-- MCP茫聙聛terminal茫聙聛filesystem茫聙聛git茫聙聛remote 氓聮?file watch 莽職聞氓聫炉氓陇聧莽聰篓氓聟路盲陆聯氓庐聻莽聨掳氓卤聻盲潞聨 Services茫聙?
-- Tool manifest茫聙聛permission茫聙聛execution admission茫聙聛result / artifact policy 氓卤聻盲潞聨 Execution Primitives 莽職?`tool-contracts`茫聙?
-- Tool provider group facts 氓卤聻盲潞聨 Execution Primitives 莽職?`tool-provider-groups`茂录聸盲陆聨氓卤?filesystem/search helper 氓卤聻盲潞聨 `tool-execution`茫聙?
-- Agent茫聙聛subagent茫聙聛prompt module茫聙聛scheduler茫聙聛session / turn facts 氓聮?hook routing 氓卤聻盲潞聨 Execution Primitives茫聙?
-- Harness workflow descriptor 氓聮?route plan 氓卤聻盲潞聨 Execution Primitives茂录聸氓聟路盲陆聯氓路楼盲陆聹忙碌聛 IO 莽聲聶氓聹篓 Services茫聙聛Adapters 忙聢聳氓聟录氓庐鹿猫路炉氓戮聞茂录聦莽聸麓氓聢掳忙聹聣莽颅聣盲禄路盲驴聺忙聤陇氓聬聨氓聠聧猫驴聛莽搂禄茫聙?
-- Capability pack茫聙聛delivery profile茫聙聛adapter/service selection 氓聮?product-full assembly 氓卤聻盲潞聨 Product Assembly茫聙?
-- 盲潞搂氓聯聛茅垄聠氓聼聼莽聤露忙聙聛茫聙聛猫搂聞氓聢聶茫聙聛port 氓聮?domain policy 氓卤聻盲潞聨 Stable Contracts and Product Domains茫聙?
-
-## 8. 忙聨楼氓聫拢盲赂聨氓庐聻莽聨掳氓聟鲁莽鲁?
-
-忙聨楼氓聫拢莽聰卤莽篓鲁氓庐職氓楼聭莽潞娄茫聙聛Runtime Services茫聙聛Tool Contracts 忙聢?Harness contract 氓庐職盲鹿聣茂录聸氓聟路盲陆聯氓庐聻莽聨掳莽聰卤 adapter茫聙聛service 忙聢聳盲潞搂氓聯聛氓聟楼氓聫拢氓聢聸氓禄潞茂录聸忙鲁篓氓聠聦氓聤篓盲陆聹氓聫陋猫聝陆氓聫聭莽聰聼氓聹?Product Assembly茫聙聜Agent Runtime茫聙聛tool contracts茫聙聛tool execution 氓聮?Harness 氓聫陋忙聨楼忙聰露氓路虏莽禄聫莽禄聞猫拢聟氓楼陆莽職聞忙聨楼氓聫拢忙聢聳 provider registry茂录聦盲赂聧莽聸麓忙聨楼氓聢聸氓禄潞氓鹿鲁氓聫掳氓庐聻莽聨掳茫聙?
-
-```mermaid
-flowchart TB
-  Interface["忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录?br/>茅聙聣忙聥漏氓聟楼氓聫拢氓聮?DeliveryProfile"]
-  Assembly["盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录?br/>氓聰炉盲赂聙忙鲁篓氓聠聦莽聜?]
-  ServiceBuilder["猫驴聬猫隆聦忙聴露忙聹聧氓聤隆氓卤聜茂录聢Runtime Services茂录?br/>RuntimeServicesBuilder"]
-  ToolBuilder["氓路楼氓聟路忙聣搂猫隆聦氓聨聼猫炉颅茂录聢Tool Primitives茂录?br/>tool contracts / groups / execution"]
-  HarnessBuilder["氓路楼盲陆聹忙碌聛莽录聳忙聨聮氓卤聜茂录聢Harness Layer茂录?br/>HarnessRegistryBuilder"]
-  AgentRegistry["Agent 忙聣搂猫隆聦氓聨聼猫炉颅茂录聢Agent Runtime茂录?br/>AgentDefinitionRegistry"]
-  CommandRegistry["忙聨楼氓聫拢 / 盲潞搂氓聯聛莽禄聞猫拢聟氓卤?br/>ProductCommandRegistry"]
-  Runtime["Agent / Tool / Harness primitives<br/>氓聫陋忙露聢猫麓鹿忙聨楼氓聫?]
-  Adapters["茅聙聜茅聟聧氓卤聜茂录聢Adapters茂录?br/>AI / API / transport / WebDriver adapters"]
-  Services["忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录?br/>OS / filesystem / Git / terminal / MCP / remote services"]
-  Contracts["莽篓鲁氓庐職氓楼聭莽潞娄盲赂聨盲潞搂氓聯聛茅垄聠氓聼聼氓卤聜茂录聢Stable Contracts and Product Domains茂录?br/>DTO / event / port trait"]
-
-  Interface --> Assembly
-  Assembly --> ServiceBuilder
-  Assembly --> ToolBuilder
-  Assembly --> HarnessBuilder
-  Assembly --> AgentRegistry
-  Assembly --> CommandRegistry
-  Assembly --> Adapters
-  Assembly --> Services
-  ServiceBuilder --> Runtime
-  ToolBuilder --> Runtime
-  HarnessBuilder --> Runtime
-  AgentRegistry --> Runtime
-  CommandRegistry --> Interface
-  Runtime --> Contracts
-  Adapters --> Contracts
-  Services --> Contracts
-  Adapters --> Services
-```
-
-忙鲁篓氓聠聦氓聶篓盲赂聨氓聣聧忙聳聡莽聸庐忙聽聡氓卤聜莽潞搂莽職聞氓炉鹿氓潞聰氓聟鲁莽鲁禄氓娄聜盲赂聥茂录職
-
-| 忙鲁篓氓聠聦氓聶?/ 莽禄聞猫拢聟莽聜?| 忙聣聙氓卤聻莽聸庐忙聽聡氓卤聜莽潞?| 氓聢聺氓搂聥忙聣驴猫陆陆盲赂聨莽聸庐忙聽聡忙聣驴猫陆?| 忙鲁篓氓聠聦氓聠聟氓庐鹿 |
-|---|---|---|---|
-| `ProductAssembler` / `ProductAssemblyPlan` | 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录?| 氓聢聺氓搂聥氓聫炉氓聹篓 `northhing-core` facade 忙聢聳盲潞搂氓聯聛氓聟楼氓聫拢茂录聸莽聸庐忙聽聡氓聫炉忙聰露忙聲聸盲赂潞 assembly owner | `DeliveryProfile`茫聙聛`CapabilitySet`茫聙聛feature group茫聙聛adapter/service 茅聙聣忙聥漏 |
-| `RuntimeServicesBuilder` | 忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录聣盲赂聨忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录聣莽職聞猫戮鹿莽聲聦 | 莽聸庐忙聽聡氓聹?`northhing-runtime-services`茂录聸猫驴聻忙聨?`northhing-runtime-ports`茫聙聛`northhing-services-*` 氓聮聦氓聢聺氓搂?service wiring | filesystem茫聙聛workspace茫聙聛session store茫聙聛Git茫聙聛terminal茫聙聛network茫聙聛MCP catalog茫聙聛remote connection / workspace / projection port |
-| `ToolRuntimeBuilder` | 忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录?| `tool-execution`茫聙聛`tool-contracts`茫聙聛`tool-provider-groups`茂录聸Cargo package 氓聬聧盲驴聺忙聦聛氓聟录氓庐?| tool provider茫聙聛tool group茫聙聛manifest茫聙聛permission gate茫聙聛tool hook |
-| `HarnessRegistryBuilder` | 氓路楼盲陆聹忙碌聛莽录聳忙聨聮氓卤聜茂录聢Harness Layer茂录?| 莽聸庐忙聽聡氓聹?`northhing-harness`茂录聸氓聢聺氓搂聥氓聫炉莽聰?`northhing-core::agentic::harness` 忙鲁篓氓聠聦 legacy-facade provider | SDD茫聙聛Deep Review茫聙聛DeepResearch茫聙聛MiniApp 莽颅?harness provider |
-| `AgentDefinitionRegistry` | 忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录?| 莽聸庐忙聽聡氓聹?`northhing-agent-runtime`茂录聸氓聢聺氓搂聥氓聫炉莽聰?`northhing-core` agent definition 盲禄拢莽聽聛忙聣驴猫陆陆 | agent茫聙聛subagent茫聙聛prompt module茫聙聛skill definition |
-| `ProductCommandRegistry` | 忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录聣盲赂聨盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录聣莽職聞猫戮鹿莽聲聦 | 盲潞搂氓聯聛氓聟楼氓聫拢忙聢?assembly 忙篓隆氓聺聴 | 猫戮聯氓聟楼忙隆聠氓聭陆盲禄陇茫聙聛氓庐隆忙聽赂氓聟楼氓聫拢茫聙聛MiniApp 氓聟楼氓聫拢氓聢?capability / harness / runtime request 莽職聞忙聵聽氓掳?|
-| adapter set | 茅聙聜茅聟聧氓卤聜茂录聢Adapters茂录?| `northhing-ai-adapters`茫聙聛`northhing-api-layer`茫聙聛`northhing-transport`茫聙聛`northhing-webdriver`茫聙聛app adapters | AI茫聙聛API茫聙聛transport茫聙聛WebDriver 莽颅聣氓聧聫猫庐庐忙聢聳氓陇聳茅聝篓 provider adapter |
-| service set | 忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录?| `northhing-services-*`茫聙聛`terminal-core` 氓聮聦氓聟路盲陆?app service implementations | OS茫聙聛filesystem茫聙聛Git茫聙聛terminal茫聙聛MCP茫聙聛remote 莽職聞氓聟路盲陆?service茂录聸Remote service 氓聠聟茅聝篓莽禄搂莽禄颅氓聦潞氓聢聠 SSH茫聙聛relay茫聙聛忙聹卢氓聹掳茅職搂茅聛聯茫聙聛猫驴聹莽芦?OS 忙聰炉忙聦聛 |
-
-忙鲁篓氓聠聦猫路炉氓戮聞氓驴聟茅隆禄忙聵炉忙聵戮氓录聫茫聙聛typed茫聙聛氓聫炉忙碌聥猫炉聲莽職聞茂录職
-
-- 忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录聣氓聫陋茅聙聣忙聥漏 `DeliveryProfile` 氓聮聦盲潞搂氓聯聛茅聟聧莽陆庐茂录聦盲赂聧莽聸麓忙聨楼忙聤聤 concrete manager 盲录聽氓聟楼 runtime茫聙?
-- 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录聣忙聽鹿忙聧庐盲潞搂氓聯聛氓陆垄忙聙聛氓聢聸氓禄潞忙聢聳忙聨楼忙聰露 adapter/service茂录聦氓鹿露猫掳聝莽聰篓 typed builder 氓庐聦忙聢聬忙鲁篓氓聠聦茫聙?
-- Tool茫聙聛OS茫聙聛Remote茫聙聛Protocol provider 氓聢聠氓聢芦莽聲聶氓聹篓氓炉鹿氓潞聰 app茫聙聛Adapters 忙聢?Services 盲赂颅茂录聦茅聙職猫驴聡氓聬聦盲赂聙莽禄?port 忙職麓茅聹虏茫聙?
-- Tauri 氓聫陋猫聝陆氓聡潞莽聨掳氓聹?Desktop app茫聙聛transport/API adapter 忙聢聳盲潞搂氓聯聛氓聟楼氓聫拢氓聭陆盲禄陇氓陇聳猫搂聜盲赂颅茂录聸Agent Runtime茫聙?
-  Tool primitives茫聙聛Harness茫聙聛Runtime Services contract 氓聮?Product Capabilities 盲赂聧氓戮聴盲戮聺猫碌聳 Tauri handle茫聙?
-  window茫聙聛command macro 忙聢?desktop app state茫聙?
-- Remote provider 氓驴聟茅隆禄忙聥聠氓聢聠莽篓鲁氓庐職猫驴聻忙聨楼忙聨楼氓聫拢氓聮聦氓聟路盲陆聯猫驴聹莽芦?OS / transport 氓庐聻莽聨掳茂录聦茅聛驴氓聟聧忙聤聤 SSH茫聙聛relay 忙聢聳猫驴聹莽芦炉氓鹿鲁氓聫掳氓路庐氓录聜忙鲁聞忙录聫氓聢掳 runtime茫聙?
-- 盲赂聧忙聰炉忙聦聛莽職聞猫聝陆氓聤聸氓聹?assembly 莽職?capability availability 盲赂颅忙聵戮氓录聫猫驴聰氓聸?unsupported / unavailable茂录聦盲赂聧氓聹?execution primitive 氓聠聟氓聠聶盲潞搂氓聯聛氓聢聠忙聰炉茫聙?
-- 莽娄聛忙颅垄盲陆驴莽聰篓忙聴聽莽卤禄氓聻?`Any` service locator茫聙聛氓聟篓氓卤聙 mutable registry 忙聢聳盲赂聥氓卤?crate 氓聫聧氓聬聭猫炉禄氓聫聳盲潞搂氓聯聛茅聟聧莽陆庐茫聙?
-
-## 9. 茅拢聨茅聶漏
-
-| 茅拢聨茅聶漏 | 盲驴聺忙聤陇忙聳鹿氓录聫 |
-|---|---|
-| 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录聣猫聠篓猫聝聙盲赂潞忙聳掳莽職聞氓聟篓氓卤聙莽聤露忙聙聛盲赂颅氓驴?| assembly 氓聫陋氓聛職忙聻聞氓禄潞忙聹聼忙鲁篓氓聠聦茂录聦猫戮聯氓聡潞盲赂聧氓聫炉氓聫?runtime parts茂录聸盲潞搂氓聯聛莽聤露忙聙聛盲禄聧氓陆?surface 忙聢?runtime owner |
-| 忙聨楼氓聫拢忙聥聠氓戮聴猫驴聡莽禄聠茂录聦氓炉录猫聡麓氓陇聧忙聺聜氓潞娄氓聮聦氓聤篓忙聙聛氓聢聠氓聫聭忙聢聬忙聹卢盲赂聤氓聧?| 盲禄?capability 氓聮聦莽篓鲁氓庐職莽聰篓盲戮聥氓庐職盲鹿?port 莽虏聮氓潞娄茂录聦莽聝颅猫路炉氓戮聞茅聛驴氓聟聧猫驴聬猫隆聦忙聴?map lookup茂录聦盲录聵氓聟?builder-time 忙鲁篓氓聟楼 |
-| 氓鹿鲁氓聫掳氓庐聻莽聨掳忙鲁聞忙录聫氓聢?Agent茫聙聛Tool 忙聢?Harness execution primitives | 盲戮聺猫碌聳忙拢聙忙聼楼莽娄聛忙颅?execution owner 盲戮聺猫碌聳 app crate茫聙聛Tauri茫聙聛CLI TUI茫聙聛ACP protocol 氓聮?concrete service crate |
-| core 忙聥聠氓聢聠氓聬聨盲禄聧茅職聬氓录聫莽禄聭氓庐職 Tauri | Tauri 氓聫陋氓聟聛猫庐赂氓聹篓 Desktop app 忙聢聳忙聵聨莽隆?feature-gated adapter茂录聸氓聬聭盲赂聥氓卤聜盲录聽茅聙?typed port茫聙聛DTO茫聙聛event fact 氓聮?capability availability |
-| 盲赂聧氓聬聦盲潞搂氓聯聛氓陆垄忙聙聛猫聝陆氓聤聸莽聼漏茅聵碌忙录聜莽搂?| Product Assembly 莽禄麓忙聤陇 capability matrix茂录聸氓聡聫氓掳聭忙聢聳忙聸驴忙聧垄猫聝陆氓聤聸忙聴露猫隆楼盲潞搂氓聯聛氓聟楼氓聫拢茅陋聦猫炉聛氓聮?unsupported 猫隆聦盲赂潞忙碌聥猫炉聲 |
-| Tool茫聙聛MCP茫聙聛ACP 莽職?manifest茫聙聛permission 忙聢聳盲潞聥盲禄露猫炉颅盲鹿聣忙聥聠猫搂拢氓聬聨盲赂聧莽颅聣盲禄?| 盲驴聺莽聲聶忙聴搂猫路炉氓戮聞氓聟录氓庐?facade茂录聦氓垄聻氓聤?manifest snapshot茫聙聛permission 氓聠鲁莽颅聳氓聮聦盲潞聥盲禄露忙聵聽氓掳聞莽颅聣盲禄路忙碌聥猫炉?|
-| Harness provider 氓聫陋氓聛職忙鲁篓氓聠聦盲陆聠猫垄芦猫炉炉猫庐陇盲赂潞氓路虏莽禄聫忙聥楼忙聹聣忙聣搂猫隆聦猫炉颅盲鹿?| descriptor-only / legacy-facade provider 氓聫陋猫聝陆莽聰聼忙聢聬 route plan茂录聸忙聣搂猫隆聦猫炉颅盲鹿聣莽搂禄氓聤篓氓驴聟茅隆禄氓聧聲莽聥卢猫炉聛忙聵聨猫隆聦盲赂潞莽颅聣盲禄?|
-| `northhing-core` 氓聫陋忙聵炉忙聰鹿氓聬聧盲赂潞忙聳掳莽職聞氓路篓氓聻?runtime crate | 忙聳?owner crate 氓驴聟茅隆禄忙聹聣氓聧聲盲赂聙猫聛聦猫麓拢氓聮聦忙聹聙氓掳聫盲戮聺猫碌聳茂录聸盲潞搂氓聯聛猫聝陆氓聤聸茫聙聛harness茫聙聛service 氓庐聻莽聨掳盲赂聧氓戮聴莽禄搂莽禄颅氓聽聠氓聟楼 agent kernel |
-| 莽聸庐忙聽聡 crate 氓聟聢猫隆聦氓聢聸氓禄潞盲陆聠忙虏隆忙聹聣莽聹聼氓庐?owner | 氓聫陋忙聹聣 owner 猫戮鹿莽聲聦茫聙聛忙聴搂猫路炉氓戮聞氓聟录氓庐鹿茫聙聛focused tests茫聙聛盲戮聺猫碌聳忙聰露莽聸聤氓聮聦 boundary check 氓聬聦忙聴露忙聢聬莽芦聥忙聴露忙聣聧氓聢聸氓禄潞 crate茂录聸氓聬娄氓聢聶莽禄搂莽禄颅莽聲聶氓聹?facade |
-
-## 10. 莽聸庐忙聽聡莽聤露忙聙聛氓聢陇氓庐?
-
-- `northhing-core` 盲赂聧氓聠聧忙聵炉盲潞聥氓庐聻盲赂聤莽職聞氓庐聦忙聲?runtime owner茂录聦猫聙聦忙聵炉氓聟录氓庐鹿 facade 氓聮?`product-full` 莽禄聞猫拢聟猫戮鹿莽聲聦茫聙?
-- Agent Runtime SDK 氓聫炉氓聹篓盲赂聧盲戮聺猫碌?`northhing-core`茫聙聛app crate 忙聢?Tauri 莽職聞忙聝聟氓聠碌盲赂聥猫垄芦氓碌聦氓聟楼茂录聦氓鹿露茅聙職猫驴聡莽篓鲁氓庐職 builder /
-  runner / event stream / registry API 忙聫聬盲戮聸 agent 猫聝陆氓聤聸茫聙?
-- Agent Runtime茫聙聛Tool Contracts / Tool Provider Groups / Tool Execution茫聙聛Runtime Services茫聙聛Harness 氓聮?Product Capabilities 氓聢聠氓聢芦忙聥楼忙聹聣氓聫炉氓庐隆忙聼楼莽職聞猫聛聦猫麓拢猫戮鹿莽聲聦茫聙?
-- 莽篓鲁氓庐職氓楼聭莽潞娄氓聮聦氓聬聞 execution owner 氓庐職盲鹿聣忙聨楼氓聫拢茂录聸氓聟路盲陆?Tool茫聙聛OS茫聙聛Remote service 莽聲聶氓聹篓 Services茂录聦氓聧聫猫庐庐氓聮聦氓陇聳茅聝篓 provider 猫陆卢忙聧垄莽聲聶氓聹篓 Adapters茫聙?
-- 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录聣忙聵炉氓聰炉盲赂聙忙鲁篓氓聠聦莽聜鹿茂录聦茅聙職猫驴聡 typed builder / registry 猫驴聻忙聨楼忙聨楼氓聫拢氓聮聦氓聟路盲陆聯氓庐聻莽聨掳茫聙?
-- Tauri 氓聫陋氓卤聻盲潞?Desktop app 忙聢聳忙聵聨莽隆?feature-gated adapter茂录聦盲赂聧猫驴聸氓聟楼 core茫聙聛execution owner 忙聢?contract crate茫聙?
-- runtime 氓聫陋盲戮聺猫碌?remote connection茫聙聛remote workspace茫聙聛remote projection 氓聮?capability facts 莽颅?port茂录聸SSH茫聙聛relay茫聙?
-  忙聹卢氓聹掳茅職搂茅聛聯茫聙聛猫驴聹莽芦?OS 氓路庐氓录聜氓聮聦猫庐陇猫炉聛忙聳鹿氓录聫氓卤聻盲潞聨氓聟路盲陆?Remote provider茫聙?
-- 盲潞搂氓聯聛氓陆垄忙聙聛氓路庐氓录聜茅聙職猫驴聡 capability matrix 氓聮?Product Assembly 猫隆篓猫戮戮茂录聦盲赂聧茅聙職猫驴聡盲赂聥忙虏聣 UI茫聙聛氓聭陆盲禄陇茫聙聛氓聧聫猫庐庐忙聢聳氓鹿鲁氓聫掳氓庐聻莽聨掳猫隆篓猫戮戮茫聙?
-- 忙聺聝茅聶聬茫聙聛氓路楼氓聟路忙聸聺氓聟聣茫聙聛盲潞聥盲禄露茫聙聛session茫聙聛remote workspace 氓聮?release 忙聻聞氓禄潞氓陆垄忙聙聛氓驴聟茅隆禄盲驴聺忙聦聛氓聤聼猫聝陆莽颅聣盲禄路茫聙?
+﻿> **v0.1.0 status note**: This document has encoding issues (GBK/UTF-8 mojibake) and references src/web-ui/ which is [missing] in v0.1.0. Preserved for historical reference only.
+
+# northhing Core 忙聥聠猫搂拢忙聻露忙聻聞
+
+忙聹卢忙聳聡忙娄聜忙聥卢 northhing core runtime 忙聥聠猫搂拢莽職聞盲赂陇盲赂陋莽篓鲁氓庐職猫庐戮猫庐隆莽禄麓氓潞娄茂录職**氓聢聺氓搂聥莽聤露忙聙?*氓聮?*莽聸庐忙聽聡莽聤露忙聙?*茫聙?
+氓聢聺氓搂聥莽聤露忙聙聛忙聫聫猫驴掳猫庐戮猫庐隆氓禄潞莽芦聥忙聴露莽職聞盲潞聥氓庐聻忙聻露忙聻聞茫聙聛猫聙娄氓聬聢氓聟鲁莽鲁禄氓聮聦盲赂禄猫娄聛茅聴庐茅垄聵茂录聸莽聸庐忙聽聡莽聤露忙聙聛忙聫聫猫驴掳忙聹聼忙聹聸氓聢聠氓卤聜茫聙聛莽篓鲁氓庐職忙聨楼氓聫拢茫聙?
+氓庐聻莽聨掳氓陆聮氓卤聻茫聙聛莽禄聞猫拢聟猫戮鹿莽聲聦茫聙聛盲戮聺猫碌聳忙聳鹿氓聬聭氓聮聦茅拢聨茅聶漏莽潞娄忙聺聼茫聙?
+
+忙聹卢忙聳聡猫聛職莽聞娄猫庐戮猫庐隆莽禄聯猫庐潞茫聙聜猫炉娄莽禄聠忙聨楼氓聫拢茫聙聛crate 氓聠聟茅聝篓忙篓隆氓聺聴氓聮聦忙碌聥猫炉聲猫庐戮猫庐隆猫搂聛
+[`agent-runtime-services-design.md`](agent-runtime-services-design.md)茫聙?
+
+## 1. 猫聝聦忙聶炉盲赂聨莽聸庐忙聽?
+
+猫庐戮猫庐隆氓禄潞莽芦聥忙聴露茂录聦northhing 氓路虏莽禄聫盲禄?`northhing-core` 盲赂颅忙聤陆氓聡潞盲潞聠猫聥楼氓鹿虏 owner crate茂录聦盲陆聠 `northhing-core` 盲禄聧忙聣驴忙聥聟氓聟录氓庐?facade茫聙?
+氓庐聦忙聲麓盲潞搂氓聯聛 runtime 莽禄聞猫拢聟茫聙聛agent loop茫聙聛service 忙聨楼莽潞驴茫聙聛tool materialization 氓聮聦茅聝篓氓聢?product domain
+adapter茫聙聜猫驴聶盲赂陋氓陆垄忙聙聛氓聹篓氓聤聼猫聝陆盲赂聤氓聫炉猫驴聬猫隆聦茂录聦盲陆聠盲录職猫庐漏 runtime 忙聥聠猫搂拢忙聦聛莽禄颅茅聺垄盲赂麓盲赂聣盲赂陋茅聴庐茅垄聵茂录?
+
+- 盲潞搂氓聯聛茅聙禄猫戮聭茫聙聛氓鹿鲁氓聫掳忙聨楼氓聟楼氓聮聦氓聟路盲陆聯 service 氓庐聻莽聨掳猫戮鹿莽聲聦盲赂聧氓陇聼莽篓鲁氓庐職茫聙?
+- Desktop茫聙聛CLI茫聙聛Server茫聙聛Remote茫聙聛ACP茫聙聛Web 莽颅聣盲潞搂氓聯聛氓陆垄忙聙聛氓庐鹿忙聵聯猫垄芦氓庐聦忙聲麓 `northhing-core` 莽聣碌氓录聲茫聙?
+- Tool茫聙聛MCP茫聙聛ACP茫聙聛subagent茫聙聛skills 莽颅聣忙聣漏氓卤聲莽聜鹿莽录潞氓掳聭莽禄聼盲赂聙莽職聞氓聢聠氓卤聜氓陆聮氓卤聻茫聙?
+
+莽聸庐忙聽聡氓陆垄忙聙聛盲赂聧忙聵炉氓聹篓 `northhing-core` 氓聠聟莽禄搂莽禄颅忙聣漏氓录聽氓庐聦忙聲?`AgentRuntime`茂录聦猫聙聦忙聵炉氓陆垄忙聢聬氓聫炉莽聥卢莽芦聥氓碌聦氓聟楼莽職聞
+Agent Runtime SDK茫聙聜莽篓鲁氓庐職氓楼聭莽潞娄氓庐職盲鹿聣盲赂聤氓卤聜氓聫炉盲戮聺猫碌聳莽職聞忙聨楼氓聫拢茂录聦Product Assembly 猫麓聼猫麓拢忙鲁篓氓聠聦氓聟路盲陆聯氓庐聻莽聨掳茂录?
+Runtime Services 氓聮?Tool primitives 氓聢聠氓聢芦茅職聰莽娄禄 service茫聙聛tool 氓聮聦盲潞搂氓聯聛氓陆垄忙聙聛氓路庐氓录聜茫聙?
+
+Agent Runtime SDK 氓聹篓忙聹卢忙聳聡盲赂颅盲赂聧忙聵炉忙聼聬盲赂陋 crate 莽職聞莽庐聙氓聧聲茅聡聧氓聭陆氓聬聧茂录聦猫聙聦忙聵炉盲赂聙莽禄聞氓聫炉氓炉鹿氓陇聳莽篓鲁氓庐職忙聣驴猫炉潞莽職聞猫驴聬猫隆聦忙聴露猫聝陆氓聤聸猫戮鹿莽聲聦茫聙?
+莽聸庐忙聽聡莽聤露忙聙聛盲赂聥茂录聦猫掳聝莽聰篓忙聳鹿氓潞聰猫聝陆茅聙職猫驴聡莽篓鲁氓庐職 API 氓聢聸氓禄潞 runtime茫聙聛忙聫聬盲潞?turn茫聙聛忙露聢猫麓鹿盲潞聥盲禄露忙碌聛茫聙聛忙鲁篓氓聠?tool / service
+provider茫聙聛氓陇聞莽聬?permission / cancellation / persistence / telemetry茂录聦猫聙聦盲赂聧茅聹聙猫娄聛盲戮聺猫碌?`northhing-core`茫聙聛app crate茫聙?
+Tauri handle 忙聢聳盲禄禄盲陆聲盲潞搂氓聯聛氓陆垄忙聙聛莽職聞 concrete manager茫聙聜氓聹篓猫炉楼莽聸庐忙聽聡猫戮戮忙聢聬氓聣聧茂录聦`execution` 氓卤聜氓聫陋猫聝陆莽搂掳盲赂潞忙聣搂猫隆聦氓聨聼猫炉颅茅聸聠氓聬聢茂录聦
+盲赂聧猫聝陆氓炉鹿氓陇聳氓庐拢莽搂掳盲赂潞氓庐聦忙聲?SDK茫聙?
+
+莽聸庐忙聽聡莽聤露忙聙聛氓驴聟茅隆禄盲驴聺忙聦聛盲潞搂氓聯聛猫隆聦盲赂潞茫聙聛茅禄聵猫庐陇猫聝陆氓聤聸茅聸聠氓聬聢茫聙聛忙聺聝茅聶聬猫炉颅盲鹿聣茫聙聛氓路楼氓聟路忙聸聺氓聟聣茫聙聛盲潞聥盲禄露猫炉颅盲鹿聣氓聮聦 release 忙聻聞氓禄潞氓陆垄忙聙聛莽颅聣盲禄路茫聙?
+
+## 2. 忙聻露忙聻聞氓聨聼氓聢聶
+
+- 盲戮聺猫碌聳氓聫陋猫聝陆盲禄聨盲潞搂氓聯聛氓聟楼氓聫?/ 盲潞搂氓聯聛莽禄聞猫拢聟忙碌聛氓聬聭盲潞搂氓聯聛猫聝陆氓聤聸茫聙聛氓聟路盲陆聯茅聙聜茅聟聧茫聙聛忙聹聧氓聤隆氓聮聦忙聣搂猫隆聦氓聨聼猫炉颅茂录聦氓聠聧忙碌聛氓聬聭莽篓鲁氓庐職氓楼聭莽潞娄茂录聸盲赂聥氓卤聜盲赂聧氓戮聴忙聞聼莽聼楼盲赂聤氓卤聜盲潞搂氓聯聛氓陆垄忙聙聛茫聙?
+- 忙聨楼氓聫拢氓聮聦氓庐聻莽聨掳氓驴聟茅隆禄氓聢聠氓录聙茂录職忙聨楼氓聫拢氓卤聻盲潞聨莽篓鲁氓庐職氓楼聭莽潞娄茫聙聛Runtime Services 忙聢?Tool primitives contract茂录?
+  氓聟路盲陆聯氓庐聻莽聨掳氓卤聻盲潞聨 Product Assembly 莽職聞忙鲁篓氓聠聦猫戮鹿莽聲聦茫聙聛Adapters 忙聢?Services茫聙?
+- Product interface 氓聫炉盲禄楼忙聹聣氓路庐氓录聜茂录聦capability contract 氓驴聟茅隆禄忙聰露忙聲聸茫聙聜盲赂聧氓聬聦盲潞搂氓聯聛氓聟楼氓聫拢氓聫炉盲禄楼茅聙聣忙聥漏盲赂聧氓聬聦猫聝陆氓聤聸茅聸聠氓聬聢茂录?
+  盲陆聠盲赂聧猫聝陆茅聙職猫驴聡盲赂聥忙虏聣 UI茫聙聛氓聭陆盲禄陇忙聢聳氓聧聫猫庐庐茅聙禄猫戮聭忙聺楼忙聧垄氓聫聳氓陇聧莽聰篓茫聙?
+- `northhing-core` 盲驴聺莽聲聶氓聟录氓庐鹿 facade 氓聮?`product-full` 莽禄聞猫拢聟猫戮鹿莽聲聦茂录聸忙聳掳 owner crate 盲赂聧氓戮聴盲戮聺猫碌聳氓聸?
+  `northhing-core`茫聙?
+- 氓炉鹿氓陇聳 SDK API 氓驴聟茅隆禄忙聵炉莽篓鲁氓庐職茫聙聛莽陋聞氓聫拢氓戮聞茫聙聛氓聫炉莽聣聢忙聹卢氓聦聳莽職聞 fa脙搂ade茂录聦盲赂聧氓戮聴忙聤聤 `northhing-core`茫聙聛`product-full`茫聙聛氓聟篓茅聡?
+  service bundle 忙聢聳盲潞搂氓聯聛氓聠聟茅聝?manager 忙職麓茅聹虏莽禄聶猫掳聝莽聰篓忙聳鹿茫聙?
+- Hook 忙聵炉氓聫聴忙聨搂忙聣漏氓卤聲莽聜鹿茂录聦Event 忙聵炉盲潞聥氓庐聻茅聙職莽聼楼茫聙聜猫聝陆忙聰鹿氓聫聵猫隆聦盲赂潞莽職?hook 氓驴聟茅隆禄忙聹聣茅隆潞氓潞聫茫聙聛timeout茫聙聛茅聰聶猫炉炉莽颅聳莽聲楼氓聮聦莽颅聣盲禄路盲驴聺忙聤陇茫聙?
+- feature group 忙聵炉忙聻聞氓禄潞猫戮鹿莽聲聦茂录聦CapabilitySet 忙聵炉盲潞搂氓聯聛猫驴聬猫隆聦忙聴露猫聝陆氓聤聸猫戮鹿莽聲聦茂录聸盲赂陇猫聙聟氓驴聟茅隆禄莽聰卤 Product Assembly
+  忙聵戮氓录聫忙聵聽氓掳聞茫聙?
+
+## 3. 氓聢聺氓搂聥莽聤露忙聙聛茅聙禄猫戮聭猫搂聠氓聸戮
+
+氓聢聺氓搂聥莽聤露忙聙聛莽職聞忙聽赂氓驴聝盲潞聥氓庐聻忙聵炉茂录職氓陇職盲赂陋 crate 氓路虏莽禄聫忙聣驴忙聨楼盲潞聠莽篓鲁氓庐職莽卤禄氓聻聥茫聙聛盲潞聥盲禄露茫聙聛stream茫聙聛tool contract茫聙聛茅聝篓氓聢?service
+helper 氓聮?product domain 莽潞炉茅聙禄猫戮聭茂录聦盲陆聠氓庐聦忙聲麓猫驴聬猫隆聦忙聴露盲禄聧盲禄?`northhing-core` 盲赂潞盲赂颅氓驴聝茫聙?
+
+```mermaid
+flowchart TB
+  Surfaces["盲潞搂氓聯聛氓聟楼氓聫拢<br/>Desktop / CLI / Server / Relay / Remote / Web"]
+  Core["northhing-core<br/>氓聟录氓庐鹿 facade + 氓庐聦忙聲麓盲潞搂氓聯聛 runtime 莽禄聞猫拢聟"]
+  Acp["northhing-acp<br/>ACP protocol surface / client behavior"]
+  Transport["transport / api-layer<br/>API 盲赂聨盲录聽猫戮?adapter"]
+  CoreTypes["northhing-core-types<br/>莽篓鲁氓庐職 DTO 氓颅聬茅聸聠"]
+  Events["northhing-events<br/>盲潞聥盲禄露盲潞聥氓庐聻盲赂?emitter 忙聤陆猫卤隆"]
+  Ports["northhing-runtime-ports<br/>trait-only runtime 猫戮鹿莽聲聦"]
+  Stream["northhing-agent-stream<br/>stream 猫聛職氓聬聢"]
+  AgentTools["northhing-agent-tools<br/>tool contract 盲赂聨莽潞炉莽颅聳莽聲楼"]
+  ToolRuntime["tool-execution<br/>tool-runtime package / 盲陆聨氓卤聜 helper"]
+
+  ServicesCore["northhing-services-core<br/>氓聼潞莽隆聙 service helper / filesystem facade"]
+  ServicesIntegrations["northhing-services-integrations<br/>MCP / Git / Remote helper owner"]
+  ProductDomains["northhing-product-domains<br/>MiniApp / function-agent 莽潞?domain"]
+  Terminal["terminal-core<br/>terminal domain"]
+  Ai["northhing-ai-adapters<br/>忙篓隆氓聻聥 provider adapter"]
+  External["氓陇聳茅聝篓莽鲁禄莽禄聼<br/>OS / Git / MCP / ACP / AI provider / remote host"]
+
+  Surfaces --> Core
+  Surfaces --> Transport
+  Surfaces --> Acp
+  Acp --> Core
+  Core --> CoreTypes
+  Core --> Events
+  Core --> Ports
+  Core --> Stream
+  Core --> AgentTools
+  Core --> ToolRuntime
+
+  Core --> ServicesCore
+  Core --> ServicesIntegrations
+  Core --> ProductDomains
+  Core --> Terminal
+  Core --> Ai
+  Core --> Transport
+  ServicesCore --> External
+  ServicesIntegrations --> External
+  Terminal --> External
+  Ai --> External
+```
+
+氓聢聺氓搂聥莽聤露忙聙聛盲赂禄猫娄聛忙篓隆氓聺聴猫聦聝氓聸麓茂录職
+
+| 忙篓隆氓聺聴 | 氓聢聺氓搂聥氓庐職盲陆聧 | 忙聻露忙聻聞氓陆卤氓聯聧 |
+|---|---|---|
+| `northhing-core` | 氓聟录氓庐鹿 facade茫聙聛agent runtime茫聙聛tool runtime 莽禄聞猫拢聟茫聙聛service 忙聨楼莽潞驴氓聮聦氓庐聦忙聲麓盲潞搂氓聯聛猫聝陆氓聤聸茅聸聠氓聬?| 盲禄聧忙聵炉盲潞聥氓庐聻盲赂聤莽職聞 runtime owner茂录聦忙聥聠猫搂拢氓驴聟茅隆禄氓聟聢盲驴聺忙聤陇猫隆聦盲赂潞莽颅聣盲禄路 |
+| `northhing-runtime-ports` | 茅聺垄氓聬聭 runtime/service 猫戮鹿莽聲聦莽職?DTO 氓聮?trait | 氓聫陋氓庐職盲鹿?contract茂录聦盲赂聧忙聥楼忙聹聣 runtime 氓庐聻莽聨掳 |
+| `tool-contracts` / `northhing-agent-tools` | provider-neutral tool DTO茫聙聛manifest茫聙聛path/result policy茫聙聛catalog contract 氓聮?deterministic execution admission gate | 茅聙聜氓聬聢忙聣驴忙聨楼莽潞?tool contract 莽颅聳莽聲楼茂录聦盲陆聠盲赂聧氓潞聰忙聥楼忙聹聣氓聟路盲陆聯 IO tool |
+| `tool-execution` / `tool-runtime` | 忙聴垄忙聹聣盲陆聨氓卤聜氓路楼氓聟路忙聣搂猫隆聦 helper crate | 莽聸庐忙聽聡忙聵炉氓聫陋忙聣驴忙聨楼盲陆聨氓卤聜 file/search/tool execution helper茂录聦盲赂聧忙聥楼忙聹聣盲潞搂氓聯聛 registry 忙聢?permission policy |
+| `northhing-services-core` | 氓聼潞莽隆聙 service helper茫聙聛忙聹卢氓聹?filesystem facade茫聙聛茅聝篓氓聢聠茅聙職莽聰篓 service 茅聙禄猫戮聭 | 茅聙聜氓聬聢盲陆聹盲赂潞忙聹卢氓聹掳氓聼潞莽隆聙 service owner茂录聦盲陆聠盲赂聧猫聝陆氓聬赂忙聰露盲潞搂氓聯聛 runtime 猫炉颅盲鹿聣 |
+| `northhing-services-integrations` | MCP茫聙聛Git茫聙聛remote-connect茫聙聛remote-SSH 莽颅?integration helper | 茅聙聜氓聬聢忙聥楼忙聹聣氓陇聳茅聝篓氓聧聫猫庐庐氓聮聦茅聡聧盲戮聺猫碌聳 service implementation茂录聦盲赂聧氓潞聰氓聫聧氓聬聭忙聞聼莽聼楼盲潞搂氓聯?interface |
+| `northhing-product-domains` | MiniApp茫聙聛function-agent 莽颅聣莽潞炉莽聤露忙聙聛茫聙聛莽颅聳莽聲楼茫聙聛port 氓聮聦茅聝篓氓聢聠氓聠鲁莽颅聳茅聙禄猫戮聭 | 茅聙聜氓聬聢忙聣驴忙聨楼 pure domain茂录聦盲赂聧氓潞聰莽聸麓忙聨楼忙聣搂猫隆?filesystem/Git/AI concrete call |
+| `northhing-acp` | ACP protocol interface 氓聮?client behavior | 氓潞聰盲驴聺忙聦聛盲潞搂氓聯聛氓聧聫猫庐庐氓聟楼氓聫拢茂录聦盲赂聧盲赂聥忙虏聣氓聢掳 Agent Runtime |
+| `transport` / `api-layer` | surface 氓聢?runtime 莽職?API/transport adapter | 氓潞聰盲驴聺忙聦聛盲录聽猫戮聯氓卤聜茂录聦盲赂聧忙聥楼忙聹聣 runtime owner |
+
+## 4. 氓聢聺氓搂聥莽聤露忙聙聛盲赂禄猫娄聛茅聴庐茅垄?
+
+### 4.1 氓聢聠氓卤聜盲赂聧忙赂聟忙聶?
+
+氓聬聦盲赂聙猫聝陆氓聤聸莽禄聫氓赂赂氓聬聦忙聴露氓聦聟氓聬芦 UI/command茫聙聛runtime orchestration茫聙聛tool execution茫聙聛service IO 氓聮?domain
+decision茫聙聜氓聢聺氓搂聥莽聤露忙聙聛盲禄拢莽聽聛盲赂颅猫驴聶盲潞聸茅聝篓氓聢聠盲禄聧氓陇搂茅聡聫茅聙職猫驴聡 `northhing-core` 盲赂虏猫聛聰茂录聦氓炉录猫聡麓忙聥聠猫搂拢忙聴露茅職戮盲禄楼氓聢陇忙聳颅芒聙聹莽搂禄氓聤篓莽職聞忙聵炉忙聨楼氓聫拢茫聙?
+氓庐聻莽聨掳茫聙聛莽禄聞猫拢聟茅聙禄猫戮聭猫驴聵忙聵炉盲潞搂氓聯聛猫隆聦盲赂潞芒聙聺茫聙?
+
+### 4.2 忙聨楼氓聫拢盲赂聨氓庐聻莽聨掳猫戮鹿莽聲聦盲赂聧莽篓鲁氓庐職
+
+氓路虏忙聹聣 `runtime-ports` 氓聮聦猫聥楼氓鹿?contract crate茂录聦盲陆聠猫庐赂氓陇職 call site 盲禄聧盲戮聺猫碌?concrete manager茫聙?
+core-owned context 忙聢聳氓庐聦忙聲?product runtime snapshot茫聙聜忙聨楼氓聫拢忙虏隆忙聹聣莽篓鲁氓庐職氓聢掳猫露鲁盲禄楼猫庐?runtime 盲赂聨氓聟路盲陆?service
+氓庐聻莽聨掳莽聥卢莽芦聥忙录聰猫驴聸茫聙?
+
+### 4.3 盲潞搂氓聯聛氓陆垄忙聙聛猫垄芦氓庐聦忙聲麓 core 莽聣碌氓录聲
+
+Desktop茫聙聛CLI茫聙聛Server茫聙聛Remote茫聙聛ACP 氓聮?Web 莽職聞氓聟楼氓聫拢氓路庐氓录聜猫戮聝氓陇搂茂录聦盲陆聠氓聢聺氓搂聥莽聤露忙聙聛盲赂聥氓陇搂氓陇職盲禄聧茅聙職猫驴聡氓庐聦忙聲麓 `northhing-core`
+猫聨路氓戮聴猫聝陆氓聤聸茫聙聜猫驴聶盲录職猫庐漏猫陆禄茅聡聫盲潞陇盲禄聵氓陆垄忙聙聛莽禄搂忙聣驴盲赂聧氓驴聟猫娄聛莽職?tool茫聙聛service茫聙聛UI 忙聢聳氓鹿鲁氓聫掳盲戮聺猫碌聳茫聙?
+
+### 4.4 Tool contract 盲赂?tool execution 忙路路氓聬聢
+
+provider-neutral manifest茫聙聛path policy茫聙聛result policy茫聙聛`ToolUseContext` runtime handle茫聙聛collapsed unlock
+lifecycle茫聙聛runtime artifact persistence 氓聮?product registry materialization 氓聹篓氓聢聺氓搂聥莽聤露忙聙聛盲赂聥盲赂?concrete tool
+execution 盲潞陇莽禄聡氓聹?core 氓聫聤氓聟露氓聟录氓庐鹿猫路炉氓戮聞盲赂颅茫聙聜莽聸庐忙聽聡莽聤露忙聙聛盲赂聥茂录聦tool contracts 氓潞聰忙聥楼忙聹?provider-neutral manifest /
+catalog / permission / result / artifact contract茂录聦core茫聙聛services 忙聢?adapter 氓聫陋盲驴聺莽聲聶氓庐聻茅聶?IO tool adapter茫聙?
+state update茫聙聛忙聴搂猫路炉氓戮聞 facade 氓聮聦忙聹聣莽颅聣盲禄路盲驴聺忙聤陇莽職聞忙聥聠猫搂拢猫戮鹿莽聲聦茫聙聜氓路楼氓聟?owner 忙聥聠猫搂拢氓娄聜忙聻聹忙虏隆忙聹聣氓驴芦莽聟搂盲驴聺忙聤陇茂录聦氓庐鹿忙聵聯忙聰鹿氓聫?
+prompt-visible manifest茫聙聛`GetToolSpec`茫聙聛MCP/ACP catalog 忙聢?oversized result 猫隆聦盲赂潞茫聙?
+
+### 4.5 Service茫聙聛MCP茫聙聛ACP 盲赂?runtime kernel 氓庐鹿忙聵聯盲潞陇氓聫聣
+
+MCP 氓聮?ACP 忙聵炉氓陇聳茅聝篓氓聧聫猫庐?猫聝陆氓聤聸忙聨楼氓聟楼茂录聦盲赂聧氓潞聰氓聫聵忙聢?Agent Runtime SDK 莽職聞氓聠聟茅聝篓氓聧聫猫庐庐盲戮聺猫碌聳茫聙聜Runtime kernel 氓聫陋氓潞聰莽聹聥猫搂聛
+external capability茫聙聛tool provider 忙聢?service port茂录聸猫驴聻忙聨楼莽聰聼氓聭陆氓聭篓忙聹聼茫聙聛茅聣麓忙聺聝茫聙聛transport 氓聮?timeout 莽颅聳莽聲楼氓潞聰莽聰卤
+Adapters茫聙聛Services 忙聢?Product Assembly 莽庐隆莽聬聠茫聙?
+
+### 4.6 忙聣漏氓卤聲莽聜鹿莽录潞氓掳聭莽禄聼盲赂聙猫炉颅盲鹿聣
+
+agent definitions茫聙聛subagents茫聙聛skills茫聙聛prompt modules茫聙聛tool providers茫聙聛MCP providers茫聙聛hooks 氓聮?
+product commands 茅聝陆忙聵炉忙聣漏氓卤聲莽聜鹿茂录聦盲陆聠莽聸庐氓聣聧忙虏隆忙聹聣莽禄聼盲赂聙猫隆篓猫戮戮氓庐聝盲禄卢氓聢聠氓聢芦氓卤聻盲潞聨氓聯陋盲赂聙氓卤聜茫聙聛氓娄聜盲陆聲忙鲁篓氓聠聦茫聙聛忙聵炉氓聬娄氓聟聛猫庐赂忙聰鹿氓聫聵猫隆聦盲赂潞茫聙?
+盲禄楼氓聫聤氓娄聜盲陆聲氓聛職忙聺聝茅聶聬氓聮聦忙碌聥猫炉聲盲驴聺忙聤陇茫聙?
+
+### 4.7 feature graph 猫驴聵盲赂聧忙聵炉盲潞搂氓聯聛猫聝陆氓聤聸莽聼漏茅聵?
+
+氓聢聺氓搂聥莽聤露忙聙聛盲赂聥茂录聦`product-full` 忙聵炉氓庐聦忙聲麓盲潞搂氓聯聛猫聝陆氓聤聸莽職聞氓庐聣氓聟篓莽陆聭茂录聦盲赂聧忙聵炉忙聹聙莽禄聢忙聦聣盲潞搂氓聯聛忙聥聠氓聢聠莽職?feature matrix茫聙聜莽聸麓忙聨楼氓聡聫猫陆禄茅禄聵猫庐?feature
+忙聢聳忙聤聤 feature group 氓陆聯忙聢聬盲潞搂氓聯聛猫聝陆氓聤聸猫戮鹿莽聲聦茂录聦茅聝陆盲录職氓录聲氓聟楼忙聻聞氓禄潞氓陆垄忙聙聛氓聮聦氓聫聭氓赂聝猫聝陆氓聤聸忙录聜莽搂禄茫聙?
+
+### 4.8 忙聻聞氓禄潞盲赂聨忙碌聥猫炉聲莽聣碌氓录聲猫驴聡氓陇?
+
+茅聡聧盲戮聺猫碌聳氓聮聦氓庐聦忙聲麓 runtime 猫聛職氓聬聢氓聹?`northhing-core` 氓聭篓氓聸麓茂录聦氓炉录猫聡麓氓卤聙茅聝篓忙碌聥猫炉聲茫聙聛owner crate 忙碌聥猫炉聲氓聮聦猫陆禄茅聡聫盲潞搂氓聯聛氓聟楼氓聫拢氓庐鹿忙聵聯猫垄芦
+盲赂聧莽聸赂氓聟鲁盲戮聺猫碌聳忙聥聳氓聟楼莽录聳猫炉聭氓聮聦茅聯戮忙聨楼猫路炉氓戮聞茫聙聜莽聸庐忙聽聡莽聤露忙聙聛氓驴聟茅隆禄猫庐漏盲戮聺猫碌聳忙聰露莽聸聤氓聫炉氓潞娄茅聡聫茂录聦氓聬聦忙聴露盲赂聧猫聝陆盲禄楼莽聣潞莽聣虏氓聤聼猫聝陆莽颅聣盲禄路忙聧垄氓聫聳忙聻聞氓禄潞忙聰露莽聸聤茫聙?
+
+### 4.9 SDK 氓聫聭氓赂聝猫戮鹿莽聲聦盲赂聧猫露鲁
+
+氓路虏忙聹聣 `northhing-agent-runtime`茫聙聛`northhing-runtime-services`茫聙聛`tool-contracts` 氓聮?`tool-execution`
+氓聮?`runtime-ports` 莽颅?SDK 氓聙聶茅聙聣氓聨聼猫炉颅茂录聦盲陆聠莽录潞氓掳聭氓聫炉氓炉鹿氓陇聳忙聣驴猫炉潞莽職聞莽禄聼盲赂聙 runtime fa脙搂ade茫聙聛莽篓鲁氓庐職茅聰聶猫炉炉忙篓隆氓聻聥茫聙聛盲潞聥盲禄露忙碌聛氓聧聫猫庐庐茫聙?
+provider 忙鲁篓氓聠聦猫戮鹿莽聲聦茫聙聛忙聦聛盲鹿聟氓聦聳/忙聛垄氓陇聧氓楼聭莽潞娄氓聮聦忙聹聙氓掳聫盲戮聺猫碌聳忙聻聞氓禄潞氓陆垄忙聙聛茫聙聜氓娄聜忙聻聹氓陇聳茅聝篓猫掳聝莽聰篓忙聳鹿盲禄聧茅聹聙猫娄聛莽聸麓忙聨楼莽聬聠猫搂?`northhing-core`茫聙?
+`product-full`茫聙聛concrete service manager 忙聢聳盲潞搂氓聯聛氓聭陆盲禄陇猫路炉氓戮聞茂录聦猫炉麓忙聵聨 SDK 猫戮鹿莽聲聦氓掳職忙聹陋氓庐聦忙聢聬茫聙?
+
+## 5. 氓炉鹿莽聟搂氓聢聠忙聻聬
+
+忙聹卢猫聤聜氓聫陋忙聫聬莽聜录氓炉鹿 northhing 氓聢聠氓卤聜忙聹聣莽聰篓莽職聞忙聻露忙聻聞盲驴隆氓聫路茂录聦盲赂聧忙聤聤氓聟露盲禄聳茅隆鹿莽聸庐莽職聞氓庐聻莽聨掳氓陆垄忙聙聛莽聸麓忙聨楼氓陇聧氓聢露氓聢掳 northhing茫聙?
+
+### 5.1 Claude Code 莽聸赂氓聟鲁氓庐聻莽聨掳氓聫聜猫聙?
+
+Claude Code 莽聸赂氓聟鲁 Rust 氓庐聻莽聨掳氓聫聜猫聙聝盲赂颅茂录聦workspace 氓掳?CLI binary茫聙聛provider API茫聙聛runtime茫聙聛tools茫聙?
+commands茫聙聛plugins 氓聮?telemetry 忙聥聠忙聢聬盲赂聧氓聬聦 crate茫聙聜氓聟露 `runtime` 猫麓聼猫麓拢 session茫聙聛config茫聙?
+permission茫聙聛MCP茫聙聛prompt 氓聮?runtime loop茂录聸`tools` 猫麓聼猫麓拢 tool specs 盲赂聨忙聣搂猫隆聦茂录聸`commands` 猫麓聼猫麓拢 slash command
+registry茂录聸`plugins` 猫麓聼猫麓拢 plugin metadata茫聙聛hook 氓聮?install/enable/disable surfaces茫聙聜猫炉楼莽禄聯忙聻聞猫炉麓忙聵聨茂录?
+
+- 氓路楼氓聟路猫搂聞忙聽录茫聙聛氓聭陆盲禄?surface茫聙聛plugin/hook 氓聮?runtime loop 氓聫炉盲禄楼氓聢聠氓录聙忙录聰猫驴聸茫聙?
+- permission茫聙聛MCP lifecycle茫聙聛task registry茫聙聛LSP registry 莽颅聣氓聫炉盲陆聹盲赂潞 runtime/service owner 莽庐隆莽聬聠茂录聦猫聙聦盲赂聧忙聵炉忙聲拢猫聬陆氓聹篓 UI茫聙?
+- 氓娄聜忙聻聹 runtime crate 氓聬聦忙聴露氓聬赂忙聰露 session茫聙聛MCP茫聙聛permission茫聙聛prompt 氓聮?tool bridge茂录聦盲鹿聼盲录職氓聫聵忙聢聬忙聳掳莽職聞茅聡聧猫聛職氓聬聢莽聜鹿茫聙?
+
+忙聙禄莽禄聯茂录職忙聥聠氓聢?crate 盲赂聧忙聵炉莽聸庐忙聽聡忙聹卢猫潞芦茂录聦氓聟鲁茅聰庐忙聵炉猫庐?CLI/TUI茫聙聛commands茫聙聛tools茫聙聛plugins茫聙聛runtime 氓聮?
+service integrations 茅聙職猫驴聡莽篓鲁氓庐職 contract 莽禄聞氓聬聢茂录聦茅聛驴氓聟聧忙聤聤 `northhing-core` 莽職聞猫聛職氓聬聢茅聴庐茅垄聵忙聬卢氓聢掳忙聳掳莽職?runtime crate茫聙?
+
+### 5.2 Opencode
+
+Opencode 氓庐聵忙聳鹿忙聳聡忙隆拢氓卤聲莽陇潞盲潞聠忙聸麓氓聛聫盲潞搂氓聯聛氓聦聳莽職聞忙聣漏氓卤聲忙篓隆氓聻聥茂录職氓聬聦盲赂聙盲赂?agent 氓聫炉盲禄楼猫驴聬猫隆聦氓聹?terminal茫聙聛desktop 忙聢?IDE茂录?
+agents 氓聢聠盲赂潞 primary agents 氓聮?subagents茂录聦氓聫炉茅聟聧莽陆庐 prompt茫聙聛model 盲赂?tool access茂录聸tools 茅聙職猫驴聡 permission 忙聨搂氓聢露茂录?
+氓鹿露氓聫炉茅聙職猫驴聡 custom tools 忙聢?MCP servers 忙聣漏氓卤聲茂录聸plugins 猫庐垄茅聵聟 command茫聙聛file茫聙聛permission茫聙聛session茫聙聛tool茫聙聛TUI
+莽颅聣盲潞聥盲禄露茂录聸skills 茅聙職猫驴聡莽聥卢莽芦聥莽聸庐氓陆聲忙聦聣茅聹聙氓聫聭莽聨掳氓聮聦氓聤聽猫陆陆茫聙?
+
+忙聙禄莽禄聯茂录?
+
+- Agent茫聙聛Tool茫聙聛MCP茫聙聛Plugin/Hook茫聙聛Skill 氓聮?Product Surface 氓潞聰猫炉楼忙聵炉盲潞聮莽聸赂猫驴聻忙聨楼莽職聞忙聣漏氓卤聲茅聺垄茂录聦猫聙聦盲赂聧忙聵炉氓聬聦盲赂聙盲赂陋忙篓隆氓聺聴氓聠聟茅聝篓莽職聞氓聢聠忙聰炉茫聙?
+- 忙聺聝茅聶聬氓聮聦氓路楼氓聟路氓聫炉猫搂聛忙聙搂氓驴聟茅隆禄忙聵炉 runtime 氓聫炉猫搂聜忙碌聥莽職聞 contract茂录聦盲赂聧猫聝陆氓聫陋氓颅聵氓聹篓盲潞?UI 忙聢?prompt 忙聥录忙聨楼盲赂颅茫聙?
+- 氓陇職盲潞搂氓聯聛氓陆垄忙聙聛茅聹聙猫娄?Product Assembly 氓聛?capability/provider 茅聙聣忙聥漏茂录聦猫聙聦盲赂聧忙聵炉猫庐漏 Agent Runtime SDK 氓聢陇忙聳颅猫掳聝莽聰篓忙聺楼猫聡陋
+  Desktop茫聙聛CLI茫聙聛Remote 猫驴聵忙聵炉 ACP茫聙?
+
+## 6. 莽聸庐忙聽聡茅聙禄猫戮聭猫搂聠氓聸戮
+
+莽聸庐忙聽聡忙聻露忙聻聞盲禄楼氓聟颅盲赂陋莽聣漏莽聬?owner 氓聢聠氓聦潞猫隆篓猫戮戮盲戮聺猫碌聳忙聳鹿氓聬聭茫聙聜`interfaces` 氓聫陋忙聣驴猫陆陆氓聧聫猫庐庐氓聮聦氓庐驴盲赂禄氓聟楼氓聫拢茂录聸`assembly` 猫麓聼猫麓拢盲潞搂氓聯聛猫聝陆氓聤聸茅聙聣忙聥漏盲赂聨忙鲁篓氓聠聦茂录聸`adapters` 猫麓聼猫麓拢氓聧聫猫庐庐茫聙聛transport 氓聮聦氓陇聳茅聝?provider 猫陆卢忙聧垄茂录聸`services` 猫麓聼猫麓拢忙聹卢氓聹掳莽鲁禄莽禄聼盲赂?runtime infrastructure 莽職聞氓聫炉氓陇聧莽聰篓氓聟路盲陆聯氓庐聻莽聨掳茂录聸`execution` 氓聫陋忙聰戮氓聫炉莽搂禄忙陇聧忙聣搂猫隆聦氓聨聼猫炉颅茂录聸`contracts` 忙聫聬盲戮聸莽篓鲁氓庐職盲潞聥氓庐聻茫聙聛port 氓聮聦盲潞搂氓聯聛茅垄聠氓聼聼猫搂聞氓聢聶茫聙聜猫驴聶忙聽路氓聫炉盲禄楼氓聬聦忙聴露氓聦潞氓聢聠芒聙聹氓聧聫猫庐庐茅聙聜茅聟聧芒聙聺氓聮聦芒聙聹忙聹聧氓聤隆氓庐聻莽聨掳芒聙聺茂录聦盲鹿聼茅聛驴氓聟聧忙聤聤 execution 猫炉炉猫搂拢盲赂潞氓庐聦忙聲麓猫驴聬猫隆聦忙聴露氓庐聻莽聨掳氓卤聜茫聙?
+
+```mermaid
+flowchart TB
+  Interfaces["忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录?br/>UI / command / protocol interface / delivery profile"]
+  Assembly["盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录?br/>compatibility facade / capability selection / adapter and service registration"]
+  Adapters["茅聙聜茅聟聧氓卤聜茂录聢Adapters茂录?br/>AI / API / transport / WebDriver / external provider translation"]
+  Services["忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录?br/>filesystem / git / terminal / MCP / remote / process / OS integration"]
+  Execution["忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录?br/>agent / stream / typed-service / tool primitives"]
+  Contracts["莽篓鲁氓庐職氓楼聭莽潞娄盲赂聨盲潞搂氓聯聛茅垄聠氓聼聼氓卤聜茂录聢Stable Contracts and Product Domains茂录?br/>DTO / event / runtime port / product domain policy"]
+  External["氓陇聳茅聝篓莽鲁禄莽禄聼茂录聢External Systems茂录?br/>OS / Git / MCP server / ACP client / AI provider / remote host"]
+
+  Interfaces --> Assembly
+  Interfaces --> Adapters
+  Assembly --> Adapters
+  Assembly --> Services
+  Assembly --> Execution
+  Assembly --> Contracts
+  Adapters --> Services
+  Adapters --> Execution
+  Adapters --> Contracts
+  Services --> Execution
+  Services --> Contracts
+  Execution --> Contracts
+  Adapters --> External
+  Services --> External
+```
+
+盲戮聺猫碌聳忙聳鹿氓聬聭氓聫陋氓聟聛猫庐赂盲禄聨盲赂聤氓聢掳盲赂聥茫聙聜忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜忙職麓茅聹虏盲潞搂氓聯聛氓陆垄忙聙聛茂录聸莽禄聞猫拢聟氓卤聜茅聙聣忙聥漏猫聝陆氓聤聸茅聸聠氓聬聢氓鹿露忙鲁篓氓聠?adapter/service茂录聸茅聙聜茅聟聧氓卤聜莽驴禄猫炉聭氓聧聫猫庐庐氓聮聦氓陇聳茅聝篓 provider茂录聸忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜忙聨楼猫搂娄 OS茫聙聛process茫聙聛filesystem茫聙聛git茫聙聛terminal茫聙聛MCP 氓聮?remote茂录聸忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜忙聫聬盲戮聸氓聫炉氓陇聧莽聰?runtime building blocks茂录聸氓楼聭莽潞娄氓卤聜忙聫聬盲戮聸莽篓鲁氓庐職盲潞聥氓庐聻茫聙聛port 氓聮聦盲潞搂氓聯聛茅垄聠氓聼聼猫搂聞氓聢聶茫聙聜盲禄禄盲陆聲盲赂聥氓卤?crate 氓聫聧氓聬聭猫炉禄氓聫聳盲潞搂氓聯聛氓聟楼氓聫拢茫聙聛莽禄聞猫拢聟茅聟聧莽陆庐忙聢聳 host state 茅聝陆猫搂聠盲赂潞猫戮鹿莽聲聦猫驴聺猫搂聞茫聙?
+
+## 7. 莽聸庐忙聽聡氓卤聜莽潞搂
+
+莽聸庐忙聽聡氓卤聜莽潞搂盲禄楼莽聣漏莽聬?owner 氓聢聠氓聦潞盲赂潞氓聟楼氓聫拢茫聙聜忙炉聫盲赂陋氓聢聠氓聦潞氓聫炉盲禄楼氓聦聟氓聬芦氓陇職盲赂?crate茂录聦盲陆聠 crate 氓聠聟茅聝篓猫聛聦猫麓拢氓驴聟茅隆禄猫聝陆氓陇聼茅聙職猫驴聡盲戮聺猫碌聳茫聙聛忙碌聥猫炉聲氓聮聦猫戮鹿莽聲聦猫聞職忙聹卢莽聥卢莽芦聥茅陋聦猫炉聛茫聙?
+
+### 7.1 忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录?
+
+忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜忙聵炉莽聰篓忙聢路茫聙聛氓聧聫猫庐庐忙聢聳氓陇聳茅聝篓莽鲁禄莽禄聼猫驴聸氓聟楼 northhing 莽職聞氓聟楼氓聫拢茂录聦猫麓聼猫麓拢 UI茫聙聛氓聭陆盲禄陇茫聙聛猫路炉莽聰卤茫聙聛氓聧聫猫庐庐忙聨楼氓聫拢茫聙聛盲潞陇盲禄聵氓陆垄忙聙聛茅聙聣忙聥漏氓聮?host integration茫聙聜氓炉鹿氓潞聰猫聦聝氓聸麓氓聦聟忙聥?`src/apps/*`茫聙聛`src/web-ui`茫聙聛`src/mobile-web`茫聙聛`northhing-Installer`茫聙聛`tests/e2e` 氓聮?`src/crates/interfaces`茫聙聜氓聟楼氓聫拢氓卤聜氓聫炉盲禄楼茅聙聣忙聥漏 `DeliveryProfile` 氓鹿露猫掳聝莽聰?assembly 忙聢?adapter API茂录聦盲陆聠盲赂聧忙聥楼忙聹聣氓聟卤盲潞?runtime 猫隆聦盲赂潞茫聙?
+
+### 7.2 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录?
+
+盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜猫麓聼猫麓拢氓聟录氓庐鹿氓炉录氓聡潞茫聙聛氓庐聦忙聲麓盲潞搂氓聯聛猫聝陆氓聤聸茅聙聣忙聥漏茫聙聛feature group 氓聢?capability set 莽職聞忙聵聽氓掳聞茫聙聛adapter/service 忙鲁篓氓聠聦氓聮?product-full 忙聨楼莽潞驴茫聙聜莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/assembly`茂录聦氓陆聯氓聣聧氓聦聟氓聬?`northhing-core` 氓聟录氓庐鹿茅聴篓茅聺垄氓聮?`northhing-product-capabilities` 猫聝陆氓聤聸忙篓隆氓聻聥茫聙聜`product-capabilities` 氓聫陋忙聫聫猫驴?capability id茫聙聛tool group茫聙聛service requirement茂录聦盲赂聧忙聣搂猫隆聦 IO茂录聦盲鹿聼盲赂聧忙聣驴猫陆陆盲潞搂氓聯聛茅垄聠氓聼聼莽聤露忙聙聛忙聹潞茫聙?
+
+### 7.3 茅聙聜茅聟聧氓卤聜茂录聢Adapters茂录?
+
+茅聙聜茅聟聧氓卤聜猫麓聼猫麓拢氓聧聫猫庐庐茫聙聛transport茫聙聛氓陇聳茅聝?provider 氓聮聦氓庐驴盲赂禄茅聙職盲驴隆猫陆卢忙聧垄茂录聦莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/adapters`茫聙聜氓聟露盲赂?`ai-adapters` 猫麓聼猫麓拢 AI provider 猫炉路忙卤聜/氓聯聧氓潞聰忙聵聽氓掳聞氓聮?provider stream 氓聧聫猫庐庐猫搂拢忙聻聬茂录聦猫搂拢忙聻聬莽禄聯忙聻聹氓潞聰猫陆卢忙聧垄盲赂?execution 氓卤聜忙聥楼忙聹聣莽職聞莽禄聼盲赂聙 stream 氓楼聭莽潞娄茂录聸`api-layer` 猫麓聼猫麓拢盲潞搂氓聯聛氓庐驴盲赂禄氓聟卤莽聰篓莽職聞氓聬聨莽芦?API adapter茂录聦`transport` 猫麓聼猫麓拢盲潞聥盲禄露忙聤聲茅聙聮氓聮聦 host transport adapter茂录聦`webdriver` 猫麓聼猫麓拢 WebDriver 氓聧聫猫庐庐氓聮聦忙碌聫猫搂聢氓聶篓猫聡陋氓聤篓氓聦?adapter茫聙聜茅聙聜茅聟聧氓卤聜盲赂聧忙聥楼忙聹聣盲潞搂氓聯聛猫聝陆氓聤聸茅聙聣忙聥漏茂录聦盲鹿聼盲赂聧忙聣驴猫陆陆氓聫炉氓陇聧莽聰篓 OS service 氓庐聻莽聨掳茫聙?
+
+### 7.4 忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录?
+
+忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜猫麓聼猫麓拢忙聨楼猫搂娄忙聹卢氓聹掳莽鲁禄莽禄聼氓聮聦 runtime infrastructure 莽職聞氓聫炉氓陇聧莽聰篓氓聟路盲陆聯氓庐聻莽聨掳茂录聦莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/services`茫聙聜氓聟露盲赂?`services-core` 忙聣驴猫陆陆猫陆禄茅聡聫 service primitive茂录聦`services-integrations` 忙聣驴猫陆陆 MCP茫聙聛Git茫聙聛remote茫聙聛file watch 氓聮聦盲潞搂氓聯聛茅垄聠氓聼?port 莽職聞氓聟路盲陆聯氓庐聻莽聨掳茂录聦`terminal` 忙聣驴猫陆陆 PTY茫聙聛shell integration 氓聮?terminal session infrastructure茫聙聜忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜氓聫炉盲禄楼氓庐聻莽聨掳 `contracts`茫聙聛`execution` 忙聢?`product-domains` 氓庐職盲鹿聣莽職?port茂录聦盲陆聠盲赂聧茅聙聣忙聥漏盲潞搂氓聯聛 profile茂录聦盲鹿聼盲赂聧莽聸麓忙聨楼忙職麓茅聹?UI/氓聧聫猫庐庐氓聟楼氓聫拢茫聙?
+
+### 7.5 忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录?
+
+忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜忙聫聬盲戮?provider-neutral 莽職?runtime building blocks茂录聦莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/execution`茫聙聜`agent-runtime`茫聙聛`agent-stream`茫聙聛`runtime-services`茫聙聛`tool-contracts` 氓聮?`tool-execution` 氓聢聠氓聢芦氓庐職盲鹿聣 agent loop facts茫聙聛莽禄聼盲赂聙 stream DTO / tool-call 莽麓炉莽搂炉 / replay 氓楼聭莽潞娄茫聙聛workflow descriptor茫聙聛typed service bundle茫聙聛tool manifest / permission / result policy茫聙聛tool group facts 氓聮聦盲陆聨氓卤?tool execution helper茫聙聜氓陆聯氓聣?Cargo package / lib 氓聬聧盲驴聺忙聦聛氓聟录氓庐鹿茂录聦盲陆聠莽聣漏莽聬聠莽聸庐氓陆聲忙聦聣猫聛聦猫麓拢氓聭陆氓聬聧茫聙聜氓庐聝盲禄卢氓聫陋猫聝陆盲戮聺猫碌聳莽篓鲁氓庐職氓楼聭莽潞娄忙聢聳忙聵聨莽隆庐莽職?provider-neutral DTO茂录聦盲赂聧莽聸麓忙聨楼氓聢聸氓禄潞 Tauri handle茫聙聛filesystem manager茫聙聛Git provider茫聙聛MCP client茫聙聛AI client 忙聢?host process茫聙?
+
+### 7.6 莽篓鲁氓庐職氓楼聭莽潞娄盲赂聨盲潞搂氓聯聛茅垄聠氓聼聼氓卤聜茂录聢Stable Contracts and Product Domains茂录?
+
+莽篓鲁氓庐職氓楼聭莽潞娄盲赂聨盲潞搂氓聯聛茅垄聠氓聼聼氓卤聜忙聵炉忙聹聙盲陆聨氓卤聜茂录聦莽聣漏莽聬聠盲陆聧莽陆庐忙聵炉 `src/crates/contracts`茫聙聜氓庐聝氓聦聟氓聬芦 `core-types`茫聙聛`events`茫聙聛`runtime-ports` 氓聮?`product-domains`茫聙聜`product-domains` 忙聵?Product Domain Model茂录聦猫麓聼猫麓?MiniApp茫聙聛function-agent 莽颅聣茅垄聠氓聼?DTO茫聙聛莽潞炉莽颅聳莽聲楼茫聙聛莽聤露忙聙聛猫搂聞氓聢聶氓聮聦莽陋?port茂录聸氓聟路盲陆?Git茫聙聛filesystem茫聙聛AI 忙聢?worker execution 氓庐聻莽聨掳氓聹?services茫聙聛adapters 忙聢?assembly/core 莽職聞氓聟录氓庐鹿猫路炉氓戮聞盲赂颅茂录聦盲赂聧氓戮聴氓聸聻忙碌聛氓聢掳 contracts茫聙?
+
+### 7.7 忙聣漏氓卤聲莽聜鹿氓陆聮氓卤?
+
+- AI茫聙聛API茫聙聛transport 氓聮?WebDriver 莽職聞氓聧聫猫庐庐猫陆卢忙聧垄氓卤聻盲潞?Adapters茫聙?
+- MCP茫聙聛terminal茫聙聛filesystem茫聙聛git茫聙聛remote 氓聮?file watch 莽職聞氓聫炉氓陇聧莽聰篓氓聟路盲陆聯氓庐聻莽聨掳氓卤聻盲潞聨 Services茫聙?
+- Tool manifest茫聙聛permission茫聙聛execution admission茫聙聛result / artifact policy 氓卤聻盲潞聨 Execution Primitives 莽職?`tool-contracts`茫聙?
+- 盲陆聨氓卤?filesystem/search helper 氓卤聻盲潞聨 `tool-execution`茫聙?
+- Agent茫聙聛subagent茫聙聛prompt module茫聙聛scheduler茫聙聛session / turn facts 氓聮?hook routing 氓卤聻盲潞聨 Execution Primitives茫聙?
+- Capability pack茫聙聛delivery profile茫聙聛adapter/service selection 氓聮?product-full assembly 氓卤聻盲潞聨 Product Assembly茫聙?
+- 盲潞搂氓聯聛茅垄聠氓聼聼莽聤露忙聙聛茫聙聛猫搂聞氓聢聶茫聙聛port 氓聮?domain policy 氓卤聻盲潞聨 Stable Contracts and Product Domains茫聙?
+
+## 8. 忙聨楼氓聫拢盲赂聨氓庐聻莽聨掳氓聟鲁莽鲁?
+
+忙聨楼氓聫拢莽聰卤莽篓鲁氓庐職氓楼聭莽潞娄茫聙聛Runtime Services茫聙聛Tool Contracts 氓庐職盲鹿聣茂录聸氓聟路盲陆聯氓庐聻莽聨掳莽聰卤 adapter茫聙聛service 忙聢聳盲潞搂氓聯聛氓聟楼氓聫拢氓聢聸氓禄潞茂录聸忙鲁篓氓聠聦氓聤篓盲陆聹氓聫陋猫聝陆氓聫聭莽聰聼氓聹?Product Assembly茫聙聜Agent Runtime茫聙聛tool contracts 氓聮?tool execution 氓聫陋忙聨楼忙聰露氓路虏莽禄聫莽禄聞猫拢聟氓楼陆莽職聞忙聨楼氓聫拢忙聢聳 provider registry茂录聦盲赂聧莽聸麓忙聨楼氓聢聸氓禄潞氓鹿鲁氓聫掳氓庐聻莽聨掳茫聙?
+
+```mermaid
+flowchart TB
+  Interface["忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录?br/>茅聙聣忙聥漏氓聟楼氓聫拢氓聮?DeliveryProfile"]
+  Assembly["盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录?br/>氓聰炉盲赂聙忙鲁篓氓聠聦莽聜?]
+  ServiceBuilder["猫驴聬猫隆聦忙聴露忙聹聧氓聤隆氓卤聜茂录聢Runtime Services茂录?br/>RuntimeServicesBuilder"]
+  ToolBuilder["氓路楼氓聟路忙聣搂猫隆聦氓聨聼猫炉颅茂录聢Tool Primitives茂录?br/>tool contracts / execution"]
+
+  AgentRegistry["Agent 忙聣搂猫隆聦氓聨聼猫炉颅茂录聢Agent Runtime茂录?br/>AgentDefinitionRegistry"]
+  CommandRegistry["忙聨楼氓聫拢 / 盲潞搂氓聯聛莽禄聞猫拢聟氓卤?br/>ProductCommandRegistry"]
+  Runtime["Agent / Tool primitives<br/>氓聫陋忙露聢猫麓鹿忙聨楼氓聫?]
+  Adapters["茅聙聜茅聟聧氓卤聜茂录聢Adapters茂录?br/>AI / API / transport / WebDriver adapters"]
+  Services["忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录?br/>OS / filesystem / Git / terminal / MCP / remote services"]
+  Contracts["莽篓鲁氓庐職氓楼聭莽潞娄盲赂聨盲潞搂氓聯聛茅垄聠氓聼聼氓卤聜茂录聢Stable Contracts and Product Domains茂录?br/>DTO / event / port trait"]
+
+  Interface --> Assembly
+  Assembly --> ServiceBuilder
+  Assembly --> ToolBuilder
+
+  Assembly --> AgentRegistry
+  Assembly --> CommandRegistry
+  Assembly --> Adapters
+  Assembly --> Services
+  ServiceBuilder --> Runtime
+  ToolBuilder --> Runtime
+
+  AgentRegistry --> Runtime
+  CommandRegistry --> Interface
+  Runtime --> Contracts
+  Adapters --> Contracts
+  Services --> Contracts
+  Adapters --> Services
+```
+
+忙鲁篓氓聠聦氓聶篓盲赂聨氓聣聧忙聳聡莽聸庐忙聽聡氓卤聜莽潞搂莽職聞氓炉鹿氓潞聰氓聟鲁莽鲁禄氓娄聜盲赂聥茂录職
+
+| 忙鲁篓氓聠聦氓聶?/ 莽禄聞猫拢聟莽聜?| 忙聣聙氓卤聻莽聸庐忙聽聡氓卤聜莽潞?| 氓聢聺氓搂聥忙聣驴猫陆陆盲赂聨莽聸庐忙聽聡忙聣驴猫陆?| 忙鲁篓氓聠聦氓聠聟氓庐鹿 |
+|---|---|---|---|
+| `ProductAssembler` / `ProductAssemblyPlan` | 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录?| 氓聢聺氓搂聥氓聫炉氓聹篓 `northhing-core` facade 忙聢聳盲潞搂氓聯聛氓聟楼氓聫拢茂录聸莽聸庐忙聽聡氓聫炉忙聰露忙聲聸盲赂潞 assembly owner | `DeliveryProfile`茫聙聛`CapabilitySet`茫聙聛feature group茫聙聛adapter/service 茅聙聣忙聥漏 |
+| `RuntimeServicesBuilder` | 忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录聣盲赂聨忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录聣莽職聞猫戮鹿莽聲聦 | 莽聸庐忙聽聡氓聹?`northhing-runtime-services`茂录聸猫驴聻忙聨?`northhing-runtime-ports`茫聙聛`northhing-services-*` 氓聮聦氓聢聺氓搂?service wiring | filesystem茫聙聛workspace茫聙聛session store茫聙聛Git茫聙聛terminal茫聙聛network茫聙聛MCP catalog茫聙聛remote connection / workspace / projection port |
+| `ToolRuntimeBuilder` | 忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录?| `tool-execution`茫聙聛`tool-contracts`茂录聸Cargo package 氓聬聧盲驴聺忙聦聛氓聟录氓庐?| tool provider茫聙聛manifest茫聙聛permission gate茫聙聛tool hook |
+
+| `AgentDefinitionRegistry` | 忙聣搂猫隆聦氓聨聼猫炉颅氓卤聜茂录聢Execution Primitives茂录?| 莽聸庐忙聽聡氓聹?`northhing-agent-runtime`茂录聸氓聢聺氓搂聥氓聫炉莽聰?`northhing-core` agent definition 盲禄拢莽聽聛忙聣驴猫陆陆 | agent茫聙聛subagent茫聙聛prompt module茫聙聛skill definition |
+| `ProductCommandRegistry` | 忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录聣盲赂聨盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录聣莽職聞猫戮鹿莽聲聦 | 盲潞搂氓聯聛氓聟楼氓聫拢忙聢?assembly 忙篓隆氓聺聴 | 猫戮聯氓聟楼忙隆聠氓聭陆盲禄陇茫聙聛氓庐隆忙聽赂氓聟楼氓聫拢茫聙聛MiniApp 氓聟楼氓聫拢氓聢?capability / runtime request 莽職聞忙聵聽氓掳?|
+| adapter set | 茅聙聜茅聟聧氓卤聜茂录聢Adapters茂录?| `northhing-ai-adapters`茫聙聛`northhing-api-layer`茫聙聛`northhing-transport`茫聙聛`northhing-webdriver`茫聙聛app adapters | AI茫聙聛API茫聙聛transport茫聙聛WebDriver 莽颅聣氓聧聫猫庐庐忙聢聳氓陇聳茅聝篓 provider adapter |
+| service set | 忙聹聧氓聤隆氓庐聻莽聨掳氓卤聜茂录聢Services茂录?| `northhing-services-*`茫聙聛`terminal-core` 氓聮聦氓聟路盲陆?app service implementations | OS茫聙聛filesystem茫聙聛Git茫聙聛terminal茫聙聛MCP茫聙聛remote 莽職聞氓聟路盲陆?service茂录聸Remote service 氓聠聟茅聝篓莽禄搂莽禄颅氓聦潞氓聢聠 SSH茫聙聛relay茫聙聛忙聹卢氓聹掳茅職搂茅聛聯茫聙聛猫驴聹莽芦?OS 忙聰炉忙聦聛 |
+
+忙鲁篓氓聠聦猫路炉氓戮聞氓驴聟茅隆禄忙聵炉忙聵戮氓录聫茫聙聛typed茫聙聛氓聫炉忙碌聥猫炉聲莽職聞茂录職
+
+- 忙聨楼氓聫拢盲赂聨氓聟楼氓聫拢氓卤聜茂录聢Interfaces and Entrypoints茂录聣氓聫陋茅聙聣忙聥漏 `DeliveryProfile` 氓聮聦盲潞搂氓聯聛茅聟聧莽陆庐茂录聦盲赂聧莽聸麓忙聨楼忙聤聤 concrete manager 盲录聽氓聟楼 runtime茫聙?
+- 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录聣忙聽鹿忙聧庐盲潞搂氓聯聛氓陆垄忙聙聛氓聢聸氓禄潞忙聢聳忙聨楼忙聰露 adapter/service茂录聦氓鹿露猫掳聝莽聰篓 typed builder 氓庐聦忙聢聬忙鲁篓氓聠聦茫聙?
+- Tool茫聙聛OS茫聙聛Remote茫聙聛Protocol provider 氓聢聠氓聢芦莽聲聶氓聹篓氓炉鹿氓潞聰 app茫聙聛Adapters 忙聢?Services 盲赂颅茂录聦茅聙職猫驴聡氓聬聦盲赂聙莽禄?port 忙職麓茅聹虏茫聙?
+- Tauri 氓聫陋猫聝陆氓聡潞莽聨掳氓聹?Desktop app茫聙聛transport/API adapter 忙聢聳盲潞搂氓聯聛氓聟楼氓聫拢氓聭陆盲禄陇氓陇聳猫搂聜盲赂颅茂录聸Agent Runtime茫聙?
+  Tool primitives茫聙聛Runtime Services contract 氓聮?Product Capabilities 盲赂聧氓戮聴盲戮聺猫碌聳 Tauri handle茫聙?
+  window茫聙聛command macro 忙聢?desktop app state茫聙?
+- Remote provider 氓驴聟茅隆禄忙聥聠氓聢聠莽篓鲁氓庐職猫驴聻忙聨楼忙聨楼氓聫拢氓聮聦氓聟路盲陆聯猫驴聹莽芦?OS / transport 氓庐聻莽聨掳茂录聦茅聛驴氓聟聧忙聤聤 SSH茫聙聛relay 忙聢聳猫驴聹莽芦炉氓鹿鲁氓聫掳氓路庐氓录聜忙鲁聞忙录聫氓聢掳 runtime茫聙?
+- 盲赂聧忙聰炉忙聦聛莽職聞猫聝陆氓聤聸氓聹?assembly 莽職?capability availability 盲赂颅忙聵戮氓录聫猫驴聰氓聸?unsupported / unavailable茂录聦盲赂聧氓聹?execution primitive 氓聠聟氓聠聶盲潞搂氓聯聛氓聢聠忙聰炉茫聙?
+- 莽娄聛忙颅垄盲陆驴莽聰篓忙聴聽莽卤禄氓聻?`Any` service locator茫聙聛氓聟篓氓卤聙 mutable registry 忙聢聳盲赂聥氓卤?crate 氓聫聧氓聬聭猫炉禄氓聫聳盲潞搂氓聯聛茅聟聧莽陆庐茫聙?
+
+## 9. 茅拢聨茅聶漏
+
+| 茅拢聨茅聶漏 | 盲驴聺忙聤陇忙聳鹿氓录聫 |
+|---|---|
+| 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录聣猫聠篓猫聝聙盲赂潞忙聳掳莽職聞氓聟篓氓卤聙莽聤露忙聙聛盲赂颅氓驴?| assembly 氓聫陋氓聛職忙聻聞氓禄潞忙聹聼忙鲁篓氓聠聦茂录聦猫戮聯氓聡潞盲赂聧氓聫炉氓聫?runtime parts茂录聸盲潞搂氓聯聛莽聤露忙聙聛盲禄聧氓陆?surface 忙聢?runtime owner |
+| 忙聨楼氓聫拢忙聥聠氓戮聴猫驴聡莽禄聠茂录聦氓炉录猫聡麓氓陇聧忙聺聜氓潞娄氓聮聦氓聤篓忙聙聛氓聢聠氓聫聭忙聢聬忙聹卢盲赂聤氓聧?| 盲禄?capability 氓聮聦莽篓鲁氓庐職莽聰篓盲戮聥氓庐職盲鹿?port 莽虏聮氓潞娄茂录聦莽聝颅猫路炉氓戮聞茅聛驴氓聟聧猫驴聬猫隆聦忙聴?map lookup茂录聦盲录聵氓聟?builder-time 忙鲁篓氓聟楼 |
+| 氓鹿鲁氓聫掳氓庐聻莽聨掳忙鲁聞忙录聫氓聢?Agent 忙聢?Tool execution primitives | 盲戮聺猫碌聳忙拢聙忙聼楼莽娄聛忙颅?execution owner 盲戮聺猫碌聳 app crate茫聙聛Tauri茫聙聛CLI TUI茫聙聛ACP protocol 氓聮?concrete service crate |
+| core 忙聥聠氓聢聠氓聬聨盲禄聧茅職聬氓录聫莽禄聭氓庐職 Tauri | Tauri 氓聫陋氓聟聛猫庐赂氓聹篓 Desktop app 忙聢聳忙聵聨莽隆?feature-gated adapter茂录聸氓聬聭盲赂聥氓卤聜盲录聽茅聙?typed port茫聙聛DTO茫聙聛event fact 氓聮?capability availability |
+| 盲赂聧氓聬聦盲潞搂氓聯聛氓陆垄忙聙聛猫聝陆氓聤聸莽聼漏茅聵碌忙录聜莽搂?| Product Assembly 莽禄麓忙聤陇 capability matrix茂录聸氓聡聫氓掳聭忙聢聳忙聸驴忙聧垄猫聝陆氓聤聸忙聴露猫隆楼盲潞搂氓聯聛氓聟楼氓聫拢茅陋聦猫炉聛氓聮?unsupported 猫隆聦盲赂潞忙碌聥猫炉聲 |
+| Tool茫聙聛MCP茫聙聛ACP 莽職?manifest茫聙聛permission 忙聢聳盲潞聥盲禄露猫炉颅盲鹿聣忙聥聠猫搂拢氓聬聨盲赂聧莽颅聣盲禄?| 盲驴聺莽聲聶忙聴搂猫路炉氓戮聞氓聟录氓庐?facade茂录聦氓垄聻氓聤?manifest snapshot茫聙聛permission 氓聠鲁莽颅聳氓聮聦盲潞聥盲禄露忙聵聽氓掳聞莽颅聣盲禄路忙碌聥猫炉?|
+
+| `northhing-core` 氓聫陋忙聵炉忙聰鹿氓聬聧盲赂潞忙聳掳莽職聞氓路篓氓聻?runtime crate | 忙聳?owner crate 氓驴聟茅隆禄忙聹聣氓聧聲盲赂聙猫聛聦猫麓拢氓聮聦忙聹聙氓掳聫盲戮聺猫碌聳茂录聸盲潞搂氓聯聛猫聝陆氓聤聸 氓聮?service 氓庐聻莽聨掳盲赂聧氓戮聴莽禄搂莽禄颅氓聽聠氓聟楼 agent kernel |
+| 莽聸庐忙聽聡 crate 氓聟聢猫隆聦氓聢聸氓禄潞盲陆聠忙虏隆忙聹聣莽聹聼氓庐?owner | 氓聫陋忙聹聣 owner 猫戮鹿莽聲聦茫聙聛忙聴搂猫路炉氓戮聞氓聟录氓庐鹿茫聙聛focused tests茫聙聛盲戮聺猫碌聳忙聰露莽聸聤氓聮聦 boundary check 氓聬聦忙聴露忙聢聬莽芦聥忙聴露忙聣聧氓聢聸氓禄潞 crate茂录聸氓聬娄氓聢聶莽禄搂莽禄颅莽聲聶氓聹?facade |
+
+## 10. 莽聸庐忙聽聡莽聤露忙聙聛氓聢陇氓庐?
+
+- `northhing-core` 盲赂聧氓聠聧忙聵炉盲潞聥氓庐聻盲赂聤莽職聞氓庐聦忙聲?runtime owner茂录聦猫聙聦忙聵炉氓聟录氓庐鹿 facade 氓聮?`product-full` 莽禄聞猫拢聟猫戮鹿莽聲聦茫聙?
+- Agent Runtime SDK 氓聫炉氓聹篓盲赂聧盲戮聺猫碌?`northhing-core`茫聙聛app crate 忙聢?Tauri 莽職聞忙聝聟氓聠碌盲赂聥猫垄芦氓碌聦氓聟楼茂录聦氓鹿露茅聙職猫驴聡莽篓鲁氓庐職 builder /
+  runner / event stream / registry API 忙聫聬盲戮聸 agent 猫聝陆氓聤聸茫聙?
+- Agent Runtime茫聙聛Tool Contracts / Tool Execution茫聙聛Runtime Services 氓聮?Product Capabilities 氓聢聠氓聢芦忙聥楼忙聹聣氓聫炉氓庐隆忙聼楼莽職聞猫聛聦猫麓拢猫戮鹿莽聲聦茫聙?
+- 莽篓鲁氓庐職氓楼聭莽潞娄氓聮聦氓聬聞 execution owner 氓庐職盲鹿聣忙聨楼氓聫拢茂录聸氓聟路盲陆?Tool茫聙聛OS茫聙聛Remote service 莽聲聶氓聹篓 Services茂录聦氓聧聫猫庐庐氓聮聦氓陇聳茅聝篓 provider 猫陆卢忙聧垄莽聲聶氓聹篓 Adapters茫聙?
+- 盲潞搂氓聯聛莽禄聞猫拢聟氓卤聜茂录聢Product Assembly茂录聣忙聵炉氓聰炉盲赂聙忙鲁篓氓聠聦莽聜鹿茂录聦茅聙職猫驴聡 typed builder / registry 猫驴聻忙聨楼忙聨楼氓聫拢氓聮聦氓聟路盲陆聯氓庐聻莽聨掳茫聙?
+- Tauri 氓聫陋氓卤聻盲潞?Desktop app 忙聢聳忙聵聨莽隆?feature-gated adapter茂录聦盲赂聧猫驴聸氓聟楼 core茫聙聛execution owner 忙聢?contract crate茫聙?
+- runtime 氓聫陋盲戮聺猫碌?remote connection茫聙聛remote workspace茫聙聛remote projection 氓聮?capability facts 莽颅?port茂录聸SSH茫聙聛relay茫聙?
+  忙聹卢氓聹掳茅職搂茅聛聯茫聙聛猫驴聹莽芦?OS 氓路庐氓录聜氓聮聦猫庐陇猫炉聛忙聳鹿氓录聫氓卤聻盲潞聨氓聟路盲陆?Remote provider茫聙?
+- 盲潞搂氓聯聛氓陆垄忙聙聛氓路庐氓录聜茅聙職猫驴聡 capability matrix 氓聮?Product Assembly 猫隆篓猫戮戮茂录聦盲赂聧茅聙職猫驴聡盲赂聥忙虏聣 UI茫聙聛氓聭陆盲禄陇茫聙聛氓聧聫猫庐庐忙聢聳氓鹿鲁氓聫掳氓庐聻莽聨掳猫隆篓猫戮戮茫聙?
+- 忙聺聝茅聶聬茫聙聛氓路楼氓聟路忙聸聺氓聟聣茫聙聛盲潞聥盲禄露茫聙聛session茫聙聛remote workspace 氓聮?release 忙聻聞氓禄潞氓陆垄忙聙聛氓驴聟茅隆禄盲驴聺忙聦聛氓聤聼猫聝陆莽颅聣盲禄路茫聙?
diff --git a/docs/status/surfaces.md b/docs/status/surfaces.md
index adb4ae7..273c14d 100644
--- a/docs/status/surfaces.md
+++ b/docs/status/surfaces.md
@@ -15,32 +15,30 @@
 
 These compile and may have partial functionality, but are **not** shipped, not tested in CI for user-facing flows, and may break without notice.
 
 | Surface | Crate / Path | Status | Notes |
 |---------|-------------|--------|-------|
 | **CLI** | `src/apps/cli` (`northhing-cli`) | 🧊 Frozen | Compiles; no release artifact. `doctor` command has false positives. See tech-debt-ledger P2. |
 | **Server** | `src/apps/server` | 🧊 Frozen | HTTP API surface; no auth layer. Not deployed. |
 | **Relay Server** | `src/apps/relay-server` (`relay-core`) | 🧊 Frozen | Binds `0.0.0.0` by default with no authentication. Do not expose. See tech-debt-ledger P1. |
 | **Mobile Web** | `src/mobile-web/` | 🧊 Frozen | PWA shell; re-pairing flow unguided, i18n has mojibake. |
 | **MiniApp UI** | `src/crates/contracts/product-domains/src/miniapp/` | 🧊 Frozen | Built-in mini-apps (PPT live, etc.) are experimental. String-mode shell commands rejected by `guard_command_execution`. |
-| **SDLC Harness** | `src/crates/execution/harness/` | 🧊 Frozen | Test/eval harness; not user-facing. |
 | **Tauri Desktop (candidate)** | `src/apps/desktop-tauri` | 🧊 Frozen | Tauri 2 + React candidate for the next baseline; flips at F4. src-tauri is its own cargo workspace (excluded from main). |
 
 ## Active Capability Crates (Agent Toolbox)
 
 These are not user-facing surfaces but are actively maintained as the agent's tool layer:
 
 | Crate | Path | Role |
 |-------|------|------|
 | `tool-contracts` | `src/crates/execution/tool-contracts` | Tool trait definitions |
 | `tool-execution` | `src/crates/execution/tool-execution` | Tool execution engine |
-| `tool-provider-groups` | `src/crates/execution/tool-provider-groups` | Tool registration/grouping |
 | `agent-dispatch` | `src/crates/execution/agent-dispatch` | Agent dispatch (lightweight actor mode) |
 | `agent-runtime` | `src/crates/execution/agent-runtime` | Agent runtime loop |
 | `agent-stream` | `src/crates/execution/agent-stream` | Streaming response handling |
 | `runtime-services` | `src/crates/execution/runtime-services` | Runtime support services |
 | `services-core` | `src/crates/services/services-core` | Core services |
 | `services-integrations` | `src/crates/services/services-integrations` | Integration services |
 | `terminal` | `src/crates/services/terminal` | Terminal service |
 | `debug-log` | `src/crates/services/debug-log` | Debug-mode runtime logging leaf crate (`log_event` + `COMP_*` constants); shared by desktop and core, re-exported from core (K4a-T5) |
 | `ai-adapters` | `src/crates/adapters/ai-adapters` | AI provider adapters |
 | `kernel-api` | `src/crates/contracts/kernel-api` | Kernel facade contracts — product surfaces reach core only through this facade (K1) |
diff --git a/scripts/core-boundaries/rules/crate-layout.mjs b/scripts/core-boundaries/rules/crate-layout.mjs
index da9c801..b320db9 100644
--- a/scripts/core-boundaries/rules/crate-layout.mjs
+++ b/scripts/core-boundaries/rules/crate-layout.mjs
@@ -4,23 +4,21 @@
 export const crateLayoutRules = [
   { crateName: 'core-types', layer: 'contracts', path: 'src/crates/contracts/core-types' },
   { crateName: 'events', layer: 'contracts', path: 'src/crates/contracts/events' },
   { crateName: 'product-domains', layer: 'contracts', path: 'src/crates/contracts/product-domains' },
   { crateName: 'runtime-ports', layer: 'contracts', path: 'src/crates/contracts/runtime-ports' },
   { crateName: 'kernel-api', layer: 'contracts', path: 'src/crates/contracts/kernel-api' },
 
   { crateName: 'agent-runtime', layer: 'execution', path: 'src/crates/execution/agent-runtime' },
   { crateName: 'agent-stream', layer: 'execution', path: 'src/crates/execution/agent-stream' },
   { crateName: 'agent-tools', layer: 'execution', path: 'src/crates/execution/tool-contracts' },
-  { crateName: 'harness', layer: 'execution', path: 'src/crates/execution/harness' },
   { crateName: 'runtime-services', layer: 'execution', path: 'src/crates/execution/runtime-services' },
-  { crateName: 'tool-packs', layer: 'execution', path: 'src/crates/execution/tool-provider-groups' },
   { crateName: 'tool-runtime', layer: 'execution', path: 'src/crates/execution/tool-execution' },
 
   { crateName: 'product-capabilities', layer: 'assembly', path: 'src/crates/assembly/product-capabilities' },
 
   { crateName: 'services-core', layer: 'services', path: 'src/crates/services/services-core' },
   { crateName: 'services-integrations', layer: 'services', path: 'src/crates/services/services-integrations' },
   { crateName: 'terminal', layer: 'services', path: 'src/crates/services/terminal' },
   { crateName: 'debug-log', layer: 'services', path: 'src/crates/services/debug-log' },
 
   { crateName: 'acp', layer: 'interfaces', path: 'src/crates/interfaces/acp' },
diff --git a/scripts/core-boundaries/rules/crate-rules.mjs b/scripts/core-boundaries/rules/crate-rules.mjs
index 7decdd3..19c3b25 100644
--- a/scripts/core-boundaries/rules/crate-rules.mjs
+++ b/scripts/core-boundaries/rules/crate-rules.mjs
@@ -1,45 +1,42 @@
 // Boundary rules for crate dependencies and lightweight profiles.
 
 export const noCoreDependencyCrates = [
   'core-types',
   'events',
   'ai-adapters',
   'agent-stream',
   'agent-runtime',
-  'harness',
   'product-capabilities',
   'runtime-ports',
   'runtime-services',
   'services-core',
   'services-integrations',
   'agent-tools',
-  'tool-packs',
   'product-domains',
   'terminal',
   'tool-runtime',
 ];
 
 export const lightweightBoundaryRules = [
   {
     crateName: 'core-types',
     reason: 'core-types must stay low-level DTO-only',
     forbiddenDeps: [
       'northhing-core',
       'northhing-events',
       'northhing-ai-adapters',
       'northhing-agent-stream',
       'northhing-runtime-ports',
       'northhing-services-core',
       'northhing-services-integrations',
       'northhing-agent-tools',
-      'northhing-tool-packs',
       'northhing-product-domains',
       'northhing-transport',
       'terminal-core',
       'tool-runtime',
       'tauri',
       'reqwest',
       'git2',
       'rmcp',
       'image',
       'tokio-tungstenite',
@@ -52,21 +49,20 @@ export const lightweightBoundaryRules = [
   },
   {
     crateName: 'runtime-ports',
     reason: 'runtime-ports must stay DTO/trait-only',
     forbiddenDeps: [
       'northhing-core',
       'northhing-agent-stream',
       'northhing-services-core',
       'northhing-services-integrations',
       'northhing-agent-tools',
-      'northhing-tool-packs',
       'northhing-product-domains',
       'northhing-transport',
       'terminal-core',
       'tool-runtime',
       'tauri',
       'reqwest',
       'git2',
       'rmcp',
       'image',
       'tokio-tungstenite',
@@ -80,21 +76,20 @@ export const lightweightBoundaryRules = [
   {
     crateName: 'runtime-services',
     reason: 'runtime-services must stay a typed service assembly contract without concrete runtime implementations',
     forbiddenDeps: [
       'northhing-core',
       'northhing-ai-adapters',
       'northhing-agent-stream',
       'northhing-services-core',
       'northhing-services-integrations',
       'northhing-agent-tools',
-      'northhing-tool-packs',
       'northhing-product-domains',
       'northhing-transport',
       'terminal-core',
       'tool-runtime',
       'tauri',
       'reqwest',
       'git2',
       'rmcp',
       'image',
       'tokio-tungstenite',
@@ -106,47 +101,20 @@ export const lightweightBoundaryRules = [
     ],
   },
   {
     crateName: 'agent-runtime',
     reason: 'agent-runtime must own portable runtime decisions without concrete service or product implementations',
     forbiddenDeps: [
       'northhing-core',
       'northhing-ai-adapters',
       'northhing-services-core',
       'northhing-services-integrations',
-      'northhing-tool-packs',
-      'northhing-product-domains',
-      'northhing-transport',
-      'terminal-core',
-      'tauri',
-      'reqwest',
-      'git2',
-      'rmcp',
-      'image',
-      'tokio-tungstenite',
-      'northhing-cli',
-      'ratatui',
-      'crossterm',
-      'arboard',
-      'syntect-tui',
-    ],
-  },
-  {
-    crateName: 'harness',
-    reason:
-      'harness must own workflow contracts without concrete service, product, or platform implementations',
-    forbiddenDeps: [
-      'northhing-core',
-      'northhing-ai-adapters',
-      'northhing-services-core',
-      'northhing-services-integrations',
-      'northhing-tool-packs',
       'northhing-product-domains',
       'northhing-transport',
       'terminal-core',
       'tauri',
       'reqwest',
       'git2',
       'rmcp',
       'image',
       'tokio-tungstenite',
       'northhing-cli',
@@ -184,21 +152,20 @@ export const lightweightBoundaryRules = [
     ],
   },
   {
     crateName: 'agent-tools',
     reason: 'agent-tools must not depend on concrete service or product runtime implementations',
     forbiddenDeps: [
       'northhing-core',
       'northhing-ai-adapters',
       'northhing-services-core',
       'northhing-services-integrations',
-      'northhing-tool-packs',
       'northhing-product-domains',
       'northhing-transport',
       'terminal-core',
       'tool-runtime',
       'tauri',
       'reqwest',
       'git2',
       'rmcp',
       'tokio-tungstenite',
       'northhing-cli',
@@ -214,21 +181,20 @@ export const dependencyProfileRules = [
   {
     crateName: 'core',
     profileName: 'no-default runtime-surface-light profile',
     reason:
       'northhing-core no-default profile must not force product/runtime integration dependencies',
     forbiddenNonOptionalDeps: [
       'aes',
       'aes-gcm',
       'northhing-product-capabilities',
       'northhing-product-domains',
-      'northhing-tool-packs',
       'chrono-tz',
       'cron',
       'dashmap',
       'eventsource-stream',
       'filetime',
       'flate2',
       'fs2',
       'git2',
       'glob',
       'globset',
@@ -256,21 +222,20 @@ export const dependencyProfileRules = [
     reason: 'core-types default profile must stay DTO-only',
     forbiddenNonOptionalDeps: [
       'northhing-core',
       'northhing-events',
       'northhing-ai-adapters',
       'northhing-agent-stream',
       'northhing-runtime-ports',
       'northhing-services-core',
       'northhing-services-integrations',
       'northhing-agent-tools',
-      'northhing-tool-packs',
       'northhing-product-domains',
       'northhing-transport',
       'terminal-core',
       'tool-runtime',
       'tauri',
       'reqwest',
       'git2',
       'rmcp',
       'image',
       'tokio-tungstenite',
@@ -285,21 +250,20 @@ export const dependencyProfileRules = [
     crateName: 'runtime-ports',
     profileName: 'default ports profile',
     reason: 'runtime-ports default profile must stay trait/DTO-only',
     forbiddenNonOptionalDeps: [
       'northhing-core',
       'northhing-ai-adapters',
       'northhing-agent-stream',
       'northhing-services-core',
       'northhing-services-integrations',
       'northhing-agent-tools',
-      'northhing-tool-packs',
       'northhing-product-domains',
       'northhing-transport',
       'terminal-core',
       'tool-runtime',
       'tauri',
       'reqwest',
       'git2',
       'rmcp',
       'image',
       'tokio-tungstenite',
@@ -314,21 +278,20 @@ export const dependencyProfileRules = [
     crateName: 'runtime-services',
     profileName: 'default runtime service assembly profile',
     reason: 'runtime-services default profile must not compile concrete service or product runtime implementations',
     forbiddenNonOptionalDeps: [
       'northhing-core',
       'northhing-ai-adapters',
       'northhing-agent-stream',
       'northhing-services-core',
       'northhing-services-integrations',
       'northhing-agent-tools',
-      'northhing-tool-packs',
       'northhing-product-domains',
       'northhing-transport',
       'terminal-core',
       'tool-runtime',
       'tauri',
       'reqwest',
       'git2',
       'rmcp',
       'image',
       'tokio-tungstenite',
@@ -341,21 +304,20 @@ export const dependencyProfileRules = [
   },
   {
     crateName: 'agent-runtime',
     profileName: 'default agent runtime decision profile',
     reason: 'agent-runtime default profile must not compile concrete services or product surfaces',
     forbiddenNonOptionalDeps: [
       'northhing-core',
       'northhing-ai-adapters',
       'northhing-services-core',
       'northhing-services-integrations',
-      'northhing-tool-packs',
       'northhing-product-domains',
       'northhing-transport',
       'terminal-core',
       'tauri',
       'reqwest',
       'git2',
       'rmcp',
       'image',
       'tokio-tungstenite',
       'northhing-cli',
diff --git a/scripts/core-boundaries/rules/feature-rules.mjs b/scripts/core-boundaries/rules/feature-rules.mjs
index ef2e8bf..3abcda4 100644
--- a/scripts/core-boundaries/rules/feature-rules.mjs
+++ b/scripts/core-boundaries/rules/feature-rules.mjs
@@ -5,21 +5,20 @@ export const optionalDependencyFeatureOwnerRules = [
     crateName: 'core',
     reason:
       'northhing-core product/runtime optional dependencies must stay owned by explicit feature gates',
     dependencies: [
       { depName: 'aes', ownerFeatures: ['service-integrations'] },
       { depName: 'aes-gcm', ownerFeatures: ['service-integrations'] },
       { depName: 'axum', ownerFeatures: ['service-integrations'] },
       { depName: 'northhing-ai-adapters', ownerFeatures: ['ai-adapter-runtime'] },
       { depName: 'northhing-product-capabilities', ownerFeatures: ['product-capabilities'] },
       { depName: 'northhing-product-domains', ownerFeatures: ['product-domains'] },
-      { depName: 'northhing-tool-packs', ownerFeatures: ['tool-packs'] },
       { depName: 'chrono-tz', ownerFeatures: ['product-full'] },
       { depName: 'cron', ownerFeatures: ['product-full'] },
       { depName: 'dashmap', ownerFeatures: ['product-full'] },
       { depName: 'eventsource-stream', ownerFeatures: ['product-full'] },
       { depName: 'filetime', ownerFeatures: ['product-full'] },
       { depName: 'flate2', ownerFeatures: ['product-full'] },
       { depName: 'fs2', ownerFeatures: ['product-full'] },
       { depName: 'git2', ownerFeatures: ['service-integrations'] },
       { depName: 'glob', ownerFeatures: ['product-full'] },
       { depName: 'globset', ownerFeatures: ['product-full'] },
@@ -128,40 +127,25 @@ export const productCoreFeatureAssemblyScanRoots = [
 ];
 
 export const coreProductFullFeatureAssemblyRule = {
   manifestPath: 'src/crates/assembly/core/Cargo.toml',
   featureName: 'product-full',
   requiredFeatureRefs: [
     'ssh-remote',
     'product-capabilities',
     'product-domains',
     'service-integrations',
-    'tool-packs',
   ],
   reason: 'northhing-core product-full must explicitly assemble current owner feature groups',
 };
 
 export const ownerCrateFeatureAssemblyRules = [
-  {
-    manifestPath: 'src/crates/execution/tool-provider-groups/Cargo.toml',
-    reason: 'tool-packs must keep product feature groups explicit and default-light',
-    requiredProductFullFeatures: [
-      'basic',
-      'git',
-      'mcp',
-      'browser-web',
-      'computer-use',
-      'image-analysis',
-      'miniapp',
-      'agent-control',
-    ],
-  },
   {
     manifestPath: 'src/crates/services/services-integrations/Cargo.toml',
     reason: 'services-integrations must keep integration feature groups explicit and default-light',
     requiredProductFullFeatures: [
       'announcement',
       'deep-research',
       'file-watch',
       'function-agents',
       'git',
       'miniapp-runtime',
diff --git a/scripts/core-boundaries/rules/source/forbidden-rules.mjs b/scripts/core-boundaries/rules/source/forbidden-rules.mjs
index 11ab16a..d09ae7c 100644
--- a/scripts/core-boundaries/rules/source/forbidden-rules.mjs
+++ b/scripts/core-boundaries/rules/source/forbidden-rules.mjs
@@ -29,35 +29,20 @@ export const forbiddenContentRules = [
         message:
           'core product_assembly must not own runtime service provider registration; use the product runtime adapter path',
       },
       {
         regex: /\bCoreSessionStorePort\b/,
         message:
           'core product_assembly must not bind concrete session store adapters directly; use the product runtime adapter path',
       },
     ],
   },
-  {
-    path: 'src/crates/assembly/core/src/agentic/harness.rs',
-    patterns: [
-      {
-        regex: /\bproduct_assembly_plan_for_profile\b/,
-        message:
-          'core agentic harness facade must not rebuild product assembly plans; use northhing-product-capabilities harness registry entrypoints',
-      },
-      {
-        regex: /\bfn product_harness_registry_for_profile\b/,
-        message:
-          'core agentic harness facade must not own profile-scoped harness registry construction',
-      },
-    ],
-  },
   {
     path: 'src/crates/assembly/core/src/agentic/persistence/session_branch.rs',
     patterns: [
       {
         regex: /\bfn\s+estimate_turn_message_count\b/,
         message:
           'session branch metadata counting belongs in services-core session lineage owner, not core persistence',
       },
       {
         regex: /\bfn\s+strip_child_session_metadata\b/,
@@ -544,50 +529,20 @@ export const forbiddenContentRules = [
         message:
           'core function-agent runtime services must not re-own Git status snapshots; use northhing-services-integrations::function_agents',
       },
       {
         regex: /\bcreate_command\("git"\)/,
         message:
           'core function-agent runtime services must not spawn Git concrete commands; use northhing-services-integrations::function_agents',
       },
     ],
   },
-  {
-    path: 'src/crates/assembly/product-capabilities/src/lib.rs',
-    patterns: [
-      {
-        regex: /\bpub struct HarnessProviderDescriptor\b/,
-        message:
-          'product-capabilities must not redefine provider-neutral harness descriptors; use northhing-harness',
-      },
-      {
-        regex: /\bfn build_harness_registry_from_descriptors\b/,
-        message:
-          'product-capabilities must not own descriptor registry construction; use northhing-harness',
-      },
-      {
-        regex: /\bpub enum ProductCapabilityBuildError\b/,
-        message:
-          'product-capabilities must not redefine tool provider group selection errors; use northhing-tool-packs',
-      },
-      {
-        regex: /\bproduct_tool_provider_group_plan\(\)\b/,
-        message:
-          'product-capabilities must not scan product tool provider plans locally; use northhing-tool-packs selector',
-      },
-      {
-        regex: /\bdefault_product_tool_provider_group_plan\b/,
-        message:
-          'product-capabilities must expose product assembly, not a separate default tool-provider plan shortcut',
-      },
-    ],
-  },
   {
     path: 'src/crates/assembly/core/src/service/filesystem/service.rs',
     patterns: [
       {
         regex: /\btokio::fs::/,
         message:
           'core filesystem service must not own async local filesystem IO; use northhing-services-core filesystem primitives',
       },
       {
         regex: /\bstd::fs::/,
@@ -2351,47 +2306,20 @@ export const forbiddenContentUnderRules = [
       {
         regex: /\bunlocked_collapsed_tools\b/,
         message: 'collapsed-tool unlock state stays in core ToolUseContext/runtime',
       },
       {
         regex: /\bToolUseContext\b/,
         message: 'ToolUseContext stays in core until a portable context port is reviewed',
       },
     ],
   },
-  {
-    path: 'src/crates/execution/tool-provider-groups/src',
-    reason:
-      'tool-packs may own provider group plans, but not product tool manifest/exposure or GetToolSpec runtime',
-    patterns: [
-      {
-        regex: /\bGetToolSpecTool\b/,
-        message: 'GetToolSpec implementation stays in core product tool runtime',
-      },
-      {
-        regex: /\bGET_TOOL_SPEC_TOOL_NAME\b/,
-        message: 'GetToolSpec manifest insertion stays in core product tool runtime',
-      },
-      {
-        regex: /\bmanifest_resolver\b/,
-        message: 'tool manifest resolution stays in core product tool runtime',
-      },
-      {
-        regex: /\bunlocked_collapsed_tools\b/,
-        message: 'collapsed-tool unlock state stays in core ToolUseContext/runtime',
-      },
-      {
-        regex: /\bToolExposure\b/,
-        message: 'expanded/collapsed exposure policy stays in core until provider migration',
-      },
-    ],
-  },
   {
     path: 'src/crates/assembly/core/src/agentic/tools/implementations',
     reason:
       'GetToolSpec concrete adapter belongs in the product tool runtime owner, not the generic concrete-tool implementations module',
     patterns: [
       {
         regex: /\bpub(?:\(crate\))? struct GetToolSpecTool\b/,
         message: 'move GetToolSpecTool into core product_runtime owner',
       },
     ],
diff --git a/scripts/core-boundaries/rules/source/required-rules.mjs b/scripts/core-boundaries/rules/source/required-rules.mjs
index 743fa57..1e098dc 100644
--- a/scripts/core-boundaries/rules/source/required-rules.mjs
+++ b/scripts/core-boundaries/rules/source/required-rules.mjs
@@ -528,52 +528,25 @@ export const requiredContentRules = [
       {
         regex: /\bprompt_cache_lookup_preserves_identity_and_expiry_semantics\b/,
         message: 'missing prompt-cache identity/expiry regression',
       },
       {
         regex: /\bprompt_cache_scope_key_preserves_legacy_mode_switch_shape\b/,
         message: 'missing prompt-cache scope-key shape regression',
       },
     ],
   },
-  {
-    path: 'src/crates/execution/harness/src/lib.rs',
-    reason:
-      'harness must own provider-neutral harness descriptors and descriptor registry wiring without concrete execution',
-    patterns: [
-      {
-        regex: /\bpub struct HarnessProviderDescriptor\b/,
-        message: 'missing provider-neutral harness provider descriptor',
-      },
-      {
-        regex: /\bpub fn build_descriptor_harness_registry\b/,
-        message: 'missing descriptor harness registry builder',
-      },
-      {
-        regex: /\bDescriptorHarnessProvider::legacy_facade\b/,
-        message: 'missing legacy-facade descriptor adapter',
-      },
-    ],
-  },
   {
     path: 'src/crates/assembly/product-capabilities/src/lib.rs',
     reason:
-      'product-capabilities must select harness descriptors from the harness owner instead of owning descriptor construction',
+      'product-capabilities must own product capability assembly without concrete execution',
     patterns: [
-      {
-        regex: /\bHarnessProviderDescriptor\b/,
-        message: 'missing harness descriptor selection in product capability packs',
-      },
-      {
-        regex: /\bbuild_descriptor_harness_registry\b/,
-        message: 'missing harness-owned descriptor registry assembly delegation',
-      },
       {
         regex: /\bProductCapabilityAssembly\b/,
         message: 'missing product capability assembly owner',
       },
     ],
   },
   {
     path: 'src/crates/execution/agent-runtime/src/agents.rs',
     reason:
       'agent-runtime must own shared mode config profile facts that are runtime-visible and product-neutral',
@@ -2440,25 +2413,20 @@ export const requiredContentRules = [
         regex: /\.flush\(\)\s*\.await/,
         message: 'missing local MCP stdin flush',
       },
     ],
   },
   {
     path: 'src/crates/assembly/core/Cargo.toml',
     reason:
       'northhing-core product-full must explicitly aggregate owner crate feature groups instead of forcing them through dependency declarations',
     patterns: [
-      {
-        regex:
-          /northhing-tool-packs = \{ path = "\.\.\/\.\.\/execution\/tool-provider-groups", default-features = false, optional = true \}/,
-        message: 'northhing-tool-packs dependency must stay optional and not force product-full outside the core feature graph',
-      },
       {
         regex:
           /northhing-services-integrations = \{ path = "\.\.\/\.\.\/services\/services-integrations", default-features = false, features = \["remote-ssh"\] \}/,
         message:
           'northhing-services-integrations dependency may keep remote workspace identity but must not force workspace-search or product-full outside the core feature graph',
       },
       {
         regex:
           /northhing-ai-adapters = \{ path = "\.\.\/\.\.\/adapters\/ai-adapters", optional = true \}/,
         message: 'northhing-ai-adapters dependency must stay optional for no-default core builds',
@@ -2488,28 +2456,20 @@ export const requiredContentRules = [
           /northhing-product-domains = \{ path = "\.\.\/\.\.\/contracts\/product-domains", default-features = false, optional = true \}/,
         message:
           'northhing-product-domains dependency must stay optional and not force product-full outside the core feature graph',
       },
       {
         regex:
           /northhing-product-capabilities = \{ path = "\.\.\/product-capabilities", default-features = false, optional = true \}/,
         message:
           'northhing-product-capabilities dependency must stay optional and not force product-full outside the core feature graph',
       },
-      {
-        regex: /"dep:northhing-tool-packs"/,
-        message: 'core tool-packs feature must explicitly enable the optional dependency',
-      },
-      {
-        regex: /"northhing-tool-packs\/product-full"/,
-        message: 'core product-full must explicitly enable tool pack product features',
-      },
       {
         regex: /"northhing-services-integrations\/product-full"/,
         message: 'core product-full must explicitly enable integration product features',
       },
       {
         regex: /"dep:northhing-product-domains"/,
         message: 'core product-domains feature must explicitly enable the optional dependency',
       },
       {
         regex: /"dep:northhing-product-capabilities"/,
@@ -5206,22 +5166,22 @@ export const requiredContentRules = [
       },
       {
         regex: /\bSnapshotToolDecorator\b/,
         message: 'missing generic snapshot decorator injection',
       },
       {
         regex: /\bcreate_product_tool_registry_from_plan\b/,
         message: 'missing product registry assembly adapter delegation',
       },
       {
-        regex: /\bproduct_assembly_plan_for_profile\b/,
-        message: 'missing product assembly plan provider group plan delegation',
+        regex: /\bPRODUCT_TOOL_GROUPS\b/,
+        message: 'missing product tool groups inline constant',
       },
       {
         regex: /\bproduct_tool_runtime_owner_preserves_registry_contract\b/,
         message: 'missing product runtime owner registry equivalence regression',
       },
       {
         regex: /\bproduct_tool_runtime_registry_preserves_provider_plan_order\b/,
         message: 'missing product tool provider plan-to-registry order regression',
       },
     ],
@@ -5479,59 +5439,20 @@ export const requiredContentRules = [
       {
         regex: /\bpub async fn resolve_get_tool_spec_execution_result_from_provider\b/,
         message: 'missing provider-backed GetToolSpec execution result helper',
       },
       {
         regex: /\bpub struct GetToolSpecRuntime\b/,
         message: 'missing provider-backed GetToolSpec runtime facade',
       },
     ],
   },
-  {
-    path: 'src/crates/execution/tool-provider-groups/src/lib.rs',
-    reason:
-      'tool-packs must keep its feature-group scaffold explicit without owning concrete tools yet',
-    patterns: [
-      {
-        regex: /\bpub enum ToolPackFeatureGroup\b/,
-        message: 'missing tool-pack feature group scaffold',
-      },
-      {
-        regex: /\bpub fn all_feature_groups\b/,
-        message: 'missing tool-pack full feature group metadata helper',
-      },
-      {
-        regex: /\bpub fn enabled_feature_groups\b/,
-        message: 'missing tool-pack compile-time feature metadata helper',
-      },
-      {
-        regex: /\bpub struct ToolProviderGroupPlan\b/,
-        message: 'missing tool-pack provider group plan contract',
-      },
-      {
-        regex: /\bpub fn product_tool_provider_group_plan\b/,
-        message: 'missing product tool provider group plan',
-      },
-      {
-        regex: /\bpub enum ToolProviderGroupPlanSelectionError\b/,
-        message: 'missing tool provider group plan selection error',
-      },
-      {
-        regex: /\bpub fn try_product_tool_provider_group_plan_for_ids\b/,
-        message: 'missing product tool provider group plan selector',
-      },
-      {
-        regex: /\bproduct_provider_group_plan_selector_rejects_unknown_provider_ids\b/,
-        message: 'missing provider group selector unknown-id regression',
-      },
-    ],
-  },
   {
     path: 'src/crates/assembly/core/src/agentic/tools/manifest_resolver.rs',
     reason:
       'core must continue owning manifest resolver wrappers while delegating product catalog access and generic manifest assembly',
     patterns: [
       {
         regex: /\bpub async fn resolve_tool_manifest\b/,
         message: 'missing tool manifest resolver owner',
       },
       {
diff --git a/scripts/core-boundaries/self-test.mjs b/scripts/core-boundaries/self-test.mjs
index b3f5037..b8c0b38 100644
--- a/scripts/core-boundaries/self-test.mjs
+++ b/scripts/core-boundaries/self-test.mjs
@@ -123,21 +123,20 @@ export function runManifestParserSelfTest({
   for (const rule of productCoreFeatureAssemblyRules) {
     if (!rule.requiredFeatures.includes('product-full')) {
       throw new Error(`${rule.manifestPath} must require northhing-core product-full`);
     }
   }
   for (const featureName of [
     'ssh-remote',
     'product-capabilities',
     'product-domains',
     'service-integrations',
-    'tool-packs',
   ]) {
     if (!coreProductFullFeatureAssemblyRule.requiredFeatureRefs.includes(featureName)) {
       throw new Error(`core product-full assembly rule must require ${featureName}`);
     }
   }
   const discoveredProductCoreManifests = collectProductCoreDependencyManifestPaths([
     {
       manifestPath: 'src/apps/desktop/Cargo.toml',
       text:
         '[dependencies]\nnorthhing-core = { path = "../../crates/assembly/core", default-features = false, features = ["product-full"] }',
@@ -151,21 +150,20 @@ export function runManifestParserSelfTest({
       text: '[dependencies."northhing-core"]\npath = "../../assembly/core"\ndefault-features = false\nfeatures = ["product-full"]',
     },
   ]);
   if (discoveredProductCoreManifests.join(',') !== 'src/apps/desktop/Cargo.toml,src/crates/interfaces/acp/Cargo.toml') {
     throw new Error('product core dependency scanner must discover only manifests that depend on northhing-core');
   }
   const ownerFeatureRulePaths = new Set(
     ownerCrateFeatureAssemblyRules.map((rule) => rule.manifestPath),
   );
   for (const manifestPath of [
-    'src/crates/execution/tool-provider-groups/Cargo.toml',
     'src/crates/services/services-integrations/Cargo.toml',
     'src/crates/contracts/product-domains/Cargo.toml',
   ]) {
     if (!ownerFeatureRulePaths.has(manifestPath)) {
       throw new Error(`owner crate feature assembly rule must cover ${manifestPath}`);
     }
   }
   for (const rule of ownerCrateFeatureAssemblyRules) {
     const declaredFeatures = new Set(rule.requiredProductFullFeatures);
     if (declaredFeatures.size !== rule.requiredProductFullFeatures.length) {
@@ -703,41 +701,20 @@ export function runManifestParserSelfTest({
     'ToolUseContext',
   ];
   const agentToolsManifestRuleText = agentToolsManifestRule.patterns
     .map((pattern) => pattern.regex.source)
     .join('\n');
   for (const contract of agentToolsRuntimeForbiddenContracts) {
     if (!agentToolsManifestRuleText.includes(contract)) {
       throw new Error(`agent-tools manifest boundary rule must forbid: ${contract}`);
     }
   }
-  const toolPacksManifestRule = forbiddenContentUnderRules.find(
-    (rule) => rule.path === 'src/crates/execution/tool-provider-groups/src',
-  );
-  if (!toolPacksManifestRule) {
-    throw new Error('missing tool-packs manifest-owner boundary rule');
-  }
-  const toolPacksManifestRuleText = toolPacksManifestRule.patterns
-    .map((pattern) => pattern.regex.source)
-    .join('\n');
-  const toolPacksManifestContracts = [
-    'GetToolSpecTool',
-    'GET_TOOL_SPEC_TOOL_NAME',
-    'manifest_resolver',
-    'unlocked_collapsed_tools',
-    'ToolExposure',
-  ];
-  for (const contract of toolPacksManifestContracts) {
-    if (!toolPacksManifestRuleText.includes(contract)) {
-      throw new Error(`tool-packs manifest boundary rule must forbid: ${contract}`);
-    }
-  }
   const serviceAgentRuntimeRuleText = forbiddenRuleTextForPath(
     'src/crates/assembly/core/src/service_agent_runtime.rs',
   );
   if (!serviceAgentRuntimeRuleText.includes('self\\.scheduler')) {
     throw new Error('service agent runtime boundary rule must forbid direct scheduler submit');
   }
   const sessionMessageRuleText = forbiddenRuleTextForPath(
     'src/crates/assembly/core/src/agentic/tools/implementations/session_message_tool.rs',
   );
   if (!sessionMessageRuleText.includes('submit_with_prepended_messages')) {
@@ -1961,33 +1938,20 @@ export function runManifestParserSelfTest({
         'install_static_provider',
         'resolve_readonly_enabled_tools',
         'build_get_tool_spec_duplicate_load_result',
         'build_get_tool_spec_detail_result',
         'resolve_get_tool_spec_execution_plan',
         'resolve_get_tool_spec_execution_result_from_provider',
         'GetToolSpecRuntime',
         'call_results',
       ],
     },
-    {
-      path: 'src/crates/execution/tool-provider-groups/src/lib.rs',
-      contracts: [
-        'ToolPackFeatureGroup',
-        'ToolProviderGroupPlan',
-        'all_feature_groups',
-        'enabled_feature_groups',
-        'product_tool_provider_group_plan',
-        'ToolProviderGroupPlanSelectionError',
-        'try_product_tool_provider_group_plan_for_ids',
-        'product_provider_group_plan_selector_rejects_unknown_provider_ids',
-      ],
-    },
     {
       path: 'src/crates/assembly/core/src/agentic/tools/tool_adapter.rs',
       contracts: [
         'ToolRegistryItem',
         'ContextualToolManifestItem',
         'Tool::dynamic_tool_info',
         'Tool::is_readonly',
         'Tool::is_enabled',
         'Tool::description_with_context',
         'Tool::input_schema_for_model_with_context',
@@ -2307,30 +2271,27 @@ export function runManifestParserSelfTest({
     },
     {
       path: 'src/crates/assembly/core/src/service/search/remote_disabled.rs',
       contracts: ['Remote SSH search is disabled', 'RemoteWorkspaceSearchService', 'remote_workspace_search_service_for_path'],
     },
     {
       path: 'src/crates/assembly/core/Cargo.toml',
       contracts: [
         'northhing-product-capabilities = \\{ path = "\\.\\.\\/product-capabilities", default-features = false, optional = true \\}',
         'northhing-ai-adapters = \\{ path = "\\.\\.\\/\\.\\.\\/adapters\\/ai-adapters", optional = true \\}',
-        'northhing-tool-packs = \\{ path = "\\.\\.\\/\\.\\.\\/execution\\/tool-provider-groups", default-features = false, optional = true \\}',
         'northhing-services-integrations = \\{ path = "\\.\\.\\/\\.\\.\\/services\\/services-integrations", default-features = false, features = \\["remote-ssh"\\] \\}',
         'northhing-product-domains = \\{ path = "\\.\\.\\/\\.\\.\\/contracts\\/product-domains", default-features = false, optional = true \\}',
         'dep:northhing-ai-adapters',
         'ai-adapter-runtime',
         'northhing-services-integrations\\/function-agents',
         'northhing-services-integrations\\/miniapp-runtime',
         'dep:northhing-product-capabilities',
-        'dep:northhing-tool-packs',
-        'northhing-tool-packs\\/product-full',
         'northhing-services-integrations\\/product-full',
         'dep:northhing-product-domains',
         'northhing-product-domains\\/product-full',
       ],
     },
     {
       path: 'src/crates/assembly/core/src/lib.rs',
       contracts: [
         'feature = "product-full"',
         'pub mod agentic',
diff --git a/src/crates/assembly/core/AGENTS-CN.md b/src/crates/assembly/core/AGENTS-CN.md
index e3daa86..f749456 100644
--- a/src/crates/assembly/core/AGENTS-CN.md
+++ b/src/crates/assembly/core/AGENTS-CN.md
@@ -42,26 +42,24 @@ SessionManager -> Session -> DialogTurn -> ModelRound
 - 功能工作必须把 `product-full` 保留为兼容性产品装配边界，除非独立的产品矩阵评审改变了默认的能力选择。
 
 ## 所有者参考
 
 需要所有权细节时请使用以下文件，而不是扩展本指南：
 
 - `docs/architecture/core-decomposition.md`
 - `docs/architecture/agent-runtime-services-design.md`
 - `src/crates/execution/agent-runtime/AGENTS.md`
 - `src/crates/execution/tool-contracts/AGENTS.md`
-- `src/crates/execution/harness/AGENTS.md`
 - `src/crates/contracts/product-domains/AGENTS.md`
 - `src/crates/contracts/runtime-ports/` 以及 `src/crates/execution/runtime-services/` 的源码文档
 - `src/crates/services/services-core/AGENTS.md`
 - `src/crates/services/services-integrations/AGENTS.md`
-- `src/crates/execution/tool-provider-groups/AGENTS.md`
 
 一些子树已存在更精细的本地指南：
 
 - `src/crates/adapters/ai-adapters/AGENTS.md`
 - `src/agentic/execution/AGENTS.md`
 - `src/agentic/deep_review/AGENTS.md`
 
 ## 验证
 
 使用匹配所触及行为的最小检查：
diff --git a/src/crates/assembly/core/AGENTS.md b/src/crates/assembly/core/AGENTS.md
index 9f2f0c6..b82d692 100644
--- a/src/crates/assembly/core/AGENTS.md
+++ b/src/crates/assembly/core/AGENTS.md
@@ -68,26 +68,24 @@ SessionManager -> Session -> DialogTurn -> ModelRound
   selection.
 
 ## Owner References
 
 Use these files for ownership details instead of expanding this guide:
 
 - `docs/architecture/core-decomposition.md`
 - `docs/architecture/agent-runtime-services-design.md`
 - `src/crates/execution/agent-runtime/AGENTS.md`
 - `src/crates/execution/tool-contracts/AGENTS.md`
-- `src/crates/execution/harness/AGENTS.md`
 - `src/crates/contracts/product-domains/AGENTS.md`
 - `src/crates/contracts/runtime-ports/` and `src/crates/execution/runtime-services/` source docs
 - `src/crates/services/services-core/AGENTS.md`
 - `src/crates/services/services-integrations/AGENTS.md`
-- `src/crates/execution/tool-provider-groups/AGENTS.md`
 
 Narrower local guides already exist for some subtrees:
 
 - `src/crates/adapters/ai-adapters/AGENTS.md`
 - `src/agentic/execution/AGENTS.md`
 - `src/agentic/deep_review/AGENTS.md`
 
 ## Verification
 
 Use the smallest check that matches the touched behavior:
diff --git a/src/crates/assembly/core/Cargo.toml b/src/crates/assembly/core/Cargo.toml
index 72600d6..9ee4aca 100644
--- a/src/crates/assembly/core/Cargo.toml
+++ b/src/crates/assembly/core/Cargo.toml
@@ -80,32 +80,26 @@ northhing-ai-adapters = { path = "../../adapters/ai-adapters", optional = true }
 
 # Lightweight agent stream processing
 northhing-agent-stream = { path = "../../execution/agent-stream" }
 
 # Agent runtime owner contracts
 northhing-agent-runtime = { path = "../../execution/agent-runtime" }
 
 # Agent dispatch (SkillActor / LongRunningSkill runtime) — K.2.3
 northhing-agent-dispatch = { path = "../../execution/agent-dispatch" }
 
-# Harness workflow contracts
-northhing-harness = { path = "../../execution/harness" }
-
 # Product capability pack contracts
 northhing-product-capabilities = { path = "../product-capabilities", default-features = false, optional = true }
 
 # Agent tool contracts
 northhing-agent-tools = { path = "../../execution/tool-contracts" }
 
-# Tool pack provider plan
-northhing-tool-packs = { path = "../../execution/tool-provider-groups", default-features = false, optional = true }
-
 # Core service owner crate
 northhing-services-core = { path = "../../services/services-core" }
 
 # Integration service owner crate
 northhing-services-integrations = { path = "../../services/services-integrations", default-features = false, features = ["remote-ssh"] }
 
 # Debug-mode runtime logging leaf crate (K4a-T5: log_event + COMP_* constants)
 northhing-debug-log = { path = "../../services/debug-log", optional = true }
 
 # Product domain owner crate
@@ -193,21 +187,20 @@ product-full = [
     "dep:indexmap",
     "dep:md5",
     "dep:northhing-debug-log",
     "dep:similar",
     "dep:tool-runtime",
     "ssh-remote",
     "product-capabilities",
     "product-domains",
     "runtime-services",
     "service-integrations",
-    "tool-packs",
 ]
 ai-adapter-runtime = ["dep:northhing-ai-adapters", "dep:reqwest"]
 product-capabilities = ["dep:northhing-product-capabilities"]
 product-domains = [
     "ai-adapter-runtime",
     "dep:northhing-product-domains",
     "northhing-services-integrations/function-agents",
     "northhing-services-integrations/miniapp-runtime",
     "northhing-product-domains/product-full",
 ]
@@ -222,21 +215,20 @@ service-integrations = [
     "dep:local-ip-address",
     "dep:md5",
     "dep:rand",
     "dep:reqwest",
     "dep:rmcp",
     "dep:sse-stream",
     "dep:tokio-tungstenite",
     "dep:tower-http",
     "northhing-services-integrations/product-full",
 ]
-tool-packs = ["dep:northhing-tool-packs", "northhing-tool-packs/product-full"]
 tauri-support = ["tauri"]  # Optional tauri support
 ssh-remote = [
     "northhing-services-integrations/remote-ssh-concrete",
     "russh",
 ]
 # QuickJS-based readability extraction (optional to avoid native DLL issues on Windows)
 readability-js = ["dep:readability-js"]
 
 [build-dependencies]
 sha2 = { workspace = true }
diff --git a/src/crates/assembly/core/src/agentic/harness.rs b/src/crates/assembly/core/src/agentic/harness.rs
deleted file mode 100644
index c711ad9..0000000
--- a/src/crates/assembly/core/src/agentic/harness.rs
+++ /dev/null
@@ -1,68 +0,0 @@
-pub use northhing_product_capabilities::{
-    default_product_harness_registry as product_harness_registry, product_harness_registry_for_profile,
-    CORE_DEEP_RESEARCH_HARNESS_PROVIDER_ID, CORE_DEEP_REVIEW_HARNESS_PROVIDER_ID, CORE_MINIAPP_HARNESS_PROVIDER_ID,
-};
-
-#[cfg(test)]
-mod tests {
-    use super::*;
-    use northhing_harness::{HarnessInput, HarnessStepKind, HarnessWorkflow};
-    use northhing_product_capabilities::DeliveryProfile;
-
-    #[test]
-    fn product_harness_registry_registers_existing_workflow_facades() {
-        let registry = product_harness_registry().expect("product harness registry should build");
-
-        assert_eq!(
-            registry.provider_ids(),
-            vec!["core.deep_review", "core.deep_research", "core.miniapp"]
-        );
-        assert_eq!(
-            registry.workflows(),
-            vec![
-                HarnessWorkflow::DeepReview,
-                HarnessWorkflow::DeepResearch,
-                HarnessWorkflow::MiniApp,
-            ]
-        );
-    }
-
-    #[tokio::test]
-    async fn product_harness_provider_plans_route_to_legacy_facade_without_execution() {
-        let registry = product_harness_registry().expect("product harness registry should build");
-        let provider = registry
-            .provider_for_workflow(HarnessWorkflow::DeepResearch)
-            .expect("DeepResearch should be registered");
-
-        let plan = provider
-            .plan(
-                Default::default(),
-                HarnessInput::new(HarnessWorkflow::DeepResearch, "research current question"),
-            )
-            .await
-            .expect("DeepResearch harness should produce a legacy route plan");
-
-        assert_eq!(plan.steps().len(), 1);
-        assert_eq!(plan.steps()[0].kind(), HarnessStepKind::LegacyFacade);
-        assert_eq!(
-            plan.steps()[0].target(),
-            "northhing-core::agentic::agents::definitions::modes::deep_research"
-        );
-
-        assert!(
-            provider.execute(Default::default(), plan).await.is_err(),
-            "PR4 must not move concrete workflow execution out of legacy paths"
-        );
-    }
-
-    #[test]
-    fn product_harness_registry_can_be_built_from_explicit_delivery_profile() {
-        let registry = product_harness_registry_for_profile(DeliveryProfile::Cli)
-            .expect("profile-scoped product harness registry should build");
-
-        assert_eq!(
-            registry.provider_ids(),
-            vec!["core.deep_review", "core.deep_research", "core.miniapp"]
-        );
-    }
-}
diff --git a/src/crates/assembly/core/src/agentic/mod.rs b/src/crates/assembly/core/src/agentic/mod.rs
index 74838e0..d9d922f 100644
--- a/src/crates/assembly/core/src/agentic/mod.rs
+++ b/src/crates/assembly/core/src/agentic/mod.rs
@@ -16,21 +16,20 @@ pub mod session;
 pub mod execution;
 
 // Tools module
 pub mod tools;
 
 // Coordination module
 pub mod context_profile;
 pub mod coordination;
 pub mod deep_review;
 pub mod deep_review_policy;
-pub mod harness;
 pub(crate) mod subagent_runtime;
 
 // Shared-context fork-agent execution module
 pub mod fork_agent;
 
 pub(crate) mod remote_file_delivery;
 /// Round-boundary injection support for steering/background updates
 pub mod round_preempt;
 
 // Image analysis module
diff --git a/src/crates/assembly/core/src/agentic/tools/product_runtime.rs b/src/crates/assembly/core/src/agentic/tools/product_runtime.rs
index 31759b2..c506915 100644
--- a/src/crates/assembly/core/src/agentic/tools/product_runtime.rs
+++ b/src/crates/assembly/core/src/agentic/tools/product_runtime.rs
@@ -4,91 +4,69 @@
 //! registry adapters, catalog manifests, GetToolSpec lookup, and snapshot
 //! decoration. Concrete tools and `ToolUseContext` stay in core so this owner
 //! remains an equivalent structural boundary rather than a behavior migration.
 
 mod catalog;
 mod get_tool_spec_tool;
 mod materialization;
 mod snapshot;
 mod unlock_state;
 
+pub(in crate::agentic::tools) use materialization::PRODUCT_TOOL_GROUPS;
+
 use crate::agentic::tools::registry::{ProductToolDecoratorRef, ToolRegistry};
 use materialization::create_product_tool_registry_from_plan;
 use northhing_agent_tools::SnapshotToolDecorator;
-use northhing_product_capabilities::{product_assembly_plan_for_profile, DeliveryProfile, ProductAssemblyPlan};
 use snapshot::ProductSnapshotToolWrapper;
 use std::sync::Arc;
 
 pub(crate) use catalog::{
     product_get_tool_spec_runtime, resolve_product_get_tool_spec_results, resolve_product_readonly_enabled_tools,
     resolve_product_resolved_tool_manifest, resolve_product_resolved_visible_tools, ProductGetToolSpecRuntime,
     ProductToolCatalogProvider,
 };
 pub use catalog::{ResolvedToolManifest, ResolvedVisibleTools};
 pub use get_tool_spec_tool::GetToolSpecTool;
 pub(crate) use unlock_state::collect_product_unlocked_collapsed_tools;
 
 #[derive(Clone)]
 pub(crate) struct ProductToolRuntime {
     tool_decorator: ProductToolDecoratorRef,
-    assembly_plan: ProductAssemblyPlan,
 }
 
 impl Default for ProductToolRuntime {
     fn default() -> Self {
         Self::new()
     }
 }
 
 impl ProductToolRuntime {
     pub(crate) fn new() -> Self {
-        Self::for_profile(DeliveryProfile::ProductFull)
-    }
-
-    pub(crate) fn for_profile(profile: DeliveryProfile) -> Self {
-        Self::with_tool_decorator_and_assembly_plan(
-            Arc::new(SnapshotToolDecorator::new(Arc::new(ProductSnapshotToolWrapper))),
-            product_assembly_plan_for_profile(profile),
-        )
+        Self::with_tool_decorator(Arc::new(SnapshotToolDecorator::new(Arc::new(
+            ProductSnapshotToolWrapper,
+        ))))
     }
 
     pub(crate) fn with_tool_decorator(tool_decorator: ProductToolDecoratorRef) -> Self {
-        Self::with_tool_decorator_and_assembly_plan(
-            tool_decorator,
-            product_assembly_plan_for_profile(DeliveryProfile::ProductFull),
-        )
-    }
-
-    pub(crate) fn with_tool_decorator_and_assembly_plan(
-        tool_decorator: ProductToolDecoratorRef,
-        assembly_plan: ProductAssemblyPlan,
-    ) -> Self {
-        Self {
-            tool_decorator,
-            assembly_plan,
-        }
+        Self { tool_decorator }
     }
 
     pub(crate) fn create_registry(&self) -> ToolRegistry {
-        let inner = create_product_tool_registry_from_plan(
-            self.assembly_plan.capability_assembly().tool_provider_group_plan(),
-            self.tool_decorator.clone(),
-        );
+        let inner = create_product_tool_registry_from_plan(self.tool_decorator.clone());
         ToolRegistry::from_inner(inner)
     }
 }
 
 #[cfg(test)]
 mod tests {
     use super::ProductToolRuntime;
     use crate::agentic::tools::registry::create_tool_registry;
-    use northhing_product_capabilities::{product_assembly_plan_for_profile, DeliveryProfile};
 
     #[test]
     fn product_tool_runtime_owner_preserves_registry_contract() {
         let runtime = ProductToolRuntime::default();
         let owner_registry = runtime.create_registry();
         let compatibility_registry = create_tool_registry();
 
         assert_eq!(
             owner_registry.tool_names(),
             compatibility_registry.tool_names(),
@@ -96,36 +74,19 @@ mod tests {
         );
         assert_eq!(
             owner_registry.collapsed_tool_names(),
             compatibility_registry.collapsed_tool_names(),
             "product tool runtime owner must preserve collapsed-tool exposure"
         );
     }
 
     #[test]
     fn product_tool_runtime_registry_preserves_provider_plan_order() {
-        let assembly = product_assembly_plan_for_profile(DeliveryProfile::ProductFull)
-            .capability_assembly()
-            .clone();
-        let planned_names = assembly
-            .tool_provider_group_plan()
+        let planned_names = super::materialization::PRODUCT_TOOL_GROUPS
             .iter()
-            .flat_map(|group| group.tool_names())
+            .flat_map(|(_, tools)| tools.iter().copied())
             .map(|tool_name| tool_name.to_string())
             .collect::<Vec<_>>();
 
         assert_eq!(planned_names, create_tool_registry().tool_names());
     }
-
-    #[test]
-    fn product_tool_runtime_can_consume_explicit_product_assembly_plan() {
-        let runtime = ProductToolRuntime::for_profile(DeliveryProfile::Cli);
-        let owner_registry = runtime.create_registry();
-        let compatibility_registry = create_tool_registry();
-
-        assert_eq!(owner_registry.tool_names(), compatibility_registry.tool_names());
-        assert_eq!(
-            owner_registry.collapsed_tool_names(),
-            compatibility_registry.collapsed_tool_names()
-        );
-    }
 }
diff --git a/src/crates/assembly/core/src/agentic/tools/product_runtime/materialization.rs b/src/crates/assembly/core/src/agentic/tools/product_runtime/materialization.rs
index 23d2c35..0fcd540 100644
--- a/src/crates/assembly/core/src/agentic/tools/product_runtime/materialization.rs
+++ b/src/crates/assembly/core/src/agentic/tools/product_runtime/materialization.rs
@@ -1,21 +1,78 @@
 //! Product tool materialization owner.
 
 use crate::agentic::tools::framework::Tool;
 use crate::agentic::tools::implementations::*;
 use crate::agentic::tools::registry::ProductToolDecoratorRef;
 use northhing_agent_tools::{
     StaticToolProviderFactory, StaticToolProviderPlan, ToolRegistry as AgentToolRegistry, ToolRuntimeAssembly,
 };
-use northhing_tool_packs::ToolProviderGroupPlan;
 use std::sync::Arc;
 
+pub(in crate::agentic::tools) const PRODUCT_TOOL_GROUPS: &[(&str, &[&str])] = &[
+    (
+        "core.basic",
+        &[
+            "LS",
+            "Read",
+            "Glob",
+            "Grep",
+            "Write",
+            "Edit",
+            "Delete",
+            "ExecCommand",
+            "WriteStdin",
+            "ExecControl",
+            "GetTime",
+        ],
+    ),
+    (
+        "core.agent",
+        &[
+            "Task",
+            "Skill",
+            "AskUserQuestion",
+            "TodoWrite",
+            "get_goal",
+            "create_goal",
+            "update_goal",
+            "CreatePlan",
+            "submit_code_review",
+            "GetToolSpec",
+            "GetFileDiff",
+            "Log",
+        ],
+    ),
+    (
+        "core.session",
+        &["SessionControl", "SessionMessage", "SessionHistory", "Cron"],
+    ),
+    (
+        "core.integration",
+        &[
+            "WebSearch",
+            "WebFetch",
+            "ListMCPResources",
+            "ReadMCPResource",
+            "ListMCPPrompts",
+            "GetMCPPrompt",
+            "GenerativeUI",
+            "Git",
+            "ReviewPlatform",
+            "InitMiniApp",
+            "ControlHub",
+            "ComputerUse",
+            "Playbook",
+        ],
+    ),
+];
+
 #[derive(Debug, Clone, Copy, Default)]
 pub(in crate::agentic::tools) struct ProductConcreteToolFactory;
 
 impl StaticToolProviderFactory<dyn Tool> for ProductConcreteToolFactory {
     fn materialize_tool(&self, tool_name: &str) -> Option<Arc<dyn Tool>> {
         match tool_name {
             "LS" => Some(Arc::new(LSTool::new())),
             "Read" => Some(Arc::new(FileReadTool::new())),
             "Glob" => Some(Arc::new(GlobTool::new())),
             "Grep" => Some(Arc::new(GrepTool::new())),
@@ -54,36 +111,41 @@ impl StaticToolProviderFactory<dyn Tool> for ProductConcreteToolFactory {
             "InitMiniApp" => Some(Arc::new(InitMiniAppTool::new())),
             "ControlHub" => Some(Arc::new(ControlHubTool::new())),
             "ComputerUse" => Some(Arc::new(ComputerUseTool::new())),
             "Playbook" => Some(Arc::new(PlaybookTool::new())),
             _ => None,
         }
     }
 }
 
 #[derive(Debug, Clone, Copy)]
-struct ProductToolProviderPlanAdapter(ToolProviderGroupPlan);
+struct ProductToolProviderPlanAdapter {
+    provider_id: &'static str,
+    tool_names: &'static [&'static str],
+}
 
 impl StaticToolProviderPlan for ProductToolProviderPlanAdapter {
     fn provider_id(&self) -> &'static str {
-        self.0.provider_id()
+        self.provider_id
     }
 
     fn tool_names(&self) -> &'static [&'static str] {
-        self.0.tool_names()
+        self.tool_names
     }
 }
 
 pub(in crate::agentic::tools) fn create_product_tool_registry_from_plan(
-    plan: &[ToolProviderGroupPlan],
     tool_decorator: ProductToolDecoratorRef,
 ) -> AgentToolRegistry<dyn Tool> {
-    let adapters = plan
+    let adapters = PRODUCT_TOOL_GROUPS
         .iter()
         .copied()
-        .map(ProductToolProviderPlanAdapter)
+        .map(|(provider_id, tool_names)| ProductToolProviderPlanAdapter {
+            provider_id,
+            tool_names,
+        })
         .collect::<Vec<_>>();
 
     ToolRuntimeAssembly::with_tool_decorator(tool_decorator)
         .create_registry_from_static_provider_plans(&adapters, &ProductConcreteToolFactory)
         .expect("product capability tool provider plan must reference concrete core tools")
 }
diff --git a/src/crates/assembly/core/src/agentic/tools/registry/tests.rs b/src/crates/assembly/core/src/agentic/tools/registry/tests.rs
index 7b03520..97e34bd 100644
--- a/src/crates/assembly/core/src/agentic/tools/registry/tests.rs
+++ b/src/crates/assembly/core/src/agentic/tools/registry/tests.rs
@@ -224,42 +224,38 @@ mod tests {
             .collect::<Vec<_>>();
         assert_eq!(
             runtime_names,
             registry.tool_names(),
             "runtime tool collection order must match registry key order"
         );
     }
 
     #[test]
     fn product_capability_provider_plan_covers_registry_manifest_in_order() {
-        let assembly = northhing_product_capabilities::default_product_capability_assembly();
-        let provider_tools = assembly
-            .tool_provider_group_plan()
+        let provider_tools = crate::agentic::tools::product_runtime::PRODUCT_TOOL_GROUPS
             .iter()
-            .flat_map(|group| group.tool_names())
+            .flat_map(|(_, tools)| tools.iter().copied())
             .map(|tool_name| tool_name.to_string())
             .collect::<Vec<_>>();
 
         assert_eq!(
             provider_tools,
             create_tool_registry().tool_names(),
             "provider-based assembly must preserve the existing builtin registry order"
         );
     }
 
     #[test]
     fn product_capability_provider_plan_keeps_owner_group_order() {
-        let assembly = northhing_product_capabilities::default_product_capability_assembly();
-        let provider_ids = assembly
-            .tool_provider_group_plan()
+        let provider_ids = crate::agentic::tools::product_runtime::PRODUCT_TOOL_GROUPS
             .iter()
-            .map(|group| group.provider_id())
+            .map(|(id, _)| *id)
             .collect::<Vec<_>>();
 
         assert_eq!(
             provider_ids,
             vec!["core.basic", "core.agent", "core.session", "core.integration"],
             "provider groups must stay stable until concrete tool-pack owners exist"
         );
     }
 
     #[test]
diff --git a/src/crates/assembly/core/src/product_assembly.rs b/src/crates/assembly/core/src/product_assembly.rs
index 0d69bf8..0d27236 100644
--- a/src/crates/assembly/core/src/product_assembly.rs
+++ b/src/crates/assembly/core/src/product_assembly.rs
@@ -1,15 +1,14 @@
 //! Product assembly compatibility facade.
 //!
 //! Provider-neutral product assembly facts are owned by
 //! `northhing-product-capabilities`. Core-specific runtime service adapters live
 //! under `product_runtime`.
 
 pub use northhing_product_capabilities::{
     default_product_assembly_plan, default_product_capability_assembly, default_product_capability_registry,
-    default_product_harness_registry, product_assembly_plan_for_profile, DeliveryProfile, ProductAssemblyPlan,
-    ProductCapabilityAssembly, ProductCapabilityId, ProductCapabilityPack, ProductCapabilityRegistry,
-    ProductCapabilitySet, ProductServiceCapabilityAvailability, ProductServiceCapabilityRequirement,
-    ProductServiceCapabilityStatus,
+    product_assembly_plan_for_profile, DeliveryProfile, ProductAssemblyPlan, ProductCapabilityAssembly,
+    ProductCapabilityId, ProductCapabilityPack, ProductCapabilityRegistry, ProductCapabilitySet,
+    ProductServiceCapabilityAvailability, ProductServiceCapabilityRequirement, ProductServiceCapabilityStatus,
 };
 
 pub use crate::product_runtime::CoreRuntimeServicesProvider;
diff --git a/src/crates/assembly/product-capabilities/Cargo.toml b/src/crates/assembly/product-capabilities/Cargo.toml
index 2923244..927e25a 100644
--- a/src/crates/assembly/product-capabilities/Cargo.toml
+++ b/src/crates/assembly/product-capabilities/Cargo.toml
@@ -3,13 +3,11 @@ name = "northhing-product-capabilities"
 version.workspace = true
 authors.workspace = true
 edition.workspace = true
 description = "northhing product capability pack contracts"
 
 [lib]
 name = "northhing_product_capabilities"
 crate-type = ["rlib"]
 
 [dependencies]
-northhing-harness = { path = "../../execution/harness" }
 northhing-runtime-ports = { path = "../../contracts/runtime-ports" }
-northhing-tool-packs = { path = "../../execution/tool-provider-groups", default-features = false }
diff --git a/src/crates/assembly/product-capabilities/src/lib.rs b/src/crates/assembly/product-capabilities/src/lib.rs
index 5663c9b..024a5aa 100644
--- a/src/crates/assembly/product-capabilities/src/lib.rs
+++ b/src/crates/assembly/product-capabilities/src/lib.rs
@@ -1,26 +1,20 @@
 #![allow(clippy::too_many_arguments)]
 //! Product capability pack contracts.
 //!
 //! This crate owns provider-neutral product capability assembly facts. Concrete
 //! workflow execution and tool implementations remain in their runtime owners.
 
 use std::collections::HashSet;
 use std::fmt;
 
-use northhing_harness::{
-    build_descriptor_harness_registry, HarnessCapability, HarnessProviderDescriptor, HarnessRegistry,
-    HarnessRegistryBuildError, HarnessWorkflow,
-};
 use northhing_runtime_ports::RuntimeServiceCapability;
-pub use northhing_tool_packs::ToolProviderGroupPlanSelectionError as ProductCapabilityBuildError;
-use northhing_tool_packs::{try_product_tool_provider_group_plan_for_ids, ToolProviderGroupPlan};
 
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
 pub enum ProductCapabilityId {
     CodeAgent,
     DeepReview,
     DeepResearch,
     MiniApp,
 }
 
 impl ProductCapabilityId {
@@ -37,54 +31,40 @@ impl ProductCapabilityId {
 impl fmt::Display for ProductCapabilityId {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         f.write_str(self.id())
     }
 }
 
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub struct ProductCapabilityPack {
     id: ProductCapabilityId,
     required_services: &'static [RuntimeServiceCapability],
-    tool_provider_group_ids: &'static [&'static str],
-    harness_provider_descriptors: &'static [HarnessProviderDescriptor],
 }
 
 impl ProductCapabilityPack {
     pub const fn new(
         id: ProductCapabilityId,
         required_services: &'static [RuntimeServiceCapability],
-        tool_provider_group_ids: &'static [&'static str],
-        harness_provider_descriptors: &'static [HarnessProviderDescriptor],
     ) -> Self {
         Self {
             id,
             required_services,
-            tool_provider_group_ids,
-            harness_provider_descriptors,
         }
     }
 
     pub const fn id(self) -> ProductCapabilityId {
         self.id
     }
 
     pub const fn required_services(self) -> &'static [RuntimeServiceCapability] {
         self.required_services
     }
-
-    pub const fn tool_provider_group_ids(self) -> &'static [&'static str] {
-        self.tool_provider_group_ids
-    }
-
-    pub const fn harness_provider_descriptors(self) -> &'static [HarnessProviderDescriptor] {
-        self.harness_provider_descriptors
-    }
 }
 
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
 #[non_exhaustive]
 pub enum DeliveryProfile {
     ProductFull,
     Desktop,
     Cli,
     Server,
     Remote,
@@ -196,22 +176,20 @@ impl ProductServiceCapabilityRequirement {
 
     pub const fn service_capability(self) -> RuntimeServiceCapability {
         self.service_capability
     }
 }
 
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub struct ProductCapabilityAssembly {
     capability_ids: Vec<ProductCapabilityId>,
     service_requirements: Vec<ProductServiceCapabilityRequirement>,
-    tool_provider_group_plan: Vec<ToolProviderGroupPlan>,
-    harness_provider_descriptors: Vec<HarnessProviderDescriptor>,
 }
 
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub struct ProductAssemblyPlan {
     profile: DeliveryProfile,
     capability_set: ProductCapabilitySet,
     capability_assembly: ProductCapabilityAssembly,
 }
 
 impl ProductAssemblyPlan {
@@ -250,38 +228,30 @@ impl ProductAssemblyPlan {
             .map(|requirement| {
                 let status = if is_available(requirement.service_capability()) {
                     ProductServiceCapabilityStatus::Available
                 } else {
                     ProductServiceCapabilityStatus::Unavailable
                 };
                 ProductServiceCapabilityAvailability::new(requirement, status)
             })
             .collect()
     }
-
-    pub fn build_harness_registry(&self) -> Result<HarnessRegistry, HarnessRegistryBuildError> {
-        self.capability_assembly.build_harness_registry()
-    }
 }
 
 impl ProductCapabilityAssembly {
     fn new(
         capability_ids: Vec<ProductCapabilityId>,
         service_requirements: Vec<ProductServiceCapabilityRequirement>,
-        tool_provider_group_plan: Vec<ToolProviderGroupPlan>,
-        harness_provider_descriptors: Vec<HarnessProviderDescriptor>,
     ) -> Self {
         Self {
             capability_ids,
             service_requirements,
-            tool_provider_group_plan,
-            harness_provider_descriptors,
         }
     }
 
     pub fn capability_ids(&self) -> &[ProductCapabilityId] {
         &self.capability_ids
     }
 
     pub fn service_requirements(&self) -> &[ProductServiceCapabilityRequirement] {
         &self.service_requirements
     }
@@ -301,32 +271,20 @@ impl ProductCapabilityAssembly {
     pub fn missing_service_requirements<F>(&self, mut is_available: F) -> Vec<ProductServiceCapabilityRequirement>
     where
         F: FnMut(RuntimeServiceCapability) -> bool,
     {
         self.service_requirements
             .iter()
             .copied()
             .filter(|requirement| !is_available(requirement.service_capability()))
             .collect()
     }
-
-    pub fn tool_provider_group_plan(&self) -> &[ToolProviderGroupPlan] {
-        &self.tool_provider_group_plan
-    }
-
-    pub fn harness_provider_descriptors(&self) -> &[HarnessProviderDescriptor] {
-        &self.harness_provider_descriptors
-    }
-
-    pub fn build_harness_registry(&self) -> Result<HarnessRegistry, HarnessRegistryBuildError> {
-        build_descriptor_harness_registry(self.harness_provider_descriptors.iter().copied())
-    }
 }
 
 #[derive(Debug, Clone, Copy)]
 pub struct ProductCapabilityRegistry {
     packs: &'static [ProductCapabilityPack],
 }
 
 impl ProductCapabilityRegistry {
     pub const fn new(packs: &'static [ProductCapabilityPack]) -> Self {
         Self { packs }
@@ -359,72 +317,25 @@ impl ProductCapabilityRegistry {
             for service_capability in pack.required_services() {
                 let requirement = ProductServiceCapabilityRequirement::new(pack.id(), *service_capability);
                 if seen.insert(requirement) {
                     requirements.push(requirement);
                 }
             }
         }
         requirements
     }
 
-    pub fn tool_provider_group_ids(self) -> Vec<&'static str> {
-        let mut seen = HashSet::new();
-        let mut provider_ids = Vec::new();
-        for pack in self.packs {
-            for provider_id in pack.tool_provider_group_ids() {
-                if seen.insert(*provider_id) {
-                    provider_ids.push(*provider_id);
-                }
-            }
-        }
-        provider_ids
-    }
-
-    pub fn try_tool_provider_group_plan(self) -> Result<Vec<ToolProviderGroupPlan>, ProductCapabilityBuildError> {
-        let provider_ids = self.tool_provider_group_ids();
-        try_product_tool_provider_group_plan_for_ids(&provider_ids)
-    }
-
-    pub fn tool_provider_group_plan(self) -> Vec<ToolProviderGroupPlan> {
-        self.try_tool_provider_group_plan()
-            .expect("product capability packs must reference known tool provider groups")
-    }
-
-    pub fn harness_provider_descriptors(self) -> Vec<HarnessProviderDescriptor> {
-        let mut seen = HashSet::new();
-        let mut descriptors = Vec::new();
-        for pack in self.packs {
-            for descriptor in pack.harness_provider_descriptors() {
-                if seen.insert(descriptor.provider_id()) {
-                    descriptors.push(*descriptor);
-                }
-            }
-        }
-        descriptors
-    }
-
-    pub fn build_harness_registry(self) -> Result<HarnessRegistry, HarnessRegistryBuildError> {
-        build_descriptor_harness_registry(self.harness_provider_descriptors())
-    }
-
-    pub fn try_build_assembly(self) -> Result<ProductCapabilityAssembly, ProductCapabilityBuildError> {
-        Ok(ProductCapabilityAssembly::new(
+    pub fn build_assembly(self) -> ProductCapabilityAssembly {
+        ProductCapabilityAssembly::new(
             self.capability_ids(),
             self.service_requirements(),
-            self.try_tool_provider_group_plan()?,
-            self.harness_provider_descriptors(),
-        ))
-    }
-
-    pub fn build_assembly(self) -> ProductCapabilityAssembly {
-        self.try_build_assembly()
-            .expect("product capability packs must build a valid assembly")
+        )
     }
 
     pub fn capability_set(self) -> ProductCapabilitySet {
         ProductCapabilitySet::new(self.capability_ids())
     }
 
     pub fn build_assembly_plan(self, profile: DeliveryProfile) -> ProductAssemblyPlan {
         let capability_set = self.capability_set();
         let capability_assembly = self.build_assembly();
         ProductAssemblyPlan::new(profile, capability_set, capability_assembly)
@@ -452,102 +363,44 @@ const DEEP_RESEARCH_SERVICES: &[RuntimeServiceCapability] = &[
     RuntimeServiceCapability::Permission,
     RuntimeServiceCapability::Events,
 ];
 const MINIAPP_SERVICES: &[RuntimeServiceCapability] = &[
     RuntimeServiceCapability::FileSystem,
     RuntimeServiceCapability::Workspace,
     RuntimeServiceCapability::Permission,
     RuntimeServiceCapability::Events,
 ];
 
-const CODE_AGENT_TOOL_GROUPS: &[&str] = &["core.basic", "core.agent", "core.session"];
-const INTEGRATION_TOOL_GROUPS: &[&str] = &["core.integration"];
-
-const DEEP_REVIEW_HARNESS_CAPABILITIES: &[HarnessCapability] = &[
-    HarnessCapability::Plan,
-    HarnessCapability::ReviewGate,
-    HarnessCapability::PostProcessor,
-];
-const DEEP_RESEARCH_HARNESS_CAPABILITIES: &[HarnessCapability] =
-    &[HarnessCapability::Plan, HarnessCapability::PostProcessor];
-const MINIAPP_HARNESS_CAPABILITIES: &[HarnessCapability] = &[HarnessCapability::Plan, HarnessCapability::Artifact];
-
-pub const CORE_DEEP_REVIEW_HARNESS_PROVIDER_ID: &str = "core.deep_review";
-pub const CORE_DEEP_RESEARCH_HARNESS_PROVIDER_ID: &str = "core.deep_research";
-pub const CORE_MINIAPP_HARNESS_PROVIDER_ID: &str = "core.miniapp";
-
-const DEEP_REVIEW_HARNESS_PROVIDER: HarnessProviderDescriptor = HarnessProviderDescriptor::legacy_facade(
-    CORE_DEEP_REVIEW_HARNESS_PROVIDER_ID,
-    HarnessWorkflow::DeepReview,
-    DEEP_REVIEW_HARNESS_CAPABILITIES,
-    "northhing-core::agentic::deep_review",
-);
-const DEEP_RESEARCH_HARNESS_PROVIDER: HarnessProviderDescriptor = HarnessProviderDescriptor::legacy_facade(
-    CORE_DEEP_RESEARCH_HARNESS_PROVIDER_ID,
-    HarnessWorkflow::DeepResearch,
-    DEEP_RESEARCH_HARNESS_CAPABILITIES,
-    "northhing-core::agentic::agents::definitions::modes::deep_research",
-);
-const MINIAPP_HARNESS_PROVIDER: HarnessProviderDescriptor = HarnessProviderDescriptor::legacy_facade(
-    CORE_MINIAPP_HARNESS_PROVIDER_ID,
-    HarnessWorkflow::MiniApp,
-    MINIAPP_HARNESS_CAPABILITIES,
-    "northhing-core::miniapp",
-);
-
-const NO_HARNESS_PROVIDERS: &[HarnessProviderDescriptor] = &[];
-const DEEP_REVIEW_HARNESS_PROVIDERS: &[HarnessProviderDescriptor] = &[DEEP_REVIEW_HARNESS_PROVIDER];
-const DEEP_RESEARCH_HARNESS_PROVIDERS: &[HarnessProviderDescriptor] = &[DEEP_RESEARCH_HARNESS_PROVIDER];
-const MINIAPP_HARNESS_PROVIDERS: &[HarnessProviderDescriptor] = &[MINIAPP_HARNESS_PROVIDER];
-
 const DEFAULT_PRODUCT_CAPABILITY_PACKS: &[ProductCapabilityPack] = &[
     ProductCapabilityPack::new(
         ProductCapabilityId::CodeAgent,
         CODE_AGENT_SERVICES,
-        CODE_AGENT_TOOL_GROUPS,
-        NO_HARNESS_PROVIDERS,
     ),
     ProductCapabilityPack::new(
         ProductCapabilityId::DeepReview,
         DEEP_REVIEW_SERVICES,
-        INTEGRATION_TOOL_GROUPS,
-        DEEP_REVIEW_HARNESS_PROVIDERS,
     ),
     ProductCapabilityPack::new(
         ProductCapabilityId::DeepResearch,
         DEEP_RESEARCH_SERVICES,
-        INTEGRATION_TOOL_GROUPS,
-        DEEP_RESEARCH_HARNESS_PROVIDERS,
     ),
     ProductCapabilityPack::new(
         ProductCapabilityId::MiniApp,
         MINIAPP_SERVICES,
-        INTEGRATION_TOOL_GROUPS,
-        MINIAPP_HARNESS_PROVIDERS,
     ),
 ];
 
 pub fn default_product_capability_registry() -> ProductCapabilityRegistry {
     ProductCapabilityRegistry::new(DEFAULT_PRODUCT_CAPABILITY_PACKS)
 }
 
 pub fn default_product_capability_assembly() -> ProductCapabilityAssembly {
     default_product_capability_registry().build_assembly()
 }
 
 pub fn product_assembly_plan_for_profile(profile: DeliveryProfile) -> ProductAssemblyPlan {
     default_product_capability_registry().build_assembly_plan(profile)
 }
 
 pub fn default_product_assembly_plan() -> ProductAssemblyPlan {
     product_assembly_plan_for_profile(DeliveryProfile::ProductFull)
 }
-
-pub fn product_harness_registry_for_profile(
-    profile: DeliveryProfile,
-) -> Result<HarnessRegistry, HarnessRegistryBuildError> {
-    product_assembly_plan_for_profile(profile).build_harness_registry()
-}
-
-pub fn default_product_harness_registry() -> Result<HarnessRegistry, HarnessRegistryBuildError> {
-    product_harness_registry_for_profile(DeliveryProfile::ProductFull)
-}
diff --git a/src/crates/assembly/product-capabilities/tests/product_capabilities.rs b/src/crates/assembly/product-capabilities/tests/product_capabilities.rs
index a8c1f54..8546dd1 100644
--- a/src/crates/assembly/product-capabilities/tests/product_capabilities.rs
+++ b/src/crates/assembly/product-capabilities/tests/product_capabilities.rs
@@ -1,137 +1,55 @@
-use northhing_harness::{HarnessCapability, HarnessWorkflow};
 use northhing_product_capabilities::{
     default_product_assembly_plan, default_product_capability_assembly, default_product_capability_registry,
-    default_product_harness_registry, product_assembly_plan_for_profile, DeliveryProfile, ProductCapabilityBuildError,
-    ProductCapabilityId, ProductCapabilityPack, ProductCapabilityRegistry, ProductServiceCapabilityRequirement,
+    product_assembly_plan_for_profile, DeliveryProfile, ProductCapabilityId, ProductServiceCapabilityRequirement,
     ProductServiceCapabilityStatus,
 };
 use northhing_runtime_ports::RuntimeServiceCapability;
 
 #[test]
-fn default_capability_registry_preserves_product_tool_provider_order() {
-    let assembly = default_product_capability_assembly();
-    let provider_ids = assembly
-        .tool_provider_group_plan()
-        .iter()
-        .map(|group| group.provider_id())
-        .collect::<Vec<_>>();
-
-    assert_eq!(
-        provider_ids,
-        vec!["core.basic", "core.agent", "core.session", "core.integration",]
-    );
-}
-
-#[test]
-fn default_capability_registry_preserves_legacy_harness_routes() {
-    let registry = default_product_harness_registry().expect("harness registry should build");
-
-    assert_eq!(
-        registry.provider_ids(),
-        vec!["core.deep_review", "core.deep_research", "core.miniapp"]
-    );
-    assert_eq!(
-        registry.workflows(),
-        vec![
-            HarnessWorkflow::DeepReview,
-            HarnessWorkflow::DeepResearch,
-            HarnessWorkflow::MiniApp,
-        ]
-    );
-}
-
-#[test]
-fn capability_packs_describe_service_tool_and_harness_requirements() {
+fn capability_packs_describe_service_requirements() {
     let registry = default_product_capability_registry();
 
     let capability_ids = registry
         .capability_ids()
         .into_iter()
         .map(ProductCapabilityId::id)
         .collect::<Vec<_>>();
     assert_eq!(
         capability_ids,
         vec!["code-agent", "deep-review", "deep-research", "miniapp"]
     );
 
     let service_capabilities = registry.required_service_capabilities();
     assert!(service_capabilities.contains(&RuntimeServiceCapability::FileSystem));
     assert!(service_capabilities.contains(&RuntimeServiceCapability::Workspace));
     assert!(service_capabilities.contains(&RuntimeServiceCapability::Permission));
     assert!(service_capabilities.contains(&RuntimeServiceCapability::Events));
-
-    let harness_capabilities = registry
-        .harness_provider_descriptors()
-        .into_iter()
-        .map(|descriptor| {
-            (
-                descriptor.provider_id(),
-                descriptor.workflow(),
-                descriptor.capabilities().to_vec(),
-            )
-        })
-        .collect::<Vec<_>>();
-
-    assert_eq!(
-        harness_capabilities,
-        vec![
-            (
-                "core.deep_review",
-                HarnessWorkflow::DeepReview,
-                vec![
-                    HarnessCapability::Plan,
-                    HarnessCapability::ReviewGate,
-                    HarnessCapability::PostProcessor,
-                ],
-            ),
-            (
-                "core.deep_research",
-                HarnessWorkflow::DeepResearch,
-                vec![HarnessCapability::Plan, HarnessCapability::PostProcessor],
-            ),
-            (
-                "core.miniapp",
-                HarnessWorkflow::MiniApp,
-                vec![HarnessCapability::Plan, HarnessCapability::Artifact],
-            ),
-        ]
-    );
 }
 
 #[test]
 fn product_assembly_plan_makes_delivery_profile_explicit_without_reducing_capabilities() {
     let expected_capabilities = vec!["code-agent", "deep-review", "deep-research", "miniapp"];
-    let expected_tool_groups = vec!["core.basic", "core.agent", "core.session", "core.integration"];
 
     for profile in DeliveryProfile::all_current_product_profiles().iter().copied() {
         let plan = product_assembly_plan_for_profile(profile);
 
         assert_eq!(plan.profile(), profile);
         assert_eq!(
             plan.capability_set()
                 .ids()
                 .iter()
                 .map(|capability_id| capability_id.id())
                 .collect::<Vec<_>>(),
             expected_capabilities,
             "{profile} must preserve the current product-full capability set until explicit trimming is proven"
         );
-        assert_eq!(
-            plan.capability_assembly()
-                .tool_provider_group_plan()
-                .iter()
-                .map(|group| group.provider_id())
-                .collect::<Vec<_>>(),
-            expected_tool_groups,
-            "{profile} must preserve current tool provider groups"
-        );
     }
 }
 
 #[test]
 fn product_assembly_plan_reports_service_availability_by_capability() {
     let plan = default_product_assembly_plan();
 
     let unavailable = plan
         .service_availability_report(|capability| {
             !matches!(
@@ -148,21 +66,21 @@ fn product_assembly_plan_reports_service_availability_by_capability() {
         unavailable[0].requirement(),
         ProductServiceCapabilityRequirement::new(ProductCapabilityId::DeepReview, RuntimeServiceCapability::Git,)
     );
     assert_eq!(
         unavailable[1].requirement(),
         ProductServiceCapabilityRequirement::new(ProductCapabilityId::DeepResearch, RuntimeServiceCapability::Network,)
     );
 }
 
 #[test]
-fn default_capability_assembly_keeps_service_tool_and_harness_facts_together() {
+fn default_capability_assembly_keeps_service_facts_together() {
     let assembly = default_product_capability_assembly();
 
     let capability_ids = assembly
         .capability_ids()
         .iter()
         .map(|capability_id| capability_id.id())
         .collect::<Vec<_>>();
     assert_eq!(
         capability_ids,
         vec!["code-agent", "deep-review", "deep-research", "miniapp"]
@@ -176,40 +94,20 @@ fn default_capability_assembly_keeps_service_tool_and_harness_facts_together() {
             RuntimeServiceCapability::Workspace,
             RuntimeServiceCapability::SessionStore,
             RuntimeServiceCapability::Permission,
             RuntimeServiceCapability::Events,
             RuntimeServiceCapability::Clock,
             RuntimeServiceCapability::Terminal,
             RuntimeServiceCapability::Git,
             RuntimeServiceCapability::Network,
         ]
     );
-
-    let tool_provider_ids = assembly
-        .tool_provider_group_plan()
-        .iter()
-        .map(|group| group.provider_id())
-        .collect::<Vec<_>>();
-    assert_eq!(
-        tool_provider_ids,
-        vec!["core.basic", "core.agent", "core.session", "core.integration"]
-    );
-
-    let harness_provider_ids = assembly
-        .harness_provider_descriptors()
-        .iter()
-        .map(|descriptor| descriptor.provider_id())
-        .collect::<Vec<_>>();
-    assert_eq!(
-        harness_provider_ids,
-        vec!["core.deep_review", "core.deep_research", "core.miniapp"]
-    );
 }
 
 #[test]
 fn capability_assembly_reports_missing_services_without_concrete_runtime_dependency() {
     let assembly = default_product_capability_assembly();
 
     let missing = assembly.missing_service_requirements(|capability| {
         !matches!(
             capability,
             RuntimeServiceCapability::Git | RuntimeServiceCapability::Network
@@ -225,38 +123,10 @@ fn capability_assembly_reports_missing_services_without_concrete_runtime_depende
                 RuntimeServiceCapability::Network,
             ),
         ]
     );
 
     assert!(
         assembly.missing_service_requirements(|_capability| true).is_empty(),
         "fully assembled product runtime must report no service capability gaps"
     );
 }
-
-#[test]
-fn capability_registry_rejects_unknown_tool_provider_groups() {
-    static BROKEN_TOOL_GROUPS: &[&str] = &["core.missing"];
-    static BROKEN_PACKS: &[ProductCapabilityPack] = &[ProductCapabilityPack::new(
-        ProductCapabilityId::CodeAgent,
-        &[],
-        BROKEN_TOOL_GROUPS,
-        &[],
-    )];
-
-    let registry = ProductCapabilityRegistry::new(BROKEN_PACKS);
-    let harness_registry = registry
-        .build_harness_registry()
-        .expect("harness registry should not depend on tool provider group validity");
-    assert!(harness_registry.provider_ids().is_empty());
-
-    let error = registry
-        .try_tool_provider_group_plan()
-        .expect_err("unknown provider groups must not be silently dropped");
-
-    assert_eq!(
-        error,
-        ProductCapabilityBuildError::UnknownToolProviderGroup {
-            provider_id: "core.missing"
-        }
-    );
-}
diff --git a/src/crates/execution/AGENTS-CN.md b/src/crates/execution/AGENTS-CN.md
index ccd3a17..07645fb 100644
--- a/src/crates/execution/AGENTS-CN.md
+++ b/src/crates/execution/AGENTS-CN.md
@@ -4,23 +4,21 @@
 
 本层拥有可复用的 agent、harness、stream、typed-service 与 tool execution 原语。它不是完整的 Agent Runtime SDK，也不是装配好的产品运行时。产品装配决定哪些原语、tool provider groups、harness providers、适配器与服务对某种交付形式生效。
 
 ## 模块
 
 | Crate | 职责 | 本地文档 |
 |---|---|---|
 | `agent-runtime` | Agent 注册、调度、prompt 缓存、hooks、goals、prompt 事实、基于端口的 `AgentRuntime` 外观、DeepReview 与 provider 无关的状态、DeepResearch 引用重编号以及运行时控制契约 | [AGENTS.md](agent-runtime/AGENTS.md) |
 | `agent-stream` | 与 provider 无关的 stream DTO、tool-call 累积以及重放契约 | [AGENTS.md](agent-stream/AGENTS.md) |
 | `tool-contracts` | 工具契约、执行闸门、输入校验以及结果展示契约。Cargo 包名仍为 `northhing-agent-tools`。 | [AGENTS.md](tool-contracts/AGENTS.md) |
-| `harness` | Harness 工作流契约与注册原语 | [AGENTS.md](harness/AGENTS.md) |
 | `runtime-services` | 类型化的运行时服务装配以及服务可用性事实 | [AGENTS.md](runtime-services/AGENTS.md) |
-| `tool-provider-groups` | Tool provider group 事实以及 product-full 工具组组合。Cargo 包名仍为 `northhing-tool-packs`。 | [AGENTS.md](tool-provider-groups/AGENTS.md) |
 | `tool-execution` | 低层的 file/search/tool IO 辅助函数。Cargo 包名仍为 `tool-runtime`。 | [AGENTS.md](tool-execution/AGENTS.md) |
 
 ## 放置规则
 
 - 在这里放置可移植的执行编排、agent 生命周期契约、工具契约、与 provider 无关的 stream 契约以及执行事实。
 - 把具体的 filesystem、git、terminal、MCP server、远程 SSH 与 OS 行为保留在 `services`，除非这些代码是纯低层工具原语。
 - 把协议投影与外部提供方请求整形保留在 `adapters`。
 - 把产品特性选择与交付配置决策保留在 `assembly`，而不是执行原语中。
 - Tool packs 应当描述 provider groups 与所需的服务；具体的服务访问应通过 ports 或类型化的运行时服务进行。
 
diff --git a/src/crates/execution/AGENTS.md b/src/crates/execution/AGENTS.md
index 7e6704a..e9a5a39 100644
--- a/src/crates/execution/AGENTS.md
+++ b/src/crates/execution/AGENTS.md
@@ -8,23 +8,21 @@ assembled product runtime. Product assembly decides which primitives, tool
 provider groups, harness providers, adapters, and services are active for a
 delivery form.
 
 ## Modules
 
 | Crate | Responsibility | Local doc |
 |---|---|---|
 | `agent-runtime` | Agent registry, scheduler, prompt cache, hooks, goals, prompt facts, port-backed `AgentRuntime` facade, DeepReview provider-neutral state, DeepResearch citation renumbering, and runtime control contracts | [AGENTS.md](agent-runtime/AGENTS.md) |
 | `agent-stream` | Provider-neutral stream DTOs, tool-call accumulation, and replay contracts | [AGENTS.md](agent-stream/AGENTS.md) |
 | `tool-contracts` | Tool contracts, execution gates, input validation, and result presentation contracts. Cargo package remains `northhing-agent-tools`. | [AGENTS.md](tool-contracts/AGENTS.md) |
-| `harness` | Harness workflow contracts and registry primitives | [AGENTS.md](harness/AGENTS.md) |
 | `runtime-services` | Typed runtime service assembly and service availability facts | [AGENTS.md](runtime-services/AGENTS.md) |
-| `tool-provider-groups` | Tool provider group facts and product-full tool group composition. Cargo package remains `northhing-tool-packs`. | [AGENTS.md](tool-provider-groups/AGENTS.md) |
 | `tool-execution` | Low-level file/search/tool IO helpers. Cargo package remains `tool-runtime`. | [AGENTS.md](tool-execution/AGENTS.md) |
 
 ## Placement Rules
 
 - Put portable execution orchestration, agent lifecycle contracts, tool
   contracts, provider-neutral stream contracts, and execution facts here.
 - Keep concrete filesystem, git, terminal, MCP server, remote SSH, and OS
   behavior in `services` unless the code is a pure low-level tool primitive.
 - Keep protocol projection and external provider request shaping in `adapters`.
 - Keep product feature selection and delivery-profile decisions in `assembly`,
diff --git a/src/crates/execution/harness/AGENTS.md b/src/crates/execution/harness/AGENTS.md
deleted file mode 100644
index e4f3f15..0000000
--- a/src/crates/execution/harness/AGENTS.md
+++ /dev/null
@@ -1,31 +0,0 @@
-# harness Agent Guide
-
-Scope: this guide applies to `src/crates/execution/harness`.
-
-`northhing-harness` owns provider-neutral workflow contracts, descriptors, plans,
-and registry wiring for multi-step workflows such as Deep Review,
-DeepResearch, MiniApp, and future SDD flows.
-
-## Guardrails
-
-- Do not depend on `northhing-core`, app crates, Tauri, concrete service crates,
-  product-domain implementations, AI adapters, transport adapters, or concrete
-  tool packs.
-- Keep concrete workflow execution on the legacy product path until a reviewed
-  migration proves behavior equivalence.
-- Harness providers may describe routing, planning, capability, review gate,
-  artifact, and post-processing boundaries. They must not own session manager
-  internals, filesystem/Git/terminal managers, or UI command behavior.
-- Product Assembly should register providers through typed registries; avoid
-  global mutable registries or untyped service locators.
-- Product capability packs may select provider descriptors; `northhing-harness`
-  owns the provider-neutral descriptor type, legacy-facade descriptor adapter,
-  and registry wiring.
-
-## Verification
-
-```bash
-cargo test -p northhing-harness
-node scripts/check-core-boundaries.mjs
-cargo test -p northhing-core --features product-full product_harness
-```
diff --git a/src/crates/execution/harness/Cargo.toml b/src/crates/execution/harness/Cargo.toml
deleted file mode 100644
index 44a8060..0000000
--- a/src/crates/execution/harness/Cargo.toml
+++ /dev/null
@@ -1,17 +0,0 @@
-[package]
-name = "northhing-harness"
-version.workspace = true
-authors.workspace = true
-edition.workspace = true
-description = "Harness workflow contracts and registry for northhing"
-
-[lib]
-name = "northhing_harness"
-crate-type = ["rlib"]
-
-[dependencies]
-async-trait = { workspace = true }
-thiserror = { workspace = true }
-
-[dev-dependencies]
-tokio = { workspace = true }
diff --git a/src/crates/execution/harness/src/lib.rs b/src/crates/execution/harness/src/lib.rs
deleted file mode 100644
index f7fc1a3..0000000
--- a/src/crates/execution/harness/src/lib.rs
+++ /dev/null
@@ -1,440 +0,0 @@
-#![allow(clippy::too_many_arguments)]
-//! Harness workflow contracts.
-//!
-//! This crate owns provider-neutral workflow descriptors and registry wiring.
-//! Concrete workflow execution remains in product/runtime owners until it can
-//! be moved behind explicit ports without changing behavior.
-
-use std::collections::HashSet;
-use std::fmt;
-use std::sync::Arc;
-
-use async_trait::async_trait;
-
-#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
-pub enum HarnessWorkflow {
-    Sdd,
-    DeepReview,
-    DeepResearch,
-    MiniApp,
-    FunctionAgent,
-}
-
-impl HarnessWorkflow {
-    pub const fn id(self) -> &'static str {
-        match self {
-            Self::Sdd => "sdd",
-            Self::DeepReview => "deep-review",
-            Self::DeepResearch => "deep-research",
-            Self::MiniApp => "miniapp",
-            Self::FunctionAgent => "function-agent",
-        }
-    }
-}
-
-impl fmt::Display for HarnessWorkflow {
-    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
-        f.write_str(self.id())
-    }
-}
-
-#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
-pub enum HarnessCapability {
-    Plan,
-    Execute,
-    ReviewGate,
-    Artifact,
-    PostProcessor,
-}
-
-#[derive(Debug, Clone, PartialEq, Eq, Hash)]
-pub struct HarnessId(String);
-
-impl HarnessId {
-    pub fn new(id: impl Into<String>) -> Self {
-        Self(id.into())
-    }
-
-    pub fn as_str(&self) -> &str {
-        &self.0
-    }
-}
-
-impl fmt::Display for HarnessId {
-    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
-        f.write_str(self.as_str())
-    }
-}
-
-#[derive(Debug, Clone, Default, PartialEq, Eq)]
-pub struct HarnessPlanningContext {
-    pub request_id: Option<String>,
-}
-
-#[derive(Debug, Clone, Default, PartialEq, Eq)]
-pub struct HarnessExecutionContext {
-    pub request_id: Option<String>,
-}
-
-#[derive(Debug, Clone, PartialEq, Eq)]
-pub struct HarnessInput {
-    workflow: HarnessWorkflow,
-    goal: String,
-}
-
-impl HarnessInput {
-    pub fn new(workflow: HarnessWorkflow, goal: impl Into<String>) -> Self {
-        Self {
-            workflow,
-            goal: goal.into(),
-        }
-    }
-
-    pub fn workflow(&self) -> HarnessWorkflow {
-        self.workflow
-    }
-
-    pub fn goal(&self) -> &str {
-        &self.goal
-    }
-}
-
-#[derive(Debug, Clone, Copy, PartialEq, Eq)]
-pub enum HarnessStepKind {
-    LegacyFacade,
-    AgentRuntime,
-    ToolRuntime,
-    RuntimeService,
-    ProductDomain,
-}
-
-#[derive(Debug, Clone, PartialEq, Eq)]
-pub struct HarnessStep {
-    id: String,
-    kind: HarnessStepKind,
-    target: String,
-}
-
-impl HarnessStep {
-    pub fn new(id: impl Into<String>, kind: HarnessStepKind, target: impl Into<String>) -> Self {
-        Self {
-            id: id.into(),
-            kind,
-            target: target.into(),
-        }
-    }
-
-    pub fn id(&self) -> &str {
-        &self.id
-    }
-
-    pub fn kind(&self) -> HarnessStepKind {
-        self.kind
-    }
-
-    pub fn target(&self) -> &str {
-        &self.target
-    }
-}
-
-#[derive(Debug, Clone, PartialEq, Eq)]
-pub struct HarnessPlan {
-    provider_id: HarnessId,
-    workflow: HarnessWorkflow,
-    goal: String,
-    steps: Vec<HarnessStep>,
-}
-
-impl HarnessPlan {
-    pub fn new(
-        provider_id: HarnessId,
-        workflow: HarnessWorkflow,
-        goal: impl Into<String>,
-        steps: Vec<HarnessStep>,
-    ) -> Self {
-        Self {
-            provider_id,
-            workflow,
-            goal: goal.into(),
-            steps,
-        }
-    }
-
-    pub fn provider_id(&self) -> &HarnessId {
-        &self.provider_id
-    }
-
-    pub fn workflow(&self) -> HarnessWorkflow {
-        self.workflow
-    }
-
-    pub fn goal(&self) -> &str {
-        &self.goal
-    }
-
-    pub fn steps(&self) -> &[HarnessStep] {
-        &self.steps
-    }
-}
-
-#[derive(Debug, Clone, Copy, PartialEq, Eq)]
-pub enum HarnessOutcomeStatus {
-    Completed,
-    Skipped,
-}
-
-#[derive(Debug, Clone, PartialEq, Eq)]
-pub struct HarnessOutcome {
-    status: HarnessOutcomeStatus,
-}
-
-impl HarnessOutcome {
-    pub fn new(status: HarnessOutcomeStatus) -> Self {
-        Self { status }
-    }
-
-    pub fn status(&self) -> HarnessOutcomeStatus {
-        self.status
-    }
-}
-
-#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
-pub enum HarnessError {
-    #[error("provider {provider_id} does not support workflow {requested}; supported workflow is {supported}")]
-    UnsupportedWorkflow {
-        provider_id: HarnessId,
-        requested: HarnessWorkflow,
-        supported: HarnessWorkflow,
-    },
-    #[error("provider {provider_id} does not execute workflow {workflow}: {reason}")]
-    UnsupportedExecution {
-        provider_id: HarnessId,
-        workflow: HarnessWorkflow,
-        reason: String,
-    },
-}
-
-#[async_trait]
-pub trait HarnessProvider: Send + Sync {
-    fn id(&self) -> &HarnessId;
-
-    fn workflow(&self) -> HarnessWorkflow;
-
-    fn capabilities(&self) -> &[HarnessCapability];
-
-    async fn plan(&self, ctx: HarnessPlanningContext, input: HarnessInput) -> Result<HarnessPlan, HarnessError>;
-
-    async fn execute(&self, ctx: HarnessExecutionContext, plan: HarnessPlan) -> Result<HarnessOutcome, HarnessError>;
-}
-
-#[derive(Debug, Clone, PartialEq, Eq)]
-pub struct DescriptorHarnessProvider {
-    id: HarnessId,
-    workflow: HarnessWorkflow,
-    capabilities: Vec<HarnessCapability>,
-    legacy_target: String,
-}
-
-impl DescriptorHarnessProvider {
-    pub fn legacy_facade(
-        id: impl Into<String>,
-        workflow: HarnessWorkflow,
-        capabilities: &[HarnessCapability],
-        legacy_target: impl Into<String>,
-    ) -> Self {
-        Self {
-            id: HarnessId::new(id),
-            workflow,
-            capabilities: capabilities
-                .iter()
-                .copied()
-                .filter(|capability| *capability != HarnessCapability::Execute)
-                .collect(),
-            legacy_target: legacy_target.into(),
-        }
-    }
-}
-
-#[derive(Debug, Clone, Copy, PartialEq, Eq)]
-pub struct HarnessProviderDescriptor {
-    provider_id: &'static str,
-    workflow: HarnessWorkflow,
-    capabilities: &'static [HarnessCapability],
-    legacy_target: &'static str,
-}
-
-impl HarnessProviderDescriptor {
-    pub const fn legacy_facade(
-        provider_id: &'static str,
-        workflow: HarnessWorkflow,
-        capabilities: &'static [HarnessCapability],
-        legacy_target: &'static str,
-    ) -> Self {
-        Self {
-            provider_id,
-            workflow,
-            capabilities,
-            legacy_target,
-        }
-    }
-
-    pub const fn provider_id(self) -> &'static str {
-        self.provider_id
-    }
-
-    pub const fn workflow(self) -> HarnessWorkflow {
-        self.workflow
-    }
-
-    pub const fn capabilities(self) -> &'static [HarnessCapability] {
-        self.capabilities
-    }
-
-    pub const fn legacy_target(self) -> &'static str {
-        self.legacy_target
-    }
-
-    pub fn into_provider(self) -> DescriptorHarnessProvider {
-        DescriptorHarnessProvider::legacy_facade(self.provider_id, self.workflow, self.capabilities, self.legacy_target)
-    }
-}
-
-pub fn build_descriptor_harness_registry<I>(descriptors: I) -> Result<HarnessRegistry, HarnessRegistryBuildError>
-where
-    I: IntoIterator<Item = HarnessProviderDescriptor>,
-{
-    let mut builder = HarnessRegistryBuilder::new();
-    for descriptor in descriptors {
-        builder = builder.install_provider(descriptor.into_provider());
-    }
-    builder.build()
-}
-
-#[async_trait]
-impl HarnessProvider for DescriptorHarnessProvider {
-    fn id(&self) -> &HarnessId {
-        &self.id
-    }
-
-    fn workflow(&self) -> HarnessWorkflow {
-        self.workflow
-    }
-
-    fn capabilities(&self) -> &[HarnessCapability] {
-        &self.capabilities
-    }
-
-    async fn plan(&self, _ctx: HarnessPlanningContext, input: HarnessInput) -> Result<HarnessPlan, HarnessError> {
-        if input.workflow() != self.workflow {
-            return Err(HarnessError::UnsupportedWorkflow {
-                provider_id: self.id.clone(),
-                requested: input.workflow(),
-                supported: self.workflow,
-            });
-        }
-
-        Ok(HarnessPlan::new(
-            self.id.clone(),
-            self.workflow,
-            input.goal(),
-            vec![HarnessStep::new(
-                format!("{}.legacy_facade", self.workflow.id()),
-                HarnessStepKind::LegacyFacade,
-                self.legacy_target.clone(),
-            )],
-        ))
-    }
-
-    async fn execute(&self, _ctx: HarnessExecutionContext, plan: HarnessPlan) -> Result<HarnessOutcome, HarnessError> {
-        Err(HarnessError::UnsupportedExecution {
-            provider_id: self.id.clone(),
-            workflow: plan.workflow(),
-            reason: "concrete execution remains on the legacy product path".to_string(),
-        })
-    }
-}
-
-#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
-pub enum HarnessRegistryBuildError {
-    #[error("duplicate harness provider id {provider_id}")]
-    DuplicateProviderId { provider_id: HarnessId },
-}
-
-#[derive(Default)]
-pub struct HarnessRegistryBuilder {
-    providers: Vec<Arc<dyn HarnessProvider>>,
-}
-
-impl HarnessRegistryBuilder {
-    pub fn new() -> Self {
-        Self::default()
-    }
-
-    pub fn install_provider<P>(mut self, provider: P) -> Self
-    where
-        P: HarnessProvider + 'static,
-    {
-        self.providers.push(Arc::new(provider));
-        self
-    }
-
-    pub fn build(self) -> Result<HarnessRegistry, HarnessRegistryBuildError> {
-        let mut provider_ids = HashSet::new();
-        for provider in &self.providers {
-            if !provider_ids.insert(provider.id().clone()) {
-                return Err(HarnessRegistryBuildError::DuplicateProviderId {
-                    provider_id: provider.id().clone(),
-                });
-            }
-        }
-
-        Ok(HarnessRegistry {
-            providers: self.providers,
-        })
-    }
-}
-
-#[derive(Default)]
-pub struct HarnessRegistry {
-    providers: Vec<Arc<dyn HarnessProvider>>,
-}
-
-impl fmt::Debug for HarnessRegistry {
-    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
-        f.debug_struct("HarnessRegistry")
-            .field("provider_ids", &self.provider_ids())
-            .field("workflows", &self.workflows())
-            .finish()
-    }
-}
-
-impl HarnessRegistry {
-    pub fn provider_ids(&self) -> Vec<&str> {
-        self.providers.iter().map(|provider| provider.id().as_str()).collect()
-    }
-
-    pub fn workflows(&self) -> Vec<HarnessWorkflow> {
-        let mut workflows = Vec::new();
-        for provider in &self.providers {
-            let workflow = provider.workflow();
-            if !workflows.contains(&workflow) {
-                workflows.push(workflow);
-            }
-        }
-        workflows
-    }
-
-    pub fn provider_for_workflow(&self, workflow: HarnessWorkflow) -> Option<&dyn HarnessProvider> {
-        self.providers
-            .iter()
-            .find(|provider| provider.workflow() == workflow)
-            .map(|provider| provider.as_ref())
-    }
-
-    pub fn provider_by_id(&self, provider_id: &str) -> Option<&dyn HarnessProvider> {
-        self.providers
-            .iter()
-            .find(|provider| provider.id().as_str() == provider_id)
-            .map(|provider| provider.as_ref())
-    }
-}
diff --git a/src/crates/execution/harness/tests/registry.rs b/src/crates/execution/harness/tests/registry.rs
deleted file mode 100644
index d59a4c5..0000000
--- a/src/crates/execution/harness/tests/registry.rs
+++ /dev/null
@@ -1,131 +0,0 @@
-use northhing_harness::{
-    build_descriptor_harness_registry, DescriptorHarnessProvider, HarnessCapability, HarnessError, HarnessInput,
-    HarnessProvider, HarnessProviderDescriptor, HarnessRegistryBuildError, HarnessRegistryBuilder, HarnessStepKind,
-    HarnessWorkflow,
-};
-
-#[tokio::test]
-async fn registry_registers_multiple_workflow_providers_and_builds_legacy_plan() {
-    let registry = HarnessRegistryBuilder::new()
-        .install_provider(DescriptorHarnessProvider::legacy_facade(
-            "core.deep_review",
-            HarnessWorkflow::DeepReview,
-            &[HarnessCapability::Plan, HarnessCapability::ReviewGate],
-            "northhing-core::agentic::deep_review",
-        ))
-        .install_provider(DescriptorHarnessProvider::legacy_facade(
-            "core.miniapp",
-            HarnessWorkflow::MiniApp,
-            &[HarnessCapability::Plan, HarnessCapability::Artifact],
-            "northhing-core::miniapp",
-        ))
-        .build()
-        .expect("two different workflow providers should register");
-
-    assert_eq!(registry.provider_ids(), vec!["core.deep_review", "core.miniapp"]);
-    assert_eq!(
-        registry.workflows(),
-        vec![HarnessWorkflow::DeepReview, HarnessWorkflow::MiniApp]
-    );
-
-    let provider = registry
-        .provider_for_workflow(HarnessWorkflow::DeepReview)
-        .expect("deep review workflow should resolve");
-    let plan = provider
-        .plan(
-            Default::default(),
-            HarnessInput::new(HarnessWorkflow::DeepReview, "review current branch"),
-        )
-        .await
-        .expect("legacy facade provider should produce a route plan");
-
-    assert_eq!(plan.provider_id().as_str(), "core.deep_review");
-    assert_eq!(plan.workflow(), HarnessWorkflow::DeepReview);
-    assert_eq!(plan.steps().len(), 1);
-    assert_eq!(plan.steps()[0].kind(), HarnessStepKind::LegacyFacade);
-    assert_eq!(plan.steps()[0].target(), "northhing-core::agentic::deep_review");
-
-    let err = provider
-        .execute(Default::default(), plan)
-        .await
-        .expect_err("execution must stay on the legacy path in PR4");
-    assert!(matches!(err, HarnessError::UnsupportedExecution { .. }));
-}
-
-#[test]
-fn registry_rejects_duplicate_provider_ids() {
-    let err = HarnessRegistryBuilder::new()
-        .install_provider(DescriptorHarnessProvider::legacy_facade(
-            "core.deep_review",
-            HarnessWorkflow::DeepReview,
-            &[HarnessCapability::Plan],
-            "northhing-core::agentic::deep_review",
-        ))
-        .install_provider(DescriptorHarnessProvider::legacy_facade(
-            "core.deep_review",
-            HarnessWorkflow::DeepResearch,
-            &[HarnessCapability::Plan],
-            "northhing-core::agentic::agents::definitions::modes::deep_research",
-        ))
-        .build()
-        .expect_err("duplicate provider ids must be rejected");
-
-    assert!(matches!(err, HarnessRegistryBuildError::DuplicateProviderId { .. }));
-}
-
-#[test]
-fn descriptor_registry_builder_installs_legacy_facade_descriptors() {
-    let registry = build_descriptor_harness_registry([
-        HarnessProviderDescriptor::legacy_facade(
-            "core.deep_review",
-            HarnessWorkflow::DeepReview,
-            &[HarnessCapability::Plan, HarnessCapability::ReviewGate],
-            "northhing-core::agentic::deep_review",
-        ),
-        HarnessProviderDescriptor::legacy_facade(
-            "core.deep_research",
-            HarnessWorkflow::DeepResearch,
-            &[HarnessCapability::Plan],
-            "northhing-core::agentic::agents::definitions::modes::deep_research",
-        ),
-    ])
-    .expect("descriptor registry should build");
-
-    assert_eq!(registry.provider_ids(), vec!["core.deep_review", "core.deep_research"]);
-    assert_eq!(
-        registry.workflows(),
-        vec![HarnessWorkflow::DeepReview, HarnessWorkflow::DeepResearch]
-    );
-}
-
-#[test]
-fn legacy_facade_provider_never_exposes_execute_capability() {
-    let provider = DescriptorHarnessProvider::legacy_facade(
-        "core.deep_review",
-        HarnessWorkflow::DeepReview,
-        &[HarnessCapability::Plan, HarnessCapability::Execute],
-        "northhing-core::agentic::deep_review",
-    );
-
-    assert_eq!(provider.capabilities(), &[HarnessCapability::Plan]);
-}
-
-#[tokio::test]
-async fn descriptor_provider_rejects_wrong_workflow_input() {
-    let provider = DescriptorHarnessProvider::legacy_facade(
-        "core.miniapp",
-        HarnessWorkflow::MiniApp,
-        &[HarnessCapability::Plan],
-        "northhing-core::miniapp",
-    );
-
-    let err = provider
-        .plan(
-            Default::default(),
-            HarnessInput::new(HarnessWorkflow::DeepReview, "wrong workflow"),
-        )
-        .await
-        .expect_err("provider should not plan a different workflow");
-
-    assert!(matches!(err, HarnessError::UnsupportedWorkflow { .. }));
-}
diff --git a/src/crates/execution/tool-execution/AGENTS.md b/src/crates/execution/tool-execution/AGENTS.md
index 25d7641..2be08c4 100644
--- a/src/crates/execution/tool-execution/AGENTS.md
+++ b/src/crates/execution/tool-execution/AGENTS.md
@@ -10,22 +10,20 @@ surface.
 ## Guardrails
 
 - Do not depend on `northhing-core`, app crates, Tauri, product-domain crates,
   transport adapters, or AI providers.
 - Keep this crate focused on reusable execution primitives and pure utilities.
   Product-specific tool exposure, prompt-visible manifests, `GetToolSpec`,
   collapsed unlock state, and `ToolUseContext` stay outside this crate.
 - Preserve existing filesystem/search behavior when moving helpers here. Do not
   change path containment, encoding, cancellation, or result presentation
   semantics as a side effect of refactoring.
-- Provider-neutral contracts belong in `tool-contracts` (`northhing-agent-tools`);
-  product provider grouping belongs in `tool-provider-groups`
-  (`northhing-tool-packs`).
+- Provider-neutral contracts belong in `tool-contracts` (`northhing-agent-tools`).
 
 ## Verification
 
 ```bash
 cargo test -p tool-runtime
 node scripts/check-core-boundaries.mjs
 ```
 
 For documentation-only changes, run `git diff --check`.
diff --git a/src/crates/execution/tool-provider-groups/AGENTS.md b/src/crates/execution/tool-provider-groups/AGENTS.md
deleted file mode 100644
index 6878834..0000000
--- a/src/crates/execution/tool-provider-groups/AGENTS.md
+++ /dev/null
@@ -1,32 +0,0 @@
-# tool-provider-groups Agent Guide
-
-Scope: this guide applies to `src/crates/execution/tool-provider-groups`.
-
-`northhing-tool-packs` owns tool feature-group scaffold metadata, the product tool
-provider group plan, and provider-group plan selection by id. It does not own
-concrete tool implementations yet.
-
-## Guardrails
-
-- Keep `default = []`; `product-full` may aggregate feature groups but must not
-  silently enable new runtime behavior. Boundary checks enforce the current
-  feature-group list.
-- Do not depend on `northhing-core`, concrete service crates, app crates, Tauri,
-  Git, MCP, network clients, or CLI UI dependencies unless a reviewed tool
-  runtime owner move explicitly changes this boundary.
-- Do not own manifest/exposure contracts, concrete runtime manifest assembly,
-  `GetToolSpec` execution, collapsed unlock state, snapshot decoration, or
-  `ToolUseContext`. Provider group plans may list group ids and tool names only.
-- Product capability packs may select provider group ids; this crate owns the
-  provider group plan and unknown provider-group validation.
-- Future concrete tool migration must preserve product registry order,
-  expanded/collapsed exposure, prompt stubs, unlock state, cancellation, runtime
-  restrictions, and Deep Review tool flow.
-
-## Verification
-
-```bash
-cargo test -p northhing-tool-packs --features basic
-cargo check -p northhing-tool-packs --features product-full
-node scripts/check-core-boundaries.mjs
-```
diff --git a/src/crates/execution/tool-provider-groups/Cargo.toml b/src/crates/execution/tool-provider-groups/Cargo.toml
deleted file mode 100644
index e040804..0000000
--- a/src/crates/execution/tool-provider-groups/Cargo.toml
+++ /dev/null
@@ -1,22 +0,0 @@
-[package]
-name = "northhing-tool-packs"
-version.workspace = true
-authors.workspace = true
-edition.workspace = true
-description = "northhing concrete tool-pack owner crate"
-
-[lib]
-name = "northhing_tool_packs"
-crate-type = ["rlib"]
-
-[features]
-default = []
-basic = []
-git = []
-mcp = []
-browser-web = []
-computer-use = []
-image-analysis = []
-miniapp = []
-agent-control = []
-product-full = ["basic", "git", "mcp", "browser-web", "computer-use", "image-analysis", "miniapp", "agent-control"]
diff --git a/src/crates/execution/tool-provider-groups/src/lib.rs b/src/crates/execution/tool-provider-groups/src/lib.rs
deleted file mode 100644
index 97eebe5..0000000
--- a/src/crates/execution/tool-provider-groups/src/lib.rs
+++ /dev/null
@@ -1,402 +0,0 @@
-#![allow(clippy::too_many_arguments)]
-//! Concrete tool-pack owner crate.
-//!
-//! The feature scaffold is intentionally behavior-neutral until the core
-//! `ToolUseContext` and registry boundaries are split into portable ports.
-
-use std::collections::HashSet;
-use std::fmt;
-
-#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
-pub enum ToolPackFeatureGroup {
-    Basic,
-    Git,
-    Mcp,
-    BrowserWeb,
-    ComputerUse,
-    ImageAnalysis,
-    MiniApp,
-    AgentControl,
-}
-
-impl ToolPackFeatureGroup {
-    pub const fn id(self) -> &'static str {
-        match self {
-            Self::Basic => "basic",
-            Self::Git => "git",
-            Self::Mcp => "mcp",
-            Self::BrowserWeb => "browser-web",
-            Self::ComputerUse => "computer-use",
-            Self::ImageAnalysis => "image-analysis",
-            Self::MiniApp => "miniapp",
-            Self::AgentControl => "agent-control",
-        }
-    }
-}
-
-pub const ALL_FEATURE_GROUPS: &[ToolPackFeatureGroup] = &[
-    ToolPackFeatureGroup::Basic,
-    ToolPackFeatureGroup::Git,
-    ToolPackFeatureGroup::Mcp,
-    ToolPackFeatureGroup::BrowserWeb,
-    ToolPackFeatureGroup::ComputerUse,
-    ToolPackFeatureGroup::ImageAnalysis,
-    ToolPackFeatureGroup::MiniApp,
-    ToolPackFeatureGroup::AgentControl,
-];
-
-pub fn all_feature_groups() -> &'static [ToolPackFeatureGroup] {
-    ALL_FEATURE_GROUPS
-}
-
-pub fn enabled_feature_groups() -> Vec<ToolPackFeatureGroup> {
-    [
-        (cfg!(feature = "basic"), ToolPackFeatureGroup::Basic),
-        (cfg!(feature = "git"), ToolPackFeatureGroup::Git),
-        (cfg!(feature = "mcp"), ToolPackFeatureGroup::Mcp),
-        (cfg!(feature = "browser-web"), ToolPackFeatureGroup::BrowserWeb),
-        (cfg!(feature = "computer-use"), ToolPackFeatureGroup::ComputerUse),
-        (cfg!(feature = "image-analysis"), ToolPackFeatureGroup::ImageAnalysis),
-        (cfg!(feature = "miniapp"), ToolPackFeatureGroup::MiniApp),
-        (cfg!(feature = "agent-control"), ToolPackFeatureGroup::AgentControl),
-    ]
-    .into_iter()
-    .filter_map(|(enabled, group)| enabled.then_some(group))
-    .collect()
-}
-
-#[derive(Debug, Clone, Copy, PartialEq, Eq)]
-pub struct ToolProviderGroupPlan {
-    provider_id: &'static str,
-    feature_groups: &'static [ToolPackFeatureGroup],
-    tool_names: &'static [&'static str],
-}
-
-impl ToolProviderGroupPlan {
-    pub const fn provider_id(self) -> &'static str {
-        self.provider_id
-    }
-
-    pub const fn feature_groups(self) -> &'static [ToolPackFeatureGroup] {
-        self.feature_groups
-    }
-
-    pub const fn tool_names(self) -> &'static [&'static str] {
-        self.tool_names
-    }
-}
-
-const CORE_BASIC_FEATURE_GROUPS: &[ToolPackFeatureGroup] = &[ToolPackFeatureGroup::Basic];
-const CORE_AGENT_FEATURE_GROUPS: &[ToolPackFeatureGroup] = &[ToolPackFeatureGroup::AgentControl];
-const CORE_SESSION_FEATURE_GROUPS: &[ToolPackFeatureGroup] = &[ToolPackFeatureGroup::AgentControl];
-const CORE_INTEGRATION_FEATURE_GROUPS: &[ToolPackFeatureGroup] = &[
-    ToolPackFeatureGroup::BrowserWeb,
-    ToolPackFeatureGroup::Mcp,
-    ToolPackFeatureGroup::Git,
-    ToolPackFeatureGroup::MiniApp,
-    ToolPackFeatureGroup::ComputerUse,
-    ToolPackFeatureGroup::ImageAnalysis,
-    ToolPackFeatureGroup::AgentControl,
-];
-
-const PRODUCT_TOOL_PROVIDER_GROUP_PLAN: &[ToolProviderGroupPlan] = &[
-    ToolProviderGroupPlan {
-        provider_id: "core.basic",
-        feature_groups: CORE_BASIC_FEATURE_GROUPS,
-        tool_names: &[
-            "LS",
-            "Read",
-            "Glob",
-            "Grep",
-            "Write",
-            "Edit",
-            "Delete",
-            "ExecCommand",
-            "WriteStdin",
-            "ExecControl",
-            "GetTime",
-        ],
-    },
-    ToolProviderGroupPlan {
-        provider_id: "core.agent",
-        feature_groups: CORE_AGENT_FEATURE_GROUPS,
-        tool_names: &[
-            "Task",
-            "Skill",
-            "AskUserQuestion",
-            "TodoWrite",
-            "get_goal",
-            "create_goal",
-            "update_goal",
-            "CreatePlan",
-            "submit_code_review",
-            "GetToolSpec",
-            "GetFileDiff",
-            "Log",
-        ],
-    },
-    ToolProviderGroupPlan {
-        provider_id: "core.session",
-        feature_groups: CORE_SESSION_FEATURE_GROUPS,
-        tool_names: &["SessionControl", "SessionMessage", "SessionHistory", "Cron"],
-    },
-    ToolProviderGroupPlan {
-        provider_id: "core.integration",
-        feature_groups: CORE_INTEGRATION_FEATURE_GROUPS,
-        tool_names: &[
-            "WebSearch",
-            "WebFetch",
-            "ListMCPResources",
-            "ReadMCPResource",
-            "ListMCPPrompts",
-            "GetMCPPrompt",
-            "GenerativeUI",
-            "Git",
-            "ReviewPlatform",
-            "InitMiniApp",
-            "ControlHub",
-            "ComputerUse",
-            "Playbook",
-        ],
-    },
-];
-
-pub fn product_tool_provider_group_plan() -> &'static [ToolProviderGroupPlan] {
-    PRODUCT_TOOL_PROVIDER_GROUP_PLAN
-}
-
-#[derive(Debug, Clone, PartialEq, Eq)]
-pub enum ToolProviderGroupPlanSelectionError {
-    UnknownToolProviderGroup { provider_id: &'static str },
-}
-
-impl fmt::Display for ToolProviderGroupPlanSelectionError {
-    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
-        match self {
-            Self::UnknownToolProviderGroup { provider_id } => {
-                write!(formatter, "unknown tool provider group {provider_id}")
-            }
-        }
-    }
-}
-
-impl std::error::Error for ToolProviderGroupPlanSelectionError {}
-
-pub fn try_product_tool_provider_group_plan_for_ids(
-    provider_ids: &[&'static str],
-) -> Result<Vec<ToolProviderGroupPlan>, ToolProviderGroupPlanSelectionError> {
-    let requested_provider_ids = provider_ids.iter().copied().collect::<HashSet<_>>();
-    let mut found_provider_ids = HashSet::new();
-    let mut plan = Vec::new();
-
-    for group_plan in product_tool_provider_group_plan() {
-        if requested_provider_ids.contains(group_plan.provider_id()) {
-            found_provider_ids.insert(group_plan.provider_id());
-            plan.push(*group_plan);
-        }
-    }
-
-    for provider_id in provider_ids {
-        if !found_provider_ids.contains(provider_id) {
-            return Err(ToolProviderGroupPlanSelectionError::UnknownToolProviderGroup { provider_id });
-        }
-    }
-
-    Ok(plan)
-}
-
-#[cfg(test)]
-mod tests {
-    use super::{
-        all_feature_groups, enabled_feature_groups, product_tool_provider_group_plan,
-        try_product_tool_provider_group_plan_for_ids, ToolPackFeatureGroup, ToolProviderGroupPlanSelectionError,
-    };
-
-    #[test]
-    fn all_feature_groups_cover_planned_tool_pack_scaffold() {
-        let feature_ids = all_feature_groups().iter().map(|group| group.id()).collect::<Vec<_>>();
-
-        assert_eq!(
-            feature_ids,
-            vec![
-                "basic",
-                "git",
-                "mcp",
-                "browser-web",
-                "computer-use",
-                "image-analysis",
-                "miniapp",
-                "agent-control"
-            ]
-        );
-    }
-
-    #[test]
-    fn enabled_feature_groups_reflect_compile_time_features() {
-        let groups = enabled_feature_groups();
-
-        assert_eq!(groups.contains(&ToolPackFeatureGroup::Basic), cfg!(feature = "basic"));
-        assert_eq!(groups.contains(&ToolPackFeatureGroup::Git), cfg!(feature = "git"));
-        assert_eq!(groups.contains(&ToolPackFeatureGroup::Mcp), cfg!(feature = "mcp"));
-        assert_eq!(
-            groups.contains(&ToolPackFeatureGroup::BrowserWeb),
-            cfg!(feature = "browser-web")
-        );
-        assert_eq!(
-            groups.contains(&ToolPackFeatureGroup::ComputerUse),
-            cfg!(feature = "computer-use")
-        );
-        assert_eq!(
-            groups.contains(&ToolPackFeatureGroup::ImageAnalysis),
-            cfg!(feature = "image-analysis")
-        );
-        assert_eq!(
-            groups.contains(&ToolPackFeatureGroup::MiniApp),
-            cfg!(feature = "miniapp")
-        );
-        assert_eq!(
-            groups.contains(&ToolPackFeatureGroup::AgentControl),
-            cfg!(feature = "agent-control")
-        );
-    }
-
-    #[test]
-    fn feature_group_ids_match_cargo_feature_names() {
-        assert_eq!(ToolPackFeatureGroup::Basic.id(), "basic");
-        assert_eq!(ToolPackFeatureGroup::Git.id(), "git");
-        assert_eq!(ToolPackFeatureGroup::Mcp.id(), "mcp");
-        assert_eq!(ToolPackFeatureGroup::BrowserWeb.id(), "browser-web");
-        assert_eq!(ToolPackFeatureGroup::ComputerUse.id(), "computer-use");
-        assert_eq!(ToolPackFeatureGroup::ImageAnalysis.id(), "image-analysis");
-        assert_eq!(ToolPackFeatureGroup::MiniApp.id(), "miniapp");
-        assert_eq!(ToolPackFeatureGroup::AgentControl.id(), "agent-control");
-    }
-
-    #[test]
-    fn product_provider_group_plan_preserves_core_runtime_order() {
-        let provider_ids = product_tool_provider_group_plan()
-            .iter()
-            .map(|group| group.provider_id())
-            .collect::<Vec<_>>();
-
-        assert_eq!(
-            provider_ids,
-            vec!["core.basic", "core.agent", "core.session", "core.integration"]
-        );
-    }
-
-    #[test]
-    fn product_provider_group_plan_preserves_builtin_tool_order() {
-        let tool_names = product_tool_provider_group_plan()
-            .iter()
-            .flat_map(|group| group.tool_names().iter().copied())
-            .collect::<Vec<_>>();
-
-        assert_eq!(
-            tool_names,
-            vec![
-                "LS",
-                "Read",
-                "Glob",
-                "Grep",
-                "Write",
-                "Edit",
-                "Delete",
-                "ExecCommand",
-                "WriteStdin",
-                "ExecControl",
-                "GetTime",
-                "Task",
-                "Skill",
-                "AskUserQuestion",
-                "TodoWrite",
-                "get_goal",
-                "create_goal",
-                "update_goal",
-                "CreatePlan",
-                "submit_code_review",
-                "GetToolSpec",
-                "GetFileDiff",
-                "Log",
-                "SessionControl",
-                "SessionMessage",
-                "SessionHistory",
-                "Cron",
-                "WebSearch",
-                "WebFetch",
-                "ListMCPResources",
-                "ReadMCPResource",
-                "ListMCPPrompts",
-                "GetMCPPrompt",
-                "GenerativeUI",
-                "Git",
-                "ReviewPlatform",
-                "InitMiniApp",
-                "ControlHub",
-                "ComputerUse",
-                "Playbook",
-            ]
-        );
-    }
-
-    #[test]
-    fn product_provider_group_plan_preserves_feature_group_mapping() {
-        let feature_groups = product_tool_provider_group_plan()
-            .iter()
-            .map(|group| {
-                (
-                    group.provider_id(),
-                    group
-                        .feature_groups()
-                        .iter()
-                        .map(|feature_group| feature_group.id())
-                        .collect::<Vec<_>>(),
-                )
-            })
-            .collect::<Vec<_>>();
-
-        assert_eq!(
-            feature_groups,
-            vec![
-                ("core.basic", vec!["basic"]),
-                ("core.agent", vec!["agent-control"]),
-                ("core.session", vec!["agent-control"]),
-                (
-                    "core.integration",
-                    vec![
-                        "browser-web",
-                        "mcp",
-                        "git",
-                        "miniapp",
-                        "computer-use",
-                        "image-analysis",
-                        "agent-control",
-                    ]
-                ),
-            ]
-        );
-    }
-
-    #[test]
-    fn product_provider_group_plan_selector_preserves_product_plan_order_for_requested_ids() {
-        let plan = try_product_tool_provider_group_plan_for_ids(&["core.integration", "core.basic"])
-            .expect("known provider groups should select");
-
-        let provider_ids = plan.iter().map(|group| group.provider_id()).collect::<Vec<_>>();
-
-        assert_eq!(provider_ids, vec!["core.basic", "core.integration"]);
-    }
-
-    #[test]
-    fn product_provider_group_plan_selector_rejects_unknown_provider_ids() {
-        let error = try_product_tool_provider_group_plan_for_ids(&["core.basic", "core.missing"])
-            .expect_err("unknown provider ids must not be silently ignored");
-
-        assert_eq!(
-            error,
-            ToolProviderGroupPlanSelectionError::UnknownToolProviderGroup {
-                provider_id: "core.missing"
-            }
-        );
-    }
-}
