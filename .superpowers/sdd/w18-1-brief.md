# W18-1 Brief — validateManifest() fail-closed + D-3 红队 probe 首批

## 任务标识

W18-1（波次计划：`.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md` v2；相位映射 0.1 + D-3）

## BASE

`eaa592f`（main HEAD，工作树已验干净）

## 允许文件集

- `scripts/verify-rot-budget.mjs`（修改）
- `scripts/fixtures/rot-budget/bogus-kind.json`（新增）
- `scripts/fixtures/rot-budget/string-ceiling.json`（新增）
- `scripts/fixtures/rot-budget/path-escape.json`（新增）
- `scripts/fixtures/rot-budget/empty-pattern.json`（新增）
- `scripts/fixtures/rot-budget/unknown-field.json`（新增）

fixture 在 `scripts/fixtures/` 子目录，不占 `dir_entries:scripts` 额度（该规则只数顶层文件）。report 写 `.superpowers/sdd/w18-1-report.md`，不进本单验收 diff。

## 背景

`verifyRotBudget()` 解析 manifest 时对坏 entry 静默跳过（fail-open）：bogus kind 进不了任何规则桶被忽略、字符串 ceiling 直接进比较、空 pattern 让 `new RegExp` 抛异常或匹配一切、`entry.dir` 无路径 confinement 可指出 projectRoot。外部审查 5 反例实测揭开整个面（D 报告 §1.1/B-1）。本单把 manifest 校验改为 fail-closed。

## 功能要求

1. 新增导出函数 `validateManifest(manifest, projectRoot)`，返回 `{ success, errors }`（逐 entry 收集全部错误，不遇错即停）。`verifyRotBudget()` 在解析 manifest 后**最先调用**；有任何 error ⇒ 全部记入 violations、整体 nonzero 退出。
2. 校验规则（逐 entry）：
   - `kind` 必填，取值 ∈ {`grep-count`, `file-lines`, `dir-entry-count`}；
   - `ceiling` 必须为有限非负整数（`Number.isInteger` && `>= 0`）；
   - `grep-count` 的 `pattern` 必须为非空字符串且 `new RegExp(pattern)` 可编译；
   - `dir-entry-count` 的解析目录（`entry.dir` 或 key 去掉 `dir_entries:` 前缀推导）经归一化（`\`→`/`）+ `path.resolve(projectRoot, dir)` 后必须仍在 projectRoot 内（禁 `..` 越界、禁 projectRoot 外绝对路径）；
   - entry 未知字段拒绝：白名单按 kind 表驱动（`const FIELD_WHITELIST = { 'grep-count': [...], 'file-lines': [...], 'dir-entry-count': [...] }` 形态）。当前启用字段：公共 `kind`/`ceiling`/`note`；`grep-count` 加 `pattern`；`dir-entry-count` 加 `dir`。**表结构须容纳终态**（后续任务只往表里加 `exempt-list` kind 与 `paths`/`action`/`authorization`/`deadSince` 字段，不改校验逻辑结构——波次计划「目标 schema 前置」）；
   - `note` 若存在必须为字符串。
3. `--selftest`（内联在 `verify-rot-budget.mjs`，风格对齐 `verify-task-gate.mjs` 的 `runSelftest()`：record/PASS/FAIL、任一失败整体非零）：
   - **5 个负向 manifest fixture**（即允许文件集中的 5 个 JSON）：bogus kind / 字符串 ceiling / `../` 路径越界 / 空 pattern / 未知字段——各自调用 `validateManifest` 必须 `success: false` 且 error 文案指向对应问题；
   - **正向**：当前仓库 `scripts/rot-budget.json` 原样通过 `validateManifest`（编排者预检 + brief review 已逐 entry 核实 14 条全合规；若实现时发现不合规 ⇒ BLOCKED 上报，由编排者另开单修 manifest，本单禁改）；
   - **阈值边界（档 A）**：selftest 在临时目录构造合成 projectRoot（`src/` 下生成恰好 800 行与 801 行的两个 `.rs` 文件 + 空 manifest `{}`），调 `verifyRotBudget({ projectRoot: tmp, silent: true })`：800 行文件不触发 violation、801 行文件触发 god-file violation——证明阈值钉在边界上；
   - **集成钉死**：取 bogus-kind fixture 经 `verifyRotBudget({ projectRoot: 合成根, manifestPath: fixture路径, silent: true })` 断言 `success === false` 且 violations 含该 schema 错误（防「写了 validateManifest 但忘了接线」）；
   - **内联补两负例**（不新增文件）：`pattern: "["`（正则编译失败）⇒ 红；`note: 123`（非字符串）⇒ 红——钉住「可编译」与「note 类型」两条规则的验收。
4. 现有仓库 manifest 的校验不改变任何现有读数与判定：`node scripts/verify-rot-budget.mjs` 输出与 BASE 基线逐一对照一致（见「验证」节基线读数）。**测量条件**：`.superpowers/sdd` 读数按 committed 状态对照（`git stash -u` 后运行）；工作树含未跟踪 brief/report 造成的 +N 属环境漂移，report 注明即可，其余读数不得有任何漂移。
5. 纯 Node 标准库，零新依赖；脚本输出 English-only。
6. 顶层 config 块（无 `kind` 的对象 entry）由「kind 必填」规则拒绝，属预期行为（B-4 的 config 字段兑现归 W18-4，本单只管拒绝）。

## Constraints（逐字自波次计划 Global Constraints，与本单相关项）

- 任何 ceiling 不得上调；manifest 删指标视同上调（禁止）。**本单不改 `scripts/rot-budget.json`**。
- commit 逐文件点名 `git add`（禁 `-A`）；message 前缀 `feat(scripts):` + `(W18-1)` 后缀。
- 不得用 Markdown 说明替代机器校验；report 中验证命令贴原文输出 + exit code。
- 产物内本地绝对路径用 `<USERPROFILE>` 等占位（hygiene 闸）；收口前必跑 `node scripts/check-repo-hygiene.mjs`。
- report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
- 本单触碰 `metaRatchetPaths`（`scripts/verify-rot-budget.mjs`），验收走双 judge + 用户拍板车道（流程项，对实现者零额外要求）。
- checker 改动**先负向 fixture 后实现**：每单 `--selftest` 负例必须先在旧代码上确认行为、在新代码上全红，正向全绿（波次计划 GC4 原文）。report「验证」节须含「旧代码行为确认」：5 个负向 fixture manifest 经 BASE 版（`git stash` 或 `git show eaa592f:scripts/verify-rot-budget.mjs` 取旧版）跑过的输出原文——注意区分「错误原因的红」（如 path-escape 在旧代码以 directory-not-exists 红）与「真 fail-open 绿」（bogus kind / unknown field），逐 fixture 如实记录。

## 禁区

- 禁改 `scripts/rot-budget.json`、`scripts/workflow-policy.json` 及 `scripts/` 下任何其他文件（fixture 新增除外，仅限允许文件集列出的 5 个 JSON）。
- 禁改 grep 扫描范围、文件收集逻辑（`collectRustFiles`）、豁免清单语义——读数零漂移是硬约束。
- 禁为过关调整 fixture 判定标准。

## 验证

编排者已在 BASE `eaa592f` 预跑基线命令（已实证）：

1. `node scripts/verify-rot-budget.mjs` —— 基线绿，读数：`unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109`；`dir_entries:scripts=44/48, docs/design=1/1, .superpowers/sdd=63/400`；6 god-file rules；1368 files。修复后输出必须与基线逐一对照一致（贴出对照）。
2. `node scripts/verify-rot-budget.mjs --selftest` —— 本单交付物：负例全红、正例全绿、边界钉死，整体 exit 0。
3. `node scripts/verify-task-gate.mjs validate-policy` —— 绿（回归）。
4. `pnpm run check:repo-hygiene` —— 绿。
5. `node scripts/verify-rot-budget.test.mjs` —— 绿（CI rot-budget job 等价命令之一，编排者已在 BASE 实证 exit 0；回归约束：12 个现存测试与新校验规则全部兼容，不得有失败）。

## skill 前置

阅读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`，遵循其中与本任务相关的约定，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w18-1-report.md`，必含三节：**改动摘要**（validateManifest 规则清单 + selftest 结构）/ **验证**（验证节全部 5 条命令原文输出 + exit code + 旧代码行为确认记录，含基线读数对照表）/ **状态**（状态词结尾）。
