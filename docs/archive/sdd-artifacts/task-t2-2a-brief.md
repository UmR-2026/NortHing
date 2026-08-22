# Task T2-2a Brief: 死代码删除第一批（安全集 ≈11.3k 行）

## Source
- backend-roadmap.md T2-2 行（部分执行：本批只含"双证死代码"，tool-provider-groups / harness 因侦察发现活接线**明确排除**，另候决策）；full-review-2026-08-16.md R-12/R-18/R-22/SW2-2。
- 侦察证据附件（已核实，含全部 file:line 与 grep 证据）：`.superpowers/sdd/task-t2-2a-recon.md`。下文操作清单摘自侦察 §1/§3/§4/§5/§6/§7，行号以当前 main（HEAD `e65d98e`）为准；**执行前必须对每项重跑零引用 grep 复核**（侦察距今若有漂移以实测为准，发现引用 → 该项跳过并在报告标注）。

## 删除清单（6 项）

### D1. insights 模块（4,393 rs 行 + 10 个 prompts/md）
- 删目录 `src/crates/assembly/core/src/agentic/insights/`（33 文件）
- 改 `src/crates/assembly/core/src/agentic/mod.rs`：删 54-55 两行（`// Insights module` / `pub mod insights;`）
- 零引用复核模式：`rg "insights::" src -g "*.rs"`（目录外应 0）；`rg "InsightsService|InsightCollector|InsightsHtml|generate_insights|render_insights|InsightFacet|FacetCache"`（全仓应 0）；`rg -i "insight"` kernel_facade / src/apps / *.slint（应 0；product-domains miniapp builtin 的 ui.js 文案命中属无关）
- 文档：surfaces.md / 根 AGENTS.md 无对应行（侦察已核），无需同步
- 注意：`tests/e2e/specs/insights-screenshot.spec.ts` 同名但非代码引用（Tauri 化石 e2e 栈），**不在本批，不动**

### D2. webdriver crate（6,044 rs 行，72 文件）
- 根 `Cargo.toml:11` 删 member 行 `"src/crates/adapters/webdriver",`
- 删目录 `src/crates/adapters/webdriver/`（含其 AGENTS.md）
- `scripts/core-boundaries/rules/crate-layout.mjs:28` 删 webdriver 行；`crate-rules.mjs:20` 从 `noCoreDependencyCrates` 删 `'webdriver',`
- `docs/status/surfaces.md:46` 删行（`| \`webdriver\` | \`src/crates/adapters/webdriver\` | WebDriver adapter |`）
- `src/crates/adapters/AGENTS.md:15` 删 webdriver 行
- 根 `AGENTS.md:25` L3 行改写：Modules 列去掉 `\`, \`webdriver\``，Owns 列 "AI/WebDriver protocol adapters" 改为 "AI protocol adapters"（保持表格其余列不动）
- 零依赖方复核：`rg "northhing_webdriver|northhing-webdriver" src -g "*.rs"` 应 0；全部 crate Cargo.toml 应 0
- 孤儿 workspace 依赖连带清理（侦察已核仅 webdriver 消费或纯孤儿；执行前 `rg` 全仓 toml 复核）：根 Cargo.toml `block2`(:183)、`objc2`(:184)、`objc2-foundation`(:185)、`objc2-app-kit`(:186)、`objc2-vision`(:187)、`objc2-web-kit`(:188)、`webview2-com`(:190)、`glib`(:193)、`gtk`(:194)、`webkit2gtk`(:195)
- `scripts/dev.cjs:30-35` watch 列表整段指向旧布局（含 :35 引 `src/crates/webdriver`），顺手清理该过期段

### D3. enigo / screenshots + 同段孤儿依赖（根 Cargo.toml [workspace.dependencies]）
- 删 :175 `screenshots = "0.8"`、:176 `enigo = "0.2"`（R-22 所指；Cargo.lock 零命中）
- 同段孤儿连带（侦察已核零消费；执行前复核）：:177 `resvg`、:178 `atspi`、:179 `leptess`、:180 `core-foundation`、:181 `core-graphics`、:182 `dispatch`
- 复核模式：`rg -i "^(enigo|screenshots|resvg|atspi|leptess|core-foundation|core-graphics|dispatch)\s" --glob Cargo.toml` 全仓仅根仓声明命中；`rg "use (enigo|screenshots|resvg|...)"` 零命中

### D4. cli-internal 零使用依赖（`src/crates/support/cli-internal/Cargo.toml`）
- 必删 :14 `northhing-core = { path = "../../assembly/core", default-features = false, features = ["product-full"] }`（拽进整个 product-full 树）
- 同删（grep 证据齐，执行前复核 main.rs 实际 use）：:15 `northhing-events`、:21 `dirs`、:22 `toml`、:26 `thiserror`、:28 `tracing`、:35 `uuid`、:36 `chrono`、:39 `sha2`
- 顺手修 surfaces.md:56 路径错误（R-20 登记 3 周）：`src/crates/cli-internal` → `src/crates/support/cli-internal`
- crate 本体保留（整 crate 删除是另一决策，不在本批）；boundary 规则不触

### D5. plan-compliance-checker（894 rs 行）
- 根 `Cargo.toml:32` 删 member 行
- 删目录 `tools/plan-compliance-checker/`
- `docs/status/surfaces.md:58` 删行
- `scripts/copy_reference.cjs:49-64` 六条对其源文件的"参考拷贝"引用：删除该六行（脚本本身无 caller，属孤儿；不删脚本本体）
- 复核：`.github/workflows` / `package.json` / `scripts/` 无调用（侦察已核）
- 注意：根 Cargo.toml:135 `pulldown-cmark` 仍被 `src/apps/cli/Cargo.toml:43` 使用——**保留勿删**

### D6. 空目录 `src/agentic/session/`（git 未跟踪）
- 纯文件系统删除，无 commit 面；若目录已不存在则跳过
- ⚠️ 绝对路径是 `northing/src/agentic/session/`（顶层 src 下）；**切勿触碰** `src/crates/assembly/core/src/agentic/session/`（8.6k 行活代码）

## Constraints
- 不 commit、不 push（编排者统一收口）；改动留在工作区
- 文档同步硬规则：crate 删除与 surfaces.md / 各 AGENTS.md / boundary 规则文件的同步必须在**同一工作区改动集**里
- 排除项勿碰：`tool-provider-groups`、`harness`、`judge_gate`、`remote_connect`、`mobile-web`、`miniapp`、`relay-*`、`tests/e2e/`
- 勿碰并行 session 资产：`memory/`、`.graph/`、`.opencode/model-capability-notes.md`、`.superpowers/sdd/` 里其它 task-* 文件、前端 session 相关文件
- cargo 命令一律 `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`（默认 GNU 工具链会失败）；命令 timeout 给足（全量 check 可能 15-30 分钟）
- 若某项复核发现实际有引用：**跳过该项**，报告标注，不要强行删

## Verification（贴原始输出）
1. `cargo check --workspace`（MSVC）必须 pass
2. `node scripts/check-core-boundaries.mjs` 必须 pass（boundary 规则已同步的证据）
3. `cargo metadata --no-deps --format-version 1` 无解析错误（workspace member 摘除干净的证据）
4. 每项删除前的零引用 grep 复核输出（命令 + 命中数）
5. 行数统计：删除前后 `git diff --stat` 摘要

## Report
写 `.superpowers/sdd/task-t2-2a-report.md`，首行 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。含：逐项执行状态（删了/跳过+原因）、diff 摘要、验证原始输出、行数对账（预期 ≈11.3k rs 行：insights 4,393 + webdriver 6,044 + plan-compliance-checker 894 + toml/脚本行）。
