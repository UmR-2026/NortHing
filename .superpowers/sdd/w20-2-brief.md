# W20-2 Brief — P2-24 清账：hygiene 存量（archive 豁免 + 全量脱敏）

## 任务标识

W20-2（清账波次单；P2-24：hygiene 全仓 fallback 口径下 162 个历史文件 384 行含本地绝对路径，ledger 待拍板项；用户 2026-09-08 拍板「豁免 archive + 其余全脱敏」）。

## BASE

`799632f`（main HEAD，代码树干净；派发时 task-gate 起点 = 本 brief 提交后的 docs commit，与 799632f 代码树等价，仅 docs 差异）。

## 背景与 BASE 证据（编排者已实测）

浅克隆复现 CI fallback 条件：`git clone --depth 1 file:///E:/agent-project/northing <tmpdir>` + `node scripts/check-repo-hygiene.mjs` → exit 1，384 行 local-absolute-path 违规 / 162 文件。分类实测：docs/archive/** = 88 文件 236 行（冻结历史）；其余 = 74 文件 148 行（SDD 历史证据 + 活文档 + 活 skill）。testFilePattern 已豁免测试文件，无 fixture 中招。

## 允许文件集

- `scripts/check-repo-hygiene.mjs`（修改，仅加 archive 豁免，见功能 1）
- `docs/status/tech-debt-ledger.md`（修改，仅 P2-24 条目 status 翻转——家规 2 同 commit）
- 附录 A 所列 74 个待脱敏文件（逐字清单，不多不少；发现清单外命中 ⇒ BLOCKED 上交，不得自行扩面）

report 写 `.superpowers/sdd/w20-2-report.md`，不进本单验收 diff（不 commit、不 git add）。

## 功能要求

1. **archive 豁免（仅 local-path 扫描）**：`check-repo-hygiene.mjs` 新增 `localPathExemptPaths = [/^docs\/archive\//]`，并入 `scanLocalPaths` 判定（现 line ~238-240：`scanLocalPaths = !isTestFile && !isLocalPathExempt`）。**token/私钥扫描对 archive 保持生效**（豁免只作用于 local-path 一项）。注释写明依据：用户拍板 2026-09-08，归档=冻结历史。
2. **74 文件全脱敏**：命中行的本地绝对路径替换为占位符——指向本仓内部的 → `<REPO_ROOT>`；其他本机路径（用户配置目录等）→ `<LOCAL_PATH>`。只替换命中 token，行内其余内容逐字不动。
3. **编码安全**：替换必须 ASCII/字节级安全（命中模式全 ASCII）；**禁用任何整文件重编码工具**（仓内有 GBK 混合编码文件先例）；改后 `git diff --stat` 行数差应≈0（只变内容不变行数）。
4. ledger P2-24 翻转 resolved（同 commit，注明处置口径与拍板日期）。

## Constraints

- commit 逐文件点名 `git add`（禁 -A，76 文件逐个或按目录显式列名——`git add docs/notes/ .superpowers/sdd/reports/` 这类目录点名可接受，禁裸 `-A`/`git add .`）；message 前缀 `chore(hygiene):` + `(W20-2)` 后缀，body 引用户拍板「2026-09-08 混合：豁免 archive + 其余全脱敏」。
- report 贴原文输出 + exit code；绝对路径用 `<REPO_ROOT>` 占位。
- 脚本改动最小化：豁免逻辑 ≤5 行。
- 收口前必跑 `node scripts/check-repo-hygiene.mjs`。
- report 结尾状态词。

## 禁区

- 禁改 docs/archive/ 下任何文件（豁免而非脱敏——历史冻结）。
- 禁改 hygiene 脚本的 token/私钥/filename 扫描逻辑。
- 禁扩大豁免路径清单（仅 docs/archive/）。
- 禁动豁免/脱敏以外的任何文件；禁重编码任何文件。

## 验证

1. **BASE 红态**（已实测，report 复跑贴原文）：浅克隆 + `node scripts/check-repo-hygiene.mjs` → exit 1，384 违规。
2. **修复后浅克隆绿**：浅克隆 TIP 重跑 → exit 0（豁免生效 + 脱敏无残余的总证明）。
3. **豁免范围探针**：在修复后浅克隆内向某 docs/archive/ 文件临时追加一行 token 形态串 → 脚本仍红（证明 token 扫描未被豁免）；然后丢弃克隆。
4. 主仓 `node scripts/check-repo-hygiene.mjs` —— 绿（日常增量口径不回归）。
5. `git diff --stat` 复核：74 文件行数变化 ≈0（逐文件 +0/-0 或极小）。
6. report 附：分类账（豁免 88 文件 / 脱敏 74 文件）+ 脱敏抽样 diff 段（3 处）。

## skill 前置

无强相关（hygiene 闸专场）；如参考口径可读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md` 的 NortHing 部署注记，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w20-2-report.md`，必含三节：**改动摘要**（含分类账）/ **验证**（6 项原文输出 + exit code）/ **状态**（状态词结尾）。

## 附录 A：待脱敏文件清单（74，逐字）
- `.agents/skills/northhing-onboarding/SKILL.md`
- `.agents/skills/northhing-v3-workflow/SKILL.md`
- `.opencode/model-capability-notes.md`
- `.opencode/sdd/kernel/b8-report.md`
- `.superpowers/sdd/briefs/W15-1f-boundary-anyhow.md`
- `.superpowers/sdd/briefs/W15-1g-symlink-fence-order.md`
- `.superpowers/sdd/consult-room/handoff-20260808.md`
- `.superpowers/sdd/consult-room/handoff-20260815.md`
- `.superpowers/sdd/consult-room/handoff-20260824.md`
- `.superpowers/sdd/consult-room/merge-main-resolve-brief.md`
- `.superpowers/sdd/consult-room/merge-main-resolve-report.md`
- `.superpowers/sdd/consult-room/plan.md`
- `.superpowers/sdd/consult-room/task-01-brief.md`
- `.superpowers/sdd/consult-room/task-ef-e1-archive-brief.md`
- `.superpowers/sdd/consult-room/task-ef-e2-space-report.md`
- `.superpowers/sdd/consult-room/task-ef-e3-settings-brief.md`
- `.superpowers/sdd/consult-room/task-ef-e4-fix1-scrollbar-brief.md`
- `.superpowers/sdd/consult-room/task-ef-e4-fix1-scrollbar-report.md`
- `.superpowers/sdd/consult-room/task-ef-e4-onboarding-brief.md`
- `.superpowers/sdd/consult-room/task-locale-repair-brief.md`
- `.superpowers/sdd/consult-room/task-p0a-bridge-brief.md`
- `.superpowers/sdd/consult-room/task-p0b-send-brief.md`
- `.superpowers/sdd/consult-room/task-p0c-approval-brief.md`
- `.superpowers/sdd/consult-room/task-p1a-chronicle-brief.md`
- `.superpowers/sdd/consult-room/task-p1b-fix-brief.md`
- `.superpowers/sdd/consult-room/task-p1b-settings-brief.md`
- `.superpowers/sdd/consult-room/task-p1c-mcp-env-brief.md`
- `.superpowers/sdd/consult-room/task-w27-fluid-cards-brief.md`
- `.superpowers/sdd/packages/AGY-FIX-review.md`
- `.superpowers/sdd/reports/agy-429-path-review.md`
- `.superpowers/sdd/reports/AGY-FIX-review.md`
- `.superpowers/sdd/reports/agy-resolution-review.md`
- `.superpowers/sdd/reports/audit-fix-mechanical-report.md`
- `.superpowers/sdd/reports/cargo-audit-2026-08-23.txt`
- `.superpowers/sdd/reports/startup-hang-render-path.md`
- `.superpowers/sdd/reports/startup-hang-trace-report.md`
- `.superpowers/sdd/reports/task-p0b-send-report.md`
- `.superpowers/sdd/reports/task-p1a-chronicle-report.md`
- `.superpowers/sdd/reports/task-p1b-settings-report.md`
- `.superpowers/sdd/reports/task-p1c-fix1-dead-helpers-report.md`
- `.superpowers/sdd/reports/task-p1c-mcp-env-report.md`
- `.superpowers/sdd/reports/task-russh-bump-review.md`
- `.superpowers/sdd/reports/W15-1f-report.md`
- `.superpowers/sdd/reports/W15-1g-report.md`
- `.superpowers/sdd/reports/W15-1g-review.md`
- `.superpowers/sdd/reports/w15-1h-report.md`
- `.superpowers/sdd/reports/w15-1i-report.md`
- `.superpowers/sdd/reports/w15-1j-report.md`
- `.superpowers/sdd/reports/w15-1k-report.md`
- `.superpowers/sdd/reports/w15-1l-report.md`
- `.superpowers/sdd/reviews/w14-1c-4b-package.txt`
- `.superpowers/sdd/reviews/w15-1b-1-package.txt`
- `.superpowers/sdd/w14-1c-4b-report.md`
- `.superpowers/sdd/w14-1e-report.md`
- `.superpowers/sdd/w14-1e-review.md`
- `.superpowers/sdd/w15-1b-1-report.md`
- `.superpowers/sdd/w15-1b-3-brief.md`
- `.superpowers/sdd/w15-1b-3-report.md`
- `.superpowers/sdd/w15-1h-brief.md`
- `.superpowers/sdd/w15-1i-brief.md`
- `.superpowers/sdd/w15-1j-brief.md`
- `.superpowers/sdd/w15-1k-brief.md`
- `.superpowers/sdd/w15-1l-brief.md`
- `docs/architecture/homerail-architecture-analysis.md`
- `docs/architecture/plugin-system-proposal.md`
- `docs/migration-2026-07-16/northing-split-execution-2026-07.md`
- `docs/notes/preflight-skill-check.md`
- `docs/PROJECT_STATE.md`
- `docs/status/full-review-2026-08-16.md`
- `docs/superpowers/plans/2026-06-18-agent-app-rebuild.md`
- `docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md`
- `docs/superpowers/plans/2026-06-19-plan-compliance-checker-impl.md`
- `docs/superpowers/plans/round44-r49-review-guide-2026-07-07.md`
- `docs/superpowers/specs/2026-06-26-frontend-onboarding-design.md`

