# W19-2 Report — checker 清账单（波终审 triage「记下一波」收口）

## 改动摘要

本单针对 W18 波级终审 triage 及 W19-1 53 Minor 遗留项进行一单清账，涉及两脚本解析统一、注释禁令扩面、悬空 lease 告警、deadSince 未来日期守卫、F2.5 断言钉机制、task-gate 续单误报修复、ceiling note 追记修复及 fixture registry polish。

### 1. 修改文件与行数体量

- `scripts/verify-rot-budget.mjs`：
  - 支持 `--flag=value` 解析与白名单校验（`ALLOWED_FLAGS = new Set(['selftest', 'base'])`），未知 flag 抛出英文错误并在 CLI 入口捕获后 exit 1；
  - `BANNED_COMMENT_REGEX` 扩面为目标形态 `^[ \t]*(?:\/\/+|\/\*+)[ \t]*allow-god-file`（保持行首锚定，覆盖 `///` 与 `/*`）；
  - `validateManifest` 增加 `todayUtc` 守卫，`deadSince > todayUtc` 拒绝（fail-closed）；
  - `attestFixtureRegistry` 缺 path 或无效条目 fallback 打印 `<invalid entry>`；
  - 预扫处增加悬空 lease 检查，lease 目标文件不存在时 emit warning（不红）。
  - 体量：原 983 行，现 999 行，净增 16 行（约束 ≤30 行，ceiling 1050）。
- `scripts/verify-task-gate.mjs`：
  - 支持 `--flag=value` 与 `-h` 解析并补 export，白名单为 `selftest/help/h/policy/base/tip/allowlist`，未知 flag 抛出英文错误并在 CLI 入口捕获后 exit 1；
  - 续单检查正则修改为 `/(?<!后)续单/`，避免误伤「后续单」；
  - 内联 selftest 追加 3 个直接调用用例（negative i、positive 6、positive 7），复现真实误报形态及 flag 解析。
  - 体量：原 806 行，现 834 行，净增 28 行（约束 ≤40 行，ceiling 847）。
- `scripts/verify-rot-budget.test.mjs`：
  - F2.5 断言从 `includes('deadSince')` 收紧为匹配 `must match YYYY-MM-DD`；
  - 尾部追加 W19-2 5 个新用例（F1 flag 解析与未知 flag 拦截、F2 注释正则双向、F3 悬空 lease 告警与 live lease 过滤、F4 未来 deadSince 拦截、F8 fixture registry polish）。
  - 体量：原 972 行，现 1070 行，38/38 用例全部通过。
- `scripts/rot-budget.json`：
  - `god_file:scripts/verify-rot-budget.mjs` 的 note 恢复累积式：`"W18-5a 扫描范围扩面登记，用户拍板 2026-09-07；W19-1 selftest 外置拆分重组 2300→1050"`。
  - 体量：103 行，净增 0 行，ceiling 保持 1050 不动。

### 2. E02 映射表

| 用例 ID / 位置 | E02 场景描述 | 预期行为 |
|---|---|---|
| `scripts/verify-rot-budget.test.mjs`: `W19-2 F1` | rot-budget CLI flag 形态与白名单 | `--base=sha` 成功拆分赋值；未知 flag / 非白名单 flag 抛错拦截 |
| `scripts/verify-rot-budget.test.mjs`: `W19-2 F2` | 注释禁令正则覆盖度 | `/// allow-god-file` 与 `/* allow-god-file */` 命中；行内字面量及普通注释不误伤 |
| `scripts/verify-rot-budget.test.mjs`: `W19-2 F3` | 异常 lease 文件存在性 | 登记文件不存在产生 warning（不红）；存在文件无 warning |
| `scripts/verify-rot-budget.test.mjs`: `W19-2 F4` | deadSince 未来时间守卫 | `deadSince > todayUtc` 拒绝 fail-closed |
| `scripts/verify-rot-budget.test.mjs`: `W18-6 F2.5` | deadSince 格式校验断言钉住 | 检验非 YYYY-MM-DD 产生既有 `must match YYYY-MM-DD` 错误文案 |
| `scripts/verify-rot-budget.test.mjs`: `W19-2 F8` | fixture registry 结构异常防御 | fixture entry 缺失 path 时错误提示兜底显示 `<invalid entry>` |
| `scripts/verify-task-gate.mjs`: `negative fixture i` | 任务门禁续单检查（真续单缺 BASE） | 包含「续单」但无独立 `BASE` 行时报错拦截 |
| `scripts/verify-task-gate.mjs`: `positive fixture 6` | 任务门禁续单检查误报防护（含「后续单」） | 六节俱全且 BASE 节为 `## BASE` 标题形态，正文含「后续单」合规通过 |
| `scripts/verify-task-gate.mjs`: `positive fixture 7` | task-gate parseArgs 拆分与拦截 | `--base=sha` / `-h` 成功拆分；未知 flag 抛错拦截 |

### 3. 已知上限声明

关于任务门禁「续单」形态识别：本单修补负向后行断言 `/(?<!后)续单/` 以解决真实发生的「后续单」误报。引号包裹的「续单」（如 `「续单」`）、接续单元等边缘形态仍可能触发包含检测并要求独立 BASE 行，此属 triage 外残余面，不在本单收口。

---

## 验证

### 1. 旧代码行为确认（先负向先行证据）

#### (1) `--base=...` 形态在旧代码中静默降级（Fail-open 实测）
旧代码解析器遇到 `--base=invalid-ref-xyz` 时无法拆分，作为 key 未识别且无未知 flag 校验，导致静默跳过 baseline 检查而以 exit code 0 误过：
```
> node scripts/verify-rot-budget.mjs --base=invalid-ref-xyz
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=82/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
Exit code: 0
```

#### (2) 未知 flag 在旧代码中被忽略（Fail-open 实测）
```
> node scripts/verify-rot-budget.mjs --unknown-flag
Exit code: 0
> node scripts/verify-task-gate.mjs validate-policy --unknown-flag
Policy validation passed: <REPO_ROOT>\scripts\workflow-policy.json
Exit code: 0
```

#### (3) 注释禁令正则在旧代码中遗漏 `///` 和 `/*`
```
node -e "import { BANNED_COMMENT_REGEX } from './scripts/verify-rot-budget.mjs'; console.log('///:', BANNED_COMMENT_REGEX.test('/// allow-god-file')); console.log('/*:', BANNED_COMMENT_REGEX.test('/* allow-god-file'));"
///: false
/*: false
```

#### (4) deadSince 未来日期在旧代码中被放行
```
node -e "import { validateManifest } from './scripts/verify-rot-budget.mjs'; console.log(validateManifest({'god_file:src/ghost.rs': {kind: 'file-lines', ceiling: 500, deadSince: '2099-01-01'}}));"
{ success: true, errors: [] }
```

#### (5) task-gate 对含「后续单」且 BASE 节为 `## BASE` 标题的 brief 误报
```
Result on old code with ## BASE title and '后续单':
{
  "success": false,
  "errors": [
    "Brief mentions \"续单\" but lacks an independent BASE line and/or \"允许文件集\" section."
  ],
  "missingSections": []
}
```

---

### 2. 新代码 7 项验证命令执行证据

#### 验证 1：`node scripts/verify-rot-budget.mjs`
```
> node scripts/verify-rot-budget.mjs
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=82/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
Exit code: 0
```
零漂移核对：5 grep、3 dir (scripts=45/48)、9 god-file、1397 files、verdict: rotting。完全一致。

#### 验证 2：`node scripts/verify-rot-budget.mjs --selftest`
```
> node scripts/verify-rot-budget.mjs --selftest
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
Exit code: 0
```

#### 验证 3：`node scripts/verify-rot-budget.mjs --base 547c228` 与等号形态及区分性探针
##### 形态 1：空格分隔
```
> node scripts/verify-rot-budget.mjs --base 547c228
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=82/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
Exit code: 0
```
##### 形态 2：等号拆分
```
> node scripts/verify-rot-budget.mjs --base=547c228
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=82/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
Exit code: 0
```
##### 区分性探针（证明 `--base=...` 真实进入 base 管线校验）
```
> node scripts/verify-rot-budget.mjs --base=invalid-ref-xyz
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
Failed to retrieve or parse base manifest from git show invalid-ref-xyz:scripts/rot-budget.json: Command failed: git show invalid-ref-xyz:scripts/rot-budget.json
fatal: invalid object name 'invalid-ref-xyz'.

Failed to inspect base scripts directory from git ls-tree invalid-ref-xyz scripts/: Command failed: git ls-tree invalid-ref-xyz scripts/
fatal: Not a valid object name invalid-ref-xyz

Rot budget verification failed with 2 violation(s) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
Exit code: 1
```

#### 验证 4：`node scripts/verify-rot-budget.test.mjs`
```
> node scripts/verify-rot-budget.test.mjs
✔ compliant fixture exits 0 and reports success (112.6533ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (104.2288ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (104.3081ms)
✔ registered god-file exceeding ceiling fails (6.4346ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (7.1759ms)
✔ dir-entry-count compliant fixture passes (102.3705ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (101.5358ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (99.3976ms)
✔ tests.rs file is excluded from rot budget measurement (6.5447ms)
✔ *_tests directory files are excluded from rot budget measurement (6.5726ms)
✔ actual workspace rot budget passes with current manifest (419.5936ms)
✔ dead god-file registration warns but does not fail verification (99.4455ms)
✔ W18-6 F1.1: workflow policy missing rotVerdictRubric fails and reports violation (4.526ms)
✔ W18-6 F1.2: workflow policy with malformed rotVerdictRubric (missing key) fails (4.003ms)
✔ W18-6 F1.3: workflow policy rotVerdictRubric value mismatch with pinned literal fails (3.954ms)
✔ W18-6 F1.4: valid rubric with 0 findings passes and verdict is healthy (3.9436ms)
✔ W18-6 F1.5: 1 warning (bounded) passes with verdict stable (3.9052ms)
✔ W18-6 F1.6: >=3 findings results in verdict rotting (4.0898ms)
✔ W18-6 F1.7: unregistered file exceeding 800 lines results in verdict rotting (unbounded) (6.023ms)
✔ W18-6 F1.8: manifest validation early exit does not carry verdict field (2.6416ms)
✔ W18-6 F1.9: green path spawn stdout contains verdict segment (93.6542ms)
✔ W18-6 F2.1: dead registration without deadSince produces warning and passes (3.8141ms)
✔ W18-6 F2.2: dead registration with deadSince 31 days ago fails with violation (3.2164ms)
✔ W18-6 F2.3: dead registration with deadSince exactly 30 days ago produces warning and passes (3.1223ms)
✔ W18-6 F2.4: live file with deadSince ignores deadSince and emits no warning (5.0329ms)
✔ W18-6 F2.5: malformed deadSince is rejected by manifest validation (2.8858ms)
✔ W18-7 F1.1: fixture registry with matching hashes passes (5.7719ms)
✔ W18-7 F1.2: fixture registry with tampered file hash fails (5.4066ms)
✔ W18-7 F1.3: fixture registry pointing to missing file fails (4.519ms)
✔ W18-7 F1.4: malformed fixture registry fails (4.1359ms)
✔ W18-7 F1.5: fixture registry entry missing sha256 fails (4.9862ms)
✔ W18-7 F1.6: fixture registry missing or empty fixtures array fails (4.6836ms)
✔ W18-7 F1.7: missing fixture registry file skips attestation and passes (3.0672ms)
✔ W19-2 F1: parseArgs supports --flag=value and rejects unknown flags (0.5268ms)
✔ W19-2 F2: BANNED_COMMENT_REGEX matches /// and /* comments and preserves non-anchored string literals (0.0959ms)
✔ W19-2 F3: dangling exception lease produces warning and live lease produces no warning (5.6185ms)
✔ W19-2 F4: future deadSince date is rejected by manifest validation (0.9197ms)
✔ W19-2 F8: attestFixtureRegistry polishes invalid entry to <invalid entry> (4.06ms)
ℹ tests 38
ℹ suites 0
ℹ pass 38
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1367.9736
Exit code: 0
```

#### 验证 5：`node scripts/verify-task-gate.mjs --selftest`
```
> node scripts/verify-task-gate.mjs --selftest
[PASS] negative fixture a: replay W15-1l real incident (detected out-of-bounds pages_archive.rs)
[PASS] negative fixture b: invalid git revision rejected
[PASS] negative fixture c: missing required section in brief rejected
[PASS] negative fixture d: unapproved exemption phrase rejected
[PASS] negative fixture e: prejudging reviewer phrase in prose rejected
[PASS] negative fixture f: bad policy missing required field rejected
[PASS] negative fixture g: policy enum mismatch rejected
[PASS] negative fixture h: rename source path omitted from allowlist rejected
[PASS] positive fixture 1: complete 8-file allowlist passes
[PASS] positive fixture 2: allowlist with unfulfilled file passes with warning
[PASS] positive fixture 3: w16-1-brief.md passes validate-brief
[PASS] positive fixture 4: default workflow-policy.json passes validate-policy
[PASS] positive fixture 5: rename with both source and destination in allowlist passes
[PASS] negative fixture i: brief mentions 续单 without BASE line fails
[PASS] positive fixture 6: brief mentions 后续单 with ## BASE title passes
[PASS] positive fixture 7: parseArgs supports --flag=value, -h, and rejects unknown flags
Selftest passed: 16 fixtures passed (9 negative, 7 positive).
Exit code: 0
```

#### 验证 6：`node scripts/verify-task-gate.mjs validate-policy`
```
> node scripts/verify-task-gate.mjs validate-policy
Policy validation passed: <REPO_ROOT>\scripts\workflow-policy.json
Exit code: 0
```

#### 验证 7：`pnpm run check:repo-hygiene`
```
> pnpm run check:repo-hygiene

> northhing@0.2.10 check:repo-hygiene <REPO_ROOT>
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (5 content files scanned, 3872 filenames checked).
Exit code: 0
```

---

## 状态

DONE
