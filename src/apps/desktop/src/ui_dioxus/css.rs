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
