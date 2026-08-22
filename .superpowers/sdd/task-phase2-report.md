# Task PHASE-2 Report — 棘轮扩展（4 新计数条目 + crate 准入守卫 + checker 读数输出）

## 1. Spec 逐条落实

- **Spec 1: checker 扩展 (`scripts/verify-rot-budget.mjs`)**
  - 新增 `dir-entry-count` kind：统计指定目录（相对仓库根路径，支持 `.` 开头的隐藏目录）顶层常规文件数量；目录不存在或非目录均判定为违规并阻断。
  - 通过输出行追加实测读数（如 `5 grep rules [unwrap_production=502/511, expect_production=1092/1093, let_underscore=388/389, unix_epoch_inline=69/69, allow_dead_code=111/111], 3 dir rules [dir_entries:scripts=45/45, dir_entries:docs/design=1/3, dir_entries:.superpowers/sdd=372/400]`），提供精确的拧紧防呆口径。
- **Spec 2: `rot-budget.json` 新增 4 条目**
  - `allow_dead_code`: grep-count (`allow\(dead_code\)`), ceiling 111, note: `R-13 allow(dead_code) ratchet: only down`
  - `dir_entries:scripts`: dir-entry-count, ceiling 45, note: `directory top-level files ratchet: only down`
  - `dir_entries:docs/design`: dir-entry-count, ceiling 3, note: `directory top-level files ratchet: only down`
  - `dir_entries:.superpowers/sdd`: dir-entry-count, ceiling 400, note: `directory top-level files cap-and-archive: cap triggers archive rotation (not only-down)`
- **Spec 3: crate 准入守卫**
  - 在 `scripts/core-boundaries/checker.mjs` 中实现 `checkCrateSurfaceRegistration` 并接入主检查 `runCoreBoundaryCheck()` 与自测 `runManifestParserSelfTest()`。
  - 支持 `surfacesExemptMembers` 豁免列表常量带注释。
  - 自测覆盖：`self-test.mjs` 与 `check-core-boundaries.test.mjs` 均加入构造未登记 crate fixture 导致变红的断言用例。
- **Spec 4: 文档同步**
  - 核验 `docs/status/surfaces.md` 已完整覆盖当前 25 个 workspace members，无遗漏。
  - 在 `AGENTS.md` 与 `AGENTS-CN.md` 家规 7 同步追加 `dir-entry-count` 中 `.superpowers/sdd` 的 cap-and-archive 语义说明。
- **Spec 5: 边界防线**
  - 未触碰 i18n 冻结区、产品代码逻辑及既有 ceiling 数值。

## 2. 复用侦察（Mandatory Reconnaissance）

- `scripts/verify-rot-budget.mjs` & `verify-rot-budget.test.mjs`：
  - 复用了 manifest 解析循环、line/rule count 结构、`collectRustFiles` 豁免列表模式、fixture 目录构建与 `spawnSync` 断言模式。
- `scripts/core-boundaries/` 规则族：
  - 复用了 `parseWorkspaceMembers` 解析器与 `ROOT` / `repoPathToFsPath` 路径规范化机制；
  - 复用了 `self-test.mjs` 的 in-memory fixture 检查与 `check-core-boundaries.test.mjs` 的 node:test 异步模块加载断言风格。

## 3. 两个"写明"点

1. **目录计数口径 (Directory entry count criteria)**:
   - 统计目标目录下直接包含的**顶层常规文件数**（`fs.readdirSync(targetDir, { withFileTypes: true }).filter(e => e.isFile()).length`），不递归统计子目录内文件，也不将子目录本身计入文件数。
   - 若目标路径不存在或不是有效目录，直接判定违规失败。
2. **crate 准入匹配规则 (Crate admission matching rule)**:
   - 遍历 `Cargo.toml` 的 `workspace.members` 路径（过滤 `surfacesExemptMembers` 豁免项）。
   - 对每个 member 路径，优先检查 `docs/status/surfaces.md` 中是否存在该路径的行内代码引用（如 `` `src/apps/desktop` ``）或整词匹配；
   - 若未直接匹配路径，则读取该 crate 的 `Cargo.toml` 获取 `package.name`（如 `northhing-core-types` 及短名 `core-types`），检查 `docs/status/surfaces.md` 中是否存在该 crate 名称的引用。
   - 两者均未命中时报告违规。

## 4. 验证命令与输出

### 4.1 `node scripts/verify-rot-budget.mjs`
```text
Rot budget verification passed (5 grep rules [unwrap_production=502/511, expect_production=1092/1093, let_underscore=388/389, unix_epoch_inline=69/69, allow_dead_code=111/111], 3 dir rules [dir_entries:scripts=45/45, dir_entries:docs/design=1/3, dir_entries:.superpowers/sdd=372/400], 7 god-file rules checked across 1362 files).
```

### 4.2 `node scripts/verify-rot-budget.test.mjs`
```text
✔ compliant fixture exits 0 and reports success (118.9657ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (106.2628ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (105.6508ms)
✔ registered god-file exceeding ceiling fails (6.0571ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (8.8572ms)
✔ dir-entry-count compliant fixture passes (110.4359ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (103.0007ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (91.4931ms)
✔ actual workspace rot budget passes with current manifest (375.7328ms)
ℹ tests 9
ℹ suites 0
ℹ pass 9
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1035.3096
```

### 4.3 `node scripts/check-core-boundaries.mjs` & test
```text
Core boundary check passed.
✔ core boundary check is split into focused modules (6.6882ms)
✔ split core boundary check keeps self-test and default execution behavior (1020.4304ms)
✔ crate admission guard flags unregistered workspace member (21.0895ms)
ℹ tests 3
ℹ suites 0
ℹ pass 3
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1054.1101
```

### 4.4 `pnpm run check:rot`
```text
> northhing@0.2.10 check:rot E:\agent-project\.worktrees\northing-p2
> node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs

✔ compliant fixture exits 0 and reports success (121.4279ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (117.3484ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (122.2304ms)
✔ registered god-file exceeding ceiling fails (6.4148ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (7.1625ms)
✔ dir-entry-count compliant fixture passes (107.3271ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (105.8742ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (89.6923ms)
✔ actual workspace rot budget passes with current manifest (360.4254ms)
ℹ tests 9
ℹ suites 0
ℹ pass 9
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1046.4009
Rot budget verification passed (5 grep rules [unwrap_production=502/511, expect_production=1092/1093, let_underscore=388/389, unix_epoch_inline=69/69, allow_dead_code=111/111], 3 dir rules [dir_entries:scripts=45/45, dir_entries:docs/design=1/3, dir_entries:.superpowers/sdd=372/400], 7 god-file rules checked across 1362 files).
```

### 4.5 `cargo check --workspace` (stable-msvc)
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 47s
```

### 4.6 `git diff --stat`
```text
 AGENTS-CN.md                           |   2 +-
 AGENTS.md                              |   2 +-
 scripts/check-core-boundaries.test.mjs |  17 +++++
 scripts/core-boundaries/checker.mjs    |  56 +++++++++++++++++
 scripts/core-boundaries/self-test.mjs  |  33 ++++++++++
 scripts/rot-budget.json                |  21 +++++++
 scripts/verify-rot-budget.mjs          |  45 ++++++++++++-
 scripts/verify-rot-budget.test.mjs     | 112 +++++++++++++++++++++++++++++++++
 8 files changed, 285 insertions(+), 3 deletions(-)
```

## 5. 偏离声明

- 零偏离（Zero deviations from brief）。
