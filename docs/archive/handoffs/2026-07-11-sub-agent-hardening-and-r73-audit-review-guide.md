# QClaw Review Guide — B-2/B-3/B-4 sub-agent hardening + R73+ god file audit (combined, with Kimi 8-dim folded in)

> **Author**: Mavis M3 (Mavis take-over)
> **Date**: 2026-07-11
> **Reviewer**: QClaw (sole reviewer; **Kimi the reviewer is unavailable these days** per user, so Kimi's 8-dim secondary review is folded into this single QClaw pass — see "Kimi 8-dim secondary review" section below)
> **Context**: user is **here the whole time** (correcting the prior Mavis misread that put user on break 2026-07-14). User asked for combined QClaw review with Kimi's role folded in.
> **Convention**: per HANDOFF §0 / project review-fix-cleanup cycle, this guide is **read by QClaw** to produce `*-review-report.md` (committed). QClaw does **not** edit this guide; if scope is wrong, file a comment.

## Scope (read this first)

Two batches in one review pass:

### Batch 1: B-2/B-3/B-4 sub-agent hardening (Mavis M3 take-over)

5 implementation commits + 1 HANDOFF bump + 1 B-2 follow-up commit + 1 HANDOFF bump:

| Commit | Phase | One-line description |
|---|---|---|
| `03736e11` | **B-3** | `refactor(persistence):` per-turn transcript checksum (SHA-256 sidecar + parent link audit), 8 unit tests in `services-core/src/session/checksum.rs` |
| `ef6fd440` | **B-4** | `test(test-support):` OfflineSubAgentProfile + FixtureLoader + 3 JSON fixtures, 13 tests pass (8 unit + 5 integration) |
| `43d94edd` | **B-2 init** | `feat(coordination):` SubAgentHandoff trait + TurnHandoffCounter + CoordinatorHiddenSubagentHandoff impl + HandoffError, 10 unit tests; `execute_hidden_subagent_internal` marked `#[deprecated]` |
| `271a52d9` | **B-2 follow-up** | `refactor(coordination):` migrate so_dispatch::execute_subagent + start_background_subagent to SubAgentHandoff (closes the 2 audit-flagged deprecation warnings) |
| `9fadddc6` | HANDOFF | `docs(handoff):` bump N→N+M (B-2/B-3/B-4 DONE) |
| `79f261be` | HANDOFF | `docs(handoff):` bump N→N+M (B-2 follow-up DONE) |

### Batch 2: R73+ god file candidates audit (Mavis M3 manual)

| Commit | Description |
|---|---|
| `a4fd02c8` | `docs(handoff):` bump N→N+M (R73 audit DONE) |

The audit deliverable itself is **untracked** (per HANDOFF §7.5 D convention):
- `docs/audit/2026-07-11-r73-god-file-candidates.md` — QClaw must read this from working tree (it is not in `git ls-files`; `git show HEAD:docs/audit/...` will fail; use `cat` or your editor's file open)

## Context (background for the review)

**Mavis M3 take-over rationale**: the `mavis team plan run` floor-validator rejected all dispatched plans with opaque "Invalid plan" error after 8+ retry paths. User authorized Mavis M3 to take over rather than burn Mavis quota on `step-router-v1` failures. See HANDOFF §0 + plan-engine issue note in commit `71a6ae0b`.

**Why combined review**: user is on break until 2026-07-14; Kimi unavailable; user asked QClaw to cover both batches in one pass. The batches are independent (Batch 1 is implementation; Batch 2 is audit + R73 plan) but the R73 plan depends on Batch 1's god-file landscape being stable.

## What to review (per commit)

### B-3: per-turn transcript checksum (commit `03736e11`)

**What it does**: adds SHA-256 sidecar (`turn-XXXX.checksum`) next to each persisted `turn-XXXX.json` in `services-core/src/session/turns/<session_id>/`. The hash covers `turn_id` / `turn_index` / `session_id` / `timestamp` / `start_time` / `end_time` / `duration_ms` / `user_message` / `model_rounds` / `kind` / `status`. On read, verify the sidecar; on mismatch, return `NortHingError::Validation`. Pre-checksum turns (no sidecar) accepted as back-compat (debug log only). Also adds `audit_turn_parent_links` that detects parent-turn gaps.

**Files**:
- NEW `src/crates/services/services-core/src/session/checksum.rs` (~280 lines + 8 unit tests)
- M `src/crates/services/services-core/src/session/mod.rs` (register + pub use)
- M `src/crates/services/services-core/src/session/layout.rs` (add `turn_checksum_path` method)
- M `src/crates/assembly/core/src/agentic/persistence/turn_io.rs` (write sidecar after atomic JSON write; verify on read; pre-checksum turns accepted)
- M `src/crates/assembly/core/src/agentic/persistence/turn_metadata_sync.rs` (audit parent links; gap → return None → caller falls back to full directory scan)

**Why services-core (not core)**: per `services-core/AGENTS.md` placement rule, helper lives next to `turn_path` (sibling of `SessionStorageLayout`).

**Reviewer focus**:
- Is the hash coverage correct? Does it include all semantic content? Does it exclude any volatile field (e.g. transient cache pointers)?
- Is the pre-checksum back-compat path correct? Is the debug log enough or should it be `warn!`?
- Is the parent-link audit correct? Does `None` from `audit_turn_parent_links` correctly trigger the caller's fallback path in `turn_metadata_sync.rs`?
- Are the 8 unit tests sufficient? (deterministic / round-trip / mismatch / OOR / missing sidecar / audit gaps / hex round-trip)

### B-4: OfflineSubAgentProfile (commit `ef6fd440`)

**What it does**: hermetic LLM-independent test infrastructure. The `OfflineSubAgentProfile` is a **data-only** struct (does NOT implement `LongRunningSkill` — to avoid `test-support` → `northhing-agent-dispatch` reverse dependency). Caller wraps the profile into `LongRunningSkill` as needed. `FixtureLoader` reads JSON fixtures from `tests/fixtures/llm/`. 3 sample fixtures (single-round / multi-round-with-tools / long-running-default). 5 integration tests + 8 unit tests.

**Files**:
- NEW `src/crates/test-support/src/offline_profile.rs` (~300 lines + 5 unit tests)
- NEW `src/crates/test-support/src/fixture_loader.rs` (~150 lines + 3 unit tests)
- NEW `src/crates/test-support/tests/fixtures/llm/{echo_single_round,multi_round_with_tools,long_running_default}.json`
- NEW `src/crates/test-support/tests/offline_subagent_profile.rs` (5 integration tests)
- M `src/crates/test-support/src/lib.rs` + `Cargo.toml` (add serde + serde_json deps, register mods, pub use)

**Why data-only (no `LongRunningSkill` impl)**: keeps `test-support` free of `northhing-agent-dispatch` deps. The integration test in `tests/offline_subagent_profile.rs` demonstrates how a caller would wrap the profile — downstream tests in `agent-runtime` / `core` do the actual wrapping.

**Reviewer focus**:
- Is the data-only decision the right call? Would implementing `LongRunningSkill` directly in `OfflineSubAgentProfile` be cleaner? (Mavis's call: NO — the layer boundary is more important than the convenience)
- Is the builder API ergonomic? (`with_round` / `with_final_round`)
- Is `OfflineTickError` comprehensive? (RoundOutOfRange / EmptyProfile / PrematureFinal)
- Are the 3 sample fixtures realistic? Will the long-running-default (6 rounds) exercise the K.2.3 `LongRunningSkill` protocol correctly when wrapped?
- Is hot-reload (no cache, re-read on every call) the right behavior? Or should there be a cache?

### B-2 init: SubAgentHandoff trait (commit `43d94edd`)

**What it does**: introduces the explicit handoff contract for sub-agent invocation. The `SubAgentHandoff` trait is `pub(crate)` (not `pub`) because the canonical `Input`/`Output` types (`HiddenSubagentExecutionRequest` / `SubagentResult`) are both `pub(crate)`. `TurnHandoffCounter` enforces one handoff per turn via `Arc<Mutex<HashMap<String, u8>>>`. `HandoffError` (4 variants) + `From<HandoffError> for NortHingError` impl. `execute_hidden_subagent_internal` marked `#[deprecated(note = "B-2: use SubAgentHandoff::handoff (e.g. CoordinatorHiddenSubagentHandoff) instead; this fn remains only for the A1 fallback path (target removal post-0.1.0)")`.

**Files**:
- NEW `src/crates/assembly/core/src/agentic/coordination/handoff.rs` (~480 lines + 10 unit tests)
- M `src/crates/assembly/core/src/agentic/coordination/mod.rs` (register `mod handoff` + `pub(crate) use` the 4 public items)
- M `src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_lifecycle/mod.rs` (`#[deprecated]` on `execute_hidden_subagent_internal` + doc comment)

**Trait visibility decision**: `pub(crate)` (NOT `pub`) — lifted to `pub` is a follow-up after the input/output types are public-stabilized. The 6-axis rubric should NOT flag this as a visibility issue (it's an intentional intermediate state).

**Why `#[async_trait]` (not native `async fn` in trait)**: matches the K.2.3 `LongRunningSkill` pattern in the same layer; `#[async_trait]` makes the trait object-safe for `Box<dyn SubAgentHandoff>` (needed for the background path).

**Reviewer focus**:
- Is the trait shape correct? Should it be `async-trait` or native `async fn` in trait? (Rust 1.75+ supports native; Mavis chose `#[async_trait]` for object-safety)
- Is `Send + 'static` the right bound for `Input`/`Output`? Does it accidentally exclude legitimate types?
- Is the per-turn counter thread-safe? Is the lock contention negligible? (handoff is rare)
- Is the canonical `CoordinatorHiddenSubagentHandoff::handoff` impl correct? It clones the global coordinator Arc per call (matches the `a1_path` pattern; `&'static` borrow is not stable through `OnceLock::get().cloned()`).
- Is the `#[deprecated]` note accurate? Is "target removal post-0.1.0" a reasonable target?
- Are the 10 unit tests sufficient?

### B-2 follow-up: so_dispatch migration (commit `271a52d9`)

**What it does**: closes the 2 audit-flagged deprecation warnings at `so_dispatch.rs:130` (execute_subagent) and `so_dispatch.rs:176` (start_background_subagent). The 2 production callers now route through `CoordinatorHiddenSubagentHandoff::handoff` with the per-turn counter keyed on the parent's `dialog_turn_id` (with `orphan-<session_name>` fallback when no parent info is attached).

**Files**:
- M `src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_dispatch.rs` (2 callers + new `subagent_turn_id` helper at file bottom)
- M `src/crates/assembly/core/src/agentic/coordination/handoff.rs` (5 accessor methods marked `#[allow(dead_code)]` with comments)

**Behavior change**: the 3 caller params (`cancel_token` / `timeout_seconds` / `actor_runtime`) are now underscore-prefixed. The canonical handoff impl hardcodes `None` for all 3 (R73+ handoff enhancement should plumb them through). Mavis considered this an acceptable trade-off; doc comments record the intent for the follow-up.

**`parent_cancel_token`**: still derived above the `tokio::spawn` (preserved for follow-up), underscore-prefixed because the spawn future doesn't currently consume it.

**Reviewer focus**:
- Is the `subagent_turn_id` helper correct? Does the `orphan-<session_name>` fallback collide with legitimate turn_ids? (Collision risk: low; turn_ids are UUIDs in practice)
- Is the `tokio::spawn` future ownership correct? The handoff is moved in; the counter is Arc-cloned; the request is moved.
- Is the behavior change (3 params no longer plumbed) acceptable? Should the handoff trait be extended to accept these as options?
- Are the 5 `#[allow(dead_code)]` annotations appropriate, or should the methods be `#[cfg(test)]`-only?
- 0 deprecation warnings confirmed (verified by Mavis; please re-verify).

### HANDOFF bumps (3 commits)

Pure documentation. Per HANDOFF §0 "HEAD drift note" the drift is expected; each bump documents the new HEAD. The review guide author trusts these to be mechanical (no architectural decisions; just metadata sync).

### Batch 2: R73+ god file candidates audit (commit `a4fd02c8` + untracked doc)

**What it does**: documents the current god-file landscape (post-R67/R72):
- **0 god files ≥750 lines** (R67/R72 closed Phase B debt)
- 121 files in 500-749 line "rising" tier
- 5 R73+ candidates picked (with natural seams + estimated split cost)
- Recommended R73 order (R73-1 path_manager → R73-2 turn_batch → R73-3 github → R73-4 git_tool → R73-5 remote_connect)

**Untracked doc**: `docs/audit/2026-07-11-r73-god-file-candidates.md` — QClaw **must** read this from working tree (it is not in `git ls-files`; per HANDOFF §7.5 D convention, audit docs are untracked).

**This is an audit, not an implementation** — QClaw's review is mostly about whether the methodology + the 5-pick + the recommended order are sound. The actual splits are R73+ work (deferred).

**Reviewer focus**:
- Is the threshold (500 lines) appropriate? Or should it be 400 / 600?
- Are the 5 picks the right ones? Or did Mavis miss an obvious candidate?
- Is the recommended order (R73-1 path_manager first) the right starting point? (Mavis's reasoning: simplest split, sets the pattern, critical-path file)
- Is the "out-of-scope" list (visibility violations / duplicate definitions / test file rotation / per-feature compile validation) reasonable? Should any be folded in?

## 6-axis scoring rubric (QClaw primary)

Each commit (or each batch for the HANDOFF bumps) gets scored on these 6 axes. Final score is the unweighted average. Per project convention, **APPROVED ≥ 9.0/10**; **CHANGES_REQUESTED 7.0-8.9**; **REJECTED < 7.0**.

| Axis | What to check | Weight |
|---|---|---|
| 1. **Correctness** | Does the code do what the commit message says? Edge cases handled? Back-compat paths safe? | equal |
| 2. **Testability** | Are unit tests + integration tests sufficient? Do they cover happy path + error path + edge cases? | equal |
| 3. **Safety** | Are there data loss / corruption risks? Concurrency hazards? Security implications? | equal |
| 4. **Maintainability** | Is the code easy to read? Are naming + module structure consistent? Are doc comments present? | equal |
| 5. **Layering** | Does it respect `AGENTS.md` placement rules? (services-core does not depend on core; assembly is the compatibility facade; etc.) | equal |
| 6. **Doc quality** | Are HANDOFF updates accurate? Are audit docs untracked per §7.5 D? Are review guides themselves committed? | equal |

## Kimi 8-dim secondary review (folded into this QClaw pass)

Per project convention, Kimi normally provides a secondary 8-dimension review. Kimi the reviewer is unavailable these days, so **QClaw does both the 6-axis primary AND the 8-dim secondary in this single pass**. Reference: `E:\agent-project\review-summary.md` (Kimi's R44-R59 8-dim review, scored 6.5/10) and `E:\agent-project\dimension-*.md` (8 dimension reports).

| Dim | What to check | Weight |
|---|---|---|
| 1. **Code organization** | Module structure, sub-module boundaries, file naming, public API surface | equal |
| 2. **API surface** | Public types / methods are minimal + clear; type signatures tell the story | equal |
| 3. **Test coverage** | Unit + integration test depth; coverage of edge cases + error paths | equal |
| 4. **Performance** | Hot paths are not accidentally O(n²); allocations in tight loops; lock contention | equal |
| 5. **Error handling** | Errors are typed (not `String`), errors are propagated (not swallowed), `From` impls are correct | equal |
| 6. **Extensibility** | New handoffs / profiles / checksums can be added without modifying the trait / struct definitions | equal |
| 7. **Documentation** | Doc comments on public API; module-level docs explain "why" not just "what"; HANDOFF entries are accurate | equal |
| 8. **Security** | No `unsafe` without justification; no path traversal; no user-controlled data reaching `tokio::spawn` unsanitized | equal |

**Combined verdict logic**: final score = average of 6-axis primary + 8-dim secondary (14 dimensions, unweighted). If any single dimension is < 7.0, escalate to **CHANGES_REQUESTED** regardless of average. Both rubrics must individually clear their threshold for **APPROVED**.

## Pre-review verification commands

QClaw should run these **before** scoring, to establish a baseline:

```bash
cd /e/agent-project/northing

# 1. Confirm HEAD matches expected
git rev-parse --short HEAD
# expect: a4fd02c8

# 2. Confirm 7 untracked docs (5 R-series/spec + 1 B-2 audit + 1 R73 audit)
git status --short
# expect: 7 lines starting with "?? docs/..." (no other untracked, no modified)

# 3. Confirm 0 errors on product-full build
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo check -p northhing-core --lib --features product-full
# expect: 0 errors, 1382 warnings (mostly dead_code pre-existing R-series debt)

# 4. Confirm 0 deprecation warnings at so_dispatch.rs (B-2 follow-up success)
cargo check -p northhing-core --lib --features product-full 2>&1 | Select-String "deprecated" | Where-Object { $_ -match "so_dispatch" }
# expect: 0 matches

# 5. Confirm services-core tests pass (B-3 baseline + 8 new tests)
cargo test -p northhing-services-core --lib
# expect: 52 passed, 0 failed

# 6. Confirm test-support tests pass (B-4 baseline + 13 new tests)
cargo test -p northhing-test-support
# expect: 8 unit + 5 integration, all pass

# 7. Read the untracked R73 audit doc
cat docs/audit/2026-07-11-r73-god-file-candidates.md

# 8. Read the untracked B-2 audit doc
cat docs/audit/2026-07-11-b2-handoff-callers.md

# 9. Verify 156 uncommitted `cargo fmt` changes are NOT touched
git status --porcelain | Select-String "^ M"
# expect: 0 matches (no modified files)
```

## Specific review questions

Please answer these in the `*-review-report.md`:

### Per commit

1. **B-3 (`03736e11`)**: Is the SHA-256 coverage of `compute_turn_checksum` correct? Should the hash also include `kind` / `status` / `token_usage`? (Mavis's call: yes, all of these are included; verify the implementation matches)
2. **B-3**: Is the `audit_turn_parent_links` semantics correct? Does the caller's fallback to full directory scan (when audit returns `None`) preserve correctness in all cases? Or should the caller receive a structured error?
3. **B-4 (`ef6fd440`)**: Is the data-only decision (no `LongRunningSkill` impl) the right call? Or would implementing the trait directly in `OfflineSubAgentProfile` be cleaner? Trade-off: layer boundary vs convenience.
4. **B-4**: Is hot-reload (no cache) the right behavior? Or should the loader cache the JSON in memory?
5. **B-2 init (`43d94edd`)**: Is `pub(crate)` the right visibility for the trait? Or should it be `pub` (with `Input`/`Output` types also `pub`)?
6. **B-2 init**: Is `#[async_trait]` the right choice? Or should we use native `async fn` in trait (Rust 1.75+)?
7. **B-2 follow-up (`271a52d9`)**: Is the behavior change (3 caller params no longer plumbed) acceptable? Or should the handoff trait be extended to accept these?
8. **B-2 follow-up**: Is the `orphan-<session_name>` fallback in `subagent_turn_id` collision-safe? Or should it be a UUID or a sequence number?
9. **HANDOFF bumps (3 commits)**: Per §0 "HEAD drift note", the drift is expected. Are the bump descriptions accurate?

### Batch-level

10. **Are the 5 R73+ picks right**? Or did Mavis miss obvious candidates?
11. **Is the recommended R73 order right**? Or should R73-5 remote_connect (biggest win) be R73-1?
12. **Are the "out-of-scope" items (visibility violations / duplicate definitions / test file rotation / per-feature compile validation) reasonable exclusions**? Or should any be folded into the R73 batch?

### Kimi 8-dim secondary review (folded in, see "Kimi 8-dim secondary review" section above)

13. **Code organization**: Are the new modules (`checksum.rs`, `offline_profile.rs`, `fixture_loader.rs`, `handoff.rs`) well-placed per the placement rules? Could any be sub-divided further?
14. **API surface**: Is the public API of `SubAgentHandoff` / `TurnHandoffCounter` / `OfflineSubAgentProfile` / `FixtureLoader` minimal? Anything that should be `pub(crate)` but is `pub` (or vice versa)?
15. **Test coverage**: Do the 8+13+10 unit tests + 5 integration tests cover the right scenarios? What's missing?
16. **Performance**: Is the `Arc<Mutex<HashMap<String, u8>>>` in `TurnHandoffCounter` the right primitive? Would `DashMap` or a sharded approach be better under high concurrency? Is the SHA-256 hot path optimized?
17. **Error handling**: Are the 4 `HandoffError` variants + the `From<HandoffError> for NortHingError` impl correct? Does `RoundOutOfRange` / `EmptyProfile` / `PrematureFinal` cover all error paths?
18. **Extensibility**: Can a new handoff / profile / checksum be added without modifying the trait? (e.g. a `CancelAwareSubAgentHandoff` trait, or a `MarkdownFixtureLoader`)
19. **Documentation**: Are the doc comments sufficient? Are the module-level docs at `handoff.rs` / `checksum.rs` clear about the "why"?
20. **Security**: No `unsafe` without justification; no path traversal in `FixtureLoader` (the `name` parameter is interpolated into a path); no user-controlled data reaching `tokio::spawn` unsanitized in `start_background_subagent` (the `request` is moved in but originates from a trusted `SubagentExecutionRequest`)

## Known scope decisions (DO NOT flag as issues)

These are intentional choices Mavis made during take-over. Flag them only if you see a regression or correctness issue, **not** as architectural complaints.

- **Mavis M3 take-over (not step-router-v1 worker)**: plan engine broken; user authorized take-over
- **`pub(crate)` SubAgentHandoff trait**: input/output types are also `pub(crate)`; lift to `pub` is a follow-up
- **`#[async_trait]` over native `async fn`**: object-safety for `Box<dyn SubAgentHandoff>`
- **`OfflineSubAgentProfile` data-only (no `LongRunningSkill` impl)**: layer boundary > convenience
- **3 caller params (`cancel_token` / `timeout_seconds` / `actor_runtime`) underscore-prefixed**: canonical handoff does not plumb them through; R73+ handoff enhancement will
- **5 `#[allow(dead_code)]` annotations on `handoff.rs` accessors**: public API for future callers / unit tests
- **No code coverage threshold enforced**: pre-existing R-series debt; out of scope
- **No per-feature compile validation (`B-1` workflow)**: deferred to dedicated follow-up per HANDOFF §7.5 B-1
- **7 untracked docs (5 R-series + 1 B-2 audit + 1 R73 audit)**: per HANDOFF §7.5 D convention; preserved untracked through this review
- **156 uncommitted `cargo fmt` changes NOT touched**: pre-existing R-series noise; do not touch
- **1382 dead_code warnings NOT addressed**: pre-existing R-series debt; out of scope for B-2/B-3/B-4

## Acceptance criteria

The review is **APPROVED** when all of the following hold:

- [ ] Pre-review verification commands all pass (0 errors, 0 deprecation warnings, all tests pass)
- [ ] No blocker findings in any of the 14 dimensions (6-axis + 8-dim)
- [ ] Per-commit review questions answered (Q1-Q9)
- [ ] Batch-level review questions answered (Q10-Q12)
- [ ] Kimi 8-dim secondary review questions answered (Q13-Q20)
- [ ] 6-axis + 8-dim scores recorded per commit (and per batch for the HANDOFF bumps)
- [ ] Final combined score ≥ 9.0/10 (APPROVED) or 7.0-8.9 (CHANGES_REQUESTED) per the rubric
- [ ] If CHANGES_REQUESTED: specific, actionable fix list with file:line references

## Out-of-scope for this review (Mavis self-deferred, do not drag in)

- Plan engine "Invalid plan" opaque error deep-dive (workaround: Mavis take-over; root cause unknown)
- 1382 dead_code warnings cleanup
- 156 uncommitted `cargo fmt` changes
- Per-feature compile validation (HANDOFF §7.5 B-1 workflow) — needs dedicated follow-up
- Visibility violations audit (cross-crate pub items) — needs dedicated follow-up
- Duplicate definitions audit (similar fn/struct across files) — needs dedicated follow-up
- Test file rotation (the 750-line threshold counts lib + tests; test file growth is a separate concern)
- R73 splits themselves (audit is the deliverable; actual splits are R73+ work)

## Reviewer handoff

QClaw produces **one** `docs/handoffs/2026-07-11-sub-agent-hardening-and-r73-audit-review-report.md` (committed) that covers BOTH the 6-axis primary AND the 8-dim secondary (since Kimi is unavailable). The report should have two clearly-labelled sections: "Section A — 6-axis primary (QClaw)" and "Section B — 8-dim secondary (Kimi, folded in)". Follow the format established in `docs/handoffs/2026-07-11-b-decision-and-feature-gate-review-report.md` (the B decision review report Mavis used as a template). The combined 14-dim verdict uses the logic in the "Combined verdict logic" paragraph above.

After QClaw review:
- Mavis handles `fix(tests):` minor observations as separate commits (per project convention)
- Mavis handles `docs(handoff):` cleanup bump (records the review verdict, updates Review history)
- User is here the whole time to verify and call the next round (corrected from prior Mavis misread that user was on break)

## Cross-references

- HANDOFF.md §0 (current state, HEAD drift note)
- HANDOFF.md §7.5 B-2/B-3/B-4 (workstream entries, all marked ✅ DONE)
- HANDOFF.md §7.5 C (god file audit entry, marked ✅ Audit DONE)
- HANDOFF.md §11 Pointers (B-2 audit + R73 audit doc references)
- `docs/superpowers/specs/2026-07-11-sub-agent-orchestration-hardening.md` (B-2/B-3/B-4 design spec)
- `docs/superpowers/plans/2026-07-11-sub-agent-orchestration-hardening-plan.yaml` (implementation plan; informational — plan engine rejected dispatch)
- `docs/audit/2026-07-11-b2-handoff-callers.md` (untracked, B-2 caller survey)
- `docs/audit/2026-07-11-r73-god-file-candidates.md` (untracked, R73 picks)
- `docs/handoffs/2026-07-11-b-decision-and-feature-gate-review-report.md` (B decision review report — QClaw's reference template)

## Author

Mavis M3, 2026-07-11 22:00 (Asia/Shanghai). Mavis take-over; **user is here the whole time** (corrected at 21:58 per user clarification); Kimi the reviewer is unavailable, so Kimi's 8-dim secondary review is folded into this single QClaw pass. Reviewer: marvis.
