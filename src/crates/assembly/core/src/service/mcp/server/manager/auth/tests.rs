use super::auth_types::{escape_html, resolve_oauth_callback_locale, OAuthCallbackLocale};

#[test]
fn escape_html_replaces_all_special_chars() {
    assert_eq!(escape_html("a<b>&c\"d'e"), "a&lt;b&gt;&amp;c&quot;d&#39;e");
}

#[test]
fn resolve_oauth_callback_locale_prefers_preferred_language() {
    assert!(matches!(
        resolve_oauth_callback_locale(Some("zh-TW"), Some("en-US")),
        OAuthCallbackLocale::ZhTW
    ));
}

#[test]
fn resolve_oauth_callback_locale_falls_back_to_accept_language() {
    assert!(matches!(
        resolve_oauth_callback_locale(None, Some("en-US")),
        OAuthCallbackLocale::EnUS
    ));
}

#[test]
fn resolve_oauth_callback_locale_defaults_to_zh_cn() {
    assert!(matches!(
        resolve_oauth_callback_locale(None, None),
        OAuthCallbackLocale::ZhCN
    ));
}
