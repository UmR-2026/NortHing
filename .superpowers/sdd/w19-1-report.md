# W19-1 实施报告 — selftest 外置拆分（lease 兑现 + ceiling 重组）

## 改动摘要

1. **selftest 纯移动外置拆分**：
   - 将原 `scripts/verify-rot-budget.mjs` 中 `runSelftest()` 的 53 项用例实现与合成 git 仓库 helper `createSyntheticGitRepo` 整体纯移动至 `scripts/fixtures/rot-budget/selftest-cases.test.mjs`（1326 行）。
   - 命名固定 `.test.mjs` 后缀并放置于 `scripts/fixtures/rot-budget/` 子目录，享有测试文件豁免与子目录忽略双保险，不进 `file-lines` 扫描面，不占 scripts 顶层计数。该文件属于测试代码而非 fixture 数据，不登记 `registry.json`。
   - `scripts/verify-rot-budget.mjs` 保留瘦 `runSelftest()` 桩：保持 `import.meta.url` 路径推导（`scriptDir`、`repoRoot`、`fixturesDir`），保持起步校验 `attestFixtureRegistry(fixturesDir, repoRoot)` 且失败提前退出，随后将 `{ fixturesDir, repoRoot }` 注入 `runSelftestCases`。
   - 核心导出函数 `verifyRotBudget` 与 `attestFixtureRegistry` 留在主文件，`scripts/verify-rot-budget.test.mjs` 零改动。

2. **主文件行数与 ceiling 重组（only-down）**：
   - 拆分后主文件 `scripts/verify-rot-budget.mjs` 实际行数 $L = 983$ 行（成功降入 $\le 1000$ 行安全线）。
   - 注：行数下降完全来自搬迁，不得且未报告为结构改善（P09）。
   - ceiling 计算按 brief 钉死公式：`ceil((L + max(5, ceil(0.05·L))) / 50) × 50`
     - $L = 983$
     - $\text{headroom} = \max(5, \lceil 0.05 \times 983 \rceil) = \max(5, 50) = 50$
     - $\text{floor} = 983 + 50 = 1033$
     - $\text{ceiling} = \lceil 1033 / 50 \rceil \times 50 = 21 \times 50 = 1050$
   - `scripts/rot-budget.json` 中 `god_file:scripts/verify-rot-budget.mjs` ceiling 由 2300 下调至 1050，note 追记 `"W19-1 selftest 外置拆分重组 2300→1050"`。

3. **lease 槽位处置**：
   - 主文件 $L = 983 \le 1000$，已脱离 god-file 硬边界，W18 承诺兑现。
   - 从 `scripts/exception-leases.json` 移除 `scripts/verify-rot-budget.mjs` 条目，仅保留存量 `scripts/i18n-audit.mjs`。

---

## 验证

### 1. 主运行与机械取证自身条目读数

```bash
node scripts/verify-rot-budget.mjs
```

**输出原文**：
```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=80/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

Exit code: 0

**机械取证自身读数/ceiling**：
```bash
node -e "import('./scripts/verify-rot-budget.mjs').then(async (m) => { const fs = await import('node:fs'); const r = await m.verifyRotBudget({ projectRoot: process.cwd() }); const c = JSON.parse(fs.readFileSync('scripts/rot-budget.json', 'utf8'))['god_file:scripts/verify-rot-budget.mjs'].ceiling; console.log(r.counts['scripts/verify-rot-budget.mjs'] + '/' + c); })"
```

**输出原文**：
```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=80/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
983/1050
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

Exit code: 0

### 2. Selftest 完整运行

```bash
node scripts/verify-rot-budget.mjs --selftest
```

**输出原文**：
```
[PASS] negative fixture: bogus-kind: rejects invalid kind in manifest
[PASS] negative fixture: string-ceiling: rejects string ceiling in manifest
[PASS] negative fixture: path-escape: rejects directory path escaping project root
[PASS] negative fixture: empty-pattern: rejects empty pattern for grep-count
[PASS] negative fixture: unknown-field: rejects unknown field in manifest
[PASS] negative inline: invalid-regex: rejects uncompilable regex pattern
[PASS] negative inline: non-string-note: rejects non-string note
[PASS] negative inline: proto-kind: rejects prototype-inherited kind without throwing
[PASS] positive: workspace manifest: current scripts/rot-budget.json passes validation
[PASS] threshold boundary: 800 vs 801 lines: 800 lines passes, 801 lines triggers god-file violation
[PASS] integration: verifyRotBudget wires validateManifest: verifyRotBudget fails-closed on invalid manifest
[PASS] negative inline: malformed-action-missing-archiveTo: rejects cap-and-archive action missing archiveTo
[PASS] negative inline: malformed-authorization-missing-expires: rejects authorization missing expires field
[PASS] negative base: unauthorized ceiling raise: rejects ceiling increase without authorization in --base mode
[PASS] negative base: expired authorization ceiling raise: rejects ceiling increase with expired authorization in --base mode
[PASS] negative base: ceiling lowering violates headroom floor: rejects ceiling lowering that breaches headroom floor
[PASS] negative base: deleted metric in tip manifest: rejects deletion of metric from manifest in --base mode
[PASS] positive base: live authorization permits ceiling raise: permits ceiling increase with valid unexpired authorization
[PASS] positive date boundary: expires equals today utc is live: expires matching today UTC is accepted as live
[PASS] negative date boundary: expires yesterday is expired: expires matching yesterday UTC is rejected as expired
[PASS] positive base: compliant ceiling lowering satisfies headroom floor: permits ceiling lowering when headroom floor is met
[PASS] positive cap-and-archive: violation includes archiveTo guidance: cap-and-archive breach outputs dedicated guidance containing archiveTo
[PASS] positive zero-headroom: warning emitted when current equals ceiling: emits zero headroom warning and points to exception lease channel
[PASS] negative inline: 1001-line file without lease: rejects file >1000 lines when no exception lease exists
[PASS] negative inline: expired exception lease: rejects file >1000 lines when exception lease has expired
[PASS] negative date boundary: lease revisit_after yesterday is expired: revisit_after matching yesterday UTC is rejected as expired
[PASS] negative inline: banned allow-god-file comment: rejects file containing banned allow-god-file comment
[PASS] negative inline: malformed exception leases file: rejects malformed exception leases file (missing fields and invalid JSON)
[PASS] negative base: dead registration with lowered ceiling: rejects dead registration with lowered ceiling in --base mode
[PASS] positive threshold boundary: exactly 1000 lines: exactly 1000 lines registered in manifest passes without exception lease
[PASS] positive inline: 1001-line file with live lease: permits file >1000 lines when covered by a valid live exception lease
[PASS] positive date boundary: lease revisit_after equals today utc: lease revisit_after matching today UTC is accepted as live
[PASS] positive zero-headroom: warning suppressed by live lease: suppresses zero headroom warning for file-lines entry with live lease
[PASS] negative inline: exempt-list path-escape: rejects exempt-list path escaping project root
[PASS] negative inline: exempt-list empty-paths: rejects empty paths array for exempt-list
[PASS] negative inline: exempt-list non-string-paths: rejects non-string element in exempt-list paths
[PASS] negative inline: exempt-list with-ceiling: rejects ceiling field on exempt-list as unknown field
[PASS] positive inline: exempt-list skips god-file check: file with 801 lines in exempt-list passes god-file limit without violation
[PASS] negative policy: rotScanScope mismatch with actual scan roots: rejects workflow policy with mismatched scan roots
[PASS] negative policy: missing rotScanScope field: rejects workflow policy missing rotScanScope field
[PASS] negative policy: malformed rotScanScope shape: rejects rotScanScope missing fileLinesRoots or having invalid shape
[PASS] negative inline: scripts 1001-line file without lease: rejects scripts .mjs file >1000 lines without exception lease
[PASS] negative inline: scripts 900-line file unregistered: rejects unregistered scripts .js file exceeding 800 lines
[PASS] negative inline: scripts file with banned allow-god-file comment: rejects scripts file containing line-anchored allow-god-file comment
[PASS] positive policy: missing policy file skips attestation: missing workflow policy skips attestation for synthetic roots
[PASS] positive inline: test-named script excluded from scan: scripts matching test or spec naming pattern are excluded from file-lines scan
[PASS] positive inline: installer 801-line file passes when registered: installer 801-line file passes when registered in manifest
[PASS] positive inline: non-anchored allow-god-file literal does not trigger comment ban: non-anchored allow-god-file literal in code or string does not trigger comment ban
[PASS] negative base: scripts retirement detects net increase when base has subdirectories: rejects net increase in scripts when base has subdirectories (discriminates blob-only from line count)
[PASS] negative base: scripts retirement compares actual count rather than ceiling: rejects net increase even when below ceiling (compares actual count rather than ceiling)
[PASS] positive base: scripts retirement permits balanced addition and deletion: permits balanced script addition and deletion (net increase is zero)
[PASS] positive base: scripts retirement permits unchanged scripts count: permits unchanged scripts count under --base mode
[PASS] positive base: scripts retirement ignores subdirectories on both base and tip: ignores subdirectories on both base and tip when count is unchanged
Selftest passed: 53 checks passed (33 negative, 20 positive).
```

Exit code: 0

### 3. `--base 1ac7438` 回溯校验

```bash
node scripts/verify-rot-budget.mjs --base 1ac7438
```

**输出原文**：
```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=80/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

Exit code: 0

### 4. 外部测试套件测试

```bash
node scripts/verify-rot-budget.test.mjs
```

**输出原文**：
```
✔ compliant fixture exits 0 and reports success (104.7879ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (101.5587ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (110.2189ms)
✔ registered god-file exceeding ceiling fails (6.2286ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (6.545ms)
✔ dir-entry-count compliant fixture passes (100.6508ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (102.6436ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (106.1551ms)
✔ tests.rs file is excluded from rot budget measurement (5.966ms)
✔ *_tests directory files are excluded from rot budget measurement (5.8602ms)
✔ actual workspace rot budget passes with current manifest (320.8086ms)
✔ dead god-file registration warns but does not fail verification (96.164ms)
✔ W18-6 F1.1: workflow policy missing rotVerdictRubric fails and reports violation (4.3007ms)
✔ W18-6 F1.2: workflow policy with malformed rotVerdictRubric (missing key) fails (3.6204ms)
✔ W18-6 F1.3: workflow policy rotVerdictRubric value mismatch with pinned literal fails (3.4705ms)
✔ W18-6 F1.4: valid rubric with 0 findings passes and verdict is healthy (3.5541ms)
✔ W18-6 F1.5: 1 warning (bounded) passes with verdict stable (3.4254ms)
✔ W18-6 F1.6: >=3 findings results in verdict rotting (3.4498ms)
✔ W18-6 F1.7: unregistered file exceeding 800 lines results in verdict rotting (unbounded) (5.102ms)
✔ W18-6 F1.8: manifest validation failure early exit does not carry verdict field (2.4437ms)
✔ W18-6 F1.9: green path spawn stdout contains verdict segment (102.5852ms)
✔ W18-6 F2.1: dead registration without deadSince produces warning and passes (3.7172ms)
✔ W18-6 F2.2: dead registration with deadSince 31 days ago fails with violation (3.1306ms)
✔ W18-6 F2.3: dead registration with deadSince exactly 30 days ago produces warning and passes (3.0171ms)
✔ W18-6 F2.4: live file with deadSince ignores deadSince and emits no warning (5.2007ms)
✔ W18-6 F2.5: malformed deadSince is rejected by manifest validation (3.1412ms)
✔ W18-7 F1.1: fixture registry with matching hashes passes (6.3785ms)
✔ W18-7 F1.2: fixture registry with tampered file hash fails (4.8824ms)
✔ W18-7 F1.3: fixture registry pointing to missing file fails (6.3862ms)
✔ W18-7 F1.4: malformed fixture registry fails (3.8003ms)
✔ W18-7 F1.5: fixture registry entry missing sha256 fails (4.4115ms)
✔ W18-7 F1.6: fixture registry missing or empty fixtures array fails (4.1441ms)
✔ W18-7 F1.7: missing fixture registry file skips attestation and passes (2.8845ms)
ℹ tests 33
ℹ suites 0
ℹ pass 33
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1260.0018
```

Exit code: 0

### 5. Task Gate 策略校验

```bash
node scripts/verify-task-gate.mjs validate-policy
```

**输出原文**：
```
Policy validation passed: <REPO_ROOT>/scripts/workflow-policy.json
```

Exit code: 0

### 6. 仓库卫生检查

```bash
pnpm run check:repo-hygiene
```

**输出原文**：
```
> northhing@0.2.10 check:repo-hygiene <REPO_ROOT>
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (4 content files scanned, 3870 filenames checked).
```

Exit code: 0

---

## 等价性证据

1. **Selftest 判定逐项一致**：
   - 拆分前 BASE 测定：53 checks passed（33 negative, 20 positive）
   - 拆分后测定：53 checks passed（33 negative, 20 positive）
   - 53 项用例 ID 及 description 逐字比对 100% 吻合，起步 registry attestation 与失败处理语义保持不变。

2. **主运行读数零漂移**：
   - `5 grep rules`:
     - `unwrap_production`: 483/502（零漂移）
     - `expect_production`: 940/1089（零漂移）
     - `let_underscore`: 370/388（零漂移）
     - `unix_epoch_inline`: 69/69（零漂移）
     - `allow_dead_code`: 104/109（零漂移）
   - `3 dir rules`:
     - `dir_entries:scripts`: 45/48（零漂移；新文件位于子目录并命名 test 后缀，顶层 entries 未增加）
     - `dir_entries:docs/design`: 1/1（零漂移）
     - `dir_entries:.superpowers/sdd`: 80/400（零漂移）
   - `9 god-file rules`:
     - 扫描文件总数：1397（src: 1368, northing-installer/src-tauri: 11, scripts: 18，零漂移）
     - 自身条目读数：由 2296 降至 983，满足新 ceiling 1050
   - `verdict`: `rotting`（完全一致）

3. **搬迁与结构声明（P09）**：
   - `scripts/verify-rot-budget.mjs` 行数从 2296 降至 983，行数下降完全归因于测试用例的物理外置搬迁，不声明为架构或结构改善。

---

DONE
