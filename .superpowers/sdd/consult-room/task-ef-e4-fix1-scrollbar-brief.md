# E4 fix1 — onboarding 窗原生滚动条 — implementer brief

编排者只设计，你写代码。不要问问题。不要 commit。**只改一个文件。**

工作地点（相对路径根、命令 workdir）：`E:\agent-project\northing\.worktrees\consult-room-build`

## 现象

用户目验报告：onboarding 窗 chat-flow 右缘露 **WebView2 原生滚动条**（浅色，与暗色主题冲突）。截图为用户手拍，口头传达即可。

根因：`pages_onboarding_css.rs` 的 ONBOARDING_CSS 是真值 `<style>` 自包含转写，而真值 HTML 无 scrollbar 规则；本窗**不注入** `css::truth_css()`（brief §5 决策），故 css.rs 里已有的自定义滚动条不生效。

## 改法（逐字转录，零发挥）

把 `src/apps/desktop/src/ui_dioxus/css.rs` **第 161-171 行**的 R4 W4 自定义细滚动条块（`::-webkit-scrollbar` 7 条规则：width 10px / track transparent / thumb `var(--line)` + 3px transparent border + background-clip padding-box / thumb:hover `var(--faint)` / button display:none / corner transparent）**逐字复制**进 `src/apps/desktop/src/ui_dioxus/pages_onboarding_css.rs` 的 `ONBOARDING_CSS` const 内（放在规则块尾部、media query 之前均可，位置自判不破坏既有结构）。

- 附一行注释注明出处与理由：`/* R4 W4 自定义细滚动条——同 css.rs 同款；本窗自包含不注 TRUTH_CSS，需自带（转写层新增，真值零 scrollbar 规则） */`（照 css.rs:161 注释语气，可精简）
- **不要**用 `display:none` 隐藏滚动条——仓库定规是细条主题化，不是消灭（room/E1-E3 均如此）
- 除该块 + 注释外**零改动**；不动 css.rs / pages_onboarding.rs / 任何其他文件

## 验证

1. `C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing`（exit 0；PATH 裸 cargo 是 GNU 工具链，禁用）
2. 确认 `pages_onboarding_css.rs` 行数仍 <800
3. 不需要 CDP/截图（编排者 rebuild 后交用户目验）

## 报告

`.superpowers/sdd/consult-room/task-ef-e4-fix1-scrollbar-report.md`（改动行号 + check 输出 + 行数，≤20 行即可）
