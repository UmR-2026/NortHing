# task-t7-visual-brief · T7 终审·视觉走查（八张 CDP 截图双光学）

> 派发对象：minimax-m3（用户点名做 T7 视觉审查）。
> 角色定位：**只读视觉判官**。不改代码、不跑 app、不执行任何脚本；只读截图与指定引用文档，输出双判决报告。
> 报告写到：`.superpowers/sdd/consult-room/task-t7-visual-report.md`（worktree 相对路径，UTF-8 无 BOM，LF）。

## 1. 证据清单（八张，全部本轮 CDP Page.captureScreenshot 实取）

目录：`C:\WINDOWS\TEMP\opencode\t7-shots\`

| 文件 | 窗 | 光学 |
|---|---|---|
| t7v-room-dark.png | room 主窗 | dark |
| t7v-sediment-dark.png | 沉积窗（self） | dark |
| t7v-facility-dark.png | 设施窗 | dark |
| t7v-work-dark.png | work 窗「身外之物」 | dark |
| t7v-room-light.png | room 主窗 | light |
| t7v-sediment-light.png | 沉积窗 | light |
| t7v-facility-light.png | 设施窗 | light |
| t7v-work-light.png | work 窗 | light |

技术事实（防误判）：
- 截图经 WebView2 CDP 抓帧，应用以 `-WindowStyle Hidden` 运行（窗在屏外但 WebView2 正常合成）；物理像素（scale≈1.25）。
- 模块窗 PNG 体积小（8–12KB）是**小窗+稀疏内容+PNG 压缩**的正常结果，非空白证据；room 85–89KB。八张均经编排者逐张目验为真实渲染（有完整文字与卡片结构），不存在空白帧。
- 取证脚本 trace：`C:\WINDOWS\TEMP\opencode\t7-cdp2-trace.log`（开窗次序：启动=room → `#trig-mind`=沉积+设施 → `#trig-work`=work → `#theme-toggle`=整组翻 light）。
- 已知仪器瑕疵：work target 的 classify 探针 return 超时（Wait-Kind 15s 未得 'work'），但截图渲染完整——若你认为这是症状可记 FYI，不属画面判决范围。

## 2. 判决基线 A：戒律十条（命中任一条 = 该维度不通过）

逐条对照（源自 `docs/design/2026-07-22-frontend-redesign/visual-iter-compass_20260802.md` §2）：

| # | 戒律 | 截图上的检验问题 |
|---|---|---|
| 1 | 拒绝 dashboard 美学 | 像安静的房间还是控制台？有无 "47 turns"、API 健康这类数字展览？ |
| 2 | 品牌水印化 | northing logo 只在左下水印（opacity 0.25）？视觉主体是 agent 名片/头像？ |
| 3 | 代表色是 agent 的灵魂 | 用户气泡、正文、思考块底、沉积轮都未染 rep？界面没有换色控件？ |
| 4 | 整屋空气染色 | rep 色弥漫空间（底 6.5% + 顶晕 + 名片晕染），不是只在按钮竖线上？ |
| 5 | 三要素语义互斥 | 暖 rep=正在做（驱力）；冷 abyss=向深处探（思考）；褪色灰=旧（沉积）？ |
| 6 | 暖灰基底 | 亮 `#F4F3F0` / 暗 `#181612`，不纯白不纯黑，色温走暖？ |
| 7 | 字体三系 | Fraunces=品牌/页面标题；Noto Sans SC 400+=正文（不用 300）；JetBrains Mono=元数据？ |
| 8 | 沉降式动效 | 慢、重、向下；静态截图只可查无 spinner 类构件痕迹；沉积是褪色不是位移？ |
| 9 | 诗意 < 功能 | 对话+操控台是视觉主体，agent 内在表达是余光里的环境体温？ |
| 10 | 反 AI 味 | 无 emoji、无毛玻璃、无紫蓝渐变、无均匀阴影、无过度对称？ |

## 3. 判决基线 B：双光学真色

- 亮：基底 `#F4F3F0` / surface `#FBFAF8` / raised `#EFEDE8` / border `#E6E3DD`；fg `#38352E` / muted `#7B766C`。
- 暗：基底 `#181612`（深暖黑，非纯黑）。
- 亮暗互为**真 token 翻转**，不允许出现：亮套里残留暗色块（深色卡片孤岛）、暗套里出现纯白底、直接反色相减式糊弄。
- 已知并在案的设计取舍（不判缺陷）：dark surface/bg 对比 1.34:1（visual-iter-compass §5 末条，用户已拍板）。

## 4. 判决基线 C：三窗制模块窗规格（W2 视觉解耦定案，2026-08-15 用户定案）

- **沉积窗（左宝石=self 对窗之首）三卡**：①沉积记忆 SEDIMENT——3 条记忆行（每条带 ✕）+ seg-bar 进度条 + 卡尾钉住「沉积·新层形成中」；②知识沉积 RAG——@philosophy-core「已挂载」；③沉积 skill——mock 候选列表（候选文案 + 右侧状态词「可整理」）。
- **设施窗两卡**：①运行 RUNTIME——引擎行（Claude 3.7·主人格）+ 上下文路由行（route.search: Haiku）+ 全局状态行 + **Token 消耗行「128.4k」+「清空」钮** + 卡尾钉住「全局设置」；②核心准则 AXIOMS——独立浮卡（维护主体边界 / 隐喻性修辞 等）。
- **work 窗零改动**（身外之物）：子体路由 ROUTING / 目标拆解 PLANNER / 文件差异审查 DIFF / 终端输出卡。
- **滚动语义**：窗永不滚（无整窗滚动条）；卡片标题钉住；子项列表内部滚（卡内Scrollbar 允许）。超额时各卡收缩，不允许内容溢出窗缘。
- room 主窗预期：chrome（左 wordmark+agent 标识、右主题钮+─□✕）、中央印章头像（序）+「知序」Fraunces italic 品牌、驱力状态胶囊、会话分隔「会话 03·开启」、对话流（agent 左气泡 / 见证者右对齐 / 产物 chip）、高危操作授权卡（批准/拒绝双钮、风险说明）、deck 输入条（+ 附件钮、输入框、发送钮）。
- work light 的终端卡深底绿字是否为真值所容（终端为代码/控制台语义域），由你依戒律 1/10 判断并明述理由。

## 5. 输出格式（报告正文）

1. **总判决**：VISUAL PASS / FAIL（任一项戒律命中或规格缺失 = FAIL）。
2. **双判决分述**：
   - Spec 合规判决（基线 B/C 逐条过：通过项可一笔带过，失败项给图证）。
   - Quality 判决（戒律十条逐条 + 视觉工艺：对齐/间距/层级/可读性）。
3. findings 分级：Critical（假渲染/错位/规格缺项/戒律命中）/ Important（可读性或 token 级偏差）/ Minor / FYI。每条注明：**截图文件名 + 图内位置 + 现象 + 依据基线条目**。
4. 一个直截了当的问题回答：**这八张图是否支持「模块窗增量可以合并」这一 visual 维度结论？**（CAN MERGE / NEEDS FIX）。
5. 文末附你的自检：八张是否全部实际打开读取；若有打不开/无法判读的图，明示（禁沉默跳过）。

## 6. 边界

- 不审查代码（diff 侧双判决已由 review-w2polish / review-w1racefix 两案在库）。
- 不做合并裁定（T7 合并决定归编排者+用户）。
- 不评估 mock 文案语义优劣（ specimen 文案系 W2 brief 已批内容），除非文案直接触犯戒律。
