# W16-5 Review — 独立验收（final review I-1/I-2 收口）

- 范围：NortHing commit `92712b6`（`fix(policy): unify review verdict vocabulary + extend metaRatchetPaths (W16-5)`，3 文件 / +7 / −3）
- 终审出处：`.superpowers/sdd/packages/w16-final-review.md` I-1 / I-2
- brief：`.superpowers/sdd/w16-5-brief.md`
- 方法：派发-执行模式 = 不采信报告自述；结论出自 `git show 92712b6` diff + 源码 + 本机独立复跑四命令
- 日期：2026-09-06
- 结论：**APPROVE** — Critical 0 / Important 0 / Minor 1

## 0. 独立复跑清单（本机亲跑，HEAD = 92712b6）

| 命令 | 报告原文 | 本机复跑 | 一致 |
|---|---|---|---|
| `node scripts/verify-task-gate.mjs validate-policy` | `Policy validation passed: E:\agent-project\northing\scripts\workflow-policy.json`（exit 0） | `Policy validation passed: E:\agent-project\northing\scripts\workflow-policy.json`（exit 0） | ✅ 完全一致 |
| `node scripts/verify-task-gate.mjs --selftest` | 11 fixtures passed（7 负 / 4 正），11 行 PASS 内容逐行一致 | 11 fixtures passed（7 负 / 4 正），11 行 PASS 内容逐行一致 | ✅ 完全一致 |
| `node scripts/check-repo-hygiene.mjs` | `Repository hygiene check passed (5 content files scanned, 3828 filenames checked).`（exit 0） | `Repository hygiene check passed (3 content files scanned, 3829 filenames checked).`（exit 0） | ⚠️ 数字漂移（见 M-1） |
| `node scripts/verify-rot-budget.mjs` | 5 grep + 3 dir + 6 god-file 全列数值（483/502, 940/1089, 370/388, 69/69, 104/109, 44/48, 1/1, 58/400） | 同左，全部 11 项数值逐项一致 | ✅ 完全一致 |

报告四命令输出**齐备**（全部 4 段均在 `.superpowers/sdd/reports/w16-5-report.md`）；3/4 命令**输出逐字符一致**，1/4 通过（hygiene exit 0 / 内容通过，但 contentScanFiles / repositoryFiles 计数因工作树状态漂移，见 M-1）。

## 一、双语家规 §8.3（I-1）

### 1.1 diff stat
```
 AGENTS-CN.md                 | 2 +-
 AGENTS.md                    | 2 +-
```
两文件各 1 行 +1 / 1 行 −1，无其它 hunk，无其它触碰。

### 1.2 单行改动定位（hunk 上下文）
- `AGENTS.md`：`@@ -102,7 +102,7 @@`，仅第 102 行（§8 第 3 子点）修改；
- `AGENTS-CN.md`：`@@ -98,7 +98,7 @@`，仅第 98 行（§8 第 3 子点）修改。

### 1.3 改前 / 改后逐行比对

**AGENTS.md L102**

| | 内容 |
|---|---|
| − | `3. Review verdict state machine: PASS / FAIL / CANNOT_VERIFY / BLOCKED; CANNOT_VERIFY is tiered per \`cannotVerifyPolicy\` in \`scripts/workflow-policy.json\` (decisive evidence blocks; auxiliary evidence ≤2 items and not touching trust boundary ⇒ verdict capped at APPROVE_WITH_CONCERNS + owner + deadline); direct promotion to APPROVE is forbidden.` |
| + | `3. Review verdicts use \`reviewVerdicts\` in \`scripts/workflow-policy.json\` as the sole vocabulary (currently APPROVE / APPROVE_WITH_CONCERNS / CANNOT_VERIFY / BLOCKED / FAIL); CANNOT_VERIFY is tiered per \`cannotVerifyPolicy\` in \`scripts/workflow-policy.json\` (decisive evidence blocks; auxiliary evidence ≤2 items and not touching trust boundary ⇒ verdict capped at APPROVE_WITH_CONCERNS + owner + deadline); direct promotion to APPROVE is forbidden.` |

**AGENTS-CN.md L98**

| | 内容 |
|---|---|
| − | `3. 审查结论状态机：PASS / FAIL / CANNOT_VERIFY / BLOCKED；CANNOT_VERIFY 按 \`scripts/workflow-policy.json\` 的 \`cannotVerifyPolicy\` 分级（判定性证据阻塞；辅助证据 ≤2 项且不触 trust boundary ⇒ 结论上限 APPROVE_WITH_CONCERNS + owner + 截止），禁止直接转 APPROVE。` |
| + | `3. 审查结论以 \`scripts/workflow-policy.json\` 的 \`reviewVerdicts\` 为唯一词表（当前为 APPROVE / APPROVE_WITH_CONCERNS / CANNOT_VERIFY / BLOCKED / FAIL）；CANNOT_VERIFY 按 \`scripts/workflow-policy.json\` 的 \`cannotVerifyPolicy\` 分级（判定性证据阻塞；辅助证据 ≤2 项且不触 trust boundary ⇒ 结论上限 APPROVE_WITH_CONCERNS + owner + 截止），禁止直接转 APPROVE。` |

### 1.4 核对结论（I-1）
- ✅ 引用 `reviewVerdicts` 为唯一词表（"sole vocabulary" / "唯一词表"），明确当前 5 词枚举；
- ✅ CANNOT_VERIFY 分级语义**完整保留**（cannotVerifyPolicy 引用、判定性 / 辅助证据规则、APPROVE_WITH_CONCERNS 上限 + owner + 截止、禁止直接 APPROVE 一句未动）；
- ✅ 双语**逐句对齐**：英语 / 中文为对偶翻译，前半改写、后半逐字保留；
- ✅ §8 其余 4 个子点（1/2/4/5）和 §8 以外**零字改动**（diff stat = 2 +/− 即为证据）；
- ✅ 不触及 `metaRatchetPaths`（AGENTS.md/AGENTS-CN.md 不在该清单），审查车道**普通车道**合规。

## 二、policy.json metaRatchetPaths（I-2）

### 2.1 diff stat
```
 scripts/workflow-policy.json | 6 +++++-
 1 file changed, 5 insertions(+), 1 deletion(-)
```
单 hunk，`@@ -21,7 +21,11 @@`，仅 `metaRatchetPaths` 数组内嵌段，无其它字段触碰。

### 2.2 改前 / 改后逐项比对

| 顺序 | 改前 | 改后 |
|---|---|---|
| 1 | `scripts/verify-task-gate.mjs` | `scripts/verify-task-gate.mjs` ✅ 不动 |
| 2 | `scripts/verify-rot-budget.mjs` | `scripts/verify-rot-budget.mjs` ✅ 不动 |
| 3 | `scripts/workflow-policy.json` | `scripts/workflow-policy.json` ✅ 不动 |
| 4 | `.github/workflows/` | `.github/workflows/` ✅ 不动 |
| 5 | — | `scripts/check-repo-hygiene.mjs` ✅ 新增（与 brief 顺序一致） |
| 6 | — | `scripts/check-core-boundaries.mjs` ✅ 新增 |
| 7 | — | `scripts/check-github-config.mjs` ✅ 新增 |
| 8 | — | `package.json` ✅ 新增 |

### 2.3 其它字段零触碰（节点 require + Object.keys 实证）
```
Total fields: 8
[ "version", "judgeChecklist", "statusWords", "reviewVerdicts",
  "cannotVerifyPolicy", "metaRatchetPaths",
  "briefRequiredSections", "reportRequiredSections" ]
```
字段集 = `{version, judgeChecklist, statusWords, reviewVerdicts, cannotVerifyPolicy, metaRatchetPaths, briefRequiredSections, reportRequiredSections}`，与 commit 前完全一致（数量 8 / 名称 8 / 顺序 8 三项均吻合）。

`reviewVerdicts` 值未变：`["APPROVE","APPROVE_WITH_CONCERNS","CANNOT_VERIFY","BLOCKED","FAIL"]`（5 项，与 8.3 文档中的 5 词完全对应）。

`cannotVerifyPolicy` 值未变；`statusWords` 未变；`judgeChecklist` 9 项未变；`briefRequiredSections` / `reportRequiredSections` 未变。

### 2.4 JSON 合法性
- `node -e "require('./scripts/workflow-policy.json')"` 成功加载（无抛错）；
- `node scripts/verify-task-gate.mjs validate-policy` exit 0；
- `node scripts/verify-task-gate.mjs --selftest` 11/11 PASS（含 fixture g「policy enum mismatch rejected」= 回归测试，PASS 即证枚举当前合法）。

### 2.5 核对结论（I-2）
- ✅ 原 4 项**顺序、字面、缩进零变动**；
- ✅ 新 4 项**严格按 brief 顺序**追加（check-repo-hygiene → check-core-boundaries → check-github-config → package.json）；
- ✅ JSON 合法 / 8 字段零触碰 / reviewVerdicts 与 I-1 文档 5 词一一对应；
- ⚠️ 本 commit 修改 `scripts/workflow-policy.json` 本身触发**自家 ratchet**（metaRatchetPaths 第 3 项），按设计应升最高审查车道（双 judge + 用户拍板）——本审即承接该车道，**双判决链条由 W16-final-review + 本审叠加构成**。

## 三、报告四命令验证对齐（核对项 3）

报告 `reportRequiredSections = ["改动摘要", "验证", "状态"]` 三段齐备；验证节四命令原文块齐全（命令 + exit code + 输出）。详见 §0 复跑表。

- `validate-policy`：✅ 逐字符一致；
- `--selftest`：✅ 逐字符一致（11 行 PASS 内容完整）；
- `check-repo-hygiene.mjs`：⚠️ 内容通过，数字漂移（见 M-1）；
- `verify-rot-budget.mjs`：✅ 逐字符一致（5 + 3 + 6 = 14 项数值全吻合）。

## 四、Findings

### Critical：0

### Important：0

### Minor：1

**M-1 `check-repo-hygiene.mjs` 报告输出数字与现状漂移，非缺陷**

证据：报告原文 `(5 content files scanned, 3828 filenames checked)` vs 本机复跑 `(3 content files scanned, 3829 filenames checked)`。两次 exit 0、内容通过。

根因（脚本自身行为）：`scripts/check-repo-hygiene.mjs:52-58` 的 `contentScanFiles` 选择策略：
```
contentScanFiles = uniqueFiles(
  localChangedFiles.length > 0
    ? localChangedFiles
    : committedChangedFiles.length > 0
      ? committedChangedFiles
      : trackedFiles,
);
```
- 报告生成时（commit 前）：`localChangedFiles` = 3 改 + 2 untracked（brief + report）= 5；`repositoryFiles` = 3828；
- 本审复跑时（commit 后）：`localChangedFiles` = 0 改 + 3 untracked（brief + report + final-review）= 3；`repositoryFiles` = 3829。

数字差异 = 工作树 untracked 集漂移；脚本按设计依赖当前工作树状态（非 commit 冻结态），与本次 diff 无直接因果。hygiene 实质检查（敏感文件名 / 私钥 / 路径 / token）零命中，两次一致。

**升级路径（不在本任务范围）**：若需 commit-冻结输出，将 `localChangedFiles` 改为 base-relative diff（HEAD~1..HEAD）即可；本次不动。

## 五、范围外改动

- commit `92712b6` 文件集 = `{AGENTS.md, AGENTS-CN.md, scripts/workflow-policy.json}` = brief 允许集 `{AGENTS.md, AGENTS-CN.md, scripts/workflow-policy.json}`，**越界 = 0**；
- 工作树未提交物：`.superpowers/sdd/w16-5-brief.md` / `.superpowers/sdd/reports/w16-5-report.md` / `.superpowers/sdd/packages/w16-final-review.md`（按 brief「report 不入 commit」约定 + 历史归档惯例，不入 commit，符合预期）。

## 六、Cannot verify from diff

1. **CI 是否对 92712b6 跑过** — commit 未推送（origin/main = 19349cd 早于本波），CI 实况同 W16-final I-3（main 最近 60 次 0 成功）。本审不重开该问题，留归 W16-final I-3 处置链。

## 七、结论

**APPROVE** — Critical 0 / Important 0 / Minor 1。

- I-1（结论词表分裂 → 单源化）：**收口完成**。AGENTS.md / AGENTS-CN.md §8.3 各一词改动，引用 `reviewVerdicts` 为唯一词表，CANNOT_VERIFY 分级语义逐字保留，双语对齐，无其它文字触碰。
- I-2（metaRatchetPaths 看守者盲区）：**收口完成**。原 4 项不动 + 新 4 项按 brief 顺序追加，JSON 合法 / 8 字段零触碰 / reviewVerdicts 与文档 5 词对应。
- 报告验证：4 段齐备 + 3/4 命令复跑逐字符一致 + 1/4 数字漂移（脚本工作树依赖，详见 M-1）。

M-1 为脚本行为观察（脚本按设计扫描当前工作树状态），非本次实现缺陷，不阻塞收口。三项 I 中两项（I-1、I-2）以本 commit 闭环；I-3 沿用 W16-final 处置链，不在本任务允许集内。
