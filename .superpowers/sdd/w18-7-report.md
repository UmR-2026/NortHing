# W18-7 实施报告

## 改动摘要

### 任务信息
- 任务标识：W18-7（波次计划：`.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md` 相位 5.3 + D-1 + E02，波末单）
- BASE：`0b63224`
- 主文件体量纪律：`scripts/verify-rot-budget.mjs` 基线 2278 行 / ceiling 2300 行；本次净增 18 行（总行数 2296 行 ≤ 2300 行，净增 18 行 ≤ 20 行硬预算）。

### 允许文件集落实
1. `scripts/fixtures/rot-budget/registry.json`（新增）：
   - 登记既有 5 个 manifest 负向 fixture（`bogus-kind.json`、`empty-pattern.json`、`path-escape.json`、`string-ceiling.json`、`unknown-field.json`）的相对路径、文件实测 sha256 与用途描述。
   - `notApplicable` 字段承载 D-1 四起历史事故的判定结果与理由。
2. `scripts/workflow-policy.json`（修改）：
   - `metaRatchetPaths` 数组追加 `"scripts/fixtures/rot-budget/registry.json"`。
3. `scripts/verify-rot-budget.mjs`（修改）：
   - 引入 `node:crypto` 模块。
   - 实现并导出 `attestFixtureRegistry(fixturesDir, projectRoot = process.cwd())` 函数，采用 plan 最低语义 + 缺失/非数组/空 fixtures 单一复合守卫，行尾 CRLF 自动归一化处理跨平台差异；无 registry 跳过。
   - `runSelftest()` 起步执行真实仓库 fixture registry 校验（scriptDir 锚定），失败直接非零退出。
4. `scripts/verify-rot-budget.test.mjs`（修改）：
   - 尾部追加 W18-7 新测试块（F1.1 ~ F1.7 共 7 项用例），覆盖哈希匹配、篡改失败、文件缺失、malformed JSON、缺少 sha256、fixtures 字段缺失/空数组、无 registry 跳过。

### D-1 Replay 判定表
| 事故编号 | 层位分析 | 裁决结果 | 判定理由 |
|---|---|---|---|
| W15-1g | 符号链接/文件系统计数 | not-applicable | Windows 环境下 `fs.symlinkSync` 依赖 Developer Mode 或系统特权，属于跨平台文件系统遍历行为而非 manifest 校验规则，无法稳定作为跨平台静态 manifest fixture 表达；因此判定为 not-applicable（不占用条件槽位）。 |
| W7-2 | 台账回滚 | not-applicable | 流程与 git 卫生层事故，属于工作流提交回滚，rot checker 无法机械表达 git 提交历史回滚状态。 |
| P1-C3 | 编译红未察觉 | not-applicable | CI / 桌面编译闸层事故，已由家规 6（`cargo check -p northhing` 桌面编译门禁）机械覆盖，不在 rot checker 职责范畴。 |
| W15-1l | 续单扩围 | not-applicable | 任务门禁边界层事故，属于任务派发与 verify-task-gate 职责，verify-task-gate 的 selftest 已有 W15 验证用例。 |

### E02 新旧规则对照清单（`df5c1ce` vs 当前 tip）
在独立临时目录通过 `git show df5c1ce:scripts/verify-rot-budget.mjs` 提取旧版 checker（230 行，仅支持基础 `grep-count`/`file-lines`/`dir-entry-count`，无 manifest 结构校验、无 `--base` 支持、无 lease 机制、无 scanScope/rubric/registry attestation），以当前 selftest 53 项清单及 Phase 0 新特性为底册逐项过筛对照：

#### 新增拒绝清单 (New Rejections: 旧规则通过/忽略，新规则拒绝，共 36 项)
1. `Fixture: bogus-kind.json` (selftest #1)
   - 旧判定：PASS（无 manifest kind 白名单校验，非内置 kind 在循环中直接跳过）
   - 新判定：FAIL (`test_entry: invalid kind "bogus-kind", must be one of: grep-count, file-lines, dir-entry-count, exempt-list`)
2. `Fixture: empty-pattern.json` (selftest #4)
   - 旧判定：setup-dependent（环境依赖：旧 checker 执行 `new RegExp("", "g")`，若扫描树为空或无 ≥9 字符文件则为 PASS；若树中含任意 ≥9 字符文件，空正则在每字符位置均匹配导致匹配数 > ceiling 10 而 FAIL）
   - 新判定：FAIL (`test_entry: "pattern" must be a non-empty string for grep-count`，静态语义守卫，不依赖扫描树内容）
3. `Fixture: string-ceiling.json` (selftest #2)
   - 旧判定：PASS（JS 弱类型隐式比对，接受字符串数值 ceiling 如 `"10"`）
   - 新判定：FAIL (`test_entry: "ceiling" must be a non-negative integer, got "10"`)
4. `Fixture: unknown-field.json` (selftest #5)
   - 旧判定：PASS（允许 manifest 条目随意夹带未知字段）
   - 新判定：FAIL (`test_entry: unknown field "unknown_extra" for kind "grep-count"`)
5. `Manifest inline: non-string-note` (selftest #7)
   - 旧判定：PASS（旧 checker 不检查 note 字段类型）
   - 新判定：FAIL (`test_entry: "note" must be a string if present, got number`)
6. `Manifest inline: proto-kind (toString 等原型属性)` (selftest #8)
   - 旧判定：PASS（旧 checker 通过 `if (rule.kind === ...)` 匹配，原型属性未命中任何分支即静默跳过）
   - 新判定：FAIL (`test_entry: invalid kind "toString", must be one of: grep-count, file-lines, dir-entry-count, exempt-list`)
7. `Cap-and-archive 缺少 action.archiveTo` (selftest #12)
   - 旧判定：PASS（目录存在且文件数未超 ceiling 时，旧 checker 完全不检查 action 对象结构）
   - 新判定：FAIL (`dir_entries: "action.archiveTo" must be a non-empty string for cap-and-archive`)
8. `Authorization 缺少 authorization.expires` (selftest #13)
   - 旧判定：PASS（旧 checker 不识别 authorization 字段对象，正常统计未超 ceiling 即放行）
   - 新判定：FAIL (`test_entry: "authorization.expires" must be a non-empty string`)
9. `1001 行文件缺少 exception lease` (selftest #24)
   - 旧判定：PASS（manifest 登记 ceiling 1005 时，1001 ≤ 1005 放行，无 >1000 行硬上限租约约束）
   - 新判定：FAIL (`god_file:src/big.rs: current 1001 exceeds hard limit 1000 without exception lease — register an active lease in scripts/exception-leases.json`)
10. `1001 行文件持有已过期 exception lease` (selftest #25)
    - 旧判定：PASS（旧 checker 不读取 exception-leases.json，仅按 manifest ceiling 比对）
    - 新判定：FAIL (`god_file:src/big.rs: current 1001 exceeds hard limit 1000 with expired lease`)
11. `Exception lease revisit_after 昨天已过期（日期边界）` (selftest #26)
    - 旧判定：PASS（旧 checker 不校验租约 revisit_after 日期）
    - 新判定：FAIL (`god_file:src/big.rs: current 1001 exceeds hard limit 1000 with expired lease (expired YESTERDAY)`)
12. `Banned allow-god-file 注释` (selftest #27)
    - 旧判定：PASS（旧规则允许 `// allow-god-file` 头注释通道）
    - 新判定：FAIL (`src/commented.rs: contains banned comment "allow-god-file" — allow-god-file comment protocol has been abolished`)
13. `Malformed exception leases 文件（非合法 JSON）` (selftest #28)
    - 旧判定：PASS（旧 checker 完全不加载 exception-leases.json，文件语法损坏不受影响）
    - 新判定：FAIL (`Failed to parse exception leases file at scripts/exception-leases.json: Unexpected token ...`)
14. `Exempt-list 路径越界 (path-escape)` (selftest #34)
    - 旧判定：PASS（旧 checker 不支持 exempt-list，遇到该 entry 直接静默忽略）
    - 新判定：FAIL (`exempt_generated_files: path "../escaped.rs" escapes project root`)
15. `Exempt-list 空 paths 数组` (selftest #35)
    - 旧判定：PASS（旧 checker 静默忽略）
    - 新判定：FAIL (`exempt_generated_files: "paths" must be a non-empty array of strings for exempt-list`)
16. `Exempt-list 包含非字符串 path` (selftest #36)
    - 旧判定：PASS（旧 checker 静默忽略）
    - 新判定：FAIL (`exempt_generated_files: paths[0] must be a non-empty string`)
17. `Exempt-list 携带非法字段 ceiling` (selftest #37)
    - 旧判定：PASS（旧 checker 静默忽略）
    - 新判定：FAIL (`exempt_generated_files: unknown field "ceiling" for kind "exempt-list"`)
18. `Workflow-policy 缺少 rotScanScope 字段` (selftest #40)
    - 旧判定：PASS（旧 checker 不读取或校验 workflow-policy.json）
    - 新判定：FAIL (`workflow-policy.json is missing required "rotScanScope" field`)
19. `Workflow-policy rotScanScope 与实际扫描根不匹配` (selftest #39)
    - 旧判定：PASS（旧 checker 无扫描面 attestation 校验）
    - 新判定：FAIL (`rotScanScope.grepRoots mismatch: declared [...] does not match actual [...]`)
20. `Workflow-policy rotScanScope 形态非法` (selftest #41)
    - 旧判定：PASS（旧 checker 不解析 policy 文件）
    - 新判定：FAIL (`workflow-policy.json "rotScanScope" must be an object`)
21. `Workflow-policy 缺少 rotVerdictRubric 或 rubric mismatch` (W18-6)
    - 旧判定：PASS（旧 checker 无 rubric SSOT attestation）
    - 新判定：FAIL (`rotVerdictRubric.healthy mismatch: declared "wrong" does not match actual "0 findings"`)
22. `Scripts 目录 1001 行脚本无 lease` (selftest #42)
    - 旧判定：PASS（旧扫描面仅限 `src/*.rs`，scripts 目录完全未受监控）
    - 新判定：FAIL (`god_file:scripts/big.mjs: current 1001 exceeds hard limit 1000 without exception lease`)
23. `Scripts 目录 900 行未登记脚本` (selftest #43)
    - 旧判定：PASS（旧扫描面未监控 scripts 目录）
    - 新判定：FAIL (`god_file:scripts/unreg.js: current 900 exceeds ceiling 800 — split, reduce, or register a justified manifest entry`)
24. `Scripts 目录含 banned allow-god-file 注释` (selftest #44)
    - 旧判定：PASS（旧扫描面未监控 scripts 目录，且旧规则不禁止此注释）
    - 新判定：FAIL (`scripts/c.mjs: contains banned comment "allow-god-file"`)
25. `Northing-installer 目录 >800 行未登记代码` (W18-5a)
    - 旧判定：PASS（旧扫描面未包含 northing-installer）
    - 新判定：FAIL (`god_file:northing-installer/...: current 850 exceeds ceiling 800`)
26. `--base 未授权上调 ceiling` (selftest #14)
    - 旧判定：PASS（旧 CLI 完全无 `--base` argv 支持，静默忽略 `--base` 参数，直接按 tip 比对通过）
    - 新判定：FAIL (`m: ceiling raised from 10 to 20 without authorization — raising a ceiling requires user sign-off`)
27. `--base 持有已过期 authorization 上调 ceiling` (selftest #15)
    - 旧判定：PASS（旧 CLI 忽略 `--base` 参数，不校验 authorization 过期）
    - 新判定：FAIL (`m: ceiling raised from 10 to 20 with expired authorization (expired 2020-01-01, today ...)`)
28. `--base 授权 authorization.expires 昨天已过期（日期边界）` (selftest #20)
    - 旧判定：PASS（旧 CLI 忽略 `--base` 参数）
    - 新判定：FAIL (`m: ceiling raised from 10 to 20 with expired authorization (expired YESTERDAY)`)
29. `--base 降低 ceiling 违反 headroom floor（余量不足）` (selftest #16)
    - 旧判定：PASS（旧 CLI 忽略 `--base`，且无余量底线规则）
    - 新判定：FAIL (`god_file:src/heavy.rs: ceiling lowered ... violates headroom floor ... register in scripts/exception-leases.json`)
30. `--base tip manifest 擅自删除已有指标` (selftest #17)
    - 旧判定：PASS（旧 CLI 忽略 `--base`，tip 删掉指标后不再比对即静默消失）
    - 新判定：FAIL (`metric exists in base manifest but was removed in tip manifest (deleting metrics is prohibited)`)
31. `--base 死注册擅自降低 ceiling` (selftest #29)
    - 旧判定：PASS（旧 CLI 忽略 `--base`，文件缺失仅 warning 或跳过）
    - 新判定：FAIL (`god_file:src/ghost.rs: dead registration with lowered ceiling ... violates headroom floor`)
32. `--base scripts 退役配额净增（含子目录情形）` (selftest #49)
    - 旧判定：PASS（旧 CLI 忽略 `--base`，仅按当前目录项数与 ceiling 比对）
    - 新判定：FAIL (`dir_entries:scripts: current exceeds base count — net increase prohibited under scripts retirement quota`)
33. `--base scripts 退役配额对比实际基线数量（非 ceiling）` (selftest #50)
    - 旧判定：PASS（旧 CLI 忽略 `--base`，只要未超 ceiling 即放行）
    - 新判定：FAIL (`dir_entries:scripts: current exceeds base count — net increase prohibited under scripts retirement quota`)
34. `Dead registration 超期 31 天升级违规` (W18-6)
    - 旧判定：PASS（旧 checker 仅输出 warning 或跳过，无 30 天限期升级机制）
    - 新判定：FAIL (`god_file:src/ghost.rs: dead registration has exceeded 30-day grace period — remove the entry`)
35. `Manifest god_file 携带格式非法 deadSince` (W18-6)
    - 旧判定：PASS（旧 checker 不解析或校验 deadSince 格式）
    - 新判定：FAIL (`god_file:src/ghost.rs: "deadSince" must be in YYYY-MM-DD format`)
36. `Fixture registry 文件被篡改/条目丢失/哈希漂移/空 fixtures 数组` (W18-7)
    - 旧判定：PASS（旧 checker 无 fixture registry sha256 锁定机制）
    - 新判定：FAIL (`sha256 mismatch` / `fixture not found` / `fixtures missing or empty`)

#### 解除拒绝清单 (Relieved Rejections: 旧规则拒绝，新规则通过，共 1 项)
1. `exempt-list file >800 lines` (selftest #38)
   - 旧判定：FAIL (`god_file:src/gen.rs: current 850 exceeds ceiling 800`，旧规则无 exempt-list 概念，生成代码超过 800 行即便登记也会因缺少有效 god-file 豁免机制被直接阻断)
   - 新判定：PASS（新规则识别 `exempt_generated_files` 的 `exempt-list` 声明，合法免除 god-file 限制）

---

## 验证

### 旧代码行为确认与 GC4 负向先行证据
在未修改 `scripts/verify-rot-budget.mjs` 前，先向 `scripts/verify-rot-budget.test.mjs` 引入新用例并导入 `attestFixtureRegistry`，执行测试：

```text
$ node scripts/verify-rot-budget.test.mjs
file:///<REPO_ROOT>/scripts/verify-rot-budget.test.mjs:9
import { verifyRotBudget, attestFixtureRegistry } from './verify-rot-budget.mjs';
                          ^^^^^^^^^^^^^^^^^^^^^
SyntaxError: The requested module './verify-rot-budget.mjs' does not provide an export named 'attestFixtureRegistry'
    at #asyncInstantiate (node:internal/modules/esm/module_job:327:21)
    at async ModuleJob.run (node:internal/modules/esm/module_job:431:5)
    at async node:internal/modules/esm/loader:643:26
    at async asyncRunEntryPointWithESMLoader (node:internal/modules/run_main:101:5)

Node.js v24.19.0
EXIT_CODE=1
```
确认旧代码无 registry 校验且无新具名导出，模块加载失败整文件红，确证 fail-open-by-absence。

---

### 1. `node scripts/verify-rot-budget.mjs`
```text
$ node scripts/verify-rot-budget.mjs
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=78/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
EXIT_CODE=0
```
读数零漂移：5 grep 规则读数完全一致；dir_entries:scripts=45/48；9 god-file 规则；checkedFiles 1397；verdict: rotting。

### 2. `node scripts/verify-rot-budget.mjs --selftest`
```text
$ node scripts/verify-rot-budget.mjs --selftest
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
EXIT_CODE=0
```

### 3. `node scripts/verify-rot-budget.mjs --base 6dc9b13`
```text
$ node scripts/verify-rot-budget.mjs --base 6dc9b13
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=78/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
EXIT_CODE=0
```

### 4. `node scripts/verify-rot-budget.test.mjs`
```text
$ node scripts/verify-rot-budget.test.mjs
✔ compliant fixture exits 0 and reports success (118.1121ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (114.6756ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (114.7651ms)
✔ registered god-file exceeding ceiling fails (6.3565ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (6.9238ms)
✔ dir-entry-count compliant fixture passes (99.8375ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (102.7869ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (94.37ms)
✔ tests.rs file is excluded from rot budget measurement (5.5589ms)
✔ *_tests directory files are excluded from rot budget measurement (5.4493ms)
✔ actual workspace rot budget passes with current manifest (311.9473ms)
✔ dead god-file registration warns but does not fail verification (98.3481ms)
✔ W18-6 F1.1: workflow policy missing rotVerdictRubric fails and reports violation (4.125ms)
✔ W18-6 F1.2: workflow policy with malformed rotVerdictRubric (missing key) fails (3.5097ms)
✔ W18-6 F1.3: workflow policy rotVerdictRubric value mismatch with pinned literal fails (3.5209ms)
✔ W18-6 F1.4: valid rubric with 0 findings passes and verdict is healthy (3.6363ms)
✔ W18-6 F1.5: 1 warning (bounded) passes with verdict stable (3.6123ms)
✔ W18-6 F1.6: >=3 findings results in verdict rotting (3.5645ms)
✔ W18-6 F1.7: unregistered file exceeding 800 lines results in verdict rotting (unbounded) (4.9709ms)
✔ W18-6 F1.8: manifest validation failure early exit does not carry verdict field (2.4638ms)
✔ W18-6 F1.9: green path spawn stdout contains verdict segment (92.6769ms)
✔ W18-6 F2.1: dead registration without deadSince produces warning and passes (3.3355ms)
✔ W18-6 F2.2: dead registration with deadSince 31 days ago fails with violation (2.9836ms)
✔ W18-6 F2.3: dead registration with deadSince exactly 30 days ago produces warning and passes (2.7766ms)
✔ W18-6 F2.4: live file with deadSince ignores deadSince and emits no warning (4.0262ms)
✔ W18-6 F2.5: malformed deadSince is rejected by manifest validation (2.4305ms)
✔ W18-7 F1.1: fixture registry with matching hashes passes (4.9188ms)
✔ W18-7 F1.2: fixture registry with tampered file hash fails (4.4639ms)
✔ W18-7 F1.3: fixture registry pointing to missing file fails (3.6238ms)
✔ W18-7 F1.4: malformed fixture registry fails (3.5574ms)
✔ W18-7 F1.5: fixture registry entry missing sha256 fails (4.5043ms)
✔ W18-7 F1.6: fixture registry missing or empty fixtures array fails (4.3795ms)
✔ W18-7 F1.7: missing fixture registry file skips attestation and passes (3.1183ms)
ℹ tests 33
ℹ suites 0
ℹ pass 33
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1255.2775
EXIT_CODE=0
```

### 5. `node scripts/verify-task-gate.mjs validate-policy`
```text
$ node scripts/verify-task-gate.mjs validate-policy
Policy validation passed: <REPO_ROOT>\scripts\workflow-policy.json
EXIT_CODE=0
```

### 6. 篡改演示
#### 篡改真实 fixture 1 字节并执行 `--selftest`：
```text
$ node -e "const fs = require('fs'); const p = 'scripts/fixtures/rot-budget/bogus-kind.json'; fs.writeFileSync(p, fs.readFileSync(p, 'utf8').replace('10', '11'), 'utf8');"
$ node scripts/verify-rot-budget.mjs --selftest
sha256 mismatch for scripts/fixtures/rot-budget/bogus-kind.json
EXIT_CODE=1
```

#### `git checkout --` 还原并复跑 `--selftest`：
```text
$ git checkout -- scripts/fixtures/rot-budget/bogus-kind.json
$ node scripts/verify-rot-budget.mjs --selftest
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
EXIT_CODE=0
```

### 7. `pnpm run check:repo-hygiene`
```text
$ pnpm run check:repo-hygiene

> northhing@0.2.10 check:repo-hygiene <REPO_ROOT>
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (6 content files scanned, 3866 filenames checked).
EXIT_CODE=0
```

---

## 状态
DONE
