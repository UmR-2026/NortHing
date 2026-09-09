# W21-1 Report — rot-budget `--base=` 空值 fail-open 修复

## 改动摘要

1. `scripts/verify-rot-budget.mjs`：主入口 CLI 参数处理将 `if (flags.base)` 改为 `if (flags.base !== undefined)`，使得 `--base=`（等号后为空串，parseArgs 产出 `flags.base === ''`）不再静默跳过基线比对，而是进入校验分支触发 `trim() === ''` 报错 `Error: --base requires a commit SHA or ref` 并 `process.exit(1)`（从 fail-open 修复为 fail-closed）。
2. `scripts/verify-rot-budget.test.mjs`：追加 spawn 级负例 `W21-1 F1: spawn verify-rot-budget.mjs --base= fails closed with exit 1 and error message`，断言 `node scripts/verify-rot-budget.mjs --base=` 进程返回 exit 1 且 stderr 包含 `--base requires a commit SHA or ref`。未改动既有 38 个测试。

## 验证

### 1. BASE 红态复现
命令：
```powershell
node scripts/verify-rot-budget.mjs --base=
```
修复前 BASE（`8ee587e`）实测输出与 exit code：
```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=88/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
exit code: 0
```
结论：修复前静默通过（fail-open），无 base 报错提示。

### 2. 修复后阴性测试
命令：
```powershell
node scripts/verify-rot-budget.mjs --base=
```
修复后实测输出与 exit code：
```
Error: --base requires a commit SHA or ref
exit code: 1
```
结论：正确拦截空值输入，fail-closed 生效。

### 3. 全量测试
命令：
```powershell
node scripts/verify-rot-budget.test.mjs
```
实测输出与 exit code：
```
✔ compliant fixture exits 0 and reports success (122.0203ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (113.3631ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (125.6438ms)
✔ registered god-file exceeding ceiling fails (5.692ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (7.0388ms)
✔ dir-entry-count compliant fixture passes (126.4879ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (113.81ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (108.9812ms)
✔ tests.rs file is excluded from rot budget measurement (5.5432ms)
✔ *_tests directory files are excluded from rot budget measurement (6.1996ms)
✔ actual workspace rot budget passes with current manifest (396.2121ms)
✔ dead god-file registration warns but does not fail verification (118.9173ms)
✔ W18-6 F1.1: workflow policy missing rotVerdictRubric fails and reports violation (3.9636ms)
✔ W18-6 F1.2: workflow policy with malformed rotVerdictRubric (missing key) fails (3.9489ms)
✔ W18-6 F1.3: workflow policy rotVerdictRubric value mismatch with pinned literal fails (3.7648ms)
✔ W18-6 F1.4: valid rubric with 0 findings passes and verdict is healthy (3.6354ms)
✔ W18-6 F1.5: 1 warning (bounded) passes with verdict stable (3.7009ms)
✔ W18-6 F1.6: >=3 findings results in verdict rotting (3.7115ms)
✔ W18-6 F1.7: unregistered file exceeding 800 lines results in verdict rotting (unbounded) (5.2698ms)
✔ W18-6 F1.8: manifest validation failure early exit does not carry verdict field (2.9164ms)
✔ W18-6 F1.9: green path spawn stdout contains verdict segment (101.235ms)
✔ W18-6 F2.1: dead registration without deadSince produces warning and passes (3.3612ms)
✔ W18-6 F2.2: dead registration with deadSince 31 days ago fails with violation (3.1997ms)
✔ W18-6 F2.3: dead registration with deadSince exactly 30 days ago produces warning and passes (2.9417ms)
✔ W18-6 F2.4: live file with deadSince ignores deadSince and emits no warning (4.7871ms)
✔ W18-6 F2.5: malformed deadSince is rejected by manifest validation (2.744ms)
✔ W18-7 F1.1: fixture registry with matching hashes passes (5.3406ms)
✔ W18-7 F1.2: fixture registry with tampered file hash fails (5.9641ms)
✔ W18-7 F1.3: fixture registry pointing to missing file fails (4.5787ms)
✔ W18-7 F1.4: malformed fixture registry fails (4.3689ms)
✔ W18-7 F1.5: fixture registry entry missing sha256 fails (5.1439ms)
✔ W18-7 F1.6: fixture registry missing or empty fixtures array fails (4.5604ms)
✔ W18-7 F1.7: missing fixture registry file skips attestation and passes (2.84ms)
✔ W19-2 F1: parseArgs supports --flag=value and rejects unknown flags (0.6246ms)
✔ W19-2 F2: BANNED_COMMENT_REGEX matches /// and /* comments and preserves non-anchored string literals (0.0991ms)
✔ W19-2 F3: dangling exception lease produces warning and live lease produces no warning (4.9666ms)
✔ W19-2 F4: future deadSince date is rejected by manifest validation (0.8887ms)
✔ W19-2 F8: attestFixtureRegistry polishes invalid entry to <invalid entry> (4.4528ms)
✔ W21-1 F1: spawn verify-rot-budget.mjs --base= fails closed with exit 1 and error message (99.3729ms)
ℹ tests 39
ℹ suites 0
ℹ pass 39
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1553.5307
exit code: 0
```
结论：39 tests pass (38 既有 + 1 新增)，0 fail。

### 4. selftest + 无 flag 日常形态不回归
命令 A：
```powershell
node scripts/verify-rot-budget.mjs --selftest
```
实测输出与 exit code：
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
exit code: 0
```

命令 B：
```powershell
node scripts/verify-rot-budget.mjs
```
实测输出与 exit code：
```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=88/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
exit code: 0
```
结论：53 selftest checks passed；无 flag 默认运行 verdict 输出与 BASE 完全等价，零回归。

### 5. 行为变化矩阵（4 行）

| 形态 | 命令行调用示例 | 修复前 exit code | 修复后 exit code | 行为与判定 |
|---|---|---|---|---|
| 无 flag | `node scripts/verify-rot-budget.mjs` | 0 | 0 | 不变，正常执行全量规则检查并通过 |
| `--base=`（空串） | `node scripts/verify-rot-budget.mjs --base=` | 0 | 1 | **修复点**：修复前静默跳过基线比对（fail-open）；修复后报错 exit 1（fail-closed） |
| 裸 `--base`（布尔 true） | `node scripts/verify-rot-budget.mjs --base` | 1 | 1 | 不变，报错 `--base requires a commit SHA or ref` |
| 正常值 | `node scripts/verify-rot-budget.mjs --base=HEAD` | 0 | 0 | 不变，正常解析 base 并执行比对 |

## 状态

DONE
