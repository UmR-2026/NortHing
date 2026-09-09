# W21-1 Brief — rot-budget `--base=` 空值 fail-open 修复（W19-2 M1 清账）

## 任务标识

W21-1（清账波次单；W19-2 波终审 Minor M1 顺延项：rot-budget CLI `--base=` 空值静默跳过基线校验 = fail-open）。

## BASE

`df4758a`（main HEAD，工作树干净，编排者已于 BASE 实测全部验证命令，见下）。

## 背景与 BASE 证据（编排者已实测）

`scripts/verify-rot-budget.mjs` 主入口（现 line 985-992）：

```js
let base;
if (flags.base) {
  if (typeof flags.base !== 'string' || flags.base.trim() === '') {
    console.error('Error: --base requires a commit SHA or ref');
    process.exit(1);
  }
  base = flags.base.trim();
}
```

`parseArgs` 对 `--base=`（等号空值）产出 `flags.base === ''`；空串 falsy ⇒ `if (flags.base)` 整块跳过 ⇒ `base === undefined` ⇒ `verifyRotBudget({ base: undefined })` 不走 only-down 基线比对，**exit 0 静默通过（fail-open）**。

BASE 实测红态：`node scripts/verify-rot-budget.mjs --base=` → exit 0，输出与不带 flag 完全等价（无任何 base 提示）。CI 调用形态为 `node scripts/verify-rot-budget.mjs`（无 --base，ci.yml:150），本修复不改变 CI 行为；唯一行为变化 = 显式给空值从静默放行变硬报错。

对照：verify-task-gate.mjs 同形态 `--base=` 已由 `verifyAttempt` 的 `!base` falsy 检查兜底报错（fail-closed），本单不动 task-gate。

## 允许文件集

- `scripts/verify-rot-budget.mjs`（修改，仅主入口 1 行条件，见功能 1）
- `scripts/verify-rot-budget.test.mjs`（修改，仅追加测试，见功能 2）

report 写 `.superpowers/sdd/w21-1-report.md`，不进本单验收 diff（不 commit、不 git add）。

## 功能要求

1. **fail-closed 条件修复**：`scripts/verify-rot-budget.mjs` 主入口 `if (flags.base)` 改为 `if (flags.base !== undefined)`。其余逐字不动。效果矩阵（逐行核对，不得有第二种读法）：
   - 无 flag：`flags.base === undefined` → 跳过（现状不变）
   - `--base=`：`''` → 进入分支 → `trim() === ''` → 报错 exit 1（**新行为，修复点**）
   - `--base`（裸，后随 `-` 开头参数或行尾）：`true` → 进入分支 → `typeof !== 'string'` → 报错 exit 1（现状已是报错，不变）
   - `--base sha` / `--base=sha` / `--base=  sha  `：正常 trim 赋值（现状不变）
2. **测试**：`scripts/verify-rot-budget.test.mjs` 追加 spawn 级负例 `W21-1 F1`：spawn `node scripts/verify-rot-budget.mjs --base=` ⇒ 断言 exit 1 且 stderr 含 `--base requires a commit SHA or ref`。命名与既有 `W19-2 F1` 等同款格式。禁改既有 38 个测试。

## Constraints

- commit 逐文件点名 `git add`（禁 -A，2 文件）；message 前缀 `fix(scripts):` + `(W21-1)` 后缀，body 注明 W19-2 M1 清账来源。
- report 贴原文输出 + exit code。
- 收口前必跑全部 4 条验证（见下）。
- report 结尾状态词。
- 本单触碰 `metaRatchetPaths`（verify-rot-budget.mjs 在 workflow-policy.json metaRatchetPaths），验收走双 judge 车道（代码级批准已委托 judge 流程，用户拍板 2026-09-07）。

## 禁区

- 禁改 parseArgs、verifyRotBudget 及脚本其他任何行。
- 禁动 task-gate、rot-budget.json、workflow-policy.json 及任何清单外文件。
- 禁顺手清账扩面（本单只修 M1 一项）。

## 验证（编排者已在 BASE 预跑：1/3/4 绿，2 复现红态）

1. **BASE 红态复现**（report 贴原文）：`node scripts/verify-rot-budget.mjs --base=` → 修复前 exit 0 且无 base 提示；修复后 exit 1 + stderr 报错行。
2. **修复后阴性**：同上命令 → exit 1，stderr 含 `--base requires a commit SHA or ref`。
3. **全量测试**：`node scripts/verify-rot-budget.test.mjs` → 39 pass（38 + 新增 1），0 fail。
4. **selftest + 无 flag 日常形态不回归**：`node scripts/verify-rot-budget.mjs --selftest` → 53 checks passed；`node scripts/verify-rot-budget.mjs` → exit 0（verdict 输出与 BASE 等价，warnings 属设计内）。
5. report 附：行为变化矩阵 4 行（无 flag / `--base=` / 裸 `--base` / 正常值）各自的修复前后 exit code。

## skill 前置

无强相关；如参考口径可读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md` 的 NortHing 部署注记，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w21-1-report.md`，必含三节：**改动摘要** / **验证**（5 项原文输出 + exit code）/ **状态**（状态词结尾）。
