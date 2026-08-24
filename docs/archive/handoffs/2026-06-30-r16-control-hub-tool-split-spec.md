# R16 Spec — control_hub_tool 2526 → facade + 4 sub-siblings

## 1. Context & motivation

`src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool.rs` is 2526 lines (Kimi P1 critical list). Single god-file combining 3 domain dispatch handlers + Tool trait impl + helpers + 510-line test module. Blocking further feature work; high-risk for any cross-domain edit.

R15 closed the last R14 D-deviation (dispatch.rs 832 → 718 via start_resume extraction). R16 continues P1 queue.

**Pattern**: same sub-domain split as R14/R15 — facade preserves public API (`pub struct ControlHubTool`, `impl Tool for ControlHubTool`), physical extraction of internal handlers to sibling modules. No behavior change. Iron rules Δ = 0.

## 2. Current state

`control_hub_tool.rs` 2526 lines, single file. Key entry points:
- `impl ControlHubTool { ... }` L55-L1713 (1658 lines) — contains handle_browser + handle_terminal + helpers
- `dispatch()` L128 — `match domain { "browser" => handle_browser, "terminal" => handle_terminal, "meta" => handle_meta }`
- `handle_meta()` L178 — meta domain
- `handle_browser()` L413-L1612 (1199 lines!) — browser domain
- `handle_terminal()` L1612-L1713 (101 lines) — terminal domain
- `impl Tool for ControlHubTool { ... }` L1765-L1921 (156 lines) — Tool trait
- Helper free fns L1713-L1765 (parse_browser_kind, parse_bracket_code_prefix, parse_hints_suffix) + L1921-L2017 (envelope_wrap_results, map_dispatch_error)
- `mod control_hub_tests { ... }` L2017-L2526 (509 lines, 22 tests)

Imports already reference `super::control_hub::{err_response, ControlHubError, ErrorCode}` — sibling error module exists (no work needed there).

## 3. Target structure (5 files)

| File | Target lines | Owner of |
|---|---|---|
| `control_hub_tool.rs` (facade) | ≤ 220 | `pub struct ControlHubTool`, `Default` impl, `new()`, `dispatch()`, `impl Tool for ControlHubTool` (name/description/input_schema/validate/call_impl/render_*), `mod control_hub_tool_tests` re-export |
| `control_hub_tool_meta.rs` | ≤ 220 | `handle_meta()` + meta helpers + meta-only static/cached data |
| `control_hub_tool_browser.rs` | ≤ 750 | `handle_browser()` + browser helpers (`is_allowed_browser_cdp_method`, `default_browser_connect_hints`, `headless_browser_connect_hints`, `browser_connect_mode_from_params`) + `static BROWSER_SESSIONS` + `browser_sessions()` fn |
| `control_hub_tool_terminal.rs` | ≤ 130 | `handle_terminal()` |
| `control_hub_tool_helpers.rs` | ≤ 90 | `parse_browser_kind`, `parse_bracket_code_prefix`, `parse_hints_suffix`, `envelope_wrap_results`, `map_dispatch_error`, `description_text` |
| `control_hub_tool_tests.rs` | ≤ 520 | `mod control_hub_tests` body (22 tests) |

**Total**: 2526 → split across 6 files, each within QClaw 800±10% tolerance. Facade ≤ 220.

## 4. Owner → sibling mapping (mandatory per R11a)

Every public/private item moves exactly once. Mapping:

| Item | Original location | Target sibling |
|---|---|---|
| `BROWSER_SESSIONS` static | control_hub_tool.rs:38 | control_hub_tool_browser.rs |
| `browser_sessions()` fn | control_hub_tool.rs:41 | control_hub_tool_browser.rs |
| `pub struct ControlHubTool` | control_hub_tool.rs:47 | **facade** (stays) |
| `impl Default for ControlHubTool` | control_hub_tool.rs:49 | **facade** (stays) |
| `ControlHubTool::new()` | control_hub_tool.rs:56 | **facade** (stays) |
| `browser_connect_mode_from_params` | control_hub_tool.rs:60 | control_hub_tool_browser.rs |
| `default_browser_connect_hints` | control_hub_tool.rs:68 | control_hub_tool_browser.rs |
| `headless_browser_connect_hints` | control_hub_tool.rs:80 | control_hub_tool_browser.rs |
| `description_text()` | control_hub_tool.rs:91 | control_hub_tool_helpers.rs |
| `dispatch()` (L128) | control_hub_tool.rs:128 | **facade** (stays) |
| `handle_meta()` (L178) | control_hub_tool.rs:178 | control_hub_tool_meta.rs |
| `is_allowed_browser_cdp_method` | control_hub_tool.rs:387 | control_hub_tool_browser.rs |
| `handle_browser()` (L413) | control_hub_tool.rs:413 | control_hub_tool_browser.rs |
| `handle_terminal()` (L1612) | control_hub_tool.rs:1612 | control_hub_tool_terminal.rs |
| `parse_browser_kind` | control_hub_tool.rs:1713 | control_hub_tool_helpers.rs |
| `parse_bracket_code_prefix` | control_hub_tool.rs:1725 | control_hub_tool_helpers.rs |
| `parse_hints_suffix` | control_hub_tool.rs:1749 | control_hub_tool_helpers.rs |
| `impl Tool for ControlHubTool` (L1765-1921) | control_hub_tool.rs:1765 | **facade** (stays, but cross-sibling calls via `super::*`) |
| `envelope_wrap_results` | control_hub_tool.rs:1921 | control_hub_tool_helpers.rs |
| `map_dispatch_error` | control_hub_tool.rs:1956 | control_hub_tool_helpers.rs |
| `mod control_hub_tests { ... }` (L2017-2526) | control_hub_tool.rs:2017 | control_hub_tool_tests.rs |
| `empty_context` helper | control_hub_tool.rs:2023 | control_hub_tool_tests.rs |
| All `#[test]` fns (22 total) | control_hub_tool.rs:2040-2510 | control_hub_tool_tests.rs |

## 5. Cross-sibling import rules

Per R9 + R13b confirmed: `pub(super)` is the standard for sibling-to-sibling visibility.

**Visibility rules**:
- Items only used by facade: no `pub` modifier (private to file)
- Items used by ≥ 1 sibling + facade: `pub(super)`
- Items used by tests only: `pub(super)` so test sibling can access

**Sibling → facade** (most common):
```rust
// in control_hub_tool_browser.rs
use super::{ControlHubTool, ...};  // facade items
```

**Facade → sibling**:
```rust
// in control_hub_tool.rs (facade)
use super::control_hub_tool_browser::handle_browser;
use super::control_hub_tool_terminal::handle_terminal;
use super::control_hub_tool_meta::handle_meta;
use super::control_hub_tool_helpers::{description_text, parse_browser_kind, ...};
```

**Tests sibling**:
```rust
// in control_hub_tool_tests.rs
use super::control_hub_tool::ControlHubTool;
use super::control_hub_tool_helpers::{map_dispatch_error, parse_bracket_code_prefix, parse_hints_suffix};
```

## 6. Migration plan

**Order of operations** (lowest risk first):
1. **Step 1**: Extract helpers (`parse_browser_kind`, `parse_bracket_code_prefix`, `parse_hints_suffix`, `description_text`, `envelope_wrap_results`, `map_dispatch_error`) → `control_hub_tool_helpers.rs`. Add `pub(super)` to each. Verify `cargo check -p northhing-core --lib`.
2. **Step 2**: Extract `handle_terminal()` → `control_hub_tool_terminal.rs`. Update facade to import. Verify `cargo check`.
3. **Step 3**: Extract `handle_meta()` → `control_hub_tool_meta.rs`. Update facade. Verify `cargo check`.
4. **Step 4**: Extract `handle_browser()` + browser helpers + `BROWSER_SESSIONS` → `control_hub_tool_browser.rs`. Update facade. Verify `cargo check`.
5. **Step 5**: Extract `mod control_hub_tests` → `control_hub_tool_tests.rs`. Update facade `mod` declaration. Verify `cargo test`.
6. **Step 6**: Verify facade final state ≤ 220 lines, all 22 tests pass with `--features 'service-integrations,product-full'` if needed.

After each step, verify:
```bash
cargo check -p northhing-core --lib 2>&1 | tee /tmp/check.log
# line count per sibling file (target: facade ≤ 220, others within budget)
```

## 7. Test plan

**Test preservation** (iron rule — 0 NEW tests, 0 removed):
- All 22 existing tests must continue to pass unchanged.
- Test file moves to `control_hub_tool_tests.rs`. Test bodies UNCHANGED.
- `empty_context()` helper moves with tests (used by `unknown_domain_is_rejected_with_message_listing_valid_domains`).

**Test command** (per R14 lesson — feature-flag gate):
```bash
# First try default features
cargo test -p northhing-core --lib control_hub_tool 2>&1 | tee /tmp/test.log
# If "0 tests" or test count drops, switch to full features
cargo test -p northhing-core --lib --features 'service-integrations,product-full' control_hub_tool 2>&1 | tee /tmp/test.log
```

**Baseline**:
- Pre-split: 899/0/1 cargo test
- Post-split target: 899/0/1 cargo test (same)
- Control-hub specific: 22 tests in `mod control_hub_tests`

## 8. Iron rules Δ check

Per project iron rules (R11 lesson — pre-existing vs new violations):
- **0 NEW unwrap/expect** in production code (tests OK)
- **0 NEW panic!** anywhere
- **0 NEW let _ =** assignments
- **0 NEW `.clone()` of large owned data** (move instead)
- **pub(super)** is the standard for sibling visibility (NOT pub)

**Pre-existing items** (must NOT be touched):
- Any unwrap/expect already in `control_hub_tool.rs` at HEAD = `fa8890e`. Do not refactor.
- If a pre-existing unwrap is in code you're moving, leave it as-is. Do not "improve" it.

**Reference**: `C:\Users\UmR\.qclaw\skills\code-rot-guard\references\coding-agent-rules.md`

## 9. Verification gates

1. **Line counts** (all must be within target):
   - facade `control_hub_tool.rs` ≤ 220
   - `control_hub_tool_meta.rs` ≤ 220
   - `control_hub_tool_browser.rs` ≤ 750
   - `control_hub_tool_terminal.rs` ≤ 130
   - `control_hub_tool_helpers.rs` ≤ 90
   - `control_hub_tool_tests.rs` ≤ 520
2. **cargo check** clean on `northhing-core` lib
3. **cargo test -p northhing-core --lib** 899/0/1 unchanged
4. **No new iron rule violations** (grep `unwrap()\|expect(\|panic!\|let _ =` against pre-fix baseline)
5. **All public items still pub** — no API surface change
6. **Imports reconciled** — `pub use` re-exports in facade for any items moved out that external code might import (unlikely; check with grep first)

## 10. Commits

Per project convention (R14 pattern):
1. `refactor(control-hub-tool): R16 sub-domain split (facade ≤ 220 + 5 siblings)`
2. `docs(handoff): R16 handoff + review guide`

## 11. Review guide (Mavis will auto-generate after merge)

Will follow R14 review guide template:
- What to review (focus on owner mapping table integrity + 22 tests pass)
- Critical observations (line counts, sibling visibility)
- Questions for reviewer
- Sign-off request (APPROVE/REJECT + score)