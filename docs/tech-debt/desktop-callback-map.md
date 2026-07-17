# Desktop Callback Map (main.slint → Rust)

> Generated as part of B-3.1 (tech-debt-cleanup-guide §4.1).
> Status: **36 callbacks declared in main.slint, 18 wired to Rust handlers, 18 unwired.**

## Legend

- ✅ **Wired**: `ui.on_X(...)` handler exists in `app_state/callbacks_*.rs`
- ❌ **Unwired**: No Rust handler. Declaration exists in main.slint but no `ui.on_X(...)` call.
- 🔁 **Forwarded**: Callback is forwarded from a child component to root in main.slint (still needs root handler).

## Callback Status Table

| # | Callback (main.slint) | Slint Line | Rust Handler | Handler File:Line | Disposition |
|---|----------------------|------------|-------------|-------------------|-------------|
| 1 | `send-message(string)` | 24 | ✅ `on_send_message` | callbacks_lifecycle.rs:26 | Wired |
| 2 | `new-session()` | 25 | ✅ `on_new_session` | callbacks_lifecycle.rs:208 | Wired |
| 3 | `switch-session(string)` | 26 | ✅ `on_switch_session` | callbacks_lifecycle.rs:316 | Wired |
| 4 | `delete-session(string)` | 27 | ✅ `on_delete_session` | callbacks_lifecycle.rs:359 | Wired |
| 5 | `toggle-theme()` | 28 | ✅ `on_toggle_theme` | callbacks_lifecycle.rs:431 | Wired |
| 6 | `toggle-show-subagents()` | 29 | ✅ `on_toggle_show_subagents` | callbacks_lifecycle.rs:446 | Wired |
| 7 | `toggle-skill(string)` | 30 | ✅ `on_toggle_skill` | callbacks_lifecycle.rs:465 | Wired |
| 8 | `load-more-messages()` | 31 | ✅ `on_load_more_messages` | callbacks_lifecycle.rs:569 | Wired |
| 9 | `refresh-sessions()` | 32 | ✅ `on_refresh_sessions` | callbacks_lifecycle.rs:625 | Wired |
| 10 | `refresh-messages()` | 33 | ✅ `on_refresh_messages` | callbacks_lifecycle.rs:652 | Wired |
| 11 | `clear-session-error()` | 34 | ✅ `on_clear_session_error` | callbacks_lifecycle.rs:682 | Wired |
| 12 | `clear-input-error()` | 35 | ✅ `on_clear_input_error` | callbacks_lifecycle.rs:691 | Wired |
| 13 | `dismiss-banner()` | 36 | ✅ `on_dismiss_banner` | callbacks_lifecycle.rs:704 | Wired |
| 14 | `clear-inline-error()` | 37 | ✅ `on_clear_inline_error` | callbacks_lifecycle.rs:714 | Wired |
| 15 | `stop-streaming()` | 38 | ✅ `on_stop_streaming` | callbacks_lifecycle.rs:725 | Wired |
| 16 | `upsert-provider(...)` | 41 | ✅ `on_upsert_provider` | callbacks_settings.rs:244 | Wired |
| 17 | `delete-provider(string)` | 42 | ✅ `on_delete_provider` | callbacks_settings.rs:65 | Wired |
| 18 | `remove-workspace(string)` | 43 | ✅ `on_remove_workspace` | callbacks_settings.rs:164 | Wired |
| 19 | `close-settings()` | 32 | ❌ | — | **Phase-X reserved**: Settings page close is handled purely in Slint (`root.current-route = "main"`). No Rust handler needed. |
| 20 | `test-provider(string)` | 65 | ❌ | — | **D2 task**: Wire to provider test API call |
| 21 | `set-default-model(string)` | 66 | ❌ | — | **D2 task**: Wire to `set_default_model` in settings |
| 22 | `cleanup-legacy-placeholders()` | 67 | ❌ | — | **Delete**: No legacy placeholders exist in v0.1.0. Remove declaration. |
| 23 | `set-skill-global(string, bool)` | 68 | ❌ | — | **Phase-X reserved**: Skill system not implemented in v0.1.0 desktop. |
| 24 | `set-skill-workspace(string, string)` | 69 | ❌ | — | **Phase-X reserved**: Skill system not implemented in v0.1.0 desktop. |
| 25 | `upsert-mcp(...)` | 70 | ❌ | — | **Phase-X reserved**: MCP config UI not implemented in v0.1.0 desktop. |
| 26 | `delete-mcp(string)` | 71 | ❌ | — | **Phase-X reserved**: MCP config UI not implemented in v0.1.0 desktop. |
| 27 | `test-mcp(string)` | 72 | ❌ | — | **Phase-X reserved**: MCP config UI not implemented in v0.1.0 desktop. |
| 28 | `pick-folder()` | 74 | ❌ | — | **D2 task**: Wire to native folder picker dialog |
| 29 | `add-workspace(string, string, bool)` | 75 | ❌ | — | **D2 task**: Wire to workspace management |
| 30 | `switch-workspace(string)` | 76 | ❌ | — | **D2 task**: Wire to workspace switching |
| 31 | `edit-identity-md(string)` | 78 | ❌ | — | **Phase-X reserved**: Identity editor not in v0.1.0 desktop. |
| 32 | `open-identity-creator()` | 79 | ❌ | — | **Phase-X reserved**: Identity creator not in v0.1.0 desktop. |
| 33 | `export-markdown()` | 115 | ❌ | — | **D2 task**: Wire to conversation export |
| 34 | `open-session-settings()` | 116 | ❌ | — | **D2 task**: Wire to session settings panel |
| 35 | `rename-session` | — | ❌ | — | **Not declared**: Guide mentioned this but no `rename-session` callback exists in main.slint. No action needed. |
| 36 | `add-workspace` (forward) | 143-144 | 🔁 | — | Forwarded from SettingsView to root. Root handler = ❌ (see #29). |

## Summary

| Category | Count | Action |
|----------|-------|--------|
| ✅ Wired | 18 | No action |
| ❌ D2 task (wire in next dev cycle) | 6 | Track in tech-debt-ledger as D2 items |
| ❌ Phase-X reserved (feature not in v0.1.0) | 7 | Keep declaration, add comment in main.slint |
| ❌ Delete (no use case) | 1 | Remove `cleanup-legacy-placeholders` declaration |
| ✅ Slint-only (no Rust needed) | 1 | `close-settings` handled in Slint |
| N/A | 1 | `rename-session` not declared (guide error) |

## Disposition Rules

1. **D2 task items**: Keep declaration in main.slint. Add to tech-debt-ledger as P2 items.
2. **Phase-X reserved**: Keep declaration. Add `// Phase-X: <feature> not implemented in v0.1.0` comment in main.slint.
3. **Delete**: Remove declaration and all forwarding references from main.slint.
4. **Slint-only**: No action. Document as "handled in Slint" for future reference.
