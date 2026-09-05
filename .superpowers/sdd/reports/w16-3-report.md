# W16-3 任务报告：家规 8 — commit-bound workflow gate（双语文档）

- 任务标识：W16-3
- BASE：`559cd6f`
- HEAD / TIP：`e9833a6` (`docs(agents): house rule 8 — commit-bound workflow gate (W16-3)`)
- 提交文件集：`AGENTS.md`、`AGENTS-CN.md`（无越界修改）

## 改动摘要

在 `AGENTS.md` 与 `AGENTS-CN.md` 的 `Housekeeping rules` / `内务规则` 现有 0-7 条之后，追加了第 8 条 **House rule 8 — Commit-bound workflow gate（Commit 绑定工作流闸）**，包含 5 个子点：
1. 任务验收以 BASE_SHA / TIP_SHA + brief 允许文件集为界，并通过 `scripts/verify-task-gate.mjs verify-attempt` 机械校验；
2. 续单必须作为新 attempt，提供独立 brief 与 BASE/允许文件集，不接受事后叙述扩围；
3. 审查结论状态机严格为 PASS / FAIL / CANNOT_VERIFY / BLOCKED，CANNOT_VERIFY 遵从 `scripts/workflow-policy.json` 的 `cannotVerifyPolicy` 分级，禁止直接转 APPROVE；
4. Meta-ratchet 机制：触碰 `scripts/workflow-policy.json` 中 `metaRatchetPaths` 的文件改动自动升级至最高审查车道（双 judge + 用户拍板）；
5. `APPROVE_WITH_CONCERNS` 确立为一等结论状态，“无法确定”不被惩罚但必须带 owner 与截止时间。

格式与编号风格严格对齐既有条目，未改动任何 0-7 条既有文字。

## 双语对照

| 子点 | 中文（`AGENTS-CN.md`） | 英文（`AGENTS.md`） | 语义对齐说明 |
|---|---|---|---|
| 标题 | `8. **Commit 绑定工作流闸**：` | `8. **Commit-bound workflow gate**:` | 标题及加粗格式完全对应。 |
| 1 | `1. 任务验收以 BASE_SHA / TIP_SHA + brief 允许文件集为界；机械比较命令：node scripts/verify-task-gate.mjs verify-attempt --base <sha> --tip <sha> --allowlist <file>，越界即失败。` | `1. Task acceptance is bounded by BASE_SHA / TIP_SHA + the brief's allowlist; mechanical verification command: node scripts/verify-task-gate.mjs verify-attempt --base <sha> --tip <sha> --allowlist <file>, failing immediately on any out-of-bounds change.` | 准确对齐任务验收边界约束、命令格式与越界即失败机制。 |
| 2 | `2. 续单 = 新 attempt：必须有独立 brief（含自己的 BASE 与允许文件集）；不接受事后叙述扩围。` | `2. Continuation = new attempt: must have an independent brief (with its own BASE and allowlist); ex-post narrative expansion is not accepted.` | 准确对齐续单作为独立 attempt、必须具备独立 brief 与拒绝事后叙述扩围规则。 |
| 3 | `3. 审查结论状态机：PASS / FAIL / CANNOT_VERIFY / BLOCKED；CANNOT_VERIFY 按 scripts/workflow-policy.json 的 cannotVerifyPolicy 分级（判定性证据阻塞；辅助证据 ≤2 项且不触 trust boundary ⇒ 结论上限 APPROVE_WITH_CONCERNS + owner + 截止），禁止直接转 APPROVE。` | `3. Review verdict state machine: PASS / FAIL / CANNOT_VERIFY / BLOCKED; CANNOT_VERIFY is tiered per cannotVerifyPolicy in scripts/workflow-policy.json (decisive evidence blocks; auxiliary evidence ≤2 items and not touching trust boundary ⇒ verdict capped at APPROVE_WITH_CONCERNS + owner + deadline); direct promotion to APPROVE is forbidden.` | 准确对齐状态机枚举、引用策略字段 `cannotVerifyPolicy` 分级标准及禁止直转 APPROVE 约束。 |
| 4 | `4. meta-ratchet：修改 scripts/workflow-policy.json 的 metaRatchetPaths 所列文件的 commit，自动升最高审查车道（双 judge + 用户拍板）。` | `4. Meta-ratchet: commits modifying any file listed in metaRatchetPaths of scripts/workflow-policy.json automatically escalate to the highest review lane (dual judges + user sign-off).` | 准确对齐引用策略字段 `metaRatchetPaths` 及其触发最高车道（双 judge + 用户）的升轨机制。 |
| 5 | `5. APPROVE_WITH_CONCERNS 是一等结论状态：“无法确定”不被惩罚，但必须带 owner 与截止时间。` | `5. APPROVE_WITH_CONCERNS is a first-class verdict: "cannot verify" is not penalized, but must specify an owner and a deadline.` | 准确对齐一等结论状态定义，明确“无法确定”不惩罚并强制绑定 owner 与 deadline。 |

## 验证

### 1. 任务闸门校验（`verify-task-gate.mjs verify-attempt`）

执行命令：
```bash
$allowFile = [System.IO.Path]::GetTempFileName()
Set-Content -Path $allowFile -Value "AGENTS.md`nAGENTS-CN.md"
node scripts/verify-task-gate.mjs verify-attempt --base 559cd6f --tip e9833a6 --allowlist $allowFile
```

输出原文：
```text
Attempt verification passed: all modified files are within allowlist.
```
exit code: 0。证明 `559cd6f..e9833a6` 提交的修改完全在允许文件集内，无越界修改。

### 2. 策略文件校验与自测

执行命令：
```bash
node scripts/verify-task-gate.mjs --selftest && node scripts/verify-task-gate.mjs validate-policy
```

输出原文：
```text
[PASS] negative fixture a: replay W15-1l real incident (detected out-of-bounds pages_archive.rs)
[PASS] negative fixture b: invalid git revision rejected
[PASS] negative fixture c: missing required section in brief rejected
[PASS] negative fixture d: unapproved exemption phrase rejected
[PASS] negative fixture e: prejudging reviewer phrase in prose rejected
[PASS] negative fixture f: bad policy missing required field rejected
[PASS] negative fixture g: policy enum mismatch rejected
[PASS] positive fixture 1: complete 8-file allowlist passes
[PASS] positive fixture 2: allowlist with unfulfilled file passes with warning
[PASS] positive fixture 3: w16-1-brief.md passes validate-brief
[PASS] positive fixture 4: default workflow-policy.json passes validate-policy
Selftest passed: 11 fixtures passed (7 negative, 4 positive).
Policy validation passed: E:\agent-project\northing\scripts\workflow-policy.json
```
exit code: 0。

### 3. 代码卫生检查（`check-repo-hygiene.mjs`）

执行命令：
```bash
node scripts/check-repo-hygiene.mjs
```

输出原文：
```text
Repository hygiene check failed:
- .superpowers/sdd/w16-2-brief.md:21 contains a local absolute path.
- .superpowers/sdd/w16-4-brief.md:37 contains a local absolute path.
- .superpowers/sdd/w16-4-brief.md:38 contains a local absolute path.
```

分析与说明：
- 本任务所修改的 `AGENTS.md` 与 `AGENTS-CN.md` 本身经扫描通过，含 0 处违规（无 token、无私钥、无本地绝对路径）。
- 报错项全部源自仓内当前未跟踪的并行任务简报 `.superpowers/sdd/w16-2-brief.md` 与 `.superpowers/sdd/w16-4-brief.md`。按照 brief 与跨任务接口要求，本任务严格不修改、不提交其它任务的未跟踪产物。

## CI 提示确认

`.github/workflows/ci.yml` 对 `**/*.md` 配置了 `paths-ignore`（第 6-8、11-13 行），本单为纯 Markdown 规范文档改动，合入后不触发 CI 工作流。hygiene 针对本任务修改的文件在本地检查无任何违规；docs-contract 闸属于 Phase 1，不在本单范围内。

## 状态

DONE
