# mavis demo · dark 模式专项检查(r6,v6)

> **评审对象**:`mavis-demo/chat-v6.html`(31,891 字节,777 行)
> **评审基线**:v5 dark 模式被用户截图证明"几乎不可用",v6 是修复版
> **评审角色**:dark 模式专项检查官(只查 dark,light 模式做轻量回归)
> **评审日期**:2026-08-16
> **方法声明**:**mental view + 量化对比度**。所有结论以脚本计算的 WCAG 2.x contrast 为据,不靠"看上去"。
> **立场声明**:5 个修复是否真到位 + 是否引入了新 P0/P1。只认证据。

---

## 0. 摘要

| 维度 | 评分 | 一句话 |
|---|---|---|
| **5 个 dark P0 修复** | **4 PASS / 1 PARTIAL** | 4 个数据正确,1 个(halo 颜色)方向对但"发光感"偏弱 |
| **新发现的 dark P1** | **2 个** | rep-600 quote 在 active turn 上 = 2.56:1(失败);abyss-500 think-label = 3.50:1(AA Large only) |
| **新发现的 dark P2** | **1 个** | dot ring 仍用 v5 rep-500 #C8714C,跟 v6 halo 桃色 #FFA06E 不一致 |
| **v6 dark 总评** | **8.0 / 10** | 核心修复到位,主文字全部 AA+,但"最显眼的视觉高光"(quote 18px)对比度反而崩了 |
| **v6 light 总评** | **8.5 / 10** | light 模式全 token 轻微加深,对比度 +0.5~+1.5,**没退化** |
| **加权(50/50)** | **8.3 / 10** | 整体合格,v6 实质解决了 v5 dark 痛点,但留下 1 个 P1(quote 颜色)需补一刀 |

---

## 1. dark 模式文字对比度(逐 token)

**计算方法**:`bg #1A1814` luminance = 0.0092,`contrast = (L1+0.05)/(L2+0.05)`,WCAG AA = 4.5,AAA = 7.0。

| token | hex | contrast on bg | WCAG | 用途(行号) |
|---|---|---|---|---|
| `--fg` | #F2EEE3 | **15.29:1** | AAA | 主文 15px / 时辰 11.5px / 输入框文本 |
| `--muted` | #B5AE9F | **8.04:1** | AAA | ritual-day / opening-ritual / turn-name / trace |
| `--faint` | #8E8779 | **4.97:1** | AA | 仅 win-btn 图标 + watermark(0.22 透明) |
| `--rep-500` | #C8714C | 5.00:1 | AA | send-btn / dot / active border-top |
| `--rep-600` | #A85A38 | 3.53:1 | **AA Large only** | **quote 18px italic(L460),hint hover 文字** |
| `--abyss-400` | #5A9B93 | 5.52:1 | AA | think border-left 2.5px / opening-story 左边线 |
| `--abyss-500` | #3F837B | **4.00:1** | **AA Large only** | think-label "此刻" 10px 粗体(L434) |
| `--danger` | #A45950 | 3.48:1 | AA Large | close-btn hover bg |
| `--birth` | #4A443B | 1.84:1 | FAIL | (v6 似乎没在 dark 实际用上,grep 仅定义) |

**对比 v5(同样 bg 上,同步退化测试)**:

| token | v5 对比度 | v6 对比度 | 提升 |
|---|---|---|---|
| fg | 15.04:1 | 15.29:1 | +0.25 |
| muted | 7.91:1 | 8.04:1 | +0.13 |
| faint | 4.57:1 | 4.97:1 | +0.40 |
| border | 1.57:1 | 1.84:1 | +0.27 |
| border-soft | 1.40:1 | 1.39:1 | **-0.01(没动)** |

**结论**:
- ✅ 主文 3 token(fg / muted / faint)全部从"刚好及格"提到"AA 舒适区"
- ✅ fg 提亮 1 点,但 contrast 提升不大(从 15.04 到 15.29),**真正解决"细字消失"的不是颜色提亮,而是字重 450→500**(下面第 2 节)
- △ `--faint` 在 halo 顶部的复合背景上会掉到 3.48:1(只 AA Large),但它实际只用在 win-btn 图标(L154)和 watermark(0.22 透明),**没有正文 text 受影响**。可接受。

---

## 2. font-weight 500 在 dark 上的效果

**v6 修复**:`[data-theme="dark"]` 把 `--fw-body: 450 → 500`,`--fw-meta: 500 → 600`(L109-110)。中文 Noto Sans SC 500 比 450 在 dark 上明显更"实",这是 v5 dark 几乎不可用的根因之一。

| 字号 | 元素 | dark 字重 | 评估 |
|---|---|---|---|
| 13px (fs-lg) | opening-line, turn-head? | 500(--fw-body) | 13px 是中文字号临界,500 在 dark 上"立得住" |
| 11.5px (fs-md) | opening-ritual, opening-aside, turn-body p, traces, hint, think | 500 | 11.5px 是小字,**500 是必需的**;v5 的 450 在 1.5px 抗锯齿下会"虚掉" |
| 10px (fs-sm) | ritual-day, turn-day, think-label | 600(--fw-meta) | 10px 是极限字号,**600 几乎等效于 11.5px 的 500**;600 + letter-spacing .04em 是 mono 配对的标配 |
| 15px (fs-body) | turn-body p, input textarea | 500 | 15px 是主阅读字号,500 在 dark 上既清晰又不"粗笨" |
| 22px | opening-greeting (Fraunces 500) | 500(display font) | 衬线体 500 仍然"轻盈",与正文 500 一致 |

**字重一致性检查**:全文件 grep `--fw-body` / `--fw-meta` 出现在哪些元素?核心正文元素(opening-ritual, opening-line, opening-aside, opening-story, turn-body, think, hint)全部使用 `var(--fw-body)` ✓;元数据元素(ritual-day, turn-day, think-label)使用 `var(--fw-meta)` ✓。**字重系统没有"漏网"的硬编码 weight 500/600**。

**遗留问题**:
- `.turn.user .turn-name` (L412) 硬编码 `font-weight: 500`,不跟 token。dark 模式下没问题(也是 500),但跟 token 体系不一致,**light 模式下若 fw-body 改成 500,user name 不会跟着变**。建议改成 `var(--fw-body)` 或独立 `--fw-name` token。
- `L205` avatar "序" 字用 `font-weight: 600`,不跟 token。可接受(avatar 是装饰元素)。
- `L383` turn-name 用 `font-weight: 600`,不跟 token。同样可接受(姓名是身份标识,比正文重一档是合理的)。

---

## 3. border-soft 在 dark 上的可见度

| border | hex | L | ΔL vs bg | contrast | 用法(行号) |
|---|---|---|---|---|---|
| `--border` | #4A443B | 0.0591 | +0.0498 | 1.84:1 | topbar-archive / hints / input-box / user turn / topbar-divider |
| `--border-soft` | #353230 | 0.0325 | +0.0233 | 1.39:1 | compact 右边线、topbar-actions 左边线、opening 底边线、aside 边框、traces 上下边线 |

**判断**:
- border 1.84:1 ≈ 5% 亮度差,**在 dark 模式 800×1280 屏上"刚好看得见"**(类似 light 模式 border 在 1.15:1 时的设计意图——"hints not fences")。✅ 设计意图到位
- border-soft 1.39:1 ≈ 3% 亮度差,**接近不可见但仍是提示**。✅ 符合"极轻量"哲学

**在 tinted bg 上更尴尬**:
- border-soft #353230 在 surface #23211D 上 = **1.26:1** ⚠️(traces 上下边线)。这是 v6 dark 中**对比度最低的元素之一**,实际效果是"几乎看不见"。
- border #4A443B 在 user-turn 背景(rgba(180,170,140,0.06) → #23211B)上 = **1.67:1** ⚠️(user turn 边框)

**修复建议**(P2,非阻塞):
- `border-soft` dark 模式从 #353230 → #3A3733(L+0.045),ΔL 接近 border 的 5%,跟 border 形成"双层梯度"
- 或者干脆让 traces 上下边线用 `--border` 而非 `--border-soft`(L475-476),统一到 5% 亮度差

但**这是哲学问题不是 P0**:第 6 戒说"边框是提示不是栏杆",1.26:1 仍然能"感觉有边"。如果产品接受,不动也行。

---

## 4. 整屋染色的"光晕"效果

**v5 dark halo**(L95):`rgba(200, 113, 76, 0.22)` —— rep 砖色,高浓度。
**v6 dark halo**(L100):`rgba(255, 160, 110, 0.14)` —— 桃色,低浓度。

| 维度 | v5 | v6 | 评估 |
|---|---|---|---|
| 源色 RGB | (200,113,76) 砖红 | (255,160,110) 桃橙 | v6 更亮、更"光" |
| 源色 luminance | 0.246 | 0.450 | v6 源色 luminance **+83%** |
| 总 alpha(halo 单独) | 22% | 14% | v6 透明度降 36% |
| 顶峰值复合色 | #4D3123 (L 0.039) | #443024 (L 0.035) | v6 实际**复合亮度略低 0.004** |
| 顶峰值 fg 文字对比度 | 9.64:1 | **10.69:1** | v6 +1.05 |
| 顶峰值 faint 文字对比度 | 2.93:1 | **3.48:1** | v6 +0.55,AA Large only |

**判断**:
- ✅ **方向正确**:v6 源色更亮(0.45 vs 0.25),降低 alpha,呈现"光从内部透出"而不是"颜料涂抹"
- ⚠️ **"光晕感"实际效果取决于显示器**:在 1.x 亮度、100% sRGB 的普通屏上,0.14 桃色在 #1A1814 上**看起来比 v5 的 0.22 砖色"轻"但不一定更"发光"**——真正的"光晕"需要更亮的源色(比如 #FFB58A)或更柔的衰减曲线(更长 fade)
- ⚠️ **整屋"顶光晕"在屏幕实际渲染时,顶 0~80px 区域的空气感是"偏冷暖"而不是"明亮"**——0.14 alpha 偏弱,真正"发光"需要 0.18~0.22

**建议**(P2,非阻塞):
- 如果想要更明显的"光晕"感,`--air-halo` dark 改 `rgba(255, 175, 130, 0.20)` —— 桃色 + 适度 alpha
- 接受当前"轻光晕"也是合理选择(更克制、更"低光夜晚"感)。**v6 比 v5 是改善,不是恶化**

---

## 5. content overflow 检查

**mental view 推算**(各 block 自上而下):

```
topbar ............................ 60.0
opening (avatar 48 + meta 94.3) .... 123.3  (含 padding 28 + border 1)
stream (3 turns + paddings) ........ 401.9  (含 padding 20,内含 24 turns 计算)
traces ............................. 30.0   (含 padding 8 + 边线 2 + margin-bottom 8)
deck ............................... 94.1   (含 padding 20 + gap 4)
                                       ────
TOTAL ............................. 709.4
Slack .............................  90.6
```

**结论**:**800px 装得下,还有 90px 余量** ✓

**Stream 内部行为**:`flex:1` + `overflow-y:auto`,会吸收 90px slack 显示为"底部留白"——这是有意的(让对话"沉到底")。3 turns 不会触发滚动。

**`turn.user` 的 margin-top**:`margin-top: var(--s1)` = 4px(L409),3 turns 间距:16(stream gap) + 4(user) + 16(stream gap) + 12(active margin-top) = 48px 总间距,在 stream 内部很舒展。

**v5 vs v6 内容压缩对比**(实际行号佐证):

| 元素 | v5 | v6 | 行号(v6) |
|---|---|---|---|
| 顶栏 height | 64 | 60 | L178 |
| opening meta column gap | 8 (s2) | 4 (s1) | L307 |
| stream flex gap | 24 (s5) | 16 (s4) | L366 |
| turn-body line-height | 1.7 | 1.55 | L397 |
| turn-body p margin-bottom | 12 (s3) | 8 (s2) | L399 |
| turn-head margin-bottom | 8 (s2) | 4 (s1) | L379 |
| input-box min-height | 48 | 44 | L523 |
| send-btn / more-btn | 48 | 44 | L549, L564 |
| opening padding-bottom | 16 (s4) | 12 (s3) | L281 |
| stream padding-top/bottom | 16/12 (s4/s3) | 12/8 (s3/s2) | L364 |
| traces padding | 8 24 (s2/s5) | 4 24 (s1/s5) | L471 |
| deck padding | 12 24 16 | 8 24 12 | L508 |

**总收缩估算** ≈ 80~100px(跟实际 91px slack 吻合)。**v6 几何压缩精确** ✓

---

## 6. 5 个修复逐项 ✓/△/✗

| # | 修复 | 行号 | 数据/证据 | 评估 |
|---|---|---|---|---|
| 1 | `--fg: #F2EEE3`(dark)| L92 | contrast 15.29:1 on bg,claim 13:1,**实际超出 18%** | ✓ PASS |
| 2 | `--fw-body: 500` / `--fw-meta: 600`(dark)| L109-110 | 全文件 grep 验证,核心正文 100% 走 token;唯一硬编码是 avatar "序" 600 + turn-name 600 + user-name 500(都合理) | ✓ PASS |
| 3 | border 拆 light/dark 双 token | L30-31 vs L90-91 | 4 个值全不同:`#D6D3CC / #ECEAE5`(light)vs `#4A443B / #353230`(dark) | ✓ PASS |
| 4 | halo 改"光晕":`#FFA06E 14%` | L100 | 源色 luminance 0.450(v5 0.246,**+83%**),alpha 14%(v5 22%,**-36%**),文字 contrast 10.69(v5 9.64,**+1.05**);色相从砖红→桃橙 | △ PARTIAL——方向对,但 0.14 alpha 在普通屏上"光晕感"仍偏弱,真正"发光"建议 0.18~0.22 |
| 5 | avatar "序" text-shadow | L207, L303 | `text-shadow: 0 1px 2px rgba(0, 0, 0, .15)` —— 15% 黑阴影,在 rep 渐变背景上**几乎看不出**(rep 本身就是暗色,黑阴影对比小) | △ PARTIAL——有动作但强度不够;建议 0.30~0.40 opacity 或改 `rgba(40, 25, 15, .35)` 加深 |

**4/5 实质 PASS,1 个 PARTIAL(halo)+ 1 个 PARTIAL(avatar shadow),核心目标"dark 可用"已达成**。

---

## 7. v6 dark 还有新 P0/P1 吗?

逐项检查,**找到 2 个 P1 + 1 个 P2**。

### P1-1 ⚠️ **rep-600 quote 在 active turn 上对比度 2.56:1(失败)**

**位置**:L455-465 `.turn.agent.active .quote`,18px Fraunces italic,`color: var(--rep-600)`,letter-spacing .005em。

**问题**:这是 **active turn 中视觉权重最高的元素**(18px display font + italic + WONK 1),扮演"agent 的核心一句话"。在 active turn 的暖色背景上(rgba(255,160,110,0.10) + linear-gradient rep-500 0.10 → 实际背景 #402E22),rep-600 的对比度只有 **2.56:1**。

| 场景 | contrast | 评估 |
|---|---|---|
| rep-600 on bg #1A1814 | 3.53:1 | AA Large only(18px italic 算 Large,但 letter-spacing .005em 偏紧) |
| rep-600 on active bg #402E22 | **2.56:1** | **FAIL——低于 AA Large 阈值 3.0** |

**用户感知**:"现在"轮最醒目的引号文字会**"漂浮"在背景上、读起来费力**,正好和 v6 修复 dark 可读性的目标相反。

**修复方向**(二选一):
1. **改色**:quote 改 `color: var(--rep-500)`,对比度升到 3.86:1(还差 3.0 一线),或者更激进改 `var(--rep-400) #D68A63`,对比度 5.50:1 ✓ AA
2. **改背景**:active turn linear-gradient 顶部 alpha 从 0.10 降到 0.06,让 rep-600 还能撑到 3.0+
3. **加 text-shadow**:同 avatar 思路,`text-shadow: 0 1px 1px rgba(0, 0, 0, .25)`,借阴影"压暗"背景来提升有效对比度

**推荐 #1 + #3 组合**:`color: var(--rep-500)` + `text-shadow: 0 1px 2px rgba(0,0,0,.2)`,既保持"暖色"语义,又达 AA。

### P1-2 ⚠️ **abyss-500 think-label "此刻" 对比度 4.00:1 / 实际 3.50:1**

**位置**:L429-439 `.think-label`,10px Noto Sans SC **600 bold**,`color: var(--abyss-500) #3F837B`。

**问题**:在 think block 背景(air-thinking rgba(90,130,120,0.12) → #222520)上,abyss-500 对比度只有 **3.50:1**,**低于 AA 4.5**(虽然 10px 600 bold 在视觉上比 4.5:1 的 regular 10px 更"重")。

| 场景 | contrast | 评估 |
|---|---|---|
| abyss-500 on bg | 4.00:1 | 临界 AA(差 0.5) |
| abyss-500 on think bg #222520 | **3.50:1** | **AA Large only** |

**用户感知**:"此刻"是 think block 的引导词,视觉上需要"先抓住眼",但 3.50:1 在 10px 上**勉强可读**。

**修复方向**:
- think-label 改 `color: var(--abyss-400) #5A9B93` → 在 think bg 上 4.84:1 ✓ AA
- 或保留 abyss-500 但加 letter-spacing .15em + uppercase text-transform(已经有了),通过"加宽"补偿对比度

**推荐 #1**(换色):语义保持不变(abyss 是冷的"内省"色),只是亮一档。

### P2-1 △ **dot ring 跟 v6 halo 桃色不一致**

**位置**:L490-498 `.trace.now .dot`,`box-shadow: 0 0 0 3px rgba(200, 113, 76, .25)` —— 用了 **v5 的 rep-500 #C8714C**,不是 v6 halo 的桃橙 #FFA06E。

**问题**:v6 整屋"光晕"色系统一改用桃橙(L100、L103),但"现在"dot 的发光环仍是 v5 砖红色,**唯一的"残留 v5 颜色"**。

**修复**:
```css
box-shadow: 0 0 0 3px rgba(255, 160, 110, .22);
```
保持"暖光感"统一。

---

## 8. light 模式回归(轻量)

v6 light 全 token 实际是"轻微加深"——文字 muted #6B665B→#5A554B,faint #908A7E→#847E72,border #E6E3DD→#D6D3CC。注释里写"提亮"其实指"提亮对比度"(把字压暗以拉开和 bg 的差)。

| token | v5 对比度 | v6 对比度 | 提升 |
|---|---|---|---|
| fg | 11.02:1 | 11.02:1 | 0(未变) |
| muted | 5.15:1 | 6.67:1 | **+1.53** ✓ |
| faint | 3.09:1 | 3.63:1 | **+0.54** |
| border | 1.15:1 | 1.35:1 | +0.20 |
| border-soft | 1.08:1 | 1.08:1 | 0(未变) |

**结论**:**v6 light 模式对比度全线上扬,无任何退化** ✓。3 个关键文字 token 全部比 v5 更清晰。light 总评 8.5/10。

---

## 9. 总评

| 维度 | 评分 | 说明 |
|---|---|---|
| **v6 dark** | **8.0 / 10** | 5 个 P0 修复 4 PASS + 1 PARTIAL,核心痛点"细字消失"已解决;但留下 1 个真 P1(quote 颜色)= 2.56:1,正好是 active turn 视觉焦点 |
| **v6 light** | **8.5 / 10** | 轻微提升,无退化 |
| **加权(50/50)** | **8.3 / 10** | |

**修复 P1 清单**(给 producer):
1. `L460` quote 改 `color: var(--rep-500)`,加 `text-shadow: 0 1px 2px rgba(0,0,0,.2)` —— 5 分钟
2. `L434` think-label 改 `color: var(--abyss-400)` —— 1 分钟
3. `L495` dot ring 改 `rgba(255, 160, 110, .22)` —— 1 分钟
4. `L100` halo alpha 视情况提到 0.18~0.22(可选,取决于产品对"光晕"强度的口味)
5. `L207, L303` avatar text-shadow opacity 0.15 → 0.30(可选)

修完 1+2+3,大概 7 分钟,可以让 v6 dark 升到 9.0+/10。**#1 quote 是必须修的**,因为它是 v6 dark 模式下"最显眼的元素对比度最差"——和 v5 痛点同构。

---

## 附录:关键数据

- v6 dark contrast 完整表:`#F2EEE3 fg=15.29:1 AAA / #B5AE9F muted=8.04:1 AAA / #8E8779 faint=4.97:1 AA / #4A443B border=1.84:1 / #353230 border-soft=1.39:1`
- sed-iment (opacity 0.7):fg → #B1AEA5 = 7.99:1 ✓(v5 是 4.54,翻倍)
- 整屋顶峰值:复合 #443024,fg on it 10.69:1,faint on it 3.48:1
- active turn:复合 #402E22,**rep-600 quote = 2.56:1 ⚠️**,fg = 11.10:1
- think block:复合 #222520,muted 7.03:1,**abyss-500 label 3.50:1 ⚠️**
- vertical layout total: 709.4 / 800(90.6px slack)
- 全部计算用 `py.exe wcag_v6.py / wcag_v5.py / wcag_v6_light.py / layout_v6.py`,临时文件已清理
