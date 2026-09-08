# W18-5b Brief — scripts 退役机械化（净不得增）

## 任务标识

W18-5b（波次计划：`.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md` v2.1；原 W18-5 拆分，本单 = -1.7 退役机械化）。前置：W18-5a 已落地（`--base` 模式在 W18-2 已落地）。

## BASE

`6dc9b13`（W18-5a 落地 commit，规则生效起点；与派发时 main HEAD 代码树等价，其间仅 docs 提交）。

## 允许文件集

- `scripts/verify-rot-budget.mjs`（修改，唯一）

report 写 `.superpowers/sdd/w18-5b-report.md`，不进本单验收 diff。fixture 全部 selftest 内联 tmpdir 合成，不新增文件。

## 背景与用户拍板

scripts 一次性额度 48（到期 2026-10-15）只兜历史存量。**退役口径 = 净不得增**（用户拍板 2026-09-07）：`--base` 模式下 `scripts/` 顶层文件计数 TIP > BASE ⇒ violation——纯计数比较，无需 diff 解析；合法一增一退 = 计数不变 = 绿。

## 功能要求

1. **退役规则**：`--base` 模式下比较 `scripts/` 顶层文件的 **BASE 实测计数 vs TIP 实测计数**（ceiling 不参与该比较）：TIP 计数 > BASE 计数 ⇒ violation（violation 文案用英文表达净不得增口径，如 'net increase prohibited under scripts retirement quota'，并含到期日 2026-10-15；不得出现中文字面量）。**BASE 计数语义钉死**：仅计顶层 regular file（blob），与 TIP 侧 `isFile()` 语义逐字一致——`git ls-tree <base> scripts/` 输出含 tree 条目（子目录），必须按 blob 类型过滤（实证：356b33c 上该命令 49 行 = 45 blob + 4 tree，朴素行计数会静默 fail-open +4）。
2. **selftest 扩展**（内联，tmpdir 合成 git repo）：
   - 负例（判别 blob-only 语义的 fail-open 方向）：BASE scripts/ 含子目录（子目录内含 ≥1 文件）且 TIP 净增数 ≤ 子目录数 ⇒ **红**（机理：朴素行计数把 tree 行计入 BASE ⇒ 基数虚高挡住净增判定 ⇒ 误绿；正确实现只计 blob ⇒ 净增 >0 ⇒ 红）；
   - 判别用例（钉「比较计数而非 ceiling」）：BASE manifest ceiling=48 但 BASE 实测=5、净增 2 ⇒ 红（错误实现 7<48 会误绿）；
   - 正例：增删平衡（绿）/ 不变（绿）/ BASE 侧 scripts/ 含子目录且 TIP 计数不变（绿，钉 TIP 侧目录不误计）。
   - 既有 selftest 全部保持全绿。
3. 纯 Node 标准库，零新依赖；输出 English-only。

## Constraints（逐字自波次计划 Global Constraints，与本单相关项）

- 任何 ceiling 不得上调；不改 `rot-budget.json`/`workflow-policy.json`/`exception-leases.json`。
- commit 逐文件点名 `git add`（禁 `-A`）；message 前缀 `feat(scripts):` + `(W18-5b)` 后缀。
- 不得用 Markdown 说明替代机器校验；report 中验证命令贴原文输出 + exit code。
- 产物内本地绝对路径用 `<USERPROFILE>` 等占位；收口前必跑 `node scripts/check-repo-hygiene.mjs`。
- report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
- 本单触碰 `metaRatchetPaths`，验收走双 judge 车道（代码级批准已委托 judge 流程，用户拍板 2026-09-07）。
- checker 改动**先负向 fixture 后实现**（GC4）：report「旧代码行为确认」节如实记录旧代码无退役规则（fail-open by absence）。

## 禁区

- 禁改收集器/扫描面/校验规则的任何其他部分；禁改 `scripts/` 下其他文件。
- 禁为过关调整 fixture 判定标准。

## 验证

1. `node scripts/verify-rot-budget.mjs` —— 绿，读数与 W18-5a 交付后基线零漂移（9 god-file rules、checkedFiles 1397）。
2. `node scripts/verify-rot-budget.mjs --selftest` —— 全绿（既有 + 本单新增）。
3. `node scripts/verify-rot-budget.mjs --base 6dc9b13` —— 绿（本单不动 scripts 文件数）。
4. `node scripts/verify-rot-budget.test.mjs` —— 绿（12 回归）。
5. `pnpm run check:repo-hygiene` —— 绿。

## skill 前置

阅读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`，遵循其中与本任务相关的约定，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w18-5b-report.md`，必含三节：**改动摘要** / **验证**（5 条命令原文输出 + exit code + 旧代码行为确认）/ **状态**（状态词结尾）。
