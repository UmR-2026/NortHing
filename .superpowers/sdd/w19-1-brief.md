# W19-1 Brief — selftest 外置拆分（lease 兑现 + ceiling 重组）

## 任务标识

W19-1（W18 波终 lease 承诺兑现：`scripts/exception-leases.json` 中 verify-rot-budget.mjs 的 next_action「波后 selftest 外置拆分」，revisit_after 2026-10-15；终审硬约束：本单是 W19 触碰 checker 的第一单）。无独立波次计划文档，本 brief 自带全部 constraints。

## BASE

`1ac7438`（main HEAD，代码树干净；派发时 task-gate 起点 = 本 brief 提交后的 docs commit，与 1ac7438 代码树等价，仅 docs 差异）。

## 允许文件集

- `scripts/verify-rot-budget.mjs`（修改，收缩）
- `scripts/fixtures/rot-budget/selftest-cases.test.mjs`（新增，**命名钉死 test 后缀**——file-lines 扫描对 test-named 豁免；必须放子目录，顶层新增会触发 W18-5b 净不得增规则。该文件是代码不是 fixture 数据，**不登记进 registry.json**）
- `scripts/rot-budget.json`（修改，仅 `god_file:scripts/verify-rot-budget.mjs` 的 ceiling 重组 + note 追记）
- `scripts/exception-leases.json`（条件槽位：仅当拆分后主文件 ≤1000 行时移除该文件对应的 lease 条目——lease 承诺已兑现，留着是悬空条目；若 >1000 则保留不动）
- `scripts/verify-rot-budget.test.mjs`（条件槽位：仅当 helper 迁移导致 import 路径变化时改 import 行，其余零改动；目标是零改动——`verifyRotBudget`/`attestFixtureRegistry` 导出必须留在主文件）

report 写 `.superpowers/sdd/w19-1-report.md`，不进本单验收 diff。

## 功能要求

1. **纯移动外置拆分**：`runSelftest`（现 line ~927–2253，53 项用例 + 合成 helper）整体迁出至 `scripts/fixtures/rot-budget/selftest-cases.test.mjs`，主文件保留瘦 `runSelftest()` 接线（含 W18-7 registry attestation 起步调用，位置语义不变：起步先校验，失败非零退出）。**用例判定逻辑逐字不变**（pure move）；模块间依赖用 ESM 静态 import 或依赖注入均可，约束 = 模块求值期互不调用（运行期调用安全）。**`fixturesDir`/`repoRoot` 由主文件注入迁出函数签名；`import.meta.url` 推导留在主文件**（用例 1–5/9/11–13 自由引用这两个值，随迁会改指 cases 文件导致路径全错）。
2. **主文件目标 ≤1000 行**（逃出 >1000 硬边界；搬迁区约 1330 行，净效果主文件约 950–990）。最终行数如实进 report。
3. **ceiling 重组（only-down）**：`god_file:scripts/verify-rot-budget.mjs` ceiling 2300 → **满足 headroom floor 的最小 50 倍数**，公式逐字钉死：`ceil((L + max(5, ceil(0.05·L))) / 50) × 50`（L = 拆分后实际行数；floor 语义出处 `verify-rot-budget.mjs:525`，例：L=966 → floor 1015 → ceiling 1050）；note 追记「W19-1 selftest 外置拆分重组 2300→<新值>」。
4. **条件槽位**：主文件 ≤1000 ⇒ 删 exception-leases.json 中该文件条目；否则不动。
5. 纯 Node 标准库，零新依赖；输出 English-only 不变。

## Constraints（自 W18 波 Global Constraints 摘录适配）

1. 纯 Node 标准库，零新依赖；脚本输出与日志 English-only。
2. **ceiling 只降不升**（本单即降，家规 7 in-scope）；manifest 删指标视同上调（禁止）；除钉死条目外不改 rot-budget.json 其他任何字段。
4. 纯移动等价纪律：report「等价性证据」节 = 拆分前后 `--selftest` 输出逐项一致（53/53）+ 主运行读数零漂移；**行数下降来自搬迁，不得报告为结构改善（P09）**。
6. 不得用 Markdown 说明替代机器校验；report 中验证命令贴原文输出 + exit code。
7. commit 逐文件点名 `git add`（禁 `-A`）；message 前缀 `feat(scripts):` + `(W19-1)` 后缀。
8. 产物内本地绝对路径用 `<REPO_ROOT>` 等占位；收口前必跑 `node scripts/check-repo-hygiene.mjs`。
10. report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

另：本单触碰 `metaRatchetPaths`（verify-rot-budget.mjs + rot-budget.json），验收走双 judge 车道（代码级批准已委托 judge 流程，用户拍板 2026-09-07）。

## 禁区

- 禁改任何校验规则/收集器/扫描面/判定逻辑（本单零行为变化）。
- 禁改 `verify-task-gate.mjs`、`workflow-policy.json`、`registry.json`、5 个既有 fixture。
- 禁删改任何既有用例的断言内容（迁移逐字）。
- 禁上调任何 ceiling。

## 验证

1. `node scripts/verify-rot-budget.mjs` —— 绿；读数零漂移（5 grep 同数、dir_entries:scripts=45/48、god-file rules 9、checkedFiles 1397、verdict: rotting）。**另附机械取证命令打印自身条目读数**（主运行摘要不打 per-file 读数）：如 `node -e "import('./scripts/verify-rot-budget.mjs').then(async (m) => { const fs = await import('node:fs'); const r = await m.verifyRotBudget({ projectRoot: process.cwd() }); const c = JSON.parse(fs.readFileSync('scripts/rot-budget.json', 'utf8'))['god_file:scripts/verify-rot-budget.mjs'].ceiling; console.log(r.counts['scripts/verify-rot-budget.mjs'] + '/' + c); })"`（counts 按 relPath 为键，不带 `god_file:` 前缀；形态可调，目标 = 机器输出「拆分后行数/新 ceiling」）。
2. `node scripts/verify-rot-budget.mjs --selftest` —— 53/53 绿（registry 起步校验绿），与拆分前输出逐项一致。
3. `node scripts/verify-rot-budget.mjs --base 1ac7438` —— 绿（ceiling 降且满足 headroom floor；scripts 顶层计数 45==45）。
4. `node scripts/verify-rot-budget.test.mjs` —— 33 绿。
5. `node scripts/verify-task-gate.mjs validate-policy` —— 绿。
6. `pnpm run check:repo-hygiene` —— 绿。

## skill 前置

阅读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`（含 2026-09-08 NortHing 部署注记），遵循其中与本任务相关的约定，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w19-1-report.md`，必含三节：**改动摘要**（含拆分后行数、新 ceiling、lease 槽位处置）/ **验证**（6 条命令原文输出 + exit code + 等价性证据节）/ **状态**（状态词结尾）。
