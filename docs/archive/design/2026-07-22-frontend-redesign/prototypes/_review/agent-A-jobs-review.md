# northing v2 原型 — 乔布斯极简设计哲学审视报告

> 审视官:jobs-design-assistant
> 范围:`theme-system.html`(范式真值)/ `space-view.html`(空间主页)/ `chat-expanded.html`(对话展开)/ `archive.html`(档案馆)+ shared 4 文件
> 基线:`_review/design-philosophy-distilled.md`("哲学十戒" + "反 AI 味 8 条")
> 方法:按 SKILL.md 要求,先整体判断 → 最大问题 3 条 → 精简建议 5 条,维度限定 ① 视觉层级 ② 简洁性 ③ 统一性 ④ 可读性 ⑤ 留白

---

## 整体判断:**合格**(良好基底,工具栏化滑坡)

整屋空气染色、暖灰基底、字体三系分离、角落水印、SVG 线性图标——这五条反 AI 味核心证据,这一套原型**几乎全部做对**。它不是又一套 Tailwind 模板,也不是又一套"渐变 + 玻璃拟态"的现代 AI 套壳。它有自己的语气:暖灰底、珊瑚强调、沉降而非弹跳、呼吸而非闪烁。这是乔布斯会坐下来的那种设计。

但"几乎"不是"完全"。在跨页迁移过程中,设计哲学悄悄向 IDE 工具栏投降——`chat-expanded.html` 底部那一条 7 控件的操控台,把"咨询室"还原成"集成开发环境"。同时,跨页的尺寸/动效存在系统性的统一性失分。

---

## 最大问题(3 条)

### 问题 1 · 操控台 + 顶栏的工具栏化倾向(违反哲学第 1 戒"拒绝 dashboard 美学")

**证据**:
- `chat-expanded.html` L630-669 的 `.deck-bar`:左到右塞了 ＋ 附件、思考分段条(5 段)、工作目录 `~/northing`、模型 `Claude 4.6 ◇ ▾`、自治 `◈ 自治·完全 ▾`、ctx 圆环(20×20,abyss-500 前景,38%)。**7 个元素压在 38px 一行**。这已经不再是"咨询室的器物",这是 VS Code 的状态栏。
- `space-view.html` L483-513 顶栏 60px 高度内塞了 compact 名片 + 设置 + 搜索框 + 筛选 + 新建按钮。**5 个控件在 60px 一行**。
- 这两处对"咨询室"的承诺,被"功能完整"的需求反噬。`onboarding.html` 的孤独/仪式感在这些页全消失了。

**为什么是问题**:哲学第 1 戒的判定问"看起来像控制台还是安静的房间?"——这两个页答案偏向"控制台"。乔布斯在 iPhone 1 代砍掉了所有快捷键、设置项、状态指示,因为"工具栏 = 用户被迫理解系统的逻辑"。当用户在咨询室与 agent 对话,他/她不需要看到"上下文已用 38%"(这是 agent 的事),也不需要在 5 段思考条上做选择(这是 agent 自己的分段)。

**修复方向**:操控台 7 控件压成 2-3 个(输入框 + 发送键 + 一个"更多"折叠入口)。ctx 圆环、思考分段条、工作目录、模型选择、自治档位全部下沉到右抽屉"身外之物"。顶栏搜索/筛选同样下沉。**让对话流成为主战场,工具栏消失**。

---

### 问题 2 · 跨页尺寸三/四态不统一(统一性失分)

**证据**:
- agent 头像:space-view 用 32px(`space-view.html` L58)、chat-collapsed/expanded 用 44px(`chat-expanded.html` L67)、archive 用 40px(`archive.html` L72)。**同一个 agent 的"名片"在三个页是三个尺寸**。
- 顶栏高度:settings 是 52px(`layout.css` L132)、space-view 是 60px(L41)、archive 是 64px(L61)、chat-collapsed/expanded 是 80px(L51)。**四种高度,差 28px**。
- 收起态把手:shared 默认 34px(`components.css` L622),space-view 覆写 8px(L419),chat-expanded 覆写 8px(L243),archive 覆写 8px(L212)。覆写三次,每次都重写背景渐变与 border-image。

**为什么是问题**:乔布斯在 iOS Human Interface Guidelines 用 44pt 触控目标、17pt 字号、20pt 默认行距——他建立的是**不可变参考系**。当 avatar 在 32/40/44 之间漂移,用户的"agent 是个稳定存在的个体"感受会碎。

**修复方向**:把 avatar 统一为两态(顶栏态 40-44px + 在场态 64px)。顶栏高度统一 64px(参考 archive,介于 60 与 80 之间最"咨询室")。把把手 8/28/34 的覆写逻辑沉到 shared 一处,各页只覆写背景与 border-image。

---

### 问题 3 · 动效向"弹跳/循环"妥协(违反哲学第 6 戒 + hard constraint #9)

**证据**:
- `space-view.html` L228-237 `@keyframes card-appear`:`translateY(6px) → translateY(0)`。**这是位移 + 渐入,是弹跳式而非沉降式**。哲学第 6 戒明确说"慢、重、向下;禁止弹跳/overshoot/spinner/无限循环"。6px 的位移看起来小,但与下方 `session-card.sediment` 的"只褪色、不位移"语义相反。
- `animations.css` L19-28 `breathe` 6s infinite、L32-41 `auraBreath` 6s infinite、L45-48 `caret` 1.2s infinite、L57-60 `moodPulse` 8s infinite。**4 个关键帧全部 `infinite`**。hard constraint #9 明确说"禁止 `@keyframes ... infinite`(用 `animation-tick()` 驱动)"。
- `chat-expanded.html` L564 还有一个 `animation: caret 1.2s infinite` 直接写在行内,违反 hard constraint #9。
- `components.css` L498 `animation: auraBreath 6s ease-in-out infinite`、L313 `animation: breathe 6s ease-in-out infinite`、L595 `animation: breathe 6s ease-in-out infinite`、L179 `animation: breathe 6s ease-in-out infinite`、L120 `animation: breathe 6s ease-in-out infinite`——同一种违规 5 次。

**为什么是问题**:哲学第 6 戒列"无限循环"为禁项,是有理由的——持续循环的视觉信号会消耗用户的注意力,把"安静的房间"变成"始终在动的工作台"。更现实的问题:这套原型号称要映射到 Slint(注释里写了 `Slint: ...` ),而 Slint 不支持 CSS keyframe 动画,所有 `infinite` 动画在 Slint 端都跑不起来。这些 infinite 不是"设计选择",是"Slint 翻译时会被砍掉的功能"。

**修复方向**:`card-appear` 改纯 opacity(0→1,400ms,ease-out,无位移)。所有 `infinite` 动画改为 Slint 兼容的 `animation-tick()` 驱动(打开页面时跑 1 次呼吸, 1.2s 后停; caret 改为"输入时显示 0.8s 后停"; moodPulse 改为"焦点在名片时 8s 跑 1 次")。在 prototype 阶段就把这条约束钉死,避免到 Slint 端发现"呼吸感全没了"。

---

## 精简建议(5 条,动宾结构)

1. **砍掉 chat-expanded 操控台 7 控件至 2 控件**——保留输入框 + 发送键,其余(分段条/工作目录/模型/自治/ctx 圆环)全部沉入右抽屉"身外之物"。
2. **统一 avatar 尺寸为 40px(顶栏)/ 64px(在场)**——删除 space-view 32px、chat 44px、archive 40px 三处覆写,只保留两个 token(顶栏 vs 在场)。
3. **删除 card-appear 的 translateY**——改为纯 `opacity: 0 → 1`,350ms ease-out,符合"沉降"语义。
4. **将 4 个 infinite 关键帧改为 animation-tick 驱动**——breathe / auraBreath / caret / moodPulse 全部去 infinite,在 prototype 中跑一次性动画,避免 Slint 翻译时丢失。
5. **删除 theme-system 顶部 渐变强调条(`.accent-bar`)与"对话组件示例"区块**——这两个是范式真值页的装饰冗余,把 token 清账页改回纯 token 列表。渐变条违反"暖冷二态"的语义互斥。

---

## 跨原型对比

| 页面 | 乔布斯视角评分 | 失分点 | 关键做对的事 |
|---|---|---|---|
| `theme-system.html` 范式真值 | **8.5/10** | 顶部渐变条、对话组件示例冗余;token hex 全部展示过度 | 48 token 完整呈现;6 套代表色实测可视;air/halo 语义混合色分得清;代表色演示交互完整 |
| `archive.html` 档案馆 | **8.5/10** | 副标题 `font-style: italic`(L291)破坏暖灰系统;`已封存` tag 7 处重复 | 720px 单列约束;`data-depth` 0-10 沉积淡化阶梯;hover"注视回升"语义;冷雾 + 时间轴概念自洽 |
| `space-view.html` 空间主页 | **7/10** | 顶栏 5 控件(破戒 1);card-appear 弹跳(破戒 6);avatar 32px 与 64px 头像不一致 | active/sediment 三态(活跃/一般/沉积)语义清晰;非活跃 saturate 0.82 + 文字 opacity 0.5 沉降逻辑优雅;"新建房间"虚线暖底卡有房间感 |
| `chat-expanded.html` 对话展开 | **6/10** | 操控台 7 控件(破戒 1);80px 顶栏过重;infinite 动效 4 处(破硬约束 #9);抽屉滑入动画同 dashboard | 活跃轮 rep-500 左竖线 + 4% 面(哲学第 7 戒精确执行);思考块 abyss-400 冷左缘(哲学第 5 戒);用户气泡不染 rep(哲学第 7 戒);归档独立按钮 |

**对比结论**:范式真值与档案馆是这套原型的"两个高点"——前者证明 token 系统可计算、可可视;后者证明"沉积"概念可以被肉眼读懂。空间主页中等——概念对了但工具栏化。对话展开是最低点——乔布斯会问"为什么用户在咨询室看到状态栏?"。

---

## 基线对照(哲学十戒 + 8 条反 AI 味)

| # | 戒律 / 证据 | 状态 | 证据 |
|---|---|---|---|
| 1 | 拒绝 dashboard 美学 | **部分做到** | theme-system/archive 做到;chat-expanded L630-669 / space-view L483-513 工具栏化失分 |
| 2 | 品牌水印化 | **做到** | `components.css` L696-713 watermark opacity .25,左下角 |
| 3 | 代表色是 agent 的灵魂 | **做到** | theme-system L711-718 6 套代表色"由 agent 自主更换";`L673 人类除首次启动外不可改色` 注释明确 |
| 4 | 整屋空气染色 | **完美做到** | tokens.css L117-128 列出 12 个语义混合色;`air-rep` 3.5% / `halo-rep` 7% / `air-rep-speaking` 4.2% / `air-rep-settings` 1.5% 都精计算过 |
| 5 | 三要素语义域 | **做到** | rep(暖/驱力)/ abyss(冷/深渊)/ birth(灰/出生)三态互斥;chat-expanded L545 思考块用 abyss 而非 rep(精准) |
| 6 | 沉降式动效 | **部分做到** | archive 的 depth opacity 递减完美;但 space-view card-appear 用 translateY 失分;infinite 循环 4 处失分 |
| 7 | 用户与 agent 边界 | **做到** | chat-expanded L401-412 user-bubble 用 elevated + border 而非 rep;`.turn.active .msg` 只 font-weight 450,不染色 |
| 8 | 诗意克制 | **部分做到** | 整体克制;但 theme-system L336-341 `.accent-bar` 暖冷渐变条与 5 戒互斥;settings-general L129 卡 drop-shadow `.08` 偏重 |
| 9 | 暖灰基底 | **完美做到** | tokens.css L81 `--bg #F4F3F0`、L140 `--bg #181612` 暖灰非纯白纯黑;基底灰阶 8 档全暖色温 |
| 10 | 字体三系分离 | **完美做到** | tokens.css L49-51 Fraunces / Noto Sans SC / JetBrains Mono 三系;font-display 用 WONK+SOFT 60 区分品牌字 |

### 反 AI 味 8 条核对

| # | 证据 | 状态 | 具体 |
|---|---|---|---|
| 1 | 暖灰基底 + 珊瑚强调(非紫蓝渐变) | **做到** | tokens.css 全套色板,无任何 #6366F1 / #8B5CF6 / #3B82F6 紫蓝 |
| 2 | 沉降式(非弹跳式)动效 | **部分做到** | archive 沉积淡化做到;但 card-appear translateY(6px) 与 caret/breathe infinite 失分 |
| 3 | 头像有呼吸(非死板图标) | **做到** | components.css L491-500 `.avatar-wrap::after` auraBreath 6s,头像外圈暖光晕呼吸 |
| 4 | 留白有目的(非过度装饰) | **做到** | chat-collapsed .stream margin 0 34px / archive .archive-inner padding 0 24px 80px / theme-system .card padding 24px 24px 都精准 |
| 5 | 颜色弥漫空间(非纯色块) | **完美做到** | `air-rep-settings` 1.5% / `air-rep` 3.5% / `halo-rep` 7% / `presence-halo-rep` 20% / `turn-active` 4% 5 档浓度分级精确 |
| 6 | 字体三系(非单字体包打天下) | **做到** | 见上 |
| 7 | SVG 线性图标(非 emoji) | **做到** | 全套 stroke="currentColor" SVG;无任何 emoji |
| 8 | 角落水印(非满屏 logo) | **做到** | watermark opacity .25,14×14 SVG + 11px "northing" 文字 |

---

## 乔布斯视角综合分:**7.5 / 10**

**理由(直观,不套公式)**:

**+ 加分(做对的部分)**:
- 整屋空气染色哲学执行得近乎完美——5 档浓度梯度(1.5% / 3.5% / 7% / 11% / 20%)让"颜色弥漫"有了 token 化的精确表达。这是整套设计最值钱的部分。
- 暖灰基底 + 珊瑚代表色 + 深渊青异常态 + birth 出生锚——四色温系统有自己的内在逻辑,不靠紫蓝渐变混日子。
- 字体三系 + WONK/SOFT 变体让品牌字 Fraunces 有"咨询室铭牌"的温度,这是 AI 套壳绝对做不出的细节。
- 档案馆 `data-depth` 0-10 的沉积淡化是"反 AI 味"的最强证据——让"时间"成为视觉变量,而不是塞个时间戳字符串。
- 角落水印、SVG 线性图标、思考块用 abyss 而非 rep——这些细节都打在了哲学十戒的精准位置。

**- 减分(做错的部分)**:
- 操控台 7 控件 + 顶栏 5 控件:**-1**。这是最致命的——整套设计花了 90% 力气做"安静的房间",却在最高频的两个页把房间改成 IDE。
- avatar 三态 + 顶栏四高 + 把手三宽:**-0.5**。统一性失分,乔布斯会因为"agent 在三个页是三个尺寸"扣分。
- card-appear translateY(6px) + 4 处 infinite:**-0.5**。沉降式动效的"两个破洞",且 infinite 是 Slint 翻译时会丢的功能,不修等于自伤。
- theme-system `.accent-bar` 暖冷渐变条与 5 戒"语义互斥"冲突:**-0.25**。范式真值页自身有冗余。

**未达 9 分的硬原因**:哲学第 1 戒在两个最高频页失守——这意味着用户 60% 的使用时间面对的是"被改造成 IDE 的咨询室"。其余失分可修补,但这条要重做。

**潜在上探 8.5 的条件**:若砍掉操控台 5 控件 + 统一 avatar 尺寸 + 删 card-appear translateY,本套原型可达 8.5/10。范式真值与档案馆已经给出 9 分的范本,chat-collapsed/expanded 与 space-view 缺的是把"范本"用到最高频页。
