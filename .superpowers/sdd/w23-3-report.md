# W23-3 实施报告

## 改动摘要

本任务为 W23 诚实化波第三单（meta-ratchet 车道，用户签字已含于 2026-09-09 拍板）。完成 D7-① ci.yml 注释诚实化与 A-4 attestFixtureRegistry 缺文件 fail-closed 及 F1.7 测试翻转：

1. **`.github/workflows/ci.yml`（D7-①）**：
   - 将第 32 行注释诚实化为：`# Windows-only per user decision 2026-09-05; terminal-core E0624 fixed 2026-09-08 (W20-1); cross-target compile sentinel planned (W24)`。
   - 零 job/step/runner 配置改动，纯注释行更新。

2. **`scripts/verify-rot-budget.mjs`（A-4）**：
   - 修改 `attestFixtureRegistry` 首行缺文件处理分支（第 386 行，≤3 行约束，实际 1 行改动）：
     由 `if (!fs.existsSync(regPath)) return { success: true, violations: [] };`
     改为 `if (!fs.existsSync(regPath)) return { success: false, violations: [`fixture registry file missing (fail-closed): ${regPath}`] };`。
   - 钉死文案 `fixture registry file missing (fail-closed): ${regPath}`。其他分支与主路径保持不动。

3. **`scripts/verify-rot-budget.test.mjs`（F1.7 翻转）**：
   - 将 F1.7 测试由 `missing fixture registry file skips attestation and passes` 改为 `missing fixture registry file fails closed`。
   - 断言更新为 `assert.equal(result.success, false)` 并验证违规列表逐字包含 `fixture registry file missing (fail-closed): ${regPath}`。

4. **关于 `selftest-cases.test.mjs` 与 `registry.json` 未修改说明**：
   - `scripts/fixtures/rot-budget/selftest-cases.test.mjs`：`attestFixtureRegistry` 由 `scripts/verify-rot-budget.mjs:939`（`runSelftest()` 入口处）起步前置校验，`selftest-cases.test.mjs` 仅承载 manifest 用例校验，不测 registry 存在性，无需改动。
   - `scripts/fixtures/rot-budget/registry.json`：本任务未增删改任何 fixture 文件，registry 列表与其内 fixture sha256 均无需调整。

Commit 信息：`e63f328` `fix(ci): honest comment and fail-closed fixture registry (W23-3)`。

---

## 验证

### 1. `node scripts/verify-rot-budget.mjs --selftest`
Exit code: 0
输出：
```text
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

### 2. `node scripts/verify-rot-budget.test.mjs`
Exit code: 0
输出：
```text
✔ compliant fixture exits 0 and reports success (110.0512ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (103.7079ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (110.8054ms)
✔ registered god-file exceeding ceiling fails (6.064ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (6.4551ms)
✔ dir-entry-count compliant fixture passes (108.5831ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (109.0836ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (101.8321ms)
✔ tests.rs file is excluded from rot budget measurement (6.5828ms)
✔ *_tests directory files are excluded from rot budget measurement (6.1282ms)
✔ actual workspace rot budget passes with current manifest (350.6199ms)
✔ dead god-file registration warns but does not fail verification (105.4053ms)
✔ W18-6 F1.1: workflow policy missing rotVerdictRubric fails and reports violation (4.8677ms)
✔ W18-6 F1.2: workflow policy with malformed rotVerdictRubric (missing key) fails (4.3485ms)
✔ W18-6 F1.3: workflow policy rotVerdictRubric value mismatch with pinned literal fails (4.3558ms)
✔ W18-6 F1.4: valid rubric with 0 findings passes and verdict is healthy (4.2078ms)
✔ W18-6 F1.5: 1 warning (bounded) passes with verdict stable (4.3034ms)
✔ W18-6 F1.6: >=3 findings results in verdict rotting (4.0566ms)
✔ W18-6 F1.7: unregistered file exceeding 800 lines results in verdict rotting (unbounded) (5.73ms)
✔ W18-6 F1.8: manifest validation failure early exit does not carry verdict field (2.6132ms)
✔ W18-6 F1.9: green path spawn stdout contains verdict segment (100.4065ms)
✔ W18-6 F2.1: dead registration without deadSince produces warning and passes (4.1927ms)
✔ W18-6 F2.2: dead registration with deadSince 31 days ago fails with violation (3.588ms)
✔ W18-6 F2.3: dead registration with deadSince exactly 30 days ago produces warning and passes (3.084ms)
✔ W18-6 F2.4: live file with deadSince ignores deadSince and emits no warning (4.7536ms)
✔ W18-6 F2.5: malformed deadSince is rejected by manifest validation (2.9499ms)
✔ W18-7 F1.1: fixture registry with matching hashes passes (5.6706ms)
✔ W18-7 F1.2: fixture registry with tampered file hash fails (5.9003ms)
✔ W18-7 F1.3: fixture registry pointing to missing file fails (4.8609ms)
✔ W18-7 F1.4: malformed fixture registry fails (4.4687ms)
✔ W18-7 F1.5: fixture registry entry missing sha256 fails (5.2402ms)
✔ W18-7 F1.6: fixture registry missing or empty fixtures array fails (5.1795ms)
✔ W18-7 F1.7: missing fixture registry file fails closed (3.2955ms)
✔ W19-2 F1: parseArgs supports --flag=value and rejects unknown flags (0.6108ms)
✔ W19-2 F2: BANNED_COMMENT_REGEX matches /// and /* comments and preserves non-anchored string literals (0.1001ms)
✔ W19-2 F3: dangling exception lease produces warning and live lease produces no warning (5.9442ms)
✔ W19-2 F4: future deadSince date is rejected by manifest validation (0.9937ms)
✔ W19-2 F8: attestFixtureRegistry polishes invalid entry to <invalid entry> (4.5972ms)
✔ W21-1 F1: spawn verify-rot-budget.mjs --base= fails closed with exit 1 and error message (93.3875ms)
ℹ tests 39
ℹ suites 0
ℹ pass 39
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1429.8807
```

### 3. `node scripts/verify-rot-budget.mjs`
Exit code: 0
输出：
```text
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=106/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

### 4. 红态探针（含旁证与恢复确认）
- 探针前 `scripts/fixtures/rot-budget/registry.json` SHA256：`2EEFED155AF96E808A44D013A046BF7D3D8C2BBEB07A3C7042C2C89151934954`
- 临时重命名移走 `registry.json` 后执行探针：
  - Selftest 红态验证：
    命令：`node scripts/verify-rot-budget.mjs --selftest`
    Exit code: 1
    输出：
    ```text
    fixture registry file missing (fail-closed): E:\agent-project\northing\scripts\fixtures\rot-budget\registry.json
    ```
  - 旁证验证（主路径行为不变）：
    命令：`node scripts/verify-rot-budget.mjs`
    Exit code: 0
    输出：
    ```text
    warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
    Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=106/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
    warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
    warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
    warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
    warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
    ```
- 恢复确认：
  - 恢复后 `scripts/fixtures/rot-budget/registry.json` SHA256：`2EEFED155AF96E808A44D013A046BF7D3D8C2BBEB07A3C7042C2C89151934954`（一致）
  - 工作树干净度检查：`git status --porcelain` 仅未跟踪的本单报告/brief-review，无任何 WIP 修改残留。

### 5. `node scripts/check-repo-hygiene.mjs`
Exit code: 0
输出：
```text
Repository hygiene check passed (1 content files scanned, 3903 filenames checked).
```

### 6. CI 语法与变更证明（`git diff 13bdc94..HEAD .github/workflows/ci.yml`）
输出：
```diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index ff3f8cc..597eca4 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -29,7 +29,7 @@ jobs:
     strategy:
       fail-fast: false
       matrix:
-        # Windows-only per user decision 2026-09-05; non-Windows builds currently broken (terminal-core E0624), see tech-debt-ledger
+        # Windows-only per user decision 2026-09-05; terminal-core E0624 fixed 2026-09-08 (W20-1); cross-target compile sentinel planned (W24)
         os:
           - windows-latest
     steps:
```
证明：仅第 32 行注释变更，零 job / step / runner 配置变动。

---

## 状态

DONE
