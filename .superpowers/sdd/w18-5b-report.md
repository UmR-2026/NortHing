# W18-5b 交付报告 — scripts 退役机械化（净不得增）

## 改动摘要

在 `scripts/verify-rot-budget.mjs` 中实现 scripts 退役机械化（净不得增口径）：
1. **退役规则逻辑**：
   - 在 `--base` 模式下，通过 `git ls-tree <base> scripts/` 获取 BASE 侧顶层 regular file 计数，严格按 `blob` 类型过滤，排除 `tree` 条目（子目录），与 TIP 侧 `fs.readdirSync` 的 `isFile()` 语义逐字一致。
   - 比较 TIP 实测计数 vs BASE 实测计数（ceiling 48 不参与此项比较）：当 TIP > BASE 时触发违规，违规信息采用 English-only 文案：
     `dir_entries:scripts: current <tip> exceeds base count <base> — net increase prohibited under scripts retirement quota (expires 2026-10-15)`
   - 零新增外部依赖，纯 Node.js 标准库实现。
2. **内联 selftest 扩展（新增 5 项，总数 48 → 53 项）**：
   - **用例 49（负例，判别 blob-only 语义的 fail-open 方向）**：BASE `scripts/` 含子目录且 TIP 净增数 ≤ 子目录数 ⇒ 捕获净增并红，证明排除了 tree 条目对 BASE 读数的虚高影响。
   - **用例 50（判别用例，钉「比较计数而非 ceiling」）**：BASE manifest ceiling=48 但 BASE 实测=5、净增 2（TIP 实测=7 < 48）⇒ 判定违规红，证明判定逻辑不依赖 ceiling。
   - **用例 51（正例，增删平衡）**：TIP 删 1 增 1，计数不变 ⇒ 绿。
   - **用例 52（正例，不变）**：TIP 文件无增减 ⇒ 绿。
   - **用例 53（正例，BASE 侧 scripts/ 含子目录且 TIP 计数不变）**：子目录存在时 TIP 侧准确排除子目录，TIP 计数与 BASE blob 计数一致 ⇒ 绿。
3. **预算与文件保护**：
   - 唯一修改文件 `scripts/verify-rot-budget.mjs`（2205 行），在 `god_file:scripts/verify-rot-budget.mjs` ceiling 2300 范围内，未上调任何 ceiling，未修改任何 manifest/policy/lease 文件。

---

## 验证

### 旧代码行为确认（GC4 先负向 fixture 后实现）

在修改前对旧代码进行实测，确认旧代码在 `--base` 模式下无 scripts 退役规则（fail-open by absence）：
- 测试场景：BASE commit 实测 2 个文件，TIP 新增 1 个文件（净增至 3 个文件，均 ≤ ceiling 48）。
- 旧代码实测输出：`{ success: true, violations: [] }`。旧代码未检查净增，静默通过（fail-open）。
- 本单实现后：同一场景精确捕获违规并报错 `net increase prohibited under scripts retirement quota (expires 2026-10-15)`。

---

### 验证命令执行记录

#### 1. `node scripts/verify-rot-budget.mjs`
```text
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=74/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```
Exit code: 0（读数与 W18-5a 基线零漂移：9 god-file rules、checkedFiles 1397、scripts 45/48）。

#### 2. `node scripts/verify-rot-budget.mjs --selftest`
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
Exit code: 0（53 项全绿，含 5 项新增退役规则用例）。

#### 3. `node scripts/verify-rot-budget.mjs --base 6dc9b13`
```text
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=74/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```
Exit code: 0（本单未增减 scripts 文件，45 == 45，全绿通过）。

#### 4. `node scripts/verify-rot-budget.test.mjs`
```text
✔ compliant fixture exits 0 and reports success (113.3235ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (107.9692ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (123.1462ms)
✔ registered god-file exceeding ceiling fails (5.9318ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (6.2626ms)
✔ dir-entry-count compliant fixture passes (104.7962ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (98.9412ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (95.5397ms)
✔ tests.rs file is excluded from rot budget measurement (6.0889ms)
✔ *_tests directory files are excluded from rot budget measurement (7.04ms)
✔ actual workspace rot budget passes with current manifest (338.5653ms)
✔ dead god-file registration warns but does not fail verification (99.7269ms)
ℹ tests 12
ℹ suites 0
ℹ pass 12
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1115.8923
```
Exit code: 0（12 项回归全绿）。

#### 5. `pnpm run check:repo-hygiene`
```text
> northhing@0.2.10 check:repo-hygiene <REPO_ROOT>
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (1 content files scanned, 3861 filenames checked).
```
Exit code: 0（全绿通过）。

---

## 状态

DONE
