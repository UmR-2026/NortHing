# W18-5a 实施报告 — 扫描范围 attestation + 收集器扩展 + 登记/lease

## 改动摘要

1. **attestation 机制**：
   - 在 SSOT `scripts/workflow-policy.json` 中声明 `"rotScanScope": {"grepRoots": ["src"], "fileLinesRoots": ["src", "northing-installer/src-tauri", "scripts"]}`。
   - 在 `scripts/verify-rot-budget.mjs` 中实现 `attestScanScope(policyPath, projectRoot)`，启动时断言声明的扫描根集合与 checker 内部定义的 `SCAN_SCOPE_GREP_ROOTS`（`['src']`）及 `SCAN_SCOPE_FILE_LINES_ROOTS`（`['src', 'northing-installer/src-tauri', 'scripts']`）一致。
   - 严格语义：policy 文件缺失 ⇒ attestation 跳过（支持既有 38 项 selftest 与 12 项 test.mjs 的合成 tmpdir 根）；policy 文件存在但缺少 `rotScanScope` 字段 ⇒ violation；`rotScanScope` 结构非法（非对象、非字符串数组）⇒ violation；声明根集合与实际根集合不一致 ⇒ violation。
   - 验证成功时（非 silent 模式），输出逐根文件计数清单 `[src: 1368, northing-installer/src-tauri: 11, scripts: 18]`。

2. **收集器扩展面**：
   - 实现 `collectScriptFiles(scriptsDir, projectRoot)`，仅收集 `scripts/` 顶层 `.mjs`/`.js`，并通过 `/\.(test|spec)\.(mjs|js)$/` 排除测试文件（保留 `test-acp.js` 纳入扫描）。
   - file-lines 扫描面扩展为三根：`src`（.rs，现有语义）、`northing-installer/src-tauri`（.rs，同现有排除规则）、`scripts`（顶层非测试脚本）。
   - grep-count 规则保持仅对 `src` 扫描，确保 grep 读数零漂移。
   - 总扫描文件数从 1368 扩至 1397（1368 + 11 + 18）。

3. **登记与 lease 清单**：
   - `scripts/rot-budget.json`（新增 3 项 god-file 登记，零触碰既有条目）：
     - `god_file:scripts/verify-task-gate.mjs`：ceiling 847
     - `god_file:scripts/verify-rot-budget.mjs`：ceiling 2300
     - `god_file:scripts/i18n-audit.mjs`：ceiling 4025
     - 全部注明 note：「W18-5a 扫描范围扩面登记，用户拍板 2026-09-07」。
   - `scripts/exception-leases.json`（新增 2 条 lease）：
     - `scripts/verify-rot-budget.mjs`：owner: "orchestrator", revisit_after: "2026-10-15", reason: "W18 波内 selftest 内联增长", next_action: "波后拆分 selftest 外置"
     - `scripts/i18n-audit.mjs`：owner: "orchestrator", revisit_after: "2026-10-15", reason: "i18n 工程冻结期存量", next_action: "解冻时拆分"

4. **检查提取方式**：
   - 提取通用面逻辑函数 `checkFileLinesSurface({ file, content, lineCount, exemptPaths, godFileRules, seenGodFiles, leaseMap, baseManifest, todayUtc })`。
   - 将 >1000 硬边界检查（需有效 lease 覆盖）、allow-god-file 注释禁令、godFileRules ceiling 检查、headroom floor 检查以及 >800 未登记检查从原有的单一 `.rs` 循环中提取为通用文件面执行逻辑，供三根收集器收集的全部文件（除 exemptPaths 豁免外）统一调用。
   - 注释禁令使用行首锚定正则 `BANNED_COMMENT_REGEX = /^[ \t]*\/\/[ \t]*allow-god-file/m`，避免 checker 自身的 8 处代码/字面量自触发，同时保留对任何行首注释形态（如 `// allow-god-file: ...`）的精准拦截。

5. **selftest 扩展**：
   - 新增 6 项负例与 4 项正例（全部基于 tmpdir 合成，零新增 fixture 文件），总用例数由 38 增至 48（31 negative, 17 positive）。

---

## 验证

### 旧代码行为确认（GC4 先负向后实现）

在旧代码（`356b33c` 基线）下，扫描逻辑仅调用 `collectRustFiles(srcDir, projectRoot)`，扫描面仅包含 `src/` 下的 `.rs` 文件。
- `northing-installer/src-tauri` 与 `scripts/` 完全不在扫描循环中。
- `scripts/` 中当时已存在 3833 行的 `i18n-audit.mjs`、1692 行的 `verify-rot-budget.mjs` 以及 806 行的 `verify-task-gate.mjs`，且若向 `scripts/` 或 installer 中写入 `// allow-god-file` 注释，旧 checker 均报告 `success: true`（0 violations）。
- 该盲区属于典型的 **fail-open by absence**。本任务通过 SSOT 声明 + attestation 校验 + 收集器扩展彻底将两处盲区纳入机械闸内。

### 扩面前后读数对照表

| 指标 / 规则 | BASE (356b33c) | TIP (W18-5a) | 差异说明 |
|---|---|---|---|
| Checked Files Count | 1368 | 1397 | +29 (+11 installer .rs, +18 scripts 非测试脚本) |
| God-file Rules | 6 | 9 | +3 项用户拍板扩面登记 |
| unwrap_production (grep) | 483 / 502 | 483 / 502 | 零漂移（grep 仅扫描 src） |
| expect_production (grep) | 940 / 1089 | 940 / 1089 | 零漂移 |
| let_underscore (grep) | 370 / 388 | 370 / 388 | 零漂移 |
| unix_epoch_inline (grep) | 69 / 69 | 69 / 69 | 零漂移 |
| allow_dead_code (grep) | 104 / 109 | 104 / 109 | 零漂移 |
| dir_entries:scripts | 45 / 48 | 45 / 48 | 不变 |
| dir_entries:docs/design | 1 / 1 | 1 / 1 | 不变 |
| dir_entries:.superpowers/sdd | 73 / 400 | 73 / 400 | 不变（实时读数） |
| 零余量 Warning 项数 | 5 | 5 | 保持 5 项（3 新增 god-file 均有充裕 headroom） |

### policy 缺失跳过的残余面说明

- **合成根（synthetic root）**：既有 38 项 selftest 及 12 项 `verify-rot-budget.test.mjs` 均在内联合成的临时目录（tmpdir）中运行，这些合成环境仅提供 `rot-budget.json`，不生成 `workflow-policy.json`。此时 `attestScanScope` 检测到 policy 文件不存在，判定为跳过 attestation。
- **真实仓库兜底机制**：在真实仓库中，`scripts/workflow-policy.json` 由 git 严格跟踪，属于 `metaRatchetPaths` 保护文件；一旦文件存在，checker 会强制校验其必须包含合法的 `rotScanScope` 且与实际扫描根完全一致（缺失或格式非法均报 violation）；此外 `verify-task-gate.mjs validate-policy` 保障 policy 整体结构完好。残余面仅限于无 policy 的孤立合成环境，在生产及 CI 流程中不存在 fail-open 风险。

---

### 验证命令执行原文及 Exit Code

#### 1. `node scripts/verify-rot-budget.mjs` (exit code: 0)

```text
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=73/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

#### 2. `node scripts/verify-rot-budget.mjs --selftest` (exit code: 0)

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
Selftest passed: 48 checks passed (31 negative, 17 positive).
```

#### 3. `node scripts/verify-rot-budget.mjs --base 356b33c` (exit code: 0)

```text
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=73/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

#### 4. `node scripts/verify-rot-budget.test.mjs` (exit code: 0)

```text
✔ compliant fixture exits 0 and reports success (115.249ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (107.3581ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (108.3984ms)
✔ registered god-file exceeding ceiling fails (6.6981ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (7.5555ms)
✔ dir-entry-count compliant fixture passes (105.089ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (108.2365ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (103.26ms)
✔ tests.rs file is excluded from rot budget measurement (7.0974ms)
✔ *_tests directory files are excluded from rot budget measurement (7.3836ms)
✔ actual workspace rot budget passes with current manifest (375.0281ms)
✔ dead god-file registration warns but does not fail verification (107.604ms)
ℹ tests 12
ℹ suites 0
ℹ pass 12
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1168.9115
```

#### 5. `node scripts/verify-task-gate.mjs validate-policy` (exit code: 0)

```text
Policy validation passed: <REPO_ROOT>/scripts/workflow-policy.json
```

#### 6. `pnpm run check:repo-hygiene` (exit code: 0)

```text
> northhing@0.2.10 check:repo-hygiene <REPO_ROOT>
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (7 content files scanned, 3859 filenames checked).
```

---

## 状态

DONE
