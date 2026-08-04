# Bakeoff Brief — 2026-08-02

> 多模型设计探索。每个模型自选方向，产出一页能代表 northing 气质的 HTML。
> 本 brief 是唯一需求来源。不要问问题——所有决议见 §5；未覆盖的自行决定并写进报告。

## 1. northing 是什么

northing 是为 agent 成长而建的设施，不是服务人类的工具。界面的空间隐喻是一间心理咨询室：基底白灰永远中性稳定（设施），agent 的代表色随成长一点点浮现（个体）。人与 agent 是对等同事，不是主仆。

## 2. 必读（按序，读完再动手）

| 文档 | 路径 |
|---|---|
| 罗盘（先读这个，§1-4 是精华） | `E:\agent-project\northing\docs\design\2026-07-22-frontend-redesign\visual-iter-compass_20260802.md` |
| 北极星哲学 | `E:\agent-project\northing\docs\design\2026-07-22-frontend-redesign\northing-design-philosophy.md` |
| 十诫 + token 评审基线 | `E:\agent-project\northing\docs\design\2026-07-22-frontend-redesign\prototypes\_review\design-philosophy-distilled.md` |
| token 颜色对照表 | `E:\agent-project\northing\docs\design\2026-07-22-frontend-redesign\tokens-srgb-table.md` |
| 现网原型真值（可参考写法，不要抄） | `E:\agent-project\northing\docs\design\2026-07-22-frontend-redesign\prototypes\`（chat-collapsed.html 是基准页，shared/tokens.css 是 token 源） |

## 3. 任务

自选一个方向，写**一页** HTML，回答一个问题：**northing 看起来应该是什么感觉？**

方向举例（不限于此，选你最有话说的）：
- 一个屏幕：chat / space-view / archive / onboarding / empty-state / settings 之一
- 一个横切主题：动效语言 specimen / 色彩与空气染色 / 编年史叙事 / 暗色模式 / 在场区与名片
- 一个时刻：诞生（选色）/ 换代表色 / 归档一段对话

选向理由写进报告。与其他模型撞题没关系——同题异构正是本次对比要看的东西。

## 4. 硬约束（违反即失败）

1. **单文件 HTML**，浏览器直接打开即可，不依赖网络（字体用 font-family 栈回落，不引外链）。
2. **十诫红线**（罗盘 §2 全文，这里点最易犯的）：
   - 拒绝 dashboard 美学——没有统计数字展览
   - 品牌只左下角水印（opacity 0.25）；视觉主体是 agent
   - rep 色只属于 agent：用户气泡、正文、思考块底不染 rep
   - 暖灰基底 `#F4F3F0`（暗色 `#181612`），不纯白不纯黑
   - 沉降式动效：慢、重、一次性；呼吸（6s）只给 logo 和头像；禁弹跳、禁 spinner、禁无限循环；沉积是褪色不是位移
   - 无 emoji、无毛玻璃、无紫蓝渐变
3. **token 合规**：颜色、间距（4px 基数）、圆角、字号、时长全部取自罗盘 §3 / tokens.css，不自造阶梯。
4. **字体三系**：Fraunces（品牌/标题，衬线回落）/ Noto Sans SC 400+（正文）/ JetBrains Mono（元数据，mono 回落）。
5. 亮色模式必须完整；暗色模式鼓励做（做了加分）。
6. **只创建自己的两个文件**（§6），不改动仓库任何现有文件。

## 5. 已解决歧义

- 内容语言：中文。
- 事实设定：agent 名「知序」，用户名「UmR」。当前代表色默认暖珊瑚 `#C8714C`（rep-500）。
- 窗口尺度：单页按 860px 收起态美学来，不必做两态变宽。
- 交互：鼓励少量有意义的交互（hover / 一次点击见证一个时刻），但不要堆功能。诗意 < 功能。
- 完成度：一页一屏为主，宁可精不可多。

## 6. 交付

输出目录：`E:\agent-project\northing\docs\design\2026-07-22-frontend-redesign\prototypes\bakeoff-20260802\`

| 文件 | 内容 |
|---|---|
| `<你的名字>.html` | 作品本体（名字见派发正文） |
| `<你的名字>-report.md` | 报告：① 选的方向及为什么 ② 读了哪些文档 ③ 逐条十诫自检（过/未过/不适用）④ 得意之笔与已知遗憾 |

报告控制在 40 行内。最终回复只给：一句话摘要 + 两个文件路径。
