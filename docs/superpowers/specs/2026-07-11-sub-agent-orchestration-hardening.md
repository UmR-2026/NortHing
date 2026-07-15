# Sub-Agent Orchestration Hardening — Design Spec

> **Spec date**: 2026-07-11
> **Author**: Mavis (draft), UmR (decision)
> **Status**: DRAFT — P2 backlog, R73+ candidate
> **Cross-references**:
>   - `docs/architecture/homerail-architecture-analysis.md` (background analysis)
>   - HANDOFF §7.5 B-2 / B-3 / B-4 (workstream entries)
>   - `docs/superpowers/plans/2026-07-11-sub-agent-orchestration-hardening-plan.md` (impl plan, untracked)

## Context

Northing's user-facing surface is a single agent, but the agent internally orchestrates multiple sub-agents (per UmR clarification 2026-07-11). Current sub-agent architecture (K.2.x route):

- `LongRunningSkill` + `spawn_long_running` — sub-agent runs in isolated context
- `CoordinatorHiddenSubagentSkill` + `A1StubSkill` — sub-agent scheduling
- 5-helper `execute_hidden_subagent_internal` split (`phase1/phase2/phase3` + 2 utility) — sub-agent orchestration primitives

3 workstreams identified for hardening based on HomeRail architecture analysis (2026-07-11):

1. **Handoff protocol explicit-ization** — formalize sub-agent handoff contract
2. **Transcript checksum integrity** — defense-in-depth against silent corruption
3. **Sub-agent offline test profile** — enable CI regression without external LLM

## Goals

- Make sub-agent orchestration **deterministic and verifiable** at the protocol level
- Add **defense-in-depth** audit (checksum sidecar) to transcript persistence
- Make sub-agent test infrastructure **LLM-independent** so CI regression doesn't require provider credentials

## Non-goals

- Not changing `LongRunningSkill` semantics (already correct)
- Not adding new sub-agent primitives (K.2.3 A1 scope complete)
- Not multi-agent DAG (Northing is user-facing single agent + internal sub-agents; not 6-agent DAG)
- Not changing `A1StubSkill` interface (just extend with profile)

## Workstream B-2: Handoff protocol explicit-ization

### Current state

K.2.3 A1 introduces `LongRunningSkill` for sub-agent context isolation. However, the handoff protocol between main agent and sub-agent is implicit:

- "When can a sub-agent be invoked?" (currently: any time within main agent's tool call turn)
- "How does the sub-agent return?" (currently: result embedded in tool result, mixed with other tool outputs)
- "What's the input contract?" (currently: any `Value` accepted)

This implicit contract risks bugs like "main agent invokes sub-agent mid-turn, then continues with stale context" or "sub-agent result overwrites other tool results in same turn".

### Target state

Define explicit handoff protocol:

```rust
/// Handoff contract: sub-agent invocation is a single, terminal action within
/// the invoking turn. After handoff, the turn ends; main agent sees only the
/// handoff result, not interleaved with other tool calls.
pub trait SubAgentHandoff {
    /// Input: structured handoff request (type-safe, not raw Value)
    type Input: Serialize + DeserializeOwned;
    /// Output: structured handoff response
    type Output: Serialize + DeserializeOwned;

    /// Invokes sub-agent in isolated context.
    /// **Per-turn enforcement**: caller must guarantee this is the only
    /// sub-agent call in the current turn.
    fn handoff(&self, input: Self::Input) -> Result<Self::Output>;
}
```

### Design decisions

1. **One call per turn** — enforced via runtime check (turn-local counter) rather than type system. Reasoning: type-level enforcement would require linear types; runtime check is simpler and matches the actual bug class.
2. **Type-safe input/output** — replaces `Value` with concrete types per sub-agent. Implementation: `SubAgentHandoff` trait with associated `Input`/`Output` types; each sub-agent implements its own.
3. **Turn-end semantics** — after `handoff()` returns, main agent turn ends. Implementation: returns `Result<TurnOutcome>` not `Result<Value>`; framework forces turn termination.

### Acceptance criteria

- [ ] `SubAgentHandoff` trait defined in `agent-runtime` crate
- [ ] `LongRunningSkill` migrated to implement `SubAgentHandoff` (back-compat shim for existing call sites)
- [ ] Per-turn enforcement check (runtime, not type-level)
- [ ] Tests: 3 scenarios (single sub-agent call OK, multiple calls in same turn = error, sub-agent exception surfaces cleanly)
- [ ] Documentation: handoff contract spelled out in `agent-runtime/README.md` or similar

### Risk

- **Low**: additive change; back-compat shim for existing call sites
- **Medium**: per-turn enforcement may break existing flows that rely on multiple sub-agent calls per turn; need to audit

## Workstream B-3: Transcript checksum integrity

### Current state

Northing transcripts rely on git for integrity:

- Transcript persistence writes jsonl/append-only log
- Integrity check: `git status` + `git log` (history)
- No application-level checksum; silent corruption (disk error, partial write, malicious edit) goes undetected

With sub-agent nesting, transcript complexity grows 10x:

- Main agent turn → 1 transcript segment
- Each sub-agent invocation → 1+ nested transcript segments
- Tool calls within sub-agent → tool-events stream
- Total: 1 main + N×sub-agents + N×tools

Silent corruption in this nested structure is hard to detect after the fact.

### Target state

Add checksum sidecar to transcript persistence:

```rust
/// Per-segment SHA-256 checksum, written alongside each transcript segment.
/// On read, verify checksum before parsing; on mismatch, fail loudly.
pub struct TranscriptSegment {
    pub segment_id: SegmentId,
    pub parent_segment_id: Option<SegmentId>,
    pub content: serde_json::Value,
    pub checksum: [u8; 32], // SHA-256 of content + parent_segment_id
}

pub fn verify_segment(s: &TranscriptSegment) -> Result<(), TranscriptError> {
    let computed = compute_checksum(&s.content, s.parent_segment_id);
    if computed != s.checksum {
        return Err(TranscriptError::ChecksumMismatch { ... });
    }
    Ok(())
}

pub fn check_completeness(transcript: &Transcript) -> Result<(), TranscriptError> {
    // Walks segment tree, verifies parent links, no gaps, no orphans
}
```

### Design decisions

1. **Per-segment checksum** (not per-file) — matches HomeRail's `transcript + tool-events` per-run approach
2. **SHA-256** (not SHA-1, not blake3) — standard library support, no extra dependency
3. **Lazy verification on read** (not on write) — write performance unaffected; corruption detected on next read
4. **Optional backfill** — older transcripts (pre-checksum) get a backfill pass on first read; results cached

### Acceptance criteria

- [ ] `TranscriptSegment` struct with checksum field
- [ ] `verify_segment` and `check_completeness` helpers
- [ ] Persistence layer writes checksum on segment write
- [ ] Read path: verify on read; on mismatch, fail with descriptive error
- [ ] Backfill pass for old transcripts
- [ ] Tests: 3 scenarios (clean read OK, single-segment corruption detected, segment gap detected)
- [ ] No regression in existing audit tests

### Risk

- **Low**: additive; old transcripts still readable via backfill
- **Medium**: silent corruption in old transcripts may surface after backfill; need migration plan

## Workstream B-4: Sub-agent offline test profile

### Current state

`A1StubSkill` exists for stubbing sub-agent behavior in tests, but it's:

- Scoped to specific sub-agent skill tests
- Doesn't cover **full agent loop** (tool calls, persistence, turn management)
- CI tests requiring real LLM provider still common (slow, flaky, cost)

### Target state

Build `OfflineSubAgentProfile` that stubs at every level:

- Sub-agent: `A1StubSkill` (existing) + variants
- Tool: stub tool catalog (existing partial, extend)
- Persistence: in-memory transcript + checksum (uses B-3)
- LLM: deterministic response fixtures (existing partial, formalize)

Full agent loop runs end-to-end without external dependencies.

### Design decisions

1. **Profile-based, not global** — `OfflineSubAgentProfile` activated per test, not project-wide. CI can run real LLM tests as optional layer.
2. **Deterministic fixtures** — every LLM call has a fixture response; tests assert on exact transcript content
3. **Real transcript** — uses production transcript persistence (with B-3 checksum); no test-only storage
4. **Replay support** (future) — record transcript in real LLM run, replay in offline mode for regression

### Acceptance criteria

- [ ] `OfflineSubAgentProfile` struct in test-support crate
- [ ] Fixture loader (JSON files, hot-reload for dev)
- [ ] Full agent loop integration test (sub-agent invocation, tool call, persistence, transcript)
- [ ] CI: `cargo test` runs without LLM provider credentials
- [ ] Existing tests migrated to offline profile where applicable
- [ ] Documentation: profile usage in `agent-runtime/README.md`

### Risk

- **Low**: additive; existing tests unaffected
- **Medium**: fixture maintenance overhead; new features may need new fixtures

## Implementation order

Per dependency analysis:

1. **B-3 (checksum)** first — B-4 (offline profile) depends on transcript persistence with checksum
2. **B-4 (offline profile)** second — uses B-3 checksum
3. **B-2 (handoff protocol)** independent — can run parallel to B-3 / B-4

Recommended dispatch order:

- Round 1: B-3 (checksum)
- Round 2: B-4 (offline profile) + B-2 (handoff protocol) in parallel

## Acceptance criteria (whole spec)

- [ ] All 3 workstreams complete
- [ ] Existing 914/914 test pass baseline maintained
- [ ] No regression in production sub-agent behavior
- [ ] B-3 checksum coverage ≥ 95% of existing transcript paths
- [ ] B-4 offline profile covers ≥ 80% of existing LLM-dependent tests

## Open questions

1. Backfill policy for B-3: eager (on first read) or lazy (on demand)?
2. B-2 per-turn enforcement: runtime check or static analysis (lint)?
3. B-4 fixture format: JSON files, embedded Rust data, or DSL?

## Reviewer

marvis
