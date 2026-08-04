# space v2 报告 — 2026-08-03 · step-explore

**继承清单核对**（真值 consult-room-main.html 逐条）：① chrome 全对齐——标题栏废除、状态行左 brand-inline 真 logo SVG + "会话空间 · 走廊"、窗控四键入主体右上（实测 968..1086 与状态行文字不再碰撞）、containment inset 12 / membrane-frame inset 13 双边界、room-fog 140px 底雾；② deck 合一按钮（空闲 ➤ 开门 / 进行中 ■ 中止，`.send.streaming`），无独立 HALT，见证注 `margin-left:auto` 右对齐（实测右缘与输入行齐）；③ 中枢 = hall-head 可收纳胶囊（▴ / 折叠转横排），但不设顶晕（见"最大一注"）；④ 抽屉 = membrane-node 双结，左结骑左缘且 JS 量测锚在亮门门灯中心线（--gem-mid 实测 221px，节中心 248 = 灯心 247），右结用 --node-right 背景相反色、y 487..551 落在门缝所见泊位（实测 ante 313..782）内，两模块默认 mod-hidden、station-head 可拖移；⑤ 呼吸复用真值 5 个 keyframes、`animation:…infinite` 计数 6（与真值同数），门灯满幅 / 膜线低幅 / 结构恒稳；⑥ 五色切换器不入产品 UI（仅留 devtools `setColor()`）。

**页面三要素**：① 走廊门语法——亮门独占 rep（accent 边 + 门缝漏光 + 门灯 8s 满幅呼吸），暗门中性无光晕无动画，沉积门冷中性 l1/l2/l3 opacity 0.72/0.52/0.36 阶梯内缩、只读禁点亮；② 门即会话——点门 = 灯移门（旧门牌改"刚刚熄灯"、灯芯 序→◦），aura 锚随亮门门灯移动，走廊唯一光源永远只有一处；③ 新房 = 开一间——deck 位对位诊室操控台，"新房会带着它现在的沉积开门"。

**戒律自检**：rep 只属 agent（暗门/沉积门/走廊自身皆无 rep 色，门缝所见明写"沉积属于房间，不属于此刻的它"）；无 dashboard 数字（计数全叙事化："一间亮着 / 三间熄灯"）；无 emoji（符号集 ☀☾─□✕▴▾×≡⌗➤■↗◦· 全为真值/v1 已用字符）；阴影仅真值同款 4 型（`var(--shadow)` / `+var(--lift)` / `0 0 26px var(--mind-glow)` / `0 0 0 1px var(--line)`）；无 backdrop-filter，color-mix 规则数 7 = 真值 7（零新增）。

**验证**：`node --check`（内联 JS）通过；标签开闭配平（div 110/110、span 43/43、button 22/22）；Edge headless 实测三态截图 + rect 量测：暗态默认 `space-v2-dark.png`、双抽屉开 `space-v2-open.png`（mind 82..362 / corridor 378..1158 / ante 1174..1494，无重叠）、亮态 `space-v2-light.png`（白昼无菌室成立：无光晕、线边界、深铜口音），均在 `C:\Users\UmR\AppData\Local\Temp\opencode\`。

**最大一注**：真值 §1.3 的 room-head「顶晕染色」在走廊里主动让位。走廊没有 agent 头像——它只住在亮着的那间房里，若中枢再点一盏顶灯，页面特则"亮门独占光源"当场失效；故 hall-head 只继承可收纳胶囊 + 状态 pill 两件语法，radial 顶晕整条不写，唯一光源留给亮门门灯（aura 也锚在那里）。这是特则压制通则的取舍，非漏抄。

**遗憾**：① 亮色无菌室下门缝漏光（`box-shadow` 取消）只剩 2px 实色线，"漏光"比喻损失约一半——沿 v1 未解；② 门缝所见默认隐藏后，悬停暗门更新的摘要在抽屉未开时无处可见（真值抽屉纪律与 v1 常驻浮站的固有冲突，选了服从真值）；③ 底雾覆到走廊尾部，为保"档案馆"按钮可读加了 78px 尾距，是真值 chat-flow 未处理的边角，属新增一行而非照抄。
