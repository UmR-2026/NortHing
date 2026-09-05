# Handoff — 2026-09-06 W16 可信核落地 + W17-1 CI 收窄，待推送

>  freshest session state。旧篇：`2026-09-05-six-tasks-green-rot-audit.md`（同目录）；再早见 `docs/archive/handoffs/`。

## 0. 一句话状态

Phase -1（最小可信核）已完整落地并全部过审；CI 已按用户拍板收窄 Windows-only；**本地 10 个 commit 未推送（origin/main = `19349cd`），推送待用户授权**。

## 1. 需求基线

- 总纲：`E:\agent-project\.opencode\external-review\2026-09-05\D-synthesis-plan-2026-09-05.md`（§3 路线图 + §9 用户四项拍板，全部有效）。
- 现行规范：`AGENTS.md` 家规 8（commit-bound workflow gate 五子点，双语已同步）。
- 策略单源：`scripts/workflow-policy.json`（reviewVerdicts 词表 / CANNOT_VERIFY 分级 / metaRatchetPaths 8 项）。
- 用户最新拍板（2026-09-05 深夜）：**Windows 限定，不考虑其他平台**。

## 2. 已完成（commit 表）

NortHing main（本地领先 origin 10 commits）：

| commit | 内容 | 审查 |
|---|---|---|
| `deae1b7` | W16-1：`verify-task-gate.mjs` + `workflow-policy.json` + scripts 额度 42→48（现 44/48，到期 2026-10-15） | AWC 0C/0I/3M |
| `e9833a6` | W16-3：家规 8 双语 | PASS |
| `cedc231` | W16-4：theme.rs unsafe O_NONBLOCK 修复 + 死代码收割，989→**979 净减**（rot 双降 104/370） | PASS 0C/0I/2M |
| `77b69df`+`4bc3fb1` | W17-1：ci.yml 矩阵收窄 windows-only + P2-23 挂账 + cli **0 warning** | 双 judge APPROVE |
| `92712b6` | W16-5：词表统一 + metaRatchetPaths 增补 4 看守者 | APPROVE |
| `559cd6f`/`353a20f`/`1c9ac2f`/`a29b39c` | 各单 docs 收口（brief/report/review/台账） | — |

agent-project 仓（已推与否未核）：`c827e02`+`63a9289` — W16-2 组装脚本清单化 + manifest + 先校验后写。

波级终审：`packages/w16-final-review.md`（0C/3I/3M；I-1/I-2 已由 W16-5 闭环，I-3 见下）。

## 3. 待决与卡点

1. **push 授权**（用户）：10 commits 待推。推送后 CI 首跑验证 Windows 腿转绿 = 终审 I-3 闭环。
2. **W17-2（push 验证后做）**：`nightly.yml`（工作日 cron）与 `cli-package.yml` 仍含非 Windows leg，触发即红——按同一拍板收窄（meta-ratchet 车道：双 judge）。
3. **P2-23**：terminal-core E0624 非 Windows 编译失败，deferred（用户拍板挂起；恢复跨平台时先修它）。

## 4. 队列（下一波 = Phase 0 checker 加固，D 报告 §3）

按序（每单仍需 brief → **brief review（reviewer-53）** → 派发 → judge（minimax-m3））：

1. `validateManifest()` fail-closed（含 entry.dir 路径 confinement）+ **D-3 红队 probe 先行**（5 反例 fixture 挂 CI/自测，成本极低）
2. only-down 机械化 + **-1.5 headroom floor**（拧 ceiling ≥ current+max(5行,5%)，顺延项在此承接）
3. >1000 硬边界走 exception lease（**禁止复活 allow-god-file 注释**——O-2 errata，PHASE-0 历史裁定）
4. config 字段实现或删承诺 + EXEMPT_FILE_PATHS 移入 manifest
5. 扫描范围 attestation（覆盖 installer + scripts 自身；checker 纳入自身监控）+ **-1.7 退役机械化**（顺延项）
6. verdict rubric 互斥化进 SSOT + dead registration warning 限期升级 violation
7. mutation 集**外部预注册锁定**（D-1 历史事故 replay 并入）

并行性：以上均触碰 `verify-rot-budget.mjs`/manifest，**互斥需串行**；W17-2（workflows）与其中任何单文件集不相交，可插空并行。

## 5. 子代理运维（本会话实证，已入 memory commit `8b8f9bf`）

- **coder 主力 = `gemini-38-flash-agy`**：W16+W17 共 6 单全交付（含 1 次合规 NEEDS_CONTEXT 上交）。`coder-qwf` 09-05 派发即取消一次，待观察。
- **brief review 环节已生效**（09-05 用户拍板）：每单派发前先派 `reviewer-53` 审 brief（五判据：可证伪/预设豁免/allowlist 完整/无预判结论/判断点已授权）。首跑抓 1C/1I，已证值得。
- judge = `minimax-m3`；波级终审 + brief review = `reviewer-53`；meta-ratchet 车道（碰 policy/闸/workflows/package.json）= 双 judge + 用户拍板。
- 新工具纪律：judge 结论词用 policy `reviewVerdicts` 词表（APPROVE / APPROVE_WITH_CONCERNS / CANNOT_VERIFY / BLOCKED / FAIL）。

## 6. 环境坑（本会话实测）

- shell PATH 是 session 快照：`rustup`/`gh` 用全路径（`<USERPROFILE>\.cargo\bin\rustup.exe`、`C:\Program Files\GitHub CLI\gh.exe`）；`ci_logs` 工具当次报证书错误，gh CLI 直连可用。
- hygiene 扫「盘符+Users 目录」形态的本地绝对路径，进仓即红：brief/report/review 里用 `<USERPROFILE>` 占位（`E:\agent-project\...` 形式不触发）。子代理产物收口前必跑 `node scripts/check-repo-hygiene.mjs`。
- `progress.md` 是 GBK 混编：追加只用 `ledger_append` 工具；先 append 再 git add（顺序反了会漏）。
- 家规 5 宵禁 03:00。

## 7. Suggested skills

- 派发 Phase 0 单前：`subagent-driven-development` + `writing-plans`（若起新计划文档）
- checker/闸改动：`anti-rot-system`（前置阅读写进 brief）
- Rust 修复单：`unsafe-checker` / `m15-anti-pattern` / `m06-error-handling` 按内容命中
- cargo 长命令：`long-running-shell`（PTY/重定向纪律）
- 本 handoff 系列惯例：`handoff` skill（下次收口同样走它）
