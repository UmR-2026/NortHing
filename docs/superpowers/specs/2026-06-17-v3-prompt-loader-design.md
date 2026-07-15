---

# ⚠️ DEPRECATED— Replaced by `2026-06-17-v3-prompt-loader-design-v2.md`

**This document is preserved for historical reference only.**

It was based on the **incorrect assumption** that northhing has a `northhing-memory` crate with `MemoryKeeperSubscriber`, `call_llm_sync`, `MemoryService`, `rusqlite`, `attohttpc`, and SQLite storage.

**Reality (verified 2026-06-17 against `E:\agent-project\northhing-v3`):**
- `northhing-memory` crate **does not exist** (deleted in v2— v3 re-architecture)
- Memory is **file-based markdown** at `<workspace>/.northhing/memory/` managed by the main agent itself (no background distiller)
- LLM calls use `reqwest` (in `northhing-ai-adapters`), not `attohttpc`
- No `MemoryKeeperSubscriber`, no `EventSubscriber` for memory
- The "73,882 token" claim was unverified; actual cached system prompt is ~46K chars / ~11.5K tokens (before tool manifest)

**See new spec for accurate v3 plan based on real code:**— `docs/superpowers/specs/2026-06-17-v3-prompt-loader-design-v2.md`

---

# v3 Prompt Loader— Converged Design (HISTORICAL)

**Status**: Superseded by v2 design
**Date**: 2026-06-17
**Author**: Brainstorming session (ZCode) with user
**Replaces**: `docs/PROMPT_LOADER_ARCHITECTURE.md` (v3 draft, kept for history)

---

## 1. Problem

northhing's main agent prompt currently bloat to **~40-65K input tokens** even for trivial inputs like "hello world". Root cause breakdown (from `CODE_REVIEW.md`):

| Block | Size | Share |
|---|---|---|
| Skill listing (24 × ~250 chars) | ~12-15K | ~20% |
| Agent listing (8 × ~400 chars) | ~6-8K | ~10% |
| Collapsed tool listing | ~5-10K | ~13% |
| Runtime context | ~3-5K | ~7% |
| User context | ~2-5K | ~5% |
| System prompt (cached) | ~10-20K | ~25% |

A 73,882-token hello world has been observed. The bloat also dilutes model attention on the actual task.

## 2. Goals

| Goal | Metric |
|---|---|
| Reduce initial input tokens for trivial input |— 2K (from 73K) |
| Preserve capability surface | LLM can still reach any skill/agent/memory on demand |
| Keep main agent in full control of decisions | LLM decides what to query, not MemoryAgent |
| Make memory maintenance happen without blocking the user | Dialog turn latency not affected by LLM extraction |
| Safe migration from v2 (`MemoryKeeperSubscriber`) | A/B flag, no memory loss in either path |

Non-goals:
- Reducing output token cost
- Replacing the existing tool registry
- Multi-tenant memory isolation
- Cross-session memory sharing beyond the user scope

## 3. Architecture (v3 Converged, "Plan B")

### 3.1 Layers

```
┌─────────────────────────────────────────────────────────────— LLM Call (main agent)— ┌──────────────────────────────────────────────────────— System: soul.md + agent.md + personality + tools— - read_memory 常驻工具 (— - search_skill / get_skill_detail (— - search_agent / get_agent_detail (— - 现有 24 个工具不— - skill_listing— 极简索引 (v3.1— - agent_listing— 极简索引 (v3.1— - collapsed_tool_listing— 保留— - runtime_context— 保留— - user_context— 保留— └──────────────────────────────────────────────────────— └─────────────────────────────────────────────────────────────— agent 主动— read_memory(id) / search_*(query)— ┌─────────────────────────────────────────────────────────────— Sync Tool Layer— - search_skill(q)— BM25 over skills.db— top-K 极简列表— - get_skill_detail(id)— 全量内容— - search_agent(q)— BM25 over agents.db— top-K 极简列表— - get_agent_detail(id)— 全量内容— - read_memory(id or query)— embedding— memory.db— └─────────────────────────────────────────────────────────────— 写库 (单向)
┌─────────────────────────────────────────────────────────────— MemoryAgent (后台 tokio::spawn task)— - 订阅 ToolCallCompleted / DialogTurnCompleted— - LLM 蒸馏: fact extraction + structure + entity linking— + importance scoring (智能在蒸馏阶— - 写库— memory.db (SQLite + embedding vector)— └─────────────────────────────────────────────────────────────— ```

### 3.2 Component Inventory

| Component | New / Modified | File (target) |
|---|---|---|
| `skills.db` (SQLite + BM25 index) | New | `src/crates/assembly/core/src/service/data/skill_index.rs` |
| `agents.db` (SQLite + BM25 index) | New | `src/crates/assembly/core/src/service/data/agent_index.rs` |
| `MemoryAgent` (background task) | New | `src/crates/assembly/core/src/service/memory_agent/{agent,distiller}.rs` |
| `PartitionedLoader` (soul + agent + personality) | New (last) | `src/crates/assembly/core/src/agentic/prompts/loader/partitioned_loader.rs` |
| `search_skill` / `get_skill_detail` tools | New | `src/crates/assembly/core/src/agentic/tools/implementations/search/skill.rs` |
| `search_agent` / `get_agent_detail` tools | New | `src/crates/assembly/core/src/agentic/tools/implementations/search/agent.rs` |
| `read_memory` tool | New | `src/crates/assembly/core/src/agentic/tools/implementations/search/memory.rs` |
| `MemoryKeeperSubscriber` (v2) | Modified (A/B flag) | `src/crates/assembly/core/src/service/memory_keeper/subscriber.rs` |
| `system.rs` (wiring) | Modified | `src/crates/assembly/core/src/agentic/system.rs` |
| `skill_agent_snapshot.rs` | Modified (P1-1/P1-2 truncation, v3.1 后整体替— | `src/crates/assembly/core/src/agentic/skill_agent_snapshot.rs` |
| `prompt_builder_impl.rs` | Modified (v3.4 分区装载) | `src/crates/assembly/core/src/agentic/agents/prompt_builder/prompt_builder_impl.rs` |

### 3.3 Data Flow

**Turn start** (no prefetch):
1. User sends message
2. Main agent receives system prompt (~2K after v3.4: soul + agent + personality + collapsed tools + 24 tools listed)
3. Main agent decides what to do

**Mid-turn, main agent wants context**:
1. Main agent calls `read_memory(query="用户上次提的X")` or `read_memory(id="mem_abc")`
2. Tool does sync embedding search over `memory.db` (BM25 fallback if embedding disabled)
3. Returns top-K structured memory entries
4. Main agent uses entries to inform response

**Mid-turn, main agent wants a skill**:
1. Main agent calls `search_skill(query="pdf generation")`
2. Tool does sync BM25 search over `skills.db`
3. Returns top-K skill summaries (id + name + 1-line description)
4. If main agent wants the full SKILL.md: `get_skill_detail(id="pdf")`— returns full content

**Background, after turn ends** (or tool call):
1. `ToolCallCompleted` or `DialogTurnCompleted` event fires
2. `MemoryAgent` task receives event via mpsc channel
3. Distiller LLM call: `extract(structure, entity_link, importance)— entries[]`
4. Each entry written to `memory.db` with embedding vector
5. A/B flag controls whether `MemoryKeeperSubscriber` (v2) also runs

**Sync, query path**:
- `read_memory` and `search_*` are all **sync** (no async/await for LLM call). They hit a local SQLite + an embedding model (also local, or cached API call).
- This means main agent latency is dominated by embedding search (~5-50ms), not by network or LLM.

### 3.4 Event Subscriptions

| Event | Subscriber | Action |
|---|---|---|
| `DialogTurnStarted` | (none) | No prefetch (decided out) |
| `UserMessageReceived` | (none) | Main agent handles in its own loop |
| `ToolCallCompleted` | `MemoryAgent` (v3) | Distill tool output— memory.db |
| `DialogTurnCompleted` | `MemoryAgent` (v3) + `MemoryKeeperSubscriber` (v2, if A/B on) | Deep extract— memory.db |

`MemoryAgent` subscribes via `event_router.subscribe_internal("memory_agent", ...)`. Wiring point: `src/crates/assembly/core/src/agentic/system.rs:92` (next to existing v2 registration).

## 4. Detailed Design

### 4.1 `skills.db` / `agents.db` Schema

```sql
-- skills.db
CREATE TABLE skill (
 id TEXT PRIMARY KEY, -- e.g. "pdf"
 name TEXT NOT NULL,
 description TEXT NOT NULL, -- 1-line for BM25 + 极简索引
 full_path TEXT NOT NULL, -- absolute path to SKILL.md
 source TEXT NOT NULL, -- "builtin" | "user" | "project"
 keywords TEXT NOT NULL, -- space-separated for BM25
 indexed_at INTEGER NOT NULL
);
CREATE INDEX skill_keywords_idx ON skill(keywords);
```

```sql
-- agents.db
CREATE TABLE agent (
 id TEXT PRIMARY KEY, -- e.g. "compress"
 name TEXT NOT NULL,
 description TEXT NOT NULL, -- 1-line for BM25 + 极简索引
 full_path TEXT NOT NULL, -- path to agent definition
 mode TEXT NOT NULL, -- "ClassA" | "ClassB" | "Reviewer"
 keywords TEXT NOT NULL,
 indexed_at INTEGER NOT NULL
);
CREATE INDEX agent_keywords_idx ON agent(keywords);
```

**BM25** uses SQLite FTS5 virtual table (built-in, zero external dep). Initial seed: scan `builtin_skills/*/SKILL.md` and `agents/definitions/**/*.rs` at startup. File watcher re-indexes on change (deferred to v3.5, not in v3.1).

### 4.2 `memory.db` Schema (extending existing `northhing-memory`)

```sql
-- memory.db (extends src/crates/memory/src/schema.rs)
CREATE TABLE memory_entry (
 id TEXT PRIMARY KEY, -- "mem_abc123"
 session_id TEXT NOT NULL,
 turn_id TEXT NOT NULL,
 category TEXT NOT NULL, -- "fact" | "preference" | "context" | "skill_use"
 content TEXT NOT NULL, -- structured fact (JSON or plain)
 importance REAL NOT NULL, -- 0.0 - 1.0 from LLM
 embedding BLOB, -- 384-dim f32 vector
 source_event TEXT NOT NULL, -- "ToolCallCompleted" | "DialogTurnCompleted"
 created_at INTEGER NOT NULL,
 last_accessed INTEGER
);
CREATE INDEX memory_session_idx ON memory_entry(session_id);
CREATE INDEX memory_category_idx ON memory_entry(category);
```

Embedding: 384-dim (sentence-transformers/all-MiniLM-L6-v2 equivalent). Model runs locally; first call cold-starts ~1s, subsequent calls ~5-20ms per query (vectorized cosine).

### 4.3 `read_memory` Tool Spec

```rust
ToolSpec {
 name: "read_memory",
 description: "Read structured memory entries by ID or by semantic query. \
 Use when the user references past context, prior decisions, \
 or anything that may be in long-term memory. \
 For broad searches use query; for known IDs use ids.",
 input_schema: json!({
 "type": "object",
 "properties": {
 "ids": {
 "type": "array",
 "items": {"type": "string"},
 "description": "Specific memory entry IDs to retrieve (fast path)"
 },
 "query": {
 "type": "string",
 "description": "Natural language query for semantic search"
 },
 "limit": {"type": "integer", "default": 5, "maximum": 20}
 },
 "anyOf": [
 {"required": ["ids"]},
 {"required": ["query"]}
 ]
 })
}
```

Output: JSON array of `{id, category, content, importance, created_at}`. LLM parses and decides how to use.

### 4.4 `search_skill` / `get_skill_detail` / `search_agent` / `get_agent_detail`

Same shape as `read_memory`:

```rust
// search_skill
{
 "query": "string, natural language",
 "limit": 5 // default, max 10
}
// returns [{id, name, description, source}]

// get_skill_detail
{
 "id": "string"
}
// returns {id, name, full_markdown_content}

// search_agent / get_agent_detail: analogous
```

All four are sync, ~1-10ms each, return 极简索引 (no full content in `search_*`).

### 4.5 `MemoryAgent` Implementation Outline

```rust
// src/crates/assembly/core/src/service/memory_agent/agent.rs

pub struct MemoryAgent {
 event_rx: mpsc::Receiver<AgenticEvent>,
 memory_db: Arc<MemoryDb>,
 llm_client: Arc<dyn LlmClient>,
 config: MemoryAgentConfig,
 debounce: DebounceState,
}

impl MemoryAgent {
 pub async fn run(mut self) {
 while let Some(event) = self.event_rx.recv().await {
 match event {
 AgenticEvent::ToolCallCompleted { tool_call, result, .. } => {
 self.handle_tool_completed(tool_call, result).await;
 }
 AgenticEvent::DialogTurnCompleted { turn_id, .. } => {
 self.handle_turn_completed(turn_id).await;
 }
 _ => {} // ignore others
 }
 }
 }

 async fn handle_tool_completed(&self, tool: ToolCall, result: ToolResult) {
 // 1. Quick filter: skip trivial results (e.g. ls of empty dir)
 if !self.should_extract(&tool, &result) { return; }
 // 2. LLM distill
 let entries = self.distill(DistillRequest {
 source: "ToolCallCompleted",
 content: format_tool_call_for_llm(&tool, &result),
 }).await;
 // 3. Embed + write
 for entry in entries {
 self.memory_db.insert_with_embedding(entry).await;
 }
 }
}
```

Key decisions:
- **Debounce**: per-session, batch within 5-second window to amortize LLM cost.
- **Failure mode**: LLM call fails— log + skip; never panic; never block main agent.
- **No main agent communication**: `MemoryAgent` has no channel back to main agent. It only writes to DB.
- **No query response**: main agent reads `memory.db` directly via `read_memory` tool.

### 4.6 `PartitionedLoader` (v3.4, last)

```rust
pub struct PartitionedLoader;

impl PartitionedLoader {
 /// Returns ~2K token system prompt for the given agent.
 /// Includes: soul.md + agent.md + personality + 极简索引 listings
 /// Excludes: full skill content, full agent content, tool specs (collapsed)
 pub fn load(&self, agent: &dyn Agent) -> SystemPrompt {
 let soul = include_str!("../prompts/soul.md"); // ~300 tok
 let agent_md = self.load_agent_md(agent); // ~600 tok
 let personality = self.load_personality(agent); // ~300 tok
 let skill_index = self.load_skill_index_summary(); // ~200 tok
 let agent_index = self.load_agent_index_summary(); // ~100 tok
 let runtime_ctx = self.load_runtime_context(agent); // ~200 tok
 let user_ctx = self.load_user_context(agent); // ~200 tok
 // total: ~1.9K
 }
}
```

**soul.md, agent.md, personality** are extracted from the existing prompt template mechanism (which already handles persona + language preference + agent memory placeholders). The change is to physically split the inline prompt into separate files for maintainability.

## 5. Token Math (v3.4 Target)

| Block | Pre-v3 | Post-v3 (no opt) | Post-v3.1 (P1-1/P1-2) | Post-v3.4 (full) |
|---|---|---|---|---|
| Skill listing | 12-15K | 12-15K | 1-2K (truncated) | 0.2K (DB-backed 极简索引) |
| Agent listing | 6-8K | 6-8K | 1K (truncated) | 0.1K (DB-backed 极简索引) |
| Collapsed tool listing | 5-10K | 5-10K | 5-10K | 2-3K (极简) |
| Runtime context | 3-5K | 3-5K | 3-5K | 0.2K |
| User context | 2-5K | 2-5K | 2-5K | 0.2K |
| System prompt (cached) | 10-20K | 10-20K | 10-20K | 0.3-0.5K |
| **Subtotal (cached)** | **~50K** | **~50K** | **~30K** | **~2K** |
| read_memory call (when used) | n/a | n/a | n/a | +0.5-1K per call |
| search_skill call (when used) | n/a | n/a | n/a | +0.2K per call |
| **Net for "hello world"** | **73K** | **73K** | **45K** | **~2K** |

For nontrivial turns, `read_memory` + `search_*` are called as needed (each is small), so net is **2K + (small) × N calls**— well under 5K for typical turns.

## 6. Migration Plan

### 6.1 Config Flag

```toml
# config.toml
[memory]
# "v2" = MemoryKeeperSubscriber (existing)
# "v3" = MemoryAgent (new, default)
extractor = "v3"

[memory.ab_test]
# When true, both extractors run, write to same DB.
# When false, only the selected extractor runs.
dual_write = false
```

Default in v3.x: `extractor = "v3"`, `dual_write = false` (clean replacement). To validate, temporarily set `dual_write = true` and compare DB content.

### 6.2 Coexistence Period (during v3.2 development)

- `MemoryKeeperSubscriber` (v2) and `MemoryAgent` (v3) **both registered** with event router.
- `dual_write = true`— both run, both write to same `memory.db`.
- Compare DB rows after N turns; if v3— v2 in count and quality, set `extractor = "v3"`, then `dual_write = false`.
- Remove v2 code only after 1 week of clean v3-only runs in default config.

### 6.3 P0/P1 Quick Wins (independent of v3)

These are fixed in v3.0 (~1-2 days) regardless of v3 architecture:

| Item | File | Fix |
|---|---|---|
| P0-1 | `src/crates/memory/src/extract.rs:309` | Add `.timeout(config.timeout)` |
| P0-2 | `src/crates/memory/src/extract.rs:399` | Replace byte slice with `chars().take(N).collect()` |
| P1-1 | `src/crates/assembly/core/src/agentic/skill_agent_snapshot.rs:30-45` | Truncate skill description to 160 chars |
| P1-2 | `src/crates/assembly/core/src/agentic/skill_agent_snapshot.rs:56-63` | Truncate agent description to 160 chars |
| P1-7 | `src/crates/assembly/core/src/agentic/coordination/coordinator.rs:5736-5739` | Same UTF-8 fix as P0-2 |
| P1-10 | `src/apps/desktop/src/theme.rs:527-530` | Same UTF-8 fix |

**Net effect**: 73K— 45K for hello world (35% reduction) before any v3 architecture work. This is a free win.

## 7. Implementation Phases

| Phase | Days | Deliverable | Token Reduction |
|---|---|---|---|
| **v3.0** | 1-2 | P0/P1 fixes (5 files) | 73K— 45K (35%) |
| **v3.1** | 2-3 | `skills.db` + `agents.db` + BM25 + 4 search tools | 45K— 30K (33%) |
| **v3.2** | 2-3 | `MemoryAgent` (background task) + A/B flag + distiller | 30K— 30K (no change yet) |
| **v3.3** | 2-3 | embedding 基础设施 + `read_memory` 工具 + memory.db schema 扩展 | 30K— 30K (still— need v3.4) |
| **v3.4** | 1-2 | `PartitionedLoader` + soul/agent/personality 文件拆分 | 30K— 2K (93%) |
| **v3.5** | (deferred) | File watcher for skills.db/agents.db, embedding cache | (no change) |

**Total: 8-13 days for full v3 architecture**. v3.0 alone is a 35% win in 1-2 days and is recommended as the first step regardless.

## 8. Error Handling

| Failure | Behavior |
|---|---|
| BM25 search returns 0 hits | Tool returns `{"results": []}`, no error |
| embedding model cold start timeout (>5s) | Fall back to BM25, log warning |
| LLM distiller call fails (timeout/network) | Log + skip; never block main agent; never panic |
| memory.db write fails (disk full) | Log + skip; re-attempt on next event |
| `skills.db` / `agents.db` read fails | Tool returns error to LLM, LLM sees "search_skill unavailable" |
| A/B comparison shows v3 missing entries | Roll back to v2 via `extractor = "v2"` flag (no data loss) |

## 9. Testing Strategy

| Test | Phase | Purpose |
|---|---|---|
| `cargo test -p northhing-memory` (P0 fixes) | v3.0 | Timeout + UTF-8 panic regressions |
| `cargo test -p northhing-core skill_index` | v3.1 | BM25 returns correct top-K for seed skills |
| `cargo test -p northhing-core agent_index` | v3.1 | Same for agents |
| Integration: `search_skill("pdf")` returns pdf skill | v3.1 | Tool wiring |
| Integration: A/B dual_write produces same row count | v3.2 | Migration safety |
| `cargo test -p northhing-memory distiller` | v3.2 | LLM distill produces valid structured entries |
| `cargo test -p northhing-core read_memory` | v3.3 | embedding + sync query path |
| End-to-end: hello world = ~2K tokens | v3.4 | Acceptance criterion |
| End-to-end: 24-skill search returns top-5 | v3.4 | Acceptance criterion |

## 10. Open Questions (for follow-up brainstorming, not blockers)

1. **Embedding model choice**: Local sentence-transformers vs API-based (OpenAI text-embedding-3-small). Local = free, no privacy concern, ~200MB model. API = better quality, costs $.
2. **Memory retention policy**: When does old memory get pruned— Importance < 0.3 + last_accessed > 90d— TBD in v3.5.
3. **Cross-session memory**: Should memory be shared across all sessions of one user, or session-isolated— Currently session-scoped; revisit when we have multi-session UX.
4. **read_memory token limit**: 5 default, 20 max. Should we cap output by token count instead of count— Add later.
5. **MemoryAgent storage quota**: `memory.db` could grow unboundedly. Need a soft cap. Add in v3.5.

## 11. Decision Log (this session)

| # | Question | Decision |
|---|---|---|
| 1 | MemoryAgent 形— | EventSubscriber + 后台 tokio::spawn task,纯蒸— |
| 1.x | 注入方式 | 常驻 `read_memory` 工具,— agent 主动— |
| 2 | Prefetch 去留 | 去掉 |
| 2.x | read_memory 响应— | 同步查库 |
| 2.x+ | 智能补全在哪 | 蒸馏阶段 |
| 3 | 检索策— | skill/agent=BM25, memory=embedding |
| 3.x | 实施顺序 | v3.0→v3.4 (5 阶段,P0/P1 优先) |
| 4 | 迁移路径 | 并行 + A/B config flag,验证后删 v2 |
| Baseline | 方案对比 |— B 收敛方案 |

---

**Next step**: User reviews this spec. After approval, invoke `writing-plans` skill to break each phase into concrete implementation tasks.
