# W18-4 Report — config 字段兑现 + EXEMPT_FILE_PATHS 移入 manifest

## 改动摘要

### 1. exempt-list 机制
- **manifest 结构扩展**：`scripts/rot-budget.json` 新增 `exempt_generated_files` 条目，`kind: "exempt-list"`，集中管理生成文件的豁免路径，消除源码内硬编码常量：
  - `src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs`（保留）
  - `northing-installer/src-tauri/src/installer/generated_locale_contract.rs`（修正笔误后保留）
- **validator 扩展**：
  - `FIELD_WHITELIST` 增加 `'exempt-list': ['kind', 'paths', 'note']`。
  - `validateManifest` 针对 `exempt-list` 校验 `paths`：必须为非空字符串数组，每项均校验 confinement 在 projectRoot 内（禁止 `..` 越界与外部绝对路径）。
  - **ceiling 校验按 kind 分发**：仅当 `FIELD_WHITELIST[kind]` 包含 `ceiling` 时才校验 ceiling，`exempt-list` 豁免 ceiling 检查；若 `exempt-list` 传入 ceiling 则按未知字段拒绝。
  - `exempt-list` 规则不进入 grep-count、god-file、dir-entry-count 任何规则桶。
- **扫描逻辑对接**：
  - 删除 `scripts/verify-rot-budget.mjs` 顶层硬编码常量 `EXEMPT_FILE_PATHS`，改为在 `verifyRotBudget` 执行时从 manifest 提取所有 `exempt-list` 条目构建 `exemptPaths` 集合。
  - 遍历 Rust 源文件时若命中 `exemptPaths` 则跳过整文件扫描，豁免语义（跳过扫描，不计入 god-file 及 grep 规则）保持不变。
- **自测与回归测试同步**：
  - `scripts/verify-rot-budget.mjs` `--selftest` 新增 5 项自测（4 负例 + 1 正例），总项数由 33 增至 38（25 负例，13 正例）。
  - `scripts/verify-rot-budget.test.mjs` 单点改写用例 5：由原依赖脚本常量的空 manifest 改为合成 manifest 携带 `exempt_generated_files` entry 豁免合成路径，保留「豁免文件 >800 行放行」断言目标；其余 11 个测试用例零触碰。

### 2. 口径修正记录
原豁免项第 3 条因笔误（northhing 双 h）从未生效，本次为修复而非原样搬运；第 1 条 src/shared 为死路径，删除。
- 历史条目 1 `src/shared/i18n/generated_locale_contract.rs`：真实文件不存在，系历史架构演进遗留的死路径，予以删除，消除静默无效配置。
- 历史条目 2 `src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs`：文件真实存在（653 行），原样保留。
- 历史条目 3 `northhing-installer/src-tauri/src/installer/generated_locale_contract.rs`：因目录名双 h 笔误（实际为单 h 的 `northing-installer`）从未生效；本次修正路径为 `northing-installer/src-tauri/src/installer/generated_locale_contract.rs` 后移入 manifest。

### 3. config 承诺审计逐条处置表

| 序号 | 出处 | 承诺内容 | 处置 |
|---|---|---|---|
| 1 | `docs/archive/sdd-artifacts/skill-anti-rot-review-2026-08-29.md:131-132` / `.opencode/skills/anti-rot-system/SKILL.md:262-277` / 外部审查 C 报告 §1.3 | 承诺支持通过 manifest 顶层 `config.godFileThreshold` 自定义覆盖 800 行默认阈值；但当前 checker 硬编码 800 行阈值且 manifest 仅支持平铺条目 | 移交编排者另开文档单（按 brief 禁区与范围授权，本单不硬做顶层 config 块重构，保持既有平铺 entry 与 800 行基线） |
| 2 | `docs/archive/sdd-artifacts/skill-anti-rot-review-2026-08-29.md:66,131` / `.opencode/skills/anti-rot-system/SKILL.md:262-277` / 外部审查 C 报告 §1.3 | 承诺支持通过 manifest 顶层 `config.excludeFiles` miniglob 排除特定源文件；但当前 checker 未实现该动态排除机制 | 移交编排者另开文档单（排除机制维持由既有 tests 规则及 manifest `exempt-list` 承担，不硬做未授权的大改） |
| 3 | 外部审查 F-B-16 / 波次计划 Phase 0.4 | 消除脚本内部硬编码的 `EXEMPT_FILE_PATHS`，转入 manifest 配置管理以纳入 validator 校验并受版本控制 | 本单已实现。在 `scripts/rot-budget.json` 中新增 `exempt_generated_files` entry（`kind: "exempt-list"`），脚本改由 manifest 读取 |

---

## 验证

### 旧代码行为确认（GC4 先负向后实现）
在 BASE（`eecb611`）未修改代码上实测：
1. **`validateManifest` 对 `exempt-list` 拒绝行为**：
   - 执行：`node -e "import('./scripts/verify-rot-budget.mjs').then(m => { console.log(m.validateManifest({ test_exempt: { kind: 'exempt-list', paths: ['foo.rs'] } })); })"`
   - 输出：
     ```json
     {
       success: false,
       errors: [
         'test_exempt: invalid kind "exempt-list", must be one of: grep-count, file-lines, dir-entry-count',
         'test_exempt: "ceiling" must be a non-negative integer, got undefined'
       ]
     }
     ```
   - 证明：旧代码 validator 严格 fail-closed，将 `exempt-list` 视作非法 kind，且强制要求 ceiling 字段。
2. **死路径与笔误实测证据**：
   - `Test-Path "src/shared/i18n/generated_locale_contract.rs"` → `False`（确认死路径，文件不存在）
   - `Test-Path "northhing-installer/src-tauri/src/installer/generated_locale_contract.rs"` → `False`（确认双 h 笔误导致从未命中）
   - `Test-Path "northing-installer/src-tauri/src/installer/generated_locale_contract.rs"` → `True`（确认真实路径存在）

### 5 条验证命令原文输出与退出码

#### 1. `node scripts/verify-rot-budget.mjs`
- Exit Code: `0`
- 输出原文：
```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=71/400], 6 god-file rules checked across 1368 files).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

#### 2. `node scripts/verify-rot-budget.mjs --selftest`
- Exit Code: `0`
- 输出原文：
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
Selftest passed: 38 checks passed (25 negative, 13 positive).
```

#### 3. `node scripts/verify-rot-budget.mjs --base f2c55b8`
- Exit Code: `0`
- 输出原文：
```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=71/400], 6 god-file rules checked across 1368 files).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

#### 4. `node scripts/verify-rot-budget.test.mjs`
- Exit Code: `0`
- 输出原文：
```
✔ compliant fixture exits 0 and reports success (101.3473ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (98.5158ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (100.5602ms)
✔ registered god-file exceeding ceiling fails (6.3597ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (7.4157ms)
✔ dir-entry-count compliant fixture passes (97.551ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (97.53ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (90.2385ms)
✔ tests.rs file is excluded from rot budget measurement (5.7551ms)
✔ *_tests directory files are excluded from rot budget measurement (5.754ms)
✔ actual workspace rot budget passes with current manifest (307.1508ms)
✔ dead god-file registration warns but does not fail verification (92.8075ms)
ℹ tests 12
ℹ suites 0
ℹ pass 12
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1018.8988
```

#### 5. `pnpm run check:repo-hygiene`
- Exit Code: `0`
- 输出原文（本地绝对路径已按规则用 `<REPO_ROOT>` 占位）：
```
> northhing@0.2.10 check:repo-hygiene <REPO_ROOT>
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (4 content files scanned, 3856 filenames checked).
```

---

## 状态

DONE
