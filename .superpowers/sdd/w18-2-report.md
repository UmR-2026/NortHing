# W18-2 Report — only-down 机械化 + headroom floor + cap-and-archive action

## 改动摘要

1. **`--base <sha>` 对比算法**：
   - CLI 新增 `--base` flag（通过 `parseArgs` 解析）。无 `--base` 时跳过对比逻辑，保持快速本地检查语义。
   - 有 `--base` 时：使用 `execFileSync('git', ['show', `${base}:scripts/rot-budget.json`])` 获取并解析 BASE manifest。
   - **禁删指标**：逐项检查 BASE 中的 key，凡 BASE 存在而 TIP 缺失的 key，产生 violation（禁止删指标）。
   - **only-down**：当 TIP ceiling > BASE ceiling 时，检查该 entry 是否携带 live 的结构化授权 `authorization: { reason, commit, expires }`。`expires` 按 UTC 日期粒度比较（`expires >= 今日(UTC)` 为 live），过期或缺失均产生 violation。

2. **headroom floor 公式（-1.5，`--base` 模式）**：
   - 针对 `file-lines` 类 entry，当 ceiling 下调（TIP ceiling < BASE ceiling）时，校验 `newCeiling >= current + max(5, ceil(current * 0.05))`（`current` 为本次实测行数）。
   - 不足 floor 要求时产生 violation，并附文案指引走 exception lease 通道（W18-3 落地）。

3. **action 与 authorization 结构化校验（0.5）**：
   - `FIELD_WHITELIST` 表驱动扩展：`COMMON_FIELDS` 增加 `authorization`；`dir-entry-count` 增加 `action`。
   - `action` 语义校验：必须为 object 且 `type` 为非空字符串；当 `type === "cap-and-archive"` 时必须包含非空字符串 `archiveTo`。
   - `authorization` 语义校验：必须为 object，`reason`、`commit`、`expires` 均为非空字符串，且 `expires` 必须匹配 `YYYY-MM-DD`。
   - `scripts/rot-budget.json` 的 `dir_entries:.superpowers/sdd` entry 增加 `"action": {"type": "cap-and-archive", "archiveTo": "docs/archive/sdd-artifacts/"}`。
   - `dir-entry-count` 触发超限且 entry 配置了 `cap-and-archive` action 时，输出包含 `archiveTo` 路径的专用归档指引文案。

4. **零余量 warning 机制**：
   - 在有/无 `--base` 模式下均执行。
   - 遍历 manifest 登记项，实测 `current == ceiling` 时追加 warning，列出当前值与 ceiling 并指引走 exception lease 通道（warning 不影响 exit code 与验证成功判定）。

## 验证

### 旧代码行为确认记录

- **旧代码对 `--base` flag 的行为**：旧代码未解析 `--base`，执行时静默忽略该参数，仅执行无基线对比的常规扫描。因此在旧代码下：
  - 未授权上调 ceiling 不会被拦截（只要实测读数不超过新 ceiling 即通过）；
  - 授权已过期上调 ceiling 不会被拦截；
  - 违反 headroom floor 的下调 ceiling 不会被拦截；
  - manifest 删除已登记指标不会被拦截。
- **旧代码对 `action` 与 `authorization` 的行为**：旧代码的 `FIELD_WHITELIST` 不含这两个字段，旧代码 `validateManifest` 会直接报告 `unknown field "action"` 或 `unknown field "authorization"`，无法支持结构化语义验证（例如不能校验 `action.archiveTo` 缺失或 `authorization.expires` 格式错误）。
- **新代码生效确认**：新代码在 `--selftest` 内联合成 git 仓库负例及 inline 负例中，对上述 6 类违规场景全部精确拦截（全红），正向全绿。

---

### 零余量 warning 真实输出清单

在当前仓库上运行，实测触发零余量 warning 的 5 项登记条目：
```text
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

---

### 验证命令输出及 Exit Code

#### 命令 1：`node scripts/verify-rot-budget.mjs`

在工作树含未跟踪 brief 时，`.superpowers/sdd` 实测 66/400；通过 `git stash -u -- .superpowers/sdd/` 暂存未跟踪文件后测量 committed 口径，读数严格保持 65/400（零漂移）：

```text
$ node scripts/verify-rot-budget.mjs
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=44/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=65/400], 6 god-file rules checked across 1368 files).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
exit: 0
```
- **Exit Code**: 0
- **读数对照**：
  - grep rules: `unwrap_production=483/502`, `expect_production=940/1089`, `let_underscore=370/388`, `unix_epoch_inline=69/69`, `allow_dead_code=104/109`（5 项完全对齐）
  - dir rules: `dir_entries:scripts=44/48`, `dir_entries:docs/design=1/1`, `dir_entries:.superpowers/sdd=65/400`（committed 口径 65/400）
  - god files: 6 god-file rules，checked across 1368 files

#### 命令 2：`node scripts/verify-rot-budget.mjs --selftest`

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
Selftest passed: 23 checks passed (15 negative, 8 positive).
exit: 0
```
- **Exit Code**: 0

#### 命令 3：`node scripts/verify-rot-budget.mjs --base eaa592f`

```text
$ node scripts/verify-rot-budget.mjs --base eaa592f
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=44/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=66/400], 6 god-file rules checked across 1368 files).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
exit: 0
```
- **Exit Code**: 0

#### 命令 4：`node scripts/verify-rot-budget.test.mjs`

```text
$ node scripts/verify-rot-budget.test.mjs
✔ compliant fixture exits 0 and reports success (100.3697ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (98.9592ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (101.0829ms)
✔ registered god-file exceeding ceiling fails (6.1685ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (6.7736ms)
✔ dir-entry-count compliant fixture passes (97.7818ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (102.5749ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (97.7389ms)
✔ tests.rs file is excluded from rot budget measurement (7.3662ms)
✔ *_tests directory files are excluded from rot budget measurement (7.0482ms)
✔ actual workspace rot budget passes with current manifest (309.9525ms)
✔ dead god-file registration warns but does not fail verification (109.6507ms)
ℹ tests 12
ℹ suites 0
ℹ pass 12
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1053.8847
exit: 0
```
- **Exit Code**: 0

#### 命令 5：`pnpm run check:repo-hygiene`

```text
$ pnpm run check:repo-hygiene
> northhing@0.2.10 check:repo-hygiene
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (3 content files scanned, 3851 filenames checked).
exit: 0
```
- **Exit Code**: 0

## 状态

DONE
