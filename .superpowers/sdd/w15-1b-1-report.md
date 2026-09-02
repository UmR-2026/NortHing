# W15-1b-1 Implementation Report — Desktop Markdown Render Foundation

## 1. 概述与完成情况

- **任务**：W15-1b-1 桌面聊天消息 Markdown 渲染基础单（事件流→RSX 直映射，零 HTML 注入面）
- **状态**：**DONE**
- **改动范围**：
  - `src/apps/desktop/Cargo.toml`：添加 `pulldown-cmark = { workspace = true }` 引用（锁版本 0.11.3，零新下载）。
  - `src/apps/desktop/src/ui_dioxus/mod.rs`：注册 `pub mod markdown_render;`。
  - `src/apps/desktop/src/ui_dioxus/markdown_render.rs`：新建模块，提供 `pub fn render_markdown(input: &str) -> Element`、`pub fn sanitize_url_scheme(url: &str) -> Option<&str>`、AST 解析与 RSX 直映射，以及完整单元测试套件（含 6 条 XSS 注入向量断言）。
  - `Cargo.lock`：cargo 自动更新 northhing 依赖。

## 2. API 形态现场结论（仲裁 §8.1 / §8.2）

1. **§8.1 `render_markdown` 返回形态**：
   - 函数签名定为 `pub fn render_markdown(input: &str) -> Element`。
   - 内部通过 `parse_markdown_to_blocks(input: &str) -> Vec<MdBlock>` 将 pulldown-cmark（`Options::empty()`）拉式事件流组织为紧凑 AST，随后通过递归 RSX 宏映射生成 `Element`（即 Dioxus `Result<VNode, RenderError>` / `Option<VNode>`），顶层挂载 `<div class="md-rendered">` 容器。
   - 支持段落（`p.md-p`）、标题（`h1.md-h1`..`h6.md-h6`）、列表（`ul.md-ul`、`ol.md-ol`、`li.md-li`）、代码块（`pre.md-code-block` + `code.md-code`）、内联码（`code.md-inline-code`）、链接（`a.md-link`）、强调与加粗（`em.md-em`、`strong.md-strong`）、引用（`blockquote.md-blockquote`）、分隔线（`hr.md-hr`）、换行（`br.md-br` / softbreak）。

2. **§8.2 测试钩子与断言形态**：
   - 测试中实现了 `render_to_html_string(el: Element) -> String`，深度遍历 `VNode.template` 与 `VNode.dynamic_nodes` / `VNode.dynamic_attrs`，并对文本节点与动态属性应用完整的 HTML 实体转义。
   - 所有单测断言均为针对真实 DOM 树序列化生成的 HTML 字符串精确断言（包含 6 条 XSS 向量与 13 条标准元素/Scheme 校验），非恒真断言。

3. **安全约束落实**：
   - 零 HTML 注入面：`Event::Html` 与 `Event::InlineHtml` 显式 drop，`Tag::HtmlBlock` 丢弃。
   - Scheme 白名单：`sanitize_url_scheme` 显式 match `http` / `https` / `mailto` / `tel` 四类放行，其余返回 `None`；非白名单链接剥除 `<a>` 标签降级为纯文本，非白名单图片不渲染 `<img>` 降级为 alt 文本。
   - 无 ammonia 依赖，无 `dangerous_inner_html`。
   - 禁碰清单（`css.rs` / `app.rs` / `pages_archive.rs` / `session_mock.rs` 等）零触碰。
   - Rot 闸位：未引入任何 `let _ =`（0 增长）。

## 3. 验证证据

### 验证 1：Desktop 编译门禁（家规 #6）
```powershell
C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing
```
输出摘录：
```text
    Checking pulldown-cmark v0.11.3
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.91s
```

### 验证 2：单测全绿（19/19 通过，含 6 条 XSS 注入向量）
```powershell
C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing --lib markdown_render
```
输出摘录：
```text
running 19 tests
test ui_dioxus::markdown_render::tests::test_render_inline_code ... ok
test ui_dioxus::markdown_render::tests::test_render_code_block ... ok
test ui_dioxus::markdown_render::tests::test_render_blockquote ... ok
test ui_dioxus::markdown_render::tests::test_render_ordered_list ... ok
test ui_dioxus::markdown_render::tests::test_sanitize_url_scheme_whitelist ... ok
test ui_dioxus::markdown_render::tests::test_render_whitelisted_image ... ok
test ui_dioxus::markdown_render::tests::test_xss_vector_1_raw_script_tag ... ok
test ui_dioxus::markdown_render::tests::test_render_paragraph ... ok
test ui_dioxus::markdown_render::tests::test_render_unordered_list ... ok
test ui_dioxus::markdown_render::tests::test_render_headings_h1_to_h6 ... ok
test ui_dioxus::markdown_render::tests::test_render_hard_and_soft_break ... ok
test ui_dioxus::markdown_render::tests::test_render_links_whitelisted ... ok
test ui_dioxus::markdown_render::tests::test_render_emphasis_and_strong ... ok
test ui_dioxus::markdown_render::tests::test_render_horizontal_rule ... ok
test ui_dioxus::markdown_render::tests::test_xss_vector_2_javascript_scheme_link ... ok
test ui_dioxus::markdown_render::tests::test_xss_vector_4_raw_img_onerror ... ok
test ui_dioxus::markdown_render::tests::test_xss_vector_6_nested_script_tags ... ok
test ui_dioxus::markdown_render::tests::test_xss_vector_3_data_scheme_image ... ok
test ui_dioxus::markdown_render::tests::test_xss_vector_5_vbscript_scheme_link ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 145 filtered out; finished in 0.00s
```

### 验证 3：Whitespace 检查
```powershell
git diff --check
```
输出：无 whitespace 错误。

### 验证 4：Workspace 整体编译与 Repo Hygiene
```powershell
C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check --workspace
pnpm run check:repo-hygiene
```
输出：编译通过，Repository hygiene check passed。

## 4. 偏差与 Follow-up

- **Spec 偏差**：无。严格遵循仲裁方案 B 与 brief 所有约束。
- **Follow-up 记项**（仲裁 §7#14）：孤儿文件 `css_files.rs` 留待 W15-2 处理（删除或注册进 mod.rs）。

## 5. 状态结论

**DONE**
