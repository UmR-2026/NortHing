# W15-1b-3 Brief — markdown_render.rs 拆测试回 rot 闸（857 → <800）

> 来源：CI rot budget check 红（run 33670235722）：`markdown_render.rs` 857 行超 800 ceiling。家规 #3 + rot 闸只许降不许升，**不许**注册新 manifest 条目（要用户签字），拆测试是唯一动作。BASE：`8a11a73`。
> 本单同时是 gemini-3.8-flash 首单冒烟——照常用证据标准要求自己。

## 现状（已磁盘核实）

- `src/apps/desktop/src/ui_dioxus/markdown_render.rs`：857 行；:509-510 `#[cfg(test)] mod tests {` 起到文件尾全是测试（含测试私有 helper `escape_html`/`render_to_html_string` 等）。
- 测试模块首行 `use super::*`（依赖父模块私有项的可见性——Rust 子模块可见祖先私有项，拆成子模块文件不受影响）。

## Spec

- S1：`markdown_render.rs` 的 `#[cfg(test)] mod tests { ... }` 整块（:509 到 EOF）剪切到新文件 `src/apps/desktop/src/ui_dioxus/markdown_render/tests.rs`；原文件末尾改为一行 `#[cfg(test)] mod tests;`。
- S2：纯位移，一个字符都不改测试体与实现体（含注释、空行）。mod.rs 不需要动（markdown_render 已注册，子模块跟随）。
- S3：拆后 `markdown_render.rs` < 800 行（实测预期 ~509）。

## Constraints

C1 只许动这两个文件（markdown_render.rs 改删尾部 + markdown_render/tests.rs 新建）。
C2 git add 只点名这两个文件。C3 以磁盘实际为准。
C4 shell 纪律：cargo 全前缀 `C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo ...`（正斜杠路径）；输出 `cmd /c "... > log 2>&1"` 重定向；禁 PowerShell 管道；禁止启动任何 GUI 应用；单条命令超 10 分钟无输出 = 杀掉报 BLOCKED。

## 验证（report 必须含命令+输出摘录）

1. `cargo check -p northhing`（硬门）
2. `cargo test -p northhing --lib markdown_render` 19 测全绿
3. `node scripts/verify-rot-budget.mjs`（在仓库根，用 `cmd /c "node scripts\verify-rot-budget.mjs > log 2>&1"`）——**必须转绿**（本单唯一目的）
4. `git diff --check`

## 报告

写 `.superpowers/sdd/w15-1b-3-report.md`：拆后行数 / 验证输出 / 状态词。完成后自行 commit（message 含 W15-1b-3）。
