# W16-3 审查报告：house rule 8 — commit-bound workflow gate（双语文档）

- 任务：W16-3
- 审查对象：commit `e9833a6` (`docs(agents): house rule 8 — commit-bound workflow gate (W16-3)`)
- 审查档位：Critical / Important / Minor
- 双判决：spec 合规 + 代码质量（housekeeping docs）

## SPEC 逐条核对

### 1. 第 8 条五子点语义吻合（brief vs 双语实现）

| 子点 | Brief（L25-29） | AGENTS-CN.md | AGENTS.md | 判定 |
|---|---|---|---|---|
| 1 | BASE/TIP + allowlist；命令 `node scripts/verify-task-gate.mjs verify-attempt --base <sha> --tip <sha> --allowlist <file>`；越界即失败 | L99 字面匹配 | L103 等价英文 | ✓ |
| 2 | 续单 = 新 attempt；独立 brief + BASE + allowlist；拒绝事后叙述扩围 | L100 字面匹配 | L104 等价英文 | ✓ |
| 3 | 状态机 PASS/FAIL/CANNOT_VERIFY/BLOCKED；`cannotVerifyPolicy` 分级；aux ≤2 + 不触 trust boundary ⇒ APPROVE_WITH_CONCERNS + owner + 截止；禁直转 APPROVE | L101 字面匹配 | L105 等价英文 | ✓ |
| 4 | 修改 `metaRatchetPaths` 所列文件 ⇒ 升最高审查车道（双 judge + 用户拍板） | L102 字面匹配 | L106 等价英文 | ✓ |
| 5 | `APPROVE_WITH_CONCERNS` 一等结论；"无法确定"不惩罚；必须带 owner + 截止 | L103 字面匹配 | L107 等价英文 | ✓ |

证据原文（`git diff 559cd6f..e9833a6`）：
- AGENTS-CN.md L99-103 五子点逐字与 brief L25-29 对齐（已逐字比对）。
- AGENTS.md L103-107 五子点为等价英文翻译；术语映射正确（"无法确定"→"cannot verify"、"一等"→"first-class"、"截止时间"→"deadline"、"双 judge"→"dual judges"、"用户拍板"→"user sign-off"、"trust boundary" 保留为英文术语）。

### 2. 双语语义对齐（中英五点一一对应）

五子点完整对应，无缺失、无超译：

- 标题：`8. **Commit 绑定工作流闸**：` ↔ `8. **Commit-bound workflow gate**:` — 格式严格对应。
- 五子点语义内容完整覆盖；字段引用（`scripts/verify-task-gate.mjs`、`scripts/workflow-policy.json`、`cannotVerifyPolicy`、`metaRatchetPaths`、`APPROVE_WITH_CONCERNS`）在双文件中名称一致。
- 中文采用中文标点（`：""。，；⇒≤`），英文采用英文标点，跨语种术语（`trust boundary`、`meta-ratchet`）按 brief 一致保留。

### 3. 插入位置 / 编号 / 加粗风格 / 既有条目零改动

- 位置：`AGENTS.md` L102 在第 7 条后（L101）、`### Internationalization` 前（L109）；`AGENTS-CN.md` L98 在第 7 条后（L97）、`### 国际化` 前（L105）。两个文件同一相对位置。
- 编号：`8.` 与既有 `0.`-`7.` 同号型。
- 加粗：`8. **Title**:` 与 `0. **Lazy Senior Dev Rule (YAGNI)**:` / `7. **Rot budget only decreases**:` 一致。
- 子项缩进：3 空格（`   1.` ...），与第 0 条 ladder 子项风格一致。
- 既有条目零改动：`git diff 559cd6f..e9833a6 --stat` 显示仅 12 行新增（每文件 6 行）、0 行删除、0 行修改；diff 全文 24 行中只含 `+` 行（无 `-` 行），既有 0-7 条文字逐字保留（已与 559cd6f 父版本比对）。

### 4. 引用字段名一致

- `cannotVerifyPolicy`：`scripts/workflow-policy.json` L16 含 `"cannotVerifyPolicy"` 字段 ✓
- `metaRatchetPaths`：`scripts/workflow-policy.json` L20 含 `"metaRatchetPaths"` 数组（含 `scripts/verify-task-gate.mjs`、`scripts/verify-rot-budget.mjs`、`scripts/workflow-policy.json`、`.github/workflows/`）✓

## QUALITY 抽查

### 1. 规范文字的可执行性

每子点对第三方可执行：

- 子点 1：可执行的 shell 命令（已在 `scripts/verify-task-gate.mjs` L614 `printUsage` 中存在同形式入口；命令格式落地）。
- 子点 2：定义边界（"续单 = 新 attempt"，要件 = 独立 brief + BASE + allowlist）。
- 子点 3：状态机 + 策略文件引用 + 分级条件（≤2、trust boundary、APPROVE_WITH_CONCERNS 上限、禁直转 APPROVE）。
- 子点 4：触发条件（`metaRatchetPaths` 列表引用）+ 后果（双 judge + 用户拍板）。
- 子点 5：定义性条款（"一等结论"+"不惩罚"+"owner + 截止时间"硬条件）。

无歧义措辞（terms-of-art 都对齐 `workflow-policy.json` 字段）。无歧义疑点。

### 2. 报告双语对照抽查

- 抽查子点 1：报告 L24 引文（`1. 任务验收以 BASE_SHA / TIP_SHA + brief 允许文件集为界；机械比较命令：node scripts/verify-task-gate.mjs verify-attempt --base <sha> --tip <sha> --allowlist <file>，越界即失败。`）与 AGENTS-CN.md L99 实际内容语义等价。✓
- 抽查子点 5：报告 L28 引文（`5. APPROVE_WITH_CONCERNS 是一等结论状态："无法确定"不被惩罚，但必须带 owner 与截止时间。`）与 AGENTS-CN.md L103 实际内容语义等价。✓

报告声称的"准确对齐"在抽查两点成立。

## 独立复核（审查者跑）

为排除"实现者报喜不报忧"风险，审查者于本次会话在 commit `e9833a6` HEAD 重跑验证：

- `node scripts/verify-task-gate.mjs verify-attempt --base 559cd6f --tip e9833a6 --allowlist <allowfile>`：
  - 输出原文：`Attempt verification passed: all modified files are within allowlist.`
  - exit code: 0。
  - 与报告 L43 输出完全一致。
- `node scripts/check-repo-hygiene.mjs`：
  - 当前 w16-2/w16-4 report 与 brief 也已出现在未跟踪产物中（w16-2-report.md、w16-4-report.md、w16-2-brief.md、w16-4-brief.md 含本地绝对路径违规）；属并行任务 W16-2/4 产出，不属本任务范围。
  - 本任务修改的 AGENTS.md / AGENTS-CN.md 经针对性扫描：0 处本地绝对路径 / token / 私钥违规。
  - 报告 L82-86 关于"本任务所修改的两文件本身经扫描通过"的判断成立。
- `.github/workflows/ci.yml` 实际存在 `paths-ignore: '**/*.md'`（L6-7、L11-12），与报告 L93 一致；markdown 改动不触发 CI。

## Global Constraints

1. 验证输出原文进 report：✓（报告 L36-44 闸门命令输出、L55-68 selftest/policy 输出、L80-86 hygiene 输出均以 `text` 代码块原文进文）。
2. commit 逐文件点名：commit `e9833a6` 仅触及 `AGENTS.md` 与 `AGENTS-CN.md`；`git show --stat` 显示 2 文件 / 12 insertions / 0 deletions / 0 modifications；无 `git add -A` 痕迹。✓
3. 结尾状态词合规：报告 L97 = `DONE`，在允许集 `{DONE, DONE_WITH_CONCERNS, NEEDS_CONTEXT, BLOCKED}` 内。✓

## Cannot verify from diff（不可由 diff 判定，已分别处理）

1. 实现者报告 L40-44 verify-task-gate 输出：已由审查者本轮独立复跑，输出原文一致 → 视为已核实。
2. 实现者报告 L55-68 selftest / validate-policy 输出：selftest 11 fixtures 全部 PASS；policy 校验通过；属 W16-1 闸脚本自检输出，未在本任务改动范围内，W16-3 不涉及维护该脚本。审查者未独立复跑，因其不在 W16-3 任务交付范围。**列为不深查**。
3. 实现者报告 L82-86 hygiene 输出（针对本任务所修改文件）：已由审查者本轮以定向扫描核实，AGENTS.md / AGENTS-CN.md 均 0 处违规 → 视为已核实。

## Findings

档位：**通过**

- Critical：0
- Important：0
- Minor：1

### Minor

- M1（report 排版）：报告 L24-28 双语对照表在每条引文中省略了内嵌反引号（如 `node scripts/...` 周围的 `` `...` ``），Markdown 表格嵌套行内代码的局限所致。语义无误，但严格意义上报告所引文本与文件实际内容有 6 处反引号差；不影响对齐结论，作为排版瑕疵记下，不构成打回理由。

## 结论

**PASS** — 实现完全满足 brief 功能要求、Global Constraints 与既有风格；规范文本可执行、双语对齐、插入位置与既有条目零改动均成立；字段引用与策略文件一致。1 处 Minor（报告排版）不阻断通过。

建议：无需派 fixer；如要消 M1，可在报告中以 `<code>...</code>` 或独立 code cell 形式重排表格内嵌代码，但属排版偏好，不强制。