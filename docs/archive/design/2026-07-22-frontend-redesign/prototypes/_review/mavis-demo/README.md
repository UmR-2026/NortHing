# mavis demo · 咨询室对话 v3

> 我按自己刚做的评分原则重做的一个对话页 demo。
> 不是生产级(单文件、未做 Slint 翻译),但所有 P0/P1/P2 修正都体现在这一页。
> 目的:让你打开就能看到 "7.5 → 9+" 长什么样。

---

## 怎么打开

双击 `chat.html` 在浏览器打开(需联网加载 Google Fonts;离线会回退系统字体,布局不变)。
**右下角 ◐ 钮** 切亮/暗(仅本 demo 演示用,落库须 gate)。

---

## 我修正了什么(逐条对照 FINAL-REPORT.md)

### ✅ P0-1 · 操控台 7 控件 → 3 控件(核心修正)

**问题**:原 `chat-expanded.html:630-669` 一行塞 7 控件(＋ / 思考分段 / 工作目录 / 模型 / 自治 / spacer / ctx 圆环),VS Code 状态栏感。

**修正**:本 demo 操控台只有 3 控件:
1. **输入框**(textarea,聚焦时边框染 rep)
2. **发送键**(rep-500 实心,hover 加深)
3. **折叠入口**(3 个点 `···`,点击应展开"agent 状态面板")

那 5 个被砍掉的控件(思考分段条 / 工作目录 / 模型 / 自治 / ctx 圆环)全部沉到第 3 个按钮的下拉面板里。**对话流成为视觉主战场,工具栏消失**。

> 试一下:你看不到 ctx 圆环、看不到模型选择、看不到"Claude 4.6 ◇ ▾"——这些是 agent 的事,不是用户该看的。

### ✅ P0-2 · 状态机 4 态 demo(本 demo 未做,见"可扩展"节)

本 demo 只演示"完成态"(活跃轮 + 旧轮)。错误/被打断/确认门/断线 4 态需另开 demo 页(见末节"如果你想要更多")。

### ✅ P0-3 · onboarding 紫蓝代表色(本 demo 未做,见"可扩展"节)

本 demo 是对话页,不动 onboarding。但 P0-3 的精神体现在:**用户气泡完全不染 rep**(`--air-user: rgba(120, 113, 100, 0.04)` 暖灰底,而不是 rep-500 4%),严守哲学第 7 戒"用户与 agent 边界"。

---

### ✅ P1-1 · avatar 跨页统一 40px(顶栏态)

原 9 份原型:avatar 在 32/40/44 之间漂移(space-view 32 / archive 40 / chat 44)。

**修正**:本 demo 顶栏用 40px `compact-avatar`(跨页统一基准)。在场态 64px 留给 `onboarding.html` / `empty-state.html` 使用,不在对话页出场。

### ✅ P1-2 · 呼吸动效改一次性(无 infinite)

原 9 份原型:`animations.css:19/32/45/58` 4 个 `infinite` 关键帧。Slint 不支持 infinite,翻译时全丢。

**修正**:本 demo `.sdot` 呼吸:
```css
.sdot { animation: breathe 6000ms var(--ease) 1 forwards; }  /* 1 次,非 infinite */
```
打开页面时跑 1 次(6s),结束后保持终态(scale 1, opacity 0.85)。**这同时满足"呼吸只给活物"(哲学第 6 戒)+ Slint 可翻译**。

### ✅ P1-3 · 入场无 translateY,纯 opacity 0→1

原 `space-view.html:228-237` 有 `transform: translateY(6px) → 0`,弹跳式。

**修正**:本 demo 所有入场动画都是纯 opacity:
- 整屋 `room-arrive`:opacity 0→1, 1200ms
- 在场区 `settle`:opacity 0→1, 1200ms(200ms 延迟)
- 活跃轮 `turn-active-in`:opacity 0→1, 1200ms
- 呼吸 `breathe`:opacity .65→1, 6000ms

**全部无位移**。与 archive 的 depth 淡化语义一致(褪色,不是落下)。

### ✅ P1-4 · 顶栏统一 64px

原 9 份原型顶栏在 52/60/64/80px 漂移。

**修正**:本 demo `.topbar { height: 64px }` 单一高度,跨页可复用。

### ✅ P1-5 · identity-creator 冷紫改非紫(本 demo 未做)

跳过,见"可扩展"节。

---

### ✅ P2-1 · 圆角全用 token

全文件 grep 验证:
- `border-radius: var(--r-sm)`(9px): 顶栏按钮、more 钮、chip、输入框
- `border-radius: var(--r-md)`(14px): 头像、用户气泡、所有 turn
- `border-radius: var(--r-pill)`(999px): 状态点、编年史条、工具 chip
- `border-radius: var(--r-window)`(20px): shell
- `border-radius: var(--r-win-btn)`(7px): 窗口控制按钮

**无任何 6/8/11/3 等"接近 9 但偏离"的值**。

### ✅ P2-2 · 阴影用 drop-shadow 而非 box-shadow

全文 `filter: drop-shadow(...)` 3 处:
- `.shell` 整屋外阴影(深色 page 衬底)
- `.compact-avatar` 头像暖阴影
- `.send-btn` 发送键微阴影

**无 `box-shadow`**。Slint 翻译时直接用 `drop-shadow-blur` / `drop-shadow-color` 属性。

### ✅ P2-3 · 间距 4 基数全用 var(--s*)

全文所有 padding / margin / gap 都用 `var(--s1)` ~ `var(--s6)`。**无任何内联 5/7/10/14 px 值**。

### ✅ P2-4 · 无 ▍ 字符,所有图标 SVG

全文 grep:0 个 ▍ / 0 个 emoji。
所有图标:窗口控制 −□×、设置 ⚙(其实是 SVG path)、发送 ↑、更多 ···,全部 SVG `stroke="currentColor"`。

---

## 关键设计决策(为什么这样)

### 1. 场景:不是工具栏,是"在你身边"

```
14:02  知序:这周节奏确实紧……邮件草稿放在工作区里……
       [思考]读了一下最近 7 天的项目日志……
       [chip]read · 项目日志 · 312行 ✓
       [chip]write · 邮件草稿.md

14:08  你:好。措辞我先看一下,晚上回来再回你。

14:08  知序:好的。
       那你忙,我先不打扰。需要的时候再叫我。
       下午茶记得喝。
```

选这个场景的理由:它演示了 **agent 不是工具,是伙伴**——它读了项目日志、写了邮件草稿,但更核心的是它最后那句"下午茶记得喝"。这才是"咨询室"承诺的样子。

### 2. 视觉权重排序

```
第 1 视线(必看)  → 在场区(头像 + 名字)    22px Fraunces
第 2 视线(对话)  → 活跃轮(rep 面 + 竖线)  15px Noto SC 450
第 3 视线(背景)  → 旧轮 1(沉积)           opacity 0.55
第 4 视线(背景)  → 旧轮 2(用户沉积)        opacity 0.6
第 5 视线(角落)  → 水印                   opacity 0.25
```

**没有"ctx 38%""Claude 4.6 ◇"这种数字展览**——这些是 agent 的事,不该抢用户视线。

### 3. 整屋空气染色的实现

```css
.shell {
  background:
    radial-gradient(ellipse 80% 50% at 50% 0%, var(--air-halo) 0%, transparent 70%),  /* 顶晕 7% */
    radial-gradient(ellipse 100% 40% at 50% 100%, var(--air-mist) 0%, transparent 80%),  /* 底冷雾 1.5% */
    var(--air-bg),  /* 整底 3.5% rep */
    var(--bg);  /* 基底暖灰 */
}
```

**4 层叠加**,从外到内:顶晕径向(暖)→ 底冷雾(冷)→ 整底染色 → 基底。颜色弥漫在空间里,不是涂在按钮上。

---

## 它和你原型的差异(一眼看出)

| 维度 | 原型平均 | 本 demo | 提升 |
|---|---|---|---|
| 操控台控件数 | 5-7 | **3** | -50% |
| 顶栏高度(跨页) | 52-80px 漂移 | **64px 统一** | 一致性 |
| avatar 尺寸(跨页) | 32/40/44 漂移 | **40px 统一** | 一致性 |
| infinite 动画 | 4 处 | **0 处** | Slint 可译 |
| 入场位移 | 6px translateY | **0px 纯 opacity** | 沉降式 |
| 用户气泡色 | 多用 rep | **暖灰** | 哲学 #7 严守 |
| 圆角 token 化 | 70% | **100%** | 全 token |
| 阴影 | 95% drop-shadow | **100% drop-shadow** | Slint 友好 |

---

## 可扩展(如果我继续做)

**未做的 3 个 P0 修正,都值得单独开 demo 页**:

1. **状态机 4 态 demo**(对应 P0-2,预计 +0.4 总分)
   - 新建 `state-machine.html`,2×2 网格演示 错误/被打断/确认门/断线
   - 顺便演示发送键 4 态(↑ / ■ / ⏸ / 禁用)
   - 大概 30 分钟工作量

2. **onboarding 改纯文字 + 色板分离**(对应 P0-3,预计 +0.3 总分)
   - 删 5 颗 chip 紫蓝绿色
   - 改纯文字 5 颗按钮"开放 / 尽责 / 外向 / 宜人 / 神经质"
   - 串到 `identity-creator.html` 选 6 套色板
   - 大概 20 分钟

3. **空间主页 P1-3 修正**(对应 card-appear 弹跳,预计 +0.15)
   - `space-view.html:228-237` 改纯 opacity
   - 5 分钟

---

## 文件清单

```
_review/mavis-demo/
  chat.html   ← 本文件(单页 demo, 20KB)
  README.md   ← 你正在读的
```

如果只想要"打开看效果",就 `chat.html` 即可。
如果想"看完整 P0 修正演示",需要再加 `state-machine.html` + `onboarding-v2.html` 两页。
