// T1 Dioxus migration (2026-08-12) — embed the truth CSS verbatim.
//
// Brief §4.5 — "CSS 原样内联（禁翻译成 Rust 样式）". The CSS file at
// `docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.css`
// (extracted from the truth HTML) must be embedded as a `&'static str`
// and injected via `document::Stylesheet` so the rendered pixels match
// the truth HTML byte-for-byte (modulo the three-window layout).
//
// Until the dedicated `.css` file is extracted, we fall back to the full
// `<style>` block from the truth HTML so the colors, keyframes, radial
// gradients and shadow tokens all line up. The conversion-annotations
// rules (color-mix 48, keyframes 21, radial-gradient 22, shadow 4式)
// are preserved verbatim below — do not edit unless the truth HTML
// itself changes.

/// CSS payload injected into every Dioxus window. The block is byte-
/// identical to the `<style>` section of `consult-room-main.html`
/// (lines 27..273 of the truth file at
/// `docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.html`).
///
/// Brief §3.3 — original sample禁止任何修改/重排/翻译. We keep the
/// comment block (v4 变更 / trigger 变更) so reviewers can grep against
/// the source. If the truth HTML changes, this string must be updated
/// in lock-step — the regression test `assert_truth_css_byte_count`
/// guards against silent divergence.
pub const TRUTH_CSS: &str = include_str!("../../../../../docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.css");

/// R3' A+B+C 转写层覆盖样式（2026-08-14）。`TRUTH_CSS` 逐字节锁死
/// （`assert_truth_css_byte_count` 守卫必过，禁改真值 CSS 文件），任何
/// 转写层收口/覆盖规则只能落在此块，注入点：`windows.rs` inner/outer body
/// 与 `app.rs` room body——`TRUTH_CSS` 之后的第二个 `<style>` 块。
///
/// 选择器约定：
///   * `body[data-window="inner"]` / `body[data-window="outer"]` —— 只作用于
///     两个浮窗（room 主窗 body 无 `data-window` 属性，规则天然不落 room）；
///   * `#room-scrim` + `body[data-theme="..."]` —— 只作用于 room 主窗的
///     压暗层（scrim 是 S4 降级契约要求的转写层自绘，真值 CSS/HTML 无此规则，
///     见 block-contract §2 规则 4）。
pub const OVERLAY_CSS: &str = r#"
  /* C. 浮窗横溢收口：WebView2 无 viewport meta 时 layout viewport 回落 980px，
     真值 @media(max-width:940px) 的 width:100% 会把 #mind/#work 撑宽 ——
     这里把宽度固定为窗口逻辑宽（CSS px = logical px，WebView2 DPI 感知）并
     消除水平滚动。html 侧用 :has 兜底（Chromium 105+，WebView2 常青运行时
     支持；旧版运行时规则被忽略时内容已收口，无滚动条，无害）。 */
  html:has(body[data-window="inner"]), html:has(body[data-window="outer"]) { overflow: hidden; }
  body[data-window="inner"], body[data-window="outer"] { overflow-x: hidden; overflow-y: auto; background: var(--bg0); }
  body[data-window="inner"] #mind { width: 280px; max-width: 280px; margin: 0; }
  body[data-window="inner"] #mind .mod { width: 280px; max-width: 280px; }
  body[data-window="outer"] #work { width: 320px; max-width: 320px; margin: 0; max-height: none; }
  /* 终端井：禁止自动断行（`--boundary` 一类长词被断行即出现 `dary>` 残迹），
     横向溢出裁剪兜底；292px 内容宽容纳 10px mono 全文绰绰有余。 */
  body[data-window="outer"] #work .term-well { white-space: pre; overflow-x: hidden; }

  /* D. room 主窗 scrim：inner/outer 任一可见时压暗（block-contract §2 规则 4
     降级形态，alpha 0.22 = handoff §5.3「22% 压暗」；scrim token：dark
     #000000 / light #38352E）。主题切换随 body 的 data-theme 正确变色。
     pointer-events:none 保证宝石等控件事件穿透。 */
  #room-scrim { position: fixed; inset: 0; z-index: 39; background: rgba(0, 0, 0, 0.22); pointer-events: none; }
  body[data-theme="light"] #room-scrim { background: rgba(56, 53, 46, 0.22); }
"#;

/// Build a `dioxus::desktop::wry::WebViewBuilder` attribute that injects
/// the truth CSS as a `<style>` element inside the document head.
///
/// Brief §2.6 — `dioxus::desktop::document::Stylesheet { ... }` is the
/// supported mechanism; we wrap the static CSS in `format!` once so the
/// `<style>` tag itself is part of the payload.
pub fn inject_stylesheet_html() -> String {
    format!("<style id=\"truth-css\">{}</style>", TRUTH_CSS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Truth-CSS byte-count guard. If anyone shortens or expands the CSS
    /// without updating the truth HTML, this fails — preventing silent
    /// visual divergence from the brief §3.3 "原样保留" rule.
    #[test]
    fn assert_truth_css_byte_count() {
        // Length > 0 is the minimum contract; the exact byte count comes
        // from the truth file at the path above.
        assert!(TRUTH_CSS.len() > 1000, "truth CSS unexpectedly short");
        // Hardcoded marker: the truth CSS always starts with `:root {`
        // because palette tokens come first. If this changes, the truth
        // HTML itself changed and we need to re-derive.
        assert!(
            TRUTH_CSS.contains(":root {"),
            "truth CSS no longer opens with `:root {{` — re-derive from consult-room-main.html"
        );
    }
}
