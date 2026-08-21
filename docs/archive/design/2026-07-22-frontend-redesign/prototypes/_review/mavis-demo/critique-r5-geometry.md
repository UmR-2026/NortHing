# mavis demo · 按键布局几何专项检查(r5,v5)

> **评审对象**:`mavis-demo/chat-v5.html`(23,289 字节,761 行)
> **评审基线**:r4 终局 9.5/10 + 用户指出的"v4 评审员漏看按键不能重叠"
> **评审角色**:按键布局几何专项检查官(只认证据,mental view 渲染)
> **评审日期**:2026-08-15
> **方法声明**:**严格 mental view 渲染**。所有 bounding box 由 CSS token + 容器约束推导,不靠"看上去"。
> **立场声明**:用户说 v4 漏看按键不能重叠 — 那我这次就**只查按键不重叠 + 间距合规**,其他维度不展开。

---

## 0. 摘要

| 维度 | 评分 | 一句话 |
|---|---|---|
| **按键几何布局** | **9.5 / 10** | v4 6 个具体问题全部实修,7 处按键 group 全部满足"不重叠 + 间距 ≥ 8px",有 1 处微瑕 |
| **视觉呼吸** | **9.0 / 10** | 顶栏 spacer 仍 ~930px(r3→r4 死区问题没继续),其他区域呼吸感比 v4 更均匀 |
| **总评** | **9.3 / 10** | 几何维度 r4 是漏看,v5 补完了;但顶栏中间的死区(不在本次 6 个修复范围内)依然是大头 |

---

## 1. mental view 元素清单(7 个 group × N 个按键)

按位置 / 尺寸 / 视觉重量 / 跟邻居间距列出**所有可交互元素**。

### 1.1 顶栏(64px 高,padding 0 24,flex gap 12)

| 元素 | 位置(x) | 尺寸 | 视觉重量 | 跟邻居间距 | 备注 |
|---|---|---|---|---|---|
| **compact-avatar** | 24 + 0 = 24 | 40×40, drop-shadow 0 2px 6px rgba(80,70,55,.18) | 高(渐变 rep-300/500/600 + shadow) | 跟 ritual:**12px**(compact 内部 gap) | L184-190 |
| **ritual**(非按键,信息) | 24 + 40 + 12 = 76 | 宽 ~50px,高 36px(列) | 低(mono fs-sm + fs-md) | 跟 compact border-right:**12px**(padding-right) + 1px | L198-214, L177-183 |
| *compact 右侧 1px 边界* | ~140 | 1×40 | 极低(--border-soft) | — | L181 |
| **topbar-spacer** | 152 → 1086 区间 | flex:1(空) | 0 | 跟 topbar-actions border-left:**12px**(topbar gap) | L216 |
| *topbar-actions 左侧 1px 边界* | 1110 | 1×40 | 极低 | — | L223 |
| **topbar-archive** | 1122 | 32 高,padding 0 12,内容 12(svg) + 8(gap) + 24(2 zh chars @ fs-md) ≈ 68 宽 | 中(border 1px border-soft,hover → raised + fg) | 跟 divider:**12 + 4 + 1 + 4 + 12 = 33px** | L232-250 |
| **topbar-divider**(垂直 1px) | 1190 附近 | 1×20,margin 0 4px | 极低 | 左右各 12 + 4px 边距 | L226-230 |
| **topbar-action**(theme ◐) | 1224 | 32×32,svg 16 | 中(hover → raised + muted) | 跟 deck:**24px**(topbar 右 padding) | L252-261 |

**关键观察**:avatar 跟 ritual 之间的 12px gap 来自 compact 内部 gap(不是顶栏 gap)。这是 v5 修复 #6 的核心 — 把两个原本散在顶栏的元素圈进一个 compact block,中间留 12px,右边用 1px border 划清 block 边界。

### 1.2 窗口控制(右上角,absolute)

| 元素 | 位置 | 尺寸 | 间距 |
|---|---|---|---|
| 最小化 | right:16, top:12 | 26×26 | 跟最大化:8px |
| 最大化 | right:50, top:12 | 26×26 | 跟关闭:8px |
| 关闭 | right:84, top:12 | 26×26 | — |

注:此区域是窗口装饰,跟主对话无交互关系,不在本次审查范围。

### 1.3 opening(padding 16 24 16, gap 16, border-bottom 1px)

| 元素 | 位置 | 尺寸 | 间距 |
|---|---|---|---|
| **opening-avatar** | 24 | 48×48, drop-shadow 0 3px 8px rgba(200,113,76,.18) | 跟 opening-meta:**16px** ✓ |
| opening-greeting | 24+48+16 = 88 | Fraunces 22px, 单行 | 跟 ritual:8px(列 gap) |
| opening-ritual | 88 | italic 11.5px, 单行 | 跟 line:8px |
| opening-line | 88 | 13px, 单行 | 跟 aside:8px |
| opening-aside | 88 | mono 11.5px, padding 4 12, background raised | 跟 story:8px(story 默认 display:none) |
| opening-story | 88 | 11.5px italic, padding-left 12, border-left 2px abyss, max-width 540, **仅 :hover 显示** | — |

**关键观察**:4 个可见 text 元素之间列 gap 8px ✓;avatar 跟 meta 之间 16px ✓。aside 是 inline pill,自身有 4×12 padding,跟邻居有 8px 列间距(足够)。

### 1.4 stream(padding 16 24 12, flex column gap 24)

| 元素 | 尺寸 | 跟邻居 |
|---|---|---|
| **turn.agent.sediment** | max-width 680,padding 0 16,opacity 0.5 | 跟 next turn:**24 + 8(user margin-top) = 32px** |
| **turn.user** | max-width 680,padding 12 16,border 1px border-soft,radius 14,margin-top 8 | 跟 next turn:**24 + 16(active margin-top) = 40px** |
| **turn.agent.active** | padding 16/16/16/20(L412),border-top 3px rep-500,radius 14,margin-top 16,background-color + bg-image 双层 | — |
| ├ turn-head | flex baseline gap 8,margin-bottom 8 | — |
| ├ turn-body | line-height 1.7, p margin-bottom 12 | — |
| └ think | padding 12 16, margin 12 0, max-width 580, border-left 2.5px abyss-400, background air-thinking | 跟 turn-body 最后一个 p:**12px**(think margin-top) |

**关键观察**:3 turns 间距 = 24(stream gap) + 各自 margin-top = 32 / 40,**总间距合规**。think 块在 active turn 内部,不是 sibling,所以不参与 stream gap 计算,12px margin 上下独立 ✓。

### 1.5 traces(padding 8 24, gap 16, border-top + border-bottom 1px, margin-bottom 8)

| 元素 | 尺寸 | 间距 |
|---|---|---|
| **trace 笔记** | 12(svg) + 8(gap) + ~120(8 chars @ 11.5px mono) ≈ 140 宽 | 跟 trace 水:**16px** ✓ |
| **trace 水** | 12 + 8 + 12(1 zh char) ≈ 32 宽 | 跟 trace 在:**16px** ✓ |
| **trace 在** | 12 + 8 + 12(1 char) + 4(margin-left) + 6(dot 6×6) ≈ 42 宽 | — |

**关键观察**:traces 跟 deck 视觉分隔 = 1px border-bottom + 8px margin-bottom(v5 修复 #4)✓。

### 1.6 deck(padding 12 24 16, flex column gap 8)

| 元素 | 位置(从 deck 左 24px 起) | 尺寸 | 间距 |
|---|---|---|---|
| **input-box** | 24 | flex:1(约 1088 宽),padding 12 16,min-height 48,border 1px border-soft,background air-input | 跟 send-btn:**12px** ✓ |
| **send-btn** | 24 + 1088 + 12 = 1124 | 48×48,background rep-500,drop-shadow 0 2px 4px rgba(80,70,55,.22),color on-rep | 跟 more-btn:**12px** ✓ |
| **more-btn** | 1124 + 48 + 12 = 1184 | 48×48,background surface,border 1px border-soft,drop-shadow 0 1px 2px rgba(80,70,55,.14),color muted | 跟 deck 右 padding:**24px** ✓ |
| **hints**(列) | — | gap 8,padding-top 4 | 跟 input-box(deck-row 底):**8(deck gap) + 4(hints padding-top) = 12px** ✓ |
| ├ **hint 我今天有点累** | 24 | fs-md 11.5px, padding 4 12, border 1px border-soft | 跟 next hint:8px |
| ├ **hint 我也不知道想说什么** | — | 同上 | 跟 next hint:8px |
| └ **hint 嗯。** | — | 同上 | — |

**关键观察**:
- input ↔ send ↔ more 三段 gap 都是 12px(v5 修复 #2)✓
- more-btn 跟 deck 右内边距 24px,**没贴边** ✓
- hints 跟 input 之间 12px 净距(8 deck gap + 4 hints padding-top,v5 修复 #5)✓
- 3 个 hint 间距 8px ✓

### 1.7 shell 本身

- 1280×800 固定尺寸(L117-118)
- 圆角 20px,filter drop-shadow 0 24px 48px
- background 4 层径向渐变 + var(--air-bg) + var(--bg)
- 内部:watermark(左下 absolute, opacity 0.22)跟所有 button 都不冲突(它在 decorative 位置)

---

## 2. 几何检查表(逐项)

### 2.1 顶栏

| 关系 | 期望 | 实际(从 CSS 推) | 结果 |
|---|---|---|---|
| avatar ↔ ritual 间距 | 12px(compact 内部 gap) | L179 `gap: var(--s3)` = 12px,avatar 40 + 12 = 52px 处 ritual 开始 | **PASS** |
| compact border-right | 1px solid border-soft | L181 `border-right: 1px solid var(--border-soft)` ✓ | **PASS** |
| archive ↔ theme 间距 | "应该有 divider,8/12" | L226-230 divider(1×20,margin 0 4px),L222 gap 12 | **PASS,但实际总间距 33px**(12+4+1+4+12)— 比其他 gap 宽不少,但 divider 视觉切分清晰,意图合理 |
| archive 距 topbar-actions border-left | ≥ 12px | archive 左边距 = 12(padding-left L222) + 1(border) + 12(gap L221) = 25px | **PASS** |
| theme 距 topbar 右 padding | ≥ 12px | theme 32 + 24(padding) = 56px | **PASS** |

### 2.2 opening

| 关系 | 期望 | 实际 | 结果 |
|---|---|---|---|
| avatar ↔ opening-meta | gap 16 | L266 `gap: var(--s4)` = 16px ✓ | **PASS** |
| 4 个 text 元素列 gap | 8px | L291 `gap: var(--s2)` = 8px ✓ | **PASS** |

### 2.3 stream

| 关系 | 期望 | 实际 | 结果 |
|---|---|---|---|
| 3 turns 间距 | gap 24 | L348 `gap: var(--s5)` = 24px ✓ | **PASS** |
| turn ↔ turn-head | turn-head margin-bottom 8 | L360 `margin-bottom: var(--s2)` = 8px ✓ | **PASS** |
| turn-head ↔ turn-body | (无 margin,通过 turn-head margin-bottom 控制) | 8px(已算) | **PASS** |
| turn-body ↔ think | think margin 12 0 | L401 `margin: var(--s3) 0` = 12px ✓ | **PASS** |

### 2.4 traces

| 关系 | 期望 | 实际 | 结果 |
|---|---|---|---|
| 3 traces 间距 | gap 16 | L448 `gap: var(--s4)` = 16px ✓ | **PASS** |
| traces ↔ deck | border-bottom 1px + margin-bottom 8px | L455 + L457,两条都在 ✓ | **PASS** |

### 2.5 deck

| 关系 | 期望 | 实际 | 结果 |
|---|---|---|---|
| input ↔ send 间距 | gap 12 | L493 `gap: var(--s3)` = 12px ✓ | **PASS** |
| send ↔ more 间距 | gap 12 | 同上,12px ✓ | **PASS** |
| more ↔ deck 边框 | ≥ 12px(没贴边) | more 右沿 = 1256, deck 右 padding = 24,左沿 = 1232,more 离右边 24px ✓ | **PASS** |
| hints ↔ input 间距 | padding-top 4 | L560 `padding: var(--s1) 0 0 0` = 4 0 0 0 ✓(v4 是 `padding: 0 var(--s1)`,v5 改成 `4 0 0 0`,4px 顶,移除左右 4px) | **PASS** |

### 2.6 hint

| 关系 | 期望 | 实际 | 结果 |
|---|---|---|---|
| 3 hints 间距 | gap 8 | L559 `gap: var(--s2)` = 8px ✓ | **PASS** |

---

## 3. 重叠检测(mental view bounding box)

逐个 group 推算 bounding box:

### 3.1 顶栏(y:0-64)

| 元素 | bbox |
|---|---|
| compact-avatar | (24, 12, 40, 40) + shadow 延伸到 (24, 12, 40, 46) |
| ritual | (76, 14, ~50, 36) |
| compact border-right | (~138, 12, 1, 40) |
| topbar-spacer | (152, 0, ~934, 64) — 空 |
| topbar-actions border-left | (1110, 12, 1, 40) |
| archive | (1122, 16, 68, 32) |
| divider | (1190, 22, 1, 20) |
| theme | (1208, 16, 32, 32) |

**重叠检测**:
- compact-avatar shadow(46px)跟 ritual(从 y=14 起)— 影子只到 y=46,ritual 从 y=14 开始,**垂直方向有重叠,但 ritual 在 avatar 右侧(x=76+,avatar 在 x=24-64),水平不重叠** ✓
- archive 跟 divider:archive 右边 x=1190,divider 左边 x=1190 — **正好相接,边界像素相邻** ✓(12px gap + 4px margin = 16px,archive 68 宽后从 1190 开始,divider 从 1190+12+4=1206 开始... 等等,让我重算)

**重算顶栏右侧**:
- topbar-actions 左沿 = 1110(border-left 1px 占 1110-1111)
- padding-left 12 → archive 从 1111+12=1123 起
- gap 12 → archive 宽 68 → archive 右边 = 1123+68 = 1191
- gap 12 → divider margin-left 4 → divider 左边 = 1191+12+4 = 1207
- divider 1px → divider 右边 = 1208
- divider margin-right 4 → gap 12 → theme 左边 = 1208+4+12 = 1224
- theme 32 → theme 右边 = 1256
- topbar 右 padding 24 → topbar 右边 = 1280 ✓

archive 跟 divider 之间净距 = 12+4 = 16px。**远超 8px 阈值** ✓
divider 跟 theme 之间净距 = 4+12 = 16px。**远超 8px 阈值** ✓

**结论:顶栏无重叠** ✓

### 3.2 opening(y:64-209,约 145px)

- opening-avatar: (24, 80, 48, 48)
- opening-meta: (88, 80, ~1144, ~112)

**重叠检测**:avatar 右边 x=72,meta 左边 x=88,**间距 16px** ✓
4 text 元素在 meta 内部 column gap 8,**垂直堆叠不重叠** ✓

### 3.3 stream(y:209-?)

3 turns + gaps,各 turn 自己 max-width 680,内容左对齐到 x=24。
- sediment turn: (24, ~209, 680, ~80)
- user turn: (24, ~313, 680, ~70) — sediment bottom 24 + 8(margin-top) = 32px 净距
- active turn: (24, ~407, 680, ~250) — 含 think 块

**重叠检测**:turns 之间间距 24+(各自 margin-top) = 32/40,远超 8px 阈值 ✓
active turn 内部 think 块有 12px margin,**不跟 turn-body 重叠** ✓

### 3.4 traces

- traces 容器: (0, ~657, 1280, ~40)
- 3 traces 在 padding 24 后开始,水平分布

**重叠检测**:traces 容器之间 16px gap ✓;traces 跟 deck 之间 border-bottom 1 + margin-bottom 8 = 9px 净距 ✓

### 3.5 deck(y: ~705-)

- input-box: (24, ~705, 1088, 48) — 假设 input 1088 宽
- send-btn: (1124, ~705, 48, 48)
- more-btn: (1184, ~705, 48, 48)

**重叠检测**:input ↔ send 12px,send ↔ more 12px,more ↔ deck 右 24px。**全部 ≥ 12px** ✓

- hints: (24, ~785, 1232, ~24)
- 3 hints 间距 8px ✓
- 第一个 hint 跟 deck 左 padding 24px ✓(没有左 padding,直接顶到 deck 内容左沿)

### 3.6 hover 状态 bounding box 变化

逐按键检查:hover 状态没有 transform: scale,没有 padding 变化,没有 width/height 变化 — **所有按键 bbox 保持不变** ✓

唯一变的是颜色(background / color / border-color)和 opacity — 不影响几何。

### 3.7 drop-shadow 溢出

| 按键 | drop-shadow | 影子延伸 | 跟邻居间隙 | 结论 |
|---|---|---|---|---|
| compact-avatar | 0 2px 6px | +6px 下,±0 左右 | topbar 高 64,avatar 顶 12+40+6=58 < 64 | ✓ 容纳 |
| opening-avatar | 0 3px 8px | +8px 下,±0 左右 | opening 底距下一元素(stream 顶)≈ 24px(opening padding 16 + stream padding 16) | ✓ 不溢出 |
| send-btn | 0 2px 4px | +4px 下,+2px 左右 | 跟 more 净 12px,影子只占 2px → 留 10px clear | ✓ |
| more-btn | 0 1px 2px | +2px 下,+1px 左右 | 跟 deck 右 padding 24px,影子只占 1px → 留 23px clear | ✓ |
| win-btn | 无 shadow | — | — | ✓ |

**没有 drop-shadow 溢出到相邻按键的现象** ✓

---

## 4. 边缘场景

### 4.1 1280 宽度下,3 个 trace 装得下吗?

- trace 1(笔记): ~140px
- gap 1: 16px
- trace 2(水): ~32px
- gap 2: 16px
- trace 3(在): ~42px
- traces 容器 padding: 24 + 24 = 48px

**总占用**:140+16+32+16+42+48 = 294px
**可用宽度**:1280px
**富余**:986px(空白在右侧,符合"traces 居左"的视觉)

**结论:装得下,且不影响** ✓

### 4.2 屏幕宽度 800(假设 demo 不支持移动)

shell 是 fixed 1280×800(L117-118),body 居中 + padding 40。如果视口是 800:
- body 容器 = 800,body padding 40+40 = 80,可用 = 720
- shell = 1280,**溢出 560px**(每边 280)
- 出现水平滚动条
- shell 内部所有元素位置不变 → **按键不会重叠**
- 但**最右侧 ~560px 不可见**(包括 right:24 处的 theme-btn)

**结论:demo 层面按键不重叠;但 800 视口下用户看不到 theme 按钮**。r4 已经标注"demo 不支持 mobile 是产品决策,非设计 bug"(L115 critique-r4-final.md),沿用。

**潜在问题**:如果用户硬把 body 改 `overflow: hidden`,会**直接切掉 right:24-1280 区域** — theme 按钮消失,但 archive 还在。这不是 v5 引入的,v1 就有。

### 4.3 顶栏 spacer 死区(用户没问,但 r3→r4 反复点名)

顶栏总占:24(左 pad) + 120(compact) + 12(gap) + **~930 spacer** + 12(gap) + 158(topbar-actions) + 24(右 pad) = 1280

**~930px 死区仍是 v5 的遗留问题**。r4 评分扣 0.1(r3 → r4 把 1100px 死区缩到 ≈400px,但仍有 400px),v5 没继续处理。

**v5 修复 #6(compact block border-right)是标记"avatar+ritual"为一个 block,视觉上"占了一段",但不缩小死区**。死区本质上是 1280 视口 - 顶栏内容 ≈ 930-1000px,**靠继续加内容(顶栏设置/搜索/通知铃)才能根治**,不在本次 6 个修复范围。

---

## 5. v4 → v5 修复确认(7 项,实查)

| # | v4 问题 | v4 状态 | v5 修复证据 | v5 实际 |
|---|---|---|---|---|
| 1 | topbar archive ↔ theme 加 vertical divider | v4 无 wrapper,无 divider,两者是 topbar 直接 flex children(L609 + L619) | v5 加 `.topbar-actions` wrapper(L219-225)+ `.topbar-divider`(L226-230)+ HTML `<span class="topbar-divider">`(L635) | **✓ 真加** |
| 2 | deck input ↔ send ↔ more gap 8→12 | v4 L480 `gap: var(--s2)` = 8px | v5 L493 `gap: var(--s3)` = 12px | **✓ 真改** |
| 3 | more-btn 加 drop-shadow(跟 send-btn 视觉重量一致) | v4 L526-537 无 filter | v5 L548 `filter: drop-shadow(0 1px 2px rgba(80,70,55,.14))` | **✓ 真加** |
| 4 | traces 跟 deck 加 margin-bottom 8px | v4 L435-444 无 margin-bottom | v5 L457 `margin-bottom: var(--s2)` = 8px | **✓ 真加** |
| 5 | hint 跟 input 加 padding-top 4px | v4 L541 `padding: 0 var(--s1)`(顶 0,左右 4) | v5 L560 `padding: var(--s1) 0 0 0`(顶 4,左右 0) | **✓ 真加**(同时把左右 padding 删了,净效果 hints 现在 flush 到 deck 左/右 24px) |
| 6 | avatar 跟 ritual 改 compact block 整体 | v4 L596-598 compact 只含 avatar,ritual 是独立 topbar child(L601-604) | v5 L615-621 compact 含 avatar + ritual,加 padding-right 12 + border-right 1px | **✓ 真改** |
| 7(用户没列,v5 header 列了) | more-btn hover 状态视觉跟 send-btn 区分 | v4 L537 `background: raised, color: muted` | v5 L552-556 `background: raised, color: fg, border-color: border` | **△ 微调**(hover 颜色 muted→fg,加 border-color 变化,但视觉差异仍小) |

**7 项修复:6 ✓ 完全 + 1 △ 微调**。**用户指出的 6 项全部确认实做**。

---

## 6. bonus 发现(用户没问,但 mental view 看到)

### 6.1 顶栏高度 56 → 64(没在 v5 header 修复列表里)

- v4 L169 `height: 56px`
- v5 L168 `height: 64px`
- 净增 8px(提升呼吸空间)

**这是 v4 → v5 的隐藏改动**。avatar drop-shadow 6px 在 56px box 里只留 4-6px 顶/底,在 64px box 里留 12-12px,视觉更稳。**正面改动** ✓

### 6.2 more-btn 跟 send-btn 视觉重量对比

| 维度 | send-btn | more-btn | 一致性 |
|---|---|---|---|
| 尺寸 | 48×48 | 48×48 | ✓ 一致 |
| border-radius | --r-md (14) | --r-md (14) | ✓ 一致 |
| 视觉背景对比 | rep-500 实心橙(强) | surface 浅灰(弱) | △ 故意对比,但有 drop-shadow 平衡 |
| drop-shadow | 0 2px 4px rgba(.22) | 0 1px 2px rgba(.14) | △ send 影子更大更深,但 v5 已加 more 影子 |
| hover 反馈 | background 加深(rep-500→rep-600) | background 浅灰→raised, border-color 加深 | △ 反馈路径不同 |

**评价**:more-btn 加 drop-shadow 后,从"贴在平面上的小方块"变成"微微抬起的方块",跟 send-btn 处于同一视觉平面(都在 z 轴上浮 1-2px)。**修复 #3 达成意图** ✓

### 6.3 v5 顶栏右侧 33px gap(archive-divider-theme)

按用户问法 "8/12?",实际 33px(12+4+1+4+12)比预期宽。但这是 v5 的设计选择:
- 4px divider margin 是为了"divider 不贴按钮边缘",在视觉上独立
- 12px gap + 4px margin = 16px 净距,比 8px 阈值宽 1 倍,反过来看**更显 divider 的"分组"语义**

**评价**:不算瑕疵,只是"如果你想 archive 跟 theme 看起来更紧密"可以收 margin 到 2px(总 12+2+1+2+12 = 29px,仍比 8px 阈值宽 3.6 倍)。

### 6.4 v4 → v5 实际新增的视觉元素

我没找到 v5 新增的视觉元素 — 7 项修复都是**几何/视觉权重的微调**,没有引入新的 UI 元素。这跟 r4 final "v4 是这一轮终点" 的论断一致。

---

## 7. 评分

### 7.1 按键几何布局:9.5 / 10

- ✓ v4 6 项修复全部实做,7 项(含 #7)5/7 完全 + 1/7 接近完全 + 1/7 微调
- ✓ 7 个按键 group 全部满足"不重叠 + 间距 ≥ 8px"
- ✓ drop-shadow 不溢出到邻居
- ✓ hover 状态不改变 bbox,无重叠风险
- ✗ 唯一瑕疵:archive ↔ theme 总间距 33px 比常规 gap 宽 ~2x,**意图合理但偏松**
- ✗ 顶栏 930px spacer 死区仍未处理(不在本次 6 项范围,不算 v5 失分)

### 7.2 视觉呼吸:9.0 / 10

- ✓ deck 呼吸:r4 8px → v5 12px(横向)+ 4px(纵向 hints),v4 评 8.5 → v5 评 9.0
- ✓ traces 呼吸:border-bottom 1 + margin-bottom 8 = 视觉"双锁",从 deck 分离
- ✓ compact block 呼吸:avatar 跟 ritual 圈成一块,border-right 标记 block 边界,顶栏左/右两侧有"对照感"
- ✗ 顶栏中段 ~930px 仍是死区(不是几何 bug,是信息密度问题)
- ✗ active turn padding 16/16/16/20 + 3px border-top + Fraunces quote + think 块,**4 层叠加**(r4 评 9.0,v5 没动)
- ✗ hint 区域从 v4 `padding: 0 var(--s1)`(左右 4)改成 v5 `padding: 4 0 0 0`(顶 4,左右 0),**hints 现在跟 deck 边缘齐平**(在 deck content 左沿),不再缩进。r4 时 hints 缩进 4px,视觉上更"有归属",v5 改后 hints 显得更独立。**这是个意外变化,值得问设计师是有意还是 typo**。

### 7.3 总评:9.3 / 10

v4 在按键几何维度有 6 个真实问题,被 r4 评审员评为"漏看"是准确的(r4 终局 9.5 但没专门查按键 bbox)。v5 把 6 项**全部实做**,mental view 验证后:
- **0 例按键重叠**
- **0 例间距 < 8px**
- **0 例 drop-shadow 溢出**
- **6/6 v4 问题修复确认**

**v5 在几何维度把 v4 的 9.5 拉到 9.8 没有任何问题**,但因为不在评分体系里加几何专项,所以 r5 总评 9.3(几何 9.5 + 视觉 9.0 综合,扣除 hint padding 改动带来的小疑点 -0.1)。

---

## 8. 给设计师的回执

1. **hint padding 改动是无意还是有意的?** v4 `padding: 0 var(--s1)` → v5 `padding: 4 0 0 0` 删了左右 4px。如果是故意让 hints 顶到 deck 左/右边缘,**需要在 commit message 标注**,不然下次评审会以为是 typo 又改回去。

2. **archive ↔ theme 总间距 33px 是设计选择还是 margin 设大了?** 如果想更紧凑,divider margin 改 0 2px(总 12+2+1+2+12 = 29px)。**保留也合理**,因为 1px divider 在 33px 中间位置视觉清晰。

3. **顶栏 56 → 64 的改动不在 v5 header 修复列表里**。建议下次 commit message 显式列出"提升顶栏呼吸 +8px"。

4. **demo 800 视口下 theme 按钮会被切掉**。r4 已说"demo 不支持 mobile 是产品决策",沿用即可。但如果未来要做 responsive,**第一个 breakpoint 建议 ≤ 1280**,把 shell 从 fixed 改 max-width:100% + flex 缩放。

5. **7 项修复中 #7(more-btn hover)差异太小**。muted → fg 在 #6B665B → #38352E 之间,对比度变化 ≈ 2:1,用户不一定感知得到。如果想强化"more-btn 可点"的 affordance,可以加 `transform: translateY(-1px)` 或 `box-shadow` 增量(0 1px 2px → 0 2px 4px on hover)。

---

## 9. 反向检查(我有没有漏看)

按"我可能漏看什么"的反向清单自查:

- ✗ 漏看 active turn 的 3px border-top 跟 padding-top 16 的关系?  —  已查:border 在 padding 之内,box-sizing border-box 决定 border 不外加高度。3px + 13px(剩余 padding) + content。**无重叠** ✓
- ✗ 漏看 think 块的 2.5px border-left 跟 active turn 的 padding-left 20 关系?  —  已查:think 在 active turn 内部,think 自身 padding 12 16,border-left 2.5px 在 think padding 之外侧,跟 active turn 内容左沿对齐(active padding-left 20 > 0,所以 think 缩进 20px)。**无重叠** ✓
- ✗ 漏看 watermark 跟其他元素重叠?  —  watermark 在 left:24, bottom:12, opacity 0.22, pointer-events none。**跟任何 button 都不重叠**(watermark 在左下,bottom:12,最近的 button 是 deck 内的 input-box,deck padding 12 24 16 → input-box 底 = 12(deck 上 pad)+ 48(input min-height) = 60px from deck top; deck top 大约在 y=705,所以 input-box 底 y=765。watermark 在 y=788(svg 11+text 10=21, bottom 12 → 21+12=33) — **在 input-box 下方 23px,无重叠** ✓
- ✗ 漏看 turn.user 跟 turn.agent.active 之间的 8 + 16 + 24 复杂 margin 计算?  —  已查:user.margin-top 8,active.margin-top 16,stream gap 24,合计 user→active = 8+24+16 = 48px(从 user border-bottom 到 active border-top)。**远超 8px** ✓
- ✗ 漏看 traces 跟 stream 之间的间距?  —  trace 跟 stream 边界:stream 没 border,traces 有 border-top 1px。stream padding-bottom 12, traces padding-top 8 = 20px 净距。**符合** ✓
- ✗ 漏看 dark mode 下的几何?  —  CSS 变量切换不影响 spacing/padding/gap/dimensions。只影响颜色。**几何不变** ✓

**没有发现额外的几何问题**。

---

## 10. VERDICT

v5 在按键布局几何维度:
- ✓ 0 例按键重叠
- ✓ 0 例间距 < 8px
- ✓ 0 例 drop-shadow 溢出
- ✓ 6/6 v4 问题修复确认
- ⚠ 1 项 hint padding 改动需设计师确认意图
- ⚠ 1 项 archive↔theme 总间距 33px 偏松(意图合理)
- ⚠ 顶栏 930px 死区非 v5 责任(产品决策)

**几何布局 9.5/10,视觉呼吸 9.0/10,总评 9.3/10。**

VERDICT: PASS
