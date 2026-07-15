# Round 13 review guide: remote_ssh/manager.rs 2810 → 1 facade + 3 sub-handlers

> Reviewer (QClaw / Kimi): please review commit range
> `811b22f..3b5f520` (spec → refactor) on branch
> `impl/round13-remote-ssh-manager-split`. Handoff doc:
> `docs/handoffs/2026-06-29-round13-remote-ssh-manager-split-impl.md`.

## What to review

| File | Lines | Note |
|---|---|---|
| `src/crates/services/services-integrations/src/remote_ssh/manager.rs` (reduced from 2810 → 2303) | 2303 | facade: SSHConnectionManager + helpers + tests |
| `src/crates/services/services-integrations/src/remote_ssh/manager_handler.rs` (new) | 251 | SSHHandler + HandlerError + 4 impls (Russh callback) |
| `src/crates/services/services-integrations/src/remote_ssh/manager_session.rs` (new) | 103 | PTYSession + Display + Drop |
| `src/crates/services/services-integrations/src/remote_ssh/manager_port_forward.rs` (new) | 191 | PortForward + Direction + PortForwardManager + Default |
| `src/crates/services/services-integrations/src/remote_ssh/mod.rs` | 52 | +3 pub mod + re-exports |

## Critical observations (please verify)

### 1. D-deviation: facade 2303 lines > 800 cap

The split moved **types** (SSHHandler, PTYSession, PortForwardManager + their associated
methods) to 3 sibling sub-handlers. But the 24 `connection_*` fns and ~50 misc fns
remained in facade. Spec §2.2 target was ~700 lines; actual 2303.

**Severity**: 187% over 800 cap (worse than R12 D1 at 169%, worse than R8 at 104%).

**Cause**: Worker correctly applied R11a struct-owner mapping but did NOT apply R7
god-method split (extract large methods into phase helpers in siblings). The
`establish_session` (~250 lines) and `execute_command_internal` (~500 lines) are still
in facade as monolithic functions.

**R13c recommended** (per handoff doc):
- Extract `manager_saved_connections.rs` (~250) + `manager_remote_workspace.rs` (~150)
  + `manager_sftp.rs` (~360) → facade to ~1500
- Or split `establish_session` / `execute_command_internal` into phase helpers
  (R7 pattern) to bring facade below 800

### 2. pre-existing vs NEW violations (R11b lesson)

R13 split preserved **16 pre-existing** unwrap/expect/panic occurrences verbatim
across 4 files (8 in handler, 8 in facade).

```bash
git diff main..HEAD -- src/crates/services/services-integrations/src/remote_ssh/ \
  | grep -cE '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# expect 0
```

Verify: 0 NEW unwrap/panic introduced. All 16 pre-existing preserved.

### 3. Cross-sibling visibility (no cyclic deps)

| Edge | Visibility | Notes |
|---|---|---|
| facade → manager_handler::SSHHandler | `pub(crate)` in handler | facade constructs `Handle<SSHHandler>` |
| facade → manager_session::PTYSession | `pub` (re-exported via mod.rs) | facade constructs `PTYSession::new` |
| facade → manager_port_forward::PortForwardManager | `pub` (re-exported via mod.rs) | facade constructs |
| handler → facade KnownHostEntry | `pub` in facade | handler stores `Vec<KnownHostEntry>` |

Verify: no cyclic imports, no `pub` exposure of crate-private types.

### 4. Public API unchanged

mod.rs re-exports 6 cfg-gated items (preserved from original):

```rust
pub use manager::{KnownHostEntry, SSHConnectionManager};
pub use manager_session::PTYSession;
pub use manager_port_forward::{PortForward, PortForwardDirection, PortForwardManager};
```

Cross-crate callers (`git grep 'use.*remote_ssh::manager'`): 1 file
(`remote_terminal.rs`) uses `crate::remote_ssh::manager::SSHConnectionManager` directly.
All other 25+ files use `crate::service::remote_ssh::*` (top-level mod.rs re-exports).

**SSHHandler / HandlerError** are NOT re-exported (crate-private per spec §1.2).
Verify: `pub(crate)` visibility, no `pub use` in mod.rs.

### 5. Iron rules verification

```bash
# 0 NEW unwrap/panic/unreachable
git diff main..HEAD -- src/crates/services/services-integrations/src/remote_ssh/ \
  | grep -cE '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# Expect: 0

# 0 NEW let _ = Result
git diff main..HEAD -- src/crates/services/services-integrations/src/remote_ssh/ \
  | grep -cE '^\+.*let _ = .*Result'
# Expect: 0

# 0 fns dropped (97 → 97)
py -c "
import re
from pathlib import Path
wt_dir = Path(r'src/crates/services/services-integrations/src/remote_ssh')
new_fns = set()
for f in wt_dir.glob('*.rs'):
    new_fns.update(re.findall(r'^\s*(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)', f.read_text(encoding='utf-8'), re.M))
print(f'worktree fns: {len(new_fns)}')
print('expected: 97')
"
```

### 6. Cargo verification

```bash
# Baseline preserved
cargo check -p northhing-services-integrations --features remote-ssh-concrete --lib  # 0 errors
cargo check -p northhing-core --features product-full --lib  # 0 errors (warnings pre-existing)
cargo test -p northhing-core --features product-full --lib  # 899 passed; 0 failed; 1 ignored
cargo test -p services-integrations --features remote-ssh-concrete --lib  # 9 passed; 0 failed
cargo fmt --check services-integrations  # clean
```

All checks verified pre-merge.

### 7. Worker hit 2 errors mid-implementation (handoff §"Step-by-step commits")

| Step | Error | Fix |
|---|---|---|
| 2 (manager_session.rs) | `PTYSession` field privacy (channel/connection_id private) | Added `pub(crate) fn new(channel, connection_id)` constructor |
| 3 (manager_port_forward.rs) | `pub use pub(crate) HandlerError/SSHHandler` not allowed | Omit `pub use` for crate-private items; document in mod.rs |

Both errors handled within R13 (no rollback needed). Verify error recovery logic is sound.

## Questions for reviewer

1. **R13c necessity**: facade 2303 lines is worse than R12 D1 (1693) and worse than R8
   round_executor (1631). Should R13c be required like R12b was, or accepted with
   D-deviation flag?

2. **`establish_session` / `execute_command_internal` extraction**: should R13c apply
   R7 god-method split pattern (extract phase helpers within facade) instead of
   extracting more sub-domain siblings?

3. **pre-existing 16 unwrap**: verified preserved. Acceptable to leave as-is
   (per R11b lesson "do not fix pre-existing debt") or call out for cleanup?

4. **`remote_exec.rs` 1195 > 1000 cap**: documented as separate R13b. Confirm
   priority vs R13c.

5. **SSHHandler / HandlerError not re-exported**: spec §2.3 listed them but spec §1.2
   said `pub(crate)`. Worker omitted per the stricter §1.2. Verify this is correct
   (Rust does not allow `pub use` of `pub(crate)` items outside crate).

## Refs

- R13 spec: `docs/handoffs/2026-06-29-round13-remote-ssh-manager-split-spec.md` (811b22f)
- R13 handoff: `docs/handoffs/2026-06-29-round13-remote-ssh-manager-split-impl.md` (3b5f520)
- R12 review (precedent for pre-existing vs new): `docs/handoffs/2026-06-29-round12-task-tool-split-review-report.md` (1db6001)
- R12b handoff (thin facade pattern): `docs/handoffs/2026-06-29-round12b-task-tool-deep-review-secondary-split-impl.md` (bfa662f)
- Iron rules reference: `C:\Users\UmR\.qclaw\skills\code-rot-guard\references\coding-agent-rules.md`

## Sign-off request

Please provide:
1. **APPROVE / REJECT** decision with score (1-10)
2. List of any **minor observations** (non-blocking)
3. Confirmation of R13c necessity decision (required or accepted with D-deviation)
4. Any structural concerns about the 4-sibling + thin facade layout
5. Recommendation for R13c scope (god-method split vs more sub-domain extraction)

Reply format: standard project review report ending in
`*-review-report.md` (will be committed by reviewer per established pattern).