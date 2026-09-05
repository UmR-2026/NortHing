# W16-5 Brief：终审收口——结论词表统一 + metaRatchetPaths 增补

- 任务标识：W16-5
- 来源：W16 波级终审（`.superpowers/sdd/packages/w16-final-review.md`）I-1 / I-2
- BASE：`353a20f`（main HEAD）

## 允许文件集（越界 = judge Critical）

1. `AGENTS.md`
2. `AGENTS-CN.md`
3. `scripts/workflow-policy.json`

禁区：其它一切文件。

## 修复项（两处，均来自终审 Important）

### 1. I-1 结论词表统一（编排者裁决：policy 枚举为准）

`AGENTS.md` 与 `AGENTS-CN.md` 家规 8 第 3 子点中的状态机枚举 `PASS / FAIL / CANNOT_VERIFY / BLOCKED` 改为：**审查结论以 `scripts/workflow-policy.json` 的 `reviewVerdicts` 为唯一词表**（当前为 APPROVE / APPROVE_WITH_CONCERNS / CANNOT_VERIFY / BLOCKED / FAIL），并保留原句后半的 CANNOT_VERIFY 分级语义不变。双语同步、各一词处改动，不动其它文字。

### 2. I-2 metaRatchetPaths 增补

`scripts/workflow-policy.json` 的 `metaRatchetPaths` 数组增补四个路径（保持现有四项不动）：

```text
scripts/check-repo-hygiene.mjs
scripts/check-core-boundaries.mjs
scripts/check-github-config.mjs
package.json
```

## 验证（输出原文进 report）

```text
node scripts/verify-task-gate.mjs validate-policy
node scripts/verify-task-gate.mjs --selftest
node scripts/check-repo-hygiene.mjs
node scripts/verify-rot-budget.mjs
```

## 报告与提交

- report：`.superpowers/sdd/reports/w16-5-report.md`（不入 commit）。
- commit：逐文件点名 add 三个允许文件；message：`fix(policy): unify review verdict vocabulary + extend metaRatchetPaths (W16-5)`。

## Global Constraints

1. 零新依赖；输出 English-only。2. 验证输出原文进 report。3. 禁 `git add -A`。4. 结尾状态词合规。
