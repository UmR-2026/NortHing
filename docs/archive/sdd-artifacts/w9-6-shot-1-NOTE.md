# W9-6 截图说明

`w9-6-shot-1.png` 的真应用截图依赖 Dioxus + Webview2 桌面壳的运行实例
（本机仅带 Rust 工具链；启动该壳需要 Windows 运行时 + Windows GUI 子系统）。
本轮交付以 SVG mockup 形式给出文件树 + 预览的视觉形状，附 `w9-6-shot-1.svg`。

后续若可启动，可重拍：
1. `pnpm run desktop:preview:debug`（cold start，无 HMR）
2. 唤起右宝石 → 「工作目录」卡可见
3. 展开 src/ → 点 main.rs → 下方预览区域显示文本
4. 截图保存为本文件名覆盖。
