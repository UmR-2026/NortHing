# W18-0 Brief — task-gate rename 漏检修复

## 任务标识

W18-0（波次计划：`.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md`；来源：2026-09-07 外部审查档 A 第一条，用户已拍板采纳）

## BASE

`a32b4f7`（main HEAD，工作树已验干净）

## 允许文件集

- `scripts/verify-task-gate.mjs`（唯一允许修改的仓库文件）

越界任何其他仓库文件 = Critical。report 写在 `.superpowers/sdd/w18-0-report.md`，随 docs 收口 commit 另行提交，不进本单验收 diff。

## 背景与已复现的缺陷（编排者在 BASE 上实证）

`verifyAttempt()`（`scripts/verify-task-gate.mjs:232`）用 `git diff --name-only <base>..<tip>` 枚举改动文件。rename 时 `--name-only` 只报新路径，**旧路径完全不可见**——allowlist 不含 rename 旧路径也能通过验收。

实证（BASE `a32b4f7` 上已跑）：`df5c1ce..a32b4f7` 含 rename `docs/handoffs/2026-09-05-six-tasks-green-rot-audit.md` → `docs/archive/handoffs/2026-09-05-six-tasks-green-rot-audit.md`；用只含新路径（缺旧路径）的 allowlist 跑 `verify-attempt`，当前输出 `Attempt verification passed`，exit=0。**修复后同一命令必须非零退出且输出含旧路径**。

## 功能要求

1. `verifyAttempt()` 的 diff 枚举改为 `git diff --name-status -z <base>..<tip>`（git 默认开启 rename 检测，保持默认），按 NUL 分隔解析 token 流：
   - 状态 token 首字符为 `R` 或 `C`（rename/copy，如 `R100`）⇒ 其后跟两个 path token（旧路径、新路径）；
   - 其他状态 ⇒ 其后跟一个 path token。
   - 路径归一化沿用现有规则：`\` → `/`，去 ` ./ ` 前缀。
2. rename/copy 的**旧路径与新路径都进入 actualFiles**：两者都必须命中 allowlist，任一越界即 error；rename 源路径的越界文案可标注 `(rename source)` 便于定位。`unfulfilled`（allowlist 有而 diff 无 → warning）语义不变。
3. `--selftest` 新增两个 fixture（先负后正，均复用真实历史，与现有 fixture a 同风格）：
   - **negative fixture h**：`--base df5c1ce --tip a32b4f7`，allowlist 只含三条：`.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md`、`docs/handoffs/2026-09-07-workflow-improvement-verdicts.md`、`docs/archive/handoffs/2026-09-05-six-tasks-green-rot-audit.md` ⇒ 必须非零退出，且输出含 `docs/handoffs/2026-09-05-six-tasks-green-rot-audit.md`（rename 旧路径被点名）。
   - **positive fixture 5**：同 base/tip，allowlist 四条（上述三条 + `docs/handoffs/2026-09-05-six-tasks-green-rot-audit.md`）⇒ exit 0。
   - 汇总行 `7 negative, 4 positive` 相应更新为 `8 negative, 5 positive`（或改为按 results 动态计数，二选一，保持输出信息准确即可）。
4. 既有 11 个 fixture 必须保持全绿（回归约束）。
5. 纯 Node 标准库，零新依赖；脚本输出 English-only（仓规）。

## Constraints（逐字自波次计划 Global Constraints，与本单相关项）

- commit 逐文件点名 `git add`（禁 `-A`）；message 前缀 `fix(scripts):` + `(W18-0)` 后缀。
- 不得用 Markdown 说明替代机器校验；report 中验证命令贴原文输出 + exit code。
- 产物内本地绝对路径用 `<USERPROFILE>` 等占位（hygiene 闸）；收口前必跑 `node scripts/check-repo-hygiene.mjs`。
- report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
- 本单触碰 `metaRatchetPaths`（`scripts/verify-task-gate.mjs`），验收走双 judge + 用户拍板车道（流程项，对实现者零额外要求）。

## 禁区

- 禁改 `scripts/` 下任何其他文件（含 `workflow-policy.json`、`verify-rot-budget.mjs`）。
- 禁改解析目标之外的任何行为（validate-brief / validate-policy 逻辑零触碰）。
- 禁调整既有 fixture 的判定标准来制造绿色。

## 验证

编排者已在 BASE `a32b4f7` 预跑全部命令（基线已实证）：

1. `node scripts/verify-task-gate.mjs --selftest` —— 基线 11 fixture 全绿；修复后须 13 全绿（8 negative / 5 positive）。
2. 漏洞复现对照（修复前 exit=0 已实证）：用缺旧路径的 allowlist 跑 `node scripts/verify-task-gate.mjs verify-attempt --base df5c1ce --tip a32b4f7 --allowlist <临时allowlist文件>`，修复后必须非零且输出含旧路径。report 贴修复前后两次输出。
3. `pnpm run check:repo-hygiene` —— 绿。
4. `node scripts/verify-task-gate.mjs validate-policy` —— 绿（回归）。

## skill 前置

阅读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`，遵循其中与本任务相关的约定，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w18-0-report.md`，必含三节：**改动摘要**（改了什么、解析算法要点）/ **验证**（上述 4 条命令的原文输出 + exit code，含漏洞复现前后对照）/ **状态**（状态词结尾）。
