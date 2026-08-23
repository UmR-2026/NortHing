# W2.7 流体卡片 — 终审 brief（judge）

只读审查。不改代码、不 commit、不跑 GUI（截图已在盘）。

## 需求

`.superpowers/sdd/consult-room/task-w27-fluid-cards-brief.md`

## 实现报告（证据，勿当结论）

`.superpowers/sdd/consult-room/task-w27-fluid-cards-report.md`

## Diff 范围（相对 HEAD `2555119`）

工作树未提交：

- `src/apps/desktop/src/ui_dioxus/windows.rs`
- `src/apps/desktop/src/ui_dioxus/css.rs`
- `src/apps/desktop/src/ui_dioxus/app.rs`（左宝石只开 self 满高）
- `src/apps/desktop/src/ui_dioxus/registry.rs`（DockSide::LeftFull）
- `src/apps/desktop/src/ui_dioxus/i18n.rs`（dead_code allow，无新键）

`flags.rs` 必须仍是 `DIOXUS_SHELL = false`。

## 截图（必须 Read 打开）

`C:\WINDOWS\TEMP\opencode\t7-shots\`

- w27-left-dark.png / w27-left-light.png
- w27-work-dark.png / w27-work-light.png
- w27-left-folded-dark.png / w27-work-folded-dark.png

## 验收点

1. 每张内容卡可折到只剩标题（左右都要，终端除外）
2. 展开卡吃折叠让出的高度
3. 右列终端吃窗底剩余
4. 左列 skill|RUNTIME 之间有分组缝
5. 卡标题不贴左缘（约 18px）
6. 未回滚半高对切；未改 TRUTH_CSS；flags=false

## 输出

写 `.superpowers/sdd/consult-room/task-w27-fluid-cards-review.md`

格式：总判决 PASS/FAIL；Spec / Quality 分述；findings C/I/M/F；CAN MERGE 与否。每条 finding 给文件+行或截图文件名。
