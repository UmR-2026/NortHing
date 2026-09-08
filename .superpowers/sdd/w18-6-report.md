# W18-6 Implementation Report — verdict rubric SSOT + dead registration 限期 + P06 文档同步

## 改动摘要

- `scripts/workflow-policy.json`:
  - 顶层增加 `rotVerdictRubric` 字段，定义 `healthy` ("0 findings")、`stable` ("1-2 bounded findings")、`rotting` (">=3 findings OR any unbounded")。
  - `metaRatchetPaths` 追加 `"scripts/rot-budget.json"`。
- `scripts/verify-rot-budget.mjs`:
  - `FIELD_WHITELIST['file-lines']` 增加可选字段 `'deadSince'`。
  - `validateManifest` 增加 `deadSince` 字段校验，非 `YYYY-MM-DD` 合法日期即 fail-closed 报错。
  - 增加 `EXPECTED_ROT_VERDICT_RUBRIC` 常量及 `attestVerdictRubric` 函数，严格校验 policy 中的 rubric 是否存在、是否为 3 键对象、键值是否逐字匹配。
  - `checkFileLinesSurface` 接收 `unboundedState`，在未登记且超 800 行文件分支标记 `unbounded = true`。
  - 预扫描死登记分支：若无 `deadSince`，输出 `warn: <key> registered but file does not exist — dead registration, add deadSince (YYYY-MM-DD) or remove the entry`；若有 `deadSince`，按 `(todayUtc - deadSince) 毫秒差 / 86400000 > 30` 判定，超 30 天报 violation，30 天内报 warning；存活文件忽略 `deadSince`。
  - `verifyRotBudget` 在完整扫描路径返回 `verdict: { class, findings, unbounded }`，并在绿路径与红路径摘要行末尾追加 ` — verdict: <class> (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).` 分段；早退路径不带 `verdict`。
  - 主文件代码行数净增 74 行（2204 -> 2278，低于 ≤90 行体量预算与 ceiling 2300 硬约束）。
- `scripts/verify-rot-budget.test.mjs`:
  - 更新死登记既有用例的两处断言为新文案。
  - 追加 14 个新用例（F1.1 ~ F1.9 共 9 个 + F2.1 ~ F2.5 共 5 个），覆盖 rubric 缺失、非法、值不符、healthy、stable、rotting、unbounded、早退无 verdict、spawn 摘要输出、死登记无日期、31 天违规、恰好 30 天宽限、存活忽略、格式非法拒绝等全部场景。
- `AGENTS.md` / `AGENTS-CN.md`:
  - 家规 3 同步废除 `allow-god-file` 头注释通道，改为 `scripts/exception-leases.json` 登记 active lease（含 `revisit_after`），并声明闸口径为 checker `countLines`，排版/换行产生的行数下降不得报告为结构改善（P09）。

---

## 验证

### 旧代码行为确认（GC4 先负向后实现）

在实现前，将更新文案断言及 14 个新用例写入 `scripts/verify-rot-budget.test.mjs`，对旧代码运行 `node scripts/verify-rot-budget.test.mjs`。
旧代码行为：未支持 `deadSince`（manifest 报 unknown field）、未校验 `rotVerdictRubric`（fail-open）、无 `verdict` 字段与摘要分段。测试结果如期产生 13 项失败，exit code 1：

```
✔ compliant fixture exits 0 and reports success (143.873ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (142.1133ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (137.244ms)
✔ registered god-file exceeding ceiling fails (9.1679ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (10.0385ms)
✔ dir-entry-count compliant fixture passes (136.2281ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (125.999ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (123.2268ms)
✔ tests.rs file is excluded from rot budget measurement (8.7501ms)
✔ *_tests directory files are excluded from rot budget measurement (8.5336ms)
✔ actual workspace rot budget passes with current manifest (408.4398ms)
✖ dead god-file registration warns but does not fail verification (8.4682ms)
✖ W18-6 F1.1: workflow policy missing rotVerdictRubric fails and reports violation (5.5852ms)
✖ W18-6 F1.2: workflow policy with malformed rotVerdictRubric (missing key) fails (5.677ms)
✖ W18-6 F1.3: workflow policy rotVerdictRubric value mismatch with pinned literal fails (5.7704ms)
✖ W18-6 F1.4: valid rubric with 0 findings passes and verdict is healthy (10.0132ms)
✖ W18-6 F1.5: 1 warning (bounded) passes with verdict stable (6.169ms)
✖ W18-6 F1.6: >=3 findings results in verdict rotting (5.1765ms)
✖ W18-6 F1.7: unregistered file exceeding 800 lines results in verdict rotting (unbounded) (8.4894ms)
✔ W18-6 F1.8: manifest validation failure early exit does not carry verdict field (4.24ms)
✖ W18-6 F1.9: green path spawn stdout contains verdict segment (125.2725ms)
✖ W18-6 F2.1: dead registration without deadSince produces warning and passes (4.176ms)
✖ W18-6 F2.2: dead registration with deadSince 31 days ago fails with violation (5.3457ms)
✖ W18-6 F2.3: dead registration with deadSince exactly 30 days ago produces warning and passes (4.3539ms)
✖ W18-6 F2.4: live file with deadSince ignores deadSince and emits no warning (5.2249ms)
✔ W18-6 F2.5: malformed deadSince is rejected by manifest validation (3.2453ms)
ℹ tests 26
ℹ suites 0
ℹ pass 13
ℹ fail 13
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1473.3843

EXIT_CODE: 1
```

---

### 1. `node scripts/verify-rot-budget.mjs`

```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=76/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```
Exit code: `0`

读数与 baseline 一致（5 grep 同数、dir_entries:scripts=45/48、9 god-file rules、checkedFiles 1397）；因当前工作区存在 5 条零余量 warning，按 rubric 字面规则映射为 `verdict: rotting`。

### 2. `node scripts/verify-rot-budget.mjs --selftest`

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
Exit code: `0`

### 3. `node scripts/verify-rot-budget.mjs --base 6dc9b13`

```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=76/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```
Exit code: `0`

### 4. `node scripts/verify-rot-budget.test.mjs`

```
✔ compliant fixture exits 0 and reports success (131.5195ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (146.9295ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (130.9671ms)
✔ registered god-file exceeding ceiling fails (8.2151ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (11.3207ms)
✔ dir-entry-count compliant fixture passes (134.9768ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (124.4958ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (118.5419ms)
✔ tests.rs file is excluded from rot budget measurement (7.3063ms)
✔ *_tests directory files are excluded from rot budget measurement (9.2314ms)
✔ actual workspace rot budget passes with current manifest (405.6296ms)
✔ dead god-file registration warns but does not fail verification (122.9911ms)
✔ W18-6 F1.1: workflow policy missing rotVerdictRubric fails and reports violation (8.4401ms)
✔ W18-6 F1.2: workflow policy with malformed rotVerdictRubric (missing key) fails (6.8815ms)
✔ W18-6 F1.3: workflow policy rotVerdictRubric value mismatch with pinned literal fails (6.5366ms)
✔ W18-6 F1.4: valid rubric with 0 findings passes and verdict is healthy (5.6861ms)
✔ W18-6 F1.5: 1 warning (bounded) passes with verdict stable (5.0364ms)
✔ W18-6 F1.6: >=3 findings results in verdict rotting (5.5803ms)
✔ W18-6 F1.7: unregistered file exceeding 800 lines results in verdict rotting (unbounded) (7.6297ms)
✔ W18-6 F1.8: manifest validation failure early exit does not carry verdict field (4.2857ms)
✔ W18-6 F1.9: green path spawn stdout contains verdict segment (117.0697ms)
✔ W18-6 F2.1: dead registration without deadSince produces warning and passes (4.4258ms)
✔ W18-6 F2.2: dead registration with deadSince 31 days ago fails with violation (5.1309ms)
✔ W18-6 F2.3: dead registration with deadSince exactly 30 days ago produces warning and passes (4.2604ms)
✔ W18-6 F2.4: live file with deadSince ignores deadSince and emits no warning (7.1661ms)
✔ W18-6 F2.5: malformed deadSince is rejected by manifest validation (4.054ms)
ℹ tests 26
ℹ suites 0
ℹ pass 26
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1555.5572
```
Exit code: `0`

### 5. `node scripts/verify-task-gate.mjs validate-policy`

```
Policy validation passed: <REPO_ROOT>\scripts\workflow-policy.json
```
Exit code: `0`

### 6. `pnpm run check:repo-hygiene`

```
> northhing@0.2.10 check:repo-hygiene <REPO_ROOT>
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (6 content files scanned, 3863 filenames checked).
```
Exit code: `0`

---

## 状态

DONE
