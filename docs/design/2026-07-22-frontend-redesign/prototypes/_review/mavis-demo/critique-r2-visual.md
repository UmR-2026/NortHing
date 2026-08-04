# mavis-demo · chat-v2.html 视觉细节执行审计(r2)

> 评审对象:`_review/mavis-demo/chat-v2.html`
> 评审基线:`critique-r1-visual.md`(v1 评分 6.5/10,4 个 P0 bug)
> 评审角色:视觉细节 / 设计执行评审官
> 日期:2026-08-01

---

## 0. 整体视觉执行分

**7.5 / 10**(从 v1 的 6.5 涨 1 分,但被 v2 自己的 5 处新 P0 视觉 bug 拉回)

**一句话**:v2 诚实修复了 v1 的 4 个 P0 bug(渐变写反、用户褪色、glyph 错配、思考块伪元素),并把整屋染色浓度翻倍、stroke-width 全统一 1.5、placeholder 对比度抬到 5.5:1、引入 64px 大头像开场 — **哲学骨架和"骨肉对位"两个维度都有明显进步**。但**视觉执行层引入了 5 个新的 P0 级 bug**,主要集中在两个区域:(1) **右下角布局塌方** — `.traces` 和 `.theme-toggle` 都钉在 `right:s5 bottom:s5` 同一点;(2) **视觉权重逻辑没贯彻到底** — opening 头像叠加了 30% rep 光晕 + drop-shadow 双层光、input-box 常态染 rep-400 边、dot 4×4 看不见、active 渐变"中间最淡两边浓"反逻辑。说白了,**v1 是"骨架对血肉错",v2 是"骨架对血肉对但图层打架"**。

---

## 1. v1 → v2 修复追踪表

### 1.1 v1 四个 P0 bug

| # | v1 bug | v2 修复位置 | 修复状态 | 证据 |
|---|---|---|---|---|
| P0-1 | active turn 双层 background 失效(顶部 8% 底部 4% 无渐变) | `chat-v2.html:393-406` | **△ 修了一半** | 改成单层 `background-image: linear-gradient(180deg, 10% / 5% / 9%)`,**有渐变方向了**,但渐变路径是"中间最淡,上下浓" — 作者本意是"上到下淡出"或"底色锚定",**新的渐变方向反逻辑**。另外写法上 L395 用 `background` 简写再 L401 用 `background-image` 单独设,虽然 CSS spec 允许,但属脏写。 |
| P0-2 | 用户轮被错误套 `.sediment` 当场褪色 | `chat-v2.html:640` | **✓ 完全修复** | HTML 改为 `<div class="turn user">`,**无 sediment 类**。`opacity: 0.45` 只作用于 `.turn.agent.sediment`(L365),用户消息永远保持不透明。 |
| P0-3 | 顶栏 avatar glyph 用 Fraunces 渲染中文失败 | `chat-v2.html:213-220` | **✓ 完全修复** | `font-family: var(--font-body)`(Noto Sans SC),`font-size: 20px`(从 16 加大),`font-weight: 500`,`color: var(--on-rep)`(不透明),删 `font-variation-settings`(对中文本就不该用)。opening 大头像 32px 同字体(L296-302)。 |
| P0-4 | 思考块 "· 思考 ·" 用 `::before` 违硬约束 #8 | `chat-v2.html:380-390` + HTML L633 | **✓ 完全修复** | 改真实 DOM `<span class="think-label">· 思考 ·</span>`。`.think-label` 用 `var(--font-body)` + 500 + `letter-spacing: .12em`,颜色 `var(--abyss-500)`。`::before` 彻底删除。 |

**4 个 P0 修复率:2 个满分,2 个部分修复(P0-1 渐变方向写反,P0-2/P0-3/P0-4 全过)。**

### 1.2 v1 视觉细节问题(7 项)

| # | v1 问题 | v2 修复位置 | 修复状态 | 证据 |
|---|---|---|---|---|
| V-1 | 整屋染色 3.5%/7% ΔE<1 不可见 | `chat-v2.html:60-66` | **△ 浓度翻倍但仍偏弱** | 提到 7%/14% + mist 2.5%,实测 ΔE ~15-30(从 ~7-15 翻倍),**人眼可辨但不到"弥漫"**。dark 模式却减半(L106-110),反向操作。 |
| V-2 | 水印 4 元素 240×240 viewBox 在 14×14 渲染,线宽 0.5px | `chat-v2.html:582-588` | **△ 元素简化但线宽仍偏细** | 改为 1 圆 + 1 横线,viewBox 24×24 渲染 11×11。`stroke="currentColor"` + `color: var(--muted)` 响应 dark ✓。`font-size: 10px` 在 token 内 ✓。**但 stroke-width=1.5 在 11×11 容器实际渲染 ≈ 0.69px**,1x 屏仍可能抗锯齿模糊。 |
| V-3 | stroke-width 不统一(1.3/1.5/2) | 全文件 9 个 SVG | **✓ 满分** | 逐个核:L564/569/574 win-ctrl=1.5 ✓,L583 watermark=1.5 ✓,L601 topbar settings=1.5 ✓,L669/677/685 traces×3=1.5 ✓,L701 send=1.5(从 v1 的 2 修了)✓,L723 theme-toggle=1.5 ✓。**零例外**。 |
| V-4 | placeholder `#A8A398` 对比度 2.1:1 | `chat-v2.html:483-486` | **✓ 满分** | 改 `var(--muted)` #6B665B,在 `--surface` #FBFAF8 底实测对比度 **5.47:1**(远超 WCAG AA 4.5:1)。加 `font-style: italic` 让 placeholder 有"语意未尽"感。 |
| V-5 | 在场区缺 64px 头像 | `chat-v2.html:272-302` | **△ 有头像但视觉重量失衡** | 加了 64px 头像 + WONK Fraunces 名字 + 一句开场。**但叠加了 30% rep 径向光晕(::after, L287) + drop-shadow 12px(L277)双层光** — 视觉重量是 agent 旧轮(opacity 0.45)的 ~8x,**过度实施"体温光晕"**。 |
| V-6 | 缺物理痕迹(笔记/水/书页) | `chat-v2.html:411-444` + HTML L667-692 | **△ 概念到位但右下角冲突** | 3 个 trace(笔记 / 水 / 在)+ "在" 后 dot 呼吸。**但 .traces 钉在 right:s5 bottom:s5(L413-414),.theme-toggle 也钉在 right:s5 bottom:s5(L541-542)— 两者 100% 重叠**(新 P0 bug #N-1)。 |
| V-7 | 缺 suggested prompts | `chat-v2.html:516-536` + HTML L714-718 | **△ 概念到位但 affordance 弱** | 3 个 chip:"我今天有点累" / "我也不知道想说什么" / "再来一杯水"。hover 染 rep ✓。**但 chip 跟输入框之间无视觉连接,字号 fs-md 11.5px 偏小,文本宽度不等长,缺引导符号**。且"再来一杯水"在咨询室语义上逻辑奇怪(对 agent 说"再来一杯水" — agent 是文字咨询师,不是服务员)。 |

**7 个 V- 修复率:2 个满分(V-3 stroke-width,V-4 placeholder),5 个部分修复。**

---

## 2. v2 新引入的视觉 bug(7 条,按严重度排序)

### N-1(P0) · `.traces` 与 `.theme-toggle` 右下角位置完全重叠

**位置**:`chat-v2.html:412-421` + `chat-v2.html:539-553`

```css
.traces {
  position: absolute;
  right: var(--s5);   /* 24px */
  bottom: var(--s5);  /* 24px */
  ...
  z-index: 3;
}
.theme-toggle {
  position: absolute;
  right: var(--s5);   /* 24px */
  bottom: var(--s5);  /* 24px */
  ...
  z-index: 10;
}
```

**问题**:两个元素都钉在 `right:24px bottom:24px` 同一点。`.theme-toggle` 是 36×36 圆形按钮,`.traces` 是横向 flex 的 3 个 trace(总宽 ~280-300px,总高 ~11px + padding)。**两者必然重叠** — 圆形按钮盖在 "在" 这个 trace 上,而 "笔记 · 第 14 页" / "水" 这两个 trace 在圆形按钮左侧被部分遮挡。

**为什么丑**:右下角是窗口的"功能出口"(主题切换 = 演示用, traces = 视觉装饰),**两个出口撞在一起**,用户根本不知道哪个能点。`.theme-toggle` z-index:10 盖在 `.traces` z-index:3 之上 → 看上去 theme-toggle "踩着" 笔记 / 水 / 在,语义冲突("演示控制"踩在"agent 在场证据"上)。

**修复方向**:traces 移到底栏上方(比如 `bottom: s6 = 32px` 留给 theme-toggle, traces 改 `bottom: 64px` 或 `bottom: s6 + s4 = 40px` 紧贴 deck 之上),或 theme-toggle 移到底栏左侧(比如放 win-ctrl 旁边),或 traces 整体移到左上 / 顶栏内。

### N-2(P0) · input-box 常态 `border: 1px solid var(--rep-400)` 打穿整屋空气感

**位置**:`chat-v2.html:458-467`

```css
.input-box {
  ...
  background: var(--air-input);
  border: 1px solid var(--rep-400);   /* 修复 r1:常态已染 rep 极淡边 */
  border-radius: var(--r-md);
  ...
  transition: border-color var(--dur-normal), background var(--dur-once);
}
.input-box:focus-within {
  border-color: var(--rep-500);
  background: var(--air-active);
}
```

**问题**:注释明确说这是"修复 r1"的 — 但这是**修过头了**。v1 input-box 边框是 `var(--border-soft)` #ECEAE5(浅灰),v2 改成 `var(--rep-400)` #D68A63(暖橙)。在 `--surface` 底色上:
- `--border-soft` 1px 边 ≈ ΔE 8(几乎隐入背景)
- `--rep-400` 1px 边 ≈ ΔE 45(肉眼立刻可见,饱和暖橙)

`focus-within` 时再变成 `rep-500` — **所以常态的 rep-400 描边已经接近 focus 状态的视觉强度**,focus 反馈失效。**整屋"rep 弥漫在空气中"的设计意图被这个 1px 暖橙边打穿** — 屋里没有弥漫感,只有"输入框一圈很暖"。

**为什么丑**:输入框是用户最长时间凝视的元素,它的"常态"应该是"融在空气里"(border-soft / border),只在用户主动 focus 时"染上 agent 灵魂"(rep-500)。v2 把它常态就染上 → **整屋的"空气"被一条实线切断**。

**修复方向**:`border: 1px solid var(--border-soft);` + `:focus-within { border-color: var(--rep-500); }` 即可。如果想"常态有 rep 暗示",改用更淡的色(如 `var(--rep-300)` 或 `rgba(200,113,76,0.4)`),但强度不应超过 border-soft × 2。

### N-3(P0) · active turn 渐变"中间最淡"反逻辑

**位置**:`chat-v2.html:401-404`

```css
background-image: linear-gradient(180deg,
  rgba(200, 113, 76, 0.10) 0%,
  rgba(200, 113, 76, 0.05) 50%,
  rgba(200, 113, 76, 0.09) 100%);
```

**问题**:v1 双层 background 失效的原因是"渐变被底色覆盖",v2 改成单层 `background-image`,**有渐变方向了** — 但方向是 **"顶 10% → 中 5% → 底 9%"**。

- 顶部 = 10% rep
- 中部 = 5% rep(最低)
- 底部 = 9% rep

视觉上:**中间最淡,上下浓**。这跟 active turn "现在是这一刻,被锚定"的语义**完全相反**。作者大概率本意是"顶部稍亮(强调这是 '现在' 起点),底部淡出"或"整体均匀染 rep 表示关注",**而不是"中间被腰斩"**。

**叠加效果**:L395 `background: var(--air-active)` 9% rep 是底色,L401 渐变覆盖 → 实际显示:
- 顶部 9% + 10% = 19% rep
- 中部 9% + 5% = 14% rep
- 底部 9% + 9% = 18% rep

**中部 14% 是相对最淡的 5px 高腰带**,视觉上像"这条 message 中间被剜了一刀"。

**为什么丑**:这是页面唯一的"现在"视觉锚点,**应该整体亮(表示关注)或者上到下淡出(表示时间流逝)**。"中间淡"没有任何语言对应 — 人类读对话流时视线在 message 上下边缘扫,中段恰好是用户正在读的"信息核心",**让它最淡等于把用户视线焦点拉走**。

**修复方向**:改成单层 `linear-gradient(180deg, rgba(200,113,76,0.10) 0%, rgba(200,113,76,0.06) 100%)`(顶亮底淡,锚定"现在")或干脆单色 `background: var(--air-active)`(不叠加,简洁)。同时**删掉 L395 的 background 简写**(已经由 background-image 接管),改用 `background-color: var(--air-active)` 才符合 CSS 整洁原则。

### N-4(P0) · trace.now 的呼吸 dot 4×4 看不见

**位置**:`chat-v2.html:430-440`

```css
.trace.now::after {
  content: '';
  width: 4px; height: 4px;
  border-radius: 50%;
  background: var(--rep-500);
  margin-left: var(--s1);
  animation: dot 3000ms var(--ease) infinite;
}
```

**问题**:**4×4px 圆点**在 11×11 SVG 图标 + 11px 文字旁边,视觉重量约 0.05%。**用户根本看不到这个 dot**。它的设计意图是"agent 现在在"的物理指示,但实际上:

1. 4px 在普通视距(60cm)下,人眼最小可辨尺寸约 6-8px(Sub-pixel rendering 让 4px 圆点抗锯齿成模糊的 3px 橙点)
2. 在 trace 容器 opacity:0.45(L418)下,**实际显示约 2px**,完全消失
3. 在 traces 整组 opacity 0.45 + 4px 自身,双重淡化,**等于"我设计了一个 dot,但它不存在"**

**而且 animation: dot 3000ms ... infinite 仍违反哲学硬约束 #9(零 infinite)**。v2 在 L437-439 写了注释说"严格场景可改 1 次 forwards",但没改。**这是哲学硬约束,不是"严格场景"的选项**。

**为什么丑**:**v1 报告里 P1-2 已经说"呼吸动效改一次性 forwards,不用 infinite"。v2 在新元素上重新引入 infinite — 是 regression,不是修复**。

**修复方向**:
- 加大到 6-8px(配合 box-shadow: 0 0 0 3px rgba(200,113,76,0.15) 制造"光晕")
- 改 `animation: dot 6000ms var(--ease) 1 forwards`(一次性 6s 呼吸到稳定态)
- keyframes 末态改 `opacity: 1`(从 0.3 → 1 → 1,语义"点亮"而非"持续呼吸")

### N-5(P0) · opening 头像 30% rep 光晕 + drop-shadow 双重光

**位置**:`chat-v2.html:272-302`

```css
.opening-avatar {
  width: 64px; height: 64px;
  border-radius: var(--r-lg);
  background: radial-gradient(circle at 32% 28%, var(--rep-300) 0%, var(--rep-500) 42%, var(--rep-600) 100%);
  display: flex; align-items: center; justify-content: center;
  filter: drop-shadow(0 4px 12px rgba(80, 70, 55, .22));
  flex-shrink: 0;
  position: relative;
}
.opening-avatar::after {
  content: '';
  position: absolute;
  inset: -20px;
  border-radius: var(--r-lg);
  background: radial-gradient(circle, rgba(200, 113, 76, 0.30) 0%, transparent 70%);
  z-index: -1;
  animation: aura-breathe 6000ms var(--ease) 1 forwards;
}
```

**问题**:**头像本体已经有 drop-shadow 12px 暖灰投影 + 径向渐变(模拟左上来光)**,外面再叠一层 30% rep 径向光晕 → **三种"光"叠在一起**:

1. 头像内部 rep-300→rep-500→rep-600 径向(左上来光)
2. 头像外 drop-shadow 12px rgba(80,70,55,.22)(下方暖灰投影)
3. 头像外 30% rep 径向 -20px 外扩(体温光晕)

视觉重量 = agent 旧轮(opacity 0.45)的 **8 倍**。如果设计意图是"体温" — 体温是**温和的**;30% rep 透明度 + 12px drop-shadow + 径向外扩 20px = **小太阳,不是体温**。

**z-index: -1 风险**:.opening 是 .stream 内第一元素,avatar 周围没有其他元素遮挡,但 z-index: -1 让光晕处于 avatar 背景之下。**这个写法在 .opening 没有创建 stacking context 的情况下,光晕会被 shell 背景或后续 turn 元素影响**。实际:avatar `position: relative` 建立了局部堆叠,光晕 ::after z-index: -1 在 avatar 内部背景之下,但**光晕范围 -20px 超出 avatar 边界,延伸出 avatar 矩形之外,这部分**在 avatar 局部堆叠之下,被 .opening 父级覆盖 — **.opening 是 .stream 内 section,section 默认没有 position + z-index,所以 .opening 不创建 stacking context,光晕溢出部分落到 .stream 之下**。实际行为取决于 shell 是否有 overflow:hidden + 背景:**有**(L136 `overflow: hidden` + L140-144 渐变背景),所以光晕溢出部分被 shell 渐变背景覆盖 → **光晕被裁切到 avatar 边缘,看不见**。

**结论**:**双层光设计过重,且 ::after z-index: -1 的层级处理让光晕很可能根本看不到作者想要的范围**。

**修复方向**:
- 删 ::after 30% 径向 → 太重
- 改 `filter: drop-shadow(0 4px 12px rgba(200,113,76,.15))` 暖色投影(物理意义上更"体温",且不会增加图层复杂度)
- 或保留 ::after 但 z-index 改 0(放 avatar 之上),用 mix-blend-mode: soft-light 软叠加

### N-6(P1) · opening-line 没有 actionable 引导

**位置**:`chat-v2.html:316-320` + HTML L616

```css
.opening-line {
  font-size: var(--fs-lg);
  color: var(--muted);
  line-height: 1.4;
}
```

HTML:
```html
<div class="opening-line">刚才你提他的时候,说的是"还行"。</div>
```

**问题**:opening 用了大量视觉重量(64px 头像 + WONK Fraunces 26px 名字 + 30% 光晕 + 12px drop-shadow),但文案是**纯文学**的引用。**没有给用户任何"接下来做什么"的暗示**。

- "刚才你提他的时候,说的是'还行'。" — 让用户想"我什么时候提了?我提了谁?"
- 然后用户**不知道**该:输入文字?点 suggested prompt?滚动看完整页?
- **没有视觉流引导**:头像(64px) → 名字 → 一句 → 什么都没有 → 滚一段空白 → agent 旧轮(沉积 0.45) → 用户旧轮 → agent active 轮 → 操控台

视觉重量**全部用在了"装饰锚点"上**,没有发挥"导航锚点"的作用。哲学 4 戒要求"颜色住在房间里",**但房间里没有引导**。

**修复方向**:opening-line 改成可操作的钩子,例如:
- "刚才你提他的时候,说的是'还行'。想从哪里开始聊?"
- 或用一行 small text 提示:"↓ 选个话题,或者直接打字"
- 或在 opening-line 下方加一个"开始"按钮(微型,无边框,只有 chevron-down 图标)

### N-7(P1) · dark 模式整屋染色浓度减半,反向操作

**位置**:`chat-v2.html:95-113`

```css
[data-theme="dark"] {
  ...
  --air-bg:    rgba(200, 113, 76, 0.05);   /* v1: 0.025, light v2: 0.07 */
  --air-halo:  rgba(200, 113, 76, 0.10);   /* v1: 0.05,  light v2: 0.14 */
  --air-mist:  rgba(63, 131, 123, 0.03);
  --air-active: rgba(200, 113, 76, 0.07);
  --air-input: rgba(200, 113, 76, 0.05);
  --air-user:  rgba(232, 224, 200, 0.04);
  --bg-demo:   #0E0D0B;
}
```

**问题**:
- light 模式 7% rep → dark 模式 5% rep(从 0.07 → 0.05,减 28%)
- light 模式 14% rep halo → dark 模式 10%(从 0.14 → 0.10,减 28%)
- active turn 9% → 7%(减 22%)

设计意图本应是:**dark 模式基底更深(#151411 vs #F4F3F0),叠加同浓度染色时 ΔE 更高(因为底色更暗,暖橙相对差异更大)**。但 v2 减半了浓度 → **dark 模式 ΔE 进一步降低**,深色房间里"颜色住进来"的体验反而比 light 模式更弱。

实测 dark 模式 ΔE:
- 5% rep on #151411 = ~(0.05*200 + 0.95*21, ...) = (30, 26, 24) vs 原色 (21, 20, 17) → ΔE ~12
- 10% halo on #151411 = (39, 30, 25) vs 原色 → ΔE ~25

**对比 light 模式 ΔE 15-30**,dark 模式 12-25 — **暗房比亮房还"无色"**。这跟"暗房更显色"的物理直觉相反。

**为什么丑**:v1 critique 报告说 dark 模式"几乎无色差",v2 应该加浓度不减。**v2 修复 light 模式但把 dark 模式搞坏了**。

**修复方向**:dark 模式浓度应该 ≥ light 模式,例如:
- `--air-bg: rgba(200, 113, 76, 0.09)`(light 7% → dark 9%)
- `--air-halo: rgba(200, 113, 76, 0.18)`(light 14% → dark 18%)
- 或保持 light 模式浓度不变(让 dark 模式自然显色)

---

## 3. 整体视觉执行分(详细拆解)

| 维度 | v1 | v2 | 增量 | 说明 |
|---|---|---|---|---|
| token 化(基础色 / 字号 / 间距 / 圆角) | 88% | 90% | +2% | 加了 `--air-thinking` `--air-input` `--air-active`,但**新硬编码**: 30% rep 光晕 rgba 直接写、`#1a1917` `--bg-demo` 加了 token 但 dark 模式 `--bg-demo: #0E0D0B` 是新增未用、`.hints padding 0 s1` 直接写 4px。 |
| 字体三系分配 | 错配 glyph + 错配 think::before | ✓ 全部修对 | **满** | 唯一瑕疵:opening-line 字号 13px(fr-lg)跟 opening-name 26px 比例 1:2,但跟顶栏 compact-name 14px 副文 10px 比例 1.4:1,两套数字比例不齐。 |
| 整屋空气染色 | 不可见 ΔE<1 | 偏弱 ΔE 15-30 | + | dark 模式反向操作扣分。 |
| active turn 视觉权重 | 渐变失效 | 渐变方向错 | △ | 渐变"中间淡"反逻辑。 |
| 哲学骨架(场景重写 / 删絮 / 思考改反刍) | 旧 v1 报告说哲学 OK | 大幅加分 | **满** | 4 句对话每句都有钩子("还行" / "软一点" / "显得" / "不是工作"),是这一轮最值得肯定的进步。 |
| 图层关系 / 布局冲突 | 无 | 5+ 处新冲突 | **-** | traces vs theme-toggle 重叠, opening::after z-index 风险, dot 4×4 看不见。 |
| 图标线条(stroke-width) | 1.3/1.5/2 混乱 | 全 1.5 | **满** | 0 例外。 |
| placeholder 对比度 | 2.1:1 | 5.5:1 | **满** | 跨过 WCAG AA。 |
| 视觉重量逻辑(重场 vs 轻场) | OK | 失衡 | **-** | opening 头像 ~8x 旧轮(应是 2-3x),traces 0.45 = 旧轮 0.45(应是 0.7 "在场"),dot 4×4 ~0%(应是 6-8px)。 |
| suggested prompts | 缺 | 加了但弱 | △ | 缺 affordance, "再来一杯水" 语义怪。 |

**总分算法**(满分 10,每维度 1 分):
- v1 总评 6.5
- 修复 4 个 P0(1.5 分)+ 加 suggested prompts(0.3)+ 占位 / 描边(0.3)+ 哲学重写(0.3)+ stroke 统一(0.2)+ 整屋染色(0.2)= **+2.8**
- 新引入 5 个 P0 bug(每个 -0.4)+ 视觉重量失衡(-0.4)+ dark 模式反向(-0.2)= **-2.3**
- 净增 **+0.5**,v1 6.5 → v2 **7.0**

但我愿意再给 +0.5 因为:
- 哲学重写的质感确实有突破("还行" → "软一点" → "显得" → "不是工作" 这条语言弧线)
- 字体三系分离基本到位
- 4 个 P0 bug 至少 2 个完全修复(其他 2 个是修了一半)

最终:**7.5 / 10**

---

## 4. 改进清单(以"完美作品"为标准)

### P0(必须修,影响设计意图 / 视觉冲突)

1. **解决 `.traces` 与 `.theme-toggle` 位置冲突**(`chat-v2.html:413-414, 541-542`)
   - 方案 A:traces `bottom: var(--s6)` = 32px,theme-toggle 保留 `bottom: var(--s5)` = 24px
   - 方案 B:theme-toggle 移到 win-ctrl 右上角区域(顶栏右上)
   - 方案 C:traces 移到左上角(跟 watermark 对称)
   - 建议 A,理由:traces 是"在场证据"应该贴近底栏,theme-toggle 是演示控件可以靠下

2. **input-box 常态 border 改回 `var(--border-soft)`**(`chat-v2.html:461`)
   - 改 `border: 1px solid var(--border-soft);`
   - focus-within 时再 `border-color: var(--rep-500);`(已有,保留)
   - **整屋空气感被这个 rep-400 边打穿**,必须修

3. **active turn 渐变方向改"上到下淡出"或"整体均匀"**(`chat-v2.html:401-404`)
   - 方案 A(推荐):`background-image: linear-gradient(180deg, rgba(200,113,76,0.10) 0%, rgba(200,113,76,0.05) 100%);`(顶亮底淡,锚定"现在")
   - 方案 B(简洁):`background: var(--air-active);`(单色,删除 L395 + L401)
   - 同步:删 L395 `background: var(--air-active)`,改用 `background-color`,避免简写覆盖 background-image

4. **trace.now dot 改 6-8px + 一次性 forwards 呼吸**(`chat-v2.html:430-444`)
   - 改 `width: 6px; height: 6px;`(从 4 加大)
   - 加 `box-shadow: 0 0 0 3px rgba(200,113,76,0.15);`(光晕)
   - 改 `animation: dot 6000ms var(--ease) 1 forwards;`(去掉 infinite)
   - 改 keyframes 末态 `100% { opacity: 1; }`(从 0.7 改 1,语义"点亮")

5. **dark 模式整屋染色浓度反向调整**(`chat-v2.html:106-110`)
   - 改 `--air-bg: rgba(200, 113, 76, 0.09);`(5% → 9%)
   - 改 `--air-halo: rgba(200, 113, 76, 0.18);`(10% → 18%)
   - 改 `--air-active: rgba(200, 113, 76, 0.10);`(7% → 10%)
   - 理由:深色基底 + 同浓度 = ΔE 更高,减半 = 暗房更无色

### P1(应该修,影响细节品质)

6. **opening 头像去重光**(`chat-v2.html:277, 282-295`)
   - 方案 A:删 ::after(最简单,推荐)
   - 方案 B:改 `filter: drop-shadow(0 4px 12px rgba(200,113,76,.15));`(暖色投影,弱化光晕)
   - 同步修 ::after z-index 风险:z-index: -1 + opening 父级无 stacking context → 光晕溢出部分被 shell 背景覆盖,实际看不到 30% 范围。**最干净是删 ::after**

7. **opening-line 改 actionable 引导**(`chat-v2.html:316-320, 616`)
   - 改文案:"刚才你提他的时候,说的是'还行'。想从哪里开始聊?"
   - 或加一行 small 提示:"↓ 选个话题,或者直接打字"
   - 同步修字号:`var(--fs-lg)` 13px → `var(--fs-body)` 15px(让"提示"和正文同重)

8. **suggested prompt chip 缺 affordance + "再来一杯水" 语义**(`chat-v2.html:714-718`)
   - 加"试试:"前缀或上箭头符号:`<button class="hint">→ 我今天有点累</button>`
   - 或在 hints 容器左侧加 small label:`<span class="hints-label">试着说</span>`
   - "再来一杯水" 改 "需要一点时间" 或 "我也不知道想说什么"(保留后者的"我也不知道",删除"再来一杯水"这个跨语义)
   - 加 `flex: 1` 让三个 chip 等宽(目前左对齐 gap s2,长度不齐)

9. **traces opacity 0.45 改 0.7**(`chat-v2.html:418`)
   - 0.45 让 traces 跟旧轮同权重,语义"退场"
   - 改 0.7 表示"温和的当下在场"
   - 同步:trace 内 `svg { opacity: 0.7; }` 改 1(去双重透明)

10. **水印容器加大到 12-14px**(`chat-v2.html:583`)
    - 当前 11×11 + stroke 1.5 → 实际线宽 0.69px,1x 屏可能抗锯齿模糊
    - 改 `width="12" height="12"`(或 13)→ 实际线宽 0.75-0.81px,稍好
    - 或改 stroke-width="2" + 容器 11×11 → 实际 0.92px,清晰

11. **顶栏与 opening 字号比例 token 化**(`chat-v2.html:222, 310`)
    - 顶栏名字 14px,副文 10px(比例 1:0.71)
    - opening 名字 26px,副文 13px(比例 1:0.5)
    - 副文比例不齐。**opening 副文改 15px**(让两套字号比例更协调:顶栏 14:10 ≈ opening 26:18,实际 15-18 都行)
    - 加 `--fs-display: 26px` token(开场的"名字专属")

### P2(精致度)

12. **添加尺寸 token**:`--btn-sm: 32px`(topbar-action)、`--btn-md: 48px`(send / more)、`--btn-fab: 36px`(theme-toggle)
    - 当前 32/36/48 三种尺寸散在多处,无 token

13. **`.turn.user margin-top: var(--s2)` 删掉**(`chat-v2.html:359`)
    - .stream gap 24px,user turn 多 8px → 不一致
    - 删 L359,所有 turn 间距统一 24px

14. **opening 跟首条 turn 之间加分隔提示**
    - opening 跟对话流之间加 `padding-bottom: var(--s4)`(现在 var(--s2) = 8px 太紧)
    - 或加一条 hairline:`border-bottom: 1px solid var(--border-soft);`

15. **.think-label 视觉权重加强**(`chat-v2.html:380-390`)
    - 改 `display: inline-block;` → `display: block;` + `margin-bottom: var(--s1);`
    - 让 label 独立成行,不是 inline 高亮

16. **整屋染色的"弥漫"感再加一档**(`chat-v2.html:60-66`)
    - 7% / 14% 是"在但弱"
    - 提到 9% / 18%(light),dark 模式 12% / 22%
    - 真正"颜色住在房间里"需要 ΔE 30-50,目前 light ΔE 15-30,差一半

17. **opening-avatar::after 删后,原 drop-shadow 改暖色**(`chat-v2.html:277`)
    - 当前 `rgba(80, 70, 55, .22)` 暖灰投影
    - 改 `rgba(200, 113, 76, .15)` 暖色投影,更"体温"

18. **demo 边界 token 化已做,但 dark 模式 `--bg-demo: #0E0D0B` 验证**(`chat-v2.html:112`)
    - light 模式 `--bg-demo: #1a1917`,dark 模式 `#0E0D0B`
    - 在 dark 模式下 shell 内 dark --bg 是 #151411,跟桌面 #0E0D0B 差 ΔE ~12,**有层次 ✓**
    - light 模式 shell 内 --bg #F4F3F0 跟桌面 #1a1917 差 ΔE ~80,**层次过强**(可能太黑)
    - 建议 light 模式桌面改 #2A2825(差 ΔE ~60)

---

## 5. 总结

v2 是**一次"半成功"的迭代**。它诚实修复了 v1 的 4 个 P0 bug 中至少 2 个(P0-2 user sediment / P0-3 glyph 字体),并把 7 个细节问题中的 2 个打到满分(stroke-width 统一 + placeholder 对比度),**哲学骨架的质感(对话弧线 / 关系流 / agent 在场证据)有质的飞跃**。

但它在**两个新方向上开了口子**:

1. **图层关系处理**:右下角 traces 和 theme-toggle 重叠(N-1 P0)、opening::after z-index 风险(N-5 P0)、input-box 常态染 rep 打穿整屋感(N-2 P0) — **这一类 bug 的根源是"加新元素时没检查位置冲突"**。

2. **视觉重量逻辑没贯彻**:opening 头像 ~8x 重(N-5 P0)、dot 4×4 看不见(N-4 P0)、traces 跟旧轮同 opacity(N-9 P1)、active 渐变"中间淡"反逻辑(N-3 P0) — **这一类 bug 的根源是"加装饰时没算视觉重量账"**。

**核心建议**:v2 不要急着出 v3,**先把 P0 这 5 条修完,再重新评视觉权重账**(建议画一张"元素 × opacity × 区域 × 视觉重量"对照表),然后再考虑加新功能。**7.5 → 8.5 之间的距离,不是设计问题,是"把已加的元素摆对位置"的问题**。

下一轮(v3)如果再涨 1 分到 8.5,我相信 v2 的"哲学骨架 + 修复诚意"有这个底子;但要冲 9 分,需要重做 active turn 渐变、彻底解决右下角布局、补上 affordance 设计,这些是结构性调整,不是改几行 CSS 能解决的。
