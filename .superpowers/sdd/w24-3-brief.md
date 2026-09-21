# W24-3 Brief — 分层表单一源校验（crate-layout.mjs → AGENTS.md/CN 表校验，meta-ratchet 双 judge）

## 1. 来源与验收标准（逐字）

Plan `.superpowers/sdd/plan-2026-09-10-w23-w26-closure-and-decoupling.md` §1：
> "W24-3 分层表单一源生成（crate-layout.mjs → AGENTS.md 表校验/生成）[双 judge；依赖 W23-1 表已修正]"

防腐建议 3①（用户 2026-09-09 拍板采纳为 D3，`anti-rot-recommendations-2026-09-09.md`）：
> "把知识做成被检查的工件：① 分层表从生成物变成源码（crate-layout.mjs 生成/校验 AGENTS.md 表格）"

依赖状态：W23-1 已修正 AGENTS.md/CN 分层表（progress.md 台账 W23-1 行）——前置满足。

范围解读（显式声明）：plan/D3① 字面为「AGENTS.md 表」，本单扩为 **EN+CN 双表**——依据 plan §1 W23-1 行「AGENTS.md/CN 分层表」先例与「W23-1 表已修正」依赖指向两表；CN layer-1 的 web-ui 残留系 W23-1 漏网项，不删则新校验一上线即红，本单一并收口（S5）为校验绿的前置。

### 验收标准（机械可核对）

- AC1: `scripts/core-boundaries/rules/crate-layout.mjs` 仍为唯一事实源：分层表期望值由其 `crateLayoutRules` + `crateLayoutLayerNames` + 新增的非 crate 补充数据推导，不在第二处复制 crate 清单。
- AC2: 新文件 `scripts/core-boundaries/layer-table.mjs` 提供分层表校验：对 `AGENTS.md` 与 `AGENTS-CN.md` 的「Layered Module Index / 分层模块索引」表做结构校验（S2 规则全列）。
- AC3: `scripts/core-boundaries/checker.mjs` 接线调用（diff ≤5 行），违规走既有 `failures[]` → `path:line` → exit 1 通道。
- AC4: `scripts/core-boundaries/self-test.mjs` 增分层表自测（正例 + ≥2 反例），挂进既有 `northhing_BOUNDARY_CHECK_SELF_TEST=1` 门控块。
- AC5: `AGENTS-CN.md` layer-1 行修正：删 `src/web-ui`（路径列）与 `Web UI`（模块列），与 SSOT 对齐。
- AC6: §6 验证命令全绿，红态探针 fail-closed 实证，输出原文进 report。

## 2. 编排者预检结论（逐项钉死，直接采信）

BASE = `5faf69d19264e8d3130a8143c3de4b0b66e5d2fd`。

| 事实 | 证据 |
|---|---|
| `node scripts/check-core-boundaries.mjs` / boundary self-test（env=1）/ hygiene / `verify-rot-budget.mjs --base HEAD~1` 在 BASE 全绿 | 编排者 BASE 实跑 exit 0（hygiene 绿基线 = 派发前工作树复测，编排者已复跑 exit 0） |
| EN 表（AGENTS.md `## Layered Module Index`）7 行内容与 crate-layout.mjs 推导值一致（见下「推导对照」） | 编排者逐行比对 |
| CN 表（AGENTS-CN.md `## 分层模块索引`）仅 layer-1 行发散：路径列多 `src/web-ui`、模块列多 `Web UI`；row 2-7 与 EN 一致 | 同上 |
| v0.1.0 权威口径：骨干不变量「only Dioxus consult-room desktop + northing-installer are shipping surfaces」；AGENTS.md 全文对 web-ui 一律标 `[missing: src/web-ui]`——故以 EN 表为准、CN 为待修 | AGENTS.md backbone invariants |
| crateName↔目录名不一致两处：`agent-tools` 住 `tool-contracts`、`tool-runtime` 住 `tool-execution`——表列的是目录名，推导时取 path 末段 | crate-layout.mjs:14,16 |
| 层序约定：`crateLayoutLayerNames` 数组序 = 表行序（interfaces=1 … support=7） | crate-layout.mjs:35-43 vs 两表 |
| checker.mjs 主线从 :967 起（`checkCrateLayoutRules(); checkCrateSurfaceRegistration();` …），failures 聚合、:1021-1027 统一打印+exit 1 | 源码 |
| self-test 门控：checker.mjs:935 `process.env.northhing_BOUNDARY_CHECK_SELF_TEST === '1'` 块内调 `runManifestParserSelfTest({...})` | 源码 |
| `scripts/core-boundaries/` 子目录文件**不计** `dir_entries:scripts` 配额（只数顶层文件），也**不在** god-file 扫描的 18 个 scripts 文件内——本单零配额成本 | rot 输出 "scripts: 18" + verify-rot-budget.mjs:830 口径 |
| CI 接线免费：core-boundaries job 已在 CI（ci.yml:125-136）；self-test 的 CI 接线是 W24-1 领地（本单不碰 ci.yml） | ci.yml |
| AGENTS-CN.md 为 UTF-8；只许用 edit 工具做单点修改（禁 PowerShell Set-Content——GBK 双重编码事故史） | 编排者纪律 |

推导对照（SSOT 推导值 = 各层 path 末段 crate 名 + 非 crate 补充）：
- layer 1 interfaces：paths = [`src/apps/*`, `northing-installer`, `tests/e2e`, `src/crates/interfaces`]；entries = [desktop, CLI, server, installer, E2E]（非 crate 补充）+ [acp]（crate 推导）
- layer 2 assembly：paths = [`src/crates/assembly`]；entries = [core, product-capabilities]
- layer 3 adapters：paths = [`src/crates/adapters`]；entries = [ai-adapters]
- layer 4 services：paths = [`src/crates/services`]；entries = [services-core, services-integrations, terminal, debug-log]
- layer 5 execution：paths = [`src/crates/execution`]；entries = [agent-dispatch, agent-runtime, agent-stream, tool-contracts, runtime-services, tool-execution]
- layer 6 contracts：paths = [`src/crates/contracts`]；entries = [core-types, events, runtime-ports, product-domains, kernel-api, disposable]
- layer 7 support：paths = [`src/crates/support`]；entries = [test-support, cli-internal]

## 3. 复用侦察（强制）

- crate 清单推导必须复用既有 `crateLayoutRules`/`crateLayoutLayerNames`（crate-layout.mjs），禁止在 layer-table.mjs 里复制第二份 crate 名单。
- failures 上报复用 checker.mjs 既有 `failures.push({path, line, message})` 形状与 `toRepoPath`。
- report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

- S1: crate-layout.mjs 追加导出 `layerTableNonCrate`：形如 `{ interfaces: { paths: ['src/apps/*', 'northing-installer', 'tests/e2e'], entries: ['desktop', 'CLI', 'server', 'installer', 'E2E'] } }` 的补充映射（仅 layer 1 有非 crate 内容；其余层可不出现）。不动既有 export 的形状与内容。
- S2: 新建 `scripts/core-boundaries/layer-table.mjs`，导出：
  - `expectedLayerRows()`：由 crateLayoutRules + crateLayoutLayerNames + layerTableNonCrate 推导 7 层期望（index、paths 集合、entries 集合）。推导公式钉死：**层 paths = { 该层各 crate path 去末段（dirname）} ∪ (layerTableNonCrate[layer]?.paths ?? ∅)；层 entries = { 该层各 crate path 末段 } ∪ (layerTableNonCrate[layer]?.entries ?? ∅)**。`layerTableNonCrate` 不得包含可由 crate path 推导出的路径（`src/crates/<layer>` 形态）——违者即违反 AC1 单一源。
  - `parseLayerTable(markdown, heading)`：纯函数，抽出指定 heading 到下一 heading 间的 markdown 表数据行，返回每行 `#` / path 列集合 / 模块列集合。归一化：去反引号、按 `,` 或 `、` 切分、trim。
  - `checkLayerTables(root, failures)`：对 AGENTS.md（heading `## Layered Module Index`）与 AGENTS-CN.md（heading `## 分层模块索引`）执行校验，违规 push 进 failures——**push 绝对路径（`join(root, 'AGENTS.md')`），与既有惯例一致（checker.mjs:50-51），经 `toRepoPath` 输出为仓库相对路径**；`line` 用表行实际行号。校验规则：
    - R1: 表存在且恰好 7 个数据行。
    - R2: 行 i 的 `#` 列 == i（1..7）。
    - R3: 行 i 的 paths 集合 == expectedLayerRows()[i-1].paths（集合相等，不计序）。
    - R4: 行 i 的模块集合 == expectedLayerRows()[i-1].entries（集合相等）。
    -  prose 列（层名 / Owns / Layer doc）**不校验**——本单只做结构列（显式声明的范围裁剪）。
- S3: checker.mjs 接线：import + 在 `checkCrateSurfaceRegistration();` 后插一行 `checkLayerTables(ROOT, failures);`（diff ≤5 行；self-test 门控块内另加一行调用 runLayerTableSelfTest，见 S4）。
- S4: self-test.mjs 增导出 `runLayerTableSelfTest({ parseLayerTable, expectedLayerRows })`：正例 = 当前 EN 表内容（可内联合成等效 markdown）通过；反例 ≥2：(a) 删一个模块条目 → 检出违规；(b) 交换两行 → 检出违规。挂进 checker.mjs:935 既有 env 门控块（+1 行调用 + import 参数透传）。
- S5: AGENTS-CN.md layer-1 行：路径列删 `、`src/web-ui``、模块列删 `、Web UI`（单行 edit，其余不动）。
- S6: 禁区：不改 `AGENTS.md` 表（已正确——若发现 EN 表与 crate-layout 现状不符，报 BLOCKED 不自行改）；不改 `.github/workflows/ci.yml`（W24-1 领地）；不改 `check-core-boundaries.mjs` shim。

## 5. Global Constraints（逐字遵守）

- 纯校验，不建「生成」通道（生成留将来；反建议精神：最小机械检查先接线）。
- 不新增顶层 `scripts/` 文件（新模块进 `scripts/core-boundaries/`）。
- 输出/日志英文，无 emoji。
- 禁整树 git 操作；只点名 add/commit。
- 测试必须真实执行，report 贴输出原文。
- AGENTS-CN.md 只用 edit 工具改，禁 Set-Content/重编码。

## 6. 验证（命令 + 输出原文进 report）

1. `node scripts/check-core-boundaries.mjs` → exit 0（两表校验通过）
2. `$env:northhing_BOUNDARY_CHECK_SELF_TEST='1'; node scripts/check-core-boundaries.mjs` → exit 0（含新自测）
3. `node scripts/check-repo-hygiene.mjs` → exit 0
4. `node scripts/verify-rot-budget.mjs --base 5faf69d19264e8d3130a8143c3de4b0b66e5d2fd` → exit 0
5. 红态探针（fail-closed 实证；**在本单 commit 之后执行**，保证 `git checkout --` 可还原）：
   a. 临时从 AGENTS.md layer-5 行模块列删 `tool-contracts` → `node scripts/check-core-boundaries.mjs` exit 1 → `git checkout -- AGENTS.md` 还原；
   b. 临时把 AGENTS-CN.md 第 3/4 行数据行互换 → exit 1 → `git checkout -- AGENTS-CN.md` 还原。
   输出原文进 report（注意：探针 b 后必须先还原再继续）。
6. `node scripts/verify-task-gate.mjs verify-attempt --base 5faf69d19264e8d3130a8143c3de4b0b66e5d2fd --tip <本单最后一个 commit sha> --allowlist .superpowers/sdd/w24-3-allowlist.txt` → exit 0（先自建 allowlist，含其自身；commit 后用真实 tip sha 替换占位。**若 diff 窗口内出现 W24-1 文件（scripts/gate-registry.json、ci.yml、package.json 等），停手报编排者复跑，不得私扩 allowlist**）

## 7. 报告

路径 `.superpowers/sdd/w24-3-report.md`。章节：改动摘要 / 复用侦察 / 验证（命令+输出原文）/ 疑虑 / 状态（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）。

## 8. 派发元信息

- BASE: `5faf69d19264e8d3130a8143c3de4b0b66e5d2fd`
- 允许文件集（diff 越出即 judge Critical）：
  - `scripts/core-boundaries/rules/crate-layout.mjs`
  - `scripts/core-boundaries/layer-table.mjs`（新建）
  - `scripts/core-boundaries/checker.mjs`
  - `scripts/core-boundaries/self-test.mjs`
  - `AGENTS-CN.md`
  - `.superpowers/sdd/w24-3-brief.md` / `w24-3-report.md` / `w24-3-allowlist.txt`
- 禁区：§4-S6 所列 + 其它未列出文件。
- commit 规则：点名 git add；message 前缀 `feat(boundaries): W24-3` 或 `docs: W24-3`，body 注「用户 2026-09-09 拍板 D3① 签名」；允许多 commit。
- 并行声明：W24-1 同波并行，其文件集（scripts/gate-registry.json、check-github-config.mjs、ci.yml、package.json、pnpm-lock.yaml、desktop-tauri-build.mjs 删除）与本单不相交；不得触碰。cargo 本单不需要运行。
- 双 judge 依据：plan §2 指定（W24-3 为 meta-ratchet 单，边界检查基础设施视同信任根）；本单允许文件集与 workflow-policy.json metaRatchetPaths 零交集，非机械触发。
