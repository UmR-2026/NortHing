# W9-7 截图说明

`w9-7-shot-1.png` 的真应用截图依赖 Dioxus + Webview2 桌面壳的运行实例
（本机仅带 Rust 工具链；启动该壳需要 Windows 运行时 + Windows GUI 子系统）。
本轮交付以 SVG mockup 形式给出设置页左列四卡 + 显示模式卡的视觉形状，附 `w9-7-shot-1.svg`。

后续若可启动，可重拍：
1. `pnpm run desktop:preview:debug`（cold start，无 HMR）
2. 唤起设置窗（左列"它的自我" + 右列"设施"）
3. 左列四卡可见：沉积（计数 + 段条）、编年史（真实会话 updated_at）、身份（默认 provider.display_name）、准则（空态文案）
4. 右列"显示模式"卡两个开关（呼吸 / 双镜）切换后 → 关闭重开 → 保持
5. 截图保存为本文件名覆盖。

## 验证集对应的可见状态

- 沉积卡：记忆 12 + 技能 8 → 累计 20 → 段条 3/5 亮（原 3/5 是巧合，现逻辑 = min(20, 5) = 5 → 全亮；mockup 用 20 演示"已沉淀"上限）。
- 编年史卡：最早/最新会话的 updated_at 渲染为 `YYYY.MM`（Genesis 2026.07 / Event 2026.08）。
- 身份卡：默认 provider 模型的 `display_name`（onboarding 时填入 agent_name）；位格空态注脚。
- 准则卡：诚实空态文案（无数据通路，非用户数据）。
- 显示模式卡：toggle 持久化到 `~/.northhing/config/app.json`，UI 注脚说明视觉绑定待后续。
