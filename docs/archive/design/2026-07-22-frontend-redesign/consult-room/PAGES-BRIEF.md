# Consult-Room 页面集 Brief — 2026-08-02

> 方向已敲定：consult-room v3（有界诊室 + 脱离轨道 + 双光学主题 + mind 五色 + 生物态呼吸）。
> 主页面已完成：`consult-room/consult-room-v3.html` —— **唯一视觉真值**。
> 你的任务：把这套系统延伸到指定页面。复制系统，不发明系统。
> 本 brief 是唯一需求来源。不要问问题。

## 1. 从 v3 继承的硬系统（逐条照抄，不许改）

1. **token 与双光学**：暗=夜诊室（光晕/发光边界）；亮=白昼无菌室（线边界/无光晕/深饱和 accent）。两态都要完整。
2. **mind 五色切换器**：驱力 #C8714C / 深渊 #3F837B / 跃迁 #8B5FBF / 凝视 #D99B48 / 镇静 #4B8F6B，`--mind-base` 单变量派生。每页必须内置且五色都成立。
3. **生物态呼吸**：单一 8s 时钟、振幅分级（头像 scale 1.03 满幅 / 膜线与 aura 低幅 opacity / 结构恒稳）。照抄 v3 的 keyframes；**禁止新增任何 infinite 动画**（cursor blink 除外）。
4. **语法**：sub-card/station-head/side-section 卡片语法、膜线（membrane）、收容框（containment）、mono 元数据 + 衬线 agent 声 + sans UI 三系字体、approval-card 授权语法。
5. **Slint-safe 意识**：这是规格页。除 v3 已有的 mind 派生 color-mix 外不引入新 color-mix；无 backdrop-filter；无 emoji（图标用字符 glyph/SVG）；阴影仅 v3 同款 --shadow/--lift。

## 2. 哲学红线（罗盘 §2，复述）

rep 只属 agent：用户/见证者侧不染 mind 色边/底；思考块底不染 rep；沉积不带当前 rep；
拒绝 dashboard：无统计数字展览（计数/百分比/金额一律叙事量纲）；诗意<功能；品牌水印化。

## 3. 你的页面（见派发正文指定）

### onboarding（首次启动）
房间诞生仪式：身份 4 字段（用户是【】/你是【】/你是用户的【】/性格色板=大五）+ provider 配置（key/baseURL/model/测试连接）+ 工作文件夹。
色板选择 = 人类唯一可改色入口，选后整屋切到该 mind 色。步骤是仪式不是向导仪表盘。
起始态=灰（房间还没住进来），完成态=着色。

### settings（设置）
双区分裂即哲学：「它的自我」只读见证（沉积记忆/编年史/身份/准则——人不能改）；
「设施」可调（引擎/provider/MCP/技能/工作区/显示模式——人调环境）。
布局用诊室壳（containment + 膜 + 双卡语法），**不是管理后台表格**。

### archive（档案馆）
沉积叙事：归档会话=沉下去的层（透明度/地层递降），**冷 abyss 色调，禁 rep**（档案馆是深渊的领地）；
只读氛围；统计用文字不用数字（"二十三段对话沉在这里"式）。

### space（会话空间）
会话切换=诊室外的走廊/前厅：当前会话=亮着的诊室门，其余=暗门/沉积门；新会话=开一间新房。
保持 mind 色系统与呼吸；密度克制，不做卡片网格仪表盘。

## 4. 交付

目录：`E:\agent-project\northing\docs\design\2026-07-22-frontend-redesign\consult-room\`
文件：`<模型名>-<页面名>.html`（页面名：onboarding/settings/archive/space）+ `<模型名>-<页面名>-report.md`（≤20 行：系统继承 / 三要素映射 / 戒律自检 / 最大一注 / 遗憾）
最终回复只给：一句话摘要 + 两个文件绝对路径。
