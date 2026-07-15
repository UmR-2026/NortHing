# R13c Spec: manager_session_lifecycle.rs 706 -> god method split

> Branch: `impl/round13c-session-lifecycle-god-split`
> Worktree: `E:\agent-project\northing-impl-round13c`
> Date: 2026-06-29
> Author: Mavis (m2.7-highspeed)

## Context

R13b closed the R13 D-deviation by splitting `manager.rs` 2303 -> 196 + 7 sub-siblings.
R13b put `manager_session_lifecycle.rs` at 706 lines (under 800 cap, OK), but inside it
Kimi flagged 3 god methods that were not split. User opted to do this as R13c follow-up.

## Scope

Mechanical god method split per R7/R12 pattern. **No new sub-sibling files**, only
refactoring inside `manager_session_lifecycle.rs`. Mavis reviews directly (no external
reviewer per user direction 2026-06-29).

## 3 god methods to split (target line count: 350-450)

### 1. `establish_session` (236 lines) -> 4 phase helpers + thin orchestrator
- `prepare_session_transport` (Phase 1: TCP connect + private key loading)
- `perform_session_handshake` (Phase 2: SSH handshake using prepared transport)
- `perform_session_auth` (Phase 3: Authenticate on the handshake handle)
- `resolve_session_server_info` (Phase 4: Resolve server_info + probe home_dir)
- Orchestrator: ~10 lines

### 2. `execute_command_internal` (212 lines) -> 3 phase helpers + thin orchestrator
- `execute_open_channel` (Phase 1: Open exec channel + send command)
- `execute_pump_loop` (Phase 2: Pump channel until exit/timeout/interrupt)
- `execute_finalize_result` (Phase 3: Apply exit code fallback logic -1/124/130)
- Orchestrator: ~6 lines

### 3. `ensure_alive_or_reconnect` (152 lines) -> 4 phase helpers + thin orchestrator
- `check_alive_and_drift` (Phase 1: Read saved config + detect config drift)
- `acquire_reconnect_lock` (Phase 2: Acquire reconnect lock + re-check under lock)
- `prepare_reconnect_config` (Phase 3: Refresh password from vault + prepare config)
- `perform_reconnect` (Phase 4: Call establish_session + update connections map)
- Orchestrator: ~10 lines

## Iron rules (MUST enforce)

1. 0 NEW unwrap/panic/let _ = Result
2. `pub(super)` for new phase helpers
3. Behavior IDENTICAL (god method split is mechanical)
4. No new dependencies
5. `#[allow(dead_code)]` if rustc complains (none expected)

## Verification

```bash
cargo check -p services-integrations --features remote-ssh-concrete --lib  # 0 errors
cargo test -p northhing-core --features product-full --lib  # 899 passed; 0 failed; 1 ignored
cargo test -p services-integrations --features remote-ssh-concrete --lib  # 9 passed; 0 failed
cargo fmt --check  # 0 diff
git diff main..HEAD -- <file> | grep -cE '^\+.*unwrap\(\)|^\+.*panic!'  # 0
wc -l <file>  # 350-450 lines
```
