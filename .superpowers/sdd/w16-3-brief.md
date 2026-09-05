# W16-3 Brief：家规 8 — commit-bound workflow gate（双语文档）

- 任务标识：W16-3
- 波次计划：`E:\agent-project\NortHing\.superpowers\sdd\plan-2026-09-05-w16-trusted-core.md`
- 来源：D-synthesis §3 Phase -1.4 文档侧 + D-2 meta-ratchet + D-8 "允许不知道"制度化
- BASE：`559cd6f`（main HEAD，W16-1 已落）

## 背景（一句话）

W16-1 的闸脚本和策略文件已落地，本单把对应规则写进仓库规范（AGENTS.md 双语的 housekeeping rules），让规则对一切贡献者（含各 session 编排者）可见。

## 允许文件集（diff 越出 = judge Critical）

1. `AGENTS.md`
2. `AGENTS-CN.md`

禁区：其它一切文件。

## 功能要求

在 housekeeping rules 现有 0-7 条之后追加**第 8 条**（中英双语，语义逐条对齐，各自符合该文件的行文风格——英文文件用英文，中文文件用中文）：

**House rule 8 — Commit-bound workflow gate（commit 绑定工作流闸）**，五个子点：

1. 任务验收以 BASE_SHA / TIP_SHA + brief 允许文件集为界；机械比较命令：`node scripts/verify-task-gate.mjs verify-attempt --base <sha> --tip <sha> --allowlist <file>`，越界即失败。
2. 续单 = 新 attempt：必须有独立 brief（含自己的 BASE 与允许文件集）；不接受事后叙述扩围。
3. 审查结论状态机：PASS / FAIL / CANNOT_VERIFY / BLOCKED；CANNOT_VERIFY 按 `scripts/workflow-policy.json` 的 `cannotVerifyPolicy` 分级（判定性证据阻塞；辅助证据 ≤2 项且不触 trust boundary ⇒ 结论上限 APPROVE_WITH_CONCERNS + owner + 截止），禁止直接转 APPROVE。
4. meta-ratchet：修改 `scripts/workflow-policy.json` 的 `metaRatchetPaths` 所列文件的 commit，自动升最高审查车道（双 judge + 用户拍板）。
5. `APPROVE_WITH_CONCERNS` 是一等结论状态："无法确定"不被惩罚，但必须带 owner 与截止时间。

插入位置：`### Housekeeping rules` 节第 7 条之后；双语文件同一相对位置。编号、加粗风格与既有 0-7 条一致。不改动任何既有条目文字。

## 验证（命令 + 结果进 report）

```text
node scripts/check-repo-hygiene.mjs
```

- 双语对照：report 中逐条列出中文第 8 条五点与英文第 8 条五点的对应关系说明（证明语义对齐）。
- CI 提示确认：report 中说明 `.github/workflows/ci.yml` 对 `**/*.md` 有 paths-ignore，本单 md 改动不触发 CI，hygiene 本地已跑绿（docs-contract 闸属 Phase 1，不在本单）。

## 报告

写到 `E:\agent-project\NortHing\.superpowers\sdd\reports\w16-3-report.md`（不入本 commit，由编排者 docs commit 收口）：改动摘要 / 双语对照 / 验证输出 / 结尾状态词。

## 派发元信息

- commit 规则：逐文件点名 `git add AGENTS.md AGENTS-CN.md`；message：`docs(agents): house rule 8 — commit-bound workflow gate (W16-3)`。

## Global Constraints（摘编自计划）

1. 验证命令输出原文进 report。
2. commit 逐文件点名，禁 `git add -A`。
3. report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
