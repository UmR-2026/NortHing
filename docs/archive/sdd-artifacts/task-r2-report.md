# Task R-2 Report: Self-pause recovery (eliminate "memory silent permanent death")

## Status: DONE

Two recovery paths (probe cadence + user-intent wake) implemented, window
reset on resume, symmetric resume observability. All §6 verification green,
no new warnings, `turn_persist.rs` net line delta = 0.

## 1. Probe cadence: exact definition + why it cannot drift

**Anchor:** `DistillStats.paused_at_turns` - a snapshot of `turns` taken at
the moment the brake engages (in `record_distill_outcome`, after the
unconditional `turns += 1`). Recorded once, never mutated while paused.

**Probe condition** (`scheduler::is_probe_window`):

```
paused && turns > paused_at_turns && (turns - paused_at_turns) % DISTILL_AUTO_PAUSE_TURNS == 0
```

**Why it cannot drift:** the anchor is an absolute `turns` value captured
exactly once at pause time. The modulo is computed against that fixed
absolute value on every turn, so every probe lands on an exact multiple of
`DISTILL_AUTO_PAUSE_TURNS` turns after the anchor - regardless of how many
non-probe turns elapsed between probes. There is no running counter to
decrement, no per-turn mutation of the anchor, and no off-by-one
accumulation. The `turns > paused_at_turns` guard excludes the pause turn
itself (difference 0, where `0 % N == 0` would otherwise fire immediately).

**Counting from the pause turn** (begin reads `turns` before the per-turn
increment): turns `paused_at_turns+1 .. paused_at_turns+N-1` do not distil;
the turn whose `turns == paused_at_turns+N` is the first probe. Subsequent
probes fire at `paused_at_turns+2N`, `paused_at_turns+3N`, ... A missed
probe does **not** mutate the anchor, so the next window still arrives
(test `probe_miss_stays_paused_and_next_window_arrives` proves two
consecutive windows fire).

## 2. New `DistillStats` field + backward compatibility

Added `pub paused_at_turns: u64` to `DistillStats` (`state.rs:25`).

**Backward compatibility:** the struct already carries `#[serde(default)]`
at the struct level (`state.rs:17`), so the new field is optional in old
blobs and deserialises to `0`. Two tests prove this:
- `old_blob_without_paused_at_turns_deserialises` - a bare `DistillStats`
  JSON with no `paused_at_turns` deserialises with the field defaulting to 0.
- `old_growth_state_blob_without_paused_at_turns_round_trips` - a full
  `GrowthState` blob in the pre-field schema round-trips through
  serialise/deserialise stably.

## 3. Window reset location + §3.3 observation confirmation

**Reset location:** `scheduler::reset_window` (private helper), called by
both resume paths:
- `record_distill_outcome` (probe hit, path A) - `scheduler.rs:178`.
- `resume_for_user_intent` (user intent, path B) - `scheduler.rs:247`.

Both zero `turns`, `hit_turns`, `paused_at_turns` and clear `paused`, so the
brake can re-engage after another 20 zero-hit turns.

**§3.3 observation - CONFIRMED.** The auto-pause condition at
`scheduler.rs:185` is:

```rust
&& state.distill.hit_turns == 0
```

This means: once a user accumulates `hit_turns >= 1` at any point in their
lifetime, the brake can **never** re-engage (the condition is perpetually
false), so the 20-turn LLM-cost ceiling loses its upper bound for that user.
This is the verbatim T4a semantic inherited from the legacy host code.
Without the window reset added by this task, the resume paths would inherit
the same flaw (a resumed distiller that immediately produces a fact would
have `hit_turns >= 1` and could never pause again). The reset clears
`hit_turns` to 0 on every resume, so the brake CAN re-engage after resume.
The broader "hit once => brake permanently disarmed" property for users who
never paused is untouched by this task and is the correct input for T13.

## 4. Phrase table vs `facts.rs:245-248`

`facts.rs:245-248` holds a near-identical bilingual table used as the **LLM
distillation keyword fallback** (`distill_facts_from_user_message`): it
decides which sentences to extract as candidate facts when the LLM path is
unavailable.

This task's table (`scheduler::detect_memory_intent`,
`scheduler.rs:225`) has a different purpose - **wake the paused distiller**.
The two are intentionally separate (documented in the function's doc
comment):

| Aspect | `facts.rs:245-248` | `detect_memory_intent` |
|---|---|---|
| Purpose | Extraction fallback (catch candidate sentences) | Wake判定 (lift the brake) |
| Includes bare `别`/`不要` | Yes (breadth) | **No** (excluded per spec §3.2: `别的`/`不要紧` over-trigger, pure negation) |
| Keeps `别忘` | n/a (uses `别再`) | Yes (second char pins intent) |
| False-positive cost | Extra candidate fact (filtered downstream) | One extra distillation + brake lift (low) |
| False-negative cost | Missed candidate fact (low) | Memory stays silently dead (high) |

The crate (`northhing-agentic-growth`, layer 6) cannot depend on
`northhing-core`, so reuse is impossible; the duplication is accepted and
documented in both the function doc comment and this report. `facts.rs` was
not modified.

## 5. `turn_persist.rs` net line delta

**Net delta = 0.** The only change is `turn_persist.rs:524`:

```diff
-            Ok(db) => growth_adapter::begin_distill_turn(db),
+            Ok(db) => growth_adapter::begin_distill_turn(db, user_input),
```

`user_input` was already in scope (used at `:558` by `boost_turn_topics`).
No new statements, no new lines. File remains 799 lines (measured via
`(Get-Content -LiteralPath <file> -Encoding UTF8).Count`), matching the
hard cap.

## 6. §6 verification - full raw output

### 6.1 `cargo test -p northhing-agentic-growth`

```
running 131 tests
... (all ok) ...
test result: ok. 131 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

New total: **131** (was 121; +10 new tests: probe cadence, wake phrases,
resume fire-once, backward-compat blob deserialisation).

### 6.2 `cargo check -p northhing-core --features product-full`

```
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
```

**19 warnings - baseline held, no new warnings, no errors.**

### 6.3 `cargo test -p northhing-core --features product-full growth_adapter`

```
running 27 tests
... (all ok) ...
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 1146 filtered out; finished in 0.28s
```

**27** (was 25; +2 host-side resume round-trip tests).

### 6.4 `cargo test -p northhing-core --features product-full turn_persist`

```
running 12 tests
... (all ok) ...
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 1161 filtered out; finished in 0.09s
```

**12** (unchanged).

### 6.5 `cargo test -p northhing-core --features product-full memory_db`

```
running 21 tests
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 1152 filtered out; finished in 0.20s
```

**21** (unchanged).

### 6.6 `node scripts/check-core-boundaries.mjs`

```
Core boundary check passed.
exit=0
```

### 6.7 Line counts (measured via `(Get-Content -LiteralPath <file> -Encoding UTF8).Count`)

| File | Lines | Limit | OK |
|---|---|---|---|
| `turn_persist.rs` | 799 | <=799, net delta 0 | yes |
| `growth_adapter.rs` | 799 | <800 | yes |
| `scheduler.rs` | 693 | <800 | yes |
| `state.rs` | 315 | <800 | yes |

## 7. Changed files

1. `src/agentic/src/state.rs` - added `paused_at_turns: u64` to `DistillStats`
   (+ doc comment); updated one test constructor.
2. `src/agentic/src/scheduler.rs` - probe cadence (`is_probe_window`),
   `should_distill` probe gate, `detect_memory_intent`, `resume_for_user_intent`,
   `reset_window`, `ResumeEvent`/`DistillTransition` types, `record_distill_outcome`
   resume-on-probe-hit; +10 tests.
3. `src/crates/assembly/core/src/agentic/growth_adapter.rs` - `begin_distill_turn`
   now takes `user_input` and runs path B resume (info log + persist);
   `finish_distill_turn` handles `DistillTransition` (warn on pause, info on
   probe-hit resume); `wall_now_ms` helper; +2 host round-trip tests; updated
   existing tests for the new signature.
4. `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs`
   - single call-site update: `begin_distill_turn(db, user_input)` (net 0 lines).

## 8. Observability decision (spec §3.4)

- **Auto-pause** (`false -> true`): `warn!` (unchanged) - a degradation.
- **Resume via user intent** (path B, in `begin_distill_turn`): `info!` - a
  user-driven resume is an expected, healthy event, not a degradation.
- **Resume via probe hit** (path A, in `finish_distill_turn`): `info!` - the
  brake lifted because hit-rate recovered; healthy.
- Both resume events fire exactly once on the `true -> false` transition;
  repeated calls while already unpaused return `None` (fire-once preserved).

## 9. Concerns

None blocking. Two notes for the ledger / downstream:

1. **§3.3 📌 confirmed** (see §3 above): `hit_turns == 0` in the pause
   condition means a user who has ever hit once can never be auto-paused.
   This task's window-reset makes the brake re-engageable **after a resume**,
   but does not change the lifetime property for never-paused users. Input
   for T13.
2. **Probe cadence wording:** spec test 2 informally says "第 20 轮放行探针".
   The implemented cadence lets the first probe through at the turn whose
   `turns == paused_at_turns + DISTILL_AUTO_PAUSE_TURNS` (i.e. the turn
   after 20 completed post-pause turns, because `should_distill` reads
   `turns` before the per-turn increment). This is the only non-drifting
   interpretation of the "record turns at pause, use difference modulo N"
   hint in spec §3.1; the exact cadence is documented in
   `is_probe_window`'s doc comment and proven by
   `probe_window_first_probe_at_anchor_plus_n`.
