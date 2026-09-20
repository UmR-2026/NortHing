# W23-4 Brief — 台账处置（P2-14 决策关单 + P2-17 按设计暂缓标注）

## 任务标识

W23-4（用户拍板 2026-09-09 决策包⑧⑨项执行）。纯台账单：只改 `docs/status/tech-debt-ledger.md` 两个条目的 Status 行，零代码、零其他条目变动。

## BASE

BASE = 编排者派发正文给出的 7 位 SHA（brief 定版时的 main HEAD = `9e024dd`；其后不得再有非本单允许文件集的 commit，否则 task-gate 起点顺延并以派发正文为准）。

## 背景与 BASE 证据（编排者已实测）

- **用户拍板原文**（2026-09-09，progress.md 台账决策行）：⑧ **P2-17 标记按设计暂缓（等第三调用方）**；⑨ **P2-14 关单（dream-sweep 即设计答案）**。
- **P2-14 现状**（tech-debt-ledger.md:182-187）：`### P2-14: C3 facts dedup is exact-text (fragile); confidence all Med / scope all Workspace (paths unimplemented)`，Status 行 = `- **Status**: active (low priority)`（:187）。
- **P2-17 现状**（tech-debt-ledger.md:204-209）：`### P2-17: init_once_with double-checked-lock skeleton is duplicated between core config and AI factory`，Status 行 = `- **Status**: active (low priority)`（:209）。其 Proposed fix 已写明「if a third caller appears, lift the helper into a shared sync utility module」。
- **Change Protocol 词汇**（ledger 文末）：status 字段合法值 = active / frozen / resolved，变更须带日期与理由；「Resolved: Mark as `resolved` with commit reference. Do not delete entries.」。仓内先例：决策关单有先例（P2-23 resolved — 用户 2026-08-19 拍板清除型条目）。

## 允许文件集

- `docs/status/tech-debt-ledger.md`（修改：**仅 P2-14 与 P2-17 两个条目的 Status 行**）

report 写 `.superpowers/sdd/w23-4-report.md`，不进验收 diff。

## 功能要求

1. **P2-14 关单**：Status 行（:187）改为 resolved，带日期 + 理由 + 决策出处。措辞锚点：`resolved (2026-09-20, W23-4: user ruling 2026-09-09 — dream-sweep is the design answer; decision-only closure, no code change)`。除 Status 行外该条目其他字段一律不动。
2. **P2-17 暂缓标注**：Status 行（:209）改为 frozen，带日期 + 理由 + 决策出处。措辞锚点：`frozen (2026-09-20, W23-4: user ruling 2026-09-09 — deferred by design; lift to shared sync utility only when a third caller appears)`。除 Status 行外该条目其他字段一律不动。
3. **编码卫生（本单最大风险）**：tech-debt-ledger.md 是 GBK/UTF-8 混合编码文件——**必须用 edit 类工具做两处行内替换**，禁止用 PowerShell Set-Content / Out-File / 重定向写整个文件（会 GBK 双重编码污染全文件，W15-2 事故先例）。改完 `git diff` 必须只有 2 行变动（-2/+2），中文字段无 mojibake。
4. 不重排条目、不改其他任何条目的 status、不"顺手"修别的文案。

## Constraints

- commit 逐文件点名 `git add`（1 文件）；message 前缀 `docs:` + `(W23-4)` 后缀，body 注明用户拍板 2026-09-09 ⑧⑨。
- `git diff --stat` 必须 = 1 file changed, 2 insertions(+), 2 deletions(-)；多一行即失败。
- report 贴验证输出 + exit code；结尾状态词。

## 验证（编排者已 BASE 预跑：1/1 命令绿）

1. `node scripts/check-repo-hygiene.mjs` → exit 0（BASE `9e024dd` 实测绿：`Repository hygiene check passed (6 content files scanned, 3907 filenames checked)`；改后复跑）。
2. `git diff --stat` = 1 file changed, 2 insertions(+), 2 deletions(-)（report 贴原文）。
3. `git diff docs/status/tech-debt-ledger.md` 全文贴入 report：必须只有两个 Status 行的 -/+ 对，无第三处改动、无 mojibake。
4. report 附：两个条目改动前后 Status 行逐字对照。

## 禁区

- 禁动 P2-14/P2-17 以外的任何条目或字段。
- 禁动 tech-debt-ledger.md 以外的任何文件。
- 禁整文件重写（见功能要求 3）。

## 报告

写 `.superpowers/sdd/w23-4-report.md`，三节：**改动摘要** / **验证**（4 项）/ **状态**。
