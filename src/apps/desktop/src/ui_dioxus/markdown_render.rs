// SPDX-License-Identifier: MIT OR Apache-2.0
//
// W15-1b-1: Desktop Markdown rendering foundation module.
//
// Direct event-stream to Dioxus RSX mapping for safe, zero-HTML-injection
// Markdown rendering in the consult-room chat surface.
//
// References:
//   * brief:       `.superpowers/sdd/w15-1b-1-brief.md`
//   * arbitration: `.superpowers/sdd/w15-1-arbitration.md` (Scheme B, §7 conditions)

use dioxus::prelude::*;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Whitelist of safe URL schemes.
/// Only `http`, `https`, `mailto`, and `tel` are permitted.
///
// ponytail: 4 类白名单（http/https/mailto/tel）；升级路径 = 读 config.toml 的 allowed-url-schemes
pub fn sanitize_url_scheme(url: &str) -> Option<&str> {
    let trimmed = url.trim();
    if let Some((scheme, _)) = trimmed.split_once(':') {
        let scheme_lower = scheme.to_ascii_lowercase();
        match scheme_lower.as_str() {
            "http" | "https" | "mailto" | "tel" => Some(trimmed),
            _ => None,
        }
    } else {
        None
    }
}

/// Structured AST representations of Markdown nodes.
#[derive(Debug, Clone, PartialEq)]
pub enum MdBlock {
    Heading {
        level: u8,
        inlines: Vec<MdInline>,
    },
    Paragraph(Vec<MdInline>),
    BlockQuote(Vec<MdBlock>),
    CodeBlock {
        lang: String,
        code: String,
    },
    List {
        ordered: bool,
        start: Option<u64>,
        items: Vec<MdListItem>,
    },
    Rule,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MdListItem {
    pub blocks: Vec<MdBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MdInline {
    Text(String),
    Code(String),
    Strong(Vec<MdInline>),
    Emphasis(Vec<MdInline>),
    Link {
        url: String,
        title: String,
        children: Vec<MdInline>,
    },
    Image {
        url: String,
        title: String,
        alt: String,
    },
    SoftBreak,
    HardBreak,
}

enum BlockContext {
    Root(Vec<MdBlock>),
    BlockQuote(Vec<MdBlock>),
    List {
        ordered: bool,
        start: Option<u64>,
        items: Vec<MdListItem>,
    },
    ListItem {
        blocks: Vec<MdBlock>,
        current_inlines: Vec<MdInline>,
    },
    Paragraph(Vec<MdInline>),
    Heading {
        level: u8,
        inlines: Vec<MdInline>,
    },
    CodeBlock {
        lang: String,
        code: String,
    },
    HtmlBlock,
}

impl BlockContext {
    fn into_block(self) -> Option<MdBlock> {
        match self {
            BlockContext::Paragraph(inlines) => Some(MdBlock::Paragraph(inlines)),
            BlockContext::Heading { level, inlines } => Some(MdBlock::Heading { level, inlines }),
            BlockContext::BlockQuote(blocks) => Some(MdBlock::BlockQuote(blocks)),
            BlockContext::CodeBlock { lang, code } => Some(MdBlock::CodeBlock { lang, code }),
            BlockContext::List { ordered, start, items } => Some(MdBlock::List { ordered, start, items }),
            _ => None,
        }
    }
}

enum InlineContext {
    Strong(Vec<MdInline>),
    Emphasis(Vec<MdInline>),
    Link {
        url: String,
        title: String,
        children: Vec<MdInline>,
    },
    Image {
        url: String,
        title: String,
        alt: String,
    },
}

impl InlineContext {
    fn push_inline(&mut self, inline: MdInline) {
        match self {
            InlineContext::Strong(children)
            | InlineContext::Emphasis(children)
            | InlineContext::Link { children, .. } => children.push(inline),
            InlineContext::Image { alt, .. } => {
                if let MdInline::Text(t) = inline {
                    alt.push_str(&t);
                }
            }
        }
    }

    fn into_inline(self) -> MdInline {
        match self {
            InlineContext::Strong(children) => MdInline::Strong(children),
            InlineContext::Emphasis(children) => MdInline::Emphasis(children),
            InlineContext::Link { url, title, children } => MdInline::Link { url, title, children },
            InlineContext::Image { url, title, alt } => MdInline::Image { url, title, alt },
        }
    }
}

/// Parse Markdown input string with CommonMark `Options::empty()` into structured `MdBlock`s.
pub fn parse_markdown_to_blocks(input: &str) -> Vec<MdBlock> {
    let parser = Parser::new_ext(input, Options::empty());
    let mut block_stack: Vec<BlockContext> = vec![BlockContext::Root(Vec::new())];
    let mut inline_stack: Vec<InlineContext> = Vec::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => block_stack.push(BlockContext::Paragraph(Vec::new())),
                Tag::Heading { level, .. } => {
                    let lvl = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                    block_stack.push(BlockContext::Heading {
                        level: lvl,
                        inlines: Vec::new(),
                    });
                }
                Tag::BlockQuote(_) => block_stack.push(BlockContext::BlockQuote(Vec::new())),
                Tag::CodeBlock(kind) => {
                    let lang = match kind {
                        CodeBlockKind::Fenced(l) => l.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    block_stack.push(BlockContext::CodeBlock {
                        lang,
                        code: String::new(),
                    });
                }
                Tag::List(start) => block_stack.push(BlockContext::List {
                    ordered: start.is_some(),
                    start,
                    items: Vec::new(),
                }),
                Tag::Item => block_stack.push(BlockContext::ListItem {
                    blocks: Vec::new(),
                    current_inlines: Vec::new(),
                }),
                Tag::HtmlBlock => block_stack.push(BlockContext::HtmlBlock),
                Tag::Strong => inline_stack.push(InlineContext::Strong(Vec::new())),
                Tag::Emphasis => inline_stack.push(InlineContext::Emphasis(Vec::new())),
                Tag::Link { dest_url, title, .. } => inline_stack.push(InlineContext::Link {
                    url: dest_url.to_string(),
                    title: title.to_string(),
                    children: Vec::new(),
                }),
                Tag::Image { dest_url, title, .. } => inline_stack.push(InlineContext::Image {
                    url: dest_url.to_string(),
                    title: title.to_string(),
                    alt: String::new(),
                }),
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::BlockQuote | TagEnd::CodeBlock | TagEnd::List(_) => {
                    if let Some(ctx) = block_stack.pop() {
                        if let Some(block) = ctx.into_block() {
                            push_block_to_parent(&mut block_stack, block);
                        }
                    }
                }
                TagEnd::Item => {
                    if let Some(BlockContext::ListItem {
                        mut blocks,
                        current_inlines,
                    }) = block_stack.pop()
                    {
                        if !current_inlines.is_empty() {
                            blocks.push(MdBlock::Paragraph(current_inlines));
                        }
                        if let Some(BlockContext::List { items, .. }) = block_stack.last_mut() {
                            items.push(MdListItem { blocks });
                        }
                    }
                }
                TagEnd::HtmlBlock => {
                    block_stack.pop();
                }
                TagEnd::Strong | TagEnd::Emphasis | TagEnd::Link | TagEnd::Image => {
                    if let Some(ctx) = inline_stack.pop() {
                        push_inline_to_target(&mut inline_stack, &mut block_stack, ctx.into_inline());
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                let text_str = text.into_string();
                if let Some(BlockContext::CodeBlock { code, .. }) = block_stack.last_mut() {
                    code.push_str(&text_str);
                } else if !block_stack.is_empty() && matches!(block_stack.last(), Some(BlockContext::HtmlBlock)) {
                    // Explicit drop for HTML block content
                } else {
                    push_inline_to_target(&mut inline_stack, &mut block_stack, MdInline::Text(text_str));
                }
            }
            Event::Code(code) => {
                push_inline_to_target(&mut inline_stack, &mut block_stack, MdInline::Code(code.into_string()));
            }
            Event::Html(_) | Event::InlineHtml(_) => {
                // Explicit drop: zero HTML injection surface
            }
            Event::SoftBreak => {
                push_inline_to_target(&mut inline_stack, &mut block_stack, MdInline::SoftBreak);
            }
            Event::HardBreak => {
                push_inline_to_target(&mut inline_stack, &mut block_stack, MdInline::HardBreak);
            }
            Event::Rule => {
                push_block_to_parent(&mut block_stack, MdBlock::Rule);
            }
            _ => {}
        }
    }

    // Flush any unclosed blocks into root
    while block_stack.len() > 1 {
        if let Some(ctx) = block_stack.pop() {
            if let Some(block) = ctx.into_block() {
                push_block_to_parent(&mut block_stack, block);
            }
        }
    }

    match block_stack.pop() {
        Some(BlockContext::Root(blocks)) => blocks,
        _ => Vec::new(),
    }
}

fn push_block_to_parent(stack: &mut [BlockContext], block: MdBlock) {
    if let Some(parent) = stack.last_mut() {
        match parent {
            BlockContext::Root(blocks) | BlockContext::BlockQuote(blocks) => blocks.push(block),
            BlockContext::ListItem {
                blocks,
                current_inlines,
            } => {
                if !current_inlines.is_empty() {
                    let inlines = std::mem::take(current_inlines);
                    blocks.push(MdBlock::Paragraph(inlines));
                }
                blocks.push(block);
            }
            _ => {}
        }
    }
}

fn push_inline_to_target(inline_stack: &mut [InlineContext], block_stack: &mut [BlockContext], inline: MdInline) {
    if let Some(top_inline) = inline_stack.last_mut() {
        top_inline.push_inline(inline);
    } else if let Some(top_block) = block_stack.last_mut() {
        match top_block {
            BlockContext::Paragraph(inlines) | BlockContext::Heading { inlines, .. } => inlines.push(inline),
            BlockContext::ListItem { current_inlines, .. } => current_inlines.push(inline),
            _ => {}
        }
    }
}

/// Render a list of `MdInline` nodes to Dioxus RSX elements.
fn render_inlines(inlines: Vec<MdInline>) -> Element {
    rsx! {
        for (idx, inline) in inlines.into_iter().enumerate() {
            {render_single_inline(inline, idx)}
        }
    }
}

/// Render a single `MdInline` node to Dioxus RSX.
fn render_single_inline(inline: MdInline, _key: usize) -> Element {
    match inline {
        MdInline::Text(text) => rsx! {
            "{text}"
        },
        MdInline::Code(code) => rsx! {
            code {
                class: "md-inline-code",
                "{code}"
            }
        },
        MdInline::Strong(children) => rsx! {
            strong {
                class: "md-strong",
                {render_inlines(children)}
            }
        },
        MdInline::Emphasis(children) => rsx! {
            em {
                class: "md-em",
                {render_inlines(children)}
            }
        },
        MdInline::Link { url, title, children } => {
            if let Some(safe_url) = sanitize_url_scheme(&url) {
                let has_title = !title.is_empty();
                rsx! {
                    a {
                        class: "md-link",
                        href: "{safe_url}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        title: if has_title { Some(title) } else { None },
                        {render_inlines(children)}
                    }
                }
            } else {
                rsx! {
                    {render_inlines(children)}
                }
            }
        }
        MdInline::Image { url, title, alt } => {
            if let Some(safe_url) = sanitize_url_scheme(&url) {
                let has_title = !title.is_empty();
                rsx! {
                    img {
                        class: "md-img",
                        src: "{safe_url}",
                        alt: "{alt}",
                        title: if has_title { Some(title) } else { None },
                    }
                }
            } else {
                rsx! {
                    span {
                        class: "md-img-alt",
                        "{alt}"
                    }
                }
            }
        }
        MdInline::SoftBreak => rsx! { " " },
        MdInline::HardBreak => rsx! { br { class: "md-br" } },
    }
}

/// Render a single `MdBlock` to Dioxus RSX.
fn render_single_block(block: MdBlock, _key: usize) -> Element {
    match block {
        MdBlock::Heading { level, inlines } => match level {
            1 => rsx! { h1 { class: "md-h1", {render_inlines(inlines)} } },
            2 => rsx! { h2 { class: "md-h2", {render_inlines(inlines)} } },
            3 => rsx! { h3 { class: "md-h3", {render_inlines(inlines)} } },
            4 => rsx! { h4 { class: "md-h4", {render_inlines(inlines)} } },
            5 => rsx! { h5 { class: "md-h5", {render_inlines(inlines)} } },
            _ => rsx! { h6 { class: "md-h6", {render_inlines(inlines)} } },
        },
        MdBlock::Paragraph(inlines) => rsx! {
            p {
                class: "md-p",
                {render_inlines(inlines)}
            }
        },
        MdBlock::BlockQuote(blocks) => rsx! {
            blockquote {
                class: "md-blockquote",
                for (idx, b) in blocks.into_iter().enumerate() {
                    {render_single_block(b, idx)}
                }
            }
        },
        MdBlock::CodeBlock { lang, code } => {
            let class_attr = if lang.is_empty() {
                "md-code-block".to_string()
            } else {
                format!("md-code-block language-{lang}")
            };
            rsx! {
                pre {
                    class: "{class_attr}",
                    code {
                        class: "md-code",
                        "{code}"
                    }
                }
            }
        }
        MdBlock::List {
            ordered,
            start: _,
            items,
        } => {
            if ordered {
                rsx! {
                    ol {
                        class: "md-ol",
                        for (_idx, item) in items.into_iter().enumerate() {
                            li {
                                class: "md-li",
                                for (b_idx, b) in item.blocks.into_iter().enumerate() {
                                    {render_single_block(b, b_idx)}
                                }
                            }
                        }
                    }
                }
            } else {
                rsx! {
                    ul {
                        class: "md-ul",
                        for (_idx, item) in items.into_iter().enumerate() {
                            li {
                                class: "md-li",
                                for (b_idx, b) in item.blocks.into_iter().enumerate() {
                                    {render_single_block(b, b_idx)}
                                }
                            }
                        }
                    }
                }
            }
        }
        MdBlock::Rule => rsx! { hr { class: "md-hr" } },
    }
}

/// Render Markdown input string directly into Dioxus RSX Element.
///
/// Wraps all rendered elements in a container with class `"md-rendered"`.
pub fn render_markdown(input: &str) -> Element {
    let blocks = parse_markdown_to_blocks(input);
    rsx! {
        div {
            class: "md-rendered",
            for (idx, block) in blocks.into_iter().enumerate() {
                {render_single_block(block, idx)}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::core::{AttributeValue, DynamicNode, TemplateAttribute, TemplateNode};

    /// Escape HTML special characters for text nodes in HTML serialization.
    fn escape_html(s: &str, out: &mut String) {
        for c in s.chars() {
            match c {
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '&' => out.push_str("&amp;"),
                '"' => out.push_str("&quot;"),
                _ => out.push(c),
            }
        }
    }

    /// Serialize a Dioxus `Element` into its rendered HTML string for verification.
    fn render_to_html_string(el: Element) -> String {
        let mut out = String::new();
        if let Ok(vnode) = el {
            render_vnode(&vnode, &mut out);
        }
        out
    }

    fn render_vnode(vnode: &dioxus::core::VNode, out: &mut String) {
        for root in vnode.template.roots() {
            render_template_node(root, vnode, out);
        }
    }

    fn render_template_node(node: &TemplateNode, vnode: &dioxus::core::VNode, out: &mut String) {
        match node {
            TemplateNode::Element {
                tag, attrs, children, ..
            } => {
                out.push('<');
                out.push_str(tag);
                for attr in *attrs {
                    match attr {
                        TemplateAttribute::Static { name, value, .. } => {
                            out.push(' ');
                            out.push_str(name);
                            out.push_str("=\"");
                            escape_html(value, out);
                            out.push('"');
                        }
                        TemplateAttribute::Dynamic { id } => {
                            if let Some(dyn_attrs) = vnode.dynamic_attrs.get(*id) {
                                for a in dyn_attrs {
                                    match &a.value {
                                        AttributeValue::Text(val) => {
                                            out.push(' ');
                                            out.push_str(a.name);
                                            out.push_str("=\"");
                                            escape_html(val, out);
                                            out.push('"');
                                        }
                                        AttributeValue::Bool(true) => {
                                            out.push(' ');
                                            out.push_str(a.name);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
                out.push('>');
                for child in *children {
                    render_template_node(child, vnode, out);
                }
                out.push_str("</");
                out.push_str(tag);
                out.push('>');
            }
            TemplateNode::Text { text } => {
                escape_html(text, out);
            }
            TemplateNode::Dynamic { id } => {
                if let Some(dyn_node) = vnode.dynamic_nodes.get(*id) {
                    render_dynamic_node(dyn_node, out);
                }
            }
        }
    }

    fn render_dynamic_node(node: &DynamicNode, out: &mut String) {
        match node {
            DynamicNode::Text(vtext) => {
                escape_html(&vtext.value, out);
            }
            DynamicNode::Fragment(vnodes) => {
                for vn in vnodes {
                    render_vnode(vn, out);
                }
            }
            _ => {}
        }
    }

    // =========================================================================
    // S4: XSS Injection Vectors (Arbitration §7#4 verbatim assertions)
    // =========================================================================

    #[test]
    fn test_xss_vector_1_raw_script_tag() {
        // <script>alert(1)</script> -> output must not contain literal `<script>`
        let input = "<script>alert(1)</script>";
        let html = render_to_html_string(render_markdown(input));
        assert!(
            !html.contains("<script>"),
            "Output must not contain literal <script>, got: {html}"
        );
        assert!(
            !html.contains("alert(1)"),
            "Raw script content should be dropped, got: {html}"
        );
    }

    #[test]
    fn test_xss_vector_2_javascript_scheme_link() {
        // [click](javascript:alert(1)) -> output must not contain `href="javascript:"`
        let input = "[click](javascript:alert(1))";
        let html = render_to_html_string(render_markdown(input));
        assert!(
            !html.contains("href=\"javascript:"),
            "Output must not contain href=\"javascript:\", got: {html}"
        );
        assert!(
            !html.contains("<a "),
            "Non-whitelisted link must strip <a> tag, got: {html}"
        );
        assert!(
            html.contains("click"),
            "Link text should remain visible as plain text, got: {html}"
        );
    }

    #[test]
    fn test_xss_vector_3_data_scheme_image() {
        // ![x](data:text/html,<b>) -> output must not contain `src="data:text/html"`
        let input = "![x](data:text/html,<b>)";
        let html = render_to_html_string(render_markdown(input));
        assert!(
            !html.contains("src=\"data:text/html"),
            "Output must not contain src=\"data:text/html\", got: {html}"
        );
        assert!(
            !html.contains("<img"),
            "Non-whitelisted image must not render <img> tag, got: {html}"
        );
        assert!(
            html.contains("x"),
            "Image alt text should remain visible as fallback, got: {html}"
        );
    }

    #[test]
    fn test_xss_vector_4_raw_img_onerror() {
        // <img src=x onerror=alert(1)> -> output must not contain `onerror=`
        let input = "<img src=x onerror=alert(1)>";
        let html = render_to_html_string(render_markdown(input));
        assert!(
            !html.contains("onerror="),
            "Output must not contain onerror=, got: {html}"
        );
        assert!(
            !html.contains("<img"),
            "Raw HTML img must be explicitly dropped, got: {html}"
        );
    }

    #[test]
    fn test_xss_vector_5_vbscript_scheme_link() {
        // [a](vbscript:msgbox(1)) -> output must not contain `vbscript:`
        let input = "[a](vbscript:msgbox(1))";
        let html = render_to_html_string(render_markdown(input));
        assert!(
            !html.contains("vbscript:"),
            "Output must not contain vbscript:, got: {html}"
        );
        assert!(
            !html.contains("<a "),
            "Non-whitelisted vbscript link must strip <a> tag, got: {html}"
        );
        assert!(
            html.contains("a"),
            "Link text should remain visible as plain text, got: {html}"
        );
    }

    #[test]
    fn test_xss_vector_6_nested_script_tags() {
        // <scr<script>ipt> -> output must not contain executable `<script>`
        let input = "<scr<script>ipt>";
        let html = render_to_html_string(render_markdown(input));
        assert!(
            !html.contains("<script>"),
            "Output must not contain executable <script>, got: {html}"
        );
    }

    // =========================================================================
    // Standard Markdown Rendering Tests
    // =========================================================================

    #[test]
    fn test_render_paragraph() {
        let input = "This is a simple paragraph.";
        let html = render_to_html_string(render_markdown(input));
        assert_eq!(
            html,
            "<div class=\"md-rendered\"><p class=\"md-p\">This is a simple paragraph.</p></div>"
        );
    }

    #[test]
    fn test_render_headings_h1_to_h6() {
        let input = "# Heading 1\n## Heading 2\n### Heading 3\n#### Heading 4\n##### Heading 5\n###### Heading 6";
        let html = render_to_html_string(render_markdown(input));
        assert!(html.contains("<h1 class=\"md-h1\">Heading 1</h1>"));
        assert!(html.contains("<h2 class=\"md-h2\">Heading 2</h2>"));
        assert!(html.contains("<h3 class=\"md-h3\">Heading 3</h3>"));
        assert!(html.contains("<h4 class=\"md-h4\">Heading 4</h4>"));
        assert!(html.contains("<h5 class=\"md-h5\">Heading 5</h5>"));
        assert!(html.contains("<h6 class=\"md-h6\">Heading 6</h6>"));
    }

    #[test]
    fn test_render_unordered_list() {
        let input = "- Item 1\n- Item 2\n- Item 3";
        let html = render_to_html_string(render_markdown(input));
        assert!(html.contains("<ul class=\"md-ul\">"));
        assert!(html.contains("<li class=\"md-li\"><p class=\"md-p\">Item 1</p></li>"));
        assert!(html.contains("<li class=\"md-li\"><p class=\"md-p\">Item 2</p></li>"));
        assert!(html.contains("<li class=\"md-li\"><p class=\"md-p\">Item 3</p></li>"));
    }

    #[test]
    fn test_render_ordered_list() {
        let input = "1. First\n2. Second\n3. Third";
        let html = render_to_html_string(render_markdown(input));
        assert!(html.contains("<ol class=\"md-ol\">"));
        assert!(html.contains("<li class=\"md-li\"><p class=\"md-p\">First</p></li>"));
        assert!(html.contains("<li class=\"md-li\"><p class=\"md-p\">Second</p></li>"));
        assert!(html.contains("<li class=\"md-li\"><p class=\"md-p\">Third</p></li>"));
    }

    #[test]
    fn test_render_code_block() {
        let input = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let html = render_to_html_string(render_markdown(input));
        assert!(html.contains("<pre class=\"md-code-block language-rust\">"));
        assert!(html.contains("<code class=\"md-code\">fn main() {\n    println!(&quot;hello&quot;);\n}\n</code>"));
    }

    #[test]
    fn test_render_inline_code() {
        let input = "Use `let x = 1;` here.";
        let html = render_to_html_string(render_markdown(input));
        assert!(html.contains("<code class=\"md-inline-code\">let x = 1;</code>"));
    }

    #[test]
    fn test_render_links_whitelisted() {
        let input = "[Web](https://northing.app) and [Mail](mailto:info@northing.app) and [Phone](tel:+18001234567)";
        let html = render_to_html_string(render_markdown(input));
        assert!(html.contains(
            "<a class=\"md-link\" href=\"https://northing.app\" target=\"_blank\" rel=\"noopener noreferrer\">Web</a>"
        ));
        assert!(html.contains(
            "<a class=\"md-link\" href=\"mailto:info@northing.app\" target=\"_blank\" rel=\"noopener noreferrer\">Mail</a>"
        ));
        assert!(html.contains(
            "<a class=\"md-link\" href=\"tel:+18001234567\" target=\"_blank\" rel=\"noopener noreferrer\">Phone</a>"
        ));
    }

    #[test]
    fn test_render_emphasis_and_strong() {
        let input = "Some *emphasized* and **strong** text.";
        let html = render_to_html_string(render_markdown(input));
        assert!(html.contains("<em class=\"md-em\">emphasized</em>"));
        assert!(html.contains("<strong class=\"md-strong\">strong</strong>"));
    }

    #[test]
    fn test_render_blockquote() {
        let input = "> To be or not to be,\n> that is the question.";
        let html = render_to_html_string(render_markdown(input));
        assert!(html.contains("<blockquote class=\"md-blockquote\">"));
        assert!(html.contains("To be or not to be, that is the question."));
    }

    #[test]
    fn test_render_horizontal_rule() {
        let input = "Before\n\n---\n\nAfter";
        let html = render_to_html_string(render_markdown(input));
        assert!(html.contains("<hr class=\"md-hr\"></hr>"));
    }

    #[test]
    fn test_render_hard_and_soft_break() {
        let input = "Line one  \nLine two\nLine three";
        let html = render_to_html_string(render_markdown(input));
        assert!(html.contains("<br class=\"md-br\"></br>"));
        assert!(html.contains("Line two Line three"));
    }

    #[test]
    fn test_render_whitelisted_image() {
        let input = "![Logo](https://northing.app/logo.png)";
        let html = render_to_html_string(render_markdown(input));
        assert!(html.contains("<img class=\"md-img\" src=\"https://northing.app/logo.png\" alt=\"Logo\"></img>"));
    }

    #[test]
    fn test_sanitize_url_scheme_whitelist() {
        assert_eq!(sanitize_url_scheme("http://example.com"), Some("http://example.com"));
        assert_eq!(
            sanitize_url_scheme("https://example.com/path?q=1"),
            Some("https://example.com/path?q=1")
        );
        assert_eq!(
            sanitize_url_scheme("mailto:user@domain.com"),
            Some("mailto:user@domain.com")
        );
        assert_eq!(sanitize_url_scheme("tel:+1234567890"), Some("tel:+1234567890"));
        assert_eq!(sanitize_url_scheme("HTTP://EXAMPLE.COM"), Some("HTTP://EXAMPLE.COM"));
        assert_eq!(sanitize_url_scheme("HTTPS://EXAMPLE.COM"), Some("HTTPS://EXAMPLE.COM"));

        // Non-whitelisted schemes must return None
        assert_eq!(sanitize_url_scheme("javascript:alert(1)"), None);
        assert_eq!(sanitize_url_scheme("vbscript:msgbox(1)"), None);
        assert_eq!(sanitize_url_scheme("data:text/html,<b>hi</b>"), None);
        assert_eq!(sanitize_url_scheme("file:///etc/passwd"), None);
        assert_eq!(sanitize_url_scheme("blob:http://example.com/uuid"), None);
        assert_eq!(sanitize_url_scheme("intent://example.com"), None);
        assert_eq!(sanitize_url_scheme("custom-scheme:data"), None);
        assert_eq!(sanitize_url_scheme("/relative/path"), None);
        assert_eq!(sanitize_url_scheme("no_colon_path"), None);
        assert_eq!(sanitize_url_scheme(""), None);
    }
}
