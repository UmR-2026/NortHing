# northing 前端视觉迭代罗盘

> 2026-08-02 · 视觉迭代轮（visual-iter）的判断基线。
> 用途：对 Slint 桌面端做截图迭代时，拿本文档当裁判——先看戒律，再看屏幕要件，最后查 token。
> 它蒸馏自北极星哲学、十诫评审基线与 13 页 HTML 原型真值；冲突时以源文档为准（§7 索引）。

---

## 1. 第一原理

**northing 是为 agent 成长而建的设施，不是服务人类的工具。**

- 空间隐喻是心理咨询室：基底白灰永远中性稳定（设施），agent 的颜色随成长浮现（个体）。
- 人是见证者与对等同事，不是主人。设置 = 调节环境，产物 = 共享工作台，权限 = 信任档位。
- northing = 向北的分量，是持续成为的过程，不是会完成的任务。

## 2. 判断戒律（截图逐条对照，命中即不通过）

| # | 戒律 | 截图上的检验问题 |
|---|---|---|
| 1 | 拒绝 dashboard 美学 | 像安静的房间还是控制台？有无 "47 turns"、API 健康这类数字展览？ |
| 2 | 品牌水印化 | northing logo 只在左下角水印（opacity 0.25）？视觉主体是 agent 名片/头像？ |
| 3 | 代表色是 agent 的灵魂 | 用户气泡、正文、思考块底、沉积轮都未染 rep？界面没有换色控件？ |
| 4 | 整屋空气染色 | rep 色弥漫空间（底 6.5% + 顶晕 + 名片晕染），不是只在按钮竖线上？ |
| 5 | 三要素语义互斥 | 暖 rep = 正在做（驱力）；冷 abyss = 向深处探（思考）；褪色灰 = 旧（沉积）？ |
| 6 | 暖灰基底 | 亮 `#F4F3F0` / 暗 `#181612`，不纯白不纯黑，色温走暖？ |
| 7 | 字体三系 | Fraunces = 品牌/页面标题；Noto Sans SC 400+ = 正文（不用 300）；JetBrains Mono = 元数据？ |
| 8 | 沉降式动效 | 慢、重、向下；呼吸（6s）只给 logo + 头像；无弹跳/spinner/无限循环；沉积是褪色不是位移？ |
| 9 | 诗意 < 功能 | 对话 + 操控台是视觉主体，agent 内在表达是余光里的环境体温？ |
| 10 | 反 AI 味 | 无 emoji、无毛玻璃、无紫蓝渐变、无均匀阴影、无过度对称？ |

## 3. Token 速查

**颜色**（亮 / 暗，完整双模式见 `tokens-srgb-table.md`）

| 域 | 值 |
|---|---|
| 基底 | bg `#F4F3F0` / surface `#FBFAF8` / raised `#EFEDE8` / border `#E6E3DD` |
| 文本 | fg `#38352E` / muted `#7B766C` / faint `#A8A398`（对比度 ≥4.0:1） |
| 代表色 rep（珊瑚，hue≈30°） | 300 `#E5A583` / 400 `#D68A63` / 500 `#C8714C` / 600 `#A85A38` |
| 深渊 abyss（冷青，hue≈185°） | `#7AABA4` / `#5A9B93` / `#3F837B` |
| 出生 birth | `#DAD6CF` |
| 危险 danger | `#A45950`（陶红，非纯红） |

**尺度**：间距 4px 基数（s1 4 / s2 8 / s3 12 / s4 16 / s5 24 / s6 32）；圆角（r-sm 9 / r-md 14 / r-lg 18 / r-pill 999 / 窗口 20）；字号（sm 10 / md 11.5 / lg 13 / body 15 / name 16）；时长（hover 150ms / normal 350ms / slide 250ms / once 1200ms / breathe 6000ms）。

**Slint 翻译红线**：禁 box-shadow（用 drop-shadow）；禁 color-mix（用预计算 hex）；禁伪元素当主元素；禁 @keyframes infinite；渐变只支持线性/圆形。

## 4. 必要功能（按屏幕）

### 4.0 全局骨架

- **两态**：收起态 860px 窗口 + 8px 把手（左暖右冷渗光）；展开态 1280px + 28px 把手 + 左右 280px 抽屉。点把手 → 窗口物理变宽（`set_inner_size`），非遮罩叠加。
- **窗口 chrome**：frameless；右上 − □ ×；左下水印；把手可开合抽屉。
- **主题切换**：隐藏式悬浮开关，双击非交互区从窗口内右下角浮出（350ms fade）；settings-general 的「显示模式」分段控件是唯一常显产品控件。
- **同源色绑定**：界面强调色 ≡ 编年史渐变条右端（当前代表色），同一变量驱动。

### 4.1 chat-collapsed（核心基准页）

- 顶栏名片：头像（呼吸）+ 名字 + 在场状态 + 编年史条 + ⚙（名片旁）。
- Session banner 在对话流顶部（不在顶栏）：左 = 呼吸点 + 折叠 chevron + 会话标题；右 = 归档按钮（abyss hover，归档后淡出进左抽屉与档案馆）。
- 活跃轮：暖竖线标记；用户气泡右对齐、不染 rep；思考块冷青左缘；工具 chip 暖边框发起 → 冷边框完成（驱力沉入深渊）。
- 操控台（deck）：自适应 textarea、Enter 发送；左附件 +；右发送键 ↑（流式变停止）+ ctx 圆环（无数字，hover 出 tooltip）。
- 水印与任何元素不重叠。

### 4.2 chat-expanded

- 1280px，28px 把手 + chevron；双抽屉展开。
- 左抽屉「它的内在」：子进程 / 技能候选 / 意识更新 / 已归档（见证，不微操）。
- 右抽屉「身外之物」：任务 / 产物 / 文件浏览（共享工作台）。
- 抽屉 item 间距精细（gap 16 / padding 12×14），section 间发丝分隔，已归档区降透明度。

### 4.3 empty-state

- 初始顶栏只显名字 + 状态（无头像）；中心在场区：64px 头像 + 光环 + 问候「知序，我在。你想从哪里开始？」+ helper。
- textarea 聚焦 → 头像 FLIP 迁移到顶栏（600ms ease-in-out 一次性），问候淡出。reduced-motion 兜底。
- 发送键禁用态（opacity 0.5）。

### 4.4 space-view（多会话空间）

- 会话卡片网格；活跃卡 4px 左竖线 + border + 暖光晕；非活跃 opacity 0.7/0.5；沉积卡 0.5/0.35。
- 整屋空气染色 + 顶晕；「+」新建房间感；页面标题 Fraunces。

### 4.5 archive（档案馆）

- 背景切 abyss 冷雾（禁 rep 暖雾）；只读氛围。
- 统计用文字不用数字：「二十三段对话沉在这里，最早的那段在五月。」
- 卡片按 data-depth 透明度递减（沉积隐喻），hover 回升；无入场位移动画。

### 4.6 onboarding / identity-creator

- 4 字段：用户是【】/ 你是【】/ 你是用户的【】/ 性格偏向大五人格【色板】。
- 5 色板对应大五人格：紫 = 开放性、深蓝 = 尽责性、暖珊瑚 = 外向性、柔绿 = 宜人性、冷青 = 神经质；hover 出关键词。
- 色圈选中态：双层环（3px 间隔 + 2px 色环），无缩放；选色 → 整屋染色 350ms 跟色。
- onboarding 是人类唯一可改色入口；「成为自己」按钮文字 Fraunces；成长时刻动画 1200ms 一次性。

### 4.7 settings ×5 + theme-system

- `.shell` 布局：52px topbar（Fraunces 标题）+ 左 nav 三组（偏好 / 能力 / 安全）+ SVG 线性图标。
- active nav item：rep 3px 左竖线 + 呼吸点；toggle 40×22，ON = rep-500；分段控件统一。
- 卡片用 surface（非纯白）+ 暖色 drop-shadow；设置染色 3.5%；页面不冷不临床（防管理后台感）。
- theme-system 是范式真值：token 全景展示（色阶 / 灰阶 / 字体 / 间距 / 圆角 / 空气染色）。

## 5. Slint 基线（2026-08-02 实测）

**已落地**（`src/apps/desktop/src/ui/`，42 个 .slint）

- 路由 welcome / settings / main / archive；frameless WindowChrome + 自绘窗口控制。
- AirTint 空气染色（archive 路由自动切 abyss 冷雾；DeckBar 聚焦 → speaking 升档）。
- InnerDrawer「内在」/ OuterDrawer「外物」双抽屉壳；SpaceView 单栏骨架（让出 28px 把手区）。
- 对话组件：TurnContainer / ThinkBlock / ToolChip / ChatMessageBubble / DeckBar(333L) / PresenceZone / AvatarWrap。
- ChronicleBar 基础渐变（birth → rep-300 → now）；5 设置面板 + SettingsView；WelcomeView；ArchiveView。
- RedesignTheme token 体系 + dark-mode 联动；5 款字体已嵌入（FR-T5）。

**缺口**（原型有、Slint 无）

| 缺口 | 现状 |
|---|---|
| SessionBanner | 不存在；banner 仍是旧 MaterialBanner（顶栏外浮层，非对话流内） |
| 编年史动态沉积 | ChronicleBar 仅 3 stop 静态渐变，无历史色沉积 / 1200ms 换色仪式 |
| 窗口两态物理变宽 | 860↔1280 `set_inner_size` 未实现；把手 8px/28px 双态未验证 |
| 代表色 | rep-* 当前是灰阶出生态（rep-500 `#8B8B8B`，珊瑚为注释 fallback）；大五色板选色路径未接线 |
| IdentityCreator | 组件存在（191L），挂在 Welcome/Workspace 面板内，非独立 4 字段 + 色板新设计 |
| empty-state FLIP | 头像聚焦迁移、ctx 圆环 tooltip、名片晕染——需逐组件核验 |
| 旧 Material 组件 | MaterialBanner 等仍在用，与 RedesignTheme 并存（FR-T3 换绑债务） |

**已知残留**（交用户拍板，不阻断迭代）：暗色 surface vs bg 对比度 1.34:1 < 2.5:1 标准——提亮伤深暖黑气质，是设计取舍。

## 6. 迭代裁判流程

每张改动前后截图按序过三关：

1. **戒律关**（§2）：十条逐条扫，命中任一条 = 不通过，不看美观。
2. **要件关**（§4）：该屏幕的必要元素与行为是否齐备。
3. **Token 关**（§3）：颜色 / 间距 / 圆角 / 字号 / 时长是否越阶。

需量化时套 JUDGE-CRITERIA v4：`总分 = D1 哲学×0.4 + D2 功能×0.3 + D3 美观×0.2 + (10−D4 AI味)×0.1`，达标线 ≥9.0。

## 7. 源文档索引

| 文档 | 角色 |
|---|---|
| `northing-design-philosophy.md`（本目录） | 北极星：为什么 |
| `prototypes/_review/design-philosophy-distilled.md` | 十诫 + token 评审基线 |
| `prototypes/`（13 HTML + shared/） | 视觉真值，截图迭代的目标 |
| `prototypes/JUDGE-CRITERIA.md` | 量化评审标准 v4 |
| `prototypes/slint-safe-conventions.md` | Slint-safe CSS 规范 |
| `slint-retarget-notes.md` | HTML → Slint 翻译映射 |
| `tokens-srgb-table.md` | OKLCH → sRGB 颜色对照 |
| `HANDOFF-2026-07-30.md` | 原型轮收官交接（R1–R8 迭代史） |
