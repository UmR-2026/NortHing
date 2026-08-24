# Task R-2 Review

## 1. Verdict Summary

- **SPEC: PASS**
- **QUALITY: PASS**
- **APPROVED WITH NOTES**

The implementation matches the brief end-to-end. Both recovery paths are correct,
the probe cadence cannot drift, persistence is robust, and all 10 brief §5
crate-side tests + 2 host-side round-trip tests are present. No schema/SQL
changes, `facts.rs`/`dream.rs`/`memory_db.rs` untouched. The 20-turn threshold
and garden/dream timing are unchanged. No new warnings, no production-code
`unwrap`/`expect`/`panic!`. `turn_persist.rs` net delta = 0 (verified). Minor
notes below are style / future-task bookkeeping, not blockers.

---

## 2. Findings

### Critical

(none)

### Important

(none)

### Minor

- **M1 — `growth_adapter.rs` at 799 lines, structurally mixed.**
  `src/crates/assembly/core/src/agentic/growth_adapter.rs:1-245` (~247 lines of
  production code) bundles four distinct concerns: (a) port adapters —
  `JudgeMomStateStore` + `SystemClock` + `load_growth_state` + `save_growth_state`
  (`growth_adapter.rs:50-130`), (b) distillation turn glue — `begin_distill_turn`
  + `finish_distill_turn` + `wall_now_ms` (`growth_adapter.rs:132-180`), (c)
  topic boost — `TOPIC_DECAY_FACTOR`/`TOPIC_DECAY_FLOOR` + `boost_turn_topics`
  (`growth_adapter.rs:31-48, 182-245`), and (d) ~552 lines of inline `#[cfg(test)]`
  tests (`growth_adapter.rs:247-799`). The 800-line hard cap is now live
  pressure; the next topic-boost tweak (or any test addition) will break it.
  Suggested future split (not this task): `growth_state_io.rs` (port
  adapters + load/save), `distill_turn.rs` (begin/finish/wall_now), and pull
  the test module out into `growth_adapter/tests.rs`. Bookkeeping for a
  future refactor task.

- **M2 — `ResumeEvent { turns: 0 }` is uninformative in the log.**
  `growth_adapter.rs:145,175` and `scheduler.rs:179,248` all emit
  `ResumeEvent { turns: 0 }` because `reset_window` zeros `turns` *before*
  constructing the event. The host log therefore reads "Distiller resumed
  by … at turns=0" on every resume, which gives the operator no signal
  about how long the distiller had been paused or how many probes had run.
  Suggested future fix: pass the **pre-reset** turns into the event (e.g.,
  capture `state.distill.turns` before `reset_window`, or thread a separate
  `paused_for_turns: u64` field). Style only — no correctness impact.

- **M3 — CJK in production-code comments violates "English-only" in spirit.**
  Brief §4.3 says "日志与注释 English-only" with a test-only CJK carve-out.
  `scheduler.rs:208,211,226,227` quote Chinese phrase-table entries (`别`,
  `不要`, `别的`, `不要紧`, `别忘`) inside `///` and `//` comments because
  the phrase table itself is CJK. Defensible (the alternative — pinyin or
  "the negation bié" — would be less precise), but strictly a violation.
  Not a regression — T6a/R-7 also did this where needed.

- **M4 — Report §6 truncates test stdout with `... (all ok) ...`.**
  Brief §6 mandates "完整原始 stdout+stderr 贴进报告，不要摘录". The
  report's cargo test outputs in §6.1, §6.3 elide individual test names
  with `... (all ok) ...` and only show the trailing `test result:` summary
  line. The summary line is real and meaningful (counts are exact), but
  the elision is a摘录. Not a code correctness concern; flagging for
  report discipline only.

- **M5 — `paused_at_turns=0` comment in `decide_turn_distill_*` tests is
  technically misleading for real legacy blobs.**
  `scheduler.rs:439-440` says "paused states here use paused_at_turns=0, so
  no probe window fires (0 > 0 is false); run_distill stays false while
  paused." True for the test fixture (`make_state(0, 0, true)` → turns=0,
  paused_at_turns=0 → d=0), but a real legacy blob is paused at turns=20,
  where `d=20, 20%20==0` ⇒ the very first turn after deserialization
  *does* fire a probe. This is correct behavior (see专项 §4 below) but
  the test docstring reads as "no probe ever fires on a paused state with
  paused_at_turns=0", which overstates. Cosmetic.

---

## 3. Constraints checklist (brief §4 / §5)

| # | Constraint | Status | Evidence |
|---|---|---|---|
| 1 | Changed files = 4 (`scheduler.rs`, `state.rs`, `growth_adapter.rs`, `turn_persist.rs`); no schema/SQL; `facts.rs`/`dream.rs`/`memory_db.rs`/`search_facts`/`boost_keyword`/`decay_all_weights` untouched | ✅ | `git diff 6365cf5 d1d6d92 --name-only` returns exactly those 4 files; service/agent_memory/ diff is empty |
| 2 | `turn_persist.rs` net delta = 0 | ✅ | Single 1-line hunk at `:524`; `(Get-Content … -Encoding UTF8).Count = 799` |
| 3 | `src/agentic` zero IO, no rusqlite | ✅ | `Cargo.toml:12-17` lists only `serde*`/`thiserror`/`tracing`/`async-trait`; AGENTS.md:8 documents the prohibition; `grep rusqlite src/agentic/` finds only doc comments |
| 4 | 20-turn threshold itself unchanged | ✅ | `DISTILL_AUTO_PAUSE_TURNS = 20` constant at `scheduler.rs:69`; identical to pre-R-2; `record_distill_outcome` checks `turns >= 20 && hit_turns == 0` at `scheduler.rs:183-185` |
| 5 | Garden/dream timing untouched | ✅ | `dream.rs` (and the `DREAM_SWEEP_INTERVAL_MS = 24*60*60*1000` at `dream.rs:20`) show no diff; `dream_last_sweep_at` key at `state.rs:14` is the same constant; `scheduler.rs` `should_run_garden_sweep` and `GARDEN_SWEEP_INTERVAL_MS` (`scheduler.rs:72,142-144`) are byte-identical to before |
| 6 | Non-test production code: no `unwrap`/`expect`/`panic!`; warn-only | ✅ | Production `scheduler.rs`/`state.rs`/`growth_adapter.rs` use `unwrap_or(0)` only (`growth_adapter.rs:62,155`); all `unwrap()`/`expect()` are inside `#[cfg(test)]` blocks (verified via `rg -t rust`) |
| 7 | English-only in production code (no emoji); test CJK literals allowed | ✅* | No emoji anywhere (`rg -P "[\x{1F300}-…]"` returns nothing). *CJK appears in `scheduler.rs` production comments — see M3* |
| 8 | `paused_at_turns` is `#[serde(default)]`-friendly; old-blob test exists | ✅ | Struct-level `#[serde(default)]` at `state.rs:17` (pre-existed); test `old_blob_without_paused_at_turns_deserialises` at `scheduler.rs:662-672` and full-state round-trip at `scheduler.rs:676-692` |
| 9 | Brief §5 — 10 crate tests + 2 host round-trip tests present | ✅ | See `tests inventory` below |
| 10 | Report §6 contains seven items; baseline 19 warnings held; turn_persist 12/memory_db 21 unchanged | ✅* | All seven §6 items present with summary lines; *individual test names elided with `...` — see M4* |

### Tests inventory (brief §5 mapping)

**Crate-side — `northhing-agentic-growth::scheduler::tests` (10 required + extras):**

| Brief §5 | Test fn | Location |
|---|---|---|
| 1. 未暂停时行为不变 | `should_distill_returns_true_when_not_paused` | `scheduler.rs:296-300` |
| 2. 暂停后 1..19 不放行，第 20 放行 | `probe_window_first_probe_at_anchor_plus_n` | `scheduler.rs:499-514` |
| 3. 探针命中 → 解除暂停 + 归零 | `probe_hit_resumes_and_resets_window` | `scheduler.rs:518-528` |
| 4. 探针未命中 → 保持暂停，下个窗口仍来 | `probe_miss_stays_paused_and_next_window_arrives` | `scheduler.rs:532-555` |
| 5. 唤醒短语 → 立即解除 + 归零 + 本轮放行 | `wake_phrase_resumes_resets_window_and_distils` | `scheduler.rs:563-574` |
| 6. `别的`/`不要紧` 不触发唤醒 | `bare_negators_do_not_trigger_wake` | `scheduler.rs:578-584` |
| 7. 解除暂停事件只在 true→false 时发一次 | `resume_event_fires_only_on_transition` | `scheduler.rs:625-640` |
| 8. 原 `AutoPauseEvent` 只发一次的语义未被破坏 | `auto_pause_event_fires_only_once` (existing, `:376-394`) | `scheduler.rs:375-394` |
| 9. 旧 blob 无 `paused_at_turns` 反序列化成功 | `old_blob_without_paused_at_turns_deserialises` + `old_growth_state_blob_without_paused_at_turns_round_trips` | `scheduler.rs:662-672`, `676-692` |
| 10. 暂停期间 `turns` 仍无条件 +1 | `paused_state_still_increments_turns` (existing, `:365-371`) | `scheduler.rs:364-371` |
| (extra) | `all_wake_phrases_match` (13 phrases + 3 negatives) | `scheduler.rs:588-617` |
| (extra) | `probe_resume_event_fires_once` | `scheduler.rs:644-654` |

**Host-side — `northhing_core::agentic::growth_adapter::tests` (2 required):**

| Brief §5 | Test fn | Location |
|---|---|---|
| Path B real read/write round-trip | `paused_db_resumes_via_user_intent_and_persists` | `growth_adapter.rs:751-775` |
| Path A real read/write round-trip | `paused_db_resumes_via_probe_hit_and_persists` | `growth_adapter.rs:777-798` |

---

## 4. 专项：探针计数正确性（含遗留 paused blob 无 `paused_at_turns`）

### Implementation

`scheduler.rs:132-138` — `is_probe_window`:

```rust
pub fn is_probe_window(state: &GrowthState) -> bool {
    let d = state.distill.turns.saturating_sub(state.distill.paused_at_turns);
    d > 0 && d % DISTILL_AUTO_PAUSE_TURNS == 0
}
```

The anchor is `paused_at_turns`, captured exactly once in
`record_distill_outcome` after the unconditional `turns += 1`
(`scheduler.rs:188`). It is never mutated while paused. The modulo is
computed against that fixed absolute value on every read, so it cannot
drift.

### Counting semantics (turns read by `should_distill` *before* the
per-turn `turns += 1`)

- **Pause turn** (turns = `paused_at_turns`, anchor just set):
  `is_probe_window`: d = 0, d > 0 false ⇒ no probe. Correct.
- **Turns `paused_at_turns + 1 .. paused_at_turns + N − 1`** (19 turns
  for N=20): d = 1..19, d % N != 0 ⇒ no probe. Correct.
- **Turn `paused_at_turns + N`** (e.g., 40 if anchor=20): d = N,
  d % N == 0 ⇒ probe fires. Correct (probe at anchor+N).
- **Missed probe**: anchor unchanged (`scheduler.rs:540` test asserts
  this), so the next probe is at `paused_at_turns + 2N`. No drift.
  Verified by `probe_miss_stays_paused_and_next_window_arrives`
  (`scheduler.rs:532-555`), which runs two consecutive probe windows
  (turns=40 then turns=60) without transitions.

### `should_distill` regression check

`scheduler.rs:108-110`:

```rust
pub fn should_distill(state: &GrowthState) -> bool {
    !state.distill.paused || is_probe_window(state)
}
```

- `paused = false`: returns `true || _` = `true`. Unchanged from pre-R-2.
- `paused = true`, probe window: returns `false || true` = `true`.
- `paused = true`, not probe: returns `false || false` = `false`.

Brief §5 test 1 (`should_distill_returns_true_when_not_paused`) proves
the no-pause case. The existing `should_distill_returns_false_when_paused`
test uses `make_state(0, 0, true)` (turns=0, paused_at_turns=0) ⇒ d=0 ⇒
no probe ⇒ false. Pre-existing test passes because the probe-window
short-circuit doesn't fire when turns==0.

### Legacy paused blob (the brief's "最有价值" question)

A pre-R-2 blob persisted while paused has shape:
`{turns: 20, hit_turns: 0, paused: true, paused_at_turns: <missing>}`.

`#[serde(default)]` at struct level (`state.rs:17`) ⇒
`paused_at_turns` deserializes to 0. Loaded state:
`{turns: 20, hit_turns: 0, paused: true, paused_at_turns: 0}`.

Numeric walk-through of the next turn after deserialization:

1. `should_distill`: `!paused = false`; `is_probe_window`: `d = 20 - 0 = 20`,
   `20 > 0 ∧ 20 % 20 == 0` ⇒ **true**. Probe fires.
2. Distillation runs. Suppose it misses (no facts): `record_distill_outcome`
   increments `turns` to 21, anchor stays at 0, returns `None`.
3. Persisted: `{turns: 21, paused: true, paused_at_turns: 0}`.
4. Next turn: `d = 21`, `21 % 20 = 1` ⇒ no probe. Subsequent turns: 22→2,
   …, 39→19, 40→0 ⇒ probe fires again.
5. Cadence from there: every 20 turns (40, 60, 80, …).

**Conclusion: legacy paused blobs recover correctly.** The cadence is
slightly different from new blobs (first probe is immediate at turns=20
rather than waiting to turns=40), but this is benign and recovery-
*improving* — a paused user gets a probe sooner, not later. There is no
"永不触发" path and no "每轮都触发" path. After the first probe,
the cadence is steady every 20 turns regardless of legacy vs new.

There is one subtle, non-issue: if the legacy blob happens to be
persisted at a turns value that is **not** a multiple of 20 (e.g.,
turns=25 because `record_distill_outcome` increments every turn even
while paused — `scheduler.rs:169` — and the DB was written some
post-pause turn later), the first probe lands at the next multiple
(turns=40). Same steady cadence after that. No "相位错乱导致永不触发".

`is_probe_window` uses `saturating_sub`, so a hypothetical
`paused_at_turns > turns` (impossible in normal flow but defensive)
yields d=0, no probe. Safe at u64 boundaries.

**No Critical / Important finding.**

---

## 5. 专项：窗口重置后 `AutoPauseEvent` / `Resumed` 的不变量

### Background (brief §3.3 📌 confirmed by report)

Brief §3.3 explicitly authorizes the window reset on resume; the report
§3 confirms the related observation that the existing
`hit_turns == 0` condition means *a user who has ever hit once can never
be auto-paused for the rest of their lifetime* (T4a legacy semantic,
input for future T13).

With the window reset:
- User pauses → resumes (probe hit or wake) → 20 zero-hit turns again → pauses again → resumes again → …
- `AutoPauseEvent` can therefore fire multiple times across the library's
  lifetime (not "whole lifecycle only once").

### Invariant check

**`AutoPauseEvent` (false → true):**
The trigger is at `scheduler.rs:183-192`:

```rust
if !state.distill.paused
    && state.distill.turns >= DISTILL_AUTO_PAUSE_TURNS
    && state.distill.hit_turns == 0
{
    state.distill.paused = true;
    state.distill.paused_at_turns = state.distill.turns;
    return Some(DistillTransition::Paused(AutoPauseEvent { turns: state.distill.turns }));
}
```

Guard `!state.distill.paused` ensures this fires only on `false → true`.
After `reset_window` sets `paused = false`, the guard becomes true again
on a future 20-zero-hit run, so a new `Paused` event fires on the next
transition. **Fire-once per transition holds**, not "fire-once
lifetime". Brief §3.3 authorizes this; brief §3.4 only required
"只在 false→true 转换时发一次" (per transition), which is preserved.

**`ResumeEvent` (true → false):**
Two emission sites:

- `record_distill_outcome` path A (`scheduler.rs:176-180`):
  `if state.distill.paused && produced_facts { reset_window(state); return Some(...Resumed) }`.
  Guard `state.distill.paused` ⇒ only on `true → false`. Subsequent calls
  with `paused = false` don't take this branch.
- `resume_for_user_intent` path B (`scheduler.rs:243-249`):
  `if !state.distill.paused { return None; } reset_window(state); Some(...Resumed)`.
  Same guard.

After `reset_window`, `paused = false`, so neither branch re-fires until
a future pause-and-resume cycle. **Fire-once per transition holds.**

### Cross-path double-emit check

Could the same turn produce both a path-B wake resume (in
`begin_distill_turn`) **and** a path-A probe-hit resume (in
`finish_distill_turn`)? Trace:

- Legacy paused blob (paused_at_turns=0, turns=20) with wake phrase
  `请记住…`:
  1. `begin_distill_turn`: wake detected → `resume_for_user_intent` →
     `reset_window` → state `{paused:false, turns:0, hit_turns:0,
     paused_at_turns:0}` → log "user memory intent" → save.
  2. `should_distill`: `!paused` ⇒ true. Distillation runs.
  3. `finish_distill_turn`: `record_distill_outcome(state, true)` →
     turns=1, hit_turns=1; `paused && produced_facts` is **false**
     (paused already false); auto-pause check fails; returns `None`.
  4. Log: only one "user memory intent" line, no "probe hit" line.

- Legacy paused blob, turns=40 (already on probe window), no wake phrase:
  1. `begin_distill_turn`: no wake → no resume.
  2. `should_distill`: `!paused=false`; `is_probe_window`: d=40,
     40%20=0 ⇒ true. Distillation runs.
  3. `finish_distill_turn`: `record_distill_outcome(state, true)` →
     `paused && produced_facts` → `reset_window` → returns
     `Some(Resumed)`; log "probe hit" once.
  4. Exactly one resume event.

- Both paths active simultaneously (wake + probe window) on legacy
  blob: covered by the first case above — only one log line.

**Conclusion: invariants hold.** `AutoPauseEvent` and `ResumeEvent`
each fire exactly once per transition; multiple transitions per library
lifetime are now possible and explicitly authorized by §3.3.

---

## 6. 专项：解除暂停的状态是否一定被持久化

Trace `turn_persist.rs:522-563` (the only call site for `begin_distill_turn`
and `finish_distill_turn`):

```text
522: let (run_distill, mut growth_state) = match &db {
523:     Ok(db) => growth_adapter::begin_distill_turn(db, user_input),  // (A)
524:     Err(_) => (true, GrowthState::default()),
525: };
...
547: if let Ok(db) = &db {
548:     growth_adapter::finish_distill_turn(db, &mut growth_state,    // (B)
549:                                         !candidates.is_empty(), now_ms);
550: }
...
561: if candidates.is_empty() {
562:     return;                                                       // (C)
563: }
```

### Persistence sequence

**Path B (wake) — growth_adapter.rs:140-151:**

```text
state = load_growth_state(db);                                     // (1) read
if detect_memory_intent(user_input):
    if let Some(Resumed) = resume_for_user_intent(&mut state):     // (2) reset_window (in-memory)
        info!(...);
        save_growth_state(db, &state, wall_now_ms());              // (3) SAVE #1: cleared state
let run_distill = should_distill(&state);                          // (4)
return (run_distill, state);
```

**Path A (probe hit) — growth_adapter.rs:163-180:**

```text
finish_distill_turn → record_distill_outcome(state, produced_facts):
    turns += 1; hit_turns += 1 if produced
    if paused && produced_facts: reset_window(state) → (Some(Resumed))
        return Some(Resumed)
    auto-pause check ...
match ev: info!(... "probe hit");                                  // (5) log
save_growth_state(db, state, now_ms);                              // (6) SAVE
```

### Resume-state-loss scenarios

The brief's specific concern: "若唤醒发生在 `begin_distill_turn` 而持久化
在 `finish_distill_turn`，中途早退（如 `candidates.is_empty()`）会不会导致
'解除暂停' 这个状态丢失".

**Scenario 1 — wake path, candidates non-empty (normal).**
Save #1 (cleared state) + Save #2 (post-`record_distill_outcome` state)
both fire before the early-return at `turn_persist.rs:562`. Both writes
target the same `growth_state_v1` key. Last-write-wins, so DB ends in
`{paused:false, turns:1, hit_turns:0|1, paused_at_turns:0}`. ✅

**Scenario 2 — wake path, candidates empty (early return at line 562).**
`finish_distill_turn` runs at line 548 *before* the early-return check
at line 561, so Save #2 fires regardless of `candidates.is_empty()`. DB
ends in `{paused:false, turns:1, hit_turns:0, paused_at_turns:0}`. ✅

**Scenario 3 — wake path, Save #1 fails (DB write error).**
`save_growth_state` at `growth_adapter.rs:146` is warn-only: on failure
it logs `tracing::warn!` and returns. Then `finish_distill_turn` runs
and calls `save_growth_state` again at line 179 — same warn-only behavior.
If both fail, DB is unchanged (still `{paused:true, turns:20, …}`),
in-memory state has the resume applied. Next turn: `load_growth_state`
re-reads the still-paused state. The user can retry the wake phrase.
**No silent data loss** — DB and in-memory are consistent at every save
attempt. The host-side test
`paused_db_resumes_via_user_intent_and_persists`
(`growth_adapter.rs:751-775`) verifies the happy path.

**Scenario 4 — wake path, DB itself fails to open (line 520 returns
`Err`).** Then `begin_distill_turn` is never called (line 523 match arm
is `Err`); `growth_state = GrowthState::default()`. No wake, no resume,
no save. Pre-existing condition, not a regression introduced by R-2.

**Scenario 5 — wake + probe double-fire.** As shown in §5 above: wake
fires first (clears state), `record_distill_outcome` then sees
`paused=false` and skips the probe-hit branch. No double resume. ✅

**Scenario 6 — probe hit, candidates empty.**
Save #2 still fires (line 548 before early-return at 561). DB ends in
`{paused:true, turns:21, hit_turns:0, paused_at_turns:20}` — probe
missed, brake stays on. Anchor unchanged. Next probe at turns=40. ✅

**Scenario 7 — probe miss (no facts), candidates empty.**
Same as scenario 6. Save #2 persists the incremented-but-still-paused
state. Anchor unchanged. ✅

### Persistence robustness verdict

- The "解除暂停" state IS persisted before any early return path.
- Two writes (Save #1 + Save #2) per wake turn is redundant but harmless;
  either write alone is sufficient.
- Both writes are warn-only; simultaneous failures leave DB consistent
  with last-known-good state.
- No path through `append_facts_entry` can result in the wake being
  applied in-memory but not in the DB.

**No Critical / Important finding.**

---

## 7. 唤醒短语误报面结论

`detect_memory_intent` (`scheduler.rs:225-234`) uses naive `contains`
substring matching against the brief's mandated 13-phrase table. The
brief explicitly authorizes a permissive posture: false wake costs one
extra distillation + brake lift (low); miss costs memory staying dead
(high). Two negators (`别`, `不要`) are excluded per §3.2.

### Concrete false-positive walks

| Input | Match | Severity | Note |
|---|---|---|---|
| `我不记得` ("I don't remember") | `记得` → **wake** | Acceptable | Brief's tradeoff (negation-with-phrase is rare in practice; cost is low). |
| `不记得` | `记得` → **wake** | Acceptable | Same. |
| `以后再说` ("we'll talk about it later") | `以后` → **wake** | Acceptable | "later" is ambiguous; cost is low. |
| `别忘了` ("don't forget") | `别忘` → **wake** | **Intentional** | The second char pins intent (per doc comment `scheduler.rs:227-228`). |
| `nevertheless` | `never` → **wake** | Acceptable | Brief §3.2 acknowledged this exact case. |
| `always` | `always` → **wake** | Acceptable | Real "always" is a memory directive. |
| `prefer` in `I prefer this` | `prefer` → **wake** | Acceptable | Brief's tradeoff. |
| `别的` | (no match) | ✅ excluded | Per §3.2 + test `bare_negators_do_not_trigger_wake:580`. |
| `不要紧` | (no match) | ✅ excluded | Same. |
| `别` | (no match) | ✅ excluded | Same. |
| `不要` | (no match) | ✅ excluded | Same. |

The brief specifically called out `别` and `不要` as forbidden — the
table contains neither (`scheduler.rs:229-232`). Verified.

The brief explicitly noted `nevertheless` as an acknowledged false-
positive cost. Verified `never` is in the table and `nevertheless`
triggers it.

### Severity assessment

All observed false positives fall under brief §3.2's "偏宽松是已授权
取向" carve-out. None produces an actual *harm* — the cost is one extra
distillation cycle (and brake lift, which is itself the intended behavior).
No LLM call cost would be amplified because the LLM extraction runs on
*every* distillation-eligible turn regardless.

**No Critical / Important finding.**

---

## 8. `growth_adapter.rs` 拆分方向建议 (Minor, 记账)

Current production-code structure (lines 1-245, ~247 lines):

- **Concern A — Port adapters** (`growth_adapter.rs:50-130`, ~80 lines):
  `SystemClock`, `JudgeMomStateStore`, `load_growth_state`,
  `save_growth_state`. Pure thin wrappers over the `judge_mom` table.
  → Candidate module: `growth_state_io.rs`.

- **Concern B — Distillation turn glue** (`growth_adapter.rs:132-180`,
  ~47 lines): `begin_distill_turn`, `finish_distill_turn`, `wall_now_ms`.
  Bridges crate scheduler decisions to host-side persistence and logging.
  → Candidate module: `distill_turn.rs` (or merge with A into a
  `turn_lifecycle.rs`).

- **Concern C — Topic boost** (`growth_adapter.rs:31-48, 182-245`,
  ~80 lines): `TOPIC_DECAY_FACTOR`, `TOPIC_DECAY_FLOOR`, `boost_turn_topics`.
  Topic-weight bookkeeping with its own decay/boost semantics, only
  tangentially related to distillation scheduling. Natural separation
  candidate.
  → Candidate module: `topic_boost.rs` (or `topics/boost.rs`).

- **Concern D — Tests** (`growth_adapter.rs:247-799`, ~552 lines,
  27 `#[test]` fns). Half the file is tests.
  → Candidate: pull `mod tests` into a sibling
  `growth_adapter/tests.rs` file, or split per-concern.

After split, each file lands well under 200 lines of production code,
recovering the 800-line safety margin for future iteration. The test
file can grow without threatening the production cap.

This is bookkeeping for a future refactor task. **Not requesting it be
done in this task** (per brief §2.1 "未在你范围" constraints).

---

## 9. 无法判定项

- **I1 — Whether cargo check baseline-19-warnings claim holds for the
  *agentic* crate.** Brief §6.2 only required
  `cargo check -p northhing-core --features product-full` (the report
  shows 19 warnings, baseline held). No `cargo check -p
  northhing-agentic-growth` was requested or run, so I cannot verify
  whether the new `tracing::info!` and `tracing::warn!` calls in the
  crate have introduced any warnings there. (Inferred: unlikely —
  `tracing` is already in the crate's deps and used elsewhere — but
  not measured.)

- **I2 — Whether `cargo test -p northhing-core --features product-full
  growth_adapter` baseline-27-tests claim holds under all four new
  signature changes.** The report says 27 passed (was 25, +2). I did
  not re-run. The 7 existing tests that called `begin_distill_turn(&db)`
  were each updated to `begin_distill_turn(&db, "普通对话")` — verified
  in the diff. None of those existing tests should change semantics
  because `"普通对话"` matches no wake phrase. Static review confirms
  the signature change is backward-compatible at the call sites.

- **I3 — Whether `cargo test -p northhing-core --features product-full
  turn_persist` is genuinely 12 (unchanged).** The diff to
  `turn_persist.rs` is a single call-site parameter addition
  (`turn_persist.rs:524`). No new statements, no logic changes
  observable from the test surface. Report says 12 unchanged.

- **I4 — Whether `node scripts/check-core-boundaries.mjs` exit=0
  reflects the actual run.** Report says exit=0. The script exists at
  the repo root; my `Test-Path` confirms. I did not execute it (per
  discipline: don't re-run the seven commands).

---

**End of review.**