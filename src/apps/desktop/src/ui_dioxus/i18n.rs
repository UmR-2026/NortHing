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
    ///
    /// R3' resilience: corrupt locale files (GBK mojibake, \r\r\n line
    /// endings) produce empty key maps rather than panicking. A warning
    /// is emitted so the corruption is visible in logs without blocking
    /// the Dioxus shell from launching.
    pub fn load(locale: &str) -> Self {
        let mut pack = LocalePack {
            by_key: HashMap::new(),
            locale: locale.to_string(),
        };
        match std::fs::read_to_string(locale_path(locale)) {
            Ok(text) => {
                parse_flat_keys(&text, &mut pack.by_key);
                let path = locale_path(locale);
                tracing::info!(
                    "ui_dioxus/i18n: loaded locale {locale} from {} ({} keys)",
                    path.display(),
                    pack.by_key.len()
                );
            }
            Err(e) => {
                tracing::warn!(
                    "ui_dioxus/i18n: failed to read locale file for {locale}: {e}; \
                     falling back to empty key map (strings will show key names)"
                );
            }
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
///
/// `CARGO_MANIFEST_DIR` is the desktop crate root (`<repo>/src/apps/desktop`).
/// The shared locale directory lives at `<repo>/src/crates/assembly/core/locales/`
/// (same prefix `src/`) - two `..` segments up from the manifest dir land on
/// `<repo>/src/`, then we descend into `crates/assembly/core/locales/`.
///
/// R3' Bug A fix (2026-08-13): earlier this function used *three* `..`
/// segments, which resolved to `<repo>/crates/assembly/core/locales/` -
/// a directory that does not exist (the actual path is under `src/`).
/// The bug silently turned every `LocalePack::load` into an `Err`, the
/// fallback swallowed it, and the consult-room shell rendered its
/// fluent strings as raw key names. Dropping the extra `..` resolves
/// the path to the real locale directory.
fn locale_path(locale: &str) -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("..") // src/apps/
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
    pub const STATUS_PILL: &str = "dioxus-room-status-pill";

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
    // R6 (2026-08-14 用户判决): DECK_WITNESS_NOTE 用法已从 app.rs 移除
    // （见证说明删除）；key 保留为词表资产，locale 词条不动。
    // 2026-08-22（审查 M1 修）：DECK_ATTACH/DECK_SEND/DECK_SEND_STREAMING
    // 复活——attach/send/stop 钮的 aria-label 改走 locale 键。
    pub const DECK_ATTACH: &str = "dioxus-room-deck-attach";
    pub const DECK_PLACEHOLDER: &str = "dioxus-room-deck-placeholder";
    #[allow(dead_code)]
    pub const DECK_WITNESS_NOTE: &str = "dioxus-room-deck-witness-note";
    pub const DECK_SEND: &str = "dioxus-room-deck-send";
    pub const DECK_SEND_STREAMING: &str = "dioxus-room-deck-send-streaming";

    // ===== Vertical label (room-handle vlabel) =====
    // R6.2: 竖签元素已从 app.rs 移除（用户判决「字变得多余」）；
    // key 保留为词表资产。
    #[allow(dead_code)]
    pub const VLABEL_INNER: &str = "dioxus-room-vlabel-inner";
    #[allow(dead_code)]
    pub const VLABEL_OUTER: &str = "dioxus-room-vlabel-outer";

    // ===== Inner / outer station heads =====
    #[allow(dead_code)]
    pub const INNER_HEAD_TITLE: &str = "dioxus-room-inner-head-title";
    #[allow(dead_code)]
    pub const INNER_HEAD_FACILITY_TITLE: &str = "dioxus-room-inner-head-facility-title";
    pub const INNER_SECTION_SEDIMENT_TITLE: &str = "dioxus-room-inner-section-sediment-title";
    pub const INNER_SECTION_SEDIMENT_EM: &str = "dioxus-room-inner-section-sediment-em";
    pub const INNER_SECTION_SEDIMENT_NOTE: &str = "dioxus-room-inner-section-sediment-note";
    // W2 视觉解耦（2026-08-21）：ENGINE/CONTEXT 两节并入 RUNTIME 卡，
    // 四个 section 键不再被引用；key 与 locale 词条保留为词表资产
    // （同 DECK_ATTACH 先例），i18n:audit 基线不受影响。
    #[allow(dead_code)]
    pub const INNER_SECTION_ENGINE_TITLE: &str = "dioxus-room-inner-section-engine-title";
    #[allow(dead_code)]
    pub const INNER_SECTION_ENGINE_EM: &str = "dioxus-room-inner-section-engine-em";
    #[allow(dead_code)]
    pub const INNER_SECTION_CONTEXT_TITLE: &str = "dioxus-room-inner-section-context-title";
    #[allow(dead_code)]
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

    // ===== W2 视觉解耦（2026-08-21，用户定案 §2.2）=====
    // 设施窗 RUNTIME 卡（模型引擎 + 上下文 + 全局状态合并卡）。
    pub const INNER_SECTION_RUNTIME_TITLE: &str = "dioxus-room-inner-section-runtime-title";
    pub const INNER_SECTION_RUNTIME_EM: &str = "dioxus-room-inner-section-runtime-em";
    // RUNTIME 卡 token 消耗行 + 清空动作。
    pub const INNER_RUNTIME_TOKEN_USAGE: &str = "dioxus-room-inner-runtime-token-usage";
    pub const INNER_RUNTIME_TOKEN_CLEAR: &str = "dioxus-room-inner-runtime-token-clear";
    // 沉积窗「沉积skill」卡：agent 自己发掘、可整理成新 skill 的候选
    // 清单（≠ settings 能力集），mock 三条候选 + 两态状态词。
    pub const INNER_SECTION_SKILL_TITLE: &str = "dioxus-room-inner-section-skill-title";
    pub const INNER_SECTION_SKILL_EM: &str = "dioxus-room-inner-section-skill-em";
    pub const INNER_SKILL_CAND_1: &str = "dioxus-room-inner-skill-cand-1";
    pub const INNER_SKILL_CAND_2: &str = "dioxus-room-inner-skill-cand-2";
    pub const INNER_SKILL_CAND_3: &str = "dioxus-room-inner-skill-cand-3";
    pub const INNER_SKILL_STAT_SHAPE: &str = "dioxus-room-inner-skill-stat-shape";
    pub const INNER_SKILL_STAT_WATCH: &str = "dioxus-room-inner-skill-stat-watch";

    // ===== chrome 控件文案（2026-08-22，审查 M1 + 终审 Minor×2 合并修）=====
    // 模块窗 fold/close 钮（箭头 ▴/▾ 为形态字面量留在代码，词走键）。
    pub const WINDOW_FOLD_BTN: &str = "dioxus-room-window-fold-btn";
    pub const WINDOW_CLOSE_BTN: &str = "dioxus-room-window-close-btn";
    // room chrome 控件簇（主题/最小化/最大化/关闭/中枢缝折叠两态）。
    pub const CHROME_THEME_TOGGLE: &str = "dioxus-room-chrome-theme-toggle";
    pub const CHROME_MINIMIZE: &str = "dioxus-room-chrome-minimize";
    pub const CHROME_MAXIMIZE: &str = "dioxus-room-chrome-maximize";
    pub const CHROME_CLOSE: &str = "dioxus-room-chrome-close";
    pub const CHROME_HEAD_FOLD: &str = "dioxus-room-chrome-head-fold";
    pub const CHROME_HEAD_UNFOLD: &str = "dioxus-room-chrome-head-unfold";
    // 宝石（模块窗唤起件）aria-label/title；左结文案随 W2 改名联动
    // （self 窗 = 「沉积」，旧「它的自我」已无引用对象）。
    pub const GEM_LEFT_LABEL: &str = "dioxus-room-gem-left-label";
    pub const GEM_LEFT_TITLE: &str = "dioxus-room-gem-left-title";
    pub const GEM_RIGHT_LABEL: &str = "dioxus-room-gem-right-label";
    pub const GEM_RIGHT_TITLE: &str = "dioxus-room-gem-right-title";

    // ===== Archive window (2026-08-24 Task EF-E1) =====
    pub const NAV_ARCHIVE: &str = "dioxus-room-nav-archive";
    pub const ARCHIVE_WINDOW_TITLE: &str = "dioxus-room-archive-window-title";
    pub const ARCHIVE_HEAD_NAME: &str = "dioxus-room-archive-head-name";
    pub const ARCHIVE_HEAD_INITIAL: &str = "dioxus-room-archive-head-initial";
    pub const ARCHIVE_HEAD_STATE: &str = "dioxus-room-archive-head-state";
    pub const ARCHIVE_STATUS_MODE: &str = "dioxus-room-archive-status-mode";
    pub const ARCHIVE_STATUS_TAG: &str = "dioxus-room-archive-status-tag";
    pub const ARCHIVE_SECTION_DEPTH_TITLE: &str = "dioxus-room-archive-section-depth-title";
    pub const ARCHIVE_SECTION_DEPTH_EM: &str = "dioxus-room-archive-section-depth-em";
    pub const ARCHIVE_SECTION_SOLAR_TITLE: &str = "dioxus-room-archive-section-solar-title";
    pub const ARCHIVE_SECTION_SOLAR_EM: &str = "dioxus-room-archive-section-solar-em";
    pub const ARCHIVE_SECTION_WITNESS_TITLE: &str = "dioxus-room-archive-section-witness-title";
    pub const ARCHIVE_SECTION_WITNESS_EM: &str = "dioxus-room-archive-section-witness-em";
    pub const ARCHIVE_FOOT_NOTE: &str = "dioxus-room-archive-foot-note";

    // ===== Space window (2026-08-24 Task EF-E2) =====
    pub const NAV_SPACE: &str = "dioxus-room-nav-space";
    pub const SPACE_WINDOW_TITLE: &str = "dioxus-room-space-window-title";
    pub const SPACE_HEAD_NAME: &str = "dioxus-room-space-head-name";
    pub const SPACE_HEAD_STATE: &str = "dioxus-room-space-head-state";
    pub const SPACE_HEAD_NOTE: &str = "dioxus-room-space-head-note";
    pub const SPACE_STATUS_CORRIDOR: &str = "dioxus-room-space-status-corridor";
    pub const SPACE_STATUS_ONE_LIT: &str = "dioxus-room-space-status-one-lit";
    pub const SPACE_STATUS_REST_DIM: &str = "dioxus-room-space-status-rest-dim";
    pub const SPACE_SECTION_ORDER_TITLE: &str = "dioxus-room-space-section-order-title";
    pub const SPACE_SECTION_ORDER_EM: &str = "dioxus-room-space-section-order-em";
    pub const SPACE_SECTION_WORKSPACE_TITLE: &str = "dioxus-room-space-section-workspace-title";
    pub const SPACE_SECTION_WORKSPACE_EM: &str = "dioxus-room-space-section-workspace-em";
    pub const SPACE_SECTION_DISPLAY_TITLE: &str = "dioxus-room-space-section-display-title";
    pub const SPACE_SECTION_DISPLAY_EM: &str = "dioxus-room-space-section-display-em";
    pub const SPACE_SECTION_PEEK_TITLE: &str = "dioxus-room-space-section-peek-title";
    pub const SPACE_SECTION_PEEK_EM: &str = "dioxus-room-space-section-peek-em";
    pub const SPACE_BTN_ARCHIVE_LINK: &str = "dioxus-room-space-btn-archive-link";

    // ===== Settings window (2026-08-24 Task EF-E3) =====
    pub const SETTINGS_WINDOW_TITLE: &str = "dioxus-room-settings-window-title";
    pub const SETTINGS_HEAD_SELF: &str = "dioxus-room-settings-head-self";
    pub const SETTINGS_HEAD_FACILITY: &str = "dioxus-room-settings-head-facility";
    pub const SETTINGS_SECTION_SEDIMENT_TITLE: &str = "dioxus-room-settings-section-sediment-title";
    pub const SETTINGS_SECTION_SEDIMENT_EM: &str = "dioxus-room-settings-section-sediment-em";
    pub const SETTINGS_SECTION_CHRONICLES_TITLE: &str = "dioxus-room-settings-section-chronicles-title";
    pub const SETTINGS_SECTION_CHRONICLES_EM: &str = "dioxus-room-settings-section-chronicles-em";
    pub const SETTINGS_SECTION_IDENTITY_TITLE: &str = "dioxus-room-settings-section-identity-title";
    pub const SETTINGS_SECTION_IDENTITY_EM: &str = "dioxus-room-settings-section-identity-em";
    pub const SETTINGS_SECTION_AXIOMS_TITLE: &str = "dioxus-room-settings-section-axioms-title";
    pub const SETTINGS_SECTION_AXIOMS_EM: &str = "dioxus-room-settings-section-axioms-em";
    pub const SETTINGS_SECTION_ENGINE_TITLE: &str = "dioxus-room-settings-section-engine-title";
    pub const SETTINGS_SECTION_ENGINE_EM: &str = "dioxus-room-settings-section-engine-em";
    pub const SETTINGS_SECTION_CONTEXT_TITLE: &str = "dioxus-room-settings-section-context-title";
    pub const SETTINGS_SECTION_CONTEXT_EM: &str = "dioxus-room-settings-section-context-em";
    pub const SETTINGS_SECTION_PROVIDER_TITLE: &str = "dioxus-room-settings-section-provider-title";
    pub const SETTINGS_SECTION_PROVIDER_EM: &str = "dioxus-room-settings-section-provider-em";
    pub const SETTINGS_SECTION_MCP_TITLE: &str = "dioxus-room-settings-section-mcp-title";
    pub const SETTINGS_SECTION_MCP_EM: &str = "dioxus-room-settings-section-mcp-em";
    pub const SETTINGS_SECTION_WORKSPACE_TITLE: &str = "dioxus-room-settings-section-workspace-title";
    pub const SETTINGS_SECTION_WORKSPACE_EM: &str = "dioxus-room-settings-section-workspace-em";
    pub const SETTINGS_SECTION_DISPLAY_TITLE: &str = "dioxus-room-settings-section-display-title";
    pub const SETTINGS_SECTION_DISPLAY_EM: &str = "dioxus-room-settings-section-display-em";
    pub const SETTINGS_BTN_RELOCATE: &str = "dioxus-room-settings-btn-relocate";
    // ===== Settings engine/provider/MCP entries (2026-08-25, gap audit) =====
    pub const SETTINGS_ENGINE_CLAUDE: &str = "dioxus-room-settings-engine-claude";
    pub const SETTINGS_ENGINE_GEMINI: &str = "dioxus-room-settings-engine-gemini";
    pub const SETTINGS_ENGINE_GPT4O: &str = "dioxus-room-settings-engine-gpt4o";
    pub const SETTINGS_ENGINE_CURRENT: &str = "dioxus-room-settings-engine-current";
    pub const SETTINGS_PROVIDER_ANTHROPIC: &str = "dioxus-room-settings-provider-anthropic";
    pub const SETTINGS_PROVIDER_GOOGLE: &str = "dioxus-room-settings-provider-google";
    pub const SETTINGS_PROVIDER_DIRECT: &str = "dioxus-room-settings-provider-direct";
    pub const SETTINGS_MCP_FILESYSTEM: &str = "dioxus-room-settings-mcp-filesystem";
    pub const SETTINGS_MCP_PHILOSOPHY: &str = "dioxus-room-settings-mcp-philosophy";
    pub const SETTINGS_MCP_TERMINAL: &str = "dioxus-room-settings-mcp-terminal";
    pub const SETTINGS_MCP_READWRITE: &str = "dioxus-room-settings-mcp-readwrite";
    pub const SETTINGS_MCP_PLUGIN: &str = "dioxus-room-settings-mcp-plugin";
    pub const SETTINGS_MCP_UNAUTHORIZED: &str = "dioxus-room-settings-mcp-unauthorized";
    pub const SETTINGS_WORKSPACE_PATH: &str = "dioxus-room-settings-workspace-path";
    pub const SETTINGS_DISPLAY_BREATH: &str = "dioxus-room-settings-display-breath";
    pub const SETTINGS_DISPLAY_BREATH_PERIOD: &str = "dioxus-room-settings-display-breath-period";
    pub const SETTINGS_DISPLAY_DUAL: &str = "dioxus-room-settings-display-dual";
    pub const SETTINGS_DISPLAY_DUAL_NOTE: &str = "dioxus-room-settings-display-dual-note";
    pub const SETTINGS_SEDIMENT_FOOT: &str = "dioxus-room-settings-sediment-foot";

    // ===== Onboarding window (2026-08-24 Task EF-E4) =====
    pub const NAV_ONBOARDING: &str = "dioxus-room-nav-onboarding";
    pub const ONBOARDING_WINDOW_TITLE: &str = "dioxus-room-onboarding-window-title";
    pub const ONBOARDING_STATUS_TITLE: &str = "dioxus-room-onboarding-status-title";
    pub const ONBOARDING_STATUS_INITIAL: &str = "dioxus-room-onboarding-status-initial";
    pub const ONBOARDING_HEAD_STATE_INITIAL: &str = "dioxus-room-onboarding-head-state-initial";
    pub const ONBOARDING_HEAD_NAME_INITIAL: &str = "dioxus-room-onboarding-head-name-initial";
    pub const ONBOARDING_HEAD_FOLD_BTN: &str = "dioxus-room-onboarding-head-fold-btn";
    pub const ONBOARDING_DRAWER_MIND_HEAD: &str = "dioxus-room-onboarding-drawer-mind-head";
    pub const ONBOARDING_DRAWER_FACILITY_HEAD: &str = "dioxus-room-onboarding-drawer-facility-head";
    pub const ONBOARDING_DRAWER_WORK_HEAD: &str = "dioxus-room-onboarding-drawer-work-head";
    pub const ONBOARDING_SECTION_IDENTITY_TITLE: &str = "dioxus-room-onboarding-section-identity-title";
    pub const ONBOARDING_SECTION_IDENTITY_EM: &str = "dioxus-room-onboarding-section-identity-em";
    pub const ONBOARDING_SECTION_PROVIDER_TITLE: &str = "dioxus-room-onboarding-section-provider-title";
    pub const ONBOARDING_SECTION_PROVIDER_EM: &str = "dioxus-room-onboarding-section-provider-em";
    pub const ONBOARDING_SECTION_WORKSPACE_TITLE: &str = "dioxus-room-onboarding-section-workspace-title";
    pub const ONBOARDING_SECTION_WORKSPACE_EM: &str = "dioxus-room-onboarding-section-workspace-em";
    pub const ONBOARDING_STEP_1: &str = "dioxus-room-onboarding-step-1";
    pub const ONBOARDING_STEP_2: &str = "dioxus-room-onboarding-step-2";
    pub const ONBOARDING_STEP_3: &str = "dioxus-room-onboarding-step-3";
    pub const ONBOARDING_LABEL_USER: &str = "dioxus-room-onboarding-label-user";
    pub const ONBOARDING_LABEL_AGENT: &str = "dioxus-room-onboarding-label-agent";
    pub const ONBOARDING_LABEL_RELATION: &str = "dioxus-room-onboarding-label-relation";
    pub const ONBOARDING_LABEL_PALETTE: &str = "dioxus-room-onboarding-label-palette";
    pub const ONBOARDING_LABEL_PALETTE_EM: &str = "dioxus-room-onboarding-label-palette-em";
    pub const ONBOARDING_LABEL_MODEL: &str = "dioxus-room-onboarding-label-model";
    pub const ONBOARDING_LABEL_BASE_URL: &str = "dioxus-room-onboarding-label-base-url";
    pub const ONBOARDING_LABEL_API_KEY: &str = "dioxus-room-onboarding-label-api-key";
    pub const ONBOARDING_LABEL_WORKSPACE: &str = "dioxus-room-onboarding-label-workspace";
    pub const ONBOARDING_BTN_TEST: &str = "dioxus-room-onboarding-btn-test";
    pub const ONBOARDING_TEST_STATUS_WAIT: &str = "dioxus-room-onboarding-test-status-wait";
    pub const ONBOARDING_BTN_BROWSE: &str = "dioxus-room-onboarding-btn-browse";
    pub const ONBOARDING_PREVIEW_UNCOLORED: &str = "dioxus-room-onboarding-preview-uncolored";
}
