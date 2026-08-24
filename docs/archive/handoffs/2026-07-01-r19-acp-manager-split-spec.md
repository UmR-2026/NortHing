# R19 Spec — acp/client/manager.rs 2519 → facade + 10 sub-siblings

## Context

`acp/client/manager.rs` is the largest remaining god-object in the project
after R5–R18 closed the others. Kimi flagged it as P1 critical (along with
`terminal/exec.rs` 2488, `runtime-ports/src/lib.rs` 2460,
`session_usage/service.rs` 2458, `config/types.rs` 2406) — the entire P1
backlog is god-objects >2400 lines.

`bitfun-acp` crate (`src/crates/interfaces/acp`) already has a partial split:
`requirements.rs` 755, `stream.rs` 711, `session_options.rs` 284,
`tool.rs` 237 — these are sibling files that `manager.rs` calls into. R19
closes the **god `manager.rs`** itself.

**Pre-R19 measurement method**: Canonical `wc -l` /
`[System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8).Count`
(per R18 addendum, NOT PowerShell `Measure-Object -Line` which excludes
blank lines).

## Baseline (must preserve)

- Worktree: new `E:\agent-project\northing-impl-r19-acp-manager-split` on
  branch `impl/r19-acp-manager-split` from main HEAD `3b21292` (R18 merged).
- `cargo check -p bitfun-acp` → 0 errors
- `cargo test -p bitfun-acp` → pre-existing baseline TBD (run preflight)
- Iron rules Δ = 0:
  ```bash
  # Pre-split unwrap() baseline (precise grep, mandatory)
  git show main:src/crates/interfaces/acp/src/client/manager.rs | grep -cE '\bunwrap\(\)'
  # Expected: 0 (must re-derive baseline before commit, do NOT inherit from
  # reviewer's number — Kimi R17 review claimed "4 unwraps in browser_session"
  # but actual was 0; same risk here)

  git show main:src/crates/interfaces/acp/src/client/manager.rs | grep -cE '\bexpect\('
  # Expected: TBD (re-derive)

  git show main:src/crates/interfaces/acp/src/client/manager.rs | grep -cE 'let _\s*=\s*Result'
  # Expected: TBD (re-derive)
  ```

## Pre-emptive split design (canonical wc-l, NOT MO Line)

`manager.rs` 2519 lines → **1 thin facade + 10 sub-siblings (11 files total)**.
Split designed **upfront** so producer doesn't split-judge at runtime
(R18 lesson: producer pre-emptively-split is faster than runtime judgment).

### File inventory (all ≤242 strict cap, 220 target)

| File | Target ≤220 (≤242 QClaw tolerance) | Owns |
|---|---|---|
| `manager.rs` (thin facade) | ≤220 | `AcpClientService` struct + `new()` + 7 constants + 4 type aliases + 4 small entry methods (create_flow_session_record, delete_flow_session_record, load_json_config, save_json_config) |
| `manager_config.rs` | ≤242 | list_clients (236-268) + probe_client_requirements (270-347) + refresh_remote_client_requirements (348-364) + private load_configs (1476-1479) + load_config_file (1480-1483) + load_config_value (1484-1491) + register_configured_tools (1492-1514) + probe_remote_client_requirements (365-446 private) |
| `manager_install.rs` | ≤100 | predownload_client_adapter (447-462) + install_client_cli (463-493) — 47 lines |
| `manager_connection.rs` | ≤242 | initialize_all (209-234) + start_client_for_session (494-510) + start_client_connection (511-657) + cleanup_failed_startup (658-666) + stop_client (667-679) + stop_connection (680-703) |
| `manager_transport.rs` | ≤220 | run_startup_step (1406-1429) + attach_remote_session (1430-1475) + start_local_transport (1586-1635) + open_transport_for_connection (1636-1658) + start_remote_transport (1659-1689) + resolve_start_client_config (1690-1731) |
| `manager_session.rs` | ≤242 | release_northhing_session (704-792) + get_session_options (849-883) + get_session_commands (884-914) + set_session_model (915-1012) + resolve_client_session (1225-1255) + resolve_or_create_client_session (1256-1283) + ensure_remote_session (1284-1405) |
| `manager_prompt.rs` | ≤220 | prompt_agent (1013-1063) + prompt_agent_stream (1064-1165) — 153 lines + dispatcher if needed |
| `manager_cancel.rs` | ≤90 | cancel_agent_session (1166-1200) + cancel_northhing_session (1201-1224) — 59 lines |
| `manager_permission.rs` | ≤130 | submit_permission_response (825-848) + handle_permission_request (1515-1578) + permission_mode_for_session (1579-1585) — 95 lines |
| `manager_process.rs` | ≤242 | impl AcpClientConnection (1801-1864) + free fns: resolve_config_for_client (1732-1743) + ensure_remote_client_supported (1744-1759) + render_remote_client_command (1760-1793) + current_unix_timestamp_ms (1794-1799) + wait_for_client_connection (1824-1849) + configure_process_group (1903-1913) + terminate_child_process_tree (1914-2004) + close_or_cancel_remote_session (2005-2061) |
| `manager_session_helpers.rs` | ≤242 | free fns: parse_config_value (1850-1864) + build_session_key (1865-1873) + session_client_connection_id (1874-1877) + aggregate_client_status (1878-1902) + new_session_response_from_load (2062-2072) + new_session_response_from_resume (2073-2083) + drain_pending_turn_updates (2084-2135) + read_turn_to_string (2136-2165) + drain_pending_turn_text (2166-2209) + append_agent_text (2210-2217) + drain_pending_session_metadata_updates (2218-2261) + discard_pending_session_updates_if_needed (2262-2312) + update_session_from_events (2313-2318) + update_session_context_usage (2319-2329) + update_session_available_commands (2330-2343) + update_session_config_options (2344-2354) |
| `manager_errors.rs` | ≤130 | free fns: protocol_error (2355-2360) + startup_timeout_error (2361-2364) + startup_timeout_error_message (2365-2374) + is_startup_timeout_error (2375-2378) + select_permission_by_kind (2379-2405) + select_permission_option_id (2406-2519) |

**Total: 11 files (1 facade + 10 siblings). Each file ≤242 (QClaw tolerance).**

### Domain split rationale

| Domain | Why grouped |
|---|---|
| Config & tools | All `load_configs`/`register_configured_tools`/`probe_client_requirements`/`list_clients` — pure read paths over persisted configs |
| Install | Pre-flight predownload + install — distinct lifecycle phase (47 lines, own file due to domain isolation) |
| Connection lifecycle | start/stop + start_client_connection internal + cleanup_failed_startup + initialize_all — all spawn/teardown |
| Transport | All transport setup (local + remote) + startup step helper + remote session attachment — configures the actual byte streams |
| Session | resolve + ensure + release + config options + model — all session state management |
| Prompt & stream | prompt_agent + prompt_agent_stream — request/response streaming (own sibling due to generator complexity) |
| Cancel | cancel_agent_session + cancel_northhing_session — minimal cancellation surface |
| Permission | submit_permission_response + handle_permission_request + permission_mode_for_session — permission flows |
| Process (free fns) | Child process lifecycle: AcpClientConnection impl + terminate_child_process_tree + configure_process_group — owns the `Child` lifecycle |
| Session helpers (free fns) | Session event drain/update + session key construction + aggregate_client_status — pure helpers for session event flow |
| Errors (free fns) | Error mapping + permission option selection + startup timeout detection — small focused helpers |

## Iron rules (MUST enforce — adapted from R17 + R18)

1. **0 NEW unwrap/panic/let _ = Result** in production code — preserve
   pre-existing counts verbatim (re-derive baseline above before commit).
2. **All sibling methods use `impl AcpClientService { ... }` blocks** for
   inherent-method dispatch (multiple `impl` blocks for same type allowed).
3. **`pub(super)` visibility**: All sibling handlers are `pub(super)` so the
   facade thin dispatcher can resolve them.
4. **No caller migration**: All public methods keep their signatures; callers
   continue to call `service.method()` as before.
5. **Single cargo check**: batch ALL edits before running `cargo check`. R8 +
   R14 lesson — 4min × N cycles is catastrophic.
6. **Read source from git HEAD**: Python split script (if used) must read
   from `git show main:path`, never from on-disk file (R8 self-overwrite bug).
7. **PowerShell safety** (Windows): do NOT use `>` redirect or `Set-Content`
   without `-Encoding UTF8` for any `.rs` file. Use the Write tool or `node`
   /`python` scripts with explicit `encoding='utf-8'`. **CRLF will silently
   corrupt `cargo check`**.
8. **`core.autocrlf=false`** must be set locally in the new worktree BEFORE
   first checkout (R17 gotcha).
9. **Line cap**: every file ≤220 (≤242 with QClaw tolerance). Verify with
   canonical `wc -l` (NOT `Measure-Object -Line`). Per R18 addendum, cite
   measurement method in commit message.
10. **Line length**: ≤120 chars per line. **≤5 new long lines per file is
    tolerable** (R18 long-line tolerance rule); >5 requires multi-line
    string literal or split fn. 120-char cap unchanged.
11. **No existing siblings touched**: `requirements.rs`, `stream.rs`,
    `session_options.rs`, `tool.rs`, `tool_card_bridge/*`,
    `remote_capability_store.rs`, `remote_session.rs`, `remote_shell.rs`,
    `config.rs`, `builtin_clients.rs`, `mod.rs` (other than adding new
    `pub mod`) — out of scope.
12. **No public API rename**: 22 pub methods keep their signatures.

## Cross-sibling imports

Each sibling file imports what it needs from `manager.rs` types and from
the existing sibling files. Pattern:

```rust
// manager_config.rs (example)
use super::builtin_clients::builtin_client_ids;
use super::config::{AcpClientConfig, AcpClientConfigFile, AcpClientInfo,
                    AcpClientRequirementProbe, AcpClientStatus};
use super::requirements::{acp_requirement_spec, probe_executable,
                          probe_npm_adapter};
use super::stream::{acp_dispatch_to_stream_events_with_tracker, ...};
use super::tool::AcpAgentTool;
use super::AcpClientService;
```

The facade `manager.rs` (thin) re-exports nothing from siblings; callers
just keep using `service.method()` directly (inherent-method dispatch).

## mod.rs registration

```rust
// client/mod.rs — add 10 new pub mod:
pub mod manager_config;
pub mod manager_connection;
pub mod manager_cancel;
pub mod manager_errors;
pub mod manager_install;
pub mod manager_permission;
pub mod manager_process;
pub mod manager_prompt;
pub mod manager_session;
pub mod manager_session_helpers;
pub mod manager_transport;
// pub mod manager; (already present, retains all its current items minus moved ones)
```

## Test path

`acp` crate currently has minimal tests (per cargo test baseline). Verify
with `cargo test -p bitfun-acp`. If tests exist, they exercise the public
`AcpClientService` API → sibling handlers via inherent-method dispatch.
Test bodies unchanged.

## Verification commands

```bash
# 0. Worktree preflight (R17/R18 autocrlf gotcha)
cd E:/agent-project/northing-impl-r19-acp-manager-split
git config --local core.autocrlf false  # Must: false

# 1. Re-derive baseline (BEFORE any code change)
echo "==unwrap() baseline (precise grep, must re-derive)=="
git show main:src/crates/interfaces/acp/src/client/manager.rs | grep -cE '\bunwrap\(\)'
echo "==expect() baseline=="
git show main:src/crates/interfaces/acp/src/client/manager.rs | grep -cE '\bexpect\('
echo "==let _ = Result baseline=="
git show main:src/crates/interfaces/acp/src/client/manager.rs | grep -cE 'let _\s*=\s*Result'
echo "==cargo test preflight=="
cargo test -p bitfun-acp 2>&1 | tee baseline-acp-test.log
echo "==cargo check preflight=="
cargo check -p bitfun-acp 2>&1 | tee baseline-acp-check.log
```

```bash
# 2. Post-split verification
# 2a. Canonical line count (per file, wc -l)
wc -l src/crates/interfaces/acp/src/client/manager*.rs
# Expected: facade ≤220, every sibling ≤242, total ~2500

# 2b. Iron rules — post-split unwrap count = baseline
echo "Post-split unwrap():"
wc -l src/crates/interfaces/acp/src/client/manager*.rs | grep -v '^      '
grep -cE '\bunwrap\(\)' src/crates/interfaces/acp/src/client/manager*.rs | tee /dev/null | grep -v ':0$' || echo "All post-split files have 0 unwrap()"
# If baseline > 0, sum across files = baseline

# 2c. Iron rules — 0 NEW unwrap/panic/let _ =
git diff main..HEAD -- src/crates/interfaces/acp/src/client/manager*.rs | grep -cE '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# Expected: 0 (no NEW unwrap/panic in production code)

# 2d. Build + test
cargo check -p bitfun-acp --message-format=short 2>&1 | grep -c 'error\['
# Expected: 0
cargo test -p bitfun-acp 2>&1 | grep '^test result:'
# Expected: same as baseline (TBD)

# 2e. Format (no NEW fmt issues in R19-touched files)
cargo fmt --check -- src/crates/interfaces/acp/src/client/ 2>&1 | grep '^Diff in' | grep -E 'manager[a-z_]*\.rs' | wc -l
# Expected: 0 (pre-existing fmt issues in unrelated files are OK)

# 2f. LF enforcement
file src/crates/interfaces/acp/src/client/manager*.rs | grep -c 'CRLF'
# Expected: 0

# 2g. Line length (Kimi R16 Bug 4 verification)
awk '{ if (length > 120) print FILENAME":"NR" "length" chars" }' src/crates/interfaces/acp/src/client/manager*.rs
# Expected: ≤5 lines per file (R18 long-line tolerance)
# Document each new long line in commit message

# 2h. Cross-crate callers preserved
git grep -n 'acp::client::manager::' -- ':!src/crates/interfaces/acp/'
# Expected: 0 hits (manager is internal to crate, callers use AcpClientService methods)

git grep -n 'AcpClientService' -- ':!src/crates/interfaces/acp/'
# Expected: hits preserved (inherent-method dispatch resolves)
```

## Commit pattern

Single commit on `impl/r19-acp-manager-split`:

```
refactor(bitfun-acp): R19 split manager.rs 2519 → facade + 10 sub-siblings (canonical wc-l)

R19 closes the bitfun-acp manager god-object (2519 lines, Kimi P1 critical).

11 files (1 facade + 10 siblings, all ≤242 QClaw tolerance):
- manager.rs 2519 → ~150 facade (struct + new + 4 small entry methods)
- manager_config.rs (~290) — config listing + probing + private helpers
- manager_install.rs (~50) — predownload + install
- manager_connection.rs (~280) — lifecycle: start/stop/initialize
- manager_transport.rs (~280) — transport setup + remote session attachment
- manager_session.rs (~430) — session resolution + config + lifecycle
- manager_prompt.rs (~160) — prompt + streaming
- manager_cancel.rs (~60) — cancellation
- manager_permission.rs (~100) — permission flows
- manager_process.rs (~280) — process lifecycle + AcpClientConnection impl
- manager_session_helpers.rs (~470) — session event/drain/update helpers
- manager_errors.rs (~165) — error helpers

Measurement method: canonical wc -l (per R18 addendum). All line counts cited
use this method.

Iron rules: 0 NEW unwrap/panic/let _ = Result (re-derived baseline from
git show main:manager.rs | grep -cE '\bunwrap\(\)').
Kimi Bug 3 fix protocol: pre-commit grep baseline re-derived, not inherited
from prior reviewer.
Long-line tolerance (R18 rule): ≤5 new long lines per file tolerable.
Tests: TBD baseline preserved.
LF enforcement: 0 CRLF.
```

## Deliverables

1. Spec doc (this file)
2. Refactor commit on branch `impl/r19-acp-manager-split`
3. Handoff doc: `docs/handoffs/2026-07-01-r19-acp-manager-split-impl.md`
4. Review guide: `docs/handoffs/2026-07-01-r19-acp-manager-split-review.md`
5. Plan deliverable: `C:\Users\UmR\.mavis\plans\<plan-id>\outputs\impl-r19-acp-manager-split\deliverable.md`

## Risk assessment

**Low risk**:
- Pure file split + method move — no behavior change
- 0 NEW unwraps (verified via re-derivation, not inheritance)
- Tests preserved (test bodies don't move; only handler locations change)
- Cross-crate callers unaffected (`AcpClientService` methods keep signatures)

**Medium risk**:
- 10 new sibling files for `impl AcpClientService { ... }` blocks — Rust
  allows multiple impl blocks per type, inherent dispatch resolves
- free fns span across multiple sibling files — some helpers used by
  multiple impl methods, need careful cross-sibling import setup
- 22 pub methods keep signatures — easy to verify but easy to typo

**Mitigation**:
- Single cargo check at end (no incremental checks)
- Preserve all comments verbatim (R17 lesson)
- Re-derive unwrap/expect/let _ = baseline via precise grep BEFORE commit
  (Kimi Bug 3 fix protocol)
- Canonical wc -l measurement (R18 lesson)

## Cross-round follow-ups (R20+ backlog, not R19 scope)

- `terminal/exec.rs` 2488 (next P1 candidate)
- `runtime-ports/src/lib.rs` 2460
- `session_usage/service.rs` 2458
- `config/types.rs` 2406
- Plus `bitfun-acp` crate-level: `requirements.rs` 755 borderline (could
  split further but borderline acceptable) + `stream.rs` 711 borderline