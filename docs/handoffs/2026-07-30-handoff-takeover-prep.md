# Handoff 2026-07-30 — FR-T5 启动 + 接手准备（session 10 末段）

> 接 `2026-07-29-fr-t4-code-complete.md`。本 session 的副产物是给下一 session 的“基础设施”，不是新功能单。

## 一句话状态
FR-T5 计划已落档、待派单；新会话需先恢复 MCP/Skills，再按 W1→W5 顺序执行；现 `.ohmyagent/` 未提交，由接手者决定首次提交时机。

## 已完成（本次 session）

### 接手准备（MCP + Skills）
- 项目级 MCP 配置：`northing/.ohmyagent/mcp.json`
  - 单一服务 `codegraph` 固定版本 `1.5.0`
  - 通过 `cmd.exe /d /c npx -y @colbymchenry/codegraph@1.5.0 serve --mcp --path E:/agent-project/northing` 启动
  - 已验证：JSON 合法、CLI 子命令可用、本地索引存在、server 启动冒烟通过
- 项目级 Skills（已复制到 `northing/.ohmyagent/skills/`）：
  - `writing-plans`
  - `subagent-driven-development`
  - `systematic-debugging`
  - `requesting-code-review`
  - `verification-before-completion`
  - `dispatching-parallel-agents`
  - `northhing-slint-desktop`（本 session 原创，按项目约束裁剪编写，非原样复制上游）
- 6 个 Skills 文件级一致性已与父目录核对；新 Skill 经独立评审修正 5 项：
  - 中英文 AGENTS 优先级（`AGENTS-CN.md` 视为翻译，冲突以 `AGENTS.md` / `package.json` / `Cargo.lock` 为准）
  - CodeGraph 不可用时降级（不再硬性要求）
  - 架构任务前置要求读 `README.md` / `CONTRIBUTING.md`
  - 文件拆分规则回归仓库规范（>1000 必拆或 `// allow-god-file`，callbacks_lifecycle 走计划专项）
  - 收敛 "kernel facade" 表述为“既有 kernel/API 边界”

### 外部 Skill 审计结论（取舍记录）
- 上游 `slint-ui/ai-plugins/slint` 已做只读静态审查（GitHub 直连 TLS 失败，通过 GitHub API 下载固定 commit `7e8abaf` 源码快照到 `_external/slint-ai-plugins/`）。
- 快照摘要：
  - 上游 commit：`7e8abaf7450b5e10edce88c703c4283a570e0e44`
  - ZIP SHA256：`bbb625c328603c01a77c604fb5a52e58e6982555ffbb6d9668316d67b4a4ffe0`
  - 无可执行脚本/二进制；纯 Markdown + JSON + YAML
  - 未发现提示注入、凭据访问、外传、自动 hook
- 风险（已避开）：
  - `tools-install.md` 用浮动 `latest` + `curl | tar` + `sudo` + 未固定 `cargo install` → 不进入自动执行流
  - `.claude-plugin/plugin.json` 默认连 `https://docs.slint.dev/mcp` → 不启用，已有 Context7 覆盖
  - 内置 UI 控制 MCP（`reference/debugging-and-mcp.md`）默认禁用；将来用必须 loopback-only + 临时进程 + 非敏感测试数据
  - 同步 CI `.github/workflows/sync-from-monorepo.yaml` 用浮动 `actions/checkout@v7` + 未固定克隆分支 → 不引入
  - 上游仅声明 `GPL-3.0 OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0`，快照无许可证正文 → 不原样复制到仓库
- 公开候选评估（均未自动安装）：
  - `rust-desktop-applications`：内容偏 Tauri
  - `visual-regression-testing`：偏浏览器/CSS，与原生 Slint 无关
- 结论：**只原创 `northhing-slint-desktop` Skill**，不引入上游整套 Skill。

## 进行中 / 卡点
- 无（`.ohmyagent/` 改动尚未 commit；FR-T5 五波均尚未派单）

## 队列（下一 session 直接派单）
按 `docs/superpowers/plans/2026-07-29-fr-t5-settings-drawers.md` 顺序：

1. **W5 功能断点（优先，最高 P0）**
   - T5-13 Identity Creator Rust 接线（onboarding 卡死）
   - T5-14 export-markdown Rust 接线
   - T5-15 open-session-settings 入口设计拍板
2. **W1 设置统一**
   - T5-1 设置壳重做（派 glm）
   - T5-2 工作文件夹页迁移（派 ling，预备续修轮）
   - T5-3 五页校订
   - T5-4 收纳确认
3. **W2 抽屉外扩（先 POC）**
   - T5-5 方案+POC（Rust + winit set_inner_size 真实变宽）
   - T5-6 全面铺开
4. **W3 右抽屉「外物」重做**
   - T5-7 收摊 / T5-8 外物空态 / T5-9 deck `/` 调 skill 列表
5. **W4 杂项**
   - T5-10 tofu glyph 排查
   - T5-11 降级项收尾
   - T5-12 onboarding 色板拍板（用户未决）
   - T5-16 housekeeping（callbacks_lifecycle 917 行拆分 + AGENTS.md K4a invariant）

## 选派指引（沿用 facts/models.md 2026-07-28 修订）
- coder 中大型：glm 首选；lc 中单停派观察
- 机械小单：bp / mimo / m27hs（禁删除/重指向）
- judge：m3 首选 / lc 备选 / glm 备选
- ⛔ qw 无额度停派；step s35/s37 勿派 judge/中大单

## 验证（每单最小预检）
```text
$env:CARGO_PROFILE_DEV_SPLIT_DEBUGINFO='off'
rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing
pnpm run fmt:rs            # 仅 Rust 改动时
```
行为/并发/取消/持久化改动必带 focused 测试；视觉改动必附 `shot-window.ps1` / `click-window.ps1` 截图证据。

## Suggested skills（下一 session）
- `northhing-slint-desktop`：所有 Slint/Rust 改动
- `writing-plans`：任务书与修订
- `subagent-driven-development`：派单与并行
- `systematic-debugging`：窗口外扩 / tofu glyph / 未接 callback 排查
- `verification-before-completion`：每单必跑验证
- `requesting-code-review`：judge 验收

## 已知雷区（不重蹈）
- PowerShell 写非 ASCII 用 edit 工具；judge 与 coder 文字汇报均以磁盘 diff 取证
- 并行同 crate cargo check 互踩，"编译干净"以最终工作区为准
- bin+lib 双 target crate `crate::` re-export 不互通，处方须明示
- 桌面 Rust i18n 已冻结；不要扩展 i18n 体系
- 启动新会话前 `.ohmyagent/mcp.json` 与 `skills/` 未提交，需决定何时入仓

## 一句话状态
**FR-T5 计划就绪、基础设施就绪、未派单、未提交**。下一 session 先按 “启动 → 验证 MCP/Skills 加载 → 派 W5 三单” 顺序开局。