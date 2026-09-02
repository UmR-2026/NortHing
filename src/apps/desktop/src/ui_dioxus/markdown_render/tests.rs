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
            '\'' => out.push_str("&#39;"),
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
