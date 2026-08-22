# Task T4a Report — scheduler.rs

## Status

**DONE**

## File

`src/agentic/src/scheduler.rs` — 325 lines (within the < 800 limit).

## Commit

```
1c986a4 feat(growth): add pure turn scheduling decisions
```

## Verification Output

### `cargo test -p northhing-agentic-growth`

```
running 121 tests
...
test scheduler::tests::auto_pause_event_fires_only_once ... ok
test scheduler::tests::after_garden_sweep_gate_is_closed ... ok
test scheduler::tests::below_auto_pause_threshold_no_event ... ok
test scheduler::tests::decide_turn_both_closed ... ok
test scheduler::tests::decide_turn_both_gates_open ... ok
test scheduler::tests::decide_turn_distill_open_garden_not_due ... ok
test scheduler::tests::decide_turn_distill_paused_garden_open ... ok
test scheduler::tests::garden_sweep_both_zero_returns_false ... ok
test scheduler::tests::garden_sweep_clock_backwards_returns_false ... ok
test scheduler::tests::garden_sweep_exact_interval_returns_true ... ok
test scheduler::tests::garden_sweep_from_zero_to_interval_returns_true ... ok
test scheduler::tests::garden_sweep_one_ms_below_interval_returns_false ... ok
test scheduler::tests::has_hit_turns_does_not_pause ... ok
test scheduler::tests::hit_turns_increments_only_on_produced_facts ... ok
test scheduler::tests::paused_state_still_increments_turns ... ok
test scheduler::tests::saturating_add_at_max_does_not_panic ... ok
test scheduler::tests::should_distill_returns_false_when_paused ... ok
test scheduler::tests::should_distill_returns_true_when_not_paused ... ok
test scheduler::tests::triggers_auto_pause_at_twenty ... ok
...
test result: ok. 121 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

All 121 tests pass (18 new scheduler tests + 103 pre-existing tests from other modules). Zero failures.

### `cargo check -p northhing-agentic-growth`

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.39s
```

Clean check, no warnings.

## §2 Status Semantics — Correspondence

### 2.1 Distillation Pause Gate

**Host code** (`turn_persist.rs:458-463`): checks `distiller_paused == "true"` from judge KV store.

**This module**: `should_distill()` (line ~85) — `!state.distill.paused`. The `paused` field on `DistillStats` is the direct equivalent of the legacy `distiller_paused` flag. Tested by `should_distill_returns_true_when_not_paused` / `should_distill_returns_false_when_paused`.

### 2.2 Counting and Self-Learning Brake

**Host code** (`turn_persist.rs:484-514`):
- `distill_turns` incremented every turn unconditionally (even when paused).
- `distill_hit_turns` incremented only when facts are produced.
- Auto-pause at `turns >= 20 && hit_turns == 0`.

**This module**: `record_distill_outcome()` (line ~105):
1. `state.distill.turns = state.distill.turns.saturating_add(1)` — always, even when paused.
2. `produced_facts` -> `state.distill.hit_turns.saturating_add(1)`.
3. Brake check: `turns >= 20 && hit_turns == 0` sets `paused = true`.
4. Returns `Some(AutoPauseEvent)` only on `false -> true` transition.

Tests:
- `below_auto_pause_threshold_no_event` — at 18 -> 19, no pause.
- `triggers_auto_pause_at_twenty` — at 19 -> 20, `paused=true`, returns `Some`.
- `has_hit_turns_does_not_pause` — at 19/1 -> 20/1, `paused=false`.
- `hit_turns_increments_only_on_produced_facts` — true increments, false does not.
- `paused_state_still_increments_turns` — 30 -> 31 even when already paused.

### 2.3 Garden (Dream) Sweep Gate

**Host code** (`dream.rs:47-62`): `now_ms.saturating_sub(last_sweep) < DREAM_SWEEP_INTERVAL_MS` — guard returns early.

**This module**: `should_run_garden_sweep()` (line ~92) — `now_ms.saturating_sub(last_sweep_at_ms) >= GARDEN_SWEEP_INTERVAL_MS`. Constant `GARDEN_SWEEP_INTERVAL_MS = 24 * 60 * 60 * 1000`.

Tests:
- `garden_sweep_exact_interval_returns_true` — exactly 24h -> true.
- `garden_sweep_one_ms_below_interval_returns_false` — 24h - 1ms -> false.
- `garden_sweep_both_zero_returns_false` — 0,0 -> false.
- `garden_sweep_from_zero_to_interval_returns_true` — 0 -> 24h -> true.
- `garden_sweep_clock_backwards_returns_false` — `last > now` -> false (saturating_sub to 0).

### 3.4 Behavioural Deviation (Spec §3.4)

**Deviation**: The legacy host re-applies the brake and emits a `warn!` on every subsequent turn while paused (because `turns` keeps growing but `hit_turns` stays at 0, so the condition `turns >= 20 && hit_turns == 0` remains true indefinitely). This module returns `Some(AutoPauseEvent)` only on the `false -> true` transition; subsequent calls while already paused return `None`.

**Evidence**: Test `auto_pause_event_fires_only_once`:
- State `{turns: 19, hit_turns: 0, paused: false}`.
- 1st call: `turns=20`, `paused=true`, returns `Some(AutoPauseEvent { turns: 20 })`.
- 2nd call: `turns=21`, `paused=true`, returns `None`.
- 3rd call: `turns=22`, `paused=true`, returns `None`.
- This proves the event fires exactly once.

**Documentation**: The module `//!` docstring at the top of `scheduler.rs` records this deviation explicitly under "Behavioural deviation from legacy (recorded per spec §3.4)".

## Git Status

```
 M src/agentic/src/scheduler.rs
```
(Staged and committed as `1c986a4`; working tree clean after commit.)

## Deviations from Brief

None. All requirements are met:
- Only `scheduler.rs` was modified (325 lines, was 1).
- Zero new dependencies.
- All functions are pure (no IO, no time sampling, no global state).
- No `unwrap()` / `expect()` in non-test code; all arithmetic uses `saturating_*`.
- English-only, no emoji.
- `cargo fmt` was not run.
- No host wiring code was added.
- `state.rs`, `ports.rs`, `lib.rs`, and `src/crates/**` were not touched.
