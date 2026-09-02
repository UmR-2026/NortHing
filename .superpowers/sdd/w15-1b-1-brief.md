# W15-1b-1 Brief — markdown_render.rs 基础单（事件流→RSX 映射）

> 来源：`.superpowers/sdd/w15-1-arbitration.md` 主判（方案 B + pulldown-cmark 0.11 复用）+ §5.1 切片 1 + §7 附带条件。BASE：`c14171f`。
> 本单只做基础层：新文件 + 依赖引用 + 单测。**不**动 app.rs / css.rs / 任何现有渲染点（那是切片 2）。

## Spec

- S1：`src/apps/desktop/Cargo.toml` 加一行 `pulldown-cmark = { workspace = true }`（workspace 根 Cargo.toml:130-132 已锁 0.11，零新下载）。
- S2：新建 `src/apps/desktop/src/ui_dioxus/markdown_render.rs`：
  - `pub fn render_markdown(input: &str) -> Element` —— pulldown-cmark 事件流直接映射 Dioxus RSX 元素（dioxus 0.8.0-alpha.1 的具体返回形态——Element / LazyNodes / 其它——由你落地实测选定，仲裁 §8.1 已声明不预设）。
  - 支持事件：段落、标题 h1-h6、列表 ul/ol/li、代码块、内联码、链接、强调 em/strong、引用 blockquote、分隔线 hr、SoftBreak/HardBreak。`Parser::new_ext(input, Options::empty())`——**Options::empty()**，不加任何 GFM flag。
  - `fn sanitize_url_scheme(url: &str) -> Option<&str>` —— 显式 match/if 链白名单 `http/https/mailto/tel` 放行，其它返回 None（外层剥 `<a>` 包装、链接文字按纯文本渲染）；禁正则黑名单。加 `// ponytail: 4 类白名单（http/https/mailto/tel）；升级路径 = 读 config.toml 的 allowed-url-schemes`。
  - `Event::Html(_)` 必须 explicit drop（不映射、不输出、不留痕迹）；`Event::InlineHtml` 若该版本存在同样 drop。
  - 图片：scheme 白名单同款；非白名单 src 不渲染 img（alt 按文本展示）。
  - 所有渲染出的元素 class 带 `md-*` 前缀或落在 `.md-rendered` 子树内（具体选择器策略你定，仲裁 §8.3 不预设；原则：markdown 样式不漏到非 md 区域）。
- S3：`ui_dioxus/mod.rs` 注册 `mod markdown_render;`（或 `pub mod`，按该文件现有兄弟模块的形态对齐——以磁盘为准）。
- S4：单测 ≥6 条 XSS 注入向量（仲裁 §7#4 逐字）+ 常规渲染断言（段落/标题/列表/代码块/内联码/链接/强调/引用/分隔线）：
  - `<script>alert(1)</script>` → 输出不含字面量 `<script>`
  - `[click](javascript:alert(1))` → 不含 `href="javascript:"`
  - `![x](data:text/html,<b>)` → 不含 `src="data:text/html"`
  - `<img src=x onerror=alert(1)>` → 不含 `onerror=`
  - `[a](vbscript:msgbox(1))` → 不含 `vbscript:`
  - 套娃 `<scr<script>ipt>` → 不含可执行 `<script>`
  - 测试断言对象的具体形态（VirtualDom 渲染字符串 / dioxus-ssr / 其它）由你跑通后选定（仲裁 §8.2 不预设），但**断言必须是真实渲染输出的字符串检查，不是恒真断言**。

## Constraints

C1 只许动：`src/apps/desktop/Cargo.toml`、`src/apps/desktop/src/ui_dioxus/markdown_render.rs`（新建）、`src/apps/desktop/src/ui_dioxus/mod.rs`（一行注册）、`Cargo.lock`（cargo 自动更新）。
C2 **禁碰清单**（仲裁 §7#15）：css.rs / pages_archive.rs / pages_archive_search.rs / contracts/kernel-api/src/session.rs / session_mock.rs / app.rs。
C3 禁止出现 ammonia 依赖；禁止任何 `dangerous_inner_html` 用于消息文本。
C4 以磁盘实际代码为准；dioxus alpha API 与 brief 描述不符时以编译器为准，偏差记 report。
C5 日志英文无 emoji；rot 闸：`let _ =` 371/388 基线不许涨。
C6 构建/测试走 `rustup run stable-x86_64-pc-windows-msvc cargo ...`，输出 `cmd /c` 重定向，禁 PS 管道；先读 skill `long-running-shell`。

## 验证（report 必须含命令+输出摘录）

1. `rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing`（家规 #6 desktop compile gate）
2. `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing --lib markdown_render` 全绿（含 6 条 XSS 向量）
3. `git diff --check` 无 whitespace error

## 报告

写 `.superpowers/sdd/w15-1b-1-report.md`：API 形态选定理由（仲裁 §8.1/8.2 的现场结论）/ 测试清单与输出 / 偏差 / 状态词。完成后自行 commit（message 含 W15-1b-1）。
