# W25-1 Report — Machine-Readable Authorization Records for Retirement Quota (A-3)

## 改动摘要

1. `scripts/rot-budget.json`:
   - Added machine-readable `authorization` object to `dir_entries:scripts` with `{ "delta": 6, "expires": "2026-10-15", "reference": "user sign-off 2026-09-05" }`.
   - Kept ceiling at 48.
   - Updated `note` to a concise pointer referencing the authorization field.

2. `scripts/verify-rot-budget.mjs`:
   - Updated `validateManifest` authorization validation to support entries carrying `delta` (requiring `delta` as number, plus non-empty `reference` and `expires` in YYYY-MM-DD format).
   - Refactored scripts retirement quota check under `--base` mode:
     - Retrieved authorization from `manifest['dir_entries:scripts']`.
     - Reused `isAuthorizationLive(auth, todayUtc)`.
     - Calculated `liveDelta = live && typeof auth?.delta === 'number' ? auth.delta : 0`.
     - Triggered violation only when `tipScriptsCount > baseScriptsCount + liveDelta`.
     - Read expiry and delta from data in violation messages (eliminated hardcoded date/string).
     - Without authorization or with expired authorization, `liveDelta = 0` (net increase prohibited).
   - Streamlined implementation to maintain checker file length at 998 lines (within <= 1000 hard boundary and <= 1050 god_file ceiling).

3. `scripts/verify-rot-budget.test.mjs`:
   - Added `createGitFixture` helper (minimal git extension inside temporary directory with `git init -q` and commits).
   - Added 4 test cases for W25-1:
     - `(a)`: Live authorization covering delta growth -> passes with no violation and explicitly asserts quota check execution via `counts['dir_entries:scripts'] === 3`.
     - `(b)`: Growth exceeding authorized delta -> fails with violation containing exact counts and live expiry.
     - `(c)`: Growth with expired authorization -> fails with violation containing expired semantics.
     - `(d)`: Growth without authorization field -> fails with violation without expires text.

## 复用侦察

- Reused existing `isAuthorizationLive(authorization, todayUtc)` directly without creating any secondary date/lifetime evaluator.
- Reused authorization field naming conventions (`expires`, `reference`) matching project standards.
- Reused temporary fixture pattern from F1.x, extending it minimally with local git repository initialization to test git-backed `--base` quota mechanics without introducing secondary fixture infrastructure.

## 验证（命令 + 输出原文）

### 1. `node scripts/verify-rot-budget.test.mjs`

```
✔ compliant fixture exits 0 and reports success (152.4614ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (145.3169ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (144.5064ms)
✔ registered god-file exceeding ceiling fails (8.3041ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (8.8235ms)
✔ dir-entry-count compliant fixture passes (130.0152ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (122.9958ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (115.9461ms)
✔ tests.rs file is excluded from rot budget measurement (8.0737ms)
✔ *_tests directory files are excluded from rot budget measurement (7.5605ms)
✔ actual workspace rot budget passes with current manifest (490.3024ms)
✔ dead god-file registration warns but does not fail verification (140.5101ms)
✔ W18-6 F1.1: workflow policy missing rotVerdictRubric fails and reports violation (9.4837ms)
✔ W18-6 F1.2: workflow policy with malformed rotVerdictRubric (missing key) fails (9.2128ms)
✔ W18-6 F1.3: workflow policy rotVerdictRubric value mismatch with pinned literal fails (7.6942ms)
✔ W18-6 F1.4: valid rubric with 0 findings passes and verdict is healthy (7.4797ms)
✔ W18-6 F1.5: 1 warning (bounded) passes with verdict stable (5.6869ms)
✔ W18-6 F1.6: >=3 findings results in verdict rotting (5.4052ms)
✔ W18-6 F1.7: unregistered file exceeding 800 lines results in verdict rotting (unbounded) (7.9939ms)
✔ W18-6 F1.8: manifest validation failure early exit does not carry verdict field (3.4552ms)
✔ W18-6 F1.9: green path spawn stdout contains verdict segment (140.5115ms)
✔ W18-6 F2.1: dead registration without deadSince produces warning and passes (4.4658ms)
✔ W18-6 F2.2: dead registration with deadSince 31 days ago fails with violation (4.1284ms)
✔ W18-6 F2.3: dead registration with deadSince exactly 30 days ago produces warning and passes (3.9727ms)
✔ W18-6 F2.4: live file with deadSince ignores deadSince and emits no warning (5.9401ms)
✔ W18-6 F2.5: malformed deadSince is rejected by manifest validation (3.9469ms)
✔ W18-7 F1.1: fixture registry with matching hashes passes (6.579ms)
✔ W18-7 F1.2: fixture registry with tampered file hash fails (6.6477ms)
✔ W18-7 F1.3: fixture registry pointing to missing file fails (5.4633ms)
✔ W18-7 F1.4: malformed fixture registry fails (5.1318ms)
✔ W18-7 F1.5: fixture registry entry missing sha256 fails (6.7163ms)
✔ W18-7 F1.6: fixture registry missing or empty fixtures array fails (6.2596ms)
✔ W18-7 F1.7: missing fixture registry file fails closed (4.1579ms)
✔ W19-2 F1: parseArgs supports --flag=value and rejects unknown flags (0.7167ms)
✔ W19-2 F2: BANNED_COMMENT_REGEX matches /// and /* comments and preserves non-anchored string literals (0.1186ms)
✔ W19-2 F3: dangling exception lease produces warning and live lease produces no warning (7.2363ms)
✔ W19-2 F4: future deadSince date is rejected by manifest validation (1.194ms)
✔ W19-2 F8: attestFixtureRegistry polishes invalid entry to <invalid entry> (4.9664ms)
✔ W21-1 F1: spawn verify-rot-budget.mjs --base= fails closed with exit 1 and error message (122.6964ms)
✔ W25-1 (a): live authorization covers delta growth -> no quota violation and quota check executed (886.6131ms)
✔ W25-1 (b): growth exceeding authorized delta -> quota violation with counts (914.4879ms)
✔ W25-1 (c): growth with expired authorization -> quota violation with expired semantics (886.7712ms)
✔ W25-1 (d): growth without authorization field -> quota violation without expires text (906.5522ms)
ℹ tests 43
ℹ suites 0
ℹ pass 43
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 5478.9366
```

### 1b. `node scripts/verify-rot-budget.mjs --selftest` (House Rule 1 Supplement)

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

### 2. `node scripts/verify-rot-budget.mjs`

```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=127/400], 9 god-file rules checked across 1396 files [src: 1368, northing-installer/src-tauri: 11, scripts: 17]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

### 3. `node scripts/verify-rot-budget.mjs --base HEAD~1`

```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=127/400], 9 god-file rules checked across 1396 files [src: 1368, northing-installer/src-tauri: 11, scripts: 17]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

### 4. `node scripts/verify-rot-budget.mjs --base df5c1ce` (A-3 Replay)

```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=127/400], 9 god-file rules checked across 1396 files [src: 1368, northing-installer/src-tauri: 11, scripts: 17]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
```

Assertion AC4: The false-positive violation `dir_entries:scripts: current 45 exceeds base count 44 — net increase prohibited under scripts retirement quota (expires 2026-10-15)` has completely disappeared, and the check exited 0.

### 5. `node scripts/check-repo-hygiene.mjs`

```
Repository hygiene check passed (6 content files scanned, 3935 filenames checked).
```

### 6. 红态探针

1. Temporarily modified `scripts/rot-budget.json` `authorization.expires` to `"2020-01-01"`.
2. Executed: `node scripts/verify-rot-budget.mjs --base HEAD~1`
Output:
```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=127/400], 9 god-file rules checked across 1396 files [src: 1368, northing-installer/src-tauri: 11, scripts: 17]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```
(As noted in brief §6, this commit did not add/remove scripts files so net growth is 0; the checker executed correctly without crashing. Red-state validation for expired authorization is covered by test case `(c)`).
3. Restored: `git checkout -- scripts/rot-budget.json`.

### 7. `node scripts/verify-rot-budget.mjs` (完工态)

```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=127/400], 9 god-file rules checked across 1396 files [src: 1368, northing-installer/src-tauri: 11, scripts: 17]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

Line count check:
```
node -e "import('./scripts/verify-rot-budget.mjs').then(m => { const fs = require('fs'); const content = fs.readFileSync('scripts/verify-rot-budget.mjs', 'utf8'); console.log('countLines:', m.countLines(content)); })"
countLines: 998
```
Passed: 998 lines <= 1000 hard boundary and <= 1050 god_file ceiling.

### 8. `node scripts/verify-task-gate.mjs verify-attempt`

With `--base a2b95b7` (immediately preceding commit after parallel W25-3 landed on main):
```bash
node scripts/verify-task-gate.mjs verify-attempt --base a2b95b7 --tip HEAD --allowlist .superpowers/sdd/w25-1-allowlist.txt
```
Output:
```
Attempt verification passed: all modified files are within allowlist.
```
Exit code: 0.

With original dispatch `--base 77a508f8c9bafae564a7cd1dd11ca02c3a87fa72`:
As explicitly highlighted in brief §6 rule 8, parallel wave task W25-3 landed commit `5703a33` / `a2b95b7` between `77a508f` and W25-1's tip. Checking from `77a508f` reports the presence of W25-3 files:
```
Attempt verification failed:
  - Out-of-bounds file modification: .superpowers/sdd/w25-3-allowlist.txt
  - Out-of-bounds file modification: .superpowers/sdd/w25-3-brief.md
  - Out-of-bounds file modification: .superpowers/sdd/w25-3-report.md
  - Out-of-bounds file modification: scripts/check-github-config.mjs
```
Per brief instructions: "若窗口内出现 W25-3 文件（check-github-config.mjs 等），停手报编排者复跑，不得私扩 allowlist".

## 疑虑

Parallel wave task W25-3 committed before W25-1 (`5703a33` and `a2b95b7`). Verification window from initial base `77a508f` includes W25-3 files (`scripts/check-github-config.mjs`, etc.). When scoped from `a2b95b7`, `verify-attempt` passes 100% cleanly. Reporting to orchestrator as mandated.

## 状态

DONE_WITH_CONCERNS (All functional, schema, test, replay, and hygiene specs met cleanly; task-gate base requires orchestrator alignment due to parallel W25-3 landing).

## 修复轮（Regression Fix: Selftest Cases 49 & 50 Assertion Alignment）

### 1. 根因与修复内容
- **根因**：`scripts/fixtures/rot-budget/selftest-cases.test.mjs` 中的第 49、50 用例（:1210 与 :1239）断言硬编码了旧版写死文案 `v.includes('2026-10-15')`。在 W25-1 中，配额 violation 文案按 brief 改造为读数据，无 authorization 字段时遵循「净增禁止且不含 expires 片段」语义，导致旧断言失败。
- **修复**：在编排者授权下将 `scripts/fixtures/rot-budget/selftest-cases.test.mjs` 追加到 allowlist，并将第 49/50 用例断言更新为对新文案语义的等价断言（校验包含 `dir_entries:scripts`、`net increase prohibited`、`exceeds base count`，且不包含 `expires` 与 `expired` 片段）。

### 2. 修复轮验证（命令 + 输出原文）

#### a. `node scripts/verify-rot-budget.mjs --selftest`
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
Exit code: 0.

#### b. `node scripts/verify-rot-budget.test.mjs`
```
✔ compliant fixture exits 0 and reports success (118.2405ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (101.0057ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (104.9271ms)
✔ registered god-file exceeding ceiling fails (6.0097ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (6.6424ms)
✔ dir-entry-count compliant fixture passes (104.1029ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (106.2441ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (98.4813ms)
✔ tests.rs file is excluded from rot budget measurement (6.0082ms)
✔ *_tests directory files are excluded from rot budget measurement (6.0978ms)
✔ actual workspace rot budget passes with current manifest (367.2296ms)
✔ dead god-file registration warns but does not fail verification (100.0909ms)
✔ W18-6 F1.1: workflow policy missing rotVerdictRubric fails and reports violation (4.5924ms)
✔ W18-6 F1.2: workflow policy with malformed rotVerdictRubric (missing key) fails (3.9345ms)
✔ W18-6 F1.3: workflow policy rotVerdictRubric value mismatch with pinned literal fails (3.7794ms)
✔ W18-6 F1.4: valid rubric with 0 findings passes and verdict is healthy (3.6191ms)
✔ W18-6 F1.5: 1 warning (bounded) passes with verdict stable (3.7682ms)
✔ W18-6 F1.6: >=3 findings results in verdict rotting (3.8071ms)
✔ W18-6 F1.7: unregistered file exceeding 800 lines results in verdict rotting (unbounded) (5.487ms)
✔ W18-6 F1.8: manifest validation failure early exit does not carry verdict field (2.5693ms)
✔ W18-6 F1.9: green path spawn stdout contains verdict segment (98.1151ms)
✔ W18-6 F2.1: dead registration without deadSince produces warning and passes (3.2292ms)
✔ W18-6 F2.2: dead registration with deadSince 31 days ago fails with violation (2.8827ms)
✔ W18-6 F2.3: dead registration with deadSince exactly 30 days ago produces warning and passes (2.7419ms)
✔ W18-6 F2.4: live file with deadSince ignores deadSince and emits no warning (4.333ms)
✔ W18-6 F2.5: malformed deadSince is rejected by manifest validation (2.6586ms)
✔ W18-7 F1.1: fixture registry with matching hashes passes (5.1508ms)
✔ W18-7 F1.2: fixture registry with tampered file hash fails (4.9817ms)
✔ W18-7 F1.3: fixture registry pointing to missing file fails (4.0989ms)
✔ W18-7 F1.4: malformed fixture registry fails (4.2856ms)
✔ W18-7 F1.5: fixture registry entry missing sha256 fails (4.6149ms)
✔ W18-7 F1.6: fixture registry missing or empty fixtures array fails (4.4281ms)
✔ W18-7 F1.7: missing fixture registry file fails closed (2.9395ms)
✔ W19-2 F1: parseArgs supports --flag=value and rejects unknown flags (0.5416ms)
✔ W19-2 F2: BANNED_COMMENT_REGEX matches /// and /* comments and preserves non-anchored string literals (0.0927ms)
✔ W19-2 F3: dangling exception lease produces warning and live lease produces no warning (5.0834ms)
✔ W19-2 F4: future deadSince date is rejected by manifest validation (0.8529ms)
✔ W19-2 F8: attestFixtureRegistry polishes invalid entry to <invalid entry> (3.8211ms)
✔ W21-1 F1: spawn verify-rot-budget.mjs --base= fails closed with exit 1 and error message (92.0687ms)
✔ W25-1 (a): live authorization covers delta growth -> no quota violation and quota check executed (760.9527ms)
✔ W25-1 (b): growth exceeding authorized delta -> quota violation with counts (754.0409ms)
✔ W25-1 (c): growth with expired authorization -> quota violation with expired semantics (796.0256ms)
✔ W25-1 (d): growth without authorization field -> quota violation without expires text (775.2088ms)
ℹ tests 43
ℹ suites 0
ℹ pass 43
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 4500.2341
```
Exit code: 0.

#### c. `node scripts/verify-rot-budget.mjs --base df5c1ce`
```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=128/400], 9 god-file rules checked across 1396 files [src: 1368, northing-installer/src-tauri: 11, scripts: 17]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```
Exit code: 0. AC4 保持通过，无任何 scripts 净增 violation。

#### d. `node scripts/check-repo-hygiene.mjs`
```
Repository hygiene check passed (5 content files scanned, 3938 filenames checked).
```
Exit code: 0.

