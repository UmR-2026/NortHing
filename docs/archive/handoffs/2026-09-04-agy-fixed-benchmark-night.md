# Handoff — 2026-09-04（凌晨终收）：agy 渠道修复+基准夜

> 上一份：`2026-09-04-ci-greens-and-startup-hang-rootcause.md`（同日前半夜）。本份覆盖：agy 修复全链、Antigravity 2.12.0 跟进、两轮模型基准 + 外部对照、reviewer-53 定岗、工具链四优化。

## 0. 一句话状态

NortHing main HEAD `4d6bba2`（已推）。agy 双渠道（3.7/3.8-flash）**修复完成且重启后实证可用**。队列顶部：**W15-1h**（MemoryDb 迁移竞态修复，qwf 的 P3 方案可直接当 brief 底子）、**桌面挂死修复单**（方案在 startup-hang-trace-report.md）。

## 1. agy 渠道全案（已闭环）

- 根因四连（交叉审查：minimax-m3 + explore）：A 内容请求发持久化指纹 2.0.6-darwin（目录层 UA 过门、请求层违规）→ 429；B "resource exhausted" 误分类 → 重试放大 16-20x；C 主 fetch 无超时；D tier 白名单缺 3.7。
- 修复：fork `bbbc28f`（四连修+回归测试），judge minimax-m3 APPROVE 0C/0I/5M，编排者亲跑双模型真 API 冒烟全过。
- Antigravity 升级 2.12.0 跟进：fork `6ee7e7c` 抬版本底线；**指纹自愈逻辑实战首验**（自动滚 2.12.0 win32）。
- **以后上游再升级：只需抬 constants.ts 的 ANTIGRAVITY_VERSION_FALLBACK + npm run build**。
- fork 在 `C:\Users\UmR\Desktop\opencode-antigravity-auth-fork`（dist gitignored，本地重建生效）。

## 2. 模型基准夜（选派表已更新，权威在 memory/BOOTSTRAP.md）

- R1（platform.rs 检索/流程/推理/代码）：38-agy / 37-agy / qwf 三家全对，风格差 = qwf 证据链最好。
- R2（MemoryDb 真题狩猎+修复设计）+ 外部对照 6 份盲评：**顶档 = qwf / minimax-m3 / dsv4f / glm 级外部**；中 = qw3.8max（幻觉版本号）；底 = step-explore（Q2 稻草人论证）。
- 结论：judge=m3 验证合格；**qwf flash 反超自家 max**；step-explore 退出审查线；**波级终审 = reviewer-53（glm-5.3，已冒烟合格，强模型无需强约束 brief——给职责+需求即可，用户拍板）**，回落 38-flash。
- 题包 `C:\WINDOWS\TEMP\opencode\bench\external-question.md` + 评分锚点 = git-replay 题库第一题（TEMP 目录，重启会丢，如需保留请移入仓库）。
- 工作流确认：并行 coder 上限 agy2+vertex2+qwen 够用，不加通道；flash 模型派发需强约束 brief（用户在并行 session 改了部分守则，注意 sync 冲突）。

## 3. 工具链四优化（本 session 落地）

- `run_detached` 加 4s 自检（静默死直接报错 + 指 PTY 回退）
- 新增 `ci_logs` 插件工具（gh 带 auth 拉失败日志，路径回退）
- `shot-window.ps1` TMP/TEMP 修复烙进脚本（csc 临时目录根因）
- 编排者 AGENTS.md 新增：取消/失败卫生 SOP（派发前 git status 查残留）+ brief 预检铁律（验证命令 BASE 上先跑通、feature 集对齐 ci.yml）；judge 模板新增「条件早退测试」必查项
- **插件改动需重启 opencode 生效**

## 4. 队列（不变，置顶两件）

1. **W15-1h** MemoryDb 迁移竞态：busy_timeout + BEGIN IMMEDIATE 事务化 check-then-act（含 text_fts 回填块）；qwf 的 P3 答案（`C:\WINDOWS\TEMP\opencode\bench\result2-qwf.md`，TEMP 易失——也可让 coder 重推）是现成处方。注意 llm3 发现的次级缺陷：PRAGMA 读取的 `.ok()` 吞错链一并治。
2. **桌面挂死修复单**：core `read_optional` 超时降级（根治）+ 桌面 F1 挪 turn_runtime（止血）；修好后重拍 W15-1 §7#11 三张截图闭环 W15-1。
3. repo hygiene 治理（**等用户拍板**：checker 豁免 vs 工件脱敏）；ubuntu/macos 编译陈账。
4. follow-up：core 裸 feature 编译缺口、turn_persist.rs:546 泄漏候选、css_files.rs 孤儿、测试污染真实配置/数据目录（E 类升级：smoke-echo + 164 个 testslug 已实证）。

## 5. 本机环境状态

- `~\AppData\Roaming\northhing\`：app.json 已删 smoke-echo（备份 .bak-20260903）；episodes 164 个 testslug 隔离在 `episodes-quarantine-20260903\`（确认无用可删）
- gh CLI：`C:\Program Files\GitHub CLI\gh.exe` 已登录；编排者 PATH 快照旧，用全路径或 ci_logs 工具
- 截图工具：`C:\WINDOWS\TEMP\opencode\win-shot.ps1`（窗口级）+ fullscreen-shot.ps1（全屏）可用；TEMP 易失，重丢失就从 shot-window.ps1（已修）重新生成

## Suggested skills

- `subagent-driven-development`、`verification-before-completion`、`long-running-shell`、`handoff`
