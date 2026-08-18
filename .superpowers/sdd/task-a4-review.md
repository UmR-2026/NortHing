# Task A4 Review — topics/competition.rs

> Reviewer: judge-m3 (independent judgment pass)
> Materials read:
>   - brief: `E:\agent-project\northing\.superpowers\sdd\task-a4-brief.md`
>   - report: `E:\agent-project\northing\.superpowers\sdd\task-a4-report.md`
>   - diff (text): `.superpowers/sdd/task-a4-diff-text-utf8.patch`
>   - live file: `E:\agent-project\northing\.worktrees\growth-a4\src\agentic\src\topics\competition.rs` (638 lines)
>   - live `topics/mod.rs` (read-only confirmation)

---

## 1. Verdicts

**SPEC: PASS**
**QUALITY: PASS**

**APPROVED WITH NOTES**

The implementation satisfies every test in brief §3, every signature/constant in §2, every rule in §4, and every check in the prompt's constraint list. The 22-test cargo run was clean per the report, and my own static walk-through reproduces the math. Two design choices go slightly beyond what the brief mandates; both are sound and non-blocking, recorded as Minor items below.

---

## 2. Findings

### Critical
None.

### Important
None.

### Minor (informational; non-blocking)

- **M-1. `boosts_to_revive` returns `None` for absent topics (broader than spec).**
  File: `competition.rs:148-152`
  Brief only specifies `None` for "already above the band" (line 84). The implementation also returns `None` when `topic` is not present in the group. This is a sensible safety choice (no member to revive ⇒ no boost count) and is consistent with `suppression_state`'s read-only nature; not exercised by any test. No fix needed; flagging because the brief is silent on this edge.

- **M-2. `suppression_state` silently treats NaN inputs as "not below threshold" (i.e. Active).**
  File: `competition.rs:131-139`, doc comment lines 127-130.
  Not explicitly forbidden by the brief but worth noting: a topic with `share = NaN` is reported Active, never Suppressed. The cleanest reading of the spec: NaN is "not strictly less than threshold" because IEEE 754 makes `NaN < x` false. This is consistent with the no-panic rule but is an implicit design choice, not a stated rule. Could be pinned by an additional test (none currently exists) — flagging only.

- **M-3. Negative-zero (`-0.0`) survives sanitization unchanged.**
  File: `competition.rs:75-78` (sanitize predicate `*w < 0.0` is false for `-0.0`).
  Sanitized list keeps `-0.0` as-is; downstream `sum == 0.0` triggers for `-0.0` because `-0.0 == 0.0`. Result identical to true `0.0` on the share path. No defect; flagging because the sanitize check is intuitively "non-positive" but reads literally as "strictly negative".

---

## 3. Constraints (10-item cross-check)

| # | Constraint                                                                 | Result | Evidence |
|---|----------------------------------------------------------------------------|--------|----------|
| 1 | Only `src/agentic/src/topics/competition.rs` modified                       | PASS | `git diff --stat HEAD~1 HEAD` shows exactly 1 file; `Cargo.toml`, `lib.rs`, `topics/mod.rs`, sibling topic files untouched |
| 2 | Zero new dependencies; no `Cargo.toml` change; fixed-sequence tests         | PASS | `git show HEAD~1..HEAD -- Cargo.toml` empty; only `std::collections::HashSet` used; tests use fixed `[(&str, f64); 10]` arrays (lines 318-329) and a `format!` loop for 200 members |
| 3 | Pure functions: no IO, no clock, no random                                  | PASS | No `std::time`, `SystemTime`, `Instant`, `OsRng`, `rand`, `thread_rng`, `Instant::now`, `Instant::now`, file/net ops |
| 4 | No `retire` / `supersede` / `deactivate` / hard-delete functions           | PASS | `grep -E "fn (retire\|supersede\|deactivate\|delete\|drop\|remove)"` returns 0 matches in the file |
| 5 | Non-test code: no `unwrap`/`expect`; divide-by-zero must be explicit       | PASS | Only `unwrap/expect` site in non-`#[cfg(test)]` region is the test helper `share_of` (line 277, `unwrap_or(0.0)` — fine); test-only `.expect("…")` at line 375. Division-by-zero explicit: `normalize` line 80 (`sum == 0.0` ⇒ equal split), `renormalize` line 245 (same) |
| 6 | `suppression_state`: strict `<` for both thresholds; equality → Active      | PASS | Lines 132-138 use `share < THRESHOLD && raw < THRESHOLD`. Pinned by `suppression_boundary_strict_less_than` (lines 544-567) including the 0.1499999999/0.1999999999 just-below case |
| 7 | `apply_boost`: clamp `0.0..=0.15`; only first match; sum within epsilon    | PASS | `clamp_boost_delta` (lines 222-230); `working.iter().position(...)` returns first match (line 113); post-condition via `renormalize` keeps sum in 1e-9 (asserted per-step in `sum_conservation_over_many_boosts`) |
| 8 | `boosts_to_revive` ≤ 100 iterations                                          | PASS | `for i in 1..=100u32` (line 157); convergence-cap tested by `revive_extreme_group_returns` (200-member group) |
| 9 | Float assertions via epsilon; English-only comments; English test names     | PASS | All f64 comparisons route through `approx(a, b)` (lines 267-269); `assert_eq!` only on `usize::len()`, `String/&str` topic names, and the `PartialEq + Eq`-derived `Suppression`/`HealthIssue` enums. No emoji found (grep `[\xE2-\xEF]` returned 0 hits inside the file). All 20 test names are snake_case English |
| 10| No `cargo fmt`; file < 800 lines; all 20 brief tests present; no group-membership logic | PASS | Indentation is 4-space manual. `wc -l` = 638. Exactly 20 `#[test]` functions cover brief §3 items 1–20 one-for-one (see §5 for mapping). No "which topics compete" logic in the file — only math on a given group |

---

## 4. Three core invariants — independent verification

### 4.1 涨必有跌 (rise has fall)

**Test:** `boost_rise_causes_fall` (lines 287-306)

**Proof shape:** 3 equal-weight members → boost A by 0.15 → assert `a1 > a0`, `b1 < b0`, `c1 < c0`, `sum == 1.0`. Uses **strict** `<`/`>`, not `<=`/`>=`. Sum guard uses the `approx` epsilon helper.

**Independent arithmetic:**

- Initial: `a0 = b0 = c0 = 1/3 ≈ 0.333333`
- Boost adds `clamped = 0.15` to A's share: `pre = [0.483333, 0.333333, 0.333333]`, `sum_pre = 1.15`
- After renormalize: each share becomes `pre[i] / 1.15`
  - `a1 = 0.483333 / 1.15 ≈ 0.420290`
  - `b1 = c1 = 0.333333 / 1.15 ≈ 0.289855`
- Verify: `a1 > a0` (0.4203 > 0.3333) ✓, `b1 < b0` (0.2899 < 0.3333) ✓, `c1 < c0` ✓, `sum ≈ 1.0` ✓

**Structural (not coincidental) reason:** `apply_boost` does `share += clamped` BEFORE renormalize, so the post-sum is `1.0 + clamped > 1.0` whenever `clamped > 0`. Renormalize divides every share by a number strictly greater than 1.0; any non-boosted member therefore has its post-share strictly less than its pre-share. This is true for every member that wasn't boosted, regardless of group size.

**Verdict:** Invariant holds. Test assertions use strict inequality as required.

### 4.2 可复活 (revivable)

**Tests:** `suppressed_member_can_revive` (lines 364-384), `zero_share_can_rise` (lines 386-400), `revive_extreme_group_returns` (lines 627-637).

**Proof shape for the central claim:** After `boosts_to_revive` returns `Some(n)`, the test **actually executes** `apply_boost` exactly `n` times and asserts the resulting share has crossed the threshold. It does not merely trust the return value.

**Independent arithmetic for `suppressed_member_can_revive`:**
- After `normalize([8,1,1])`: shares = `8/10, 1/10, 1/10 = [0.8, 0.1, 0.1]`. B has share 0.1 < 0.15 (suppressed).
- Simulation loop, iteration 1: B share = `0.1 + 0.15 = 0.25`; sum = `0.8 + 0.25 + 0.1 = 1.15`; new B = `0.25/1.15 ≈ 0.2174` (≥ 0.15) ⇒ return `Some(1)`.
- Test loop: applies 1 boost, then asserts `share ≥ 0.15`. Yes. ✓

**Why members cannot die irreversibly:** `apply_boost` only ADDS the clamped delta (or inserts a new member); it never removes entries. A zero-share member, on a boost, becomes `clamped / renormalized_sum`, which is strictly positive because the renormalized_sum is finite (≤ 1.0 + 0.15). Tested by `zero_share_can_rise`.

**Edge case review on the 200-member test:** I independently computed:

- 200 equal-weight members → each share = 0.005, sum = 1.0
- 1st boost to t0: `0.005 + 0.15 = 0.155`; sum = `0.155 + 199·0.005 = 1.15`; new t0 = `0.155/1.15 ≈ 0.1348` (< 0.15)
- 2nd boost to t0: `0.1348 + 0.15 = 0.2848`; sum ≈ `1.15`; new t0 = `0.2848/1.15 ≈ 0.2476` (≥ 0.15) ⇒ `Some(2)`

The function converges at iteration 2 (not 1). Test asserts `is_some()` — true. **Not semantically broken:** the geometric convergence in renormalized space means a buried topic can come back in O(log) boosts even from a 200-member group. This matches the brief's intent ("可复活") rather than contradicting it.

**Verdict:** Invariant holds. The "no irreversibility" sub-clause (zero-share rises) is also proven.

### 4.3 无硬作废 (no hard retire)

**Static evidence:**
- `grep -E "fn (retire|supersede|deactivate|delete|drop|remove)" competition.rs` → 0 matches (the only textual occurrences are negation in comments at lines 23 and 404, and one inline assertion message at line 421 / 433 — no function definitions).
- `apply_boost` body (lines 109-123) never calls `remove`, `pop`, `drain`, `retain`, `swap_remove`, `Vec::clear` on the member vector. The only mutation path is `working[idx].share += clamped` and the renormalize loop, both of which preserve length.

**Test evidence:** `no_member_removed_by_boost` (lines 402-436):
- (a) Equal-weighted 3-group; boost A; assert topic-set is subset of before ⇒ no removals.
- (b) "Big"/"tiny" 1000/0.0001-ratio pair; boost big 5×; assert "tiny" still in group ⇒ even under heavy, repeated squeezing the structure is preserved.

`apply_boost` either finds the topic (preserving order, increasing one share) or appends a new one; renormalize maps every existing share to a smaller number but never drops any. The share value can become very small (~`1/n_total` of a max boost) but is never explicitly zeroed and never disappears from the vector.

**Verdict:** Invariant holds. No hard-retire function or path exists.

---

## 5. Brief §3 test mapping (1-to-20)

| Brief item | Test name | Line | Status |
|------------|-----------|------|--------|
| 1. rise has fall            | `boost_rise_causes_fall`        | 287 | matches (strict `<`/`>`) |
| 2. sum conservation         | `sum_conservation_over_many_boosts` | 308 | matches (10 fixed boosts) |
| 3a. boost cap               | `boost_clamp_and_negative_noop` | 339 | matches |
| 3b. negative no-op          | `boost_clamp_and_negative_noop` | 339 | matches |
| 4. suppressible can revive  | `suppressed_member_can_revive`  | 364 | matches (executes `n` boosts) |
| 5. zero-share rises         | `zero_share_can_rise`           | 386 | matches |
| 6. empty group              | `empty_group_handling`          | 440 | matches |
| 7. single-member share=1.0  | `single_member_group`           | 450 | matches |
| 8. all-zero → equal split   | `all_zero_weights_split_equally`| 462 | matches |
| 9. NaN / negative → 0       | `nan_and_negative_treated_as_zero` | 476 | matches |
| 10. boost new topic inserts | `boost_inserts_new_topic`       | 491 | matches |
| 11. duplicate topic         | `duplicate_topic_boost_and_health` | 504 | matches (boosts only first A: 0.6/1.10; second A: 0.3/1.10) |
| 12. both below → Suppressed | `suppression_both_below`        | 526 | matches |
| 13. raw high → Active       | `suppression_raw_high_stays_active` | 532 | matches |
| 14. share high → Active     | `suppression_share_high_stays_active` | 538 | matches |
| 15. exact-threshold → Active| `suppression_boundary_strict_less_than` | 544 | matches (boundary pinned) |
| 16. drift 0.9               | `health_sum_drift`              | 571 | matches |
| 17. 1.5 / -0.1 → OutOfRange | `health_out_of_range`           | 584 | matches |
| 18. healthy → empty         | `health_healthy_group`          | 602 | matches |
| 19. already above → None    | `revive_already_above_returns_none` | 615 | matches |
| 20. 200-member converges    | `revive_extreme_group_returns`  | 627 | matches |

All 20 brief items are covered; no extras beyond what the brief mandated (the only "extra" is `no_member_removed_by_boost`, which is the static-evidence proxy for the no-hard-retire invariant — directly referenced by brief §1 invariant 3).

---

## 6. Cannot-be-verified-from-diff items

These rely on claims external to the change set:

- **"Only modify competition.rs"** — verified via `git diff --name-only HEAD~1 HEAD` (1 file) and `git diff HEAD~1 HEAD -- Cargo.toml` (empty). Strong.
- **"No new dependencies"** — verified via the diff at HEAD (no `Cargo.toml` change). Cross-cutting new deps in upstream files would not be visible from this commit alone, but the brief says other parallel tasks are in different files; out-of-scope for this review pass.
- **"No `cargo fmt` run"** — verified by inspection of indentation (4-space, consistent) and absence of any fmt-specific cosmetics; cannot prove negative claim about the command not being executed.
- **"Test counts match the report"** — I did not re-run `cargo test`. The report shows 22 passed; static test function count is 21 in this file (the 22nd is `error::tests::error_display_includes_context` from the crate scaffold). The report correctly accounts for that.

---

## 7. Recommendation

**APPROVED WITH NOTES.** No Critical or Important findings. Three Minor items (M-1, M-2, M-3) are record-only; none warrants a re-spin. The implementation is mathematically correct, structurally enforces "rise has fall", has a hard-bounded convergence simulation, and contains no path to hard-delete a topic.
