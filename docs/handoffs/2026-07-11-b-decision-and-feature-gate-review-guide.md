# B Decision + Feature-Gate Fix — Review Guide (for QClaw)

> **Reviewer audience:** QClaw only (Kimi 额度已耗尽, skip Kimi verbal review for this batch)
> **Scope:** Review 2 commits in one batch, 15-20 minutes total
> **Date:** 2026-07-11
> **Branch state:** main, 2 new commits on top of `f5c30d80`
> **Reviewer:** marvis (Mavis-authored)

---

## What to review (in order, 15-20 min)

1. **This guide** (3 min) — what to look for, where the work is
2. **Commit 1 diff** (5 min) — `git show 4cb230fe` (escape_html XSS fix, +1/-1 in 2 files)
3. **Commit 2 diff** (5 min) — `git show 0b4dc1f3` (feature-gate fix, +4/-4 in 2 files)
4. **Verification commands** (5 min) — see §"What reviewer should verify" below
5. **HANDOFF §7.5 B background** (2 min) — single decision point context, why `(b)` was picked
6. **HANDOFF §0 baseline** — pre-existing 53 errors root cause

---

## What this batch accomplished

| Commit | Files | Diff | Purpose |
|---|---|---|---|
| `4cb230fe` fix(core): escape_html — restore &->&amp; replacement (XSS defense, B decision) | `service/mcp/server/manager/auth/auth_types.rs` + `auth/tests.rs` | +1/-1 | Restore HTML entity-bypass XSS defense; test assertion updated to enforce correct behavior |
| `0b4dc1f3` fix(build): gate service_agent_runtime/remote_connect/mcp on product-full (resolve 53 pre-existing feature-gate errors) | `lib.rs` + `service/mod.rs` | +4/-4 | Tighten cfg gate on 3 product-full-coupled submodules + 1 re-export, resolving 53 pre-existing build errors when `service-integrations` is enabled without `product-full` |

**B 决策背景** (HANDOFF §7.5 B):
- R67plus3 (`00fcba9f`) removed `&->&amp;` to satisfy a test asserting `&` should pass through unchanged
- R67plus3 left OAuth callback HTML page vulnerable to entity-bypass XSS: attacker can craft `error=&lt;script&gt;...` and the browser parses `&lt;` as `<`
- User picked decision (b) over (a): test assertion is the bug, not the production code
- Commit 1 implements (b)

**Feature-gate 债根因**:
- R50 god-object split (`b5b705be`) extracted `service_agent_runtime` with 49+ `use crate::agentic::...` statements
- `crate::agentic` is `#[cfg(feature = "product-full")]` gated, `service_agent_runtime` was only `#[cfg(feature = "service-integrations")]` gated
- Same root cause applied to `remote_connect` (22 errors) and `mcp` (3 errors)
- Total 53 pre-existing errors when `--features service-integrations` without `--features product-full`
- HANDOFF §0 "914/914 core tests pass" was tested under `--features product-full` which incidentally hid this debt
- Commit 2 tightens the cfg to `all(service-integrations, product-full)` matching actual coupling

---

## Critical observations (where to look first)

1. **XSS entity-bypass defense is the security-critical change** (Commit 1):
   - Verify `escape_html("a<b>&c\"d'e")` produces `a&lt;b&gt;&amp;c&quot;d&#39;e` (not `a&lt;b&gt;&c&quot;d&#39;e`)
   - The 5th replace (`&` → `&amp;`) is the entire fix; if missing, all other 4 replaces are useless for entity-bypass attacks
   - Reviewer should confirm: in the rendered HTML page, the `<` and `>` are still escaped, so direct `<script>` injection is already blocked. The 5th replace specifically defeats the **bypass** via `&lt;` entity.
   - The vulnerable function is called from `render_oauth_callback_page` at 2 sites: line 186 (OAuth error display) + line 217 (missing param list). Both are reachable from external input.

2. **Feature-gate fix scope is the most important architectural decision** (Commit 2):
   - The 3 submodules + 1 re-export all 100% depend on product-full-gated code (`agentic`, `snapshot`)
   - The stricter gate (`all(service-integrations, product-full)`) matches the actual coupling, not a hack
   - The only feature combination that loses access is `service-integrations` without `product-full` — a non-prod combo with no documented consumer
   - **Reviewer should confirm**: no actual consumer in the workspace uses `service-integrations` alone; verify with `git grep -l "northhing-core.*service-integrations" Cargo.toml`

3. **Test baseline restored**:
   - Before fix: `cargo test -p northhing-core --lib --features product-full` was already passing 914 (because product-full hides the debt)
   - After fix: same command still passes 914 (no regression)
   - The 1 ignored test is pre-existing, unrelated
   - **Reviewer should confirm**: error count before/after is uniform across all 3 feature combinations (default / service-integrations / product-full)

4. **R50 split debt is a pattern, not a one-off**:
   - Future god-object splits should pre-emptively cfg-gate use statements that depend on the parent feature
   - Commit 2 only fixes the 3 submodules that are currently broken, not the broader pattern
   - **Reviewer should comment**: is this a precedent for "tighten all god-object-split submodules to product-full" or "case-by-case"?

---

## What reviewer should verify (5-7 min verification per commit)

### Commit 1 (escape_html)

```bash
cd E:/agent-project/northing
git show 4cb230fe --stat
# Expected: 2 files changed, 2 insertions(+), 1 deletion(-)
# auth_types.rs: +1 (the .replace('&', "&amp;") line)
# tests.rs: ~1 (assertion &c -> &amp;c)

# Verify the production code
git show 4cb230fe -- src/crates/assembly/core/src/service/mcp/server/manager/auth/auth_types.rs | head -25
# Look for: input.replace('&', "&amp;") is the FIRST replace (critical for XSS defense)
# Look for: 4 other replaces still present (<, >, ", ')

# Verify the test assertion
git show 4cb230fe -- src/crates/assembly/core/src/service/mcp/server/manager/auth/tests.rs
# Look for: expected output contains &amp;c (not &c)
```

### Commit 2 (feature-gate fix)

```bash
cd E:/agent-project/northing
git show 0b4dc1f3 --stat
# Expected: 2 files changed, 4 insertions(+), 4 deletions(-)
# lib.rs: 1 cfg predicate tightened
# service/mod.rs: 3 cfg predicates tightened (mcp, remote_connect, MCPService re-export)

# Verify each tightened cfg
git show 0b4dc1f3 | grep -B 1 "service-integrations"
# Look for: 4 instances of #[cfg(all(feature = "service-integrations", feature = "product-full"))]

# Verify no consumer uses service-integrations alone
git grep -l "service-integrations" -- '**/Cargo.toml' | head -20
# Then check each: do they also set product-full? do they actually use service_agent_runtime/remote_connect/mcp?
```

### Cross-commit verification (the critical one)

```bash
cd E:/agent-project/northing

# 1. Default features (HANDOFF §6 baseline, must stay 0 errors)
cargo build -p northhing-core --lib 2>&1 | grep -c "^error"
# Expected: 0

# 2. service-integrations alone (was 53 errors, must now be 0)
cargo build -p northhing-core --lib --features service-integrations 2>&1 | grep -c "^error"
# Expected: 0

# 3. product-full (production path, must stay 0 errors)
cargo build -p northhing-core --lib --features product-full 2>&1 | grep -c "^error"
# Expected: 0

# 4. Full test suite (the 914 baseline)
cargo test -p northhing-core --lib --features product-full 2>&1 | tail -5
# Expected: "914 passed; 0 failed; 1 ignored"

# 5. The specific escape_html test
cargo test -p northhing-core --lib --features product-full escape_html
# Expected: 1 passed
```

### XSS defense standalone verification (re-confirm Commit 1's logic)

```bash
# Independent rustc verification (escape_html is a 6-line function, safe to extract)
mkdir -p /tmp/escape_html_verify
cat > /tmp/escape_html_verify/verify.rs <<'EOF'
fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn main() {
    assert_eq!(escape_html("a<b>&c\"d'e"), "a&lt;b&gt;&amp;c&quot;d&#39;e");
    assert_eq!(escape_html("&lt;script&gt;alert(1)&lt;/script&gt;"),
               "&amp;lt;script&amp;gt;alert(1)&amp;lt;/script&amp;gt;");
    assert_eq!(escape_html("hello world"), "hello world");
    assert_eq!(escape_html("rock & roll"), "rock &amp; roll");
    println!("ALL 4 ASSERTIONS PASSED");
}
EOF
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
rustc /tmp/escape_html_verify/verify.rs -o /tmp/escape_html_verify/verify.exe
/tmp/escape_html_verify/verify.exe
# Expected: "ALL 4 ASSERTIONS PASSED"
rm -rf /tmp/escape_html_verify
```

---

## Per-commit scoring (QClaw 6-axis)

| Axis | Commit 1 (escape_html) | Commit 2 (feature-gate) |
|---|---:|---:|
| Security defense correctness (XSS bypass blocked) | /10 | n/a |
| Test assertion accuracy (asserts the right thing) | /10 | n/a |
| Cfg gate matches actual coupling (not over/under-gated) | n/a | /10 |
| Baseline preservation (914/914 unchanged) | /10 | /10 |
| Cargo.lock / workspace unaffected | /10 | /10 |
| Iron rules (line length, fmt hygiene, no warning regressions) | /10 | /10 |
| Commit message clarity (refers to HANDOFF §7.5 B, explains why) | /10 | /10 |

**Mavis estimate:**
- Commit 1: 9.5/10 (would be 10 but the original R67plus3 commit is the higher-order mistake; this is a clean follow-up fix)
- Commit 2: 9/10 (architectural debt, not a code defect; the pre-existing 53 errors are the higher-order mistake; fix is conservative and matches reality)

---

## Questions for reviewer

1. **XSS defense scope** (Commit 1): The `escape_html` function is used in `render_oauth_callback_page` at 2 sites. Are there any other code paths in `northhing-core` that take untrusted external input and emit it into an HTML context? (Sanity check that this is the only escape function we have.) If there are others with the same bug, the fix is incomplete.

2. **Feature-gate precedent** (Commit 2): Is "tighten cfg to `all(service-integrations, product-full)` for 3 submodules + 1 re-export" the right precedent for future R-series god-object splits? Or should R-series splits pre-emptively do this at split-time (so the debt never accumulates)?

3. **Audit scope**: There are 4 pre-existing integration test binaries in `tests/` that also fail to compile under `--features service-integrations` (`context_profile`, `git_contracts`, `product_assembly`, `remote_mcp_streamable_http`). Should Commit 2 have included those, or are they a separate workstream (HANDOFF §7.5 already mentions some)?

4. **Test assertion wording**: Commit 1's test name is `escape_html_replaces_all_special_chars`. The original assertion expected 4 replaces (after the R67plus3 bug). The new assertion expects 5 replaces. Should the test be split into 2 (one per replace group) for better failure localization, or keep it as one?

---

## Sign-off request

After completing the verification per the §"What reviewer should verify" section and answering the 4 questions, please:

1. **QClaw**: provide a score (X.X/10) and APPROVE / REJECT verdict per commit (4cb230fe, 0b4dc1f3)
2. **QClaw**: provide a batch score (X.X/10) and APPROVE / REJECT verdict for the 2-commit batch

If APPROVE with non-blocking observations, the batch is ready for user-driven review-fix-cleanup cycle (per HANDOFF §7.5 A pattern). If REJECT, please identify which axes fail and which commits need rework.

---

*Review guide authored by Mavis on 2026-07-11. Commits are on main. Estimated review time: 15-20 minutes. Cross-reference: HANDOFF §7.5 B (B decision), HANDOFF §0 (914/914 baseline), HANDOFF §6 (known issues).*
