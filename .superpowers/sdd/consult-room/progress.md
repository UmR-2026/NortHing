# Consult-Room Slint 建构期 Progress Ledger

计划：`.superpowers/sdd/plan.md`
分支：`feat/consult-room-slint`（worktree `E:\agent-project\northing\.worktrees\consult-room-build`）
基线：8e43dc4 (main, 2026-08-04)；终裁：五页套全部按现状通过（FINAL-RULING-20260804.md）

| Task | 状态 | Commits | 备注 |
|---|---|---|---|
| T0 setup: complete (orchestrator) | 基线 cargo check -p northhing pass（7m53s）；spike 资产移植（palette 25 mind token + oklch-to-srgb.py + tokens-srgb-table + feasibility doc；探针未移植）；i18n 已生成 | e487cd8 + 5ea3f6a + c1e107c + 748031c（被 amend 收拢，见 FYI）|
| T1 chrome 与系统层: complete (双判决 PASS + 4 Important 修复完 + 复审 PASS) | gemini-31-pro (Implementer) + gemini-31-pro (fixer round 1+2)；judge-m3 双判决 PASS（spec+quality 0 Critical / 4 Important / 7 Minor + 1 FYI）；fixer round 2 处置：API 清单补全 / drag 备注 / BOM 去 + LF 统一 / SpaceView 28px 让位移除；重截暗亮；commit amend 至 748031c | 748031c（FYI：commit 含 SDD 文档+截图，留终审清理）|
| T2 主诊室页: complete, review 待派 | 主路由 SpaceView 移除 PresenceZone（让位给 ChatPaneView 内 RoomHead）；新增 RoomHead / DoorbellGem / MindMod / WorkMod 4 组件；ChronicleBar 双击换色绑定；DeckBar 合一按钮；theme 3 档（缝线 16% mind / speaking 整屋升档 / agent 代词着色）；mock 会话流（agent / tool / chip / witness / approval）| ac86998 |
| T3 onboarding v2 | 待派 |  |  |
| T4 settings v2 | 待派 |  |  |
| T5 archive v2 | 待派 |  |  |
| T6 space v2 | 待派 |  |  |
| T7 终审 | 待派 |  |  |

## T2 备注
- 主路由 avatar 重复问题已由编排者修（SpaceView 移除 PresenceZone）。
- theme 切换 light 截图未立即捕获（点击位置 / 时序）— FYI 留 T7 终审再补。
- 状态行/room-head drag 接线仍未做（沿用 T1 FYI，留 FR-T3）。
- 抽屉内容 mock 4 项 / 3 项；T4 settings 抽屉可一起细化。

## FYI 终审清理清单
- e311aeb / 748031c commit 误捎 SDD 文档（plan.md / progress.md / task-01-brief.md / task-01-report.md / task-01-review-brief.md / task-01-review.md）+ 截图（build-shots/*.png）；合并前 `git rm --cached` + 单独 "chore(consult-room): 剥离 T1 误捎" commit。
- I-4 部分（SpaceView 内部 layout 仍可能微调）留 T2 处理完整 room 居中/双抽屉布局重构。
- Minor m-1（亮色 line token 缺失）可由 T2 追加 palette token 顺手收尾。
- Minor m-2（WindowChrome.signal in property 残留）可在 T2 顺手清理。