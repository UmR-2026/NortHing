# W18-2 Brief — only-down 机械化 + headroom floor + cap-and-archive action

## 任务标识

W18-2（波次计划：`.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md` v2；相位映射 0.2 + 0.5 + -1.5）。前置：W18-1 已落地 `validateManifest` + `FIELD_WHITELIST` 表驱动结构（本单加表项 + 新增 action/authorization 语义校验分支，不动 W18-1 表驱动结构）。

## BASE

`f2c55b8`（main HEAD，工作树已验干净）

## 允许文件集

- `scripts/verify-rot-budget.mjs`（修改）
- `scripts/rot-budget.json`（**仅**给 `dir_entries:.superpowers/sdd` entry 增 `action` 字段；任何 ceiling/key 不得动）

report 写 `.superpowers/sdd/w18-2-report.md`，不进本单验收 diff。本单不新增 fixture 文件（`--base` 对比类 fixture 用 selftest 内联合成，见功能要求 5）。

## 背景

家规 7「ceiling 只降不升」目前靠 commit message 自觉；headroom floor（-1.5）与 cap-and-archive 结构化（0.5）无机器执行。本单给 `verify-rot-budget.mjs` 加 `--base <sha>` 模式做 BASE/TIP manifest 机械对比，并落地 floor 规则与 action 字段。

## 功能要求

1. **`--base <sha>` 模式**：CLI 增加 `--base` flag（参照 `verify-task-gate.mjs` 的 `parseArgs` 风格自行最小实现；无 `--base` 时全部对比逻辑跳过，exit code 与 violation 语义与现状一致——功能 3 的零余量 warning 除外，它在两种模式下都新增 warning 输出）。有 `--base` 时：用 `execFileSync('git', ['show', '<sha>:scripts/rot-budget.json'], { cwd: projectRoot })` 取 BASE manifest（parse 失败/缺文件 ⇒ violation），与 TIP manifest 逐 key 对比：
   - **only-down**：TIP ceiling > BASE ceiling ⇒ violation，除非 TIP 该 entry 携带**未过期**结构化授权 `authorization: {reason: string, commit: string, expires: "YYYY-MM-DD"}`；`expires` 按 UTC 日期粒度比较，`expires >= 今日(UTC)` 为 live，过期 ⇒ 视同无授权。
   - **禁删指标**：BASE 有而 TIP 无的 key ⇒ violation。
2. **headroom floor**（-1.5，仅 `--base` 模式）：`file-lines` 类 ceiling 下调（TIP < BASE）必须满足 `newCeiling >= current + max(5, ceil(current * 0.05))`，其中 `current` = 本次运行实测行数；违反 ⇒ violation 且文案指引走 exception lease 通道（W18-3 落地，本单只出文案）。
3. **零余量 warning**（两种模式都做）：任何登记 entry 实测 current == ceiling ⇒ warning（非 violation）列出并指引 lease 通道。当前预期清单 5 项：css.rs、selectors.rs、manager.rs、unix_epoch_inline、`dir_entries:docs/design`（1/1）——selftest 用合成数据钉机制即可，真实清单以 report 贴出的实际输出为准。
4. **cap-and-archive 结构化 action**（0.5）：
   - `FIELD_WHITELIST` 表项扩展（只动表）：`dir-entry-count` 加 `action`；全 kind 公共加 `authorization`。`action` 校验：存在时必须是 object 且 `type` 为非空字符串；`type === "cap-and-archive"` 时必须含非空字符串 `archiveTo`。`authorization` 校验：存在时必须是 object 且 `reason`/`commit`/`expires` 三个非空字符串字段齐全（`expires` 必须匹配 `YYYY-MM-DD`）。
   - `scripts/rot-budget.json` 的 `dir_entries:.superpowers/sdd` entry 增 `"action": {"type": "cap-and-archive", "archiveTo": "docs/archive/sdd-artifacts/"}`。
   - `dir-entry-count` 触发超限且 entry 带 `action.type === "cap-and-archive"` ⇒ violation 文案改为专用指引（含 `archiveTo` 路径与归档动作说明），不再用通用文案。
5. **selftest 扩展**（内联，沿用 W18-1 的 record/PASS/FAIL 结构；`--base` 对比类用例在 tmpdir 合成 git repo：`git init` + 提交 BASE manifest 文件后运行。实现提示：新 repo 首次 commit 需 `git -c user.name=test -c user.email=test@test commit`；合成 manifest 若含 dir-entry-count entry 须先在合成根建对应目录，否则触发 directory-not-exists violation）：
   - 负例：未授权上调（红）/ 授权已过期上调（红）/ 违反 floor 的下调（红）/ 删指标（红）/ 畸形 `action`（红，如缺 `archiveTo`）/ 畸形 `authorization`（红，如缺 `expires`）；
   - 正例：授权未过期上调（绿）/ `expires` = 今日 UTC（绿，日期边界档 A）/ `expires` = 昨日（红，日期边界档 A）/ 合规下调满足 floor（绿）/ cap-and-archive 触发时输出含 `archiveTo` 指引（断言语义而非逐字）。
   - 既有 11 项 selftest 必须保持全绿。
6. 纯 Node 标准库，零新依赖；输出 English-only。

## Constraints（逐字自波次计划 Global Constraints，与本单相关项）

- 任何 ceiling 不得上调；manifest 删指标视同上调（禁止）。本单对 `rot-budget.json` 的唯一改动 = 上述 `action` 字段新增。
- commit 逐文件点名 `git add`（禁 `-A`）；message 前缀 `feat(scripts):` + `(W18-2)` 后缀。
- 不得用 Markdown 说明替代机器校验；report 中验证命令贴原文输出 + exit code。
- 产物内本地绝对路径用 `<USERPROFILE>` 等占位（hygiene 闸）；收口前必跑 `node scripts/check-repo-hygiene.mjs`。
- report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
- 本单触碰 `metaRatchetPaths`（`scripts/verify-rot-budget.mjs`），验收走双 judge + 用户拍板车道（流程项，对实现者零额外要求）。
- checker 改动**先负向 fixture 后实现**：负例必须先在旧代码上确认行为、在新代码上全红，正向全绿（GC4）。本单旧代码无 `--base` 模式——report「旧代码行为确认」节如实记录旧代码对该 flag 的行为（忽略/报错），并确认各负例语义在旧代码下不可能被拦截。

## 禁区

- 禁改 `ceiling`/`key`/`pattern`/`note` 等任何既有 manifest 内容（唯一例外 = 功能要求 4 的 `action` 新增）。
- 禁改 `scripts/workflow-policy.json`、`scripts/verify-task-gate.mjs`、`scripts/fixtures/` 及 `scripts/` 下任何其他文件。
- 禁改 grep 扫描范围/收集器/豁免语义（读数零漂移，`.superpowers/sdd` 的 committed 口径对照按 W18-1 测量条件）。
- 禁为过关调整 fixture 判定标准。

## 验证

编排者已在 BASE `f2c55b8` 预跑基线命令（已实证）：

1. `node scripts/verify-rot-budget.mjs` —— 基线绿；读数基线同 W18-1（483/940/370/69/104；44/48、1/1；6 god-file；1368 files；`.superpowers/sdd` committed 口径 **65/400**，以 `git ls-tree` 计数为准）。修复后逐一对照（committed 口径，`git stash -u` 后运行；环境漂移 +N 注明即可）。
2. `node scripts/verify-rot-budget.mjs --selftest` —— 基线 11 绿；交付后全绿（含本单新增用例）。
3. `node scripts/verify-rot-budget.mjs --base eaa592f` —— 交付后必须绿（本单未动任何 ceiling，only-down/floor/删指标均不触发。`eaa592f` 与计划任务卡钉死的 `df5c1ce` 的 manifest 同 blob，等价替换，已实证）。
4. `node scripts/verify-rot-budget.test.mjs` —— 绿（12 现存测试回归）。
5. `pnpm run check:repo-hygiene` —— 绿。

## skill 前置

阅读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`，遵循其中与本任务相关的约定，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w18-2-report.md`，必含三节：**改动摘要**（`--base` 对比算法 / floor 公式 / action 与 authorization 校验 / 零余量 warning 机制）/ **验证**（验证节全部 5 条命令原文输出 + exit code + 旧代码行为确认记录 + 零余量 warning 真实输出清单）/ **状态**（状态词结尾）。
