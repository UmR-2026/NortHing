# Handoff — consult-room 前端方向（2026-08-03 收工，明日继续）

## 当前状态

- 方向定稿：**consult-room**（有界诊室 + 脱离轨道 + 双光学 + mind 五色 + 8s 生物态呼吸 + 膜结触发器）。
- 规格集齐 `docs/design/2026-07-22-frontend-redesign/consult-room/`：
  - `consult-room-main.html` = 唯一视觉真值（send/stop 合一、中枢胶囊可收纳、膜结、色板已撤、收纳钮并入窗控簇）
  - 四面板 v2：`consult-room-{onboarding,settings,archive,space}-v2.html`
- **待用户终裁**（明日入口）。终裁焦点：呼吸节奏、亮色无菌室、膜结存在感、中枢折叠态、approval 卡宽度节奏。

## 已记录、留给前端建构期的修复

- 右膜结 `bottom:230px` 静态值 → Slint 化改跟随抽屉的表达式。
- approval 两卡宽度不齐（可统一节奏）；witness 标记可选加更克制的小记。
- 亮色态整体再扫一眼（色点撤除后右结深墨渗光在亮底的表现）。
- Slint 词汇：breathe→opacity、mind 25 token 已在 palette、i18n 生成、双 compile 顺序（见 capability-notes）。

## 明日顺序

1. 用户终裁五页套 → 记录改动、合入真值。
2. 逐页 Slint 建构（spike 词汇表备好），shot-window 截图循环验收。
3. settings 压力页若终裁有变，先修规格再建构。

## 2026-08-03 夜间追加（收工前决策）

- 主题色三档获批并铺：缝线 16% mind 色（main/onboarding/space）、流式整屋升档（body.speaking）、agent 代词着色。**分段胶囊编年史被否（突兀）**。
- 编年史定稿形态：平滑渐变条（尖角、4px、opacity .7），历史色按龄褪向出生灰，右端 ≡ --mind-base 同源；双击演示换色（agent 自主，人不可改）。
- **头像方形化**：main .agent-avatar / onboarding .agent-avatar / archive .depth-marker / space .door-lamp 全 radius 0。近尖角语言全局确认。
- settings 方向：「它的自我」mind 着色 / 「设施」中性分治，待细化。

## 哲学红线复述（加页/建构时勿犯）

代表色是 agent 的灵魂（人类仅 onboarding 可改）；拒绝 dashboard 数字；诗意<功能；品牌水印化；呼吸 8s 单钟、振幅分级（主体>边界>结构）；rep 只属 agent（用户/见证者侧不染）；**近尖角语言**（头像/条/ pill 皆尖角，极小圆点除外）。
