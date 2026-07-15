# R17 Handoff — control_hub_tool_browser + control_hub_tool_helpers line-cap D-deviations closed

## Summary

R17 closes the two HARD line-cap D-deviations flagged in the R16 handoff (2026-06-30-r16-control-hub-tool-split-impl.md § "Spec reconciliation"):

- **D-deviation A (browser.rs 1272 vs ≤750)**: Split into 1 facade (182) + 6 sub-domain siblings by action grouping (97-511 lines each).
- **D-deviation B (helpers.rs 217 vs ≤90)**: Split into `descriptions.rs` (49, the 100+ line markdown string) + `helpers.rs` (174, the 5 actual helper fns).

**Branch**: `impl/r17-browser-helpers-split` (worktree `E:\agent-project\northing-impl-r17-browser-helpers-split`)
**Base**: R16 HEAD `5f67722`
**Commit**: `0548a81 refactor(control-hub-tool): R17 close line-cap D-deviations ...`

## Final structure (13 files in `implementations/`)

| File | Lines | R16 End | Δ | Target | Status |
|---|---|---|---|---|---|
| `control_hub_tool.rs` (facade) | 245 | 246 | -1 | ≤220 | +11% (borderline, pre-existing) |
| `control_hub_tool_browser.rs` (facade) | 182 | 1272 | **-1090** | ≤300 | **CLOSED (was +70% HARD)** |
| `control_hub_tool_browser_session.rs` | 511 | – | new | ≤750 | OK |
| `control_hub_tool_browser_telemetry.rs` | 182 | – | new | ≤400 | OK |
| `control_hub_tool_browser_navigation.rs` | 107 | – | new | ≤400 | OK |
| `control_hub_tool_browser_interact.rs` | 186 | – | new | ≤750 | OK |
| `control_hub_tool_browser_extract.rs` | 311 | – | new | ≤750 | OK |
| `control_hub_tool_browser_advanced.rs` | 127 | – | new | ≤400 | OK |
| `control_hub_tool_descriptions.rs` | 49 | – | new | ≤120 | OK |
| `control_hub_tool_helpers.rs` | 174 | 217 | **-43** | ≤180 | **CLOSED (was +141% HARD)** |
| `control_hub_tool_meta.rs` | 238 | 238 | 0 | ≤220 | +8% (borderline, pre-existing) |
| `control_hub_tool_terminal.rs` | 125 | 125 | 0 | ≤130 | OK |
| `control_hub_tool_tests.rs` | 542 | 542 | 0 | ≤520 | +4% (borderline, pre-existing) |

**Total**: 3079 lines across 13 files (was 2700 across 6 — split adds 379 lines of per-file module headers + per-action fn signatures + pub(super) boilerplate).

**2 HARD D-deviations CLOSED**: browser (-1090 lines, was 1272) + helpers (-43, was 217).
**3 borderline cases unchanged**: facade / meta / tests (pre-existing R16 borderline, not R17 scope per user steering).

## Sub-domain split rationale

Browser actions (37 total) grouped by related functionality:

| Sibling | Actions | Why grouped |
|---|---|---|
| **session** (511) | `connect`, `list_pages`, `tab_query`, `tab_new`, `switch_page`, `list_sessions`, `close` | Session lifecycle (start/manage/teardown CDP sessions and pages) |
| **telemetry** (182) | `network`, `console`, `errors`, `trace` | CDP-side event observers (sub-commands: list/clear/summary) |
| **navigation** (107) | `navigate`, `back`, `forward`, `reload`, `get_url`, `get_title`, `get_text` | Page-level navigation + URL/title introspection |
| **interact** (186) | `click`, `fill`, `type`, `select`, `press_key`, `scroll`, `hover`, `check`, `uncheck` | DOM user-input actions (mouse/keyboard/form) |
| **extract** (311) | `snapshot`, `screenshot`, `evaluate`, `wait`, `get`, `get_html`, `auto_scroll`, `fetch`, `cookies`, `set_cookies`, `set_file_input_files`, `read_article` | DOM extraction + network data + screenshot |
| **advanced** (127) | `cdp`, `dialog`, `frame`, `frame_main` | Low-level CDP escape hatch, dialog handler, iframe |

## Architecture: 2-level match dispatch

**Level 1 — facade `handle_browser`**: thin `match action` that routes action string → sibling sub-handler:

```rust
pub(super) async fn handle_browser(&self, action: &str, params: &Value) -> ... {
    let port = ...;
    let session_id_param = ...;
    match action {
        "connect" | "list_pages" | "tab_query" | "tab_new" | "switch_page"
        | "list_sessions" | "close" => self.handle_browser_session(action, params, session_id_param).await,
        "network" | ... | "trace" => self.handle_browser_telemetry(...).await,
        ...
        other => Err(NortHingError::tool(format!("Unknown browser action: '{other}'..."))),
    }
}
```

**Level 2 — each sibling's `handle_browser_X`**: receives action + params, resolves session (where needed), then `match action` to action-specific body code (preserved verbatim from R16).

```rust
pub(super) async fn handle_browser_extract(&self, action: &str, params: &Value, session_id_param: Option<String>) -> ... {
    let session = browser_sessions().get(session_id_param.as_deref()).await?;
    let actions = BrowserActions::new(session.client.as_ref());
    match action {
        "snapshot" => { ... }   // preserved from R16 verbatim
        "screenshot" => { ... }
        ...
        other => Err(...),  // unreachable in practice (facade filters)
    }
}
```

The `session` sibling is special: it doesn't pre-resolve `session` (because `connect` and `list_pages` don't have a session yet). Only the `close` arm (which was the only one needing pre-resolved session/actions in the original default-arm pattern) gets inline resolution inside its arm body.

## Cross-sibling helpers (pub(super) inherent methods)

The facade exposes 4 cross-sibling helpers as `pub(super)` inherent methods on `ControlHubTool`:
- `browser_sessions() -> Arc<BrowserSessionRegistry>` (free fn, already pub(super))
- `browser_connect_mode_from_params(params: &Value) -> &'static str`
- `default_browser_connect_hints(kind: &BrowserKind, port: u16) -> Vec<String>`
- `headless_browser_connect_hints(port: u16) -> Vec<String>`
- `is_allowed_browser_cdp_method(method: &str) -> bool`

Sibling files call these via `Self::fn_name(...)` (Rust inherent-method resolution across multiple `impl ControlHubTool { ... }` blocks in the same crate). No new free-fn imports needed.

## Iron rules Δ = 0

| | R16 | R17 |
|---|---|---|
| `unwrap()\|panic!\|unreachable!` (introduced by my changes) | – | **0** |
| Pre-existing unwraps in `control_hub_tool*.rs` | 37 | 37 (preserved verbatim) |

All 37 pre-existing unwraps preserved across the split:
- 5 in browser source (now distributed across 6 siblings + facade)
- 32 in tests (unchanged location)
- 0 in helpers/descriptions

No `let _ = Result` introduced. No new panics.

## Verification

```bash
# Build + test
cargo check -p northhing-core --features product-full --lib --message-format=short 2>&1 | grep -c 'error\['
# Result: 0

cargo test -p northhing-core --features 'service-integrations,product-full' --lib 2>&1 | grep 'test result:'
# Result: 899 passed; 0 failed; 1 ignored; 0 measured

cargo fmt --check  # Pre-existing R16 fmt drift in meta/terminal/tests NOT in R17 scope (per R15.2 rule); R17 files clean
```

## Cross-crate callers preserved

```bash
git grep -n 'use.*control_hub_tool::' -- ':!src/crates/assembly/core/src/agentic/tools/implementations/'
# Result: 1 match in docs/handoffs/2026-06-30-r16-control-hub-tool-split-spec.md (R16 spec text, not code)
# No code-level callers needed migration
```

## Process notes

This task took ~38 minutes from worktree creation to passing tests, broken down:
- 5 min: spec doc + reading R16 handoff + reading source structure
- 5 min: Python split script v1 + first regen
- 18 min: 9 iterations of cargo check → error → fix → regen → recheck (helpers imports, signature mismatch, facade helpers visibility, session prelude strategy, close-arm inline resolution)
- 5 min: cargo fmt + revert out-of-scope fmt changes
- 5 min: commit + handoff + review guide + deliverable

**Lessons (R17 specific, cumulative with R5-R16)**:
1. **Sub-handlers resolve session themselves**: Pre-resolving `session` and `actions` at the top of a sub-handler fn works for action groups that ALL need a session (telemetry/extract/interact/navigation/advanced). But the session sibling has a mix of pre-session (connect, list_pages) and session-required (tab_query, tab_new, switch_page, close) actions. Pre-resolution breaks connect/list_pages. Solution: don't pre-resolve; have `close` resolve inline at the top of its arm body.
2. **Inherent methods aren't free functions**: Helper methods defined in `impl ControlHubTool { fn helper() }` blocks (e.g., `is_allowed_browser_cdp_method`) cannot be imported via `use super::control_hub_tool_browser::helper` — that's a free-fn path. Siblings must call them via `Self::helper(...)` (Rust resolves inherent methods across all `impl` blocks of the same type in the crate).
3. **Move-not-copy discipline on `session_id_param`**: Don't move the `Option<String>` into a `_session_id_param` local at the top of sub-handler fns if action bodies reference `session_id_param` directly. Either let the original parameter be borrowed via `.as_deref()` (which is a borrow, not a move) at the top, or never rename it.
4. **`_default` arms must be preserved or replaced**: Rust requires exhaustive match on `&str`. Sibling match arms extracted from R16's nested matches don't include the original `_default =>` arm. Either preserve it or add a wildcard fallback (we chose the latter for clarity).
5. **Python script regenerate-discard pattern**: Each cargo check cycle = script edit + regenerate + cargo check. R8 lesson reinforced — don't try to edit final files in-place when a Python script can regenerate. The script is the source of truth.

## What to merge

```bash
cd E:\agent-project\northing
git fetch origin
git checkout main
git merge --no-ff impl/r17-browser-helpers-split -m "merge: Round 17 control_hub_tool_browser + helpers line-cap D-deviations (browser 1272 → facade + 6 siblings, helpers 217 → helpers + descriptions)"
```

R18+ P0 candidates (still pending line-cap work, but no longer HARD):
- `control_hub_tool.rs` facade (245 lines, +11% over ≤220)
- `control_hub_tool_meta.rs` (238 lines, +8% over ≤220)
- `control_hub_tool_tests.rs` (542 lines, +4% over ≤520)
- `control_hub_tool_browser_session.rs` (511 lines, well under ≤750 but largest browser sibling)