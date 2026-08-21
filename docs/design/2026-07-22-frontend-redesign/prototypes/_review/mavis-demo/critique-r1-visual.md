# mavis-demo · chat.html 视觉细节执行审计(r1)

> 评审对象:`_review/mavis-demo/chat.html`(20,475 字节,633 行)
> 评审基线:`_review/design-philosophy-distilled.md`(v2,10 戒 + 10 硬约束)
> 评审角色:视觉细节 / 设计执行评审官
> 日期:2026-08-01

---

## 0. 整体视觉执行分

**6.5 / 10**(需要重做一轮视觉细节)

**一句话**:token 化做得非常彻底(95%+ 数值走 var),字体三系分离基本到位,但**至少 4 处 P0 bug**让"看起来用心"的页变成"半成品"——尤其 active turn 的双层 background 写反了导致颜色完全失效,以及把刚发的用户消息套上 .sediment 让它当场褪色。哲学落地的"骨架"已对,执行层的"骨肉"有几处明显塌陷。

---

## 1. 审计清单(16 项)

### 1.1 整屋空气染色的实现 — △

**代码**:`chat.html:126-130`
```css
background:
  radial-gradient(ellipse 80% 50% at 50% 0%, var(--air-halo) 0%, transparent 70%),
  radial-gradient(ellipse 100% 40% at 50% 100%, var(--air-mist) 0%, transparent 80%),
  var(--air-bg),
  var(--bg);
```

**token**(L51-56):`--air-bg` 3.5% rep / `--air-halo` 7% rep / `--air-mist` 1.5% abyss

**评价**:
- ✓ 4 层叠加结构正确(三明治:顶晕 + 底冷雾 + 底 rep + 基底),色相正确(rep 200,113,76 + abyss 63,131,123)
- ✓ token 化彻底,无硬编码
- ✗ **浓度在视觉上几乎不存在**:3.5% 暖橙叠加 #F4F3F0 → 实际色差约 0.7% sRGB,人眼基本看不出"弥漫"感。哲学第 4 戒要求"颜色住在房间里",但当前实现下颜色是"几乎不在房间里"
- ✗ 顶晕 ellipse 80% × 50% at 50% 0%,从顶部延伸到 50% 高度,7% 透明度在 1280×800 视口下 → 顶部 1/3 区域每像素 rep 加权 ~3.5%,效果是"屏幕顶部微微发暖",而非"房间里有一束光"
- ✗ `--air-mist` 1.5% 透明度 → 数学上等于不可见,删除也不影响

**修复方向**:3.5% 提到 5-6%,7% 顶晕提到 10%,1.5% 冷雾删掉或改 2.5%。

---

### 1.2 居中在场区 — △

**代码**:`chat.html:249-291`

**评价**:
- ✗ **没有 64px 头像在场区**(只顶栏 40px 头像),哲学第 2 戒说"视觉主体是 agent 头像+名字",但在场区只有 22px Fraunces 文字 + 6px 细编年史条。进入页面时,首屏视觉锚点缺失
- ✗ 编年史条 right 端 `nd` 圆点(L283-291):直径 9px + 1.5px border = 12px,但编年史条本身只 6px 高 — 圆点上下各溢出 3px,加 1.5px 白色 border 在 --surface 背景上形成"挂着小气球"效果,而不是"现在时刻嵌在条上"
- ✓ Fraunces WONK 1 + SOFT 60 22px presence-name 设计意图清晰

**修复方向**:
1. 在场区加 64px 头像(和顶栏 40px 形成两档 hierarchy)
2. nd 圆点去掉 border,改成 6px 实心点紧贴条右端(right: 0),或改为竖线刻度(2px × 8px)更克制

---

### 1.3 顶栏 avatar (40px) + glyph — △

**代码**:`chat.html:191-204`
```css
.compact-avatar { width: 40px; height: 40px; border-radius: var(--r-md); ... }
.compact-avatar .glyph { font-family: var(--font-display); font-size: 16px; ... font-weight: 600; }
```

**评价**:
- ✗ **字体错配**:`font-family: var(--font-display)` 即 Fraunces 显示中文字符"序"。Fraunces 是拉丁衬线体,不支持 CJK — 浏览器会回落到系统字体(Windows 回落 Microsoft YaHei / PingFang SC),导致 WONK 1、font-weight 600 在中文上**完全无效**,视觉上"序"字就是普通系统字体 16px 600。设计意图(用品牌字重)丢失
- ✗ 16px 在 40px 头像里**偏小**:40px 头像里汉字最佳视觉大小约 20-22px(占比 50-55%),16px 占比 40% 显得"字浮在角落"
- ✗ 颜色 `rgba(255, 250, 245, .88)` 是 `--on-rep` 的半透明 — 但 avatar 背景是 rep-300 → rep-500 → rep-600 径向渐变,在 rep-300 浅色区域,88% 不透明乳白字对比度仅 ~2.5:1,**字几乎看不清**(尤其左上和左上 1/4 区域)
- ✗ `font-variation-settings: "WONK" 1` 对中文字符无效
- ✓ 圆角 14px(r-md)+ drop-shadow 是对的
- ✓ 渐变 `radial-gradient(circle at 32% 28%, ...)` 模拟左上来光,细节到位

**修复方向**:
1. glyph 改用 `var(--font-body)`(Noto Sans SC),font-size 20px,font-weight 500
2. 颜色改纯 `var(--on-rep)`(#FFFAF5),不透明
3. 渐变光源位置 32% 28% 可保留,但在浅色区域保证 glyph 对比度

---

### 1.4 活动轮 background — ✗(P0 bug)

**代码**:`chat.html:386-394`
```css
.turn.agent.active {
  background:
    linear-gradient(180deg, var(--air-active) 0%, transparent 100%),
    var(--air-active);
  ...
}
```

**问题**:双层 background 完全失效。
- 第一层:`linear-gradient(180deg, var(--air-active) → transparent)` — 顶部 4% rep → 底部 0% rep
- 第二层:`var(--air-active)` 纯色 — 整体 4% rep
- 叠加:底部 = 0% + 4% = 4%;顶部 = 4% + 4% = 8%
- **实际效果**:均匀的 8% rep,**没有任何渐变**

作者意图可能是"顶部稍亮,底部淡出"(强调这是"现在")但写法完全没达到。

**修复方向**:删掉第二层 `var(--air-active)`,只留单层 linear-gradient,或者用更精细的渐变(如 `linear-gradient(180deg, rgba(200,113,76,0.06) 0%, rgba(200,113,76,0.02) 100%)` 在 4-6% 之间过渡)。

---

### 1.5 思考块 — △

**代码**:`chat.html:346-365`

**评价**:
- ✓ 背景 `--air-thinking` 6% abyss 青,哲学硬约束 #2 思考块不染 rep ✓
- ✓ 左边 2px abyss-400 线
- ✗ **违反硬约束 #8**(L358-365):`::before` 伪元素当主元素("· 思考 ·"标签)
  ```css
  .think::before { content: "· 思考 ·"; ... }
  ```
  哲学硬约束 #8 明确:"禁止 `::before` / `::after` 伪元素当主元素(Slint 翻译限制)"
- ✗ 视觉重量:在 active turn 内,正文 `font-weight: 450`(L399),思考块 11.5px italic 400 — 差别小,active 状态下思考块容易被误读为"普通段落"
- ✗ 左边 2px 线 vs active turn 左边 2.5px rep-500 线 — 思考块在 active turn 内,2px 比 2.5px 细,**视觉上被外层压住**
- ✗ 思考块 `max-width: 480px` 跟 turn.max-width 760px 不对齐,转行点在视觉上不对齐(思考块 480 处换行,正文 760 处换行)

**修复方向**:
1. 改用真实 DOM 元素 `<span class="think-label">· 思考 ·</span>` 替代 ::before
2. font-weight 改 500,加强对比
3. 左边线改 2.5px 或保留 2px 但加 box-shadow inset 1px 0 0 模拟
4. max-width 改 520px 与正文比例对齐

---

### 1.6 工具 chip — △

**代码**:`chat.html:373-383`
```css
.chip { padding: var(--s1) var(--s3); border-radius: var(--r-pill); ... }
.chip .ck { color: var(--abyss-500); }
```

**评价**:
- ✓ padding s1 s3(4 12) 紧凑合理
- ✓ pill 圆角 999
- ✓ mono 字体 + abyss 高亮符合"工具 = 系统"语义
- ✗ 边框 `1px solid var(--border-soft)` #ECEAE5 在 1.5% 透明基底上几乎不可见,加边框反而让 chip 边缘有"虚化"毛刺感
- ✗ 间距 `gap: var(--s1)` 4px,ck 文字和后面内容之间 4px,跟文字字号 10px 几乎等距 → "·"看起来离两边都一样远,失去分组语义
- ✗ chip 内 "read · 项目日志 · 312行 ✓" 三段用"·"分隔,但 ck 高亮只高亮 "read",其他保持 muted — 一致性 OK,但 ".✓" 是 unicode 而非 SVG,**违反哲学 P2-4(所有图标 SVG,无 ▍ 字符残留)**
- ✗ 10px 在 1px 边框下,文字几乎贴边,padding 4 12 偏紧(中文/数字接近边框)

**修复方向**:
1. 边框改 0.5px solid 或干脆删除(底色已够区分)
2. ck 和正文之间用 `&nbsp;` 或更大 gap(var(--s2))
3. ✓ 改成 SVG checkmark `<svg ...><path d="M3 8 l3 3 l6 -6" .../></svg>`
4. padding 改 s1 s2.5(4 10)或加 line-height 1.6

---

### 1.7 用户气泡 — ✗(P0 bug)

**代码**:`chat.html:330-340`(CSS)+ `chat.html:575-583`(HTML)
```html
<div class="turn user sediment">
```

**问题**:
- ✓ 背景 `var(--elevated)` #FFFFFF + `var(--border)` #E6E3DD,哲学第 7 戒用户气泡不染 rep ✓
- ✗ **P0 逻辑错误**:`.turn.user` 加了 `.sediment` 类,触发 `opacity: 0.55`(L343)。但这是 14:08 用户消息,紧接着 14:08 · 现在 的 agent 活跃轮 — **这是"刚发的最新消息",不是沉积的旧消息**。给它套 .sediment 等于"刚说完就褪色",直接打破"沉积 = 旧"的语义
- ✗ 用户气泡用 #FFFFFF 纯白:哲学 L33 token 里 elevated=#FFF 是设计选择,但基底 #F4F3F0 + 白方块 → 在暖灰房间里有"塑料贴片"感,违反第 8 戒"无通用白卡片"的隐性精神
- ✗ `align-self: flex-end` 让气泡右对齐,但 turn-name "你" 右对齐(L339)— 时间 + 名字反过来排列,头部不自然
- ✗ 气泡 max-width 540 vs agent 760,在长文本上宽度差视觉上跳跃

**修复方向**:
1. 删掉 `sediment` 类,用户轮永远保持不透明
2. 用户气泡底色改 `var(--surface)` #FBFAF8(更暖)或 `var(--raised)` #EFEDE8,边框用 border-soft,削弱白方块感
3. 头部顺序调成"名字 + 时间"一致(像 agent 一样)

---

### 1.8 圆角 token 应用 — ✓

**检查**:全文硬编码 `border-radius` 只有:
- L288 `.nd` `50%`(圆形,合理)
- L473 `.theme-toggle` `50%`(圆形,合理)

其余全部走 `var(--r-window)` / `var(--r-md)` / `var(--r-sm)` / `var(--r-pill)` / `var(--r-win-btn)`,无偏差。

**评价**:**满分**。圆角 token 化 100%。

---

### 1.9 间距 4 基数 — △

**违规清单**:
- L112 `body { padding: 40px }` — **40 不在 s-token 里**(s5=24, s6=32)
- L213 `.compact-meta { margin-top: 2px }` — 2px 不在 4 基数(可作为 hairline 接受,但严格说不该)
- L113 body padding 40 是 demo 边界(假桌面)处理,可加 token `--s-demo: 40px` 命名

**评价**:
- ✓ 主体内 95%+ 间距用 var(--s*)
- ✗ 40 / 2 两处硬编码 — 40 应单独 token,2 可接受

---

### 1.10 字体三系 — △

**三系分配**:
- Fraunces:presence-name(L260)、compact-avatar.glyph(L199,违规)、watermark .bt(L172)
- Noto Sans SC:正文 / UI 默认
- JetBrains Mono:turn-time(L319)、chip(L374)、think::before(L360,违规)

**问题**:
- ✗ **L199 compact-avatar.glyph 字体错配**:中文"序"用 Fraunces 完全无效(详见 1.3)
- ✗ L360 .think::before 用 mono 显示"· 思考 ·",但中文字符在 mono 字体下不自然(中文字符宽度相等但字形不变 → 视觉上"·思考·"三个字撑开得很松散,不像 label 像"点点 思考 点")

**修复方向**:
1. glyph 改 Noto Sans SC 20px 500
2. think 标签改 Noto Sans SC 500,letter-spacing .05em

---

### 1.11 动效写法 — ✓

**@keyframes 清单**:
- L134 `room-arrive`:opacity 0→1 ✓
- L223 `breathe`:scale 1→1.18→1,opacity 0.65→1→0.85(用于 .sdot,合理)
- L254 `settle`:opacity 0→1 ✓
- L395 `turn-active-in`:opacity 0→1 ✓

**animation 清单**:
- L132 `room-arrive 1200ms ... both` ✓
- L221 `breathe 6000ms ... 1 forwards` ✓(`1` 是 iteration-count,1 次非无限)
- L252 `settle 1200ms ... 200ms both` ✓
- L393 `turn-active-in 1200ms ... both` ✓

**评价**:
- ✓ **零 `infinite` 残留**(硬约束 #9)
- ✓ **零 `translateY` 残留**(P1-3 修正已落实)
- ✓ 命名规范(动宾或语义:room-arrive / breathe / settle / turn-active-in)
- △ `breathe` 末态 scale 1 + opacity 0.85,但起始 opacity 0.65 → 末态 0.85,**末态高于初态但低于峰值**,逻辑"点亮后保持半高" — 可接受但有点奇怪

**唯一微瑕**:breathe 用 transform: scale 而非 opacity-only,但呼吸语义需要"形变"所以允许。

---

### 1.12 SVG 图标 — △(线条粗细不一致)

**逐个检查**:
- L492 minimize:line,stroke-width 1.5,stroke-linecap round ✓
- L495 maximize:rect,stroke-width **1.3** ← 异常
- L498 close:line × 2,stroke-width 1.5,stroke-linecap round ✓
- L504-509 watermark:circle stroke-width 6,path 8,path 10,line 9 ← 4 个不同粗细
- L527-530 settings:1.5 ✓
- L609-612 send:line + polyline,stroke-width **2** ← 异常
- L615-619 more:fill,三个圆 r=1.5 ✓
- L625-627 theme-toggle:circle stroke-dasharray ✓

**问题**:
- ✗ **L495 maximize stroke-width 1.3 vs 1.5**:三个窗口控制按钮中,只有最大化是 1.3,其他 1.5。视觉上最大化按钮的边框看起来"细一圈"
- ✗ **L609 send stroke-width 2 vs 其他 1.5**:发送键的箭头明显比顶栏设置图标粗,操控台三个按钮(more/send)粗细不一致(more 用了 fill 无 stroke 概念,send 是 2)
- ✗ **L504-509 水印 4 个不同 stroke-width(6/8/10/9)**:虽然有"从内到外递增"的设计意图,但 9 跟 10 差 1,8 跟 9 差 1,这种"递增但乱"的渐变在 14×14 渲染下完全看不出,只看到"线条粗细不一致的怪图"
- ✓ 所有 SVG 都有 viewBox、stroke="currentColor"、stroke-linecap round(water 缺 stroke-linejoin 但都用 line/circle/path 不需 join)
- ✗ 工具 chip 内的"✓"是 Unicode(L569),不是 SVG(违反 P2-4)

**修复方向**:
1. 全部 stroke-width 统一为 1.5(win-ctrl)、1.5(顶栏)、1.5(其他)
2. 水印简化为 1-2 个元素(如一个圆 + 一条横线),stroke-width 统一
3. ✓ 改 SVG

---

### 1.13 水印 — △

**代码**:`chat.html:161-176` + `chat.html:503-511`

**问题**:
- ✗ **stroke="#7B766C" 硬编码**(L504),不响应 dark mode 切换。dark mode 下基底是 #151411,#7B766C opacity 0.25 = 浅灰,**水印在 dark 下变成"深色背景上的浅色文字"**,对比度过高(违反水印"低调存在"的设计意图)
- ✗ **14×14px 渲染 240×240 viewBox**:缩放比 1:17,stroke-width 6/8/10/9 在 14px 下 → 实际线宽 0.35/0.47/0.59/0.53 px。**0.5px 线在 1x 屏会消失/抗锯齿成毛边**,在 retina 屏可能勉强可读但模糊
- ✗ 4 个元素叠在一起,14px 下根本看不出"northing logo 是什么" — 用户只能看到一个模糊的"小圆 + 横线"
- ✗ L172 `font-size: 11px` 不在 fs-token 里(fs-sm 10, fs-md 11.5, 没有 11)
- ✓ `font-variation-settings: "WONK" 1` 让 "northing" 字母有衬线变化,设计意图对
- ✓ opacity 0.25 数值符合哲学第 2 戒
- ✓ `left: s5; bottom: s4` 位置合理

**修复方向**:
1. 改用 2 个简单元素(例如一个圆 + 一个字"n"或一个简化版三角罗盘)
2. stroke 改 `var(--muted)`,opacity 改用颜色 alpha
3. 整体 viewBox 简化到 24×24 或 32×32
4. 文字大小改 10px 或 11.5px

---

### 1.14 窗口控制 — △

**代码**:`chat.html:140-159` + `chat.html:490-500`

**评价**:
- ✓ 三按钮 28×28,border-radius 7px(--r-win-btn),背景 raised,色 muted
- ✓ hover 三态(min→raised+fg, max→raised+fg, close→danger+on-rep)
- ✗ **close hover 颜色 #A45950 在 #F4F3F0 屋底上对比度约 5.5:1**,确实过重(但仅在 hover 触发,可接受)
- ✗ **三按钮视觉权重完全相同**(色 / 圆角 / 边框 / 阴影),没有 macOS 那种"关闭按钮平时是 dot,hover 才显示 ×"的层次
- ✗ 按钮图标细节:maximize rect rx=1.5 是合理的圆角,但描边粗 1.3 vs 其他 1.5 不一致(见 1.12)
- ✗ 窗口控制固定 top:16px right:24px,但顶栏高度 64px,从顶部算起按钮中心在 30px,**跟顶栏视觉上分离** — 应该跟顶栏右对齐(s5=24)或紧贴顶部

**修复方向**:
1. 窗口控制位置改成"顶栏内"或"紧贴顶栏下方"
2. 平时 close 用 dot,hover 才显示 ×(macOS 风)
3. 统一 stroke-width 1.5

---

### 1.15 textarea placeholder — △

**代码**:`chat.html:425-435` + `chat.html:606`
```css
.input-box textarea::placeholder { color: var(--faint); }
<textarea rows="1" placeholder="说点什么…"></textarea>
```

**问题**:
- ✗ **`--faint` #A8A398 对比度违规**:在 `--surface` #FBFAF8 底上,#A8A398 的对比度约 **2.1:1**(WCAG AA 要求 4.5:1,AAA 7:1;哲学 L34 说"≥4.0:1")。**placeholder 几乎不可读**
- ✗ 字重 normal,在 15px body 字号下显得"飘"
- ✗ 位置:rows=1,textarea 在 input-box 内部(input-box padding s3 s4 = 12 16),所以 textarea 内缩 12 顶 16 左 — 但 textarea 默认有浏览器自带 padding(通常 2px),文字视觉上离 input-box 上边约 14px,不对称
- ✓ "说点什么…" 用 ellipsis 字符,符合"语意未尽"诗意

**修复方向**:
1. placeholder 改 `var(--muted)` #7B766C(对比度约 4.5:1,达标)
2. 或加深 --faint 本身到 #908A7E(对比度约 3.0:1,仍在低线)
3. input-box padding 改 s3.5 s4(14 16)与 textarea 内置 padding 对齐

---

### 1.16 demo 边界处理 — △

**代码**:`chat.html:103-113`
```css
body {
  background: #1a1917;
  min-height: 100vh;
  display: flex; align-items: center; justify-content: center;
  padding: 40px;
}
```

**评价**:
- ✗ `#1a1917` 硬编码深色衬底,不在任何 token 里(dark --bg 是 #151411,不一样)
- ✗ `padding: 40px` 硬编码,不在 s-token 里
- ✗ 在 dark mode 下,body #1a1917 跟 shell 内的 dark --bg #151411 几乎无色差(差 ~3%),失去"假桌面"的层次
- △ 这种"窗口漂浮在桌面上"包装是评审原型的合理选择,但**实现没在 dark 模式做适配** — 浅色 demo 看不出来,深色 demo 会"窗口背景和桌面背景融合"
- ✓ flex 居中布局是对的

**修复方向**:
1. 加 token `--bg-demo: #1a1917` 和 `--s-demo: 40px`
2. dark mode 下改用更浅的桌面色(例如 #0E0D0B)拉开层次
3. 或直接 demo 化掉,改用全屏居中(无 padding)

---

## 2. "丑"的具体证据(7 条)

按严重度排序,每条都配 HTML 行号。

### #1 · active turn 双层 background 完全失效(最严重)

**位置**:`chat.html:387-389`
```css
background:
  linear-gradient(180deg, var(--air-active) 0%, transparent 100%),
  var(--air-active);
```
**问题**:双层叠加后,顶部 4% + 4% = 8% rep,底部 0% + 4% = 4% rep — 应该是"顶部 8% → 底部 4%"的微弱渐变,但视觉上几乎看不出方向(因为 4% 在 #F4F3F0 上等于 0.5% 色差)。**作者想要的"从上到下淡出"效果完全没实现**,active 轮就是一块均匀的"微微发橙"的方块,跟其他 turn 视觉无差。

**为什么丑**:这是页面唯一的"现在"标记,应该是视觉最强的锚点,但实现上它**既不"鲜"也不"渐变"**。

### #2 · 用户消息被错误套上 .sediment 当场褪色

**位置**:`chat.html:575`
```html
<div class="turn user sediment">
```
**问题**:这是 14:08 用户消息,紧接着 14:08 · 现在 的 agent 活跃轮 — 是当前对话流的**最新消息**。但代码给它加了 `.sediment` 类,触发 L343 `opacity: 0.55`。

**为什么丑**:用户刚说完的话,屏幕上看起来像"三天前的老消息"。"沉积 = 旧"的语义被打破,用户会困惑"我刚说的话为什么这么淡"。更严重的是,active agent 轮(14:08 · 现在)的 .turn-name 颜色是 fg + font-weight 450,而用户消息的 .turn-name 变成 muted(L340)— 视觉上 **"我刚说的"比"现在的 agent 回应"还要弱**,整个对话流的视觉重心被颠倒。

### #3 · 顶栏"序"字字体错配,Fraunces 渲染中文失败

**位置**:`chat.html:198-204`
```css
.compact-avatar .glyph {
  font-family: var(--font-display);  /* Fraunces */
  font-size: 16px;
  font-variation-settings: "WONK" 1;
  font-weight: 600;
}
```
**问题**:Fraunces 是拉丁衬线体,**不支持 CJK**。浏览器会回落到系统字体(Windows = Microsoft YaHei UI, macOS = PingFang SC),WONK 1 和 font-weight 600 在中文上是"无法控制"的 — 实际显示是系统默认 16px 中等字重。

**为什么丑**:头像里的"序"字应该是品牌视觉锚点(跟整屋染色 + 名字 一起构成"知序 = 这个人"),但实际显示是"普通中文字",没有衬线,没有 WONK 的笔画变化。**头像看起来就是"系统默认 16px 600 普通字",没有任何设计痕迹**。

### #4 · 思考块"· 思考 ·"用 ::before 伪元素(违反硬约束 #8)

**位置**:`chat.html:358-365`
```css
.think::before {
  content: "· 思考 ·";
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  color: var(--abyss-500);
  ...
}
```
**问题**:哲学硬约束 #8 明确禁止 `::before` / `::after` 伪元素当主元素(Slint 翻译限制)。这里"· 思考 ·"是思考块的核心 label,不是装饰 — 它告诉用户"下面是 agent 的思考"。

**为什么丑**:(1) 违反硬约束,落库时 Slint 翻译会丢;(2) mono 字体渲染中文字符"思考",在等宽下字距撑得太开,看起来不像 label 像"散落的字";(3) 没法响应 dark mode 独立调色,也没法被屏幕阅读器识别为 label。

### #5 · 水印 14×14 渲染 4 层 SVG,实际线宽 0.5px

**位置**:`chat.html:504-509`
```html
<svg width="14" height="14" viewBox="0 0 240 240" fill="none" stroke="#7B766C" stroke-linecap="round">
  <circle cx="120" cy="120" r="16" stroke-width="6"/>
  <path d="M120 60 A60 60 0 1 1 78 78" stroke-width="8"/>
  <path d="M120 30 A90 90 0 1 1 50 60" stroke-width="10"/>
  <line x1="30" y1="120" x2="210" y2="120" stroke-width="9"/>
</svg>
```
**问题**:
- 14px 容器 × viewBox 240 → 缩放比 1:17
- 实际渲染线宽 6/8/10/9 × (14/240) = **0.35/0.47/0.59/0.53 px**
- 0.5px 线在 1x 屏(DPI 96)会**抗锯齿成不可见或半透明毛边**,在 retina 屏勉强可读但模糊
- 4 个不同粗细(6/8/10/9)递增但"不规整"(8→9 差 1,9→10 差 1),看不出"从内到外渐粗"的设计意图

**为什么丑**:左下角水印应该是"低调存在",但现在它要么"看不见"(1x 屏),要么"模糊一团"(retina),要么"颜色不对"(dark mode 下硬编码 #7B766C 不变)。完全没达到"角落品牌露出"的效果。

### #6 · placeholder 颜色对比度 2.1:1(违反无障碍标准)

**位置**:`chat.html:435`
```css
.input-box textarea::placeholder { color: var(--faint); }  /* #A8A398 */
```
**问题**:--faint #A8A398 在 --surface #FBFAF8 底上,WCAG 对比度 **2.1:1**(标准要求 4.5:1,哲学要求 ≥4.0:1)。

**为什么丑**:"说点什么…"是输入框唯一的引导,在用户键入前吸引他们说话 — 但**用户根本看不清写的是什么**。在 13 寸笔记本上几乎要凑近屏幕才能确认 placeholder。

### #7 · 操控台发送键 stroke-width 2 vs 全站 1.5

**位置**:`chat.html:609`
```html
<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <line x1="12" y1="19" x2="12" y2="5"/>
  <polyline points="5 12 12 5 19 12"/>
</svg>
```
**问题**:发送箭头 stroke-width=2,顶栏设置图标 1.5,窗口控制按钮 1.5(最大化还是 1.3)— **三个操控按钮里,发送键的图标明显比 more(三圆点)粗**。在 18×18 渲染下,1.5 vs 2 视觉差约 30%。

**为什么丑**:发送键本身背景是 rep-500(暖橙)实心块,前景白色箭头已经够醒目,**stroke-width 2 是双重强调**,反而让箭头显得"用力过猛"像 emoji。同时跟顶栏 / 窗口控制 的图标线条粗细不一致,整套图标的视觉语言被打散。

---

## 3. token 偏离清单(汇总)

| 位置 | 当前值 | token 应是 | 偏离 |
|---|---|---|---|
| L107 | `background: #1a1917` | 应新增 `--bg-demo` | 硬编码深色衬底,不在 token 体系 |
| L112 | `padding: 40px` | 应新增 `--s-demo: 40px` | 不在 s1-s6(24/32)范围 |
| L173 | `font-size: 11px` | 应改 `var(--fs-md)` 11.5 或 `var(--fs-sm)` 10 | 不在 fs-token 里 |
| L200 | `font-size: 16px`(glyph) | 应 `var(--fs-body)` 15 或 `var(--fs-name)` 16(但 font 错配更严重) | 偏离 |
| L201 | `font-variation-settings: "WONK" 1` | 应对中文无效 | 字体错配 |
| L213 | `margin-top: 2px` | 应 `var(--s1)` 4 或 0 | 不在 4 基数(可接受) |
| L287 | `width: 9px; height: 9px` | nd 圆点直径,无对应 token | 数值,无需 token |
| L304 | `width: 6px`(scrollbar) | 无 token | 细节,无需 token |
| L335 | `max-width: 540px`(user) | 跟 agent 760px 不成 token 化 | 比例不规整 |
| L355 | `max-width: 480px`(think) | 跟 turn 760px 不成 token 化 | 比例不规整 |
| L416 | `min-height: 44px` | 无 token | 44 不是 4 基数 |
| L438 | `width: 44px; height: 44px`(send-btn) | 无 token | 44 不是 4 基数 |
| L455 | `width: 44px; height: 44px`(more-btn) | 无 token | 同上 |
| L473 | `width: 36px; height: 36px`(theme-toggle) | 无 token | 36 不是 4 基数 |
| L504 | `stroke="#7B766C"` | 应 `var(--muted)` | 硬编码颜色 |
| L495 | `stroke-width="1.3"` | 应 1.5 | 线条粗细不一致 |
| L609 | `stroke-width="2"` | 应 1.5 | 线条粗细不一致 |
| L569 | `✓` Unicode | 应 SVG | 违反 P2-4 |

**token 化程度**:**88%**(估算)。基础 token(颜色/字号/间距/圆角)90%+ 都用 var,但**SVG 细节(线条粗细、颜色)、尺寸(44/36 这种 4 倍数但不规整)、demo 边界(40px padding)三处有明显 token 空白**。

---

## 4. 改进清单

### P0(必须修,影响设计意图)

1. **修复 active turn 双层 background**(`chat.html:387-389`)
   - 删掉第二层 `var(--air-active)`,只留 `linear-gradient(180deg, rgba(200,113,76,0.05) 0%, rgba(200,113,76,0.015) 100%)` 单层
   - 或:用更显眼的染色,如 `rgba(200, 113, 76, 0.06)` 单层

2. **删除 user 轮 .sediment 类**(`chat.html:575`)
   - `<div class="turn user">`(去掉 sediment)
   - 用户消息永远保持不透明,只有 agent 旧轮才沉积

3. **修复顶栏 glyph 字体**(`chat.html:198-204`)
   - 改 `font-family: var(--font-body)`,`font-size: 20px`,`font-weight: 500`
   - 颜色改 `color: var(--on-rep)`(不透明)
   - 去掉 `font-variation-settings`(对中文无效)

4. **修复思考块 ::before 伪元素**(`chat.html:358-365`)
   - 改用真实 DOM:`<span class="think-label">· 思考 ·</span>` 在 .think 第一行
   - 样式改为 `.think-label { font-family: var(--font-body); font-weight: 500; font-size: var(--fs-sm); color: var(--abyss-500); letter-spacing: .05em; }`

### P1(应该修,影响细节品质)

5. **在场区加 64px 头像**(`chat.html:535-541`)
   - 跟顶栏 40px 头像形成 64/40 两档 hierarchy
   - avatar 和名字水平 baseline 对齐

6. **修复 nd 圆点视觉**(`chat.html:283-291`)
   - 去掉 1.5px border,改成 6px 实心点紧贴右端(`right: 0`)
   - 或改为竖线刻度(`width: 2px; height: 10px; right: -3px`)

7. **统一 SVG stroke-width**(全文件)
   - 窗口控制 1.5(已统一,只改 L495 1.3→1.5)
   - 顶栏 1.5(保持)
   - 操控台发送 1.5(改 L609 2→1.5)
   - 水印统一(6/8/10/9 → 简化为 2 个元素 stroke-width 1.5)

8. **水印改用 var(--muted)**(`chat.html:504`)
   - 改 `stroke="currentColor"` + CSS `color: var(--muted)`
   - 简化图形:一个圆 r=10 + 一条横线
   - 文字 font-size 改 10px 或 11.5px

9. **placeholder 颜色加深**(`chat.html:435`)
   - 改 `color: var(--muted)`(对比度 4.5:1,达标)
   - 或:加深 --faint 到 #908A7E(对比度 3.0:1,仍在低线,折中)

10. **加 token 化 44/36 尺寸**
    - 加 `--btn-sm: 36px; --btn-md: 44px;` (theme-toggle 36, send-btn/more-btn 44)
    - `min-height: 44px` 也用 token

11. **整屋染色浓度提升**
    - `--air-bg` 3.5% → 5%
    - `--air-halo` 7% → 10%
    - `--air-mist` 1.5% → 删掉或 2.5%

### P2(可改可不改,精致度)

12. **demo 边界 40px padding 加 token**:`--s-demo: 40px`
13. **watermark .bt font-size 11 → 10 或 11.5**
14. **用户气泡底色**:`--elevated` #FFF → `--surface` #FBFAF8(更暖,消除白方块感)
15. **删除 chip 边框** `border: 1px solid var(--border-soft)` → 底色已够区分
16. **chip 内 "✓" 改 SVG**:`<svg viewBox="0 0 12 12"><path d="M2 6 l3 3 l6 -6" stroke="currentColor" fill="none" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>`
17. **breathe keyframe 末态归位**:从 scale 1, opacity 0.85 改为 scale 1, opacity 1(清晰"点亮"语义)或 scale 1, opacity 0.65(明确"呼吸循环结束"语义,看设计意图)
18. **meta margin-top 2px 改 0 或 var(--s1)**:取决于"名字 + 状态"两行距离设计

---

## 5. 总结

这份 demo 的"骨架"(token 化、字体三系、整屋染色、用户/agent 边界、操控台 3 控件)做得很扎实,**完全有 8 分底子**。

但**执行层有 4 处 P0 bug** 让它掉到 6.5:
1. active turn 渐变写反
2. user 消息当场褪色
3. glyph 字体错配
4. think ::before 违规

这 4 条都是"骨架对但血肉错" — 不是哲学理解问题,是写代码时的手滑 / CSS 双层 background 误用 / 中文不支持 CJK 字体的盲区。

**建议**:**先把 P0 4 条修完再评分**,修完应该能到 8.0-8.5。P1 / P2 是精致度问题,可以下一轮迭代。
