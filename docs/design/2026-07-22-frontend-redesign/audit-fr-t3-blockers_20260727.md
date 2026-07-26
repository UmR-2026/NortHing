# FR-T3 组件换绑阻塞面分析报告

> **审计日期**：2026-07-27  
> **审计范围**：northing 仓库 Slint 组件层  
> **目标**：量化 FR-T3（MaterialTheme → RedesignTheme 换绑）的工作量与阻塞面  
> **基线**：`RedesignTheme` 已在 `main.slint` import（第 4 行），0 个组件实际使用

---

## 表格 1：旧 MaterialTheme token → RedesignTheme 对应 token 映射

### 1.1 颜色 token 映射

| MaterialTheme token | 类型 | RedesignTheme 对应 | 备注 |
|---|---|---|---|
| `primary` | color (暗) | `t.rep-500` | Material 紫色 → v2 代表色（灰阶出生态） |
| `primary-variant` | color | **缺失** | Material 深紫变体，v2 无对应概念，废弃 |
| `secondary` | color | `t.abyss-500` | Material 青绿 → v2 深渊青（哲学常量） |
| `background` | color (暗) | `t.bg` | 直接映射 |
| `surface` | color (暗) | `t.surface` | 直接映射 |
| `error` | color (暗) | `t.danger` | 重命名（error → danger），陶红低饱和 |
| `on-primary` | color (暗) | **需新增** | v2 rep 上的文字色——亮态 #FFF9F5 / 暗态需定义 |
| `on-secondary` | color (暗) | **需新增** | abyss 上的文字色——v2 范式中未显式定义 |
| `on-background` | color (暗) | `t.fg` | on-bg = 前景色，直接映射 |
| `on-surface` | color (暗) | `t.fg` | on-surface = 前景色，直接映射 |
| `on-error` | color (暗) | **需新增** | danger 上的文字色，v2 范式未显式定义 |
| `light-primary` | color (亮) | `t.rep-500` | 亮暗统一走 `t` 三元 |
| `light-primary-variant` | color (亮) | **缺失** | 同 primary-variant，废弃 |
| `light-secondary` | color (亮) | `t.abyss-500` | 统一走 `t` |
| `light-background` | color (亮) | `t.bg` | 统一走 `t` |
| `light-surface` | color (亮) | `t.surface` | 统一走 `t` |
| `light-error` | color (亮) | `t.danger` | 统一走 `t` |
| `light-on-primary` | color (亮) | **需新增** | 同 on-primary |
| `light-on-secondary` | color (亮) | **需新增** | 同 on-secondary |
| `light-on-background` | color (亮) | `t.fg` | 统一走 `t` |
| `light-on-surface` | color (亮) | `t.fg` | 统一走 `t` |
| `light-on-error` | color (亮) | **需新增** | 同 on-error |
| `dark-mode` | bool | `dark` | 重命名 |

### 1.2 Function getter 映射

| MaterialTheme getter | RedesignTheme 替代 |
|---|---|
| `current-primary()` | `t.rep-500` |
| `current-surface()` | `t.surface` |
| `current-background()` | `t.bg` |
| `current-on-surface()` | `t.fg` |
| `current-on-background()` | `t.fg` |

### 1.3 字号 token 映射

| MaterialTheme token | 值 | RedesignTheme 对应 | 备注 |
|---|---|---|---|
| `font-size-headline` | 24px | **缺失** | v2 无 headline 档；Fraunces 18px 承担名字，p-name 用 `fs-name`(16px) |
| `font-size-title` | 20px | `t.fs-lg` (13px) | **显著缩小**：v2 层级靠颜色+字体而非字号差 |
| `font-size-subtitle` | 16px | `t.fs-name` (16px) | 值相同，语义变化 |
| `font-size-body` | 14px | `t.fs-body` (15px) | v2 正文略大（15px） |
| `font-size-caption` | 12px | `t.fs-sm` (10px) + `t.fs-md` (11.5px) | v2 拆为两档，需逐处判断 |

### 1.4 间距 token 映射

| MaterialTheme token | 值 | RedesignTheme 对应 | 备注 |
|---|---|---|---|
| `spacing-xs` | 4px | `t.s1` | 直接映射 |
| `spacing-sm` | 8px | `t.s2` | 直接映射 |
| `spacing-md` | 16px | `t.s4` (16px) | 跳过 s3(12px)，值相同 |
| `spacing-lg` | 24px | `t.s5` | 直接映射 |
| `spacing-xl` | 32px | `t.s6` | 直接映射 |
| — | — | `t.s3` (12px) | v2 新增档位，MaterialTheme 无对应 |

### 1.5 Elevation token 映射

| MaterialTheme token | RedesignTheme 对应 | 备注 |
|---|---|---|
| `elevation-0/1/2/4/8` | **缺失** | v2 不使用 Material elevation 阶梯；阴影通过 `drop-shadow-*` 属性局部控制，无系统化 token |

### 1.6 RedesignTheme 新增 token（MaterialTheme 无对应）

| RedesignTheme token | 类型 | 用途 |
|---|---|---|
| `t.elevated` | color | 浮起卡片层（比 surface 更亮） |
| `t.raised` | color | 控件底色（按钮/输入框底） |
| `t.border` | color | 标准边框 |
| `t.border-soft` | color | 弱边框 |
| `t.muted` | color | 次级文字色 |
| `t.faint` | color | 弱化文字色 (≥4.0:1) |
| `t.rep-300` | color | 代表色大面积 |
| `t.rep-400` | color | 代表色发光/竖线 |
| `t.rep-600` | color | 代表色 hover/深 |
| `t.abyss-300` | color | 深渊青浅色 |
| `t.abyss-400` | color | 深渊青中色（思考块左缘） |
| `t.r-sm/md/lg/pill` | length | 圆角阶梯（9/14/18/999px） |
| `t.dur-normal` | duration | 常规过渡 350ms |
| `t.dur-once` | duration | 一次性入场 1200ms |

### 1.7 缺失 token 汇总（需在 FR-T3 前补入 RedesignTheme）

| 缺失 token | 用途 | 建议默认值 |
|---|---|---|
| `on-rep` | rep 色上的文字色（如发送按钮白字） | 亮 `#FFF9F5` / 暗 `#FFF9F5` |
| `on-abyss` | abyss 色上的文字色 | 亮 `#FFFFFF` / 暗 `#FFFFFF` |
| `on-danger` | danger 色上的文字色 | 亮 `#FFFFFF` / 暗 `#FFFFFF` |
| `fs-headline` | 大标题（若需保留） | 可废弃，v2 不用此层级 |

---

## 表格 2：.slint 文件换绑清单

> 复杂度判定标准：  
> - **低**：≤5 引用，仅 token 替换，无结构变化  
> - **中**：6-20 引用，token 替换 + 少量属性迁移（如 border-radius 改圆角阶梯）  
> - **高**：>20 引用，或需要结构性重构（如硬编码色删除、布局重构、新增动效绑定）

| 文件 | 行数 | MaterialTheme 引用数 | 复杂度 | 备注 |
|---|---|---|---|---|
| `views/ChatPaneView.slint` | 431 | 58 | **高** | 引用最多；含 current-primary/.with-alpha() 混色、error 色、spacing/font-size 全系；需拆 turn-meta、加活跃轮竖线+面 |
| `views/ProviderSettingsPanel.slint` | 452 | 57 | **高** | 大量表单布局；current-primary 高频、error 验证态、spacing 全系；需 on-rep 补全 |
| `views/SidebarView.slint` | 470 | 55 | **高** | 会话列表主组件；dark-mode 硬编码色 (#CF6679/#B00020/#000000/#FFFFFF) 需清；padding/spacing 密集；需适配 v2 沉积淡化+编年史 |
| `views/WelcomeView.slint` | 364 | 47 | **高** | 首次启动页；v2 需重构为 onboarding 范式（在场区+光环+心境语）；spacing-xl 高频 |
| `views/WorkspaceSettingsPanel.slint` | 336 | 42 | **高** | 工作区列表+技能覆盖；current-primary/.with-alpha() 高频 |
| `views/SettingsView.slint` | 264 | 37 | **高** | 设置壳页；5 个 tab 卡片选中态用 current-primary().with-alpha(0.12)；需改 rep 染色 |
| `views/MCPSettingsPanel.slint` | 238 | 30 | **中** | MCP 服务器列表；结构与 ProviderSettingsPanel 类似但更短 |
| `views/IdentityCreatorView.slint` | 218 | 25 | **高** | 身份创建器；v2 需映射到 onboarding 范式（五色板+诞生时刻）；结构重构 |
| `views/SkillsSettingsPanel.slint` | 153 | 25 | **中** | 技能列表；toggle 开关用 current-primary/current-surface |
| `views/InspectorView.slint` | 107 | 18 | **中** | 检查器面板；border-color 用 current-on-surface 需改 t.border |
| `components/ToolCallCard.slint` | 81 | 14 | **中** | 工具调用卡片；secondary/error 状态色需映射；spacing-sm/md |
| `components/ChatMessageBubble.slint` | 67 | 13 | **中** | 消息气泡；role 色分支需重写（user→elevated, assistant→surface）；on-primary 需 on-rep |
| `components/MaterialTextField.slint` | 52 | 13 | **中** | 输入框；border-color/current-on-surface/current-primary 需换绑；圆角 4px→r-sm(9px) |
| `components/MaterialBanner.slint` | 104 | 12 | **中** | 横幅；error/on-error 全替换为 danger/on-danger；spacing 全系 |
| `views/StatusBarView.slint` | 48 | 11 | **低** | 状态栏；border/current-on-surface/spacing 替换量小 |
| `components/MaterialList.slint` | 46 | 10 | **低** | 列表项；selected 态用 current-primary/on-primary；spacing-sm |
| `components/MaterialButton.slint` | 41 | 6 | **中** | 按钮；圆角 4px→r-sm(9px)；硬编码 #666666/#888888/#AAAAAA 需清；drop-shadow 需调 |
| `components/CodeBlock.slint` | 29 | 5 | **低** | 代码块；仅 font-size-body/current-on-surface |
| `components/MaterialBadge.slint` | 21 | 4 | **低** | 徽章；error→danger、on-error→on-danger、圆角 8px→r-md(14px) |
| `components/MaterialIconButton.slint` | 40 | 4 | **低** | 图标按钮；current-on-surface → t.fg/muted |
| `components/MaterialCard.slint` | 16 | 3 | **低** | 卡片；surface→t.surface、圆角 8px→r-md(14px)、shadow 需调 |
| `components/MarkdownText.slint` | 14 | 3 | **低** | 纯文字；font-size-body + current-on-surface |
| `main.slint` | 338 | 3 | **低** | 已 import RedesignTheme；仅需 current-background() → t.bg、dark-mode → dark |
| `theme.slint` | 159 | 1 | **低** | 定义文件本身；FR-T3 后逐步废弃（struct 仍需保留） |
| `strings.slint` | 134 | 0 | — | 无引用，无需改 |
| `redesign_palette.slint` | 148 | 2 | — | 仅注释提及 MaterialTheme；无需改 |

### 统计汇总

| 指标 | 数值 |
|---|---|
| 需换绑的 .slint 文件总数 | **24**（不含 strings.slint + redesign_palette.slint） |
| MaterialTheme 总引用数 | **528** |
| 高复杂度文件 | **8** |
| 中复杂度文件 | **8** |
| 低复杂度文件 | **8** |
| 含硬编码颜色的文件 | 至少 **4**（SidebarView、MaterialButton、ChatMessageBubble、main.slint） |

---

## 清单 3：需新建的 Slint 组件

> 基于 v2 范式（`theme-system.html` + 9 个原型）与现有 Slint 组件的 gap 分析。

| # | 组件名（建议） | 对应 HTML 范式元素 | 说明 | 优先级 |
|---|---|---|---|---|
| 1 | `PresenceBar.slint` | `.presence` 在场区 | 头像 64px + 光环 auraBreath + 名字 + 状态点 + 编年史条 + 心境语；v2 核心标志组件 | P0 |
| 2 | `AvatarWrap.slint` | `.avatar-wrap` + `.avatar` | 头像容器 + 呼吸光环 + 径向渐变头像底；含 `animation-tick()` 呼吸驱动 | P0 |
| 3 | `ChronicleBar.slint` | `.p-chrono` | 编年史进度条：`@linear-gradient(90deg, birth, rep-300, rep-500, rep-400)` | P0 |
| 4 | `MoodText.slint` | `.p-mood` | 心境语：延迟淡入 + 呼吸脉冲动画（moodIn + moodPulse） | P1 |
| 5 | `DeckBar.slint` | `.deck` + `.deck-bar` | v2 操控台：输入区 + 工具栏（模型选择器染 rep、思考深度、上下文环、发送按钮） | P0 |
| 6 | `WindowChrome.slint` | `.win-ctrl` + `.watermark` + `.handle` | 窗口控制三按钮 + 品牌水印 + 侧把手；v2 chrome 层 | P0 |
| 7 | `AirTint.slint` | `#app` 背景层 | 整屋空气染色：底色平铺 rep 3.5% + 顶晕径向 7% + 底部冷雾 1.5%；预计算 hex 平铺 Rectangle | P0 |
| 8 | `TurnContainer.slint` | `.turn` + `.turn.active` + `.turn.sediment` | 对话轮容器：活跃轮 4% rep 面 + 2.5px 竖线 + 沉积轮 opacity 0.5→0.7 hover | P0 |
| 9 | `ThinkBlock.slint` | `.think` | 思考块：abyss-400 左缘 2px + 冷底色 rgba(63,131,123,.05) | P1 |
| 10 | `ToolChip.slint` | `.chip` + `.chip.running` + `.chip.done` | 工具调用 chip：running 染 rep、done 染 abyss | P1 |
| 11 | `ContextRing.slint` | `.ctx-ring` | 上下文使用率环形指示器（SVG 圆环 + 百分比） | P2 |
| 12 | `ScrollbarStyled.slint` | `.stream::-webkit-scrollbar` | 自定义滚动条样式（rep-400 混色 thumb） | P2 |
| 13 | `ThemeToggle.slint` | `.theme-toggle` ◐ | 亮/暗演示切换钮（FR-T5 正式接入后为显示模式切换） | P2 |

### 现有组件的 v2 改造（非新建，但需大改）

| 现有组件 | 改造内容 |
|---|---|
| `ChatMessageBubble.slint` | 拆分为 `UserBubble`（elevated 底 + border + shadow）和 `AgentMessage`（裸 .msg + fg 色） |
| `ToolCallCard.slint` | 重构为 `ToolChip` 风格（pill 圆角 + running/done 状态色） |
| `MaterialButton.slint` | 改名 `ActionButton`；圆角→r-sm；清除硬编码色；primary 态用 rep-500 + on-rep |
| `MaterialCard.slint` | 改名 `Surface`；圆角→r-md；shadow 调整为 v2 柔影 |
| `MaterialTextField.slint` | 改名 `DeckInput`；圆角→r-md；focus 态用 rep-500 边框 + 光晕 |
| `MaterialBanner.slint` | 改名 `AlertBanner`；error→danger |
| `MaterialList.slint` | 改名 `NavList`；selected 态用 rep 染色 |
| `MaterialBadge.slint` | 改名 `StatusPill`；圆角→r-pill |

---

## 清单 4：v2 标志性视觉元素落地状态

| 视觉元素 | HTML 范式状态 | Slint 落地状态 | 阻塞原因 | FR-T3 可落地？ |
|---|---|---|---|---|
| **整屋空气染色**（底平铺 rep 3.5% + 顶晕 7%） | ✅ theme-system.html 完成 | ❌ 未开始 | 需预计算 color-mix hex（扩 oklch-to-srgb.py 生成器） | ✅ 低风险（POC 验证通过） |
| **顶晕径向渐变** | ✅ `#app::before` radial-gradient | ❌ 未开始 | `@radial-gradient` at 参数需用 `parent.width/2` 表达式 | ✅ 低风险 |
| **底部冷雾**（abyss 1.5%） | ✅ `#app::after` | ❌ 未开始 | 同上，径向渐变 | ✅ 低风险 |
| **头像体温光晕**（30% rep 径向） | ✅ `.presence::before` | ❌ 未开始 | 需新建 PresenceBar + Rectangle 叠层 | ✅ 中风险（径向定位） |
| **头像呼吸光环**（auraBreath 6s） | ✅ `.avatar-wrap::after` | ❌ 未开始 | `animation-tick()` + `Math.sin()` 绑 scale-x/y；POC 未验端到端 | ⚠️ 中风险（POC 标注"机制可行、闭环待验"） |
| **编年史条**（linear-gradient birth→rep） | ✅ `.p-chrono` | ❌ 未开始 | `@linear-gradient(90deg, ...)` | ✅ 低风险 |
| **心境语延迟淡入 + 呼吸** | ✅ `.p-mood` moodIn+moodPulse | ❌ 未开始 | 需 animation-tick + opacity 绑定 | ⚠️ 中风险（双动画叠加） |
| **活跃轮竖线 + 4% rep 面** | ✅ `.turn.active` | ❌ 未开始 | 需 TurnContainer 组件 + 预计算 4% 混色 hex | ✅ 低风险 |
| **沉积轮 hover 淡入** | ✅ `.turn.sediment` opacity .5→.7 | ❌ 未开始 | TouchArea has-hover + opacity 绑定 | ✅ 低风险 |
| **操控台模型名 ◇ 染 rep** | ✅ `.db .di.rep` | ❌ 未开始 | 直接 `color: t.rep-500` | ✅ 低风险 |
| **输入聚焦整屋升档**（speaking 状态） | ✅ `#app.speaking` | ❌ 未开始 | 需 DeckBar focus-in/out 回调 → 全局状态 → AirTint 背景切换 | ⚠️ 中风险（跨组件状态传递） |
| **思考块撤回**（abyss 左缘 + 冷底） | ✅ `.think` | ❌ 未开始 | 需 ThinkBlock 组件 | ✅ 低风险 |
| **窗口控制 −□×** | ✅ `.win-ctrl` | ❌ 未开始 | 需 WindowChrome 组件 + Rust 侧窗口管理回调 | ⚠️ 需 Rust 侧配合 |
| **品牌水印**（左下 opacity .25） | ✅ `.watermark` | ❌ 未开始 | 需 WindowChrome 子元素 | ✅ 低风险 |
| **把手垂直居中** | ✅ `.handle` justify-content:center | ❌ 未开始 | 需 WindowChrome 布局调整 | ✅ 低风险 |
| **暗色皮肤翻转** | ✅ `[data-theme="dark"]` | ✅ RedesignTheme.dark 三元已实现 | — | ✅ 已就绪 |
| **自定义滚动条**（rep-400 混色） | ✅ `.stream::-webkit-scrollbar` | ❌ 未开始 | Slint 无原生 scrollbar 自定义样式 API | ⚠️ 高风险（需自定义 ScrollView 或放弃） |
| **::selection 染色** | ✅ `::selection` | ❌ 不适用 | Slint 无文本选区概念 | ❌ 无法落地（放弃） |
| **:focus-visible outline** | ✅ `:focus-visible` | ❌ 不适用 | Slint 用 TouchArea has-focus | ⚠️ 需逐组件适配 |

---

## 总估算：FR-T3 工作量

### 文件数统计

| 类别 | 数量 |
|---|---|
| 需换绑的现有 .slint 文件 | 24 |
| 需新建的 Slint 组件 | 13 |
| 需大改（改名+重构）的现有组件 | 8 |
| 需扩展的生成器脚本（oklch-to-srgb.py） | 1 |
| 需补充的 RedesignTheme token（on-rep/on-abyss/on-danger） | 3-4 个 |
| **总触及文件数** | **~30** |

### 工作量分解

| 工作项 | 预计复杂度 | 说明 |
|---|---|---|
| **Phase 1：Token 补全** | 低 | 补 on-rep/on-abyss/on-danger 到 RedesignTheme；扩生成器 |
| **Phase 2：低复杂度文件换绑**（8 文件，~43 引用） | 低 | 纯 token 替换：`MaterialTheme.current-X()` → `RedesignTheme.t.X` |
| **Phase 3：中复杂度文件换绑**（8 文件，~100 引用） | 中 | token 替换 + 圆角阶梯迁移 + 硬编码色清理 + 组件改名 |
| **Phase 4：高复杂度文件换绑**（8 文件，~385 引用） | 高 | token 替换 + 结构重构（侧边栏 sediment/编年史、设置 tab 染色、onboarding 重构） |
| **Phase 5：新建 v2 标志组件**（13 个） | 高 | PresenceBar/AvatarWrap/DeckBar/WindowChrome/AirTint/TurnContainer 为核心 |
| **Phase 6：动画闭环验证** | 中-高 | 呼吸（animation-tick → scale）、心境语双动画、speaking 升档——POC 标注的"闭环待验"项 |
| **Phase 7：硬编码色清扫** | 中 | SidebarView 4 处、MaterialButton 3 处、ChatMessageBubble 分支色、main.slint 1 处 |
| **Phase 8：Rust 侧回调** | 中 | 窗口控制（最小化/最大化/关闭）需接 Rust window management |

### 阻塞依赖链

```
Token 补全 (on-rep/on-abyss/on-danger)
    ↓
低复杂度文件换绑（可并行）
    ↓
中复杂度文件换绑（依赖 token 补全）
    ↓
高复杂度文件换绑（依赖中复杂度完成 + 新组件就绪）
    ↓
新建 v2 组件（AirTint → PresenceBar → TurnContainer → DeckBar 顺序）
    ↓
动画闭环验证（呼吸/心境语/speaking 升档）
    ↓
Rust 侧窗口控制回调
    ↓
整体走查 + 暗色验证
```

### 风险评估

| 风险项 | 等级 | 说明 |
|---|---|---|
| 呼吸动画端到端 | 中 | POC 验了"算得出"未验"接得上+看得见"；`animation-tick()` → `Math.sin()` → `scale-x/y` 需肉眼确认 |
| 径向渐变定位 | 中 | `@radial-gradient(circle at Xpx Ypx, ...)` 的 at 只吃 px 不吃 %，需绑 `parent.width/2` 表达式；resize 时需验证不偏移 |
| 自定义滚动条 | 高 | Slint 无原生 scrollbar 样式 API；可能需放弃此视觉元素或自定义 ScrollView |
| color-mix 预计算 | 中 | 需扩 `oklch-to-srgb.py` 生成器产出混合色；FR-T3 期间若频繁调色需多次重跑生成器 |
| 工具链环境债 | 中 | `pnpm run desktop:check` 不可用（gcc DLL 地狱）；需走 MSVC 绕路验证 |
| 跨组件状态传递 | 中 | speaking 升档需 DeckBar focus → 全局状态 → AirTint 背景切换；Slint 无全局状态总线（需通过 property 链） |

### 总判

**FR-T3 是一个中大型重构任务**：触及 ~30 个文件、528 个 token 引用替换、13 个新组件、8 个组件改名重构。核心阻塞面不在 token 替换本身（机械工作），而在于：

1. **v2 标志组件新建**（PresenceBar/DeckBar/AirTint 等 6 个 P0 组件）
2. **动画闭环**（呼吸/心境语/speaking 升档——POC 留了"最后一公里"未验）
3. **高复杂度 view 重构**（SidebarView 470 行 55 引用 + 硬编码色、ChatPaneView 431 行 58 引用）
4. **3-4 个缺失 token 补全**（on-rep/on-abyss/on-danger——阻塞按钮/徽章/验证态组件）

建议分两批执行：
- **FR-T3a**（基础设施）：Token 补全 + 生成器扩展 + 低复杂度文件换绑 + 新建 AirTint/WindowChrome
- **FR-T3b**（核心重构）：高复杂度 view 换绑 + 新建 PresenceBar/DeckBar/TurnContainer + 动画闭环验证

---

*报告生成：2026-07-27 01:49 CST · 审计员：Slint 前端审计 subagent*
