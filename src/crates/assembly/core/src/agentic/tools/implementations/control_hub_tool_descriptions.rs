//! ControlHubTool description text.
//!
//!
//! R17 split: extracted from `control_hub_tool_helpers.rs` so the
//! markdown-string content lives next to the rendering helpers (and
//! stays out of the helpers cap). Pure content — no logic, no
//! ControlHubTool deps.

/// Long-form markdown description shown to the model when it expands
/// the ControlHub tool manifest. Lists the supported domains
/// (browser, terminal, meta), the unified `{ domain, action, params }`
/// envelope, and the response shape (`ok` / `error.code` branching).
pub(super) fn description_text() -> String {
    r#"ControlHub — the unified control entry point for browser, terminal, and routing metadata.

Use this tool via `{ domain, action, params }` for browser automation, terminal signalling, and capability/routing introspection. Local computer and operating-system actions have moved out of ControlHub: use the dedicated `ComputerUse` tool/agent for desktop UI control, screenshots, OCR, mouse/keyboard input, app launching, file/url opening, clipboard access, OS facts, and local scripts.

## Domains

### domain: "browser"  (DOM/CDP browser control)
- Browser modes:
* `connect { mode: "default" }` (default) — start or attach the stable managed browser profile with CDP enabled.
* `connect { mode: "headless" }` — start or attach the stable managed headless browser profile for project Web UI testing that does not depend on user login state.
- Actions: connect, tab_new, navigate, back, forward, reload, snapshot, click, hover, fill, type, check, uncheck, select, press_key, scroll, auto_scroll, wait, get, get_text, get_url, get_title, get_html, screenshot, evaluate, fetch, cookies, set_cookies, set_file_input_files, cdp, network, console, errors, trace, dialog, frame, frame_main, read_article, close, list_pages, tab_query, switch_page, list_sessions.
- Workflow: connect -> navigate -> snapshot (returns @e1, @e2 ... refs) -> click/fill using refs.
- Take a fresh snapshot after any DOM mutation; stale refs return `error.code = STALE_REF`.

### domain: "terminal"
- list_sessions, kill (`terminal_session_id`), interrupt (`terminal_session_id`).
- Use the `Bash` tool to run new commands; this domain only signals existing terminal sessions.

### domain: "meta"
- `capabilities` — returns `{ domains: { browser, terminal, meta }, host: { os, arch }, schema_version }`.
- `route_hint` — maps a free-form intent to the appropriate ControlHub domain, or tells you to use `ComputerUse` for local computer/system/desktop work.

## Unified Response Envelope

Every call returns a stable JSON shape:

// success
{ "ok": true,  "domain": "...", "action": "...", "data": { ... } }
// failure
{ "ok": false, "domain": "...", "action": "...", "error": { "code": "...", "message": "...", "hints": ["..."] } }

Branch on `ok` and `error.code`, not on English messages.
"#
    .to_string()
}
