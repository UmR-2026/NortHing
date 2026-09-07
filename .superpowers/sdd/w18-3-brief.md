# W18-3 Brief — >1000 硬边界 exception lease + allow-god-file 禁令机械化

## 任务标识

W18-3（波次计划：`.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md` v2；相位映射 0.3 + O-2 errata 机械化）。前置：W18-1（validateManifest/FIELD_WHITELIST）、W18-2（`--base` 模式 / authorization / floor / 零余量 warning）已落地。**本单是 W18-5 的硬前置**（lease 机制未落地，W18-5 不得扩扫描范围——D07）。

## BASE

`febfc45`（main HEAD，工作树已验干净）

## 允许文件集

- `scripts/verify-rot-budget.mjs`（修改）
- `scripts/exception-leases.json`（新增；初始内容 = 空 lease 集，本单不登记真实 lease，首例由 W18-5 登记；`dir_entries:scripts` 44→45/48，commit message 记账：额度属用户 2026-09-05 拍板 42→48 之内）

report 写 `.superpowers/sdd/w18-3-report.md`，不进本单验收 diff。本单不新增 fixture 文件（>1000/lease/注释类用例全部 selftest 内联 tmpdir 合成）。

## 背景

>1000 行目前靠「未登记 >800 即 violation」间接挡，登记项 ceiling>1000 则无闸；PHASE-0 已裁定删除 allow-god-file 头注释方案（注释自身腐化），但无机器防复活。本单落地 exception lease（handoff §5.7 / D 报告 0.3），并把注释禁令机械化。

## 功能要求

1. **exception lease 文件**：`scripts/exception-leases.json`，结构为对象数组（或对象映射，实现者二选一并在 report 说明），每条 lease 字段：`file`（非空字符串，仓库相对路径，归一化 `\`→`/`）、`owner`（非空字符串）、`reason`（非空字符串）、`revisit_after`（`YYYY-MM-DD`）、`next_action`（非空字符串）。文件不存在 = 无 lease（合法态）；文件存在但 JSON 解析失败/结构非法/字段缺失 ⇒ violation（fail-closed）。live 判定：`revisit_after >= 今日(UTC)`（与 W18-2 authorization 同语义同粒度）。
2. **>1000 硬边界**：任何 file-lines 读数 > 1000 ⇒ violation，**除非**存在该文件的 live lease；lease 过期 ⇒ violation（不自动续）。已登记 manifest 的 god-file 也不例外（ceiling>1000 的登记同样须持 live lease）。violation 文案含 lease 文件名与字段指引。**检查在既有扫描循环内对非豁免文件执行，`EXEMPT_FILE_PATHS` 豁免语义不变——`verify-rot-budget.test.mjs` 的 1200 行豁免文件绿案（L172-192）必须保持绿**。
3. **零余量 lease 通道**（承接 W18-2 warning）：file-lines 类登记 entry 有 live lease 时（lease 按 `file` 路径匹配），W18-2 的零余量 warning 不再输出该条。仅适用 file-lines 类（grep-count / dir-entry-count 的零余量 warning 不受 lease 影响）。
4. **allow-god-file 注释禁令机械化**（O-2）：扫描到的生产 `.rs` 文件内容含 `allow-god-file` ⇒ violation（防注释方案复活；该注释在仓库现状为零，本单落地探测）。
5. **floor 旁路收口**（承接 W18-2 53-judge Minor-2）：`--base` 模式下，`file-lines` 登记项「文件缺失（dead registration）且 ceiling 下调」并存 ⇒ violation（不再仅 warning），文案指引进 lease 或正式退役流程。
6. **顺手清配额**（家规 1，W18-2 53-judge Minor-6）：`verify-rot-budget.mjs` 现存两处中文注释改英文——L248 与 L589（全文件仅此两处含 CJK，L589 为「档 A」字样）。
7. **selftest 扩展**（内联沿用既有结构，tmpdir 合成）：
   - 负例：无 lease 的 1001 行文件（红）/ 过期 lease（红）/ `revisit_after` = 昨日（红，日期边界）/ 含 `allow-god-file` 注释的文件（红）/ 畸形 lease 文件（红，JSON 坏或缺字段）/ dead registration + ceiling 下调并存（红，`--base` 合成）；
   - 正例：恰好 1000 行（绿，阈值边界档 A）/ live lease 覆盖的 1001 行（绿）/ `revisit_after` = 今日 UTC（绿，日期边界）/ 零余量登记项有 live lease ⇒ warning 消失（绿）。
   - 既有 23 项 selftest 必须保持全绿。
8. 纯 Node 标准库，零新依赖；输出 English-only。

## Constraints（逐字自波次计划 Global Constraints，与本单相关项）

- 任何 ceiling 不得上调；manifest 删指标视同上调（禁止）。**本单不改 `scripts/rot-budget.json`**。
- 禁止复活 `allow-god-file` 头注释方案；>1000 只能走 exception lease（O-2 errata）。
- commit 逐文件点名 `git add`（禁 `-A`）；message 前缀 `feat(scripts):` + `(W18-3)` 后缀 + scripts 额度记账说明。
- 不得用 Markdown 说明替代机器校验；report 中验证命令贴原文输出 + exit code。
- 产物内本地绝对路径用 `<USERPROFILE>` 等占位（hygiene 闸）；收口前必跑 `node scripts/check-repo-hygiene.mjs`。
- report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
- 本单触碰 `metaRatchetPaths`（`scripts/verify-rot-budget.mjs`），验收走双 judge 车道（用户已拍板代码级批准委托 judge 流程，2026-09-07）。
- checker 改动**先负向 fixture 后实现**（GC4）：负例必须先在旧代码上确认行为、在新代码上全红，正向全绿。report「旧代码行为确认」节如实记录：旧代码对 >1000 未登记文件的行为（现有 >800 规则已红——如实说明硬边界的既有覆盖与本单新增的 lease 通道/注释禁令/登记项>1000 须 lease 三个增量面）。

## 禁区

- 禁改 `scripts/rot-budget.json`、`scripts/workflow-policy.json`、`scripts/verify-task-gate.mjs`、`scripts/fixtures/` 及 `scripts/` 下任何其他文件。
- 禁改 grep 扫描范围/收集器/豁免语义（读数零漂移）。
- 禁为过关调整 fixture 判定标准。

## 验证

编排者已在 BASE `febfc45` 预跑基线命令（已实证绿）：`node scripts/verify-rot-budget.mjs`、`--selftest`（23 绿）、`node scripts/verify-rot-budget.test.mjs`（12 绿）、`pnpm run check:repo-hygiene`。交付验收：

1. `node scripts/verify-rot-budget.mjs` —— 绿；读数与基线零漂移（483/940/370/69/104；44→45/48 因本单新增 exception-leases.json 属预期 +1，report 注明；1/1；6 god-file；1368 files；`.superpowers/sdd` committed 口径以 `git ls-tree` 为准）；零余量 warning 仍为 5 项。
2. `node scripts/verify-rot-budget.mjs --selftest` —— 全绿（既有 23 + 本单新增）。
3. `node scripts/verify-rot-budget.mjs --base f2c55b8` —— 绿（本单不动 manifest，only-down/floor/删指标/dead+下调均不触发）。
4. `node scripts/verify-rot-budget.test.mjs` —— 绿（12 回归）。
5. `pnpm run check:repo-hygiene` —— 绿。

## skill 前置

阅读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`，遵循其中与本任务相关的约定，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w18-3-report.md`，必含三节：**改动摘要**（lease schema / live 判定 / >1000 规则 / 注释禁令 / floor 旁路收口）/ **验证**（验证节全部 5 条命令原文输出 + exit code + 旧代码行为确认记录）/ **状态**（状态词结尾）。
