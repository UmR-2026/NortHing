# Task PHASE-2 Review — 棘轮扩展（4 新计数条目 + crate 准入守卫 + checker 读数输出）

- **Range reviewed**: `fe91147..f90b396`（1 commit, 10 files, +472/-3）
- **Worktree**: `E:\agent-project\.worktrees\northing-p2`（branch `feat/phase2-ratchet-0821`）
- **Role**: 独立验收（judge-m3）
- **Result**: **APPROVED** (0 Critical / 0 Important / 2 Minor)

---

## 1. SPEC 判决

| Spec | 要求 | 实测 | Verdict |
|---|---|---|---|
| 1 | checker 扩 `dir-entry-count` kind（目录缺失即违规）；通过输出带实测读数 | 实现于 `verify-rot-budget.mjs:152-176`；读数输出格式 `dir_entries:scripts=45/45, dir_entries:docs/design=1/3, dir_entries:.superpowers/sdd=374/400` 在 `:181-193` | ✅ |
| 2 | rot-budget.json 新增 4 条目（ceiling 逐字 = 111/45/3/400；note 写明语义） | 4 条目均在；ceiling 逐字匹配预检表；`sdd` 条目 note 含 `cap-and-archive`，其余 3 条 `only down` | ✅ |
| 3 | crate 准入守卫：workspace member 必须在 surfaces.md；自测含"未登记 fixture → 变红" | `checkCrateSurfaceRegistration` 实现于 `checker.mjs:418-465`，挂入 `runCoreBoundaryCheck()`；自测 3 处（`self-test.mjs` 1 绿 1 红、`check-core-boundaries.test.mjs` 1 红） | ✅ |
| 4 | surfaces.md 不漏成员；家规 7 在 AGENTS.md/CN 追加 sdd cap-and-archive 语义 | 25 workspace members 全部命中 surfaces.md（手验匹配规则）；AGENTS.md:101 与 AGENTS-CN.md:97 语义等价 | ✅ |
| 5 | 不碰 i18n/产品代码/既有 ceiling | diff 仅 10 文件，全在 scripts + AGENTS + SDD 工件 | ✅ |

### SPEC 约束逐字核验
- ✅ `allow_dead_code=111` / `scripts=45` / `docs/design=3` / `sdd=400` —— 全部命中
- ✅ 目录不存在 = 违规（手验：临时改 `dir_entries:scripts` 指向 `scripts/this_does_not_exist` → exit 1，stderr 含 `directory does not exist`）
- ✅ 通过输出带各指标实测读数（grep + dir 双段拼接）
- ✅ crate 准入：全部 25 members 命中；豁免数组 `surfacesExemptMembers = []` 带注释说明用途
- ✅ 自测含"未登记 crate fixture → 变红"用例
- ✅ C1（unwrap 清零）未在 diff 出现改动

---

## 2. QUALITY 判决

### 2.1 简单够用（最小可行）
- 直接用 `fs.readdirSync(d, { withFileTypes: true }).filter(e => e.isFile())`，无自定义递归或缓存
- crate 匹配用单个 `RegExp` + fallback `Cargo.toml` 解析，单层循环
- 通过输出拼接用 `[...].filter(Boolean).join(', ')` 三行，无模板字符串拼接
**Pass**

### 2.2 没重复造轮子
- 复用了 `parseWorkspaceMembers`（checker 已有）
- 复用了 `escapeRegex`（checker 已有）
- 复用了 `fs.existsSync` / `statSync` 模式
- 自测复用 `createFixtureDir` + `spawnSync` 既有 helper
- 没新增依赖
**Pass**

### 2.3 可读、可维护
- 命名一致（`dirRules`、`dirRelPath`、`checkCrateSurfaceRegistration`）
- 错误信息含定位路径 + 操作指引（如 "non-existent directory violates dir-entry-count guard"、"raising a ceiling requires user sign-off"）
- `surfacesExemptMembers` 注释明确豁免用途（"Non-product workspace members exempt ..."）
- 测试命名直接陈述断言
**Pass**

---

## 3. 编排者重点专查结论

### 3.1 dir-entry 口径不一致（编排者 pre-flag #1）
**实测**：
```
docs/design  →  files=1, subdirs=2, total entries=3  →  ceiling=3 (checker 读 1/3)
scripts      →  files=45, subdirs=3, total entries=48 →  ceiling=45 (checker 读 45/45)
.superpowers/sdd → files=374, subdirs=1, total=375    →  ceiling=400 (checker 读 374/400, cap-and-archive)
```

**判断**：
- 实施语义（files-only）合理——适合 "scripts 下文件数" 与 "sdd 工件数" 这两类指标，且与 `file-lines` 同口径（计量"看的见的文件"）。
- 但 `docs/design` ceiling=3 是按"全条目"算的（2 目录 + 1 文件），与 files-only 实施口径**不一致**：当前 1 文件，距 ceiling 还差 2 个文件才触发，留 2 文件 slack。
- `scripts` ceiling=45 巧合 = files-only 读数（45 文件），口径一致。
- `sdd` 是 cap-and-archive 语义，26 文件 slack 是**故意**留余量，note 已写明。

**修正建议**（写在报告即可，不打回——实施按 brief 字面值执行，brief 本身有歧义）：
1. 优先：rebase `dir_entries:docs/design` ceiling 至 `1`（匹配 files-only 实际读数，与 only-down 语义对齐）；
2. 或次选：checker 改用 `entries.length`（文件+目录）计数——但这会让 `scripts` 现状 48 直接超 ceiling 45，需同时 rebase 多条；
3. 或保留现状但在 note 里明示"ceiling 含历史目录条目；新文件按 only-down 计入"。

**报告缺失**：报告 §3 "目录计数口径" 写明 files-only，但**未指出 docs/design ceiling 与读数口径不一致**——这条应作为 Minor 留底，让编排者裁决。

**Severity**: Minor（实施按 brief 字面值执行；仅影响 docs/design 单条；非破坏性）

### 3.2 sdd cap 语义（编排者 pre-flag #2）
**实测**：worktree committed tracked files = 374（与报告读数一致）；主工作区 395 含未跟踪——属于其他并发任务产物，不在本任务 scope。

**判断**：
- `fs.readdirSync` 不区分 tracked/untracked，cap 实际拦截**磁盘上所有文件**。
- 主工作区 395 已含未跟踪工件 → 如运行 verify-rot-budget.mjs 会**立即失败**（395 > 400 仍未超，但 5 文件余量几乎耗尽；继续添加即破）。
- 这意味着 cap 的"防沉积"对**已 commit 的内容**有效，对**未跟踪内容**也有效（同一计数器），但**无法区分**——主工作区有未跟踪工件时，其他开发者加 1-2 个未跟踪文件就可能破 cap。
- 语义正确（count files on disk，与 cap-and-archive "触发归档" 语义一致），但读数差与 cap 余量较薄是真实的。
- 当前不是 bug，是设计选择；note 已写明语义。

**Severity**: Minor（已在报告交代语义，但未量化"主工作区 395 已逼近 cap 400"这条风险信号）

### 3.3 crate 准入匹配规则（编排者 pre-flag #3）
**实测**：25/25 members 命中（先尝试路径 backtick+word-boundary 匹配，不中则读 Cargo.toml 取 `name`，再去掉 `northhing-` 前缀试短名）。

**新增发现（手验）**：
路径正则 `\`${escapeRegex(member)}\`|\b${escapeRegex(member)}\b` 的**第二支** `\b...\b` 在 JS regex 中把 `-` 视为非 word 字符，导致 `\bsrc/apps/desktop\b` 会匹配 `src/apps/desktop-tauri`（因为 `t` 与 `-` 之间有 boundary）。
**具体场景**：若未来某 workspace member 路径是 `src/apps/desktop-experimental`（未登记），而 surfaces.md 已含 `src/apps/desktop` 或 `src/apps/desktop-tauri`——该未登记成员会被**假阳性通过**。

**当前 25 members 均无此 pattern**（手验 25 条全部走第一支 backtick 精确匹配路径通过），所以**今天不是 bug**。pkg name 分支用 backtick-only，无此漏洞。

**Severity**: Minor（未来陷阱；当前不触发；建议修复时把 `\b...\b` 改为 `(?=\`|[\s,;|]|$)` 要求后跟 backtick 或分隔符）

### 3.4 家规 7 同步
- AGENTS.md:101 —— "The `dir-entry-count` metric for `.superpowers/sdd` uses cap-and-archive semantics (triggers archiving rotation when full, rather than strictly decreasing)."
- AGENTS-CN.md:97 —— "dir-entry-count 指标的 sdd 条目是 cap-and-archive 语义（达到上限触发归档，而非只降不升）。"

**语义等价** ✅（同语义、同范围、同例外对象；translation 完整）

---

## 4. 独立验证（实跑命令 + 真实输出）

### 4.1 `node scripts/verify-rot-budget.mjs`
```text
Rot budget verification passed (5 grep rules [unwrap_production=502/511, expect_production=1092/1093, let_underscore=388/389, unix_epoch_inline=69/69, allow_dead_code=111/111], 3 dir rules [dir_entries:scripts=45/45, dir_entries:docs/design=1/3, dir_entries:.superpowers/sdd=374/400], 7 god-file rules checked across 1363 files).
```

### 4.2 `node scripts/verify-rot-budget.test.mjs`
```text
✔ compliant fixture exits 0 and reports success (118.3811ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (103.981ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (107.3543ms)
✔ registered god-file exceeding ceiling fails (6.7569ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (8.06ms)
✔ dir-entry-count compliant fixture passes (102.0005ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (99.2849ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (95.1882ms)
✔ actual workspace rot budget passes with current manifest (375.7627ms)
ℹ tests 9, pass 9, fail 0
```

### 4.3 `node scripts/check-core-boundaries.mjs` + test
```text
Core boundary check passed.
✔ core boundary check is split into focused modules (5.8661ms)
✔ split core boundary check keeps self-test and default execution behavior (1017.9535ms)
✔ crate admission guard flags unregistered workspace member (21.2657ms)
ℹ tests 3, pass 3, fail 0
```

### 4.4 `pnpm run check:rot`
```text
[9 tests pass] + Rot budget verification passed (5 grep rules [...], 3 dir rules [...], 7 god-file rules checked across 1363 files).
```

### 4.5 `cargo check --workspace` (stable-msvc)
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.46s
```
（含 19 warnings，与本任务无关——属既有 code base）

### 4.6 手动变红验证（临时改 manifest，验后还原）

| 探针 | 期望 | 实测 | 还原 |
|---|---|---|---|
| 改 `dir_entries:docs/design.ceiling` 至 0 | exit 1 + stderr 含 `exceeds ceiling 0` | ✅ exit 1, stderr: `dir_entries:docs/design: current 1 exceeds ceiling 0` | ✅ ceiling 还原至 3 |
| 改 `dir_entries:scripts.dir` 指向 `scripts/this_does_not_exist` | exit 1 + stderr 含 `directory does not exist` | ✅ exit 1, stderr: `dir_entries:scripts: directory does not exist at scripts/this_does_not_exist` | ✅ 还原 |
| 临时往 `Cargo.toml` 注入 `src/apps/fake_unregistered_fixture` member | exit 1 + stderr 含 `not registered in docs/status/surfaces.md` | ✅ exit 1, stderr: `workspace member crate "src/apps/fake_unregistered_fixture" is not registered in docs/status/surfaces.md — add entry to shipping, frozen, or capability table (House Rule 2)` | ✅ Cargo.toml 还原 |

### 4.7 `git diff --stat` 与 brief 一致
```text
 .superpowers/sdd/task-phase2-brief.md  |  56 +++
 .superpowers/sdd/task-phase2-report.md | 131 +++
 AGENTS-CN.md                           |   2 +-
 AGENTS.md                              |   2 +-
 scripts/check-core-boundaries.test.mjs |  17 +++
 scripts/core-boundaries/checker.mjs    |  56 +++
 scripts/core-boundaries/self-test.mjs  |  33 +++
 scripts/rot-budget.json                |  21 +++
 scripts/verify-rot-budget.mjs          |  45 ++++-
 scripts/verify-rot-budget.test.mjs     | 112 ++++++
 10 files changed, 472 insertions(+), 3 deletions(-)
```

---

## 5. Findings 汇总

| # | Severity | 位置 | 描述 | 修复建议 |
|---|---|---|---|---|
| 1 | Minor | `scripts/rot-budget.json` `dir_entries:docs/design` ceiling=3 | brief baseline 3 含 2 dirs + 1 file，但实施语义是 files-only → 当前 1/3，2 文件 slack，与 "only-down" 语义不完全对齐 | 优先级：rebase ceiling 至 1（最严）或改 checker 计数为 entries.length（含子目录） |
| 2 | Minor | `scripts/core-boundaries/checker.mjs:440` 正则 `\b...\b` 替代支 | word boundary 把 `-` 当非 word，可能让 `\bsrc/apps/desktop\b` 假阳性命中 `src/apps/desktop-tauri`；当前 25 members 不触发，但未来新成员有前缀模式时会被钻空子 | 把 `\b${escapeRegex(member)}\b` 改为 `(?=\`\|[\\s,\|;]|$)` 要求后跟 backtick 或分隔符 |
| 3 | Minor | 报告 §3（已写明 files-only）但未量化风险 | 主工作区 sdd 395（含未跟踪）距 cap 400 仅 5 文件余量；任何继续添加未跟踪文件即可能破 cap | 在报告或 manifest note 加"主工作区含未跟踪文件可能逼近 cap"的提示 |

**Critical: 0**
**Important: 0**
**Minor: 3**（均不阻塞合入，可在后续 ledger 跟踪或下轮清理）

---

## 6. 一句话结论

**APPROVED** — 实施正确落地 spec 1-5，自测覆盖合规+违规双路，AGENTS 双语同步语义等价，所有 25 workspace members 经 surfaces.md 路径精确匹配通过；3 条 Minor 均为 brief 继承的口径歧义或未来陷阱，不影响本次门禁。
