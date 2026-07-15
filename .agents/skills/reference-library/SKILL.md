---
name: reference-library
description: "MUST consult BEFORE writing code in any of these 4 domains: skill loader / skill registry / skill resolver / resolve_for_prompt / SkillActor, lightweight actor / one-shot dispatcher / ToolDispatcher / ActorRuntime / USE_LIGHTWEIGHT_ACTOR, session / multi-session / ConversationCoordinator / start_dialog_turn / create_session / delete_session / list_sessions / get_messages / DialogTriggerSource / DialogSubmissionPolicy, plan-compliance-checker / check_plan / parse_plan / CheckResult / TaskResult. Use when implementing or extending skill resolution, actor patterns, session lifecycle, checker rules, or fixtures. Read .agents/reference/<domain>/SIGNATURES.md and NOTES.md first; copy patterns from .agents/reference/<domain>/0N-*.rs; preserve the project's const-flag + regression-test + commit pattern. Skipping this is a violation — copy from reference before re-deriving."
---

# Reference Library — Code Patterns

> **This skill is the entry point to the 4-domain reference library at
> `.agents/reference/`.** The library mirrors the load-bearing code for
> Skill / Registry, Actor / Dispatcher, Session / Coordinator, and
> Plan Compliance Checker. Use it to avoid re-deriving the same patterns
> from scratch.

## When to trigger this skill

Trigger when the user's request involves any of the following:

| Domain | Trigger keywords |
|---|---|
| **Skill / Registry** | "skill", "skill tool", "skill resolver", "skill registry", "SkillRegistry", "SKILL.md", "skill mode", "use_skill_registry" |
| **Actor / Dispatcher** | "actor", "dispatcher", "SkillActor", "ToolDispatcher", "ActorRuntime", "one-shot subagent", "USE_LIGHTWEIGHT_ACTOR", "USE_ONESHOT_DISPATCHER" |
| **Session / Coordinator** | "session", "coordinator", "ConversationCoordinator", "start_dialog_turn", "create_session", "delete_session", "list_sessions", "get_messages", "DialogTriggerSource", "DialogSubmissionPolicy" |
| **Plan Compliance Checker** | "plan-compliance-checker", "check_plan", "parse_plan", "CheckResult", "TaskResult", "plan.md fixture", "checker rule" |
| **External tech evaluation** | "should we use", "is X worth integrating", "evaluate this library", "tech selection", "code-graph-style tools", "open-source tool triage" |

If the user mentions a const flag in one of these domains
(`USE_SKILL_REGISTRY`, `USE_LIGHTWEIGHT_ACTOR`, `SESSION_TREE_VIEW`,
etc.), this skill also applies.

## The mandatory 4-step workflow

**Skipping any of these steps is a violation.**

### Step 1 — Read the domain's `README.md`

`README.md` gives the high-level shape, the file ordering, and the
selection guide. Start there.

```bash
# Linux / macOS / Git Bash
cat .agents/reference/<domain>/README.md
```

### Step 2 — Read the domain's `SIGNATURES.md`

`SIGNATURES.md` is the one-page function signature card. Find the right
function or trait here before opening the full mirror.

```bash
cat .agents/reference/<domain>/SIGNATURES.md
```

### Step 3 — Read the domain's `NOTES.md` for "do NOT copy" warnings

`NOTES.md` lists the patterns that **look** reusable but are actually
legacy, bug-prone, stub-only, or already known to be wrong. Read this
**before** copying any code from the mirror.

```bash
cat .agents/reference/<domain>/NOTES.md
```

### Step 4 — Open the specific `NN-*.rs` mirror and copy the pattern

The mirrors are full copies of the source files (with a header noting
the source path and commit SHA). Open the file that contains the
function or pattern you need, and copy it into your new code with a
reference comment:

```rust
// Pattern source: .agents/reference/<domain>/0N-xxx.rs
// Original: src/path/to/file.rs:line
// See: .agents/reference/<domain>/NOTES.md for warnings
```

## When to update the mirror

If you change the upstream pattern in `src/`, you must update the
corresponding mirror in `.agents/reference/<domain>/` in the **same
commit** (or a follow-up `docs(reference): re-sync after <sha>`
commit). The mirror header records the SHA it was last synced from:

```rust
// REFERENCE — copied from src/path/to/file.rs
// Last synced: 2813b36 (v3-restructure)
```

## Domain → mirror map

| Domain | Mirror path | Status |
|---|---|---|
| Skill / Registry | `.agents/reference/skills/` | Shipped (A4 done) |
| Actor / Dispatcher | `.agents/reference/actor/` | Designed, not implemented |
| Session / Coordinator | `.agents/reference/session/` | Shipped (A6 done) |
| Plan Compliance Checker | `.agents/reference/checker/` | Shipped (Phases 1–4 done) |
| Upstream references | `.agents/reference/_upstream/` | Optional |

## Re-derivation is forbidden

**Do not re-derive any pattern that already exists in the mirror.** The
mirror exists precisely so future implementations can copy from it. If
you find yourself re-deriving a pattern (e.g. "I'll just write a new
weighted-Jaccard resolver"), the right answer is:

1. Read the mirror.
2. Copy the pattern.
3. If the pattern needs to change, update the mirror in the same commit
   and explain why in the commit message.

## Failure modes

If you do NOT use this skill and re-derive instead, the typical
consequences are:

| Domain | Typical re-derivation bug |
|---|---|
| Skills | "I called `resolve_for_prompt` directly and rendered an empty list when the prompt was whitespace-only" — see `NOTES.md` ⛔ #3 |
| Skills | "I conflated `resolver.rs` (v1) with `resolver_v2.rs` (v2) and built a new boolean gate that broke the ranking" — see `NOTES.md` ⛔ #2 |
| Skills | "I copied the install path but dropped the legacy cleanup list, leaking 14 Superpowers-era skill dirs" — see `NOTES.md` ⛔ #4 |
| Actor | "I extended `ToolDispatcher` to support multi-round loops" — see `NOTES.md` ⛔ #2 |
| Actor | "I used `OnceLock<mpsc::Sender>` for the hot path" — see `NOTES.md` ⛔ #3 |
| Session | "I duplicated `SessionState` instead of extending the canonical enum" — see `NOTES.md` ⛔ #5 |
| Session | "I added a 6th `start_dialog_turn_*` facade" — see `NOTES.md` ⛔ #1 |
| Checker | "I changed the JSON shape without coordinating CI" — see `NOTES.md` ⛔ #7 |
| Checker | "I added a new `CheckResult` variant but only updated one of the 3 places in `report.rs`" — see `NOTES.md` ⛔ #4 |

## Pair with other skills

This skill is **domain-specific**; pair it with the meta-skills:

- `superpowers:test-driven-development` — every pattern copied from
  the mirror should be backed by a regression test.
- `superpowers:writing-plans` — when adding a new check rule, use
  this skill + the writing-plans skill to produce a step-by-step plan.
- `superpowers:requesting-code-review` — when you've copied a pattern,
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

- **General Rust idioms.** Use the official Rust book.
- **tokio async patterns.** See `.agents/reference/_upstream/tokio-actor-pattern.md`.
- **LLM provider abstractions.** See `.agents/reference/_upstream/rig-core-providers.md`.
- **Other domains** (workspace, persistence, IPC, webdriver, etc.).
  The mirror covers 4 domains only.

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
