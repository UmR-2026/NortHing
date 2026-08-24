# E4 onboarding 终审 brief

只读。不改代码、不 commit。

工作地点（所有相对路径根、命令 workdir）：`E:\agent-project\northing\.worktrees\consult-room-build`

需求：`task-ef-e4-onboarding-brief.md`（含 §0 五处真值裁决）+ 总 brief `task-ef-pages-master-brief.md`
真值：`docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-onboarding-v2.html`（602 行）
报告：`task-ef-e4-onboarding-report.md`
（均在 `.superpowers/sdd/consult-room/` 下）

## 必读

- diff（相对 HEAD `d52a619`，tracked 部分）：`task-ef-e4-review.diff`（289 行：app.rs +22 / i18n.rs +35 / mod.rs +2 / registry.rs +33 / 三份 ftl）
- 新文件（未跟踪，不在 diff 里，直接 Read 全文）：
  - `src/apps/desktop/src/ui_dioxus/pages_onboarding.rs`（约 700 行）
  - `src/apps/desktop/src/ui_dioxus/pages_onboarding_css.rs`（约 209 行）
- 截图 Read 打开：
  - `C:\WINDOWS\TEMP\opencode\t7-shots\e4-onboarding-dark.png`
  - `e4-onboarding-light.png`
  - `e4-onboarding-selected.png`
- `src/apps/desktop/src/flags.rs` 必须 `DIOXUS_SHELL=false`
- TRUTH_CSS 字节未改：`git diff HEAD --stat -- src/apps/desktop/src/ui_dioxus/css.rs src/apps/desktop/src/ui_dioxus/windows.rs` 必须为空

## 验收

1. 独立 OS 窗 `id="onboarding"`，`DockSide::Fullscreen`（新变体）spawn 覆盖 room 当前几何，room geom 无效回落 1280×860；单例（重复 mark_opening 拒绝）；单测 `test_onboarding_registration_and_lifecycle` 在位
2. 真值转写保真：五色板（驱力 #C8714C / 深渊 #3F837B / 跃迁 #8B5FBF / 凝视 #D99B48 / 镇静 #4B8F6B + 关键词）逐字；中枢初始「?」+「未命名诊室」+「沉寂态 · 等待人类决定第一个色彩」；deck「房间诞生完毕后，印记将融入基质」+「☩ 唤醒诊室 · 开启印记」；右抽屉诞生存根含仪式关卡/印记预览/term-well；三章 ritual-divider 文案
3. 交互状态机：选色即时 `--mind-base` + `data-inhabited="true"` + 虚线转实 + 右抽屉 step1 已凝结 + 左抽屉行2/seg 联动；测试连接 mock ok 态；完成钮未选色内联提示（无 alert）、选色后完成 mock 态、不自动关窗
4. 接线：app.rs Fullscreen 几何臂 + room 状态行 `#nav-onboarding`（复用 `.status-nav-link`，stop_propagation 在位）；registry 注册；mod.rs 声明
5. CSS：`pages_onboarding_css.rs` 自包含转写真值 `<style>`（color-mix 逐字、8s 呼吸、双光学、prefers-reduced-motion）；不注入 truth_css()/OVERLAY_CSS；css.rs / windows.rs / TRUTH_CSS / consult-room-main.css 零触碰
6. 行数 <800：两新文件 + app.rs(652) / registry.rs(502) / i18n.rs(368)
7. i18n 三语对称（zh-CN/zh-TW/en-US 各 33 键）；`pnpm run i18n:audit` exit 0
8. E1 archive / E2 space / E3 settings 未被破坏（pages_* 三文件在、registry 六插件齐、既有 7 测仍过）
9. flags=false 实证；无 commit（HEAD 仍 d52a619）
10. 报告门禁证据齐全（check exit 0 / ui_dioxus 8 passed / flags 3 passed / audit 1 grandfathered）

输出：`task-ef-e4-onboarding-review.md`
总判决 PASS/FAIL；Spec/Quality 双判决；C/I/M/F 分级；CAN MERGE。
