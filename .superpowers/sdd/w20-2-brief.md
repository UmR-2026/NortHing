# W20-2 Brief — P2-24 清账：hygiene 存量（archive 豁免 + 全量脱敏）

## 任务标识

W20-2（清账波次单；P2-24：hygiene 全仓 fallback 口径下 162 个历史文件 384 行含本地绝对路径，ledger 待拍板项；用户 2026-09-08 拍板「豁免 archive + 其余全脱敏」）。

## BASE

`799632f`（main HEAD，代码树干净；派发时 task-gate 起点 = 本 brief 提交后的 docs commit，与 799632f 代码树等价，仅 docs 差异）。

## 背景与 BASE 证据（编排者已实测，53 复审修正分类账）

浅克隆复现 CI fallback 条件：`git clone --depth 1 file:///E:/agent-project/northing <tmpdir>` + `node scripts/check-repo-hygiene.mjs` → exit 1，**391 违规行 / 166 文件 = 384 local-path（162 文件：archive 88 文件 238 行 + 非 archive 74 文件 146 行）+ 7 token-like-secret（archive 3 文件 6 行假串 fixture 讨论文本 + `.superpowers/sdd/reviews/2026-08-23-staged/brief.md` 1 行）**。testFilePattern 已豁免测试文件，无 fixture 中招。

**用户拍板 2026-09-08（方案 a）**：staged brief.md 加入脱敏清单（第 75 文件）；**fallback 口径永久余 6 条 archive 假串 token 红**（CI 已 fetch-depth:2 不走 fallback，日常不受影响）；历史冻结不破、token 安全扫描不弱化。

## 允许文件集

- `scripts/check-repo-hygiene.mjs`（修改，仅加 archive 豁免，见功能 1）
- `docs/status/tech-debt-ledger.md`（修改，仅 P2-24 条目 status 翻转——家规 2 同 commit）
- 附录 A 所列 75 个待脱敏文件（74 + `.superpowers/sdd/reviews/2026-08-23-staged/brief.md`，逐字清单，不多不少；发现清单外 **local-path 型**命中 ⇒ BLOCKED 上交，不得自行扩面）

report 写 `.superpowers/sdd/w20-2-report.md`，不进本单验收 diff（不 commit、不 git add）。

## 功能要求

1. **archive 豁免（仅 local-path 扫描）**：`check-repo-hygiene.mjs` 新增 `localPathExemptPaths = [/^docs\/archive\//]`，并入 `scanLocalPaths` 判定（现 line ~238-240：`scanLocalPaths = !isTestFile && !isLocalPathExempt`）。**token/私钥扫描对 archive 保持生效**（豁免只作用于 local-path 一项）。注释写明依据：用户拍板 2026-09-08，归档=冻结历史。
2. **75 文件全脱敏**：命中行的本地绝对路径替换为占位符。**决策表钉死（53 复审实测：本批 146 个命中 token 零个指向本仓 checkout 根）——local-path 型一律 `<LOCAL_PATH>`；token 型命中（仅第 75 文件 1 行）替换为 `<TOKEN>`；`<REPO_ROOT>` 在本批 BASE 实测为 0 命中，验收 diff 出现任何 `<REPO_ROOT>` 即可疑打回**（防 `...projects\northing` 类含仓名字样的误判）。只替换命中 token，行内其余内容逐字不动。
3. **编码安全**：替换必须 ASCII/字节级安全（命中模式全 ASCII；75 文件经 53 实测全部严格 UTF-8，GBK 恐惧不成立但仍禁整文件重编码）；**禁 split(/\r?\n/)+join('\n') 式行重组**（会把 CRLF 归一成 LF），替换作用于完整内容串/Buffer；改后 `git diff --stat` 每文件 +/- 相等、净差必须为 0、全批变更行 ≤147（146 local-path 行 + 第 75 文件 1 行 token 行）。
4. ledger P2-24 翻转 resolved（同 commit，注明处置口径与拍板日期）。

## Constraints

- commit 逐文件点名 `git add`（禁 -A，77 文件（75 脱敏 + 脚本 + ledger）逐个或按目录显式列名——`git add docs/notes/ .superpowers/sdd/reports/` 这类目录点名可接受，禁裸 `-A`/`git add .`）；message 前缀 `chore(hygiene):` + `(W20-2)` 后缀，body 引用户拍板「2026-09-08 混合：豁免 archive + 其余全脱敏」。
- report 贴原文输出 + exit code；绝对路径用 `<REPO_ROOT>` 占位。
- 脚本改动最小化：豁免逻辑 ≤5 行。
- 收口前必跑 `node scripts/check-repo-hygiene.mjs`。
- report 结尾状态词。
- 本单触碰 `metaRatchetPaths`（check-repo-hygiene.mjs），验收走双 judge 车道（代码级批准已委托 judge 流程，用户拍板 2026-09-07）。

## 禁区

- 禁改 docs/archive/ 下任何文件（豁免而非脱敏——历史冻结）。
- 禁改 hygiene 脚本的 token/私钥/filename 扫描逻辑。
- 禁扩大豁免路径清单（仅 docs/archive/）。
- 禁动豁免/脱敏以外的任何文件；禁重编码任何文件。

## 验证

1. **BASE 红态**（编排者已实测 391=384+7，report 复跑贴原文）：浅克隆 + `node scripts/check-repo-hygiene.mjs` → exit 1。
2. **修复后浅克隆终态（方案 a 口径）**：浅克隆 TIP 重跑 → **exit 1 且恰好余 6 条 token-like-secret 违规（全部在 docs/archive/），local-path 违规全清零**。**防空洞绿钉死**：report 贴完整首行输出，显式断言含 `WARNING: full-repo scan fallback active` 行 + content files scanned ≥3000（注意：scanned 计数仅 pass 路径打印，终态 exit-1 运行无此行——改以 WARNING 行 + 恰好 6 条违规跨 3 个 archive 文件的完整清单为全量范围证明，并附克隆内 `git ls-files` 计数（BASE 实测 3876 ≥3000）作旁证）；**输出重定向到克隆外**（重定向进克隆会让 localChangedFiles 非空、扫描塌缩为单文件假绿——53 实测踩中）。
3. **豁免范围探针（断言式双向）**：在修复后浅克隆内 ① 向某 docs/archive/ 文件追加一行 token 形态串 ⇒ 输出新增该文件「contains a token-like secret」精确条目（方案 a 下余留 6 条已天然证明一半，此步补差分）；② 阴性对照：向某 docs/archive/ 文件追加一行 Windows 形态本地绝对路径（盘符字母 + 冒号 + `\Users\x\y` 段拼接而成的串；本 brief 不内嵌该字面量——内嵌会使 brief 自身成为清单外 local-path 命中，53 复审实测）⇒ 断言**无**新增 local-path 违规。然后丢弃克隆。
4. 主仓 `node scripts/check-repo-hygiene.mjs` —— 绿（日常增量口径不回归；archive 豁免侧只经克隆验证，分工正确）。
5. `git diff --stat` 复核：75 文件每文件 +/- 相等、净差 0、全批变更行 ≤147（口径同功能 3）。
6. report 附：分类账（豁免 88 文件 / 脱敏 75 文件 / 永久残留 6 条 archive token）+ 脱敏抽样 diff 段（3 处）+ `<REPO_ROOT>` grep 零命中证明（grep 范围 = 验收 diff / 75 文件内容；不做全仓 grep——report 自身按 Constraints 用 `<REPO_ROOT>` 占位会污染全仓结果）。

## skill 前置

无强相关（hygiene 闸专场）；如参考口径可读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md` 的 NortHing 部署注记，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w20-2-report.md`，必含三节：**改动摘要**（含分类账）/ **验证**（6 项原文输出 + exit code）/ **状态**（状态词结尾）。

## 附录 A：待脱敏文件清单（75，逐字）
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
- `.superpowers/sdd/reviews/2026-08-23-staged/brief.md`（第 75 个，token 型命中，方案 a 拍板加入）

