# W15-1b-2 Brief — Markdown 集成单（app.rs 三渲染点 + CSS）

> 来源：`.superpowers/sdd/w15-1-arbitration.md` §5.1 切片 2 + §7 附带条件；切片 1 已验收（commit `329cc8f`，APPROVE 0C/0I）。BASE：`329cc8f`。
> 切片 1 交付：`ui_dioxus/markdown_render.rs` — `pub fn render_markdown(input: &str) -> Element` + `pub fn sanitize_url_scheme` + 19 单测全绿。

## Spec

- S1：新建 `src/apps/desktop/src/ui_dioxus/pages_chat_md_css.rs`，形态仿 `pages_onboarding_css.rs`（`pub const CHAT_MD_CSS: &str = r#"..."#;`）。内容（选择器作用域限 `.md-rendered` 子树，仲裁 §8.3 你自决但样式不许漏到非 md 区域）：
  - `.msg-agent` / `.body` 的 `.md-rendered` 修饰：`white-space: pre-wrap; word-break: break-word;`（仲裁 §7#7 硬性要求）
  - 段落/标题/列表排版；`pre` 代码块（等宽 + pre-wrap）；`code` 内联码（`--font-mono`）；`a` 链接色（`var(--accent-solid)`，若该变量不存在用现有 accent 变量，以磁盘为准）；`blockquote` 左灰条；`hr` dashed
  - 风格对齐 TRUTH_CSS 既有设计语言（衬线正文不动，只加 markdown 结构样式）
- S2：`ui_dioxus/mod.rs` 注册 `mod pages_chat_md_css;`（形态与兄弟模块对齐）。
- S3：注入：app.rs 主 style 注入点（:330-331 一带）追加一个 `style { dangerous_inner_html: "{pages_chat_md_css::CHAT_MD_CSS}" }`（**不改原行**；CHAT_MD_CSS 是受信任常量，此注入形态与现有 26 处同款）。注入窗数量仲裁 §8.4 你自决（哪个窗真渲染消息正文就注入哪个，别盲注 9 窗）。
- S4：三渲染点接 `render_markdown`：
  - app.rs 流式 draft（:502-509 一带）：`div.msg-agent` 内 `{draft}` → `{render_markdown(&draft)}`，div 加 `md-rendered` class
  - app.rs assistant 完成态（:747 一带）：`{body}` → `{render_markdown(&body)}`，加 class
  - app.rs witness 完成态（:757 一带）：`{body}` → `{render_markdown(&body)}`，加 class
  - **绝不许** `dangerous_inner_html: "{body}"`（仲裁 §7#2 打回红线）
- S5：切片 1 遗留 Minor 顺手修（家规 #1 顺手清配额）：
  - `render_single_inline`/`render_single_block` 的 `_key: usize` 实参未传到 RSX `key:` 属性——接上（流式 draft 重渲染性能）
  - `escape_html` 补 `'` 转义（一行）
  - 三处 `_ => {}` catch-all 加 `// ponytail:` 注释（Options::empty() 下不可达；升级路径 = pulldown-cmark 升版时改穷尽匹配）
  - 图片 alt 内格式化被丢弃 → 加一行注释说明已知限制

## Constraints

C1 只许动：app.rs（四处：style 注入 + 三渲染点）、pages_chat_md_css.rs（新建）、mod.rs（一行）、markdown_render.rs（仅 S5 四处小修）。
C2 禁碰（仲裁 §7#15）：css.rs（一个字符都不行）、pages_archive.rs、pages_archive_search.rs、contracts/kernel-api/src/session.rs、session_mock.rs。`git diff` 中 css.rs 必须为空。
C3 测试数与覆盖不下降；markdown_render 19 单测保持绿。
C4 以磁盘实际为准，行号是参考坐标不是合同；偏差记 report。
C5 日志英文无 emoji；rot 闸 let _ = 371/388 不涨。
C6 shell 纪律（违者命令会永久挂起，照抄下列范式）：
- cargo 必须全前缀：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo <args>`
- 输出重定向只用 cmd：`cmd /c "<上面整条> > C:\WINDOWS\TEMP\opencode\<name>.log 2>&1"`，读完日志文件再删。**禁止**任何 PowerShell 管道（`|` / `2>&1 |` / `Out-File`）——测试派生的孙进程继承 stdout 句柄会让管道永久阻塞。
- 重复跑测试：先 `cargo test --no-run` 构建一次拿到二进制路径，之后直接跑二进制（~1s/次）。
- 不同时跑两个 cargo（互等 .cargo-lock）。

## 验证（report 必须含命令+输出摘录）

1. `cmd /c "cd /d E:\agent-project\NortHing && %USERPROFILE%\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing > C:\WINDOWS\TEMP\opencode\w15b2-check-desktop.log 2>&1"`（家规 #6 硬门）
2. 同上范式跑 `cargo check --workspace`（日志名 w15b2-check-ws.log）
3. `cargo test -p northhing --lib --no-run` 构建一次拿二进制路径，再直接跑二进制（全绿，含 markdown_render 19 测；日志 w15b2-test.log）
4. `git diff --check`
5. ~~截图~~ **改由编排者负责**（修订：连续三位 coder 卡死在 shell，截图步骤剥离出本单）。**你禁止启动桌面 GUI 应用**（`cargo run` / 运行 northhing.exe 会永久阻塞 shell）。
6. **熔断规则**：任何单条命令超过 10 分钟无输出，立即杀掉该进程，把该命令原文 + 已等待时长写进 report，状态词报 BLOCKED，不要死等。

## 报告

写 `.superpowers/sdd/w15-1b-2-report.md`：改动清单 / 验证输出 / 对半成品的裁定 / 偏差 / 状态词。完成后自行 commit（message 含 W15-1b-2）。
