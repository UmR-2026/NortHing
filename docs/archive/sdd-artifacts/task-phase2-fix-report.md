# Task PHASE-2 Fix Report — Crate 准入守卫路径正则分界符修复 (M2)

## 1. 修复说明

- **根因分析 (M2)**：原 `scripts/core-boundaries/checker.mjs` 中 `pathPattern` 使用 `\b${escapeRegex(member)}\b`，因 JavaScript 正则将 `-` 视为非 word 字符，导致 `\bsrc/apps/desktop\b` 会误匹配 `src/apps/desktop-tauri` 等前缀相似的未登记路径。
- **修复方案**：将 `pathPattern` 改为基于 Markdown/路径分界符的前后断言：
  `new RegExp('(?<=[`\\s|()]|^)' + escapeRegex(member) + '(?=[/`\\s,;|()]|$)')`
  严密拦截 `-`、`_`、字母数字等后缀衍生的相似路径。
- **自测加固**：
  - `scripts/core-boundaries/self-test.mjs` 增加 `src/apps/desktop-unknown` 前缀碰撞拦截断言。
  - `scripts/check-core-boundaries.test.mjs` 增加 `crate admission guard rejects prefix-similar paths to prevent false positives` 单元测试。

## 2. 验证输出

### 2.1 `node scripts/check-core-boundaries.test.mjs`
```text
✔ core boundary check is split into focused modules (7.8973ms)
✔ split core boundary check keeps self-test and default execution behavior (1182.42ms)
✔ crate admission guard flags unregistered workspace member (22.9444ms)
✔ crate admission guard rejects prefix-similar paths to prevent false positives (0.886ms)
ℹ tests 4
ℹ suites 0
ℹ pass 4
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1220.2001
```

### 2.2 `node scripts/check-core-boundaries.mjs`
```text
Core boundary check passed.
```

### 2.3 `pnpm run check:rot`
```text
> northhing@0.2.10 check:rot E:\agent-project\.worktrees\northing-p2
> node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs

✔ compliant fixture exits 0 and reports success (115.1762ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (118.2775ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (110.1756ms)
✔ registered god-file exceeding ceiling fails (5.9646ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (6.8186ms)
✔ dir-entry-count compliant fixture passes (102.6575ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (113.7213ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (108.812ms)
✔ actual workspace rot budget passes with current manifest (394.7005ms)
ℹ tests 9
ℹ suites 0
ℹ pass 9
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1084.0591
Rot budget verification passed (5 grep rules [unwrap_production=502/511, expect_production=1092/1093, let_underscore=388/389, unix_epoch_inline=69/69, allow_dead_code=111/111], 3 dir rules [dir_entries:scripts=45/45, dir_entries:docs/design=1/3, dir_entries:.superpowers/sdd=375/400], 7 god-file rules checked across 1363 files).
```

### 2.4 `git diff --stat`
```text
 scripts/check-core-boundaries.test.mjs | 17 +++++++++++++++++
 scripts/core-boundaries/checker.mjs    |  4 ++--
 scripts/core-boundaries/self-test.mjs  | 16 ++++++++++++++++
 3 files changed, 35 insertions(+), 2 deletions(-)
```
