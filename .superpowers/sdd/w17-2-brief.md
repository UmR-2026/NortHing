# W17-2 Brief：hygiene 口径修复 + 打包工作流 Windows 收窄

- 任务标识：W17-2
- BASE：`56b752f`（main HEAD，已推送；CI run 33982832690：Windows Tests/Build 绿、唯 repo-hygiene 红）
- 来源：CI run 33982832690 hygiene 失败根因链 + 用户拍板 2026-09-05「Windows 限定」（W17-1 同拍板的延伸）+ judge-53 范围外观察（nightly/cli-package 非 Windows leg）

## 背景（两段，实现前必读）

1. **hygiene 红的根因**：`actions/checkout` 默认 `fetch-depth: 1`（浅克隆）→ `HEAD^1` 不存在 → `check-repo-hygiene.mjs:49-57` 的 fallback 链走到**全仓 tracked 扫描** → 历史存量文件（约 170 个含本地路径的文档/测试）全部炸出。CI 上 hygiene 从未按设计口径（扫本次 commit 变更）跑过。
2. **存量不在本单清理**：170 个历史文件（多为归档文档与测试 fixture 路径）脱敏是独立工程，本单只恢复口径 + 让放大可见 + 挂账。

## 允许文件集（越界 = judge Critical）

1. `.github/workflows/ci.yml`（仅 repo-hygiene job 一处）
2. `.github/workflows/nightly.yml`
3. `.github/workflows/cli-package.yml`
4. `scripts/check-repo-hygiene.mjs`（仅 fallback warning 一处）
5. `docs/status/tech-debt-ledger.md`（新增 P2-24 一条）

禁区：其它一切文件。**本单全部文件属 metaRatchetPaths 或文档，双 judge 车道已由编排者安排，用户拍板（Windows-only + 推送）已记录。**

## 改动一：ci.yml repo-hygiene job checkout 加深

- 该 job 的 `actions/checkout@v4` 步骤加 `with: fetch-depth: 2`，并加注释说明原因（浅克隆使 HEAD^1 不存在触发全仓 fallback 扫描，run 33982832690 实证）。
- ci.yml 其它任何内容零改动。

## 改动二：check-repo-hygiene.mjs fallback fail-loud

- `contentScanFiles` 走到全仓 `trackedFiles` fallback 分支时，先打印一行显眼 warning（English-only，例如 `WARNING: full-repo scan fallback active — HEAD^1 unavailable or no local changes; scan scope is ALL tracked files`），再照常扫描。
- 不改任何判定逻辑、skip 规则、正则。

## 改动三：nightly.yml / cli-package.yml 收窄

- 两文件的打包矩阵仅保留 Windows leg（windows-latest 及对应 target），删除 ubuntu-latest / ubuntu-24.04-arm / macos-15 / macos-15-intel leg。
- 前置/汇总 job（如 `runs-on: ubuntu-latest` 的非矩阵 job）：**若含 cargo/Rust 构建或测试步骤 → 改 windows-latest；若纯编排/通知/调度性质 → 保留 ubuntu 不动**，report 中逐一说明每个保留 ubuntu 的 job 性质与理由。
- 删除 leg 后检查残留引用（matrix.platform.name/target 的 include 结构、缓存 key、上传条件），确保 YAML 合法且剩余 leg 自洽。

## 改动四：ledger 挂账 P2-24

- 新增条目（就近模仿既有格式）：症状 = hygiene 全仓 fallback 口径下约 170 个历史文件（归档文档/测试 fixture）含本地绝对路径，口径恢复后不再触发；状态 = deferred；处置方向 = 存量脱敏或规则豁免，待拍板；关联 = W17-2 口径修复。

## 验证（输出原文进 report）

1. **fallback warning 实证**（关键）：`git clone --depth 1` 本仓到临时目录 → 在浅克隆里跑 `node scripts/check-repo-hygiene.mjs` → 必须出现全仓 fallback warning 且扫描执行（可因存量路径失败退出——这正是设计行为，report 说明）。浅克隆用完删除。
2. 本仓正常跑 `node scripts/check-repo-hygiene.mjs` 绿（口径不变，无 fallback warning）。
3. `node scripts/verify-rot-budget.mjs` 绿。
4. 三个 workflow 文件 YAML 结构合法（用本仓已有手段或逐行自审；不得引入新依赖）。

## 提交

单 commit：`ci: hygiene fetch-depth fix + fail-loud fallback + packaging workflows windows-only (W17-2)`，逐文件点名 add。

## 报告

`.superpowers/sdd/reports/w17-2-report.md`（不入 commit）：四改动逐条 file:line / 浅克隆实证输出原文 / 保留 ubuntu job 的逐项理由 / 结尾状态词。

## Global Constraints

1. 零新依赖；输出 English-only。2. 验证输出原文进 report。3. ci.yml 除改动一外零触碰。4. 禁 `git add -A`。5. 结尾状态词合规。
