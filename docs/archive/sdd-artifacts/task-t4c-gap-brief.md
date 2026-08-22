# T4c gap investigation — read-only report

## Location and baseline

- Source worktree (read only): `E:\agent-project\northing\.worktrees\growth-core-0804`
- Expected branch: `feat/growth-core-0804`
- Expected HEAD: `8b64aa8`
- Write the report only to: `E:\agent-project\northing\.superpowers\sdd\task-t4c-gap-report.md`
- Do not edit source, tests, plans, ledgers, or model notes. Do not commit.
- Do not dispatch another subagent.

## Why this investigation exists

The original T4 plan required four scattered turn hooks to converge behind one
`GrowthCore::on_turn_finalized` facade while preserving ordering and warn-only behavior.
The plan's file locations and most of its underlying assumptions are now stale after
T4a, T4b, T6a, R-7, R-2, and S-1. The current code has already moved scheduler decisions,
distill state, facts gating, and topic boosting into focused helpers. The user requested a
complete gap report before deciding whether to implement T4c now or defer the remaining
facade work to T12, which already must rewrite dream/garden behavior on this path.

## Authoritative inputs

Read these before drawing conclusions:

1. `E:\agent-project\northing\.superpowers\sdd\handoff-2026-08-06.md`
2. `E:\agent-project\northing\.superpowers\sdd\plan-2026-08-04-growth-core.md`, especially
   T4, T12, R-2, R-7, and global constraints
3. `E:\agent-project\northing\.superpowers\sdd\progress.md`, Growth Core entries for T4a,
   T4b, T6a, R-7, R-2, S-1, and T5b
4. Worktree root `AGENTS.md`
5. `src/crates/assembly/core/AGENTS.md`
6. `src/agentic/AGENTS.md`
7. `docs/architecture/core-decomposition.md` (it is known to contain mojibake; use the
   readable rules and repository guides as the operative constraints)

## Files and symbols to trace

At minimum, read the complete relevant functions and their callers in:

- `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs`
- `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist_facts.rs`
- `src/crates/assembly/core/src/agentic/growth_adapter.rs`
- `src/crates/assembly/core/src/service/agent_memory/dream.rs`
- `src/crates/assembly/core/src/service/agent_memory/distiller.rs`
- `src/agentic/src/lib.rs`
- `src/agentic/src/scheduler.rs`

Trace at least these symbols and all production call sites:

- `append_episode_log_entry`
- `append_facts_entry`
- `begin_distill_turn`
- `finish_distill_turn`
- `boost_turn_topics`
- `run_dream_sweep`
- `load_last_assistant_text`
- any existing `on_turn_finalized` or `on_session_end`

Use `git show` on the relevant commits when needed to distinguish planned work from work
already completed. Key commits are listed in the handoff and ledger. Do not rely on stale
line numbers from the plan.

## Questions the report must answer

### 1. Exact current lifecycle

Describe the current successful-turn lifecycle as an ordered call graph. State where each
step occurs, which steps are conditional, which failures are warn-only, and every early
return that can prevent later growth work. Explicitly answer whether episode logging, facts
distillation, topic boost/decay, and dream sweep are independent or coupled today.

### 2. Original T4 requirement: completed vs remaining

Make a requirement matrix for every clause in the original T4 text:

- pause gate / counters / auto-pause decision moved to pure logic
- 24-hour dream interval moved to pure logic
- four hooks converged behind one facade
- episode/facts ordering preserved
- assistant-text truncation preserved
- warn-only behavior preserved
- integration evidence that one finalize still produces facts and episode

For each clause mark `complete`, `partial`, `not started`, or `obsolete`, with concrete
file/symbol evidence and the commit/task that supplied it when known.

### 3. Smallest honest implementation if T4c is done now

Give the smallest source/test change set that would satisfy the still-relevant facade goal
without implementing T12 early. Name files and symbols, but do not write code. State:

- whether the facade belongs in the pure growth crate or the core host adapter, given that
  the operations perform DB, filesystem, LLM, session, and episode IO
- the proposed function boundary and inputs/outputs
- whether it can truly be called once, or whether async ordering / data dependencies force
  multiple phases
- which existing calls would move behind it and which must remain outside
- exact tests needed to prove behavior equivalence and non-duplication
- likely diff size and number of production files/tests touched (give ranges, not invented
  exact counts)

### 4. Interaction with T12

Identify every part T12 will necessarily replace or reshape: dream-to-garden semantics,
candidate selection migration, independent `on_session_end` triggering, state migration,
profile summary, and the supersede boundary rule. For each proposed T4c change, classify it
as `reused unchanged`, `modified again`, or `deleted by T12`.

### 5. Risk comparison

Compare two options:

A. Implement T4c now, then T12 later.
B. Defer the facade and implement it as part of T12.

Score each option `low`, `medium`, or `high` on behavioral regression risk, duplicated work,
review burden, rollback clarity, and delay to G2 value. Explain every score from the actual
call graph. Pay special attention to the active conversation finalization path, early returns,
duplicate decay/boost, duplicate dream runs, and loss of facts or episode writes.

### 6. Recommendation and trigger to revisit

Give one recommendation, not a neutral list. If recommending defer, specify the exact T12
acceptance criteria needed so T4c cannot disappear silently. If recommending implement now,
state the concrete benefit that cannot wait for T12 and why existing helpers do not already
provide it.

## Evidence rules

- Every mechanism claim must cite a file and line or a commit.
- If a conclusion cannot be verified, label it `Needs confirmation`; do not guess.
- Separate facts from judgment.
- Do not run broad tests. This is read-only analysis; `git status`, `git log`, `git show`,
  searches, and file reads are sufficient.
- Use plain Chinese suitable for a non-programmer, with technical evidence in parentheses.

## Report structure

1. Executive conclusion
2. Current ordered lifecycle
3. T4 requirement matrix
4. Minimal T4c-now change set
5. T12 overlap matrix
6. Option A/B risk table
7. Recommendation and T12 acceptance criteria
8. Needs confirmation
9. Evidence index
