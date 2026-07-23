# Slint 重定向说明（2026-07-23，编排者记，用户拍板）

> 背景：原重设计计划（`docs/plans/2026-07-22-frontend-redesign-plan.md`，用户侧文件）实施面为 desktop-tauri/ui（React）；2026-07-23 用户决策删除 desktop-tauri（Slint 唯一桌面壳），随后拍板：**Slint 不动，重设计做"翻译"而非放弃**。本文件记录翻译映射，不取代原计划（哲学/token/拍板表继续有效）。

## 什么活着、什么死了

| 原计划资产 | 处置 |
|---|---|
| 设计哲学 / 咨询室隐喻 / 代表色 HSL / 拍板表（§5）/ 异常态决议（§12） | **全活着**——壳无关，继续是北极星 |
| tokens-draft.css（双模式 OKLCH） | 数值照用，**格式翻译**：OKLCH→sRGB hex、[data-theme]→Slint 全局调色板翻转 |
| 字体（Fraunces 可变轴/JetBrains Mono/Noto SC 子集，staged 于 `fonts/`） | 翻译：Slint 嵌入格式实测（woff2 直用 or fonttools 实例化 ttf） |
| HTML mockup（视觉基准） | 继续当走查基准（人对照，不要求跑起来） |
| React 组件规范 / CSS 动效 / hooks 适配 | **死了**，改写为 .slint 组件 + property animation（动效人格"慢重向下一次性"恰好是 slint 保守动效的主场） |
| B9/B10（facade archive/跨 workspace 枚举） | **已落地**（34a2397 卷入，HEAD 复测绿）——壳无关，档案馆后端照用 |
| B11-B14 | B11/B12/B13 壳无关照用；**B14（desktop-tauri commands 透出）作废**——Slint 侧走既有 app_state 回调通道，不需要 tauri commands |

## 现状锚点（侦察 2026-07-23）

- Slint **1.17.1**，material style，`build.rs` 编译 `src/ui/main.slint`（334 行）；views 无 god-file（最大 SidebarView 470 行）。
- 现有主题机制：`src/ui/theme.slint` `MaterialTheme` global（dark-mode bool + current-* getter）——原语可用但价值体系是 Material 暗色，**不改动它**，重设计调色板新建独立文件，T3 起组件逐步换绑。
- 字体现状：仅 2 处 `font-family: "Consolas, monospace"`（CodeBlock/ToolCallCard），系统字体，无自托管。

## 翻译票据（取代原 P1.x 编号）

| 单 | 内容 | 依赖 |
|---|---|---|
| FR-T1 | OKLCH→sRGB 生成器（可重跑，P1.1 定稿时重跑）+ `redesign_palette.slint`（struct 双色 token 集 + dark 翻转 + 动效时长常量） | 无 |
| FR-T2 | Slint 1.17 字体机制实测 → woff2 直用或实例化 ttf → `src/ui/fonts/` + build.rs + FONTS.md | 无（与 T1 并行） |
| FR-T3 | 组件骨架换绑：topbar 名片 / 对话流（活跃轮竖线、思考块深渊青左缘、工具 chip 暖→冷 350ms、turn-meta mono）/ 操控台（发送键变形）——对照 mockup 走查 | T1（+T2 字体就位） |
| FR-T4 | 空态出生态 + 懒建流转；sess-tag 菜单（重命名/封存） | T3 |
| FR-T5 | 设置·通用页：临时代号、显示模式（跟随系统/亮/暗，接 T1 翻转） | T1 |
| 档案馆 v1（原 P2.3） | Slint 设置页新 nav + B9/B10 后端（已就绪） | T5 后 |

验收纪律沿用：每单 cargo check -p northhing 绿；每 Phase 末用户视觉走查对 mockup。
