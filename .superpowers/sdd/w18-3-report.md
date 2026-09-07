# W18-3 Implementation Report — >1000 硬边界 exception lease + allow-god-file 禁令机械化

## 改动摘要

1. **Lease Schema 与校验 (`scripts/exception-leases.json`)**
   - 新增 `scripts/exception-leases.json`，初始内容为 `[]`（空 lease 集，首例由 W18-5 登记）。
   - 数据结构选择**对象数组**（`Array of objects`）：结构清晰直观、天然支持按条目扩展与 git diff 审查。
   - 每条 lease 包含 5 个必需字段：
     - `file`：非空字符串，仓库相对路径，归一化 `\`→`/`，校验不得越界逃逸项目根目录。
     - `owner`：非空字符串。
     - `reason`：非空字符串。
     - `revisit_after`：`YYYY-MM-DD` 格式日期字符串。
     - `next_action`：非空字符串。
   - 校验规则：支持未知字段白名单过滤（`LEASE_FIELD_WHITELIST`）、文件路径重复检查（`duplicate lease for file`）；文件不存在为合法态（无 lease），文件存在但 JSON 解析错误/非数组/字段缺失/类型非法均为 violation（fail-closed）。

2. **Live 判定 (`isLeaseLive`)**
   - 判定标准：`lease.revisit_after >= todayUtc`（同 `isAuthorizationLive` 的 UTC 日期粒度与语义）。

3. **>1000 硬边界规则**
   - 在 `verifyRotBudget` 的生产文件扫描循环中（跳过 `EXEMPT_FILE_PATHS`）：对任何 `lineCount > 1000` 的 `.rs` 文件，必须持有关联的 live lease。
   - 若无 lease：报 violation 并输出 lease 文件路径及必填字段指引；
   - 若 lease 已过期（`revisit_after < todayUtc`）：报 violation 并输出过期日期与续签/拆分指引。
   - 保持 `EXEMPT_FILE_PATHS` 既有豁免语义（`verify-rot-budget.test.mjs` 中的 1200 行豁免文件绿案保持全绿）。

4. **零余量 Lease 通道**
   - 针对 `file-lines` 登记项：若当前读数等于 ceiling（`current === entry.ceiling`）且存在 live lease，则抑制零余量 warning；若无 live lease 或过期则正常输出 warning。
   - `grep-count` 与 `dir-entry-count` 保持既有 warning 逻辑不受 lease 影响。

5. **allow-god-file 注释禁令机械化 (O-2)**
   - 扫描生产 `.rs` 文件内容若含 `allow-god-file`，触发 violation，指引转用 `scripts/exception-leases.json`。

6. **Floor 旁路收口 (W18-2 Minor-2)**
   - 在 `--base` 模式下，若 `file-lines` 登记项存在 dead registration（文件在磁盘不存在）且 ceiling 发生下调（`rule.ceiling < baseCeiling`），由原仅 warning 收口为直接报 violation，提示使用 exception lease 或走正式指标退役流程。

7. **顺手清配额 (House rule 1, W18-2 Minor-6)**
   - `scripts/verify-rot-budget.mjs` L248（原 L248 中文注释改英文 `Metric deletion prohibition...`）与原 L589（中文「档 A」改英文 `Tier A`）。全文件 CJK 字符清零（0 匹配）。

8. **Selftest 扩展**
   - 新增 10 项 selftest（6 项负例 + 4 项正例），总数由 23 项扩至 33 项（21 negative, 12 positive），全部通过：
     - 负例 24：无 lease 的 1001 行文件（红）
     - 负例 25：过期 lease 的 1001 行文件（红）
     - 负例 26：`revisit_after` = 昨日 UTC（红，日期边界）
     - 负例 27：含 `allow-god-file` 注释的文件（红）
     - 负例 28：畸形 lease 文件（缺字段 + 损坏 JSON 双分支校验，红）
     - 负例 29：dead registration + ceiling 下调并存（红，`--base` 模式合成）
     - 正例 30：恰好 1000 行（绿，阈值边界 Tier A）
     - 正例 31：live lease 覆盖的 1001 行（绿）
     - 正例 32：`revisit_after` = 今日 UTC（绿，日期边界）
     - 正例 33：零余量登记项持 live lease 时 warning 消失（绿）

---

## 验证

### 旧代码行为确认记录 (GC4)

在 BASE `febfc45` 上对旧代码行为进行了实证确认：
1. **未登记 >1000 行文件**：旧代码因 `lineCount > GOD_FILE_LINE_THRESHOLD (800)` 已会报红（触发 `current 1001 exceeds ceiling 800`），但旧代码不存在 >1000 硬边界规则，未提供 exception lease 机制与字段指引。
2. **已登记 >1000 行文件**：在旧代码中，若 manifest 登记 `ceiling: 1001` 且文件 1001 行，旧代码直接通过（`success: true, violations: []`），对已登记 god-file 突破 1000 行完全「无闸」。新代码在无 live lease 时判定为 violation。
3. **`allow-god-file` 注释**：旧代码对文件内容中的 `// allow-god-file` 无探测，直接通过（`success: true`）。新代码判定为 violation。
4. **Dead registration + ceiling 下调**：在 `--base` 模式下，旧代码对缺失文件仅输出 `warn: ... dead registration`，返回值仍为 `success: true, violations: []`。新代码收口为 violation。
5. **Exception lease 文件**：旧代码不感知 `scripts/exception-leases.json`。新代码对其实现 fail-closed 结构与必填字段校验。

---

### 5 条交付验证命令原文输出与 Exit Code

#### 1. `node scripts/verify-rot-budget.mjs`
- **Exit code**: `0`
- **输出**:
```text
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=68/400], 6 god-file rules checked across 1368 files).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```
*注：读数与基线完全一致零漂移（483/940/370/69/104；1/1；68/400；6 god-file rules；1368 files；5 项 zero headroom warnings）。`dir_entries:scripts` 读数由 44→45/48，系本单新增 `scripts/exception-leases.json` 预期 +1，在 2026-09-05 用户拍板 42→48 额度内。*

#### 2. `node scripts/verify-rot-budget.mjs --selftest`
- **Exit code**: `0`
- **输出**:
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
Selftest passed: 33 checks passed (21 negative, 12 positive).
```

#### 3. `node scripts/verify-rot-budget.mjs --base f2c55b8`
- **Exit code**: `0`
- **输出**:
```text
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=68/400], 6 god-file rules checked across 1368 files).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

#### 4. `node scripts/verify-rot-budget.test.mjs`
- **Exit code**: `0`
- **输出**:
```text
✔ compliant fixture exits 0 and reports success (93.3014ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (89.0147ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (97.2313ms)
✔ registered god-file exceeding ceiling fails (5.0589ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (6.0081ms)
✔ dir-entry-count compliant fixture passes (93.0023ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (106.1378ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (99.4804ms)
✔ tests.rs file is excluded from rot budget measurement (5.7503ms)
✔ *_tests directory files are excluded from rot budget measurement (5.6332ms)
✔ actual workspace rot budget passes with current manifest (299.3874ms)
✔ dead god-file registration warns but does not fail verification (102.7566ms)
ℹ tests 12
ℹ suites 0
ℹ pass 12
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1010.2086
```

#### 5. `pnpm run check:repo-hygiene`
- **Exit code**: `0`
- **输出**:
```text
> northhing@0.2.10 check:repo-hygiene <REPO_ROOT>
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (3 content files scanned, 3854 filenames checked).
```

---

## 编排者收口建议

建议 commit 命令（逐文件点名 `git add`，禁止 `-A`）：
```bash
git add scripts/verify-rot-budget.mjs scripts/exception-leases.json
git commit -m "feat(scripts): add exception leases for >1000 god-files and enforce comment ban (W18-3)

- Land scripts/exception-leases.json (empty initial set, dir_entries:scripts 44->45/48 within user-approved 42->48 budget)
- Implement >1000 lines hard boundary check requiring live exception leases
- Suppress zero-headroom warning for file-lines entries holding live leases
- Prohibit allow-god-file comment in production .rs files
- Turn dead registration + ceiling lowering under --base into hard violation
- Clean up CJK comments in verify-rot-budget.mjs
- Add 10 new selftests (33 total, all green)"
```

---

DONE
