# W25-3 任务报告 — check-github-config 加严批（W24 终审 defer 批收口，meta-ratchet 双 judge）

## 改动摘要

- **AC1 / S1 (schema 校验严格化)**: 在 `scripts/check-github-config.mjs` 增加每条 gate 的键集合严格相等校验（`EXPECTED_GATE_KEYS = new Set(['name', 'entry', 'enforcedAt', 'blocking'])`）。若存在多余字段，上报违规并明确列出未知键名；若缺少必需字段，上报缺失键违规；保留既有字段类型校验。
- **AC2 / S2 (workflowJobs 键含文件名与 CI 引用重名检测)**:
  - `workflowJobs` 键改为 `${fileName}:${jobId}`（如 `ci.yml:meta-gates`），避免跨 workflow 同名 job 在 Map 中被静默覆盖（last-wins）。
  - 维护 `jobsByJobId` 索引与 `referencedCiJobIds` 集合。仅当某 job id 被 `registry.gates` 的任一 `ci:` 目标显式引用，且在多个 workflow 文件中定义时，触发 `Ambiguous job id` violation 并列出涉及文件；未被引用的重名 job id（如 `prepare` / `package` / `upload-release-assets`）不予报错。
  - R1 查找逻辑通过唯一文件名与 jobId 拼接后的新键从 `workflowJobs` 中定位。
- **AC3 / S3 (R1 needle 采用 entry 全文)**:
  - R1 的子串匹配目标由正则截取 `scripts/...` 路径改为 `entry.trim()` 全文（`inline:` 前缀闸维持跳过）。
  - 彻底杜绝了 `boundary-selftest` 闸若丢失 `northhing_BOUNDARY_CHECK_SELF_TEST=1` 环境变量前缀时的静默假绿问题。
  - 现状 11 条闸在新规则下全部校验通过。
- **约束与禁区遵守 (S4)**:
  - 未修改 `scripts/gate-registry.json`、`.github/workflows/ci.yml`。
  - 未触碰 W25-1 / W25-2 领地文件（`verify-rot-budget.mjs`、`rot-budget.json`、`workflow-policy.json` 等）。
  - 未新建顶层 scripts 脚本，所有改动均为就地加严。

## 复用侦察

- **就地修改与流程复用**: 全部三项加严均在 `scripts/check-github-config.mjs` 内部就地修改完成，复用既有的 YAML 读取、解析与 `range` 提取逻辑，未新建文件亦未抽离外部库。
- **通道复用**: 违规检查复用既有的 `errors` 收集队列与 fail-closed 退出机制（统一于末尾集中输出并 `process.exit(1)`）。

## 验证（命令 + 输出原文）

BASE: `77a508f8c9bafae564a7cd1dd11ca02c3a87fa72`

### 1. GitHub 配置与 Gate Registry 检查
```bash
node scripts/check-github-config.mjs
```
输出：
```
GitHub config and gate registry check passed (9 YAML files, 11 gates verified).
```
退出码：0

### 2. 仓库卫生检查
```bash
node scripts/check-repo-hygiene.mjs
```
输出：
```
Repository hygiene check passed (4 content files scanned, 3933 filenames checked).
```
退出码：0

### 3. Rot Budget 基线检查
```bash
node scripts/verify-rot-budget.mjs --base 77a508f8c9bafae564a7cd1dd11ca02c3a87fa72
```
输出：
```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=126/400], 9 god-file rules checked across 1396 files [src: 1368, northing-installer/src-tauri: 11, scripts: 17]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```
退出码：0

### 4. 红态探针（Commit 后执行，每个还原后再做下一个）

#### a. 探针 4a：临时给 gate 加未知字段 `owner`
```bash
node scripts/check-github-config.mjs
```
输出：
```
GitHub config and gate registry check failed:
- Gate at index 0 ("rust-build-check") has unknown keys: owner
```
退出码：1
还原：`git checkout -- scripts/gate-registry.json`，工作树干净。

#### b. 探针 4b：临时在 `.github/workflows/` 放合法 workflow 且含被引用的重名 job id (`meta-gates`)
临时文件 `.github/workflows/temp-probe-ambiguous.yml` 内容：
```yaml
name: Temp Probe Ambiguous
on: push
jobs:
  meta-gates:
    runs-on: ubuntu-latest
    steps:
      - run: echo probe
```
执行命令：
```bash
node scripts/check-github-config.mjs
```
输出：
```
GitHub config and gate registry check failed:
- Ambiguous job id: CI job "meta-gates" referenced by gate registry is defined in multiple workflow files: .github/workflows/ci.yml, .github/workflows/temp-probe-ambiguous.yml
```
退出码：1（输出明确归因为 ambiguous job id violation，非 YAML parse error）
还原：点名删除 `.github/workflows/temp-probe-ambiguous.yml`。

#### c. 探针 4c：临时删除 `ci.yml` 中 `boundary-selftest` run 行的环境变量前缀
```bash
node scripts/check-github-config.mjs
```
输出：
```
GitHub config and gate registry check failed:
- Gate "boundary-selftest": entry substring "northhing_BOUNDARY_CHECK_SELF_TEST=1 node scripts/check-core-boundaries.mjs" not found in CI job "meta-gates" (.github/workflows/ci.yml)
```
退出码：1
还原：`git checkout -- .github/workflows/ci.yml`，工作树干净。

### 5. Task-gate 校验
- 实现提交 `5703a33` 校验：
```bash
node scripts/verify-task-gate.mjs verify-attempt --base 77a508f8c9bafae564a7cd1dd11ca02c3a87fa72 --tip 5703a334b25012254abdc74457e27215ed0f8749 --allowlist .superpowers/sdd/w25-3-allowlist.txt
```
输出：
```
Warnings:
  - Unfulfilled allowlist entry (not modified): .superpowers/sdd/w25-3-report.md
Attempt verification passed: all modified files are within allowlist.
```
退出码：0

- 包含报告的最终提交 `a2b95b7` 校验：
```bash
node scripts/verify-task-gate.mjs verify-attempt --base 77a508f8c9bafae564a7cd1dd11ca02c3a87fa72 --tip a2b95b70e6a5f152c9dd124e259f183df78c26d3 --allowlist .superpowers/sdd/w25-3-allowlist.txt
```
输出：
```
Attempt verification passed: all modified files are within allowlist.
```
退出码：0

## 疑虑

无。所有验收标准均已机械核验通过，红态探针均严格复现预期归因并完全还原，共享工作树无污染。

## 状态

DONE
