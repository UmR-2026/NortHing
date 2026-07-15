# HomeRail Architecture Analysis — Reference for Northing Sub-Agent Orchestration

> **Investigation target**: `xiaotianfotos/homerail` (GitHub, MIT)
> **Nature**: Voice-first, local multi-agent DAG orchestration runtime. TypeScript monorepo (~366 TS files, 6 core packages).
> **Core design bet**: Human attention is the scarcest resource; systems should occupy as little as possible.
> **Investigation date**: 2026-07-11
> **Author**: UmR (user); cross-reference analysis by Mavis 2026-07-11
> **Local clone**: `C:\Users\UmR\WorkBuddy\Claw\tmp\homerail` (depth 1)

## Why this analysis exists

Northing's user-facing surface is a **single agent**, but the agent internally orchestrates **multiple sub-agents** (per user clarification 2026-07-11). This is the same architectural family as HomeRail's multi-agent DAG — just with a smaller agent count and a simpler DAG shape (sequential or simple parallel, not 6-way parallel dependency graph).

The 10 borrowable patterns + 5 landing recommendations below are evaluated against Northing's actual sub-agent architecture (K.2.x route: `LongRunningSkill` + `spawn_long_running` + `A1StubSkill` + 5-helper `execute_hidden_subagent_internal` split).

---

## 1. Project positioning (HomeRail's own framing)

HomeRail converts "one-shot agent chat" into **auditable, replayable, evaluable graph workflows**:

- **Voice-in**: Voice Surface defaults to Chinese; collects intent across multiple turns before acting (narrow funnel)
- **Generative UI-out**: Agent doesn't dump JSON, generates widget file (TOML) + voice memo; UI layer renders
- **DAG multi-agent**: Behind the scenes is a DAG; each node is an independent-context agent

One-liner: refactor the Claw/Codex "single-thread long conversation" model into "graph-based, layered, verifiable" orchestration.

## 2. Core architecture (HomeRail)

### 2.1 Monorepo structure (6 packages)

```
homerail/
├── homerail_cli/              CLI entry (hr command); parses pipeline.yml / .homerail
├── homerail_manager/          Manager runtime: orchestration, DAG scheduling, handoff routing, audit
│   └── src/orchestration/    dag-engine.ts / graph.ts / handoff.ts
│   └── src/audit/            index.ts (transcript + tool-events + checksum)
│   └── src/eval/             scorecard.ts
├── homerail_protocol/         ★ Protocol layer single source of truth (schema + validation + codec)
│   └── src/schemas.ts         Draft-07 JSON Schema
│   └── src/validation.ts      ajv validation + cross-version tests
├── agent_runtime_primitives/  Manager Agent tool primitives (widget/voice/knowledge tools)
├── agent_runtime_patterns/    DAG pattern library (quorum/ratchet/sparring/trust-ledger…)
└── homerail_node/             Node runtime: Worker container, per-run workspace isolation
```

### 2.2 Runtime topology

Supports 3 workspace types:

- `manager-orchestration`: Manager local, Node in Docker
- `node-worker`: pure Worker, scheduled by remote Manager
- `skill-agent-pair`: skill author runs agent + evaluator pair locally

```
            ┌─────────────┐
  Voice ───▶│   Voice     │  Generative UI ───▶ Human
            │  Surface    │◀─────────────────┘
            └──────┬──────┘
                   │ widget file (TOML) + voice memo
            ┌──────▼──────┐
            │  Manager    │  DAG scheduling / handoff routing / audit
            └──────┬──────┘
        ┌──────────┼──────────┐
   handoff     handoff     handoff   (each node independent context window)
        ▼          ▼          ▼
   ┌────────┐ ┌────────┐ ┌────────┐
   │ Node A │ │ Node B │ │ Node C │  (each with own provider/model; "*" wildcard fallback)
   │ Worker │ │ Worker │ │ Worker │
   └────────┘ └────────┘ └────────┘
        └── per-run workspace: ${HOMERAIL_HOME}/workspace/<run_id>/
```

### 2.3 DAG execution engine (dag-engine.ts)

- **Node state machine**: `PENDING → READY → RUNNING → COMPLETED / FAILED / CANCELLED / SKIPPED`
- **Dependency tracking**: `afterSatisfied` (after_dep sequential) + `inputSatisfied` (explicit edge data dependency) dual sets
- **Mailbox mechanism**: `mailboxes: Map<nodeId, Map<port, unknown[]>>` — each inbound port has independent inbox
- **Failure routing**: `FAILURE_PORTS = {failed, failure, rejected, error}`, edge conditions `always / on_failure`
- **Loop support**: `loopSources: Set<string>` marks loop source nodes
- **Isolation**: each DAG node runs in independent context window; context never inflates into a single giant thread

### 2.4 Protocol layer (homerail_protocol) — single source of truth

- All runtime message contracts centralized in independent package
- `schemas.ts` defines **Draft-07 JSON Schema**, `validation.ts` validates via **ajv**
- Dedicated `cross-version.test.ts` prevents protocol drift (backward-compatible fields, gradual migration)
- Dual-runtime compatibility: `handoff` contains both TS Manager and legacy Python Manager fields

### 2.5 Handoff mechanism (handoff.ts)

- `handoff` tool can only be called **once per turn**; calling immediately ends the current turn
- Call routes current node's output to downstream node; downstream only gets context "on handoff receipt"
- Each node configurable `provider` / `model`; `"*"` wildcard for fallback (expensive models plan/review, cheap models execute)

### 2.6 Audit and replay (audit/index.ts)

- Each run writes `transcript` (jsonl) + `tool-events`
- **Checksum sidecar**: `verifyTranscriptChecksum` validates integrity
- `checkAuditCompleteness` detects missing audit record segments
- `replay` supported (relies on per-run workspace isolation)

### 2.7 Quality assessment (scorecard.ts)

- Results in 3 levels: `hard_error` / `soft_warning` / `blind_spot`
- `intervention` statistics (by_node / by_mode / by_direction)
- `quality_gate`: quantified checks decide if flow may proceed (not subjective adjectives)

### 2.8 Voice Surface / Generative UI (manager-agent-widget-tools.ts)

- Agent generates **widget file (TOML format) + voice memo**; UI layer renders
- Toolset: `update_voice_memo / validate_widget_file / write_widget_file / read_widget_file / remove_widget_file / show_widget_toml_example`
- Voice defaults to Chinese, collects intent across turns before acting

## 3. 10 borrowable patterns (ranked by relevance to Northing sub-agent)

### 1. Protocol layer single source of truth
- **HomeRail design**: all runtime message contracts centralized in independent package; Draft-07 JSON Schema + ajv validation; cross-version tests
- **Northing evaluation (sub-agent view)**: **Already implemented** — `contracts` layer is layer 6, independent of `execution`/`adapters`/`services`. No new work needed.
- **Mapping**: Northing god-object split (R50-R66b) could retrospectively benefit, but current state OK.

### 2. DAG pattern library 4-layer separation (docs/dag-patterns.md)
- **HomeRail design**: `runtime primitives → abstract patterns → skill guidance → concrete instances`; patterns are "control flow invariants", not bound to model/provider/task
- **Northing evaluation**: **Partially applicable** — Northing isn't a 6-agent DAG, but "primitive → pattern → instance" thinking could classify sub-agent orchestration (sequential / parallel / retry / cancel).
- **Mapping**: Could inform sub-agent mode taxonomy in K.2.x follow-up.

### 3. Explicit handoff + per-node independent context
- **HomeRail design**: handoff called once per turn to end it; each node independent context window; each node configurable provider/model
- **Northing evaluation (sub-agent view)**: **Partially implemented + gap** — `LongRunningSkill` provides per-skill context isolation ✅, but the handoff **protocol** (sub-agent input/output contract, "once per turn" enforcement) isn't explicitly formalized.
- **Mapping**: K.2.3 A1 covers context isolation, but the handoff protocol (one call per turn, deterministic termination) is the next-level formalization. This is a **real gap**.

### 4. Auditability (transcript + tool-events + checksum)
- **HomeRail design**: each run writes transcript (jsonl) + tool events, with checksum sidecar + `verifyTranscriptChecksum` + `checkAuditCompleteness`
- **Northing evaluation**: **High value** (the real gap) — Northing transcript relies on git, **no checksum validation**; silent corruption goes undetected. With sub-agent nesting, transcript complexity grows 10x; corruption risk amplifies.
- **Mapping**: Augment transcript persistence layer with checksum sidecar; add `verifyTranscriptChecksum` and `checkAuditCompleteness` helpers.

### 5. Scorecard 3-level quality assessment
- **HomeRail design**: `hard_error` / `soft_warning` / `blind_spot` 3 levels + intervention statistics + `quality_gate`
- **Northing evaluation**: **Medium value** — Northing's refactor "exit on threshold" (Phase B) could borrow this grading. Useful for sub-agent output acceptance.
- **Mapping**: Lower priority; K.2.3 A1 doesn't have explicit quality gate yet but could add later.

### 6. Generative UI / Voice Surface Contract
- **HomeRail design**: agent generates widget file (TOML) + voice memo, not dump JSON; voice defaults Chinese, cross-turn intent collection
- **Northing evaluation**: **N/A for sub-agent** — Northing uses React GUI, not voice/UI widgets. Different product surface.
- **Mapping**: Skip; not applicable.

### 7. Docker isolation + per-run workspace + replay
- **HomeRail design**: each run workspace isolated in `<run_id>`; each node preset Worker container; replay supported
- **Northing evaluation**: **Partially applicable** — Northing has TestTempDir (R67 merge 12→1) but no system-level per-run isolation concept. Sub-agent runs in main process.
- **Mapping**: Lower priority; defer until K.2.x sub-agent evolution needs it.

### 8. Deterministic offline profile (testability)
- **HomeRail design**: two-node template with `offline-deterministic` profile, runs topology checks without real model provider
- **Northing evaluation**: **High value** — Northing has stub (`A1StubSkill`) but no system-level "offline-deterministic" profile. With sub-agent multi-layer + async, **CI must have offline mode** to run regression without external LLM.
- **Mapping**: Build an `OfflineSubAgentProfile` that runs full agent loop with stub at every level; integrate with existing test infrastructure.

### 9. README-as-agent-readable-runbook
- **HomeRail design**: README written as `hr` command execution manual; can hand to coding agent for setup
- **Northing evaluation**: **Low value** — different product domain
- **Mapping**: Skip.

### 10. Dual-runtime compatibility (gradual migration)
- **HomeRail design**: handoff contains both TS Manager and legacy Python Manager fields; compatibility period
- **Northing evaluation**: **N/A** — Northing has one runtime
- **Mapping**: Skip.

## 4. 5 landing recommendations for Northing (sub-agent scope)

If UmR wants to borrow from HomeRail in Northing sub-agent orchestration, **the 5 most valuable** (3 actually applicable, 2 for context):

1. **Handoff protocol explicit-ization** (K.2.3 A1 follow-up)
   - Define sub-agent handoff contract: input/output types, "once per turn" enforcement, termination semantics
   - Borrowed from HomeRail §2.5 (handoff.ts)
   - Real gap; small focused scope

2. **Transcript checksum integrity verification**
   - Add checksum sidecar to sub-agent transcript persistence
   - Implement `verifyTranscriptChecksum` + `checkAuditCompleteness` helpers
   - Borrowed from HomeRail §2.6 (audit/index.ts)
   - High value; defense-in-depth against silent corruption

3. **Sub-agent offline test profile**
   - `OfflineSubAgentProfile`: full agent loop with stub at every level (sub-agent, tool, persistence)
   - Integrate with existing A1StubSkill + northhing-test-support
   - Borrowed from HomeRail §2.8 (offline-deterministic profile)
   - High value; enables CI regression without external LLM

4. **DAG pattern classification (for context)** — K.2.x follow-up only if sub-agent count grows
5. **Per-run workspace isolation (for context)** — defer until K.2.x sub-agent needs replay

These 3 workstreams are scheduled as **P2 backlog** in HANDOFF §7.5 B-2/B-3/B-4. Not hot-now; R73+ picks up.

## 5. Why this doesn't change Northing's near-term priorities

- Northing's 0-god-file state, 914/914 test pass, B decision (escape_html + feature-gate) all confirmed clean 2026-07-11
- The 3 workstreams are **architectural improvements**, not bug fixes
- They don't unblock any current P0/P1 work
- Defer to Phase J / R73+ per the existing R-series backlog model

## 6. Cross-references

- HANDOFF §7.5 B-2/B-3/B-4 — landing recommendations as P2 workstreams
- `docs/superpowers/specs/2026-07-11-sub-agent-orchestration-hardening.md` — design spec (untracked, R-mode)
- `docs/superpowers/plans/2026-07-11-sub-agent-orchestration-hardening-plan.md` — implementation plan (untracked, R-mode)
- HANDOFF §7.5 B-1 — R-series per-feature compile validation workflow (sibling recommendation, R50 retrospective)

---

*Analysis authored by Mavis on 2026-07-11. Cross-references: HANDOFF §0, HANDOFF §7.5, K.2.x route. Northing sub-agent architecture facts: `LongRunningSkill` + `A1StubSkill` + 5-helper `execute_hidden_subagent_internal` split. Local homerail clone: `C:\Users\UmR\WorkBuddy\Claw\tmp\homerail` (depth 1).*
