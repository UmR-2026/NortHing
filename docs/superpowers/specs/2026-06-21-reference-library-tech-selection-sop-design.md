# Tech Selection SOP for External Projects — Design Spec

> **Status:** Draft (post-brainstorming, 2026-06-21)
> **Author:** ZCode session
> **Target skill:** `.agents/skills/reference-library/SKILL.md`
> **Worked example:** [colbymchenry/codegraph](https://github.com/colbymchenry/codegraph) (evaluated 2026-06-21)

## 1. Motivation

The `reference-library` skill (created 2026-06-19) currently covers one
workflow: **copying patterns from the 4 internal mirror domains** (Skill,
Actor, Session, Checker). It does not address a different recurring
question: when a new external project / tool / library comes up, **should
we integrate it, study it, or ignore it?**

This was made concrete on 2026-06-21 when evaluating CodeGraph
(tree-sitter → SQLite → MCP code-intelligence tool, 52k stars, 5 months
old). The session did the evaluation ad-hoc, mixing GitHub API fetches
with judgment calls. Without a shared SOP, the next agent will repeat
the same ad-hoc reasoning — and likely arrive at a different verdict for
similar inputs.

Goal: bake the evaluation heuristic into the existing skill so any
future agent runs the same checklist, applies the same skeptical lenses,
and reaches a decision that survives scrutiny.

## 2. Non-goals

- **Not an ADR template.** Decision-record machinery (option B in
  brainstorming) is deferred to a future iteration. This spec only
  delivers the checklist + skeptical rebuttal appendix.
- **Not multi-project examples.** Only one worked example (CodeGraph)
  ships in this iteration. The appendix structure leaves room for
  future examples but does not pre-populate them.
- **Not a published blog post / public artifact.** The SOP lives only
  in `.agents/skills/reference-library/` (project-local).
- **Not a runtime tool.** No new crate, no binary, no MCP server. The
  artifact is a section in an existing skill file.

## 3. Design

### 3.1 Scope of change

A single insertion into `reference-library/SKILL.md`, plus one new
empty directory:

1. **New trigger keyword row** in the existing `## When to trigger this
   skill` table (line 17-25). One row, no other modifications to that
   table.

2. **New top-level section** `## Tech Selection for External Projects`,
   inserted **after** the existing `## Pair with other skills` section
   (line 138), and **before** the existing `## What this skill does NOT
   cover` section (line 141). The new section has two subsections:

   - **§A — The 7 Decision Gates** (main): a 7-question checklist that
     bails out at any failure. Each gate has explicit "what is a bad
     answer / what is a good answer" examples, and a decision tree
     where applicable. Followed by the CodeGraph worked example table.

   - **§B — Appendix: Red-Flag Triage** (anti-marketing appendix):
     4 skeptical lenses (recency / selection bias / survivorship /
     counterfactual) applied to the project's marketing bullets. The
     CodeGraph worked example rebuts 9 specific claims.

3. **Maintenance log entry** appended to the existing `## Maintenance`
   section (line 147): one line noting the 2026-06-21 addition.

4. **New empty directory** `.agents/skills/reference-library/evaluations/`
   with a `.gitkeep` placeholder. Future evaluations land here, one
   file per external project evaluated. The v2 trigger condition (see
   §5) counts files in this directory.

### 3.2 What does NOT change

- Frontmatter `description:` field (line 3) — preserved verbatim to
  avoid polluting the trigger.
- `## The mandatory 4-step workflow` (line 29) — unchanged.
- `## When to update the mirror` (line 75) — unchanged.
- `## Domain → mirror map` (line 88) — unchanged.
- `## Re-derivation is forbidden` (line 99) — unchanged.
- `## Failure modes` table (line 110) — unchanged (it covers internal
  pattern misuse, orthogonal to external selection).
- `## What this skill does NOT cover` (line 141) — unchanged boundary.
- `.agents/reference/` contents — unchanged.

### 3.3 Section §A — The 7 Decision Gates (full content)

```markdown
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
```

### 3.4 Section §B — Red-Flag Triage appendix (full content)

```markdown
### Appendix — Red-Flag Triage: Reading Past Marketing

> **Use this after Gate 5 (health signals).** Take the project's
> top-N selling points (usually 4-8 bullets from README), and **for
> each one, write the skeptical rebuttal**. This trains the instinct
> that "promotion ≠ reality."

#### Why this matters

Every well-funded project leads with cherry-picked metrics and omits
the friction. The northhing team has already paid this tax several
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

#### Worked example — CodeGraph's 8 marketing bullets, rebutted

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
```

### 3.5 Trigger keyword row (full content)

Appended to the existing `## When to trigger this skill` table:

```markdown
| **External tech evaluation** | "should we use", "is X worth integrating", "evaluate this library", "tech selection", "code-graph-style tools", "open-source tool triage" |
```

## 4. Verification criteria

1. **Document completeness** — `SKILL.md` line 3 (frontmatter unchanged),
   line 17-25 table has the new row, line 138 has the new top-level
   section, line 141+ preserves `## What this skill does NOT cover`,
   line 147 `## Maintenance` has the appended log entry. No duplicates,
   no omissions.

2. **Executable by a fresh agent** — any agent reading only this skill
   (no other context) can:
   - Recognize the trigger ("external project evaluation scenario")
   - Walk the 7 gates and write one answer sentence per gate
   - Apply the 4 lenses to the project's marketing bullets
   - Produce a final verdict (integrate / study / ignore) with rationale
   - **Write Gate 7 revisit triggers** even on a "don't integrate" verdict

3. **Worked example is traceable** — every numeric claim in the
   CodeGraph table (52k stars, 246 issues, v1.0.1 on 2026-06-13, 5
   months old) is grounded in the WebFetch calls performed during this
   session (2026-06-21), not fabricated.

4. **No regression to existing behavior** — `git diff` of SKILL.md
   shows pure-additive insertion; no existing line is modified except
   the maintenance log append.

5. **Self-check script** — `scripts/check-skill-trigger.sh` runs 6
   assertions:
   - Frontmatter `name: reference-library` present
   - Trigger table contains "External tech evaluation"
   - Section heading `## Tech Selection for External Projects` exists
   - Worked-example table contains the substring "CodeGraph"
   - All 7 Gate headings present (`Gate 1` through `Gate 7`)
   - Worked-example table has 7 verdict rows (not 5)

   Outputs PASS/FAIL exit code. Catches future accidental deletion or
   gate-count regression.

## 5. Out of scope (explicitly deferred)

- **ADR template + multi-example expansion** (option B in
  brainstorming). Captured as a future-spec stub.
- **Trigger condition for v2**: when
  `.agents/skills/reference-library/evaluations/` contains ≥3
  independent evaluation records (each record = one external project
  walked through all 7 gates + the §B rebuttal table + a Gate 7
  revisit-trigger note). Counter is observable via
  `ls .agents/skills/reference-library/evaluations/ | wc -l` — no
  subjective threshold.
- **Cross-skill references** (e.g. wiring `reference-library` to
  `brainstorming` or `writing-plans` via a shared "external project"
  concept). Stay self-contained for now.
- **Blog post / public artifact.** Spec only; no publishing step.

## 6. Risks

| Risk | Mitigation |
|---|---|
| Future modification of `SKILL.md` accidentally deletes §A or §B | `scripts/check-skill-trigger.sh` self-check (verification criterion 5) |
| The CodeGraph worked example becomes outdated as the project evolves | Add a "last verified" date in the table; re-verify on any future encounter. Not auto-updated. |
| Agent reads §A as a "form to fill out" rather than a heuristic to reason with | Each gate has a "bad answer / good answer" example showing the difference between checklist-theater and real evaluation. |
| §B red-flag rebuttal bias toward rejection | §B is positioned as one lens among several, not a veto. Gate 3 (workflow fit) is the ultimate verdict gate. |
| Gate 7 revisit triggers get written but ignored | Self-check script (verification 5) includes "verdict row count = 7" so accidental omission of Gate 7 is caught. The triggers themselves are not auto-checked (too domain-specific); rely on the agent's writing discipline. |

## 7. Rollout

Single commit on `v3-restructure`:

```
docs(skill): reference-library — add external-project tech-selection SOP

Insert §A (7 Decision Gates — direction + revisit-trigger aware) + §B
(Red-Flag Triage appendix) with CodeGraph (evaluated 2026-06-21) as
the worked example.

No existing skill behavior changed. Trigger keyword table gets one
appended row. Maintenance log gets one appended line.

Self-check: scripts/check-skill-trigger.sh (6 assertions, PASS/FAIL)
```

## 8. Open questions

None. Brainstorming resolved all scope questions. Spec is ready for
user review.