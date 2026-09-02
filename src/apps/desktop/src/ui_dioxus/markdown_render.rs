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
                // Known limitation: nested inline formatting inside image alt text is flattened to plain text.
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
                // ponytail: Options::empty() 下其余 Tag::* 不可达；升级路径 = pulldown-cmark 升版或扩展 GFM 时改穷尽匹配
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
                // ponytail: Options::empty() 下其余 TagEnd::* 不可达；升级路径 = pulldown-cmark 升版或扩展 GFM 时改穷尽匹配
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
            // ponytail: Options::empty() 下其余 Event::* (FootnoteReference/TaskListMarker/...) 不可达；升级路径 = pulldown-cmark 升版或扩展 GFM 时改穷尽匹配
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
fn render_single_inline(inline: MdInline, key: usize) -> Element {
    match inline {
        MdInline::Text(text) => rsx! {
            "{text}"
        },
        MdInline::Code(code) => rsx! {
            code {
                key: "{key}",
                class: "md-inline-code",
                "{code}"
            }
        },
        MdInline::Strong(children) => rsx! {
            strong {
                key: "{key}",
                class: "md-strong",
                {render_inlines(children)}
            }
        },
        MdInline::Emphasis(children) => rsx! {
            em {
                key: "{key}",
                class: "md-em",
                {render_inlines(children)}
            }
        },
        MdInline::Link { url, title, children } => {
            if let Some(safe_url) = sanitize_url_scheme(&url) {
                let has_title = !title.is_empty();
                rsx! {
                    a {
                        key: "{key}",
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
                        key: "{key}",
                        class: "md-img",
                        src: "{safe_url}",
                        alt: "{alt}",
                        title: if has_title { Some(title) } else { None },
                    }
                }
            } else {
                rsx! {
                    span {
                        key: "{key}",
                        class: "md-img-alt",
                        "{alt}"
                    }
                }
            }
        }
        MdInline::SoftBreak => rsx! { " " },
        MdInline::HardBreak => rsx! { br { key: "{key}", class: "md-br" } },
    }
}

/// Render a single `MdBlock` to Dioxus RSX.
fn render_single_block(block: MdBlock, key: usize) -> Element {
    match block {
        MdBlock::Heading { level, inlines } => match level {
            1 => rsx! { h1 { key: "{key}", class: "md-h1", {render_inlines(inlines)} } },
            2 => rsx! { h2 { key: "{key}", class: "md-h2", {render_inlines(inlines)} } },
            3 => rsx! { h3 { key: "{key}", class: "md-h3", {render_inlines(inlines)} } },
            4 => rsx! { h4 { key: "{key}", class: "md-h4", {render_inlines(inlines)} } },
            5 => rsx! { h5 { key: "{key}", class: "md-h5", {render_inlines(inlines)} } },
            _ => rsx! { h6 { key: "{key}", class: "md-h6", {render_inlines(inlines)} } },
        },
        MdBlock::Paragraph(inlines) => rsx! {
            p {
                key: "{key}",
                class: "md-p",
                {render_inlines(inlines)}
            }
        },
        MdBlock::BlockQuote(blocks) => rsx! {
            blockquote {
                key: "{key}",
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
                    key: "{key}",
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
                        key: "{key}",
                        class: "md-ol",
                        for (idx, item) in items.into_iter().enumerate() {
                            li {
                                key: "{idx}",
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
                        key: "{key}",
                        class: "md-ul",
                        for (idx, item) in items.into_iter().enumerate() {
                            li {
                                key: "{idx}",
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
        MdBlock::Rule => rsx! { hr { key: "{key}", class: "md-hr" } },
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

#[cfg(test)] mod tests;
