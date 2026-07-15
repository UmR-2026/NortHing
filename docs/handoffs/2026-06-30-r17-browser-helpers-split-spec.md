# R17 Spec — control_hub_tool_browser + control_hub_tool_helpers line-cap D-deviations

## Context

Round 16 (`impl/round16-control-hub-tool-split`, merge-base `1f19784` → HEAD `5f67722`) decomposed
`control_hub_tool.rs` (2526 lines) into 1 facade + 5 siblings. The mechanical split landed handlers
in sibling files but the line caps were not respected. Two HARD D-deviations remained:

| Sibling | Lines (R16 end) | Cap | Deviation |
|---|---|---|---|
| `control_hub_tool_browser.rs` | **1272** | ≤750 | **+70% HARD** |
| `control_hub_tool_helpers.rs` | **217** | ≤90 | **+141% HARD** |
| `control_hub_tool.rs` (facade) | 246 | ≤220 | +12% borderline |
| `control_hub_tool_meta.rs` | 238 | ≤220 | +8% borderline |
| `control_hub_tool_tests.rs` | 542 | ≤520 | +4% borderline OK |
| `control_hub_tool_terminal.rs` | 125 | ≤130 | OK |

**R17 P0 (per R16 handoff section "Spec reconciliation")**:
- **A**: Split `handle_browser` (1199 lines of pure action dispatch) into per-sub-domain sibling files.
- **B**: Move `description_text` (the 100+ line markdown string) out of helpers.rs into a dedicated
  `control_hub_tool_descriptions.rs`.

This spec covers both D-deviations. The borderline cases (facade / meta / tests) are tolerated for
now — they are 4-12% over caps, not HARD violations.

## Baseline (must preserve)

- Worktree: `E:\agent-project\northing-impl-r17-browser-helpers-split` on branch
  `impl/r17-browser-helpers-split` (from R16 HEAD `5f67722`)
- `cargo test -p northhing-core --features 'service-integrations,product-full' --lib`:
  **899 passed; 0 failed; 1 ignored**
- `cargo check --workspace` → 0 errors (R16 baseline preserved)
- Iron rules Δ: 0 (37 pre-existing unwraps in `control_hub_tool*.rs`, 0 NEW)

## Target structure

### control_hub_tool_browser split → 1 facade + 6 sub-siblings (7 files total)

| File | Target | Owns | Lines (est) |
|---|---|---|---|
| `control_hub_tool_browser.rs` (facade) | ≤300 | `BROWSER_SESSIONS` registry, `browser_sessions()` accessor, `browser_connect_mode_from_params`, `default_browser_connect_hints`, `headless_browser_connect_hints`, `is_allowed_browser_cdp_method`, thin `handle_browser` dispatcher that maps action → sub-handler method | ~180 |
| `control_hub_tool_browser_session.rs` | ≤750 | Connect + session lifecycle: `handle_browser_connect`, `handle_browser_list_pages`, `handle_browser_tab_query`, `handle_browser_tab_new`, `handle_browser_switch_page`, `handle_browser_list_sessions`, `handle_browser_close` | ~480 |
| `control_hub_tool_browser_telemetry.rs` | ≤400 | CDP telemetry: `handle_browser_network`, `handle_browser_console`, `handle_browser_errors`, `handle_browser_trace` | ~170 |
| `control_hub_tool_browser_navigation.rs` | ≤400 | Navigation: `handle_browser_navigate`, `handle_browser_back`, `handle_browser_forward`, `handle_browser_reload`, `handle_browser_get_url`, `handle_browser_get_title`, `handle_browser_get_text` | ~190 |
| `control_hub_tool_browser_interact.rs` | ≤750 | User input: `handle_browser_click`, `handle_browser_fill`, `handle_browser_type`, `handle_browser_select`, `handle_browser_press_key`, `handle_browser_scroll`, `handle_browser_hover`, `handle_browser_check` | ~370 |
| `control_hub_tool_browser_extract.rs` | ≤750 | DOM/screenshot/data extraction: `handle_browser_snapshot`, `handle_browser_screenshot`, `handle_browser_evaluate`, `handle_browser_wait`, `handle_browser_get`, `handle_browser_get_html`, `handle_browser_auto_scroll`, `handle_browser_fetch`, `handle_browser_cookies`, `handle_browser_set_cookies`, `handle_browser_set_file_input_files`, `handle_browser_read_article` | ~420 |
| `control_hub_tool_browser_advanced.rs` | ≤400 | Low-level: `handle_browser_cdp`, `handle_browser_dialog`, `handle_browser_frame`, `handle_browser_frame_main` | ~130 |

**Total browser**: ~1940 lines (was 1272 in one file — split adds ~668 lines of module headers,
imports, per-action fn signatures, dispatch arm wrappers).

### control_hub_tool_helpers split → 2 files (descriptions out, helpers stays)

| File | Target | Owns | Lines (est) |
|---|---|---|---|
| `control_hub_tool_descriptions.rs` | ≤120 | `description_text()` (the 36-line markdown content string) | ~55 |
| `control_hub_tool_helpers.rs` | ≤180 | `parse_browser_kind`, `parse_bracket_code_prefix`, `parse_hints_suffix`, `envelope_wrap_results`, `map_dispatch_error` | ~165 |

**Total helpers**: ~220 lines (was 217 — small overhead from per-file module header).

## Action grouping rationale

| Group | Actions | Why grouped |
|---|---|---|
| **pre-session + session lifecycle** (browser_session.rs) | `connect`, `list_pages`, `tab_query`, `tab_new`, `switch_page`, `list_sessions`, `close` | Session lifecycle: starts/manages/teardown sessions and pages |
| **telemetry** (browser_telemetry.rs) | `network`, `console`, `errors`, `trace` | CDP-side event observers (subcommands: list/clear/summary) |
| **navigation** (browser_navigation.rs) | `navigate`, `back`, `forward`, `reload`, `get_url`, `get_title`, `get_text` | Page-level navigation and URL/title introspection |
| **interact** (browser_interact.rs) | `click`, `fill`, `type`, `select`, `press_key`, `scroll`, `hover`, `check`, `uncheck` | DOM user-input actions (mouse/keyboard/form) |
| **extract** (browser_extract.rs) | `snapshot`, `screenshot`, `evaluate`, `wait`, `get`, `get_html`, `auto_scroll`, `fetch`, `cookies`, `set_cookies`, `set_file_input_files`, `read_article` | DOM extraction + network data + screenshot |
| **advanced** (browser_advanced.rs) | `cdp`, `dialog`, `frame`, `frame_main` | Low-level CDP escape hatch, dialog handler, iframe |

## Iron rules (MUST enforce)

1. **0 NEW unwrap/panic/let _ = Result** in production code — preserve pre-existing 5 unwraps
   in browser + 0 in helpers verbatim.
2. **All sibling methods use `impl ControlHubTool { ... }` blocks** for handler methods.
   Free helpers cross-siblings remain `pub(super) fn`.
3. **`pub(super)` pattern**: All sibling handlers are `pub(super)` so the facade's thin
   `handle_browser` dispatcher can resolve them via inherent-method dispatch.
4. **No caller migration**: facade `handle_browser` keeps its `pub(super) async fn` signature;
   external callers still call `self.handle_browser(action, params)`.
5. **Single cargo check**: batch ALL edits before running cargo check. R8 + R14 lesson —
   4min × N cycles is catastrophic.
6. **Read source from git HEAD**: Python split script must read from `git show HEAD:path`,
   never from on-disk file (avoids the R8 self-overwrite bug).

## Facade dispatch design

The thin `handle_browser` in the facade becomes a 35-line `match action` that routes to
sub-handler methods:

```rust
pub(super) async fn handle_browser(&self, action: &str, params: &Value) -> NortHingResult<Vec<ToolResult>> {
    let port = params.get("port").and_then(|v| v.as_u64()).map(|p| p as u16).unwrap_or(DEFAULT_CDP_PORT);
    let session_id_param = params.get("session_id").and_then(|v| v.as_str()).map(str::to_string);
    match action {
        "connect" => self.handle_browser_connect(action, params, port).await,
        "list_pages" => self.handle_browser_list_pages(action, params, port).await,
        "tab_query" | "tab_new" | "switch_page" => {
            self.handle_browser_tab_action(action, params, session_id_param).await
        }
        "list_sessions" | "close" => {
            self.handle_browser_session_mgmt(action, params, session_id_param).await
        }
        "network" | "console" | "errors" | "trace" => {
            self.handle_browser_telemetry(action, params, session_id_param).await
        }
        "navigate" | "back" | "forward" | "reload" | "refresh"
        | "get_url" | "get_title" | "get_text" => {
            self.handle_browser_navigation(action, params, session_id_param).await
        }
        "click" | "fill" | "type" | "select" | "press_key" | "scroll"
        | "hover" | "check" | "uncheck" => {
            self.handle_browser_interact(action, params, session_id_param).await
        }
        "snapshot" | "screenshot" | "evaluate" | "wait"
        | "get" | "get_html" | "content"
        | "auto_scroll" | "fetch" | "cookies" | "get_cookies"
        | "set_cookies" | "set_file_input_files" | "file_upload"
        | "read_article" => {
            self.handle_browser_extract(action, params, session_id_param).await
        }
        "cdp" | "dialog" | "frame" | "frame_main" => {
            self.handle_browser_advanced(action, params, session_id_param).await
        }
        other => Err(NortHingError::tool(format!(
            "Unknown browser action: '{}'. Valid: connect, tab_new, navigate, ..."
        ))),
    }
}
```

Each `handle_browser_X` is a thin sub-dispatcher that internally `match action` again for
fine-grained action-specific dispatch (preserving original control flow with minimal
restructuring). This is a **2-level match** pattern (R7 god-method-split precedent) instead
of a single flat dispatch — keeps each sub-dispatcher under 80 lines while not duplicating
session-resolution logic.

## Cross-sibling imports

Each browser sibling needs:
```rust
use super::super::control_hub::{err_response, ControlHubError, ErrorCode};
use super::computer_use_actions::truncate_with_marker;
use super::control_hub_tool_browser::{
    browser_sessions, browser_connect_mode_from_params, default_browser_connect_hints,
    headless_browser_connect_hints, is_allowed_browser_cdp_method,
};
use super::control_hub_tool_helpers::parse_browser_kind;
use super::ControlHubTool;
```

The facade (`control_hub_tool_browser.rs`) keeps all imports it currently has; sibling files
import the helpers they need from the facade (R13b pattern — siblings depend on facade for
shared types/helpers, never the other way).

`control_hub_tool_descriptions.rs` needs no ControlHubTool deps (just the markdown string).
`control_hub_tool_helpers.rs` keeps its current deps.

## Test path

No test file changes. All 22 tests live in `control_hub_tool_tests.rs` (542 lines) and
exercise the public `ControlHubTool::call_impl` API → `dispatch()` → sibling handlers. The
sibling handlers are `pub(super)` but reachable via inherent dispatch from `dispatch()`. Tests
pass unchanged.

## Verification commands

```bash
# 1. Build + test (R17 R16 baseline + sibling additions)
cargo check -p northhing-core --features product-full --lib --message-format=short 2>&1 | grep -c 'error\['
# Expected: 0

cargo test -p northhing-core --features 'service-integrations,product-full' --lib 2>&1 | grep 'test result:'
# Expected: 899 passed; 0 failed; 1 ignored

cargo check --workspace 2>&1 | grep -c 'error\['
# Expected: 0 (pre-existing workspace errors at most 2: cli/agent/core_adapter.rs:121 + cli/modes/chat/run.rs:80)

cargo fmt --check -- src/crates/assembly/core/src/agentic/tools/implementations/
# Expected: 0 diff

# 2. Iron rules
git diff 5f67722..HEAD -- src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool*.rs | grep -cE '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# Expected: 0

# 3. File sizes
wc -l src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool*.rs
# Expected: facade <=300, every browser sibling <=750, descriptions <=120, helpers <=180

# 4. Cross-crate callers preserved
git grep -n 'use.*control_hub_tool::' -- ':!src/crates/assembly/core/src/agentic/tools/implementations/'
# Expected: every entry preserved (no caller migration)
```

## Commit pattern

Single commit on `impl/r17-browser-helpers-split`:
```
refactor(control-hub-tool): R17 close line-cap D-deviations (browser 1272 → facade + 6 siblings, helpers 217 → helpers + descriptions)

[paste line count summary, file table, iron rules verification]
```

## Deliverables

1. Spec doc (this file)
2. Refactor commit on branch `impl/r17-browser-helpers-split`
3. Handoff doc: `docs/handoffs/2026-06-30-r17-browser-helpers-split-impl.md`
4. Review guide: `docs/handoffs/2026-06-30-r17-browser-helpers-split-review.md`
5. Deliverable: `C:\Users\UmR\.mavis\plans\plan_d63f72cf\outputs\impl-r17-browser-helpers-split\deliverable.md`

## Risk assessment

**Low risk**:
- Pure file split + thin dispatcher — no behavior change
- 0 NEW unwraps (verified: 5 in browser + 0 in helpers preserved verbatim)
- Tests unchanged (test bodies don't move)
- Cross-crate callers unaffected (sibling handlers still `pub(super)`, resolved via inherent dispatch)

**Medium risk**:
- New `pub(super)` method declarations on `ControlHubTool` in 6 sibling files — Rust allows
  multiple `impl` blocks for the same type, so inherent dispatch works
- Splitting `handle_browser` into ~37 sub-handler methods adds 37 method declarations — must
  all be `pub(super)` so facade dispatcher can see them
- 2-level match dispatch (facade `match action` → sub-dispatcher `match action`) means each
  action string appears twice — must keep in sync

**Mitigation**:
- Write Python script that mechanically extracts action bodies by matching the action string
  and `}` boundary, preserving original logic
- Single cargo check + cargo test cycle at the end (no incremental checks)
- Preserve all comments verbatim — including "Phase 3" note on `select` and "UX shortcut" on
  `connect`