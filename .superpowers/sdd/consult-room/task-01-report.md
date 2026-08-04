# Task 1 Report

## 状态
DONE

## 三缺陷处置

1. **窗控字形损坏**：在 WindowChrome.slint 中使用 Slint 几何绘制 (\Path\ 和 \Rectangle\) 重新实现了 ☀、─、□、✕ 四键，避免了因 Noto Sans SC/JetBrains Mono 字体缺失导致的问题。
2. **窗控簇左侧异常图标**：经过排查，在重置上一个 implementer 留下的未提交脏改动并修复 WindowChrome 的 \HorizontalLayout\ 容器宽度后（110px -> 120px，避免控件挤压溢出到左侧），异常图标已从主视图移除。
3. **亮色态头像光环过硬**：在 PresenceZone.slint 和 AirTint.slint 中将光环边缘 \	ransparent\ 改为同色的 \.with-alpha(0)\ 以消除 banding；并在 PresenceZone.slint 和 AirTint.slint 中将亮色模式的不透明度（\opacity\）乘数分别收敛到 0.4 和 0.2（或按真值隐藏），使得亮色模式的呼吸光晕更加克制，符合“亮色无菌室”的真值设计。

## 定稿系统 API 清单
- **WindowChrome**：新增 callback: \	oggle-theme()\（其余 5 callback 名不变）；新增 in property: \signal: bool\、\inner-drawer-open: bool\、\outer-drawer-open: bool\。
- **AvatarWrap**：默认 \initial\ 从 "知" 改为 ""（无回归，两处调用方都传入覆盖）；\reathing\ 默认 \	rue\；签名不变。
- **ChronicleBar**：新签名暴露 \irth: color\（默认 RedesignTheme.t.birth）+ \
ow: color\（默认 RedesignTheme.t.rep-500），高度 4px、opacity 0.7。
- **AirTint**：\speaking\/\cold\ 入参不变；顶晕新增 \opacity: dark ? amp-aura : 0\（亮色隐藏）。
- **呼吸常量**：新增 \src/apps/desktop/src/ui/system_constants.slint\，导出 \reathe-phase\（cos）、\reathe-progress\、三档 \mp-avatar\/\mp-membrane\/\mp-aura\。

## 验证截图
- 暗色：E:\agent-project\northing\.worktrees\consult-room-build\docs\design\2026-07-22-frontend-redesign\consult-room\build-shots\t1-main-dark.png
- 亮色：E:\agent-project\northing\.worktrees\consult-room-build\docs\design\2026-07-22-frontend-redesign\consult-room\build-shots\t1-main-light.png

## API 变动
详见“定稿系统 API 清单”节。

## 遗留/风险
- 侧栏 toggle-left/right 在 WindowChrome 失去把手后暂无点击触发区域（留给门铃宝石任务处理）。
- drag 接线未实现；当前 WindowChrome 状态行 Rectangle 无 TouchArea/pointer-event 转发，Rust 侧也无 -webkit-app-region 等效接线（slint winit 后端不支持 CSS app-region，需 WindowProperties 自定义）。留待 FR-T3 框架化处理；本任务按 brief §3.1 接受现状。
- SpaceView 原有的 \x: 28px\ 左移让位已随废除左把手而移除。如果 SpaceView 内部仍有偏移导致 room 框位置怪异，留待 T2 处理完整布局重构。
