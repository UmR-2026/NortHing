# T2-2 MiniApp 子系统整删 — 删除前侦察报告（recon 基线 commit 3702baf）

> 侦察执行：explore 子代理，2026-08-19；纯侦察未改文件。本文件是 MiniApp 批（M1-M5）的批次划分权威文档。
> 决策依据：decision-register.md:40（P-14 MiniApp 整删，生效）、backend-roadmap.md:96（删除前唯一要求=先摘所有启动入口）、:167（T2-2 行）、:190-192（PCS-3 已提炼 permission_policy 语义，删码依据自足）、handoff 2026-08-18:9（前置①已清）。

## 规模实测（与估值对账）

| 目标 | 实测（行） | 估值 |
|---|---|---|
| `contracts/product-domains/src/miniapp/`（16 个 .rs） | **3,885** | — |
| └ `builtin/assets/`（6 个内置应用资产，含 1 个 27,805 行 vendored bundle） | **55,889** | — |
| `services/services-integrations/src/miniapp/`（11 个 .rs） | **2,989** | — |
| `assembly/core/src/miniapp/`（14 个 .rs） | **2,349** | — |
| `product-domains/tests/`（6 文件，全部 miniapp 专测） | **2,011** | — |
| 顶层 `MiniApp/`（Skills + Demo） | **7,953** | — |
| **Rust 子系统小计** | **9,223 + 2,011 测试 ≈ 11.2k** | roadmap 说 ≈6k / 任务估计 ≈8k |
| 共享文件外科切除点 | 约 15 个文件、每处 1-40 行（见 Q3/Q5） | — |

⚠️ 两处口径偏差（不影响删除范围，影响 brief 行数预估）：① "内置四件套" 实为 **6 个** bundle（证据见 Q2）；② Rust 侧实测 9.2k 超估值。

## Q1. 全量清单（按 crate/层分组）

### A. 纯 MiniApp（可整删）

**contracts/product-domains**（feature `miniapp` 门控，`lib.rs:7-8`）：
| 行 | 文件 |
|---|---|
| 16 | `src/miniapp/mod.rs` |
| 257 / 147 / 504 / 311 | `types.rs` / `permission_policy.rs` / `host_routing.rs` / `lifecycle.rs` |
| 544 / 241 / 113 | `runtime_facade.rs` / `runtime.rs` / `worker.rs` |
| 230 / 64 / 53 | `storage.rs` / `draft.rs` / `exporter.rs` |
| 253 / 424 / 283 / 114 | `compiler.rs` / `customization.rs` / `bridge_builder.rs` / `ports.rs` |
| 331 | `builtin.rs`（BUILTIN_APPS 注册表 + seed 策略） |
| 55,889 | `builtin/assets/`（6 应用全部 html/css/js/json/mjs） |

**services/services-integrations**（feature `miniapp-runtime` 门控，`lib.rs:27-28`）：
`src/miniapp/` 整目录 2,989 行 — `mod.rs`(11) `host_dispatch.rs`(691) `storage.rs`(165) `storage_port.rs`(124) `storage_app_io.rs`(313) `storage_drafts.rs`(237) `storage_imports_io.rs`(122) `storage_tests.rs`(544) `builtin_io.rs`(164) `worker.rs`(177) `worker_pool.rs`(441)

**assembly/core**（feature `product-domains` 门控，`lib.rs:17-18`）：
`src/miniapp/` 整目录 2,349 行 — `mod.rs`(30) `host_dispatch.rs`(40，纯 facade) `js_worker.rs`(4) `js_worker_pool.rs`(276) `runtime_detect.rs`(2) `compiler.rs`(33) `exporter.rs`(33) `storage.rs`(302) `builtin/mod.rs`(584) `manager/mod.rs`(469) `manager/mgr_types.rs`(56) `manager/mgr_registry.rs`(34) `manager/mgr_runtime.rs`(114) `manager/mgr_lifecycle.rs`(372)

**其它纯删文件**：
- `assembly/core/src/agentic/tools/implementations/miniapp_init_tool.rs`（193+28 行，InitMiniApp 工具本体）
- `assembly/core/src/service/announcement/content/tips/{en-US,zh-CN,zh-TW}/013_miniapp.md`（3 文件）
- `product-domains/tests/`：`builtin_and_ports.rs`、`compiler_export_storage_and_runtime.rs`、`host_routing_and_lifecycle_helpers.rs`、`permissions_and_bridge.rs`、`runtime_facade_and_customization.rs`、`common/mod.rs`（6 文件 2,011 行，每个都 `#![cfg(feature = "miniapp")]`）
- 顶层 `MiniApp/`：`Skills/miniapp-dev/`（4 md，696 行）+ `Demo/git-graph/`（≈6,028 行）+ `Demo/icon-design-system/`（≈1,229 行）

### B. 共享文件里的 MiniApp 段落（外科切除，清单见 Q3/Q5 逐条）

## Q2. "内置四件套"真相

`product-domains/src/miniapp/builtin.rs:59-120` `BUILTIN_APPS` 注册 **6 个** bundle（:251-263 有测试锁定 id 列表）：

| id | version | 资产目录 | 资产行数 |
|---|---|---|---|
| builtin-gomoku | 11 | `builtin/assets/gomoku/` | 1,214 |
| builtin-daily-divination | 21 | `builtin/assets/divination/` | 2,880 |
| builtin-regex-playground | 16 | `builtin/assets/regex-playground/` | 1,500 |
| builtin-coding-selfie | 28 | `builtin/assets/coding-selfie/` | 2,286 |
| builtin-pr-review | 3 | `builtin/assets/pr-review/` | 5,748 |
| builtin-ppt-live | 167 | `builtin/assets/ppt-live/` | 42,261（含 `src/vendor/ppt-export.bundle.mjs` 27,805 行 vendored minified bundle + `dist/ui.bundle.js` 632 + `src/` 15 个 js 7,236） |

资产通过 `include_str!` 嵌入（builtin.rs:63-118）。顶层 `MiniApp/Demo/`（git-graph、icon-design-system）是**未注册的演示应用**，`MiniApp/Skills/miniapp-dev/` 是开发技能文档——两者无任何脚本/配置引用（证据：`rg 'MiniApp[\\/]Skills|MiniApp[\\/]Demo|miniapp-dev'` 全仓仅命中历史 docs + miniapp_init_tool.rs:82 描述文案 "see miniapp-dev skill"）。

## Q3. 活消费图谱（关键）

**总判断：MiniApp 在当前树里已是"半死"状态——desktop/CLI/server/ACP/installer 零代码引用；全局 manager 从未被初始化（`initialize_global_miniapp_manager` 全仓零调用方）；headless 检测标记无生产者。真正活着的入口只剩三类：① Cargo feature 链传递编译；② InitMiniApp 工具仍在 agent manifest 暴露（调用即报错）；③ announcement tips 三语言安利卡。**

### 零引用证据（UI/宿主面）
```
rg -l -i 'miniapp|mini_app' src/apps                → 0 命中（desktop/cli/server 全无）
rg -l -i 'miniapp|mini_app' src/crates/interfaces   → 0 命中（ACP 无）
rg -l -i 'miniapp' northing-installer               → 0 命中
rg -i 'miniapp' -g '*.slint' src/apps/desktop       → 0 命中
git log -S 'miniapp' -- src/apps/desktop            → 0 commit（本 git 史 desktop 从未接线 miniapp）
rg 'miniapp' src/crates/.../service/config          → 0 命中；GlobalConfig 字段表（app_shell.rs:43-67）无 miniapp 键；无 MINIAPP_* 环境变量
```
注：handoff 2026-08-18:33 说"前置②UI 入口摘除需前端 session 配合"——**本侦察实测 UI 入口为空集**，与 T2-2c remote 栈结论一致；该前置对 MiniApp 批同样不阻塞（剩余"入口"全在编排线可动范围）。

### 活入口①：Cargo feature 链（编译期拉起）
- 消费端：`desktop/Cargo.toml:15`、`cli/Cargo.toml:14`、`interfaces/acp/Cargo.toml:12` 均 `northhing-core features=["product-full"]`
- core `Cargo.toml`：`product-full`(:172-194) → `"product-domains"`(:191)；`product-domains` feature 块 :197-203，其中 **:201 `"northhing-services-integrations/miniapp-runtime"`**、**:202 `"northhing-product-domains/product-full"`**（传递 miniapp）
- core `lib.rs:17-18`：`#[cfg(feature = "product-domains")] pub mod miniapp;`
- services-integrations `Cargo.toml`：`miniapp-runtime` feature :78-87（含 :80 `northhing-product-domains/miniapp`）；`product-full` :121 含 `"miniapp-runtime"`；`lib.rs:27-28` cfg 门控
- product-domains `Cargo.toml`：**:22 `miniapp = ["dirs", "sha2", "which"]`；:24 `product-full = ["miniapp", "function-agents"]`**；`lib.rs:7-8` cfg 门控
- ⚠️ 共享门控注意：core `product-domains` feature 同时门控 `function_agents`（lib.rs:14-15，**存活功能**）——摘 feature 时只能抽掉 miniapp 两行（:201 整行删、:202 改 `function-agents`），不能删整个 feature。

### 活入口②：InitMiniApp 工具（agent 可见）
- 工具本体：`agentic/tools/implementations/miniapp_init_tool.rs`（:124-126 调用 `try_get_global_miniapp_manager()`，恒报错 "MiniAppManager not initialized"）
- 注册点：`product_runtime/materialization.rs:61`（collapsed 列表）、**:111**（工厂 `"InitMiniApp" => InitMiniAppTool`，import 走 :4 `implementations::*` 通配）；`implementations/mod.rs:51,93`（mod + re-export）
- agent 工具清单：`agentic/agents/mod.rs:96`（`"InitMiniApp".to_string()`）
- 曝光文档：`agentic/tools/agent-tool-exposure.md:44`（`| InitMiniApp | Collapsed | None | - |`）
- 测试锚：`agentic/tools/registry/tests.rs:209`（工具列表）、:349（`assert!(!registry.is_tool_collapsed("InitMiniApp"))`）
- 事件：`miniapp_init_tool.rs:200-204` 发 `miniapp-created` 事件——desktop 无消费者（零命中佐证）

### 活入口③：headless agent-run 限制通路（半死）
- 定义：`agentic/tools/restrictions.rs:8-88`（`is_miniapp_headless_agent_run` + `miniapp_headless_agent_tool_restrictions`，文件共 226 行，其余为 ToolPathPolicy/canonicalize 等**存活共享逻辑，文件不可整删**）
- **唯一真实调用点**：`agentic/coordination/dialog_turn/sub_handle_out.rs:157-158`
- re-export：`agentic/tools/mod.rs:39-42`（`pub use restrictions::{...}` 组内需摘 2 个名字）
- **7 个文件只 import 从未调用**（死 import，顺手清）：`coordinator.rs:36`、`dialog_turn/compaction.rs:41`、`session.rs:41`、`thread_goal.rs:41`、`workspace.rs:41`、`subagent_orchestrator/so_dispatch.rs:17`、`so_types.rs:8`
- 标记生产者：`rg 'miniapp-agent:|miniapp_agent'` 全 src 仅 restrictions.rs 自身（:18,:22,:160,:164 测试）——**无任何代码设置该 surface/created_by，检测恒 false**

### 其它 core 内耦合点
- `product_domain_runtime.rs:14,25-27`：`CoreProductDomainRuntime::miniapp_runtime_facade`——**零调用方**（rg 实测仅定义点 + product-domains 测试）；该文件其余方法服务 function_agents（存活），只摘 miniapp 方法 + use 行
- `infrastructure/app_paths/path_manager/user_paths.rs:99-106`：`miniapps_dir()` / `miniapp_dir(app_id)`；**`path_manager/init.rs:35` 每次启动创建 `~/.config/northhing/data/miniapps/` 目录**（启动副作用，roadmap "配置/拉起"语义上的入口）；`path_manager.rs:9` 文档注释提及。消费方仅 miniapp_init_tool.rs:186 与 core miniapp/storage+manager
- `manager/mod.rs:22,27`：`initialize_global_miniapp_manager` / `try_get_global_miniapp_manager`——初始化函数**全仓零调用方**（含 desktop 启动路径），manager 永远不会被创建

### product-capabilities（capability 选择入口）
- `assembly/product-capabilities/src/lib.rs`：:17 `MiniApp` 变体、:26 `=> "miniapp"`、:366-371 `MINIAPP_SERVICES`、:386-389 注册进 `DEFAULT_PRODUCT_CAPABILITY_PACKS`
- 消费方：core `product_assembly.rs:8-10` 仅泛型 re-export（无 miniapp 专行）；测试断言见 Q5
- contracts 层死变体（**serde wire 契约，删除需 brief 显式授权**，参照 T2-2f C4 惯例）：
  - `core-types/src/surface.rs:52` `RuntimeArtifactKind::MiniApp`（无任何构造/match，rg 实测）
  - `services-core/src/session/session_metadata.rs:27` `SessionRelationshipKind::Miniapp`（零构造）
  - `services-core/src/session/lineage.rs:19` `BRANCH_EXCLUDED_TAGS` 含 `"miniapp"`（无生产者设置该 tag）

## Q4. 边界/工具链引用

**boundary checker（必须随各批同 commit 同步，house rule 2；required-rules 里多条规则强制 miniapp 文件/行存在，不摘即红）：**
- `scripts/core-boundaries/rules/feature-rules.mjs`：:50,52,56,59,65,77,78（services-integrations dep→ownerFeatures 表含 `miniapp-runtime`）；**:86-88**（product-domains `dirs`/`sha2`/`which` 独占 `['miniapp']`）；:141（services-integrations requiredProductFullFeatures 含 `'miniapp-runtime'`）；:151（product-domains requiredProductFullFeatures 含 `'miniapp'`）
- `rules/source/required-rules.mjs`：**324 处命中**，主块 :2447（强制 core Cargo.toml product-domains feature 含 miniapp-runtime）、:2495（强制 lib.rs `pub mod miniapp`）、:5370-6816 大段（逐文件锚定 core/services-integrations/product-domains 全部 miniapp 文件与符号）
- `rules/source/forbidden-rules.mjs`：56 处，:480-510（core miniapp facade 禁令）、:689、:1878、:2341
- `self-test.mjs`：**82 处锚点**，含 :613-616（Command::new 例外仅限 miniapp runtime.rs）、:2120/:2134（feature/模块存在性自检）、:2202-2710（逐文件契约锚）
- `crate-layout.mjs`：**无 miniapp 条目**（miniapp 非独立 crate）
- **依赖清理提示**：services-integrations 的 miniapp 关联 optional dep **全部为共享 owner，无 orphan**（feature-rules.mjs:50-78：base64/reqwest 共享 mcp，dirs/uuid 共享 remote-ssh-concrete，which 共享 workspace-search）——M3 只需删 feature 块，**不动 [dependencies]**；product-domains 的 `dirs`/`sha2`/`which` 是 miniapp 独占（:86-88），M4 随 feature 删

**i18n / 构建 / CI / pnpm：**
- `scripts/i18n-audit.mjs:1823-1827`：`core-miniapp` locale-format 扫描面（root 指向 `product-domains/src/miniapp/builtin/assets`，predicate=.js）——M4 删目录后该 spec 指向不存在路径，需同批摘（i18n 工程 frozen + 该脚本有 pre-existing 语法损伤，改动仅限删此 5 行 spec，见 Q6）
- `locales.json`、`i18n-governance-baseline.json`、`i18n-hardcoded-baseline.json`、`i18n-contract.test.mjs`、`generate-i18n-contract.mjs`：**零 miniapp 命中**（rg 实测）
- 根 `package.json`、`pnpm-workspace.yaml`、`.github/`、根 `Cargo.toml` members、`Cargo.lock`：**零 miniapp 命中**
- `assembly/core/build.rs:303-306`：announcement 嵌入是 **build 脚本目录扫描**（`content/{tips,features}/{locale}/*.md`），删 013_miniapp.md 无需改代码
- e2e：`tests/e2e/specs/l0-navigation.spec.ts:14`、`l1-navigation.spec.ts:18,173,194,232` 引用选择器 `.northhing-nav-panel__miniapp-entry`——**src/ 内零对应物（死选择器）**，wdio 配置仍跑这两 spec（tests/e2e/package.json:14,23；wdio.conf_l0.ts:10）

**文档面（需同 commit 同步）：**
- `docs/status/surfaces.md:22`（MiniApp UI Frozen 行）
- 根 `AGENTS.md:26,35,176,179`；`AGENTS-CN.md:25,34,137,140`（:137 骨架不变量提 "MiniApp string 模式命令…拒绝"——guard 本体保留，只改措辞）
- `README.md:43`（Frozen-experimental 枚举）；`docs/tech-debt-cleanup-guide.md:12,75,115`
- `src/crates/services/AGENTS.md:7,15,22` + `AGENTS-CN.md:5,12,17`；`services-integrations/AGENTS.md:34-37`；`services-core/AGENTS.md:25`；`product-domains/AGENTS.md:24,29` + `AGENTS-CN.md:17,21`
- `backend-roadmap.md`：:85(SW1-1)、:96、:117、:151(T1-1)、:167(T2-2 标 done)、:179(PCS-3)、:185(T2-5 miniapp::manager 行)、:216(T3-5)、:247；`decision-register.md:40`（P-14 补执行回链）
- `docs/archive/**`、`docs/handoffs/**`、`docs/superpowers/plans/**`、`docs/migration-2026-07-16/**`、`research/**`：历史文档，按惯例**不改**

## Q5. 测试面

**纯 miniapp 专测（随目录删）：**
- `product-domains/tests/` 6 文件 2,011 行（全部 `#![cfg(feature = "miniapp")]`，该 crate tests/ 目录 100% 是 miniapp）
- `services-integrations/src/miniapp/storage_tests.rs`（544 行，在 src 内随目录删）
- core miniapp 内联测试：`builtin/mod.rs` 内（:171,:198,:216,:243,:253,:277 等）、`manager/` 内 cfg(test)
- `product-domains/src/miniapp/builtin.rs:242-361` 内联测试（含 :276 `#[ignore]` 的 ppt-live 契约测试）

**共享测试文件里的 miniapp 段（外科切除）：**
| 文件 | 行 | 内容 |
|---|---|---|
| `core/src/agentic/tools/registry/tests.rs` | :209, :349 | InitMiniApp 列表项 + collapsed 断言 |
| `core/src/agentic/tools/restrictions.rs` | :149-167 | 2 个 miniapp_headless_* 测试（:170 起 `runtime_restrictions_allow_all_when_empty` 等存活测试保留） |
| `execution/agent-stream/src/tool_call_accumulator.rs` | :150 | 测试用例表 `("InitMiniApp", "Markdown Viewer")` 一行 |
| `assembly/product-capabilities/tests/product_capabilities.rs` | :19, :31, :86 | capability id 列表断言含 `"miniapp"` |
| `tests/e2e/specs/l0-navigation.spec.ts` / `l1-navigation.spec.ts` | 见 Q4 | 死选择器段 |

## Q6. i18n 面

- `locales.json`：零 miniapp 引用（无 miniapp surface 注册——与 mobile-web 不同，miniapp 从未入 i18n 契约）
- 唯一挂点：`scripts/i18n-audit.mjs:1823-1827` `core-miniapp` locale-format 扫描 spec（审计 builtin assets 里的 `Intl.*`/`toLocale*` 用法）
- baseline JSON / contract test / generate 脚本：零命中
- 约束遵守：i18n 工程 frozen 且 i18n-audit.mjs 有 pre-existing 语法损伤——**本侦察未动**；建议 M4 同批仅删 :1823-1827 那 5 行 spec 对象（删除面注册，不触碰存活逻辑），并在 brief 注明该脚本当前不可运行的 pre-existing 状态

## Q7. 与其它活跃功能的耦合（复用检查）

| 候选复用点 | 实测结论 | 证据 |
|---|---|---|
| `permission_policy` 被 PCS/skills/MCP 复用？ | **无**。消费方仅 miniapp 内部 + 专测。PCS-3 语义已提炼进 roadmap:190-192（自足，明确"不回溯旧码"） | rg `resolve_policy` 全 src 仅 miniapp + tests；roadmap:190 |
| miniapp `types`（MiniAppPermissions 等）被外部复用？ | 仅 miniapp_init_tool.rs:6 + core miniapp | rg `crate::miniapp::types` |
| runtime IO / worker_pool 被其它服务复用？ | **无**。`dispatch_host`/`is_host_primitive`/`JsWorkerPool`/`MiniAppStorage` 外部调用数为零（core facade 后无下家） | rg 实测，Q3 |
| `host_routing`/`bridge_builder`（build_bridge_script/build_csp_content） | 消费方仅 product-domains 内部 + 专测 | rg 实测 |
| restrictions.rs 文件 | **共享**：ToolPathOperation/ToolPathPolicy/ToolRuntimeRestrictions re-export 自 `northhing_agent_tools`（:2-4）+ `is_local_path_within_root`（:95+）均存活，只摘 :8-88 miniapp 段与 :149-167 测试 | 文件实读 |
| PathManager miniapp 方法 | 仅 miniapp 消费；`init.rs:35` 建目录副作用随删 | rg `miniapp_dir|miniapps_dir` |
| product_domain_runtime.rs | 共享：function_agents 三方法存活（commit_generator/startchat 等 4 处消费），只摘 :14 + :25-27 | rg `CoreProductDomainRuntime` |
| core-types `RuntimeArtifactKind::MiniApp` / services-core `SessionRelationshipKind::Miniapp` / lineage "miniapp" tag | 全部零构造/零生产者的死变体； serde 契约层，**删除需 brief 显式授权**（建议：保留变体只删 tag 也行，整删也行——交用户拍板，参照 E-09 惯例） | rg 实测 |
| `northhing_product_domains` dep 本体 | **保留**：function-agents 存活在用（core Cargo.toml:106 不可删） | — |

## Q8. 建议删除批次划分

**总判断：无前端 session 依赖（UI 入口为空集）。依赖方向 core → services-integrations → product-domains，自上而下删，每批独立绿。boundary 规则强制 miniapp 存在（required-rules 324 处），必须"删到哪层就同 commit 摘哪层规则"。**

- **M1 — 入口摘除（agent 面 + 用户面，先灭拉起路径）**：materialization.rs:61,111 摘 InitMiniApp 注册 + 删 miniapp_init_tool.rs + implementations/mod.rs:51,93 + agents/mod.rs:96 + agent-tool-exposure.md:44 + registry/tests.rs:209,349；sub_handle_out.rs:157-158 死分支 + tools/mod.rs:39-42 摘名 + restrictions.rs :8-88,:149-167 miniapp 段 + 7 个死 import 行；product-capabilities lib.rs :17,:26,:366-371,:386-389 + tests :19,:31,:86；tips 013_miniapp.md ×3（build.rs 自动重扫，零代码改动）；e2e 死选择器 5 处。预估 ≈600 行删。**门禁**：`cargo check --workspace` + `cargo check -p northhing` + `node scripts/check-core-boundaries.mjs`。完成后：agent 不可见 InitMiniApp、无 capability 选择、无 tips 安利、headless 假通路灭。前置：无。
- **M2 — assembly/core miniapp 整删**：`src/miniapp/`（2,349 行）+ lib.rs:17-18 + Cargo.toml :201 删行 + :202 改 `"northhing-product-domains/function-agents"` + product_domain_runtime.rs:14,25-27 + path_manager（user_paths.rs:99-106、init.rs:35、path_manager.rs:9 注释）。boundary 同步：required/forbidden/self-test 中 core miniapp 锚（含 :2447/:2495 两条强制入口规则）。预估 ≈2.5k。前置：M1。门禁：同上（含 desktop MSVC 门禁）。
- **M3 — services-integrations miniapp 整删**：`src/miniapp/`（2,989 行）+ lib.rs:27-28 + Cargo.toml :78-87 feature 块 + :121 product-full 摘名；feature-rules.mjs :50,52,56,59,65,77,78 摘 miniapp-runtime owner 名（**无 orphan dep**）；required/forbidden/self-test 对应锚；services-integrations/AGENTS.md:34-37、services/AGENTS.md(CN) miniapp 措辞。预估 ≈3.1k。前置：M2。门禁：`cargo check --workspace` + boundary。
- **M4 — product-domains miniapp 整删（含内置 6 件套资产）**：`src/miniapp/`（3,885 rs + 55,889 资产）+ `tests/` 6 文件（2,011）+ lib.rs:7-8 + Cargo.toml :15,17,18 独占 optional dep（dirs/sha2/which）+ :22 feature + :24 product-full 改 `["function-agents"]`；i18n-audit.mjs:1823-1827 摘 core-miniapp spec（frozen 约束见 Q6）；product-domains AGENTS.md:24,29 / AGENTS-CN:17,21；required-rules :5370-6816 大段 + forbidden :480-510 等 + self-test 全部 miniapp 锚。预估 ≈6k rs/test + 55.9k 资产。前置：M3。门禁：`cargo test -p northhing-product-domains --no-default-features` + `cargo test -p northhing-product-domains --features function-agents` + `cargo check --workspace` + boundary。
- **M5 — 顶层 MiniApp/ + 契约死变体 + 文档收口**：`MiniApp/`（7,953 行）整删；**决策点**（需用户/brief 授权）：core-types surface.rs:52、services-core session_metadata.rs:27 两个 serde 变体删/留，lineage.rs:19 tag 与 tool_call_accumulator.rs:150 测试串顺手清；surfaces.md:22、根 AGENTS.md/CN 6 行、README:43、tech-debt-cleanup-guide:12,75,115、roadmap :85,:96,:117,:151,:167 标 done + :185 T2-5 行处理 + decision-register P-14 回链 + T1-1/T3-5/SW1-1 关闭标注；boundary/self-test 残余锚点终扫（`rg -i miniapp scripts/core-boundaries` 归零）。预估 ≈8k（主要是 MiniApp/ 目录）+ 文档行。前置：M1-M4。门禁：`cargo check --workspace` + `cargo check -p northhing` + boundary 归零自检。

**风险标注：**
1. M1 触碰 agentic 热点文件（coordinator/dialog_turn/subagent_orchestrator 8 文件 import 行 + sub_handle_out 分支）——行数小但文件热，建议 reviewer ≥ 中档；restrictions.rs 不可整删（Q7）。
2. core `product-domains` feature 与 function_agents 共享（M2 只抽两行，别动整个 feature）；product-domains `product-full` 同理（M4）。
3. `cargo check -p northhing`（MSVC，P2-15 教训）M1/M2/M5 必跑。
4. 每批收束自检：`rg -i 'miniapp' <本批范围>` 归零 + `rg -i 'miniapp' scripts/core-boundaries` 计数递减，终态归零。
5. e2e 死选择器（M1）与 i18n-audit spec（M4）均为"删除即安全"，但两处属 frozen/跨 session 敏感面——建议在 brief 里各写一行授权说明。
