# T2-2a 侦察报告 — 死代码删除第一批候选（只侦察，未改任何文件）

- 基线：HEAD `e65d98e`（main）
- 行数口径：`rs_only` = 目录内全部 `*.rs` 的 `(Get-Content).Count` 合计；`total` 含 toml/md/fixture。review 的 4 个数字（4,393 / 402 / 894 / 571）全部与 rs_only 口径精确吻合。
- 对账总表：

| 候选 | review 数字 | 实测 rs | 实测 total | 数字对账 |
|---|---|---|---|---|
| insights | 4,393 | 4,393 | 4,711（33 文件，含 10 个 prompts/*.md 共 305 行） | 吻合 |
| tool-provider-groups | 402 | 402（lib.rs 单文件） | 456 | 吻合 |
| plan-compliance-checker | 894 | 894（src 664 + tests 230） | 993 | 吻合 |
| harness | 571 | 571（lib.rs 440 + tests/registry.rs 131） | 619 | 吻合 |
| webdriver | （未给数） | 6,044 | 6,109（72 文件） | 补充：比 insights 还大 |
| cli-internal | （未给数） | 323（main.rs 单文件） | 366 | 补充 |

---

## 1. insights —— 结论：确认可删（零代码引用成立），有 2 处名义关联

**路径**：`src/crates/assembly/core/src/agentic/insights/`（33 文件；4,393 rs 行 / 4,711 总行；含 html/ 4 文件 903 行、prompts/ 10 个 md 305 行）

**零引用证据（已搜模式清单）**：
- `rg "insights" src -g "*.rs" -g "!**/agentic/insights/**"` → 仅 3 命中：`agentic/mod.rs:55`（模块声明本体）、`service/workspace/accessors.rs:63` 与 `service_state.rs:38`（均为 doc 注释里 "metadata (insights, maintenance...)"，语义无关）
- `rg "insights::" src -g "*.rs"`（目录外）→ 0
- `rg "InsightsService|InsightCollector|InsightsHtml|generate_insights|render_insights|InsightFacet|FacetCache"` 全仓 → 0
- `rg -i "insight"`：kernel_facade/ → 0；src/apps（cli+desktop）→ 0；*.slint → 0；mobile-web / installer → 0；contracts/ → 仅 `product-domains/src/miniapp/builtin/assets/divination/ui.js`（内置 MiniApp 的 JS 文案，与 Rust 模块无关）；core Cargo.toml → 0；core/tests/ → 0
- feature flag：无；配置 schema：无；kernel_facade 接线：无；UI 入口：无；CI/scripts：无

**删除操作清单**：
1. 删目录 `src/crates/assembly/core/src/agentic/insights/`
2. 改 `src/crates/assembly/core/src/agentic/mod.rs`：删 54-55 两行（原文：`// Insights module` / `pub mod insights;`）
3. 文档同步：**surfaces.md 无 insights 行、根 AGENTS.md 无 insights 行**（均核实），无需同步。CHANGELOG.md:52 为历史拆分记录，不动。

**连带影响**：feature flag 无 / 配置无 / UI 无 / CI 无 / 脚本无。
**名义关联（非代码引用）**：`tests/e2e/specs/insights-screenshot.spec.ts`（32 行）名为 insights 场景截图，实为 WDIO 开 app 截屏，不触达 insights 模块；且该 e2e 栈本身是 Tauri 时代化石（见 §4）。

## 2. tool-provider-groups —— ⚠️ 风险：review 的"零调用"被推翻

**路径**：`src/crates/execution/tool-provider-groups/`（package `northhing-tool-packs`，lib `northhing_tool_packs`；lib.rs 402 行 + Cargo.toml 20 + AGENTS.md 26）"自认 behavior-neutral" 属实（lib.rs:3-4 注释原文："The feature scaffold is intentionally behavior-neutral until..."）

**依赖方证据（非零）**：
- `src/crates/assembly/product-capabilities/Cargo.toml:15` 硬依赖；`product-capabilities/src/lib.rs:15-16` 实际使用 `try_product_tool_provider_group_plan_for_ids` / `ToolProviderGroupPlan`；其 tests/product_capabilities.rs:2 亦用
- `src/crates/assembly/core/Cargo.toml:100` optional 依赖；`:232` `tool-packs = ["dep:northhing-tool-packs", "northhing-tool-packs/product-full"]`；`:203` `"tool-packs"` 编入 `product-full`（desktop/cli/acp/cli-internal 全部以 product-full 依赖 core，见各自 Cargo.toml）
- `core/src/agentic/tools/product_runtime/materialization.rs:9`：`use northhing_tool_packs::ToolProviderGroupPlan;`
- 边界检查器深度硬编码：`scripts/core-boundaries/rules/crate-layout.mjs:16`、`crate-rules.mjs:16`（另 :36/:63/:91 forbiddenDeps 字符串）、`feature-rules.mjs:15,:138,:145-150`、`self-test.mjs:133,:161,:713-733,:1972-1976,:2313-2326`（self-test 甚至断言 core Cargo.toml 里 `northhing-tool-packs = { path = ... }` 原文存在）。该检查器在 CI 跑（`.github/workflows/ci.yml:132,143`；package.json:21 `check:core-boundaries`）

**删除操作清单（若仍要删）**：根 Cargo.toml:24 member 行；core Cargo.toml:100、:203（"tool-packs"）、:232；materialization.rs 的 ToolProviderGroupPlan 消费点；product-capabilities 的 Cargo.toml:15 + lib.rs:15-16 + tests；上述 4 个 boundary 规则文件全部同步；surfaces.md:36 行、execution/AGENTS.md:20 行、根 AGENTS.md:27（L5 行）、core/AGENTS.md:83 行。
**风险标注**：**此项不是"直接删"**。删它需要同步改活的 product-capabilities 与 core feature 装配，且 CI 边界自测会红。建议从 T2-2a 剔除或单独立项。

## 3. 空/平行 session 目录 —— 确认：唯一全空的是顶层 `src/agentic/session/`（git 未跟踪）

**全仓 session* 目录清单（files = 递归文件数）**：
| 路径 | files | 状态 |
|---|---|---|
| `src/agentic/session/` | 0 | **全空；`git ls-files` 零命中（未跟踪）** —— R-18 所指 |
| `src/crates/assembly/core/src/agentic/session/` | 54（8,627 行） | 活：SessionManager 本体 |
| `.../agentic/session/session_persistence/` | 5 | 活 |
| `.../agentic/session/session_manager_lifecycle_tests/` | 5 | 活 |
| `.../agentic/tools/implementations/session_message_tool/` | 6 | 活 |
| `src/crates/assembly/core/src/service/session/` | 1（mod.rs 3 行 re-export shim） | **活**：`crate::service::session::*` 被 coordinator/compaction/sar_types 等 10+ 处引用；`service/mod.rs:31` 声明 |
| `src/crates/assembly/core/src/service/session_usage/` | 11 | 活（service/mod.rs:33） |
| `src/crates/services/services-core/src/session/` | 12 | 活（shim 的源头） |
| `src/crates/services/services-core/src/session_usage/` | 5 | 活 |
| `src/crates/services/terminal/src/session/` | 11 | 活 |

**删除操作清单**：仅 `src/agentic/session/` 空目录 —— git 不跟踪空目录，删除是纯文件系统动作，**无 commit 面**。
**风险标注**：R-18"5 个平行 session 目录（含全空的 src/agentic/session/）"表述有歧义：核心 crate 里的同名 `agentic/session/` 是 8.6k 行活代码；若按字面路径 `src/crates/.../agentic/session/` 理解会误删活模块。派发 fixer 时必须写绝对路径 `northing/src/agentic/session/`。另：`src/web-ui/` 仅剩 1 文件（AGENTS.md 宣称 missing），`src/shared/` 4 文件 —— 顶层目录卫生可顺带核。

## 4. webdriver —— 结论：确认死 crate（零依赖方），但连带面比 review 写的大

**路径**：`src/crates/adapters/webdriver/`（package `northhing-webdriver`；72 文件，6,044 rs / 6,109 总行）

**零依赖方证据**：
- Cargo.lock 中 `northhing-webdriver` 仅出现 1 次（自身 package 条目，:6285），无任何其他 package 的 dependencies 引用
- `rg "northhing_webdriver|northhing-webdriver" src -g "*.rs"` → 0；全部 crate Cargo.toml → 0
- desktop（Slint）Cargo.toml 与 src 均无 webdriver 引用；crate 自身 Cargo.toml 依赖 tauri/webview2-com/gtk/webkit2gtk —— Tauri 时代化石坐实
- 名义契约：`tests/e2e/config/embedded-driver.ts:427` 以 `northhing_WEBDRIVER_PORT` 环境变量启动桌面 app，crate `src/lib.rs:25` 也读该变量 —— 但无任何宿主把 crate 编进二进制；且 e2e 探针等待 `window.__TAURI__`（desktop 已无 Tauri）、dev-server 指向不存在的 `src/web-ui` → 整条 e2e 栈同为化石。删 crate 不会让现状更坏，但 tests/e2e 是连带死代码（超出本批范围，标记）

**删除操作清单**：
1. 根 Cargo.toml:11 member 行 `"src/crates/adapters/webdriver",`
2. 删目录（含其 AGENTS.md）
3. `scripts/core-boundaries/rules/crate-layout.mjs:28` 删 webdriver 行；`crate-rules.mjs:20` 从 `noCoreDependencyCrates` 删 `'webdriver',`
4. surfaces.md:46 删行（原文：`| \`webdriver\` | \`src/crates/adapters/webdriver\` | WebDriver adapter |`）
5. `src/crates/adapters/AGENTS.md:15` 删行（原文：`| \`webdriver\` | Embedded WebDriver protocol and browser automation adapter | [AGENTS.md](webdriver/AGENTS.md) |`）
6. 根 AGENTS.md:25 L3 行改写（原文：`| 3 | Adapters | \`src/crates/adapters\` | AI/WebDriver protocol adapters and external-provider translation | \`ai-adapters\`, \`webdriver\` | [AGENTS.md](src/crates/adapters/AGENTS.md) |`）
7. 可连带清理根 Cargo.toml 孤儿 workspace 依赖（仅 webdriver 消费，已核全仓 toml）：`block2`(:183)、`objc2`(:184)、`objc2-foundation`(:185)、`objc2-app-kit`(:186)、`objc2-web-kit`(:188)、`webview2-com`(:190)、`glib`(:193)、`gtk`(:194)、`webkit2gtk`(:195)；`objc2-vision`(:187) 连 webdriver 都不用（纯孤儿）
8. `scripts/dev.cjs:35` 引 `src/crates/webdriver`（路径本就过期，watch 列表整段 30-35 指向旧布局，顺手清）
**连带影响**：feature flag 无（crate 自带 `embedded` feature 无外部引用）/ 配置无 / UI 无 / CI 无直接 job（但 boundary 规则见上）/ 脚本：dev.cjs:35（过期）。

## 5. enigo / screenshots —— 结论：确认零消费，删 2 行

- 声明位置：根 Cargo.toml:175 `screenshots = "0.8"`、:176 `enigo = "0.2"`（[workspace.dependencies] "Desktop support" 段）
- 证据：Cargo.lock 中 `enigo`/`screenshots` **零命中**（从未进锁文件）；全仓 crate Cargo.toml 零引用；全仓 `*.rs` 零 `use`（仅有 prompt/注释里的英文单词 "screenshots"，语义无关）
- 删除操作清单：删根 Cargo.toml:175-176 两行；无 surfaces.md/AGENTS.md 行；无 CI/脚本引用
- **顺带发现（超出 R-22 范围，同性质）**：`resvg`(:177)、`atspi`(:178)、`leptess`(:179)、`core-foundation`(:180)、`core-graphics`(:181)、`dispatch`(:182) 同为零消费孤儿声明（全仓 toml 核实；installer src-tauri 亦未用）。是否同批清理由编排拍板。

## 6. cli-internal 零使用 core 依赖 —— 结论：确认，且不止 core 一个

**crate**：`src/crates/support/cli-internal`（package `northhing-cli-internal`，bin `northhing-internal`；main.rs 323 行；无 dependent——纯 bin crate）
**main.rs 实际使用**：anyhow、clap、tokio(宏)、rand、tracing-subscriber（+std）。
**零使用依赖（`src/crates/support/cli-internal/Cargo.toml` 行号）**：
- **:14 `northhing-core = { path = "../../assembly/core", default-features = false, features = ["product-full"] }`** ← R-22/SW2-2 所指（拽进整个 product-full 依赖树）
- :15 `northhing-events`、:21 `dirs`、:22 `toml`、:26 `thiserror`、:28 `tracing`（用了 tracing-subscriber 但没用 tracing 宏）、:35 `uuid`、:36 `chrono`、:39 `sha2` 同样零使用
**删除操作清单**：删 :14（必须）；:15/:21/:22/:26/:28/:35/:36/:39 建议同删（grep 证据齐）。surfaces.md:56 有行 `| \`cli-internal\` | \`src/crates/cli-internal\` | CLI internal utilities |`——crate 保留则行保留，但路径本就错（实为 `src/crates/support/cli-internal`，R-20 已记 3 周未修），可顺手改路径。boundary 规则仅 crate-layout.mjs:35 涉 crate 本体，删依赖不触规则。
**附注**：crate 全部 handler 均为 "not yet implemented" 占位；main.rs:12 引用 `docs/internal/cli.md`——**该文件不存在**（Test-Path=False）。整 crate 删除属另一决策，不在本批。

## 7. plan-compliance-checker —— 结论：确认近死（零 CI/脚本调用）

**路径**：`tools/plan-compliance-checker`（21 文件；894 rs 行：src 664 + tests 230；total 993 含 fixtures/README）
**证据**：根 Cargo.toml:32 member 行；`.github/workflows` 零命中；package.json 零命中；`scripts/` 内仅孤儿脚本 `copy_reference.cjs:49-64`（不在 package.json/CI，7/22 台账已记 self-test 孤儿同类问题）引用其 6 个源文件作"参考拷贝"；文档引用全部是历史记录（docs/plans/2026-06-23 两篇、docs/superpowers/specs/2026-06-19 设计稿、docs/archive/handoffs、research/audit_redim03.md、backend-roadmap.md:167 T2-2 计划行本身）——无需同步
**删除操作清单**：根 Cargo.toml:32 member 行；删目录；surfaces.md:58 删行（原文：`| \`plan-compliance-checker\` | \`tools/plan-compliance-checker\` | Plan compliance tooling |`）；`copy_reference.cjs:49-64` 六条引用随孤儿脚本一并处理（脚本本身无 caller）。
**连带影响**：workspace 依赖 `pulldown-cmark`（根:135）仍被 `src/apps/cli/Cargo.toml:43` 使用——保留。

## 8. harness —— ⚠️ 风险：标称 frozen 但编译期已接线；与 docs/sdlc-harness 同名不同物

**路径**：`src/crates/execution/harness`（package `northhing-harness`；571 rs 行 = lib.rs 440 + tests/registry.rs 131；+Cargo.toml 17 +AGENTS.md 31 = 619）
**依赖方证据（非零）**：
- `core/Cargo.toml:91` **非 optional** 硬依赖（`northhing-harness = { path = "../../execution/harness" }`）——但 core 非测试代码零使用，仅 `agentic/harness.rs:9`（#[cfg(test)]）用其类型
- `product-capabilities/Cargo.toml:13` 硬依赖；`lib.rs:10-13` 非测试代码真实使用 `build_descriptor_harness_registry`/`HarnessCapability`/`HarnessProviderDescriptor`/`HarnessRegistry`/`HarnessRegistryBuildError`/`HarnessWorkflow`；其 tests 亦用
- core 侧接线：`agentic/mod.rs:26` `pub mod harness;`；`agentic/harness.rs:1-5` facade re-export `product_harness_registry`；`product_assembly.rs:9` re-export `default_product_harness_registry`
- 运行时语义：registry 仅在测试中构建；provider `execute()` 按设计报错（测试原文："PR4 must not move concrete workflow execution out of legacy paths"）——是"已接线的脚手架"，不是"零引用"
**与 docs/sdlc-harness 关系**：`rg "execution/harness" docs/sdlc-harness` 零命中 —— **同名不同物确认**。surfaces.md:25 把 "SDLC Harness" 标签贴到该 crate 路径（原文：`| **SDLC Harness** | \`src/crates/execution/harness/\` | 🧊 Frozen | Test/eval harness; not user-facing. |`），属误标。
**test-support 覆盖**：`rg "harness" src/crates/support/test-support` 零命中 —— backend-roadmap T2-2 的"或并入 test-support"无现成承接面，test-support 只做 fixture/offline profile。
**删除操作清单（若删）**：根 Cargo.toml:22 member 行；core Cargo.toml:91；product-capabilities Cargo.toml:13 + lib.rs 的 registry 构建段 + tests；core 的 agentic/harness.rs 整文件 + agentic/mod.rs:26 + product_assembly.rs:9 对应 re-export；crate-layout.mjs:14、crate-rules.mjs:9；surfaces.md:25；execution/AGENTS.md:18；根 AGENTS.md:27（L5 行）；core/AGENTS.md:78。
**风险标注**：删它需要改活的 product-capabilities 与 core 装配面，工作量与 tool-packs 相当；review 的"frozen 设施"定性低估了接线面。

---

## 9. 与 review 结论不一致 / 需重点标注的发现汇总

1. **【重大】tool-provider-groups 非零调用**：product-capabilities + core(product-full→tool-packs) + materialization.rs 真实消费；CI 边界自测硬编码其 manifest 原文。R-18/SW2-2"直接删"不成立。
2. **【中】harness 非"未接线 frozen"**：core 非 optional 依赖 + product-capabilities 真实使用；test-support 不覆盖。删除需改活代码装配面。
3. **【中】R-18 session 表述歧义**：全空的只有顶层 `src/agentic/session/`（git 未跟踪）；core 内同名目录是 8.6k 行活代码，误读会删错。
4. **【小】insights 名义关联**：`tests/e2e/specs/insights-screenshot.spec.ts` 同名（非代码引用；整个 tests/e2e 是 Tauri 时代化石，连带等待 `window.__TAURI__`、指向缺失的 src/web-ui、依赖无人消费的 webdriver 契约）。
5. **【补充】webdriver 体量**：6,044 rs 行（review 未给数），单 crate 即超 insights；删除可连带清 9+ 条孤儿 workspace 依赖声明。
6. **【补充】孤儿 workspace 依赖**：enigo/screenshots 之外另有 resvg/atspi/leptess/core-foundation/core-graphics/dispatch/objc2-vision 零消费（根 Cargo.toml:177-187 段）。
7. **【流程面】任何 crate 删除都必须同步 `scripts/core-boundaries/`**（crate-layout.mjs / crate-rules.mjs / feature-rules.mjs / self-test.mjs），否则 CI `core-boundaries` job（ci.yml:132-143）红。review 全文未提此连带面。

## 10. 建议的第一批"纯删"安全集（供编排参考，非决策）

- 零风险纯删：insights（§1）、空 `src/agentic/session/`（§3）、webdriver（§4，含 boundary 规则同步）、enigo/screenshots 两行（§5）、cli-internal 死依赖（§6，:14 起 9 行）、plan-compliance-checker（§7）
- 需单独评估/拆任务：tool-provider-groups（§2）、harness（§8）——两者被 product-capabilities/core 活接线，不满足"直接删"前置条件