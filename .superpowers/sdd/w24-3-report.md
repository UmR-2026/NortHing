# W24-3 任务报告 — 分层表单一源校验（crate-layout.mjs → AGENTS.md/CN 表校验）

## 改动摘要

- **S1 (`scripts/core-boundaries/rules/crate-layout.mjs`)**: 追加导出 `layerTableNonCrate`，仅在 `interfaces` 层映射非 crate 的 paths (`['src/apps/*', 'northing-installer', 'tests/e2e']`) 与 entries (`['desktop', 'CLI', 'server', 'installer', 'E2E']`)，其余层无非 crate 路径。既有规则与导出保持不变。
- **S2 (`scripts/core-boundaries/layer-table.mjs`)**: 新建校验模块，导出：
  - `expectedLayerRows()`: 由 `crateLayoutRules` + `crateLayoutLayerNames` + `layerTableNonCrate` 严格推导 7 层期望（paths = crate dirnames ∪ non-crate paths；entries = crate basenames ∪ non-crate entries）。
  - `parseLayerTable(markdown, heading)`: 纯函数，提取 heading 到下一 heading 间的 markdown 表，归一化反引号与 `,`/`、` 分隔符，返回行号及解析后的 Set。
  - `checkLayerTables(root, failures)`: 检验 `AGENTS.md`（heading `## Layered Module Index`）与 `AGENTS-CN.md`（heading `## 分层模块索引`），按 R1（7 行数据行）、R2（# 列 = 1..7）、R3（paths 集合相等）、R4（entries 集合相等）上报违规到既有 failures 收集器。
  - `areSetsEqual(a, b)`: 辅助纯函数。
- **S3 (`scripts/core-boundaries/checker.mjs`)**: 接线调用 `checkLayerTables(ROOT, failures);`，并在 self-test 门控下接入 `runLayerTableSelfTest({ parseLayerTable, expectedLayerRows });`。diff 恰好 4 行（满足 ≤5 行要求）。
- **S4 (`scripts/core-boundaries/self-test.mjs`)**: 追加导出 `runLayerTableSelfTest({ parseLayerTable, expectedLayerRows })`，包含正例（等效 EN 表通过）与两个反例（反例 a：删模块条目检出违规；反例 b：交换第 3/4 行检出违规）。
- **S5 (`AGENTS-CN.md`)**: 使用 edit 工具单行修改 layer 1，删除路径列 `、`src/web-ui`` 与模块列 `、Web UI`，与 SSOT 对齐。
- **S6 / 约束遵守**: 未触碰 `ci.yml`、`AGENTS.md` 表及任何 W24-1 专有文件；新建模块位于 `scripts/core-boundaries/`，未新增顶层 `scripts/` 文件。

## 复用侦察

- **crate 事实源复用**: 期望值由 `scripts/core-boundaries/rules/crate-layout.mjs` 中的既有 `crateLayoutRules` 与 `crateLayoutLayerNames` 直接派生，未在 `layer-table.mjs` 中重复定义任何 crate 名称或目录列表，严格满足 AC1 单一源。
- **错误上报通道复用**: `checkLayerTables` 复用 `checker.mjs` 既有的 `failures.push({ path, line, message })` 格式与绝对路径惯例，由统一的 `toRepoPath` 输出 `path:line: message`。
- **自测门控通道复用**: 分层表自测复用既有的 `process.env.northhing_BOUNDARY_CHECK_SELF_TEST === '1'` 门控机制，与既有 manifest parser self-test 并列执行。

## 验证（命令 + 输出原文）

### 1. 边界检查（两表校验通过）
```bash
node scripts/check-core-boundaries.mjs
```
输出：
```
Core boundary check passed.
```
退出码：0

### 2. 边界检查自测（含分层表自测）
```powershell
$env:northhing_BOUNDARY_CHECK_SELF_TEST='1'; node scripts/check-core-boundaries.mjs
```
输出：
```
Core boundary check self-test passed.
```
退出码：0

### 3. 仓库卫生检查
```bash
node scripts/check-repo-hygiene.mjs
```
输出：
```
Repository hygiene check passed (2 content files scanned, 3922 filenames checked).
```
退出码：0

### 4. 防腐预算检查
```bash
node scripts/verify-rot-budget.mjs --base 5faf69d19264e8d3130a8143c3de4b0b66e5d2fd
```
输出：
```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=120/400], 9 god-file rules checked across 1396 files [src: 1368, northing-installer/src-tauri: 11, scripts: 17]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```
退出码：0

### 5. 红态探针（fail-closed 实证）

#### 探针 5a：临时从 AGENTS.md layer-5 模块列删 `tool-contracts`
执行命令：
```bash
node scripts/check-core-boundaries.mjs
```
输出：
```
Core boundary check failed.
AGENTS.md:27: row 5 (execution): modules mismatch: expected [agent-dispatch, agent-runtime, agent-stream, runtime-services, tool-contracts, tool-execution], found [agent-dispatch, agent-runtime, agent-stream, runtime-services, tool-execution]
```
退出码：1（检出违规并退出）。
还原验证：`git checkout -- AGENTS.md` 执行后，`git status` 确认工作树干净。

#### 探针 5b：临时将 AGENTS-CN.md 第 3/4 行数据行互换
执行命令：
```bash
node scripts/check-core-boundaries.mjs
```
输出：
```
Core boundary check failed.
AGENTS-CN.md:24: row 3: expected index 3, found 4
AGENTS-CN.md:24: row 3 (adapters): path mismatch: expected [src/crates/adapters], found [src/crates/services]
AGENTS-CN.md:24: row 3 (adapters): modules mismatch: expected [ai-adapters], found [debug-log, services-core, services-integrations, terminal]
AGENTS-CN.md:25: row 4: expected index 4, found 3
AGENTS-CN.md:25: row 4 (services): path mismatch: expected [src/crates/services], found [src/crates/adapters]
AGENTS-CN.md:25: row 4 (services): modules mismatch: expected [debug-log, services-core, services-integrations, terminal], found [ai-adapters]
```
退出码：1（检出违规并退出）。
还原验证：`git checkout -- AGENTS-CN.md` 执行后，`git status` 确认工作树干净。

### 6. Task-gate 校验
根据 brief §6 step 6，同波 W24-1 并行在 `5faf69d` 上提交了 `5bc0038`。

- 在 `--base 5faf69d19264e8d3130a8143c3de4b0b66e5d2fd` 下运行：
```bash
node scripts/verify-task-gate.mjs verify-attempt --base 5faf69d19264e8d3130a8143c3de4b0b66e5d2fd --tip 6503e72 --allowlist .superpowers/sdd/w24-3-allowlist.txt
```
输出：
```
Warnings:
  - Unfulfilled allowlist entry (not modified): .superpowers/sdd/w24-3-report.md
Attempt verification failed:
  - Out-of-bounds file modification: .github/workflows/ci.yml
  - Out-of-bounds file modification: .superpowers/sdd/w24-1-allowlist.txt
  - Out-of-bounds file modification: .superpowers/sdd/w24-1-brief.md
  - Out-of-bounds file modification: package.json
  - Out-of-bounds file modification: pnpm-lock.yaml
  - Out-of-bounds file modification: scripts/check-github-config.mjs
  - Out-of-bounds file modification: scripts/desktop-tauri-build.mjs
  - Out-of-bounds file modification: scripts/gate-registry.json
```
精确触发 brief §6 step 6 预告的条款：「若 diff 窗口内出现 W24-1 文件（scripts/gate-registry.json、ci.yml、package.json 等），停手报编排者复跑，不得私扩 allowlist」。未私自扩大 allowlist。

- 对本任务提交 `6503e72` 相对其直接 parent `5bc0038` 进行孤立验证：
```bash
node scripts/verify-task-gate.mjs verify-attempt --base 5bc0038 --tip 6503e72 --allowlist .superpowers/sdd/w24-3-allowlist.txt
```
输出：
```
Warnings:
  - Unfulfilled allowlist entry (not modified): .superpowers/sdd/w24-3-report.md
Attempt verification passed: all modified files are within allowlist.
```
确认本单 6503e72 改动的全部文件严格在 `w24-3-allowlist.txt` 范围内。

## 疑虑

- 仅存在跨任务接口的并发现象：W24-1 的 commit `5bc0038` 位于 `5faf69d` 与 W24-3 之间。由于本单严格遵守禁区规则（未触碰 `package.json`、`gate-registry.json`、`ci.yml` 等），文件集完全不相交。按 brief §6 step 6 规定，如实上报 diff 窗口内包含 W24-1 产物的事实，由编排者做波级基准对齐。

## 状态

DONE
