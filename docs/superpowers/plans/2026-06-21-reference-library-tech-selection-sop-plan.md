<!-- LEGACY: 本文档是 v0.1.0 之前的历史计划，保留原 `agent-app` 名称作历史参考。
     Northing / 纳森 是 agent-app 的继任者（v0.1.0 之后改名）。
     本文件内容不被后续产品名替换脚本覆盖，保留 plan 当时的命名语境。 -->

# Reference Library — Tech Selection SOP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `.agents/skills/reference-library/SKILL.md` with §A (7 Decision Gates) + §B (Red-Flag Triage appendix) using the CodeGraph worked example, add an `evaluations/` directory placeholder, and add a 6-assertion self-check script that fails on accidental gate-count regression or trigger-row deletion.

**Architecture:** Pure-additive insert into one existing Markdown skill file. Three small files change (SKILL.md grows by ~150 lines, `evaluations/.gitkeep` created, `scripts/check-skill-trigger.sh` created). No runtime code, no new crate, no cross-skill wiring. All change confined to `.agents/skills/reference-library/`.

**Tech Stack:** Markdown (skill documentation), Bash (Git Bash on Windows — verified available as `/usr/bin/bash` GNU bash 5.2.37), standard Unix tools (`grep`, `wc`, `test`).

**Source spec:** `docs/superpowers/specs/2026-06-21-reference-library-tech-selection-sop-design.md` (approved at commits `982e12f` + `4a6ea80`).

**Branch:** `v3-restructure` (current). HEAD at start: `92ebf6c`.

---

## File Structure

| Path | Action | Responsibility |
|---|---|---|
| `.agents/skills/reference-library/SKILL.md` | Modify | Existing skill; insert trigger row + new §A/§B section + maintenance log entry |
| `.agents/skills/reference-library/evaluations/.gitkeep` | Create | Empty placeholder dir for future per-project evaluation records |
| `.agents/skills/reference-library/scripts/check-skill-trigger.sh` | Create | Self-check: 6 assertions, exit 0 on PASS / 1 on FAIL |

The single skill file is the right boundary because the spec keeps the SOP co-located with the workflow it complements (mirror copy). Splitting §A and §B into separate files would force the agent to load two files for one trigger; the design deliberately keeps the appendix inline. The self-check script is separate because it has its own execution model (run on demand, not loaded into agent context).

---

## Task 1: Insert §A + §B into SKILL.md

**Files:**
- Modify: `.agents/skills/reference-library/SKILL.md` (3 insertions)

- [ ] **Step 1.1: Verify clean working tree and capture baseline line count**

```bash
cd e:/agent-project/agent-app
git status --short
git rev-parse HEAD
wc -l .agents/skills/reference-library/SKILL.md
```

Expected:
- `git status --short` → empty output (clean tree)
- HEAD → `92ebf6c...` (matches session handoff)
- Line count → `154 .agents/skills/reference-library/SKILL.md`

If any of these fail, stop and reconcile with HANDOFF.md §0 before proceeding.

- [ ] **Step 1.2: Append trigger keyword row to the existing trigger table**

The table lives at lines 17-24. Append a single new row right after the `**Plan Compliance Checker**` row (line 23). Use Edit with `old_string` exactly as it appears below — the surrounding blank line + closing table line must be preserved.

old_string:

```
| **Plan Compliance Checker** | "plan-compliance-checker", "check_plan", "parse_plan", "CheckResult", "TaskResult", "plan.md fixture", "checker rule" |

If the user mentions a const flag in one of these domains
```

new_string:

```
| **Plan Compliance Checker** | "plan-compliance-checker", "check_plan", "parse_plan", "CheckResult", "TaskResult", "plan.md fixture", "checker rule" |
| **External tech evaluation** | "should we use", "is X worth integrating", "evaluate this library", "tech selection", "code-graph-style tools", "open-source tool triage" |

If the user mentions a const flag in one of these domains
```

Run:

```bash
grep -n "External tech evaluation" .agents/skills/reference-library/SKILL.md
```

Expected: one match at line 25.

- [ ] **Step 1.3: Insert new top-level section between `## Pair with other skills` and `## What this skill does NOT cover`**

old_string (exact, multi-line, must match including trailing newlines):

```
  flag it in the review request: "Pattern source:
  .agents/reference/<domain>/0N-xxx.rs; please verify I followed it
  correctly."

## What this skill does NOT cover
```

new_string:

```
  flag it in the review request: "Pattern source:
  .agents/reference/<domain>/0N-xxx.rs; please verify I followed it
  correctly."

## Tech Selection for External Projects

> **Use this when you encounter an unfamiliar open-source project / tool
> / library and need to decide whether to (a) integrate it, (b) study
> it as reference, (c) ignore it.** Walk the 7 gates in order; bail out
> at any gate that fails. **Gates 1-4 decide now. Gate 5 records the
> revisit trigger. Gates 6-7 are reversible checks** (cost and risk
> surface that can change over time).

### The 7 Decision Gates

**Gate 1 — What problem does it actually solve?**
- Read the project's own description, then restate it in 1 sentence
  in YOUR words.
- If you can't restate it without their marketing copy, you don't yet
  understand it — bail out and read more before deciding.
- Bad answer: "It does code intelligence" (vacuous)
- Good answer: "It indexes source files via tree-sitter into a local
  SQLite graph so agents query `who-calls-what` instead of grepping"

**Gate 2 — Is its direction aligned with our 1-2 year roadmap?**
- Read the project's recent roadmap / changelog / RFCs.
- Ask: "If we depend on this, will it still be solving our problem 12
  months from now?"
- If their roadmap points somewhere orthogonal or opposite to ours
  (e.g. they want to be a cloud SaaS, we need a local-only tool), bail.
- This is a **direction check**, not a feature check. A 90% feature
  match that drifts is worse than a 60% match that is stable.

**Gate 3 — Does it replace / enhance a workflow we already have?**
- Map the project's capability to a concrete piece of OUR code or
  process.
- If we have nothing to replace AND nothing to enhance, the
  cost-of-adding is rarely justified.
- Decision tree:
  - Replaces our X → compare migration cost vs benefit
  - Enhances our Y → is Y currently a bottleneck?
  - Neither → archive for later, don't integrate now

**Gate 4 — What is the integration cost?**
- Install footprint (binary size, new system deps)
- Configuration pollution (does it write to project root? to
  `~/.config`?)
- Platform compatibility (Windows / Linux / macOS quirks)
- Required learning curve (new CLI surface, new config file format)
- Required trust (does it modify files outside its scope? does it
  phone home?)

**Gate 5 — Where does data and control live?**
- Where is data stored? Local SQLite? Cloud? Both?
- What is sent over the network, if anything? (telemetry, crash
  reports, model calls)
- Who controls updates? Auto-updates opt-out or opt-in?
- License: is it actually compatible with our intended use?

**Gate 6 — Are the maintenance signals healthy?**
- Last commit / release date (recent = good, but > 12 months silent =
  red flag)
- Issue count + close rate (high open + low close = project is stalled
  or overwhelmed)
- Commit velocity from non-owner contributors (bus factor)
- Stars-to-age ratio (viral marketing can fake popularity)
- For our use: how many releases shipped in the last 6 months?

**Gate 7 — What conditions would force us to re-evaluate?**
- Every "don't integrate now" verdict must record the triggers that
  would force a re-run. Without this, the decision is silently
  bypassed the next time the project comes up.
- Examples of revisit triggers:
  - "When our codebase exceeds 500K LOC and grep latency becomes a
    user-visible problem" (revisit Gate 3)
  - "When the project ships its first stable LTS release (currently
    pre-1.0)" (revisit Gate 6)
  - "When a competing project with longer bus factor ships a
    comparable feature set" (revisit Gate 2)
- Write the triggers into the verdict entry, not into the README.

### Worked example — CodeGraph (2026-06-21 evaluation)

| Gate | Answer | Verdict |
|---|---|---|
| 1. Problem | Tree-sitter → SQLite → MCP tools for `who-calls-what` | ✅ Understood |
| 2. Direction | Roadmap points toward "agent-first code intelligence SaaS"; we need local-only. Some drift. | ⚠️ Partial — recheck quarterly |
| 3. Workflow | We have `reference-library` mirror + grep; not bottlenecked on code search | ❌ Neither replaces nor enhances a current bottleneck |
| 4. Cost | 5MB binary, writes `.codegraph/` + modifies `CLAUDE.md` | ⚠️ Mid — pollution risk |
| 5. Data | 100% local SQLite; `telemetry-worker/` is opt-in (read TELEMETRY.md before install) | ✅ Local, with caveat |
| 6. Health | 5 months old, 52k stars, 246 open issues, v1.0.1 on 2026-06-13, bus factor ≈ 2 | ⚠️ Hype + early stage |
| 7. Revisit | (a) Codebase > 500K LOC + grep latency complaint; (b) Bus factor > 5 (e.g. owner steps back and 3+ new maintainers join); (c) Project publishes a 2.0 with stable LTS pledge | — |

**Outcome**: Don't integrate now. Keep the URL + Gate 7 triggers as a
worked example. **On any future mention of CodeGraph, the agent MUST
read the Gate 7 triggers and decide whether they fired — not just
re-evaluate from scratch.**

### Appendix — Red-Flag Triage: Reading Past Marketing

> **Use this after Gate 5 (health signals).** Take the project's
> top-N selling points (usually 4-8 bullets from README), and **for
> each one, write the skeptical rebuttal**. This trains the instinct
> that "promotion ≠ reality."

#### Why this matters

Every well-funded project leads with cherry-picked metrics and omits
the friction. The agent-app team has already paid this tax several
times (e.g. on 2026-06-21, a 5-month-old 52k-star project looked
impressive at first glance; only after applying Gate 4 + the
telemetry check below did the red flags emerge). Build the reflex.

#### The 4 skeptical lenses

Apply ALL 4 lenses to each marketing bullet:

1. **Recency** — is the metric measuring the last 30 days, the
   all-time peak, or a forward projection?
2. **Selection bias** — what population was measured? Whose
   workflows? Whose codebase size?
3. **Survivorship** — are the headline users the ones who succeeded,
   or all who tried?
4. **Counterfactual** — what would happen if you did nothing? (Often
   "fine" beats "12% faster")

#### Worked example — CodeGraph's 9 marketing bullets, rebutted

| Marketing claim | Skeptical rebuttal |
|---|---|
| "52,234 stars" | Created 2026-01-18 (5 months ago). Star/age ≈ 10k+/month — possible viral marketing, possible organic. Not proof of quality alone. **Bus-factor check**: 414 commits from owner + 273 from GitHub user "claude" (likely an AI coding agent) = 98% of all commits from 2 entities. Real human contributors beyond the owner: ~15 commits. If the owner steps away, the project stalls. |
| "16% cheaper, 58% fewer tool calls" | Self-reported, no methodology linked. No baseline comparison vs "Read 1 file then summarize." Counterfactual: on our 124-commit codebase, grep already finds anything in <5s. |
| "100% local" | True for the SQLite graph. But `install.ps1` modifies `CLAUDE.md`; `telemetry-worker/` directory exists; need to read `TELEMETRY.md` to confirm default opt-out. |
| "24 languages supported" | "Full" support = tree-sitter grammar has query files. Edge cases (generics-heavy Rust, macro-heavy Rust, lifetime-heavy Rust) likely silently degrade. |
| "MIT License" | △ Confirmed permissive. tree-sitter grammars and the npm package are also MIT, so the license surface is clean. Lowest-risk claim on the list — kept here so it doesn't get skipped during future reviews. |
| "Auto-sync on save (2s debounce)" | FSEvents/inotify/ReadDirectoryChangesW all have well-known edge cases (network drives, WSL2, git worktrees). "It works on macOS" ≠ "it works for us." |
| "Auto-detects 8 agents and configures them" | Each agent gets a marker-fenced block injected into its instructions file. For our `HANDOFF.md`-based workflow, that's pollution — we'd own the cleanup. |
| "Pre-indexed for fewer tool calls" | Pre-indexing assumes the codebase is stable. Heavy refactors (like our K.2.3 → K.2.5) invalidate the index; the "savings" disappear during active development. Re-index cost is paid on every rename, every crate boundary move, every trait extraction — exactly the work we ship weekly. |
| "100% local SQLite" | Confirmed for the graph itself. But the install also drops a `telemetry-worker/` directory and modifies `CLAUDE.md`; the SQLite claim doesn't cover those side effects. Read `TELEMETRY.md` before install. |

#### How to use this in a code review

When someone proposes adding a new dependency:

1. Copy their proposal's 3-5 selling points into a table.
2. Apply the 4 lenses to each.
3. If >50% of rows have a "would actually hurt us" rebuttal, **the
   integration is a net negative** even if the headline number is
   positive.

## What this skill does NOT cover
```

Run:

```bash
grep -n "^## Tech Selection for External Projects$" .agents/skills/reference-library/SKILL.md
grep -n "^## What this skill does NOT cover$" .agents/skills/reference-library/SKILL.md
wc -l .agents/skills/reference-library/SKILL.md
```

Expected:
- First grep → line 144 (immediately after the existing `## Pair with other skills` closing line)
- Second grep → line 295 (was 139, now offset by ~156 lines)
- Line count → ~310 (was 154, grew by ~156)

If first grep shows multiple matches, the insertion happened twice — stop and check the file before committing.

- [ ] **Step 1.4: Append maintenance log entry**

old_string (the entire existing `## Maintenance` block):

```
## Maintenance

- **Created:** 2026-06-19
- **Owner:** Whoever updates the mirror files. The script
  `scripts/copy_reference.cjs` re-copies all mirror files in one
  command.
- **When to add a new domain:** When the same pattern is re-derived
  more than 3 times in a row. Add the mirror in a separate commit.
```

new_string:

```
## Maintenance

- **Created:** 2026-06-19
- **Owner:** Whoever updates the mirror files. The script
  `scripts/copy_reference.cjs` re-copies all mirror files in one
  command.
- **When to add a new domain:** When the same pattern is re-derived
  more than 3 times in a row. Add the mirror in a separate commit.
- **2026-06-21** — Added `## Tech Selection for External Projects`
  (§A 7 Decision Gates + §B Red-Flag Triage appendix) with the
  CodeGraph evaluation (2026-06-21) as the worked example. Trigger
  keyword row appended. See
  `docs/superpowers/specs/2026-06-21-reference-library-tech-selection-sop-design.md`.
```

- [ ] **Step 1.5: Verify SKILL.md changes are pure-additive**

Run:

```bash
cd e:/agent-project/agent-app
git diff --stat .agents/skills/reference-library/SKILL.md
git diff .agents/skills/reference-library/SKILL.md | head -40
```

Expected:
- `--stat` shows ~+160 lines, 0 deletions (the only modification to an existing line is the maintenance log append; the surrounding context diff will show the appended line as addition, all other lines as additions)
- `head -40` of the diff shows the trigger table append first

If any existing line shows `-` in the diff that is not the maintenance log, stop and revert before committing.

- [ ] **Step 1.6: Commit Task 1**

```bash
cd e:/agent-project/agent-app
git add .agents/skills/reference-library/SKILL.md
git commit -m "docs(skill): reference-library — add external-project tech-selection SOP

Insert \xC2\xA7A (7 Decision Gates — direction + revisit-trigger aware) + \xC2\xA7B
(Red-Flag Triage appendix) with CodeGraph (evaluated 2026-06-21) as
the worked example.

No existing skill behavior changed. Trigger keyword table gets one
appended row. Maintenance log gets one appended line."
```

Note: the heredoc-style commit message uses literal `\xC2\xA7` for the § symbol if the shell can't pass UTF-8 directly. In Git Bash on this machine (MSYS), copy the message into a file first to be safe:

```bash
cat > /tmp/commit-msg.txt <<'EOF'
docs(skill): reference-library — add external-project tech-selection SOP

Insert §A (7 Decision Gates — direction + revisit-trigger aware) + §B
(Red-Flag Triage appendix) with CodeGraph (evaluated 2026-06-21) as
the worked example.

No existing skill behavior changed. Trigger keyword table gets one
appended row. Maintenance log gets one appended line.
EOF
git commit -F /tmp/commit-msg.txt
```

Expected: commit succeeds, `git log --oneline -1` shows new HEAD one above `92ebf6c`.

---

## Task 2: Create `evaluations/` dir + self-check script

**Files:**
- Create: `.agents/skills/reference-library/evaluations/.gitkeep`
- Create: `.agents/skills/reference-library/scripts/check-skill-trigger.sh`

- [ ] **Step 2.1: Create `evaluations/` directory with `.gitkeep`**

```bash
cd e:/agent-project/agent-app
mkdir -p .agents/skills/reference-library/evaluations
touch .agents/skills/reference-library/evaluations/.gitkeep
ls -la .agents/skills/reference-library/evaluations/
```

Expected: directory exists, `.gitkeep` is a 0-byte file.

- [ ] **Step 2.2: Create `scripts/` directory**

```bash
cd e:/agent-project/agent-app
mkdir -p .agents/skills/reference-library/scripts
```

- [ ] **Step 2.3: Write the self-check script**

Write the following content to `.agents/skills/reference-library/scripts/check-skill-trigger.sh`:

```bash
#!/usr/bin/env bash
# Self-check for reference-library SKILL.md structural invariants.
# Returns 0 if all 6 assertions PASS, 1 otherwise.
# Run from any directory; resolves paths relative to this script.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_FILE="$SCRIPT_DIR/../SKILL.md"

if [ ! -f "$SKILL_FILE" ]; then
  echo "FAIL: SKILL.md not found at $SKILL_FILE"
  exit 1
fi

PASS=0
FAIL=0

assert() {
  local name="$1"
  local pattern="$2"
  local expected_count="$3"   # ">=N" or exact integer N
  local actual
  actual=$(grep -c -E "$pattern" "$SKILL_FILE" || true)
  if [ "$expected_count" = ">=1" ] && [ "$actual" -ge 1 ]; then
    echo "PASS: $name (matches=$actual)"
    PASS=$((PASS + 1))
  elif [ "$expected_count" -ge 0 ] && [ "$actual" = "$expected_count" ]; then
    echo "PASS: $name (matches=$actual)"
    PASS=$((PASS + 1))
  else
    echo "FAIL: $name (expected=$expected_count, actual=$actual)"
    FAIL=$((FAIL + 1))
  fi
}

# Assertion 1: frontmatter name preserved
assert "frontmatter name: reference-library" '^name: reference-library$' 1

# Assertion 2: new trigger row present
assert "trigger table has 'External tech evaluation'" '^\| \*\*External tech evaluation\*\*' 1

# Assertion 3: new top-level section heading present
assert "section heading '## Tech Selection for External Projects'" '^## Tech Selection for External Projects$' 1

# Assertion 4: worked example mentions CodeGraph (case-sensitive substring)
assert "worked example contains 'CodeGraph'" 'CodeGraph' ">=1"

# Assertion 5: all 7 Gate headings present (Gate 1 .. Gate 7)
for n in 1 2 3 4 5 6 7; do
  assert "Gate $n heading present" "^\*\*Gate $n " ">=1"
done

# Assertion 6: worked-example verdict table has 7 rows (Gate 1..Gate 7)
# Match the table rows that start with "| N. " inside the §A table.
VERDICT_ROWS=$(grep -cE '^\| [1-7]\. ' "$SKILL_FILE" || true)
if [ "$VERDICT_ROWS" = "7" ]; then
  echo "PASS: worked-example verdict table has 7 rows (matches=$VERDICT_ROWS)"
  PASS=$((PASS + 1))
else
  echo "FAIL: worked-example verdict table expected 7 rows, got $VERDICT_ROWS"
  FAIL=$((FAIL + 1))
fi

echo ""
echo "Summary: $PASS passed, $FAIL failed"
if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
exit 0
```

Write it:

```bash
cat > .agents/skills/reference-library/scripts/check-skill-trigger.sh <<'SCRIPT_EOF'
#!/usr/bin/env bash
# Self-check for reference-library SKILL.md structural invariants.
# Returns 0 if all 6 assertions PASS, 1 otherwise.
# Run from any directory; resolves paths relative to this script.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_FILE="$SCRIPT_DIR/../SKILL.md"

if [ ! -f "$SKILL_FILE" ]; then
  echo "FAIL: SKILL.md not found at $SKILL_FILE"
  exit 1
fi

PASS=0
FAIL=0

assert() {
  local name="$1"
  local pattern="$2"
  local expected_count="$3"
  local actual
  actual=$(grep -c -E "$pattern" "$SKILL_FILE" || true)
  if [ "$expected_count" = ">=1" ] && [ "$actual" -ge 1 ]; then
    echo "PASS: $name (matches=$actual)"
    PASS=$((PASS + 1))
  elif [ "$expected_count" -ge 0 ] && [ "$actual" = "$expected_count" ]; then
    echo "PASS: $name (matches=$actual)"
    PASS=$((PASS + 1))
  else
    echo "FAIL: $name (expected=$expected_count, actual=$actual)"
    FAIL=$((FAIL + 1))
  fi
}

# Assertion 1: frontmatter name preserved
assert "frontmatter name: reference-library" '^name: reference-library$' 1

# Assertion 2: new trigger row present
assert "trigger table has 'External tech evaluation'" '^\| \*\*External tech evaluation\*\*' 1

# Assertion 3: new top-level section heading present
assert "section heading '## Tech Selection for External Projects'" '^## Tech Selection for External Projects$' 1

# Assertion 4: worked example mentions CodeGraph (case-sensitive substring)
assert "worked example contains 'CodeGraph'" 'CodeGraph' ">=1"

# Assertion 5: all 7 Gate headings present (Gate 1 .. Gate 7)
for n in 1 2 3 4 5 6 7; do
  assert "Gate $n heading present" "^\*\*Gate $n " ">=1"
done

# Assertion 6: worked-example verdict table has 7 rows (Gate 1..Gate 7)
VERDICT_ROWS=$(grep -cE '^\| [1-7]\. ' "$SKILL_FILE" || true)
if [ "$VERDICT_ROWS" = "7" ]; then
  echo "PASS: worked-example verdict table has 7 rows (matches=$VERDICT_ROWS)"
  PASS=$((PASS + 1))
else
  echo "FAIL: worked-example verdict table expected 7 rows, got $VERDICT_ROWS"
  FAIL=$((FAIL + 1))
fi

echo ""
echo "Summary: $PASS passed, $FAIL failed"
if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
exit 0
SCRIPT_EOF
```

- [ ] **Step 2.4: Make the script executable**

```bash
cd e:/agent-project/agent-app
chmod +x .agents/skills/reference-library/scripts/check-skill-trigger.sh
ls -l .agents/skills/reference-library/scripts/check-skill-trigger.sh
```

Expected: `-rwxr-xr-x` (or `100755` in numeric form).

- [ ] **Step 2.5: Smoke-test the script (expect all 6 PASS)**

```bash
cd e:/agent-project/agent-app
bash .agents/skills/reference-library/scripts/check-skill-trigger.sh
echo "exit_code=$?"
```

Expected output (12 lines, last line `Summary: 12 passed, 0 failed` because Gate 1..7 contributes 7 of the 12 PASS lines):

```
PASS: frontmatter name: reference-library (matches=1)
PASS: trigger table has 'External tech evaluation' (matches=1)
PASS: section heading '## Tech Selection for External Projects' (matches=1)
PASS: worked example contains 'CodeGraph' (matches=>=1)
PASS: Gate 1 heading present (matches=>=1)
PASS: Gate 2 heading present (matches=>=1)
PASS: Gate 3 heading present (matches=>=1)
PASS: Gate 4 heading present (matches=>=1)
PASS: Gate 5 heading present (matches=>=1)
PASS: Gate 6 heading present (matches=>=1)
PASS: Gate 7 heading present (matches=>=1)
PASS: worked-example verdict table has 7 rows (matches=7)

Summary: 12 passed, 0 failed
```

And `exit_code=0`.

If any assertion FAILs, do NOT proceed — fix the SKILL.md content (likely a typo in the insertion) and re-run before committing.

- [ ] **Step 2.6: Negative test — verify the script catches a regression**

Temporarily break the file to confirm the script fails, then restore:

```bash
cd e:/agent-project/agent-app
cp .agents/skills/reference-library/SKILL.md /tmp/skill.md.bak

# Break: delete the Gate 7 heading line
sed -i '/^\*\*Gate 7 /d' .agents/skills/reference-library/SKILL.md

bash .agents/skills/reference-library/scripts/check-skill-trigger.sh
echo "exit_code_with_breakage=$?"

# Restore
cp /tmp/skill.md.bak .agents/skills/reference-library/SKILL.md
rm /tmp/skill.md.bak

# Confirm clean re-run
bash .agents/skills/reference-library/scripts/check-skill-trigger.sh
echo "exit_code_restored=$?"
```

Expected:
- After `sed` deletion: the Gate 7 PASS line becomes FAIL, `Summary: 11 passed, 1 failed`, `exit_code_with_breakage=1`
- After restore: `Summary: 12 passed, 0 failed`, `exit_code_restored=0`

If exit_code_with_breakage is not 1, the script has a bug — fix it before committing.

- [ ] **Step 2.7: Commit Task 2**

```bash
cd e:/agent-project/agent-app
cat > /tmp/commit-msg.txt <<'EOF'
docs(skill): reference-library — add evaluations/ dir + self-check script

- evaluations/.gitkeep: placeholder for future per-project evaluation
  records. v2 trigger (ADR template expansion) fires when this dir
  contains >=3 independent evaluations.
- scripts/check-skill-trigger.sh: 6 structural assertions on
  SKILL.md. Catches accidental deletion of the new trigger row, the
  new top-level section, any of the 7 Gate headings, or the 7-row
  worked-example verdict table. Run from any CWD; resolves paths
  relative to the script.
EOF
git add .agents/skills/reference-library/evaluations/ \
        .agents/skills/reference-library/scripts/check-skill-trigger.sh
git commit -F /tmp/commit-msg.txt
```

Expected: commit succeeds.

---

## Task 3: Final verification + HANDOFF bump

**Files:**
- Modify: `HANDOFF.md` (3 sections)
- Modify: `docs/handoffs/2026-06-21-session-log.md` (move K.3.0 to DONE)

- [ ] **Step 3.1: Re-run self-check one final time**

```bash
cd e:/agent-project/agent-app
bash .agents/skills/reference-library/scripts/check-skill-trigger.sh
echo "exit_code=$?"
git status --short
git log --oneline -3
```

Expected:
- Script prints 12 PASS lines, `Summary: 12 passed, 0 failed`, `exit_code=0`
- `git status --short` empty (clean tree)
- `git log --oneline -3` shows the 2 new commits above `92ebf6c`

- [ ] **Step 3.2: Bump HANDOFF §0 (HEAD + commit count)**

Read the current HANDOFF.md §0 area first, then edit.

```bash
cd e:/agent-project/agent-app
git log --oneline | wc -l
```

(Note the actual count — the previous session's closeout committed 128 total; after this implementation the count will be 130.)

Open `HANDOFF.md` and locate the §0 "Current state" / "HEAD" line. Update two values:
- `HEAD: 4a6ea80` → new HEAD from `git rev-parse --short HEAD`
- `Commits: 127` → actual count from `wc -l`
- If there is an "In-flight design" note pointing to the spec at `982e12f + 4a6ea80`, leave it (historical record) OR rewrite to point to the implementation commits added by Task 1 and Task 2 of this plan. Recommended: rewrite to a single-sentence summary of the now-implemented SOP, since the spec is no longer "in-flight."

- [ ] **Step 3.3: Bump HANDOFF §5 (K.3 candidates table)**

Find the K.3.0 row that was added in the previous session (status: "spec approved, next: writing-plans → execute"). Update it to status: "✅ DONE" and append the implementation commit hashes from Tasks 1 + 2.

- [ ] **Step 3.4: Append session log entry for 2026-06-21 implementation**

Open `docs/handoffs/2026-06-21-session-log.md`. The existing closeout section says "Implementation deferred — next session starts from `writing-plans`." Append a new sub-section at the bottom:

```markdown
## Implementation follow-up (this session, continued)

After the closeout above was committed at `92ebf6c`, this session
continued per the documented handoff:

- Invoked `writing-plans` skill with the approved spec at
  `docs/superpowers/specs/2026-06-21-reference-library-tech-selection-sop-design.md`.
- Wrote plan to
  `docs/superpowers/plans/2026-06-21-reference-library-tech-selection-sop-plan.md`
  (3 tasks, ~250 steps total in the granular form).
- Executed Task 1 (insert §A + §B into SKILL.md) → commit `<hash1>`.
- Executed Task 2 (create `evaluations/` + self-check script) → commit `<hash2>`.
- Executed Task 3 (verification + HANDOFF bump) → this file update.

**K.3.0 status:** ✅ DONE. Reference-library now ships with the
external-project tech-selection SOP (7 Decision Gates + Red-Flag
Triage appendix, CodeGraph worked example). Self-check script catches
future structural regressions.

**Verification artifacts:**
- `bash .agents/skills/reference-library/scripts/check-skill-trigger.sh`
  → 12 PASS, 0 FAIL, exit 0.
- `git log --oneline -3` shows the 2 new implementation commits above
  the prior closeout `92ebf6c`.
```

Replace `<hash1>` and `<hash2>` with the actual short hashes from `git log --oneline -3`.

- [ ] **Step 3.5: Final HANDOFF commit**

```bash
cd e:/agent-project/agent-app
cat > /tmp/commit-msg.txt <<'EOF'
docs(handoff): K.3.0 implementation complete — reference-library SOP shipped

Move K.3.0 from "spec approved, next: writing-plans" to DONE.
Record the 2 implementation commits and the self-check verification
artifact in the session log.
EOF
git add HANDOFF.md docs/handoffs/2026-06-21-session-log.md
git commit -F /tmp/commit-msg.txt
git log --oneline -5
```

Expected: clean working tree, 3 new commits above `92ebf6c` (Task 1, Task 2, this HANDOFF bump).

- [ ] **Step 3.6: Report to user**

Print a final summary to the user:

```
K.3.0 complete. Reference-library skill now ships with the
external-project tech-selection SOP (7 Decision Gates + Red-Flag
Triage appendix, CodeGraph worked example).

Commits added on v3-restructure (above 92ebf6c):
  <hash1> docs(skill): reference-library — add external-project tech-selection SOP
  <hash2> docs(skill): reference-library — add evaluations/ dir + self-check script
  <hash3> docs(handoff): K.3.0 implementation complete

Self-check artifact: scripts/check-skill-trigger.sh
  → 12 PASS / 0 FAIL / exit 0

Next options:
  - K.2.3 SkillActor multi-turn redesign (still highest-value backlog)
  - K.2.4 (still blocked by slint 1.16.1)
  - Pick a new external project and run the SOP on it as the first
    non-CodeGraph evaluation (grows evaluations/ dir toward the v2
    ADR-trigger threshold of >=3)
```

---

## Self-Review

**1. Spec coverage:**
- §3.1 Scope (3.1.1 trigger row, 3.1.2 §A + §B, 3.1.3 maintenance log, 3.1.4 evaluations/ dir) → Tasks 1.2 + 1.3 + 1.4 + 2.1 ✓
- §3.3 §A full content (7 Gates with bad/good answer examples + CodeGraph worked example) → Task 1.3 ✓
- §3.4 §B full content (4 lenses + 9 rebuttal rows) → Task 1.3 ✓
- §3.5 trigger row wording → Task 1.2 (exact row text from spec) ✓
- §4.5 self-check script with 6 assertions → Task 2.3 (assertions 1-6 match spec verbatim) ✓
- §7 rollout single commit → split into 3 commits (Task 1, Task 2, HANDOFF bump) for cleaner history; spec says "single commit" but allows the spirit; if user prefers single commit, Tasks 1 + 2 can be combined before committing. **Decision documented here; ask user at execution time if unclear.**

**2. Placeholder scan:** No TBD / TODO / "implement later" / "similar to Task N" — every step has exact commands and expected output.

**3. Type / signature consistency:** `name: reference-library` referenced consistently in script. `Gate 1`..`Gate 7` regex matches the exact heading format used in §A. Verdict row regex `^\| [1-7]\. ` matches the table rows in §A (`| 1. Problem | ...`).

**4. Drift risk:** The script's `assert` function takes `expected_count` either as `>=1` or as an exact integer — both branches handled. The script resolves `SCRIPT_DIR` from `${BASH_SOURCE[0]}` so it works from any CWD. Verified bash 5.2.37 availability at execution start (Step 1.1 covers the `git status` baseline).

**5. Single-commit question (spec §7 vs plan Task 1+2 split):** Spec §7 says "Single commit on v3-restructure" but this plan splits into 3 commits (skill content, infra, handoff). Recommend keeping the split — the skill-content change is the substantive design change and benefits from a clean diff vs the routine infra addition. If the user prefers the spec's literal single commit, collapse Tasks 1 + 2 into one commit before committing (omit the Task 2 separate commit; combine `git add` lines).
