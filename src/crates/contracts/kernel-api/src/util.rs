//! Utility free functions.

/// Strip prompt markup (abnormal item 10 solution: unified entry point).
/// Source: #19
pub fn strip_prompt_markup(text: &str) -> String {
    // Design draft — implementation deferred to K2
    text.trim().to_string()
}
