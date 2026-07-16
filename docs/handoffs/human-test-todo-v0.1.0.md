# Human Smoke Test — v0.1.0-human-usable

> 用"第一次用 northing 的人类"视角走一遍。每项打 ✅ 或 ❌ + 截图/录屏证据。

## 1. 新用户拿到仓库第一印象

- [ ] clone 仓库，读 README Quick Start
- [ ] 按 README 的命令装依赖 + build，**实际走一遍**
- [ ] 标注哪一步卡了、哪一步文档不准

## 2. Desktop 启动

- [ ] `cargo run -p northhing` — 等 window 出来
- [ ] 看到 Welcome / Provider 配置页面了吗？
- [ ] 跳过 / 下一步按钮能用吗？鼠标能点吗？
- [ ] 创建一个 session，随便发一句话（有 API key 的话）
- [ ] 全程截图

## 3. CLI 启动

- [ ] `cargo run -p northhing-cli`
- [ ] 能打出 help / 进入主界面吗？
- [ ] 基本导航（上下左右、Esc、q）能跑吗？
- [ ] 输入框能打字吗？

## 4. Installer（有 Rust + Node 环境的话）

- [ ] `cd northing-installer && pnpm install`
- [ ] `pnpm run dev` 或 `pnpm run build`
- [ ] 看安装器 UI 渲染是否正常

## 5. 文档实测准确性

- [ ] README 里每条命令实际跑一遍
- [ ] 标记哪些命令结果跟文档不符
- [ ] Quick Start 步骤是否真的 Quick（< 10min 能到"可用"状态）

## 6. GitHub 发布材料检查

- [ ] GitHub Release 页面第一眼看过去是否是"可用的产品"
- [ ] Release notes 经过 mojibake 检测了吗（用 chardetect 或 Python 跑）
- [ ] README 里的截图、GIF 是否最新
- [ ] Issues / Discussions 是否配置好
- [ ] LICENSE 是否可见

## 7. 摩擦点汇总

- [ ] 列出所有"按文档操作却失败的"路径
- [ ] 列出所有"文档缺失、需要自己猜"的地方
- [ ] 列出所有"UI 明显异常"（字体、布局、按钮、响应）

---

每条 ❌ 标注优先级：P0（修复后才能发）/ P1（下个版本修）
