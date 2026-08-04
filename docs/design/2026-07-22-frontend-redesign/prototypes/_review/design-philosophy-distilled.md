# northing 设计哲学蒸馏(v2)

> 评审用基线文档。从 `README.md` / `JUDGE-CRITERIA.md` / `slint-safe-conventions.md` 及
> 9 份 HTML 原型(`theme-system.html` 范式真值 + `space-view.html` 主页 + 7 个状态页)综合提炼。
> 用作"评分锚"——评审时凡与本蒸馏冲突,即视为失分。

---

## 1. 核心定位(一句话)

**"AI 咨询室"**——一个让 agent 拥有"自我颜色"、有"呼吸"和"沉积"、克制统计数字的安静空间。
不是 dashboard,不是工具栏,不是聊天软件。

## 2. 哲学十戒(评估时逐一对照)

| # | 戒律 | 评估问题 |
|---|---|---|
| 1 | **拒绝 dashboard 美学** | 看起来像控制台,还是像安静的房间?有无"47 turns""API 健康"这种统计数字展览? |
| 2 | **品牌水印化** | northing 商标只出现在角落水印(opacity 0.25),不参与视觉主战场;视觉主体是 agent 头像+名字 |
| 3 | **代表色是 agent 的灵魂** | 暖珊瑚(rep-500)是 agent 自己选的"个性",人类不能改;界面不放置换色控件 |
| 4 | **整屋空气染色** | rep 色不只在按钮/竖线上,而是弥漫在空间(底 3.5% + 顶晕 7%)——颜色"住"在房间里 |
| 5 | **三要素语义域** | 暖(rep 驱力/行动) / 冷(abyss 深渊/思考) / 灰(sediment 沉积/旧)三态对应三色,语义互斥 |
| 6 | **沉降式动效** | 慢(1200ms 一次)、重、向下;呼吸 6s ±1.5% 只给 logo+头像;禁止弹跳/overshoot/spinner/无限循环 |
| 7 | **用户与 agent 边界** | 用户气泡不染 rep 色——rep 是 agent 的色,气泡是用户的话 |
| 8 | **诗意克制** | 装饰不过度,留白有目的,无 emoji,无毛玻璃,无通用紫蓝渐变 |
| 9 | **暖灰基底** | bg `#F4F3F0`(不纯白不纯黑),暗色 `#181612`(深暖黑非纯黑);所有色温走"暖" |
| 10 | **字体三系分离** | Fraunces 品牌/名字、Noto Sans SC 400 正文(不用 300,Windows 太虚)、JetBrains Mono 元数据 |

## 3. 设计 Token 速查(评估扣分依据)

### 3.1 颜色(48 token 双模式)

- **基底**:`bg` `#F4F3F0` / `surface` `#FBFAF8` / `elevated` `#FFF` / `raised` `#EFEDE8` / `border` `#E6E3DD`
- **文本**:`fg` `#38352E` / `muted` `#7B766C` / `faint` `#A8A398`(faint 对比度须 ≥4.0:1)
- **代表色 rep**(暖珊瑚,默认 hue≈30°):`#E5A583` / `#D68A63` / `#C8714C` / `#A85A38` (300/400/500/600)
- **深渊 abyss**(冷青 hue≈185°):`#7AABA4` / `#5A9B93` / `#3F837B`
- **出生 birth**:`#DAD6CF`
- **危险 danger**:`#A45950`(陶红,非纯红)

### 3.2 间距阶梯(4px 基数)

`s1` 4 / `s2` 8 / `s3` 12 / `s4` 16 / `s5` 24 / `s6` 32

### 3.3 圆角阶梯

`r-sm` 9 / `r-md` 14 / `r-lg` 18 / `r-pill` 999 / 窗口 20 / win-btn 7

### 3.4 字号

`fs-sm` 10 / `fs-md` 11.5 / `fs-lg` 13 / `fs-body` 15 / `fs-name` 16

### 3.5 动效

`dur-hover` 150ms / `dur-normal` 350ms / `dur-slide` 250ms / `dur-once` 1200ms / `dur-breathe` 6000ms

## 4. 硬约束(违反即扣分,每条最多 -2)

1. **品牌 logo 不染 rep**
2. **思考块底不染 rep**(用 abyss 冷系)
3. **沉积轮不染当前 rep**(保持褪色灰)
4. **用户气泡不染 rep 边/底**
5. **正文 `.msg` 不染 rep**
6. **禁止 `box-shadow`**(用 `filter: drop-shadow()`)
7. **禁止 `color-mix(in srgb, ...)`**(用预计算 hex)
8. **禁止 `::before` / `::after` 伪元素当主元素**(Slint 翻译限制)
9. **禁止 `@keyframes ... infinite`**(用 `animation-tick()` 驱动)
10. **z-index 不写 `#app>*` 通配**(会压垮 absolute 把手/水印)

## 5. 反 AI 味(可作正向证据)

✓ 暖灰基底 + 珊瑚强调(非紫蓝渐变)
✓ 沉降式(非弹跳式)动效
✓ 头像有呼吸(非死板图标)
✓ 留白有目的(非过度装饰)
✓ 颜色弥漫空间(非纯色块)
✓ 字体三系(非单字体包打天下)
✓ SVG 线性图标(非 emoji)
✓ 角落水印(非满屏 logo)

## 6. 9 份原型文件清单(评审范围)

| 文件 | 页面 | 评审重点 |
|---|---|---|
| `theme-system.html` | **范式真值** | 6 套代表色 + 亮/暗 + 整屋染色 + token 全景 |
| `onboarding.html` | 首次启动 | 四字段 + 五色板 + 诞生时刻;**人类唯一可改色入口** |
| `empty-state.html` | 空态 | 居中在场区 + 开场白 |
| `chat-collapsed.html` / `chat-expanded.html` | 对话 | 活跃轮竖线 + 用户气泡 + 思考块 |
| `space-view.html` | 空间(多会话) | 会话卡片网格 + 沉积褪色 + 新建房间感 |
| `settings-general.html` ~ `settings-access.html` | 设置 5 屏 | 淡档染色 + 字段规范 |
| `archive.html` | 档案馆 | 冷雾 only(深海青)+ 时间轴 + 只读氛围 |
| `identity-creator.html` | 身份创建 | 衍生品(待确认) |
| `shared/tokens.css` | 设计 token 源 | 评估 token 化程度 |

## 7. 评估输出建议(给 agent 的格式要求)

### 7.1 乔布斯视角(jobs-design-assistant)

按 SKILL.md 要求的格式:
- 整体判断:优秀/合格/不及格/需要重构
- 最大问题(最多 3 条):问题 + 为什么 + 修复方向
- 精简建议(最多 5 条):动宾结构,可立刻执行

### 7.2 量化视角(JUDGE-CRITERIA.md)

按 D1(40%) / D2(30%) / D3(20%) / D4(-10% 扣分)四档打分,最后合成总分。要求:
- 每档给具体证据(行号 / token / 文件名)
- 列出 D4 命中/未命中的扣分项
- 给出总分及是否过 9.0 达标线

### 7.3 共同要求

- 不给套话("还不错""见仁见智")
- 批评要具体到元素/行
- 表扬要具体到做对了什么
- 跨原型对比(哪个页做得好,哪个页失分)
