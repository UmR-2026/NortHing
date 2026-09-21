# W25-2 Report — Verdict Three-Tier Classification Phase 1 (D4 Signal Hygiene)

## 改动摘要

1. `scripts/workflow-policy.json`:
   - Updated `rotVerdictRubric` to exactly three keys with verbatim wording specified in AC1:
     - `"clean": "0 violations, 0 warnings, 0 advisories"`
     - `"at-limit": "0 violations; warnings/advisories present (bounded, stable)"`
     - `"degrading": ">=1 violation OR any unbounded"`

2. `scripts/verify-rot-budget.mjs`:
   - Updated `EXPECTED_ROT_VERDICT_RUBRIC` and `attestVerdictRubric` to enforce the new 3-key rubric and error message.
   - Migrated zero headroom detection (:862-885) from `warnings` into new `advisories` channel.
   - Added `advisories` array to top-level return object of `verifyRotBudget` (alongside `warnings`).
   - Implemented exact verdict truth table:
     - `clean`: `violations.length === 0 && warnings.length === 0 && advisories.length === 0 && !unboundedState.hasUnbounded`
     - `at-limit`: `violations.length === 0 && !unboundedState.hasUnbounded && (warnings.length + advisories.length) > 0`
     - `degrading`: `violations.length >= 1 || unboundedState.hasUnbounded`
   - Verdict object carries `{ class, violations, warnings, advisories, unbounded }`.
   - Updated console passed/failed lines to print verdict class and channel counts: `(violations: X, warnings: Y, advisories: Z; rubric SSOT: scripts/workflow-policy.json rotVerdictRubric)`.
   - Preserved checker line count at 993 lines (well within <= 1000 hard boundary and <= 1050 god_file ceiling).

3. `scripts/verify-rot-budget.test.mjs`:
   - Updated rubric test cases F1.2 and F1.3 for new 3-key set and mismatch message.
   - Updated verdict assertion test cases:
     - F1.4: 0 findings -> `clean`.
     - F1.5: 1 warning -> `at-limit`.
     - F1.6: >=1 violation -> `degrading`.
     - F1.7: unregistered file >800 lines -> `degrading` (unbounded).
     - F1.9: stdout assertion updated to match new format regex.
   - Added new test case `W25-2: zero headroom produces advisory only and passes with verdict at-limit and exit 0`.
   - Verified that no legacy verdict class names (`rotting`, `healthy`) remain in assertions.

4. `scripts/fixtures/rot-budget/selftest-cases.test.mjs`:
   - Updated case 23 (zero headroom positive test) to assert `res.advisories.some(...)`.
   - Updated case 33 (live lease suppression test) to assert both `!res.warnings.some(...)` and `!res.advisories.some(...)`.

5. `.superpowers/sdd/w25-2-allowlist.txt`:
   - Added with self and in-scope files.

## 复用侦察

- `advisories` channel reuses existing array accumulation, iteration, and console output patterns without introducing a separate output abstraction.
- `attestVerdictRubric` exact key/value validation and discrepancy collection pattern reused directly with updated expected constants.
- Zero-headroom manifest iteration and live exception lease suppression logic reused intact, modifying only the target collection from `warnings.push` to `advisories.push`.
- Verdict consumer check: rg confirmed no third-party consumers of `verdict.findings` exist outside `scripts/verify-rot-budget.mjs` and `scripts/verify-rot-budget.test.mjs`:
  ```
  rg '\bfindings\b' scripts/
  scripts/i18n-audit.mjs (internal variables for literal fallbacks and CJK scan)
  scripts/i18n-contract.test.mjs (test description string)
  ```
  No other tool or test references `verdict.findings`.

## 验证（命令 + 输出原文）

### 1. `node scripts/verify-rot-budget.test.mjs`

```
✔ compliant fixture exits 0 and reports success (115.9064ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (102.4957ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (102.3421ms)
✔ registered god-file exceeding ceiling fails (5.7222ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (6.7711ms)
✔ dir-entry-count compliant fixture passes (101.0786ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (103.3546ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (97.6781ms)
✔ tests.rs file is excluded from rot budget measurement (6.8158ms)
✔ *_tests directory files are excluded from rot budget measurement (6.8915ms)
✔ actual workspace rot budget passes with current manifest (375.7217ms)
✔ dead god-file registration warns but does not fail verification (102.0569ms)
✔ W18-6 F1.1: workflow policy missing rotVerdictRubric fails and reports violation (4.4673ms)
✔ W18-6 F1.2: workflow policy with malformed rotVerdictRubric (missing key) fails (3.7951ms)
✔ W18-6 F1.3: workflow policy rotVerdictRubric value mismatch with pinned literal fails (3.9265ms)
✔ W18-6 F1.4: valid rubric with 0 findings passes and verdict is clean (3.6154ms)
✔ W18-6 F1.5: 1 warning (bounded) passes with verdict at-limit (3.547ms)
✔ W18-6 F1.6: >=1 violation results in verdict degrading (5.2645ms)
✔ W18-6 F1.7: unregistered file exceeding 800 lines results in verdict degrading (unbounded) (5.7292ms)
✔ W18-6 F1.8: manifest validation failure early exit does not carry verdict field (2.6602ms)
✔ W18-6 F1.9: green path spawn stdout contains verdict segment (96.0836ms)
✔ W25-2: zero headroom produces advisory only and passes with verdict at-limit and exit 0 (98.3044ms)
✔ W18-6 F2.1: dead registration without deadSince produces warning and passes (3.3396ms)
✔ W18-6 F2.2: dead registration with deadSince 31 days ago fails with violation (2.7361ms)
✔ W18-6 F2.3: dead registration with deadSince exactly 30 days ago produces warning and passes (2.6341ms)
✔ W18-6 F2.4: live file with deadSince ignores deadSince and emits no warning (3.9943ms)
✔ W18-6 F2.5: malformed deadSince is rejected by manifest validation (2.4009ms)
✔ W18-7 F1.1: fixture registry with matching hashes passes (5.0142ms)
✔ W18-7 F1.2: fixture registry with tampered file hash fails (4.5218ms)
✔ W18-7 F1.3: fixture registry pointing to missing file fails (3.6164ms)
✔ W18-7 F1.4: malformed fixture registry fails (4.2309ms)
✔ W18-7 F1.5: fixture registry entry missing sha256 fails (4.7268ms)
✔ W18-7 F1.6: fixture registry missing or empty fixtures array fails (4.097ms)
✔ W18-7 F1.7: missing fixture registry file fails closed (2.7565ms)
✔ W19-2 F1: parseArgs supports --flag=value and rejects unknown flags (0.5576ms)
✔ W19-2 F2: BANNED_COMMENT_REGEX matches /// and /* comments and preserves non-anchored string literals (0.0999ms)
✔ W19-2 F3: dangling exception lease produces warning and live lease produces no warning (5.0155ms)
✔ W19-2 F4: future deadSince date is rejected by manifest validation (0.9039ms)
✔ W19-2 F8: attestFixtureRegistry polishes invalid entry to <invalid entry> (3.6554ms)
✔ W21-1 F1: spawn verify-rot-budget.mjs --base= fails closed with exit 1 and error message (87.2749ms)
✔ W25-1 (a): live authorization covers delta growth -> no quota violation and quota check executed (772.345ms)
✔ W25-1 (b): growth exceeding authorized delta -> quota violation with counts (744.5279ms)
✔ W25-1 (c): growth with expired authorization -> quota violation with expired semantics (803.8521ms)
✔ W25-1 (d): growth without authorization field -> quota violation without expires text (766.9761ms)
ℹ tests 44
ℹ suites 0
ℹ pass 44
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 4593.5756
```

### 2. `node scripts/verify-rot-budget.mjs`

```
advisory: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=133/400], 9 god-file rules checked across 1396 files [src: 1368, northing-installer/src-tauri: 11, scripts: 17]) — verdict: at-limit (violations: 0, warnings: 0, advisories: 5; rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
advisory: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
advisory: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
advisory: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
advisory: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

### 3. `node scripts/verify-rot-budget.mjs --base e57c6387de88110789ae600d90a4853da05aa90a`

```
advisory: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=133/400], 9 god-file rules checked across 1396 files [src: 1368, northing-installer/src-tauri: 11, scripts: 17]) — verdict: at-limit (violations: 0, warnings: 0, advisories: 5; rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
advisory: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
advisory: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
advisory: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
advisory: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

### 4. `node scripts/verify-rot-budget.mjs --selftest`

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

### 5. `node scripts/check-repo-hygiene.mjs`

```
Repository hygiene check passed (7 content files scanned, 3942 filenames checked).
```

### 6. `node scripts/verify-task-gate.mjs validate-policy`

```
Policy validation passed: <REPO>/scripts/workflow-policy.json
```

### 7. Red-state probe (Attestation Fail-Closed)

Temporarily modified `at-limit` key to `at-limit-probe` in `scripts/workflow-policy.json`:
```
$ node scripts/verify-rot-budget.mjs
advisory: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
advisory: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
advisory: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
advisory: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
advisory: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
workflow-policy.json "rotVerdictRubric.at-limit" must be a string
Rot budget verification failed with 1 violation(s) — verdict: degrading (violations: 1, warnings: 0, advisories: 5; rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
Exit code: 1
```
Restored: `git checkout -- scripts/workflow-policy.json`.

### 8. `node scripts/verify-task-gate.mjs verify-attempt`

With `--base e112942` (immediately preceding commit on main, after parallel W25-4 landed):
```bash
node scripts/verify-task-gate.mjs verify-attempt --base e112942 --tip HEAD --allowlist .superpowers/sdd/w25-2-allowlist.txt
```
Output:
```
Attempt verification passed: all modified files are within allowlist.
```
Exit code: 0.

With original dispatch `--base e57c6387de88110789ae600d90a4853da05aa90a`:
As anticipated in brief §6 item 8, parallel wave task W25-4 landed on main (`ee93158`, `b5b49aa`, `0f5a18b`, `e112942`) while W25-2 was executing. Checking from `e57c638` reports the presence of W25-4 files:
```
Attempt verification failed:
  - Out-of-bounds file modification: .superpowers/sdd/w25-4-allowlist.txt
  - Out-of-bounds file modification: .superpowers/sdd/w25-4-brief.md
  - Out-of-bounds file modification: .superpowers/sdd/w25-4-report.md
  - Out-of-bounds file modification: Cargo.lock
  - Out-of-bounds file modification: src/apps/desktop/Cargo.toml
  - Out-of-bounds file modification: src/apps/desktop/src/ui_dioxus/windows/work.rs
```
Per brief instructions: "窗口内若出现 W25-4 文件（Cargo.toml/Cargo.lock/work.rs），停手报编排者复跑，不得私扩 allowlist".

### 9. Checker Line Count Check (AC6)

```bash
node -e "import('./scripts/verify-rot-budget.mjs').then(m => { const fs = require('fs'); const content = fs.readFileSync('scripts/verify-rot-budget.mjs', 'utf8'); console.log('countLines:', m.countLines(content)); })"
countLines: 993
```
Passed: 993 lines <= 1000 hard boundary and <= 1050 god_file ceiling.

## 疑虑

Parallel wave task W25-4 landed on main (`ee93158`..`e112942`) while W25-2 was executing. The verification window from the initial dispatch BASE `e57c638` includes W25-4 files (`Cargo.lock`, `src/apps/desktop/Cargo.toml`, etc.). As instructed in brief §6 item 8 ("若出现 W25-4 文件，停手报编排者复跑，不得私扩 allowlist"), reporting to orchestrator for alignment. When evaluated from `e112942`, `verify-attempt` passes 100% cleanly.

## 状态

DONE_WITH_CONCERNS
