# W18-6 Brief — verdict rubric SSOT + dead registration 限期 + P06 文档同步

## 任务标识

W18-6（波次计划：`.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md` v2.1，相位 0.7/0.8 + P06 档 A + W18-2 Minor-3 处置 + W18-3 carry-forward）。前置：W18-3（exception lease + allow-god-file 禁令）、W18-5a（rotScanScope attestation 通道）已落地。

## BASE

`50e97dd`（main HEAD，代码树干净；派发时 task-gate 起点 = 本 brief 提交后的 docs commit，与 50e97dd 代码树等价，仅 docs 差异）。

## 允许文件集

- `scripts/workflow-policy.json`（修改）
- `scripts/verify-rot-budget.mjs`（修改）
- `scripts/verify-rot-budget.test.mjs`（修改，编辑范围钉死：① 更新死登记文案两处断言——line ~417-420 全等断言 + line ~427-430 regex 断言，新文案逐字写入；② 文件尾部追加本单新用例块；其余零改动）
- `AGENTS.md`（修改，仅家规 3 段落）
- `AGENTS-CN.md`（修改，仅家规 3 对应段落）

fixture 全部 tmpdir 合成（日期用例必须相对 today 计算，禁用静态日期），不新增文件。**承载位置（brief review round-2 裁决）**：主文件 `verify-rot-budget.mjs` 自身 ceiling 2300 / 现 2204，余量仅 96 行——本单新用例（约 11 个）**全部写进既有 `verify-rot-budget.test.mjs`**（node --test 形态，与该文件既有合成根用例同构；test-named 免于 file-lines 扫描，顶层文件数不变 ⇒ dir_entries 与 W18-5b 净不得增规则零影响）；主文件内 selftest 保持 53 项零新增。由此验证 2 的「本单新增」由验证 4 承载。report 写 `.superpowers/sdd/w18-6-report.md`，不进本单验收 diff。

**主文件体量纪律**：`verify-rot-budget.mjs` 净增 ≤ 90 行（ceiling 2300 硬约束，家规 7：上调须用户拍板，不在本单授权内）。若核心逻辑无法在该预算内交付 ⇒ BLOCKED 上交编排者转用户决策，不得压线硬塞。

允许文件集较 plan 任务卡扩充了 AGENTS.md/AGENTS-CN.md——依据：plan 功能要求 4（P06）明文要求对齐 AGENTS.md，handoff 2026-09-08 队列项 2 追加 W18-3 carry-forward（家规 3 的 allow-god-file 表述必须改为 exception lease）；plan 允许文件集遗漏属计划内部矛盾，以此 brief 为准。

## 功能要求

### 1. rotVerdictRubric 进 SSOT（相位 0.7）

- `scripts/workflow-policy.json` 顶层新增字段，逐字：
  `"rotVerdictRubric": {"healthy": "0 findings", "stable": "1-2 bounded findings", "rotting": ">=3 findings OR any unbounded"}`
- `verify-rot-budget.mjs` 读取 policy 的 rotVerdictRubric（复用 W18-5a 的 policy 读取通道）：
  - policy 文件存在但 rubric 缺失 / 非三键对象 / 值非字符串 ⇒ violation（fail-closed，与 rotScanScope attestation 同哲学）；policy 文件不存在（合成环境）⇒ 跳过校验（与既有 attestation 行为一致）。
  - **declared-vs-actual 对账**：policy 三键的值须与 checker 内钉死字面（`0 findings` / `1-2 bounded findings` / `>=3 findings OR any unbounded`）逐字相等，不等 ⇒ violation（防三处尺度静默漂移，同 W18-5a attestation 模式）。
  - **可观测面钉死**：`verifyRotBudget` 返回值新增 `verdict` 字段（含 findings 计数与 unbounded 布尔）；verdict 分段 `verdict: <class> (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric)` 同时追加到**绿路径与红路径两条摘要行**（红路径用例经返回值断言，不依赖 stdout）；**verdict 仅在完整扫描路径定义——manifest 缺失 / 校验失败早退不带 verdict 字段与分段**。映射钉死：findings = 本次运行 violations + warnings 总数；0 ⇒ healthy；1–2 ⇒ stable；≥3 或任一 unbounded ⇒ rotting；**unbounded = 未在 manifest 登记的 file-lines 违规**（即无 ceiling 登记的 >800 行文件违规，字面「无界」）。
- 用例（承载于 `verify-rot-budget.test.mjs`，tmpdir 合成）：policy 缺 rubric ⇒ 红；rubric malformed（缺键）⇒ 红；rubric 值与钉死字面不等 ⇒ 红；合法 rubric ⇒ 绿且 verdict=healthy；1 条 warning ⇒ stable；≥3 findings ⇒ rotting；未登记 >800 文件违规 ⇒ rotting（unbounded 方向，经返回值断言）。

### 2. dead registration 30 天限期（相位 0.8）

- manifest `file-lines` entry 增可选字段 `deadSince`（FIELD_WHITELIST['file-lines'] 加入；validateManifest fail-closed：非 `YYYY-MM-DD` 合法日期 ⇒ 拒绝）。
- 预扫描死登记分支（现行 line ~667 块）扩展：
  - 文件缺失 + 无 deadSince ⇒ warning（保持绿），文案指引补登 deadSince 或移除登记；**同步更新 `verify-rot-budget.test.mjs` 的死登记文案两处断言（新文案逐字写入，见允许文件集编辑范围）**；
  - 文件缺失 + deadSince ⇒ 日差判定：复用既有 todayUtc/YYYY-MM-DD 约定，**新增 UTC 日差计算（算法口径钉死：`(todayUtc - deadSince) 毫秒差 / 86400000 > 30`）**；> 30 天 ⇒ violation；≤ 30 天 ⇒ warning（宽限期，绿）；
  - 文件存活 ⇒ 无 warning（deadSince 被忽略——刻意语义，写进注释）。
- 用例（承载于 `verify-rot-budget.test.mjs`，tmpdir 合成，日期相对 today）：死登记无日期 ⇒ warning 不红 / 死登记 31 天 ⇒ 红 / 死登记恰好 30 天 ⇒ warning 绿（边界钉死）/ 文件存活带 deadSince ⇒ 无 warning / malformed deadSince ⇒ manifest 拒绝。既有 selftest 与 test.mjs 用例全绿。

### 3. P06 文档同步（档 A）+ W18-3 carry-forward

- `AGENTS.md` 家规 3 现状句逐字：「production `.rs` files over 800 lines raise review pressure; over 1000 lines must be split or carry a `// allow-god-file` justification comment at the top of the file. New modules start below the line.」——将 allow-god-file 通道替换为 exception lease 通道（>1000 ⇒ split 或 `scripts/exception-leases.json` 登记 lease 含 revisit_after），并钉两句口径：**allow-god-file 头注释通道已被机械禁令废除**；**闸口径 = checker `countLines`，仅换行/重排产生的行数下降不得报告为结构改善（P09）**。
- `AGENTS-CN.md` 对应段落同步同口径改写。
- 跨仓部分不由本单执行：anti-rot skill（`E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`，属 agent-project 仓）的头注释口径对齐由编排者在该仓另行提交，不进本单 diff。

### 4. W18-2 Minor-3 处置

- `scripts/workflow-policy.json` 的 `metaRatchetPaths` 数组追加 `"scripts/rot-budget.json"`（改 manifest = 改判据，升最高车道）。

### 5. ~~P03 最小版~~（brief review 裁决剔除，顺延下波）

裁决理由（reviewer-53）：纯死配置（无消费方）与本波 W18-4 清除死豁免的反模式自相矛盾；plan 最小形态（按任务类型）未达成；正确形态 = W18-7 改 policy.json 时与消费方同单落地。plan L145 已预授权「判超范围则顺延」。

## Constraints（逐字自波次计划 Global Constraints，与本单相关项）

1. 纯 Node 标准库，零新依赖；脚本输出与日志 English-only。
2. **任何 ceiling 不得上调**；manifest 删指标视同上调（禁止）。本单不改 `scripts/rot-budget.json`、`scripts/exception-leases.json`。
4. checker 改动**先负向 fixture 后实现**（GC4）：report「旧代码行为确认」节如实记录旧代码行为（无 rubric 校验、死登记无限期——fail-open by absence）。
5. **禁止复活 `allow-god-file` 头注释**（O-2 errata）；>1000 只能走 exception lease。
6. 不得用 Markdown 说明替代机器校验；report 中验证命令贴原文输出 + exit code。
7. commit 逐文件点名 `git add`（禁 `-A`）；message 前缀 `feat(scripts):` + `(W18-6)` 后缀。
8. 产物内本地绝对路径用 `<REPO_ROOT>` 等占位（hygiene 闸）；收口前必跑 `node scripts/check-repo-hygiene.mjs`。
10. report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

另：本单触碰 `metaRatchetPaths`（policy.json + verify-rot-budget.mjs），验收走双 judge 车道（代码级批准已委托 judge 流程，用户拍板 2026-09-07）。

## 禁区

- 禁改收集器/扫描面/其他校验规则的任何部分；禁改 `scripts/` 下其他文件（`verify-rot-budget.test.mjs` 除允许文件集钉死的两处断言 + 尾部新用例块外零改动）；禁改 `verify-task-gate.mjs`。
- 禁为过关调整 fixture 判定标准。
- 禁上调任何 ceiling、禁删 manifest 指标。
- AGENTS.md/CN 仅改家规 3 段落，其他行零改动。

## 验证

1. `node scripts/verify-rot-budget.mjs` —— 绿；读数零漂移（5 grep 同数、dir_entries:scripts=45/48、9 god-file rules、checkedFiles 1397）；摘要新增 verdict 分段（当前 5 条零余量 warning ⇒ 预期 `verdict: rotting`，rubric 字面口径的诚实读数）。
2. `node scripts/verify-rot-budget.mjs --selftest` —— 全绿（既有 53 项，本单不新增 selftest 内联用例）。
3. `node scripts/verify-rot-budget.mjs --base 6dc9b13` —— 绿（本单不动 manifest、不动 scripts 文件数）。
4. `node scripts/verify-rot-budget.test.mjs` —— 全绿（既有 12 + 本单新增用例；死登记文案两处断言同步更新后绿）。
5. `node scripts/verify-task-gate.mjs validate-policy` —— 绿（policy 新增字段后结构完好）。
6. `pnpm run check:repo-hygiene` —— 绿。

## skill 前置

阅读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`，遵循其中与本任务相关的约定，不因此扩展任务范围。（注意：该文件本身的口径修正在另一仓由编排者执行，本单不得改动它。）

## 报告

写 `.superpowers/sdd/w18-6-report.md`，必含三节：**改动摘要** / **验证**（6 条命令原文输出 + exit code + 旧代码行为确认）/ **状态**（状态词结尾）。
