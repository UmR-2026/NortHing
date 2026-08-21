# Consult-Room 交付说明（交用户终裁）— 2026-08-02

> 目录内 .html 全部浏览器打开过目。v4 为主页面现行版；v3 留档作呼吸系统首版记录。
> 审查 + 迭代已完成（下文），最终裁定权在用户。

## 交付清单

| 文件 | 页 | 作者 | 状态 |
|---|---|---|---|
| consult-room-v4.html | 主诊室（默认态） | qwen | 现行：模块化侧栏 + 窗控入流 + 印章 + 上下文收纳 |
| consult-room-v3.html | 主诊室 | qwen | 留档（呼吸系统首版） |
| gemini-36-flash-onboarding.html | 首次启动 | 36-flash | 已迭代（chrome 同步、emoji 修、placeholder key 清） |
| gemini-31-pro-settings.html | 设置 | 31-pro | 已迭代（chrome 同步） |
| minimax-m3-archive.html | 档案馆 | minimax | 已迭代（chrome 同步） |
| step-explore-space.html | 会话空间 | step-explore | 已迭代（chrome 同步） |

## 审查意见（逐页）

- **v4 主**：系统完整。呼吸分级（头像满幅/膜线低幅/结构恒稳）成立；窗控入流后标题栏废除，room-head 兼拖拽区；上下文聚焦浮现不占常驻位。
- **onboarding**：灰→着色仪式成立，色板=唯一改色入口守住了。问题已修：emoji×2（⚡→↯）、硬编码 placeholder key 清空。遗留：报告最薄，provider 测试态的视觉反馈简单。
- **settings**：它的自我（只读、沉积不带 rep）/ 设施（可调）分裂成立，非管理后台。遗留：无头像锚点，aura 悬屏中上，跟随感弱于主页（可接受或终裁时定补锚）。
- **archive**：12 地层透明度递降 + 节气轴 + 禁 rep（仅 chrome 参与 mind 派生）——"禁 rep vs 五色必内置"的张力处置得当（默认深渊 + 切换器机械可用）。遗留：地层不展开全文，是氛围页非工具页（作者自陈，可接受）。
- **space**：本轮最强。亮门独占 rep/光晕/呼吸，换房=灯移门，沉积门 opacity 阶梯禁 rep，叙事量纲计数。遗留：亮色无菌室态门缝漏光比喻损失约一半（作者自陈）。

## 迭代记录

1. v3→v4：侧栏模块化可拖移（station-head 把手）；窗控四键入对话流；logo 改房间内左下印章；上下文收纳进操控台（聚焦浮现）。
2. 四页 chrome 同步：标题栏废除，窗控簇/控制器簇/印章锚收容框（侧栏可移动故 chrome 不锚侧栏）。
3. 纪律修复：onboarding emoji×2、placeholder key。
4. 呼吸系统：8s 单钟、振幅分级、无新增 infinite（五页验证过）。

## 已知 caveat（终裁时知晓）

- HTML 规格层：aura 用 rAF 追踪头像；Slint 建构时改 parent.width 表达式（spike 已验）。拖移=HTML 演示，Slint 用 TouchArea+root 坐标。
- 四页残留惰性 `#win-titlebar` CSS 规则（HTML 已删），无碍。
- scale 呼吸仅 HTML；Slint 一律 opacity（spike 探针4：scale-x/scale-y 不存在）。
- mind 25 token 预计算表已在 spike 分支入 palette；五色亮色派生均不发闷。

## 终裁建议看三点

1. 呼吸是否"缓慢生物态"（8s 够不够慢、膜线幅度够不够轻）。
2. 亮色无菌室是否立得住（五页都切亮色过目）。
3. 模块化侧栏拖移后，chrome（窗控/印章/控制器）位置是否仍顺手。
