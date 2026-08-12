// T1 Dioxus migration (2026-08-12) — fluent string lookup for the
// consult-room shell.
//
// The three `.ftl` files at `src/crates/assembly/core/locales/` are the
// source of truth for every UI string. This module reads them at startup
// and exposes a `t(key)` helper that returns the active-locale string
// (zh-CN by default; en-US / zh-TW are the other two as required by the
// i18n:audit baseline parity check).
//
// The locale selection mirrors the existing Slint shell behavior — read
// `northhing_core::kernel_facade` for the active locale id, fall back to
// "zh-CN" if unavailable. The strings added by this task follow the
// `dioxus-room-*` prefix so they're easy to grep and audit.

use std::collections::HashMap;

/// Locale id used as fallback when the kernel does not report one. Mirrors
/// the default in the Slint shell's `AppStrings` global.
pub const DEFAULT_LOCALE: &str = "zh-CN";

/// A very small fluent subset. We don't pull in `fluent-bundle` because
/// the consult-room shell has a flat set of keys (no plural variants, no
/// nested message references) and the existing shell hard-codes its
/// strings the same way. If future tasks need real fluent features, swap
/// this for `fluent-bundle::FluentBundle` (already in workspace deps).
#[derive(Debug, Default)]
pub struct LocalePack {
    by_key: HashMap<String, String>,
    locale: String,
}

impl LocalePack {
    /// Load a locale pack by reading the matching `.ftl` file from the
    /// shared locales directory. Keys already exist for the consult-room
    /// shell (`dioxus-room-*`); this function parses just those keys
    /// out of the file so we don't depend on a full fluent parser.
    pub fn load(locale: &str) -> Self {
        let mut pack = LocalePack {
            by_key: HashMap::new(),
            locale: locale.to_string(),
        };
        if let Ok(text) = std::fs::read_to_string(locale_path(locale)) {
            parse_flat_keys(&text, &mut pack.by_key);
        }
        pack
    }

    /// Lookup a key. Falls back to the key itself if not present — this
    /// surfaces untranslated strings rather than swallowing them.
    ///
    /// The returned `&str` is borrowed from `self` (when the key is in
    /// the map) or from the `key` argument (when it's not). The function
    /// returns `Cow<'_, str>` so both cases are covered with explicit
    /// lifetimes.
    pub fn t<'a>(&'a self, key: &'a str) -> std::borrow::Cow<'a, str> {
        match self.by_key.get(key) {
            Some(v) => std::borrow::Cow::Borrowed(v.as_str()),
            None => std::borrow::Cow::Borrowed(key),
        }
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }
}

/// Locate the locale file inside the workspace's assembly core.
fn locale_path(locale: &str) -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR is the desktop crate root.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("..") // apps/
        .join("..") // crates/
        .join("..") // src/
        .join("crates")
        .join("assembly")
        .join("core")
        .join("locales")
        .join(format!("{locale}.ftl"))
}

/// Minimal flat-key parser. Reads `key = value` lines and ignores
/// continuations / multi-line / variants — none of which the consult-
/// room shell uses today.
fn parse_flat_keys(text: &str, out: &mut HashMap<String, String>) {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim().to_string();
            // Strip surrounding quotes if present; the locale files use
            // bare strings for the consult-room keys.
            let raw_value = v.trim();
            let value = raw_value
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(raw_value)
                .to_string();
            out.insert(key, value);
        }
    }
}

/// Keys used by the consult-room shell. Adding a key here forces both an
/// entry in `strings.rs` and a translation in each `.ftl` file (the
/// `i18n:audit` parity check enforces the latter).
pub mod keys {
    // ===== Window chrome / state pill =====
    pub const WINDOW_TITLE_ROOM: &str = "dioxus-room-window-title";
    pub const WINDOW_TITLE_INNER: &str = "dioxus-room-inner-window-title";
    pub const WINDOW_TITLE_OUTER: &str = "dioxus-room-outer-window-title";
    pub const STATE_PILL_DRIVE: &str = "dioxus-room-state-drive";

    // ===== Status row (room-status) =====
    pub const STATUS_IDENTITY: &str = "dioxus-room-status-identity";
    pub const STATUS_CONTEXT: &str = "dioxus-room-status-context";

    // ===== Room head =====
    pub const ROOM_HEAD_NAME: &str = "dioxus-room-head-name";
    pub const ROOM_HEAD_INITIAL: &str = "dioxus-room-head-initial";
    pub const ROOM_HEAD_STATE: &str = "dioxus-room-head-state";

    // ===== Mock chat session =====
    pub const SESSION_BANNER: &str = "dioxus-room-session-banner";
    pub const AGENT_WHO: &str = "dioxus-room-agent-who";
    pub const AGENT_BODY: &str = "dioxus-room-agent-body";
    pub const AGENT_TOOL_LOG: &str = "dioxus-room-agent-tool-log";
    pub const AGENT_ARTIFACT_CHIP: &str = "dioxus-room-agent-artifact-chip";
    pub const AGENT_BODY_2: &str = "dioxus-room-agent-body-2";
    pub const WITNESS_WHO: &str = "dioxus-room-witness-who";
    pub const WITNESS_BODY: &str = "dioxus-room-witness-body";
    pub const APPROVAL_HEAD: &str = "dioxus-room-approval-head";
    pub const APPROVAL_MAIN: &str = "dioxus-room-approval-main";
    pub const APPROVAL_RISK: &str = "dioxus-room-approval-risk";
    pub const APPROVAL_APPROVE: &str = "dioxus-room-approval-approve";
    pub const APPROVAL_REJECT: &str = "dioxus-room-approval-reject";
    pub const APPROVAL_HEAD_2: &str = "dioxus-room-approval-head-2";
    pub const APPROVAL_MAIN_2: &str = "dioxus-room-approval-main-2";
    pub const APPROVAL_STATE: &str = "dioxus-room-approval-state";

    // ===== Deck / input =====
    pub const DECK_ATTACH: &str = "dioxus-room-deck-attach";
    pub const DECK_PLACEHOLDER: &str = "dioxus-room-deck-placeholder";
    pub const DECK_WITNESS_NOTE: &str = "dioxus-room-deck-witness-note";
    pub const DECK_SEND: &str = "dioxus-room-deck-send";
    pub const DECK_SEND_STREAMING: &str = "dioxus-room-deck-send-streaming";

    // ===== Vertical label (room-handle vlabel) =====
    pub const VLABEL_INNER: &str = "dioxus-room-vlabel-inner";
    pub const VLABEL_OUTER: &str = "dioxus-room-vlabel-outer";

    // ===== Inner / outer station heads =====
    pub const INNER_HEAD_TITLE: &str = "dioxus-room-inner-head-title";
    pub const INNER_HEAD_FACILITY_TITLE: &str = "dioxus-room-inner-head-facility-title";
    pub const INNER_SECTION_SEDIMENT_TITLE: &str = "dioxus-room-inner-section-sediment-title";
    pub const INNER_SECTION_SEDIMENT_EM: &str = "dioxus-room-inner-section-sediment-em";
    pub const INNER_SECTION_SEDIMENT_NOTE: &str = "dioxus-room-inner-section-sediment-note";
    pub const INNER_SECTION_ENGINE_TITLE: &str = "dioxus-room-inner-section-engine-title";
    pub const INNER_SECTION_ENGINE_EM: &str = "dioxus-room-inner-section-engine-em";
    pub const INNER_SECTION_CONTEXT_TITLE: &str = "dioxus-room-inner-section-context-title";
    pub const INNER_SECTION_CONTEXT_EM: &str = "dioxus-room-inner-section-context-em";
    pub const INNER_SECTION_AXIOMS_TITLE: &str = "dioxus-room-inner-section-axioms-title";
    pub const INNER_SECTION_AXIOMS_EM: &str = "dioxus-room-inner-section-axioms-em";
    pub const INNER_SECTION_RAG_TITLE: &str = "dioxus-room-inner-section-rag-title";
    pub const INNER_SECTION_RAG_EM: &str = "dioxus-room-inner-section-rag-em";
    pub const INNER_RAG_MOUNTED: &str = "dioxus-room-inner-rag-mounted";
    pub const INNER_GLOBAL_SETTINGS: &str = "dioxus-room-inner-global-settings";

    pub const OUTER_HEAD_TITLE: &str = "dioxus-room-outer-head-title";
    pub const OUTER_SECTION_ROUTING_TITLE: &str = "dioxus-room-outer-section-routing-title";
    pub const OUTER_SECTION_ROUTING_EM: &str = "dioxus-room-outer-section-routing-em";
    pub const OUTER_SECTION_ROUTING_INTERVENING: &str = "dioxus-room-outer-routing-intervening";
    pub const OUTER_SECTION_ROUTING_STANDBY: &str = "dioxus-room-outer-routing-standby";
    pub const OUTER_SECTION_PLANNER_TITLE: &str = "dioxus-room-outer-section-planner-title";
    pub const OUTER_SECTION_PLANNER_EM: &str = "dioxus-room-outer-section-planner-em";
    pub const OUTER_SECTION_PLANNER_INPROGRESS: &str = "dioxus-room-outer-planner-inprogress";
    pub const OUTER_SECTION_DIFF_TITLE: &str = "dioxus-room-outer-section-diff-title";
    pub const OUTER_SECTION_DIFF_EM: &str = "dioxus-room-outer-section-diff-em";
    pub const OUTER_DIFF_REVERTED: &str = "dioxus-room-outer-diff-reverted";
    pub const OUTER_TERMINAL_PROMPT: &str = "dioxus-room-outer-terminal-prompt";

    // ===== Empty states (F5 — F6 contract) =====
    pub const EMPTY_CHAT_FLOW: &str = "dioxus-room-empty-chat-flow";
    pub const EMPTY_STREAMING_INTERRUPT: &str = "dioxus-room-empty-streaming-interrupt";
    pub const EMPTY_PROVIDER_TEST_FAILED: &str = "dioxus-room-empty-provider-test-failed";
    pub const EMPTY_APPROVAL_TIMEOUT: &str = "dioxus-room-empty-approval-timeout";
}
