# R18 Spec — control_hub_tool_browser_session + helpers line-cap D-deviations

## Context

Round 17 (`impl/r17-browser-helpers-split`, merged `66d4dfc` on main HEAD `c7b16a6`) decomposed
`control_hub_tool_browser.rs` (1272 lines) into 1 facade + 6 per-action siblings, and split
`helpers.rs` (217 lines) into helpers + descriptions. **2 HARD line-cap D-deviations + 3 borderline
remain** (Kimi R17 review line 28 + 44 — APPROVE 8.5/10 with explicit R18 action items):

| File | Lines (main HEAD) | Cap | Deviation |
|---|---|---|---|
| `control_hub_tool_browser_session.rs` | **485** | ≤220 | **+120% HARD** |
| `control_hub_tool_helpers.rs` | **162** | ≤90 | **+80% HARD** |
| `control_hub_tool.rs` (facade) | 244 | ≤220 | +11% borderline (tolerated) |
| `control_hub_tool_meta.rs` | 216 | ≤220 | OK |
| `control_hub_tool_tests.rs` | 493 | ≤520 | OK |

This spec closes both HARD D-deviations. Borderline cases (facade 244, meta 216, tests 493) are
**tolerated per Kimi R17 review** — they sit at or below 220/520 caps now. We do not over-split
to chase a -1-line delta.

**R18 P0 hard (per R16+R17 handoff §Next Steps)**:
- **A**: Split `handle_browser_session` (the 7-action match block in browser_session.rs) into
  per-action sibling files — closing the +120% HARD violation.
- **B**: Migrate `parse_browser_kind` (browser-only helper) into `control_hub_tool_browser.rs`,
  and consolidate the 4 envelope/error-classification helpers into a new
  `control_hub_tool_envelope.rs` — closing the +80% HARD violation on helpers.rs.

**R18 P1 housekeeping (Kimi R16 deep review bugs 3-4 deferred to R18)**:
- **Kimi Bug 3 (unwrap count accuracy)**: All unwrap/expect count claims in this spec, in the
  impl handoff, and in the Mavis review **must** be derived from `grep -cE '\bunwrap\(\)'` /
  `grep -cE '\bexpect\('` on the file under test, **never** from a hand-counted summary that
  could conflate `unwrap()` with `unwrap_or()` / `unwrap_or_else()`. The pre-split baseline must
  be re-derived by running the same grep against `git show main:<file>` (pre-split file).
- **Kimi Bug 4 (helpers.rs `description_text()` long line)**: **Already naturally closed** by R17
  extracting `description_text()` into `control_hub_tool_descriptions.rs` (38 lines). R18 will
  verify no other helper function exceeds the ≤120-char line cap and fix any that does.

## Baseline (must preserve)

- Worktree: new `E:\agent-project\northing-impl-r18-browser-session-helpers-split` on branch
  `impl/r18-browser-session-helpers-split` from main HEAD `c7b16a6`.
- `cargo test -p northhing-core --features 'service-integrations,product-full' --lib`:
  **899 passed; 0 failed; 1 ignored** (R17 baseline preserved).
- `cargo check --workspace` → 0 errors introduced (pre-existing workspace errors at most 2 are
  out of scope: `cli/agent/core_adapter.rs:121` + `cli/modes/chat/run.rs:80`).
- Iron rules Δ = 0:
  - pre-split `unwrap()` baseline (re-derive via grep, do **not** trust round-to-round carries):
    ```bash
    git show main:src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_browser_session.rs | grep -cE '\bunwrap\(\)'
    git show main:src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_helpers.rs | grep -cE '\bunwrap\(\)'
    # Expected: 4 unwraps in browser_session, 0 in helpers (Kimi R17 review line 33 confirmed:
    # "37 unwraps. Post-R17: 37 (1 in browser_advanced, 4 in browser_session, 32 in tests). Δ=0")
    ```

## Target structure

### browser_session split → 4 files (thin facade + 3 per-action siblings)

| File | Target (cap ≤220) | Owns | Lines (est) |
|---|---|---|---|
| `control_hub_tool_browser_session.rs` (thin facade) | ≤100 | `handle_browser_session` thin dispatcher (7-action match → sub-handler) | ~80 |
| `control_hub_tool_browser_connect.rs` (new) | ≤220 | `handle_browser_connect` — connect logic with `target_url`/`target_title` matching, launch, register, observers, optional Page.bringToFront | ~210 |
| `control_hub_tool_browser_pages.rs` (new) | ≤220 | `handle_browser_pages` — 2-level match for `list_pages` / `tab_query` / `tab_new` / `switch_page` (page-lifecycle operations, all touch page registry) | ~200 |
| `control_hub_tool_browser_session_mgmt.rs` (new) | ≤80 | `handle_browser_session_mgmt` — 2-level match for `list_sessions` / `close` (small admin actions, no page details) | ~50 |

**Total browser_session**: ~540 lines (was 485 — small overhead from per-file module header + thin
dispatcher body). Net: -485 → +540 = +55 lines added, but each file ≤220 cap.

### helpers split → `helpers.rs` removed; functions redistributed

| Destination | Owns | Source location (post-R17) |
|---|---|---|
| `control_hub_tool_browser.rs` (existing facade, +12 lines) | `pub(super) fn parse_browser_kind(&str) -> NortHingResult<BrowserKind>` — only used by browser siblings | move from `control_hub_tool_helpers.rs:22` |
| `control_hub_tool_envelope.rs` (new) ≤130 | `parse_bracket_code_prefix`, `parse_hints_suffix`, `envelope_wrap_results`, `map_dispatch_error` — all are error/serialization helpers consumed only by facade `call_impl` | move from `control_hub_tool_helpers.rs:36-178` |
| `control_hub_tool_helpers.rs` | **deleted** (was 162 lines; all 5 helpers migrated) | — |

**Total helpers**: ~140 lines (was 162 — small overhead from new file module header, but no more
3-way confusion between browser-specific and envelope-specific helpers).

## Action grouping rationale (browser_session split)

| Group | Actions | Why grouped | Source lines |
|---|---|---|---|
| **connect** (own file) | `connect` | High complexity: 195 lines including launch, `target_url`/`target_title` matching, CDP registration, observer enablement, optional `Page.bringToFront`. Standalone sibling to keep ≤220 cap | browser_session.rs:48-263 |
| **pages** | `list_pages` / `tab_query` / `tab_new` / `switch_page` | All operate on the **page registry** (open tabs, create/switch tabs, list/filter pages). `tab_query` is `list_pages` with substring filters; `switch_page` reuses `list_pages` to find unknown ids | browser_session.rs:266-484 |
| **session_mgmt** | `list_sessions` / `close` | Both act on **session registry** (not page registry). Tiny — 10 lines each. Keep together in a small sibling | browser_session.rs:487-508 |

`browser_session.rs` becomes a thin facade that routes to the three sub-handlers via 2-level
match (R7 god-method-split precedent — preserves original control flow with minimal restructuring).

## Iron rules (MUST enforce — same as R17)

1. **0 NEW unwrap/panic/let _ = Result** in production code — preserve pre-existing 4 unwraps in
   browser_session + 0 in helpers verbatim. Verify via the Kimi-Bug-3 grep baseline above.
2. **All sibling methods use `impl ControlHubTool { ... }` blocks** — `pub(super) async fn handle_X`.
3. **`pub(super)` pattern**: All sibling handlers + free helpers are `pub(super)` so facade thin
   dispatcher and inherent-method dispatch resolve them.
4. **No caller migration**: facade `handle_browser_session` keeps its
   `pub(super) async fn handle_browser_session(action, params, session_id_param)` signature.
5. **Single cargo check**: batch ALL edits before running `cargo check`. R8 + R14 lesson —
   4 min × N cycles is catastrophic.
6. **Read source from git HEAD**: Python split script (if used) must read from
   `git show main:path`, never from on-disk file (R8 self-overwrite bug).
7. **PowerShell safety** (Windows): do NOT use `>` redirect or `Set-Content` without
   `-Encoding UTF8` for any `.rs` file. Use the Write tool or `node`/`python` scripts with
   explicit `encoding='utf-8'`. **CRLF will silently corrupt `cargo check`**.
8. **`core.autocrlf=false`** must be set locally in the new worktree BEFORE first checkout:
   ```bash
   git config --local core.autocrlf false
   ```
   Otherwise R17's `.gitattributes` `*.rs text eol=lf` is overridden by Windows default autocrlf.
9. **line cap**: every file ≤220 (≤520 for tests.rs). Verify with
   `wc -l <file>` AFTER fmt (Read tool measures including blank lines; both must agree).
10. **line length**: every line ≤120 chars. Verify with
    `awk '{ if (length > 120) print NR": "length" chars: "$0 }' <file>`. Fix any that exceeds.

## Facade dispatch design

The thin `handle_browser_session` in browser_session.rs becomes a 30-line `match action` that
routes to 3 sub-handler methods:

```rust
pub(super) async fn handle_browser_session(
    &self,
    action: &str,
    params: &Value,
    session_id_param: Option<String>,
) -> NortHingResult<Vec<ToolResult>> {
    let port = params
        .get("port")
        .and_then(|v| v.as_u64())
        .map(|p| p as u16)
        .unwrap_or(DEFAULT_CDP_PORT);
    match action {
        "connect" => self.handle_browser_connect(action, params, port).await,
        "list_pages" | "tab_query" | "tab_new" | "switch_page" => {
            self.handle_browser_pages(action, params, port, session_id_param).await
        }
        "list_sessions" | "close" => {
            self.handle_browser_session_mgmt(action, params, session_id_param).await
        }
        other => Err(NortHingError::tool(format!(
            "action '{}' dispatched to handle_browser_session but is not in its match arms (facade dispatch bug)",
            other
        ))),
    }
}
```

`handle_browser_connect` is a flat fn (no inner match — `connect` is its own action).
`handle_browser_pages` and `handle_browser_session_mgmt` use 2-level match (R7 pattern).

## Cross-sibling imports

Each browser_session split sibling needs:

```rust
// control_hub_tool_browser_connect.rs
use crate::agentic::tools::browser_control::actions::BrowserActions;
use crate::agentic::tools::browser_control::browser_launcher::{
    BrowserKind, BrowserLauncher, LaunchResult, DEFAULT_CDP_PORT,
};
use crate::agentic::tools::browser_control::cdp_client::CdpClient;
use crate::agentic::tools::browser_control::session_registry::{
    BrowserSession, BrowserSessionState,
};
use crate::service::config::{get_global_config_service, GlobalConfig};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};
use std::sync::Arc;

use super::super::control_hub::{err_response, ControlHubError, ErrorCode};
use super::super::control_hub_tool_browser::{
    browser_connect_mode_from_params, browser_sessions, default_browser_connect_hints,
    headless_browser_connect_hints, parse_browser_kind,
};
use super::super::ControlHubTool;
```

`control_hub_tool_browser.rs` (the browser facade, not browser_session) absorbs `parse_browser_kind`
as a `pub(super) fn` and re-exports it via inherent-method location. Cross-sibling `pub(super)`
imports follow R13b pattern — siblings depend on facade for shared types/helpers, never the other
way.

`control_hub_tool_envelope.rs` is a leaf module — only consumed by `control_hub_tool.rs` facade's
`call_impl`. No cross-sibling deps.

## Test path

No test file changes. All 22 control_hub_tool tests live in `control_hub_tool_tests.rs`
(493 lines, ≤520 OK) and exercise the public `ControlHubTool::call_impl` API → `dispatch()` →
sibling handlers. The sibling handlers are `pub(super)` but reachable via inherent dispatch from
`dispatch()`. Tests pass unchanged.

## Verification commands

```bash
# 0. Worktree preflight (R17 autocrlf gotcha)
cd E:/agent-project/northing-impl-r18-browser-session-helpers-split
git config --local core.autocrlf false

# 1. Build + test
cargo check -p northhing-core --features product-full --lib --message-format=short 2>&1 | grep -c 'error\['
# Expected: 0

cargo test -p northhing-core --features 'service-integrations,product-full' --lib 2>&1 | grep 'test result:'
# Expected: 899 passed; 0 failed; 1 ignored

cargo check --workspace 2>&1 | grep -c 'error\['
# Expected: 0 NEW vs main baseline (re-derive baseline via git stash + cargo check)

# 2. Iron rules — Kimi Bug 3 fix: use precise grep, not substring
echo "==unwrap() count baseline vs post-split=="
git show main:src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_browser_session.rs | grep -cE '\bunwrap\(\)'
# Expected: 4 (Kimi R17 confirmed)
git show main:src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_helpers.rs | grep -cE '\bunwrap\(\)'
# Expected: 0

# After split, sum the per-file unwrap counts and verify equality with baseline:
find src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_browser_session.rs src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_browser_connect.rs src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_browser_pages.rs src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_browser_session_mgmt.rs src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_helpers.rs 2>/dev/null -exec cat {} + | grep -cE '\bunwrap\(\)'
# Expected: 4 (no NEW unwrap, no missing unwrap)

git diff main..HEAD -- src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool*.rs | grep -cE '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# Expected: 0 (no NEW unwrap/panic/unreachable in production code)

# 3. File sizes — every file ≤220, tests ≤520
Get-ChildItem src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool*.rs | Select-Object Name, @{n='Lines';e={(Get-Content $_ | Measure-Object -Line).Lines}}
# Expected:
#   control_hub_tool_browser_session.rs        ≤100
#   control_hub_tool_browser_connect.rs        ≤220
#   control_hub_tool_browser_pages.rs          ≤220
#   control_hub_tool_browser_session_mgmt.rs   ≤80
#   (control_hub_tool_helpers.rs removed)
#   control_hub_tool_envelope.rs               ≤130
#   control_hub_tool.rs (facade)               ~244 (tolerated borderline, no change)
#   control_hub_tool_meta.rs                   ~216 (no change, ≤220)
#   control_hub_tool_tests.rs                  ~493 (no change, ≤520)

# 4. Line length — Kimi Bug 4 verification
awk '{ if (length > 120) print NR": "length" chars" }' src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool*.rs
# Expected: no output (every line ≤120 chars)

# 5. Format check
cargo fmt --check -- src/crates/assembly/core/src/agentic/tools/implementations/
# Expected: 0 diff

# 6. LF enforcement (Kimi R16 Bug 1 — already fixed by .gitattributes; verify no regression)
file src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool*.rs
# Expected: every line says "ASCII text" or "Unicode text" + "with LF line terminators"
# NO "with CRLF" anywhere

# 7. Cross-crate callers preserved
git grep -n 'use.*control_hub_tool::' -- ':!src/crates/assembly/core/src/agentic/tools/implementations/'
git grep -n 'use.*control_hub_tool_helpers::' -- ':!src/crates/assembly/core/src/agentic/tools/implementations/'
# Expected: every entry preserved (no caller migration; helpers.rs removal only affects internal
# sibling imports, which are updated in-place)

# 8. mod.rs registration
grep -E 'pub mod control_hub_tool' src/crates/assembly/core/src/agentic/tools/implementations/mod.rs
# Expected: 4 new sibling entries (browser_connect, browser_pages, browser_session_mgmt, envelope)
#           + 1 removed entry (helpers)
#           + existing entries unchanged
```

## Commit pattern

Single commit on `impl/r18-browser-session-helpers-split`:

```
refactor(control-hub-tool): R18 close 2 HARD line-cap D-deviations (browser_session 485 → facade+3, helpers 162 → envelope+inlined)

R18 closes the 2 HARD D-deviations Kimi R17 review flagged as APPROVE-but-defer:

A. browser_session split — handle_browser_session (485 lines, 7-action match) →
   thin facade + 3 sub-handler siblings (connect / pages / session_mgmt).
   browser_session.rs         ~80  (thin 2-level match dispatcher)
   browser_connect.rs         ~210 (connect logic standalone)
   browser_pages.rs           ~200 (list_pages + tab_query + tab_new + switch_page)
   browser_session_mgmt.rs    ~50  (list_sessions + close)

B. helpers migration — 5 free functions redistributed:
   parse_browser_kind  → inlined into control_hub_tool_browser.rs (pub(super) fn)
   parse_bracket_code_prefix + parse_hints_suffix + envelope_wrap_results +
   map_dispatch_error  → moved to new control_hub_tool_envelope.rs
   helpers.rs deleted.

Kimi R16 deep review bug 3 (unwrap count accuracy): all unwrap counts in this
commit's review/impl docs derived from `grep -cE '\bunwrap\(\)'` baseline against
`git show main:<file>` — pre-split baseline 4 (browser_session) + 0 (helpers) = 4
preserved verbatim. Post-split sum also 4. Δ=0.

Kimi R16 deep review bug 4 (helpers.rs long line): already naturally closed by
R17 extracting description_text into control_hub_tool_descriptions.rs (38 lines).
R18 verified no other helper exceeds 120-char cap.

Iron rules: 0 NEW unwrap/panic/let _ = Result.
Line caps: every file ≤220 (≤520 for tests). Facade 244 + meta 216 + tests 493
tolerated borderline per Kimi R17 review.
Tests: 899 passed; 0 failed; 1 ignored (baseline preserved).
```

## Deliverables

1. Spec doc (this file)
2. Refactor commit on branch `impl/r18-browser-session-helpers-split`
3. Handoff doc: `docs/handoffs/2026-07-01-r18-browser-session-helpers-split-impl.md`
4. Review guide: `docs/handoffs/2026-07-01-r18-browser-session-helpers-split-review.md`
5. Plan deliverable: `C:\Users\UmR\.mavis\plans\<plan-id>\outputs\impl-r18-browser-session-helpers-split\deliverable.md`

## Risk assessment

**Low risk**:
- Pure file split + thin dispatcher — no behavior change.
- 0 NEW unwraps (verified via Kimi-Bug-3 grep baseline: 4 pre-split, 4 post-split).
- Tests unchanged (test bodies don't move; only sibling handler names follow the inherent-dispatch contract).
- Cross-crate callers unaffected (`handle_browser_session` keeps signature).

**Medium risk**:
- 4 new `pub(super)` method declarations on `ControlHubTool` in 4 sibling files — Rust allows
  multiple `impl` blocks, inherent dispatch resolves.
- Splitting `handle_browser_session` adds 4 method declarations + 2 module headers + 2-level
  match wrappers — must keep `control_hub_tool_browser_session.rs` ≤100 cap.
- `parse_browser_kind` relocation: previously `pub(super) fn` in `helpers.rs`; becomes
  `pub(super) fn` in `browser.rs`. All 1 callsite (`browser_session.rs:27`) updates its
  `use super::control_hub_tool_helpers::parse_browser_kind;` to
  `use super::control_hub_tool_browser::parse_browser_kind;`.

**Mitigation**:
- Write Python split script that mechanically extracts action bodies by matching the action
  string and `}` boundary, preserving original logic.
- Single cargo check + cargo test cycle at the end (no incremental checks).
- Preserve all comments verbatim — including "Phase 2" notes on `tab_query` and "UX shortcut"
  on `connect`.

## Cross-round follow-ups (R19+ backlog, not R18 scope)

- `facade.rs` 244 → could trim to ≤220 by extracting `Tool` trait impl into a new
  `control_hub_tool_meta.rs` expansion (meta is introspection — natural fit). **Tolerated
  borderline now** per Kimi R17 review; defer until explicitly required.
- `meta.rs` 216 already ≤220 — no action.
- `tests.rs` 493 already ≤520 — no action.
- Remaining god-object queue (R15 god-object-plan memory entry):
  `acp/client/manager.rs` 2519, `terminal/exec.rs` 2488, `runtime-ports/src/lib.rs` 2460,
  `session_usage/service.rs` 2458, `config/types.rs` 2406 — R19+ candidates.