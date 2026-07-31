# Slint-Safe CSS 规范（基于 Slint 1.17 平台限制）

> 本文档定义 HTML/CSS 原型 → Slint 翻译时的约束规范与替代方案。
> 配套文件：`tokens-srgb-table.md`（颜色 token 对照表）、`slint-feasibility-poc.md`（POC 实测结论）。

---

## 1. 翻译红线（FR-T3 必须遵守）

> 以下规则为硬约束，违反即打回。

1. **品牌 logo 不染 rep** — logo 区域保持原色，不叠加 rep 色系
2. **思考块底不染 rep** — 思考区域使用 abyss 冷色系
3. **沉积轮不染当前 rep** — 编年史区域独立配色
4. **用户气泡不染 rep 边/底** — 用户消息气泡保持中立
5. **正文不染 rep** — 正文文本区域不叠加 rep 色
6. **z-index 叠层用 Rectangle 声明顺序** — 无 `#app>*` 通配概念，勿用全局覆盖压住绝对定位元素
7. **暗色模式直接读 `RedesignTheme.t`** — 不要另起变量体系
8. **color-mix 走生成器** — 扩 `oklch-to-srgb.py` 把混合色算进 `redesign_palette.slint` 的 struct，组件读 `RedesignTheme.t.<token>`；POC 局部硬编码 hex 仅探针用，不进产品

---

## 2. v2 设计范式（11 条）

> 来源：prototypes/README.md + theme-system.html 范式真值。Slint 翻译时每条须有对应实现。

### 2.1 整屋空气染色

底平铺 rep 3.5% + 顶晕 7%（衰减到 100% 才透明，整窗带色相）+ 头像体温光晕 30% + 底冷雾 1.5%；输入聚焦整屋升档（底 4.5% + 顶 10%）。设置页淡档（底 1.5% + 顶 2%）。档案馆冷雾（abyss 底 1.5% + 顶 3%）。

**Slint**：全屏 `Rectangle` 平铺预计算 hex；顶晕 = `@radial-gradient(circle at ...)`；体温 = 头像叠层径向；冷雾 = 底部 `Rectangle` + abyss 预计算 hex。

### 2.2 居中在场区

头像 64px 方 + 光环 `auraBreath`（6s 呼吸 ±1.5%）+ 名字 + 状态 + 编年史条 + 心境语。

**Slint**：`VerticalLayout` 居中；光环 = 嵌套 `Rectangle` + `animation-tick()` 驱动 `scale-x`/`scale-y`。

### 2.3 Chrome 布局

把手垂直居中；窗口控制 −□× 右上 `right:44px`；品牌水印左下 `left:44px bottom:22px` opacity .25。

**Slint**：绝对定位 `Rectangle`，`x: parent.width - 44px - self.width`；水印 `opacity: 0.25`。

### 2.4 对话活跃轮

4% rep 面 + 左 2.5px 竖线 + `.msg` weight 450；turn-meta 行删除；模型名 ◇ 染 rep。

**Slint**：面 = 预计算 hex `Rectangle`；竖线 = 2.5px 宽 `Rectangle` + `@linear-gradient(180deg, ...)`。

### 2.5 暗色模式

`[data-theme="dark"]`：深暖黑 `#181612` 系（禁纯黑/霓虹），rep 走辉光逻辑（顶晕/体温降档）。

**Slint**：`dark ? DARK : LIGHT` 三元，`RedesignTheme` 已落地。

### 2.6 呼吸动效

6s 周期 ±1.5% scale，仅活物（头像光环、在场区）。

**Slint**：`property<float> s: 1.0 + 0.015 * Math.sin(animation-tick() / 6000ms * 360deg)` 绑 `scale-x`/`scale-y`。

### 2.7 编年史渐变

`@linear-gradient(90deg, birth, rep-300, rep-500)`，沉积轮保持褪色灰（不染当前 rep）。

**Slint**：`@linear-gradient(90deg, t.birth, t.rep-300, t.rep-500)`。

### 2.8 档案馆冷雾

abyss 色系：底 1.5% + 顶 3%，只读氛围。

**Slint**：`page == "archive"` 时选 abyss 预计算 hex。

### 2.9 设置页淡档

rep 底色 1.5% + 顶晕 2%，比主对话区更淡。

**Slint**：独立预计算 hex，不用 `color-mix`。

### 2.10 代表色自主权

代表色由 agent 自主更换；人类除首次 onboarding 选色板外不可改色。界面与设置页不放人类换色控件。

**Slint**：`property <color> rep-500` 由 Rust 侧赋值，无 UI 控件暴露。

### 2.11 字体体系

Fraunces（WONK 1, SOFT 60）品牌/名字 · Noto Sans SC 400 正文 · JetBrains Mono 元数据。字体已预实例化为静态 ttf。

**Slint**：`import "./fonts/Fraunces-Display.ttf";` + `font-family: "Fraunces Display"`。

---

## 3. Design Token 规范

> Token 是设计范式的原子单位。Slint 中所有组件通过 `RedesignTheme.t.<token>` 读取，**禁止硬编码数值**。

### 3.1 颜色 Token

> 完整 48 token 双模式映射见 `tokens-srgb-table.md`。

#### 色彩语义域

| 域 | 色相 | 用途 | 红线 |
|---|---|---|---|
| **rep**（代表色） | 暖（默认珊瑚 hue≈30°） | 整屋染色、在场光晕、体温光环、活跃轮竖线/面、模型名 ◇ | logo/思考块/沉积轮/用户气泡/正文不染 rep |
| **abyss**（深渊青） | 冷（hue≈185°） | 思考块底、底冷雾、暗色体温、档案馆 | — |
| **warm**（暖中性） | 暖灰阶 | 背景/表面/边框/文本（结构色） | — |
| **birth**（出生灰） | 无彩 | 编年史条最左端 | — |

#### 高频颜色速查

| token | light | dark | 用途 |
|---|---|---|---|
| `bg` | `#F4F3F0` | `#151411` | 主背景 |
| `surface` | `#FBFAF8` | `#1C1A17` | 卡片底 |
| `elevated` | `#FFFFFF` | `#23211F` | 浮起卡片 |
| `raised` | `#EFEDE8` | `#2A2925` | 控件底 |
| `border` | `#E6E3DD` | `#3A3833` | 边框 |
| `border-soft` | `#ECEAE5` | `#32302C` | 弱边框 |
| `fg` | `#38352E` | `#EBE8E0` | 正文 |
| `muted` | `#7B766C` | `#9C988F` | 次级文本 |
| `faint` | `#A8A398` | `#74716A` | 弱化文本（≥4.0:1） |
| `rep-300` | `#B6B6B6` | `#B6B6B6` | 大面积灰（珊瑚 fallback） |
| `rep-400` | `#9F9F9F` | `#9F9F9F` | 发光/竖线灰 |
| `rep-500` | `#8B8B8B` | `#8B8B8B` | 强调渐变右端灰 |
| `rep-600` | `#727272` | `#727272` | hover/深灰 |
| `abyss-300` | `#7AABA4` | `#7AABA4` | 深渊青浅 |
| `abyss-400` | `#5A9B93` | `#5A9B93` | 深渊青中 |
| `abyss-500` | `#3F837B` | `#3F837B` | 深渊青深 |
| `danger` | `#A45950` | `#C37D73` | 陶红 |
| `birth` | `#DAD6CF` | `#4A443B` | 编年史起点 |
| `scrim` | `#38352E` | `#000000` | 遮罩暗化 |
| `shadow` | `#38352E` | `#000000` | 阴影投射 |

### 3.2 间距 Token

| token | 值 | 用途 |
|---|---|---|
| `s1` | `4px` | 最小间距 |
| `s2` | `8px` | 紧凑间距 |
| `s3` | `12px` | 默认内间距 |
| `s4` | `16px` | 模块间距 |
| `s5` | `24px` | 区域间距 |
| `s6` | `32px` | 大区域间距 |

**Slint**：`padding: t.s3;` / `gap: t.s2;`

### 3.3 圆角 Token

| token | 值 | 用途 |
|---|---|---|
| `r-sm` | `9px` | 控件、deck-bar 按钮 |
| `r-md` | `14px` | 左栏卡片、头像、用户气泡 |
| `r-lg` | `18px` | 大卡片 |
| `r-pill` | `999px` | chip、自我认知渐变条、呼吸点 |

**Slint**：`border-radius: t.r-md;`（窗口 20px 为全局常量，不进 token）

### 3.4 字号 Token

| token | 值 | 用途 |
|---|---|---|
| `fs-sm` | `10px` | 时间戳、chip 细节、turn-meta |
| `fs-md` | `11.5px` | turn-head、状态行、ctx 标签 |
| `fs-lg` | `13px` | 名字、chip、模块标题 |
| `fs-body` | `15px` | 对话正文 |
| `fs-name` | `16px` | agent 名 |

**Slint**：`font-size: t.fs-body;`

### 3.5 动效时长 Token

| token | 值 | 用途 |
|---|---|---|
| `dur-hover` | `150ms` | hover 背景变化 |
| `dur-normal` | `350ms` | 常规过渡 |
| `dur-slide` | `250ms` | 抽屉滑入 |
| `dur-once` | `1200ms` | 沉积式一次性过渡 |
| `dur-breathe` | `6000ms` | 呼吸周期 |

**Slint**：`animate background { duration: t.dur-normal; easing: ease-in-out; }`

---

## 4. 禁止使用的 CSS 特性及替代方案

| # | 禁止使用 | 替代方案 | 说明 |
|---|---|---|---|
| 1 | `box-shadow` | `filter: drop-shadow()` 或纯 border 层次 | Slint 用 `drop-shadow-blur` / `drop-shadow-color` / `drop-shadow-offset-x/y` 属性 |
| 2 | `color-mix(in srgb, X p%, Y)` | 预计算 hex 值 | 查 `tokens-srgb-table.md` 获取对应 hex；大量混合色须扩 `oklch-to-srgb.py` 生成器产出进 `redesign_palette.slint`，**勿手改该文件** |
| 3 | `radial-gradient(ellipse X% Y% at A% B%)` | `@radial-gradient(circle at Xpx Ypx, ...)` | Slint 仅支持 `circle`，`at` 位置用 px / length 表达式（如 `parent.width/2`），**禁硬编码 px**（否则 resize 偏移） |
| 4 | `::before` / `::after` 伪元素 | 显式 `<div>` 嵌套元素 | Slint 用嵌套 `Rectangle` 替代；叠层顺序由 Rectangle 声明顺序决定 |
| 5 | `@keyframes ... infinite` 无限循环 | 标注 `animation-tick()` 驱动等效 | Slint: `property<float> s: 1 + 0.015 * Math.sin(animation-tick() / 6000ms * 360deg)`；`Math.sin` 参数须为 angle，无 `Math.PI` |
| 6 | `::selection` | 删除，标注 "Slint: 不支持文本选区" | — |
| 7 | `::-webkit-scrollbar` | 删除，标注 "Slint: 平台默认滚动条" | — |
| 8 | `font-variation-settings` | 删除，字体已预实例化 | WONK1+SOFT60 已烘焙进静态 ttf；Slint 用 `import "./fonts/...ttf"; font-family:"..."` |
| 9 | `transition: all` | 逐属性标注 `duration` / `easing` | Slint 用 `animate <prop> { duration: ...; easing: ... }`，必须逐属性声明 |
| 10 | `max-height` + `overflow: auto` | 标注为 Slint `height` + `clip: true` | Slint 无 CSS overflow 概念，用 `clip: true` 裁剪超出内容 |
| 11 | `%` 定位值 | 标注等效 px 表达式 | `left: 50%` → `left: parent.width / 2`；`top: 30%` → `top: parent.height * 0.3` |

---

## 5. HTML 原型标注规范

每个 HTML 元素应添加注释，标注其 Slint 映射关系，便于 FR-T3 翻译阶段直接对照：

```html
<!-- Slint: Rectangle { background: t.elevated; border-radius: t.r-md; drop-shadow-blur: 8px } -->
<div class="card">...</div>

<!-- Slint: Rectangle { background: t.surface; border-radius: t.r-lg; } -->
<!-- Slint:   animate background { duration: t.dur-normal; easing: ease-in-out } -->
<div class="panel">...</div>

<!-- Slint: TouchArea { has-hover => animate background { duration: t.dur-hover } } -->
<div class="btn" data-hover="true">...</div>
```

**标注要求：**

- 每个可视元素必须有 `<!-- Slint: ... -->` 注释
- 包含动画的属性单独一行标注
- 颜色值使用 token 名（如 `t.elevated`），不写死 hex
- 嵌套关系反映在缩进层级中

---

## 6. 动效映射表

| HTML/CSS 动效 | Slint 实现 | Token |
|---|---|---|
| `transition: background 350ms ease-in-out` | `animate background { duration: 350ms; easing: ease-in-out }` | `t.dur-normal` |
| `transition: background 1200ms ease-in-out` | `animate background { duration: 1200ms; easing: ease-in-out }` | `t.dur-once` |
| `@keyframes breathe { 50% { transform: scale(1.015) } }` infinite | `property<float> s: 1 + 0.015 * Math.sin(animation-tick() / 6000ms * 360deg)` 绑 `scale-x` / `scale-y` | `t.dur-breathe` |
| 抽屉 x 滑入 `transition: transform 250ms` | `animate x { duration: 250ms; easing: ease-in-out }` | `t.dur-slide` |
| `:hover` 背景变化 `transition: background 150ms` | `animate background { duration: 150ms }` | `t.dur-hover` |
| `linear-gradient(to right, ...)` | `@linear-gradient(90deg, ...)` | — |
| `linear-gradient(to bottom, ...)` | `@linear-gradient(180deg, ...)` | — |

**动效注意事项：**

- `Math.sin()` 参数必须为 angle 类型：`.../周期ms * 360deg`
- Slint 无 `Math.PI`，不可使用
- `animation-tick()` 返回 `duration` 类型，可直接做除法
- 每个 `animate` 块必须逐属性声明，不可用 `all`

---

## 7. 已验证可实现的 Slint 特性（POC 实测）

以下特性均通过 `poc_v2_visual_probe.slint` 编译验证（Slint 1.17），零不可行项：

| # | 特性 | Slint 实现 | 折扣 |
|---|---|---|---|
| 1 | **整屋空气染色** | 全屏 `Rectangle { background: <预计算hex>; }` 平铺 | 0 |
| 2 | **顶晕径向渐变** | `@radial-gradient(circle at <Xpx> <Ypx>, c1, c100 70%)`，位置绑 `parent.width/2` 表达式 | 低 |
| 3 | **头像体温径向** | `@radial-gradient(circle, c30, c00 70%)` 叠头像径向 | 低 |
| 4 | **呼吸无限循环** | `animation-tick()` 驱动 `Math.sin()` → 绑 `scale-x`/`scale-y`（纯 Slint，无需 Rust Timer） | 低-中（FR-T3 须确认 scale 绑定+肉眼可见） |
| 5 | **编年史线性渐变** | `@linear-gradient(90deg, ...)` | 低 |
| 6 | **活跃轮竖线+面** | 面=预计算 hex Rectangle + 竖线 `@linear-gradient(180deg,...)` 2.5px | 0 |
| 7 | **暗色模式翻转** | `dark ? DARK : LIGHT` 三元（`RedesignTheme` 已落地） | 0 |
| 8 | **抽屉滑入** | `animate x { duration: 250ms; easing: ease-in-out }` | 0 |
| 9 | **窗口控制** | Slint 原生窗口 API | 0 |

---

## 8. HTML data 属性 → Slint 条件表达式映射

> **核心原则**：HTML 中的 `data-*` 属性是颜色语义的**唯一真值源**，CSS 类名（`.rep-500`、`.abyss-300`、`.think` 等）仅是样式选择器，不承载语义。Slint 翻译时**忽略类名**，将 `data-*` 属性转为 property 条件表达式。

### 8.1 主题切换

| HTML data 属性 | CSS 选择器 | Slint 映射 |
|---|---|---|
| `data-theme="dark"` | `[data-theme="dark"] { --bg: #181612; ... }` | `property<bool> dark;` → `dark ? t_dark.bg : t_light.bg`（已由 `RedesignTheme.dark` 三元落地） |
| 无 `data-theme`（light 默认） | `:root { --bg: #F4F3F0; ... }` | `dark == false` 分支 |

**Slint 实现**：`RedesignTheme` 已封装 `dark` 三元，组件直接读 `RedesignTheme.t.<token>`，不要另起变量体系。

### 8.2 色彩语义域映射

HTML 原型中颜色通过 CSS 变量（`var(--rep-500)`、`var(--abyss-500)`）注入到不同语义区域。Slint 中需将**语义区域**建模为 property，用条件表达式选色：

| 语义域 | HTML 中的 CSS 变量用法 | 典型 CSS 选择器 | Slint property | Slint 条件表达式 |
|---|---|---|---|---|
| **rep（代表色暖系）** | `var(--rep-300/400/500/600)` | `#app` 整屋染色、`.presence::before` 在场光晕、`.avatar-wrap::after` 体温光环、活跃轮竖线 | `property<string> area: "rep";` | `area == "rep" ? t.rep-500 : ...` |
| **abyss（深渊青冷系）** | `var(--abyss-300/400/500)` | `.think` 思考块底、`#app::after` 底冷雾、`.avatar::after` 暗色体温 | `area == "abyss"` | `area == "abyss" ? t.abyss-500 : t.rep-500` |
| **warm（暖中性）** | `var(--bg)` ~ `var(--raised)` 暖灰阶 | `.shell` 设置页底、用户气泡、正文区 | 隐含（默认分支） | 直接读 `t.bg` / `t.surface` 等结构 token |
| **birth（出生灰）** | `var(--birth)` | 编年史条最左端 | 常量 | `t.birth`（无需条件） |

### 8.3 状态驱动的条件染色

HTML 原型中通过类名切换（`.speaking`）实现状态驱动染色，Slint 须转为 property 条件：

| HTML 状态 | CSS 实现 | Slint 映射 |
|---|---|---|
| `#app.speaking`（agent 说话中） | `#app.speaking { background: linear-gradient(... rep-500 4.2% ...) }` 整屋升档 | `property<bool> speaking;` → `speaking ? t.rep-500-mix-high : t.rep-500-mix-low` 绑渐变 stop |
| `.speaking::before`（顶晕升档） | `color-mix(in srgb, var(--rep-500) 10%, transparent)` | `speaking ? 0.10 : 0.055` 绑 drop-shadow 或 gradient stop alpha |
| 输入聚焦（整屋升档） | 底 4.5% + 顶 10% | `property<bool> input-focused;` → 条件选预计算 hex |
| 档案馆冷雾 | `var(--abyss-500)` 底 1.5% + 顶 3% | `page == "archive" ? t.abyss-500-mix : t.rep-500-mix` |

### 8.4 完整翻译示例

**HTML 原型：**
```html
<!-- 整屋染色：rep 代表色暖系 -->
<div id="app" data-theme="dark" class="speaking">
  <!-- 顶晕：rep 光晕 -->
  <!-- 底冷雾：abyss 青系 -->
  <!-- 在场区光晕：rep 体温 -->
  <!-- 思考块：abyss 冷底 -->
</div>
```

**Slint 翻译：**
```slint
// property 声明（语义真值，不依赖类名）
property <bool> dark: true;
property <bool> speaking: true;
property <string> area: "rep";       // 语义域：rep / abyss / warm
property <string> page: "home";      // 页面：home / archive / settings

// 整屋背景 — 条件选预计算 hex
Rectangle {
    background: speaking
        ? @linear-gradient(180deg, t.rep-500-mix-8, t.rep-500-mix-3 50%, t.bg 100%)
        : @linear-gradient(180deg, t.rep-500-mix-6, t.rep-500-mix-2 50%, t.bg 100%);
    animate background { duration: t.dur-once; easing: ease-in-out; }
}

// 顶晕 — 条件 alpha
Rectangle {
    background: @radial-gradient(circle at (parent.width/2) (parent.height*0.01),
        speaking ? #C8714C19 : #C8714C0E, transparent 90%);
    animate background { duration: t.dur-once; }
}

// 思考块 — abyss 语义域
Rectangle {
    // area == "abyss" 时直接读 abyss token，无需条件
    background: #3F837B1A;  // abyss-500 @ 10%
}
```

### 8.5 翻译规则总结

1. **data 属性 → property**：每个 `data-*` 属性对应一个 Slint `property`，由 Rust 侧或父组件赋值
2. **类名 → 忽略**：`.rep-500`、`.abyss-300`、`.speaking` 等类名不翻译，其语义已由 property 承载
3. **CSS 变量 → token 读取**：`var(--rep-500)` → `t.rep-500`，`color-mix(...)` → 预计算 hex（查 `tokens-srgb-table.md` 或生成器产出）
4. **条件分支 → 三元表达式**：`[data-theme="dark"]` → `dark ? X : Y`；`.speaking` → `speaking ? X : Y`
5. **语义域互斥**：rep 与 abyss 不混用——同一元素只读一个域的 token，由 `area` property 决定
