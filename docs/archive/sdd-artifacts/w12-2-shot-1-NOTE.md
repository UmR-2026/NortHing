# W12-2 截图说明

`w12-2-shot-1.png` 的真应用截图依赖 Dioxus + Webview2 桌面壳的运行实例
（本机环境构建与验证无 GUI 交互子系统）。
本轮交付以 SVG mockup 形式给出归档页会话全文搜索（含防抖、标题命中前置排序、snippet 渲染及详情展开）的视觉结构，附 `.superpowers/sdd/w12-2-shot-1.svg`。

后续若在带图形界面的机器上启动，可按以下步骤重拍真机图：
1. `pnpm run desktop:preview:debug`（cold start 启动 Dioxus consult-room 桌面端）
2. 导航至「档案馆 / 沉渊境界」窗口
3. 在顶部搜索框输入搜索词（如 "重构"），观察 300ms 防抖后服务端返回全文搜索结果
4. 验证标题匹配项排在前面，结果展示会话名 + snippet + 时间，点击可展开底部只读消息详情
5. 截图保存为 `.superpowers/sdd/w12-2-shot-1.png` 覆盖。
