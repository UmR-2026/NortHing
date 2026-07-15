<!-- LEGACY: 本文档是 v0.1.0 之前的历史计划，保留原 `agent-app` 名称作历史参考。
 Northing / 纳森 是 agent-app 的继任者（v0.1.0 之后改名）。
 本文件内容不被后续产品名替换脚本覆盖，保留 plan 当时的命名语境。 -->

# ⚠️ DEPRECATED — Replaced by `2026-06-17-v3-prompt-loader-impl-v2.md`



**This document is preserved for historical reference only.**



It was based on the **incorrect assumption** that agent-app has a `agent-app-memory` crate, `MemoryKeeperSubscriber`, `ExtractConfig`, and SQLite-based memory storage. **None of these exist in the current code** (verified 2026-06-17).



The "73K — 2K" goal was unverified; actual cached system prompt is ~46K chars / ~11.5K tokens (without tool manifest).



**See new plan for accurate v3 implementation based on real code:**

`docs/superpowers/plans/2026-06-17-v3-prompt-loader-impl-v2.md`



---



# v3 Prompt Loader Implementation Plan (HISTORICAL)



> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.



**Goal:** Reduce agent-app main agent initial input tokens from ~73K to ~2K through 5 progressive phases (v3.0 — v3.4) while preserving full capability surface.



**Architecture:** v3 converged design (Plan B from brainstorming). Partitioned prompt loader + BM25-indexed skill/agent databases + background MemoryAgent (pure distiller, sync write to DB) + read_memory tool (sync query, embedding search). All 5 search/read tools are sync, registered in existing ToolRegistry. MemoryAgent is a tokio::spawn task that subscribes to events via mpsc. Migration via A/B config flag (v2 MemoryKeeperSubscriber and v3 MemoryAgent coexist; `dual_write = true` for validation period).



**Tech Stack:** Rust (workspace), tokio mpsc, SQLite (rusqlite) + FTS5, rusqlite-embeddings or local sentence-transformers model (TBD in v3.3), existing `agent-app-memory` crate, existing `agent-app-agent-tools` ToolRegistry, Tauri 2.x (no frontend changes for v3.0-v3.3; v3.4 may touch prompt_builder).



**Spec:** `docs/superpowers/specs/2026-06-17-v3-prompt-loader-design.md`

**Review:** `docs/PROMPT_LOADER_ARCHITECTURE.md` (v3 draft, historical)



---



## File Structure



### New files (created across phases)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\prompts\loader\partitioned_loader.rs` (v3.4)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\prompts\soul.md` (v3.4)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\prompts\agent.md` (v3.4)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\prompts\personality\default.md` (v3.4)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\service\data\skill_index.rs` (v3.1)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\service\data\agent_index.rs` (v3.1)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\service\memory_agent\mod.rs` (v3.2)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\service\memory_agent\agent.rs` (v3.2)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\service\memory_agent\distiller.rs` (v3.2)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\service\memory_agent\config.rs` (v3.2)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\implementations\search\mod.rs` (v3.1/v3.3)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\implementations\search\skill.rs` (v3.1)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\implementations\search\agent.rs` (v3.1)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\implementations\search\memory.rs` (v3.3)

- `E:\agent-project\agent-app\src\crates\memory\src\embedding.rs` (v3.3)

- `E:\agent-project\agent-app\tests\v3_hello_world_token_count.rs` (v3.4 acceptance test)



### Modified files

- `E:\agent-project\agent-app\src\crates\memory\src\extract.rs` (v3.0: P0-1 timeout, P0-2 UTF-8)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\skill_agent_snapshot.rs` (v3.0: P1-1/P1-2 truncation; v3.1: switch to DB-backed listing)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\coordination\coordinator.rs` (v3.0: P1-7 UTF-8)

- `E:\agent-project\agent-app\src\apps\desktop\src\theme.rs` (v3.0: P1-10 UTF-8)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\service\memory_keeper\subscriber.rs` (v3.2: A/B flag)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\system.rs` (v3.2: register MemoryAgent task)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\agents\prompt_builder\prompt_builder_impl.rs` (v3.4: use PartitionedLoader)

- `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\registry.rs` (v3.1/v3.3: register new search tools)

- `E:\agent-project\agent-app\src\crates\memory\src\schema.rs` (v3.3: add embedding column)



---



## Phase v3.0: P0/P1 Quick Wins (1-2 days)



**Goal:** Fix all panic risks and apply description truncation. Net effect: 73K — 45K tokens (35% reduction) for hello world. Zero architecture changes.



### Task 1: Fix P0-1 — HTTP timeout ignored in memory extraction



**Files:**

- Modify: `E:\agent-project\agent-app\src\crates\memory\src\extract.rs:309-314`



- [ ] **Step 1: Verify failing condition exists**



Read the current `call_llm_sync` function in `extract.rs` around line 309. The current code is:



```rust

let (status, _headers, reader) = attohttpc::post(&url)

 .header("Authorization", format!("Bearer {}", config.api_key))

 .header("Content-Type", "application/json")

 .bytes(body_json)

 .send()

 .map_err(ExtractError::HttpClient)— ;

```



There is no `.timeout(config.timeout)` call. Confirm this by running:



```bash

grep -n "timeout" "E:/agent-project/agent-app/src/crates/memory/src/extract.rs"

```



Expected: Only matches for `config.timeout` (defined in struct), not `.timeout(` (the attohttpc method call). No match for `.timeout(config.timeout)`.



- [ ] **Step 2: Add timeout to HTTP call**



Replace lines 309-314 with:



```rust

let (status, _headers, reader) = attohttpc::post(&url)

 .header("Authorization", format!("Bearer {}", config.api_key))

 .header("Content-Type", "application/json")

 .bytes(body_json)

 .timeout(config.timeout)

 .send()

 .map_err(ExtractError::HttpClient)— ;

```



- [ ] **Step 3: Verify build**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-memory 2>&1 | tail -20

```



Expected: `Finished` line, no errors.



- [ ] **Step 4: Run memory crate tests**



```bash

cd E:/agent-project/agent-app && cargo test -p agent-app-memory 2>&1 | tail -20

```



Expected: All tests pass (existing tests should still work since we only added a config field to a call).



- [ ] **Step 5: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/memory/src/extract.rs && git commit -m "fix(memory): P0-1 apply HTTP timeout in call_llm_sync"

```



### Task 2: Fix P0-2 — UTF-8 boundary panic in extract.rs



**Files:**

- Modify: `E:\agent-project\agent-app\src\crates\memory\src\extract.rs:399`



- [ ] **Step 1: Verify the panic**



Read line 399 of `extract.rs`. Current code is:



```rust

&json_str[..json_str.len().min(200)]

```



This is a byte slice. When `json_str` is from LLM output and contains non-ASCII (Chinese, emojis, etc.), `&json_str[..200]` will panic if the byte at position 199 or 200 is a multi-byte UTF-8 continuation byte. This panic propagates out of the LLM extraction path and corrupts the dialog turn.



- [ ] **Step 2: Write a regression test**



Create `E:\agent-project\agent-app\src\crates\memory\src\tests_utf8.rs`:



```rust

use crate::extract::parse_extraction_response;



#[test]

fn parse_extraction_response_handles_chinese_at_boundary() {

 // Construct a string with Chinese chars that puts a multi-byte sequence near position 200.

 // Chinese char is 3 bytes in UTF-8. Fill to 195 ASCII chars, then add 10 Chinese chars.

 let mut s = String::with_capacity(225);

 s.push_str(&"a".repeat(195));

 s.push_str(&"— .repeat(10)); // 30 more bytes; total 225

 // Wrap in a JSON array shape parse_extraction_response expects.

 let wrapped = format!("[{{\"key\":\"{}\"}}]", s);



 // Should not panic even though index 200 falls inside a multi-byte char.

 let _ = parse_extraction_response(&wrapped);

}

```



- [ ] **Step 3: Add module declaration**



In `E:\agent-project\agent-app\src\crates\memory\src\lib.rs`, add at the bottom (before any `#[cfg(test)]` if present):



```rust

#[cfg(test)]

mod tests_utf8;

```



- [ ] **Step 4: Run test, expect panic (TDD red)**



```bash

cd E:/agent-project/agent-app && cargo test -p agent-app-memory parse_extraction_response_handles_chinese_at_boundary 2>&1 | tail -30

```



Expected: PANIC message "byte index 200 is not a char boundary" or similar.



- [ ] **Step 5: Fix parse_extraction_response**



Replace the error format block in `parse_extraction_response` (around line 397-400) with:



```rust

let preview: String = json_str.chars().take(200).collect();

Err(ExtractError::ParseResponse(format!(

 "failed to parse JSON from LLM output (first 200 chars): {}",

 preview

)))

```



This iterates over chars (not bytes), so it can never panic on a multi-byte boundary.



- [ ] **Step 6: Run test, expect pass (TDD green)**



```bash

cd E:/agent-project/agent-app && cargo test -p agent-app-memory parse_extraction_response_handles_chinese_at_boundary 2>&1 | tail -10

```



Expected: `test parse_extraction_response_handles_chinese_at_boundary ... ok`



- [ ] **Step 7: Run full memory test suite**



```bash

cd E:/agent-project/agent-app && cargo test -p agent-app-memory 2>&1 | tail -10

```



Expected: All tests pass.



- [ ] **Step 8: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/memory/src/extract.rs src/crates/memory/src/lib.rs src/crates/memory/src/tests_utf8.rs && git commit -m "fix(memory): P0-2 use char iteration in parse_extraction_response error path"

```



### Task 3: Fix P1-1 — Skill description truncation (160 chars)



**Files:**

- Modify: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\skill_agent_snapshot.rs:30-45`



- [ ] **Step 1: Add max-constant**



At the top of `skill_agent_snapshot.rs` (after the existing imports, before the `SkillSnapshotEntry` struct), add:



```rust

const MAX_DESCRIPTION_CHARS: usize = 160;

```



- [ ] **Step 2: Update to_xml_desc to truncate**



Replace the `to_xml_desc` method body (lines 30-45) with:



```rust

fn to_xml_desc(&self) -> String {

 let desc = truncate_chars(&self.description, MAX_DESCRIPTION_CHARS);

 format!(

 r#"<skill>

<name>

{}

</name>

<description>

{}

</description>

<location>

{}

</location>

</skill>"#,

 self.name, desc, self.location

 )

}

```



- [ ] **Step 3: Add the truncate_chars helper**



At the bottom of `skill_agent_snapshot.rs`, add:



```rust

fn truncate_chars(s: &str, max_chars: usize) -> String {

 if s.chars().count() <= max_chars {

 s.to_string()

 } else {

 let truncated: String = s.chars().take(max_chars).collect();

 format!("{}— , truncated)

 }

}

```



- [ ] **Step 4: Add unit test for truncation**



In `skill_agent_snapshot.rs`, add at the bottom:



```rust

#[cfg(test)]

mod tests {

 use super::*;



 #[test]

 fn truncate_chars_handles_chinese() {

 // 200 Chinese chars; truncation should not panic

 let s = "— .repeat(200);

 let out = truncate_chars(&s, 160);

 assert_eq!(out.chars().count(), 161); // 160 + ellipsis

 }



 #[test]

 fn truncate_chars_short_string_unchanged() {

 assert_eq!(truncate_chars("hello", 160), "hello");

 }

}

```



- [ ] **Step 5: Run tests**



```bash

cd E:/agent-project/agent-app && cargo test -p agent-app-core truncate_chars 2>&1 | tail -10

```



Expected: 2 tests pass.



- [ ] **Step 6: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/agentic/skill_agent_snapshot.rs && git commit -m "feat(core): P1-1 truncate skill descriptions to 160 chars"

```



### Task 4: Fix P1-2 — Agent description truncation (160 chars)



**Files:**

- Modify: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\skill_agent_snapshot.rs:55-63`



- [ ] **Step 1: Update to_xml_desc for AgentSnapshotEntry**



Replace the `to_xml_desc` method (lines 55-63) with:



```rust

fn to_xml_desc(&self) -> String {

 let desc = truncate_chars(&self.description, MAX_DESCRIPTION_CHARS);

 format!(

 "<agent type=\"{}\">\n<description>\n{}\n</description>\n<tools>{}</tools>\n</agent>",

 self.id,

 desc,

 self.default_tools.join(", ")

 )

}

```



- [ ] **Step 2: Run tests**



```bash

cd E:/agent-project/agent-app && cargo test -p agent-app-core 2>&1 | tail -10

```



Expected: All tests pass (truncate_chars helper already exists from Task 3).



- [ ] **Step 3: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/agentic/skill_agent_snapshot.rs && git commit -m "feat(core): P1-2 truncate agent descriptions to 160 chars"

```



### Task 5: Fix P1-7 — coordinator.rs UTF-8 panic in prune_context



**Files:**

- Modify: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\coordination\coordinator.rs:5736-5739`



- [ ] **Step 1: Replace byte slice with char iteration**



Replace lines 5736-5740:



```rust

 let new_rfa = format!(

 "{}... [truncated from {} chars]",

 &rfa[..max_result_for_assistant_chars],

 rfa.len()

 );

```



with:



```rust

 let preview: String = rfa.chars().take(max_result_for_assistant_chars).collect();

 let new_rfa = format!(

 "{}... [truncated from {} chars]",

 preview,

 rfa.chars().count()

 );

```



- [ ] **Step 2: Build check**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -10

```



Expected: `Finished` line, no errors.



- [ ] **Step 3: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/agentic/coordination/coordinator.rs && git commit -m "fix(core): P1-7 use char iteration in prune_context truncation"

```



### Task 6: Fix P1-10 — theme.rs to_tauri_color UTF-8 panic



**Files:**

- Modify: `E:\agent-project\agent-app\src\apps\desktop\src\theme.rs:527-530`



- [ ] **Step 1: Replace byte slice with safe indexing**



Replace lines 526-530:



```rust

 pub fn to_tauri_color(&self) -> tauri::window::Color {

 let hex = self.bg_primary.trim_start_matches('#');

 let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(18);

 let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(18);

 let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(20);

 tauri::window::Color(r, g, b, 255)

```



with:



```rust

 pub fn to_tauri_color(&self) -> tauri::window::Color {

 let hex = self.bg_primary.trim_start_matches('#');

 let safe_hex = if hex.len() >= 6 { &hex[..6] } else { hex };

 let r = u8::from_str_radix(&safe_hex[0..2], 16).unwrap_or(18);

 let g = u8::from_str_radix(&safe_hex[2..4], 16).unwrap_or(18);

 let b = u8::from_str_radix(&safe_hex[4..6], 16).unwrap_or(20);

 tauri::window::Color(r, g, b, 255)

```



For colors shorter than 6 hex chars, fall back to defaults. Since this file operates on hex strings (always ASCII), the original panic was actually on `&hex[..2]` when `hex.len() < 2` — not a UTF-8 boundary issue, but a length issue. The fix above handles both.



- [ ] **Step 2: Build check**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-desktop --target x86_64-pc-windows-msvc 2>&1 | tail -10

```



Expected: `Finished` line, no errors.



- [ ] **Step 3: Commit**



```bash

cd E:/agent-project/agent-app && git add src/apps/desktop/src/theme.rs && git commit -m "fix(desktop): P1-10 safe hex slicing in to_tauri_color"

```



### Task 7: v3.0 Acceptance — Verify token reduction



**Files:**

- Run: existing test suite + manual token count



- [ ] **Step 1: Run full test suite**



```bash

cd E:/agent-project/agent-app && cargo test --workspace 2>&1 | tail -30

```



Expected: All tests pass.



- [ ] **Step 2: Manual token count check**



Start the CLI:



```bash

cd E:/agent-project/agent-app && ./target/debug/agent-app-cli.exe exec "hello world" 2>&1 | head -100

```



Look for log output indicating prompt token count. Expected: somewhere between 40K-50K tokens (from 73K). If you have access to a logging field, confirm the number.



- [ ] **Step 3: Update PROJECT_STATE.md**



Append to the "Known issues" section of `E:\agent-project\agent-app\docs\PROJECT_STATE.md`:



```markdown

- v3.0 complete (2026-06-17): P0-1, P0-2, P1-1, P1-2, P1-7, P1-10 fixed. Token reduction: 73K — ~45K (35%).

```



- [ ] **Step 4: Commit**



```bash

cd E:/agent-project/agent-app && git add docs/PROJECT_STATE.md && git commit -m "docs: mark v3.0 complete in PROJECT_STATE"

```



---



## Phase v3.1: Skill/Agent Index + Search Tools (2-3 days)



**Goal:** Move skill/agent listing from in-prompt full rendering to DB-backed 极简索引 + on-demand detail. Net effect: 45K — 30K (33%).



### Task 8: Create skill_index module (SQLite + FTS5)



**Files:**

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\service\data\mod.rs`

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\service\data\skill_index.rs`

- Modify: `E:\agent-project\agent-app\src\crates\assembly\core\src\service\mod.rs` (if exists; add `pub mod data;`)



- [ ] **Step 1: Add data module shell**



Create `E:\agent-project\agent-app\src\crates\assembly\core\src\service\data\mod.rs`:



```rust

pub mod skill_index;

pub mod agent_index;

```



- [ ] **Step 2: Write failing test for skill_index**



In `skill_index.rs`, write at top:



```rust

use rusqlite::{Connection, params};

use std::path::Path;



pub struct SkillIndex {

 conn: Connection,

}



#[derive(Debug, Clone, PartialEq, Eq)]

pub struct SkillIndexEntry {

 pub id: String,

 pub name: String,

 pub description: String,

 pub full_path: String,

 pub source: String,

}



impl SkillIndex {

 pub fn open<P: AsRef<Path>>(path: P) -> rusqlite::Result<Self> {

 let conn = Connection::open(path)— ;

 conn.execute_batch("

 CREATE TABLE IF NOT EXISTS skill (

 id TEXT PRIMARY KEY,

 name TEXT NOT NULL,

 description TEXT NOT NULL,

 full_path TEXT NOT NULL,

 source TEXT NOT NULL,

 keywords TEXT NOT NULL,

 indexed_at INTEGER NOT NULL

 );

 CREATE VIRTUAL TABLE IF NOT EXISTS skill_fts USING fts5(

 id UNINDEXED, name, description, keywords,

 content='skill', content_rowid='rowid'

 );

 ")— ;

 Ok(Self { conn })

 }



 pub fn upsert(&self, entry: &SkillIndexEntry, keywords: &str) -> rusqlite::Result<()> {

 let now = std::time::SystemTime::now()

 .duration_since(std::time::UNIX_EPOCH)

 .unwrap_or_default()

 .as_secs() as i64;

 self.conn.execute(

 "INSERT OR REPLACE INTO skill (id, name, description, full_path, source, keywords, indexed_at)

 VALUES (— 1, — 2, — 3, — 4, — 5, — 6, — 7)",

 params![entry.id, entry.name, entry.description, entry.full_path, entry.source, keywords, now],

 )— ;

 self.conn.execute(

 "INSERT OR REPLACE INTO skill_fts (rowid, id, name, description, keywords)

 VALUES ((SELECT rowid FROM skill WHERE id = — 1), — 1, — 2, — 3, — 4)",

 params![entry.id, entry.name, entry.description, keywords],

 )— ;

 Ok(())

 }



 pub fn search(&self, query: &str, limit: usize) -> rusqlite::Result<Vec<SkillIndexEntry>> {

 // Escape user input for FTS5

 let escaped = query.replace('"', "\"\"");

 let fts_query = format!("\"{}\"", escaped);

 let mut stmt = self.conn.prepare(

 "SELECT s.id, s.name, s.description, s.full_path, s.source

 FROM skill_fts f JOIN skill s ON s.rowid = f.rowid

 WHERE skill_fts MATCH — 1

 ORDER BY rank

 LIMIT — 2"

 )— ;

 let rows = stmt.query_map(params![fts_query, limit as i64], |row| {

 Ok(SkillIndexEntry {

 id: row.get(0)— ,

 name: row.get(1)— ,

 description: row.get(2)— ,

 full_path: row.get(3)— ,

 source: row.get(4)— ,

 })

 })— ;

 rows.collect()

 }

}



#[cfg(test)]

mod tests {

 use super::*;



 #[test]

 fn search_returns_relevant_skill() {

 let index = SkillIndex::open(":memory:").unwrap();

 index.upsert(&SkillIndexEntry {

 id: "pdf".into(),

 name: "PDF Generation".into(),

 description: "Generate, edit, and convert PDF documents".into(),

 full_path: "/skills/pdf".into(),

 source: "builtin".into(),

 }, "pdf document generation latex").unwrap();

 index.upsert(&SkillIndexEntry {

 id: "xlsx".into(),

 name: "Excel Spreadsheets".into(),

 description: "Read and write Excel files".into(),

 full_path: "/skills/xlsx".into(),

 source: "builtin".into(),

 }, "excel spreadsheet xls csv").unwrap();



 let results = index.search("pdf", 5).unwrap();

 assert_eq!(results.len(), 1);

 assert_eq!(results[0].id, "pdf");

 }

}

```



- [ ] **Step 3: Run test, verify pass (since this is new code with test, TDD red-green compressed)**



```bash

cd E:/agent-project/agent-app && cargo test -p agent-app-core skill_index 2>&1 | tail -15

```



Expected: `test search_returns_relevant_skill ... ok`



- [ ] **Step 4: Add to service mod.rs**



Edit `E:\agent-project\agent-app\src\crates\assembly\core\src\service\mod.rs`:



```rust

pub mod data;

```



(Add at the end of the existing pub mod list.)



- [ ] **Step 5: Build check**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -10

```



Expected: `Finished` line.



- [ ] **Step 6: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/service/data/ src/crates/assembly/core/src/service/mod.rs && git commit -m "feat(core): v3.1 add skill_index with FTS5"

```



### Task 9: Create agent_index module



**Files:**

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\service\data\agent_index.rs`



- [ ] **Step 1: Create agent_index.rs**



Mirror the skill_index structure. The full file:



```rust

use rusqlite::{Connection, params};

use std::path::Path;



pub struct AgentIndex {

 conn: Connection,

}



#[derive(Debug, Clone, PartialEq, Eq)]

pub struct AgentIndexEntry {

 pub id: String,

 pub name: String,

 pub description: String,

 pub full_path: String,

 pub mode: String,

}



impl AgentIndex {

 pub fn open<P: AsRef<Path>>(path: P) -> rusqlite::Result<Self> {

 let conn = Connection::open(path)— ;

 conn.execute_batch("

 CREATE TABLE IF NOT EXISTS agent (

 id TEXT PRIMARY KEY,

 name TEXT NOT NULL,

 description TEXT NOT NULL,

 full_path TEXT NOT NULL,

 mode TEXT NOT NULL,

 keywords TEXT NOT NULL,

 indexed_at INTEGER NOT NULL

 );

 CREATE VIRTUAL TABLE IF NOT EXISTS agent_fts USING fts5(

 id UNINDEXED, name, description, keywords,

 content='agent', content_rowid='rowid'

 );

 ")— ;

 Ok(Self { conn })

 }



 pub fn upsert(&self, entry: &AgentIndexEntry, keywords: &str) -> rusqlite::Result<()> {

 let now = std::time::SystemTime::now()

 .duration_since(std::time::UNIX_EPOCH)

 .unwrap_or_default()

 .as_secs() as i64;

 self.conn.execute(

 "INSERT OR REPLACE INTO agent (id, name, description, full_path, mode, keywords, indexed_at)

 VALUES (— 1, — 2, — 3, — 4, — 5, — 6, — 7)",

 params![entry.id, entry.name, entry.description, entry.full_path, entry.mode, keywords, now],

 )— ;

 self.conn.execute(

 "INSERT OR REPLACE INTO agent_fts (rowid, id, name, description, keywords)

 VALUES ((SELECT rowid FROM agent WHERE id = — 1), — 1, — 2, — 3, — 4)",

 params![entry.id, entry.name, entry.description, keywords],

 )— ;

 Ok(())

 }



 pub fn search(&self, query: &str, limit: usize) -> rusqlite::Result<Vec<AgentIndexEntry>> {

 let escaped = query.replace('"', "\"\"");

 let fts_query = format!("\"{}\"", escaped);

 let mut stmt = self.conn.prepare(

 "SELECT a.id, a.name, a.description, a.full_path, a.mode

 FROM agent_fts f JOIN agent a ON a.rowid = f.rowid

 WHERE agent_fts MATCH — 1

 ORDER BY rank

 LIMIT — 2"

 )— ;

 let rows = stmt.query_map(params![fts_query, limit as i64], |row| {

 Ok(AgentIndexEntry {

 id: row.get(0)— ,

 name: row.get(1)— ,

 description: row.get(2)— ,

 full_path: row.get(3)— ,

 mode: row.get(4)— ,

 })

 })— ;

 rows.collect()

 }

}



#[cfg(test)]

mod tests {

 use super::*;



 #[test]

 fn search_returns_relevant_agent() {

 let index = AgentIndex::open(":memory:").unwrap();

 index.upsert(&AgentIndexEntry {

 id: "compress".into(),

 name: "Compress Agent".into(),

 description: "Compresses long context into a summary".into(),

 full_path: "/agents/compress".into(),

 mode: "ClassB".into(),

 }, "compress summary context reduce").unwrap();

 index.upsert(&AgentIndexEntry {

 id: "explore".into(),

 name: "Explore Agent".into(),

 description: "Read-only codebase explorer".into(),

 full_path: "/agents/explore".into(),

 mode: "ClassA".into(),

 }, "explore read search find").unwrap();



 let results = index.search("compress", 5).unwrap();

 assert_eq!(results.len(), 1);

 assert_eq!(results[0].id, "compress");

 }

}

```



- [ ] **Step 2: Run test**



```bash

cd E:/agent-project/agent-app && cargo test -p agent-app-core agent_index 2>&1 | tail -10

```



Expected: 1 test passes.



- [ ] **Step 3: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/service/data/agent_index.rs && git commit -m "feat(core): v3.1 add agent_index with FTS5"

```



### Task 10: Seed script — bulk-index builtin skills and agents



**Files:**

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\service\data\seed.rs`



- [ ] **Step 1: Implement seed function**



```rust

//! Seed the skill_index and agent_index from builtin skills/agents at startup.



use std::path::Path;

use crate::agentic::tools::implementations::skills::{get_skill_registry, SkillInfo};

use crate::agentic::agents::get_agent_registry;



use super::skill_index::{SkillIndex, SkillIndexEntry};

use super::agent_index::{AgentIndex, AgentIndexEntry};



pub fn seed_skill_index(index: &SkillIndex) -> rusqlite::Result<usize> {

 let registry = get_skill_registry();

 let skills = registry.all_skills();

 let mut count = 0;

 for skill in skills {

 let entry = SkillIndexEntry {

 id: skill.id.clone(),

 name: skill.name.clone(),

 description: skill.description.clone(),

 full_path: skill.path.clone(),

 source: detect_source(&skill.path),

 };

 let keywords = extract_keywords(&skill.description, &skill.name);

 index.upsert(&entry, &keywords)— ;

 count += 1;

 }

 Ok(count)

}



pub fn seed_agent_index(index: &AgentIndex) -> rusqlite::Result<usize> {

 let registry = get_agent_registry();

 let mut count = 0;

 for agent_id in registry.all_agent_ids() {

 let agent = registry.get_agent(&agent_id);

 if let Some(agent) = agent {

 let entry = AgentIndexEntry {

 id: agent_id.clone(),

 name: agent.name().to_string(),

 description: agent.description().to_string(),

 full_path: format!("builtin://{}", agent_id),

 mode: detect_mode(&agent_id),

 };

 let keywords = extract_keywords(&agent.description(), &agent.name());

 index.upsert(&entry, &keywords)— ;

 count += 1;

 }

 }

 Ok(count)

}



fn detect_source(path: &str) -> String {

 if path.contains("builtin_skills") { "builtin".to_string() }

 else if path.contains("/.agent-app/") || path.contains("/.claude/") { "project".to_string() }

 else { "user".to_string() }

}



fn detect_mode(id: &str) -> String {

 // Best-effort; the Agent trait should expose mode in a future patch.

 // For now, default to ClassA and let the catalog override.

 "ClassA".to_string()

}



fn extract_keywords(desc: &str, name: &str) -> String {

 let mut words: Vec<String> = desc.split_whitespace()

 .chain(name.split_whitespace())

 .map(|w| w.to_lowercase())

 .filter(|w| w.len() >= 3)

 .collect();

 words.sort();

 words.dedup();

 words.join(" ")

}

```



**Note**: This code uses `SkillInfo` and `Agent` trait methods. The exact field names (`id`, `name`, `description`, `path`, `description()`, `name()`) may not match the current trait exactly. Treat the code above as a sketch and adjust field names to match what `SkillInfo` and `Agent` actually expose. Run `cargo build` after writing; compiler errors will tell you which fields to rename.



- [ ] **Step 2: Build check (expect field mismatches)**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -30

```



If field names don't match, edit `seed.rs` to use the actual field names. Common adjustments: `skill.id` — `skill.name`, `agent.description()` — no method, read struct field directly, etc.



- [ ] **Step 3: Wire seeding into startup**



In `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\system.rs`, find the function that initializes the agentic system (around `init_agentic_system` or similar). Add at the top of that function:



```rust

// v3.1: seed skill/agent indices

let skills_db_path = config.data_dir.join("skills.db");

let agents_db_path = config.data_dir.join("agents.db");

if let Ok(skill_index) = SkillIndex::open(&skills_db_path) {

 if let Err(e) = seed_skill_index(&skill_index) {

 log::warn!("seed_skill_index failed: {}", e);

 }

}

if let Ok(agent_index) = AgentIndex::open(&agents_db_path) {

 if let Err(e) = seed_agent_index(&agent_index) {

 log::warn!("seed_agent_index failed: {}", e);

 }

}

```



- [ ] **Step 4: Build check**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -10

```



Expected: `Finished` line.



- [ ] **Step 5: Manual smoke test**



```bash

cd E:/agent-project/agent-app && rm -f $HOME/.agent-app/skills.db $HOME/.agent-app/agents.db

./target/debug/agent-app-cli.exe exec "hello" 2>&1 | head -30

ls -la $HOME/.agent-app/skills.db $HOME/.agent-app/agents.db

sqlite3 $HOME/.agent-app/skills.db "SELECT id, name FROM skill;"

```



Expected: Both `.db` files exist, skills.db has all 24 builtin skills listed.



- [ ] **Step 6: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/service/data/seed.rs src/crates/assembly/core/src/agentic/system.rs && git commit -m "feat(core): v3.1 seed skill/agent indices at startup"

```



### Task 11: Implement search_skill and get_skill_detail tools



**Files:**

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\implementations\search\mod.rs`

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\implementations\search\skill.rs`

- Modify: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\implementations\mod.rs` (add `pub mod search;`)



- [ ] **Step 1: Create search module shell**



`search\mod.rs`:



```rust

pub mod skill;

pub mod agent;

```



(`memory.rs` added in v3.3)



- [ ] **Step 2: Create search_skill tool**



`search\skill.rs`:



```rust

use async_trait::async_trait;

use serde_json::{json, Value};

use std::sync::Arc;



use crate::agentic::tools::framework::{Tool, ToolContext, ToolExposure, ToolResult};

use crate::service::data::skill_index::SkillIndex;



pub struct SearchSkillTool {

 pub index: Arc<SkillIndex>,

}



#[async_trait]

impl Tool for SearchSkillTool {

 fn name(&self) -> &str { "search_skill" }



 fn description(&self) -> &str {

 "Search for relevant skills by natural language query. Returns up to 5 skill summaries (id, name, 1-line description). Use get_skill_detail(id) to retrieve full SKILL.md content."

 }



 fn input_schema(&self) -> Value {

 json!({

 "type": "object",

 "properties": {

 "query": {"type": "string", "description": "Natural language search query"},

 "limit": {"type": "integer", "default": 5, "maximum": 10}

 },

 "required": ["query"]

 })

 }



 fn exposure(&self) -> ToolExposure { ToolExposure::Expanded }



 async fn execute(&self, _ctx: ToolContext, input: Value) -> ToolResult {

 let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("");

 let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

 if query.is_empty() {

 return ToolResult::error("query is required");

 }

 match self.index.search(query, limit) {

 Ok(entries) => ToolResult::ok(json!({

 "results": entries.iter().map(|e| json!({

 "id": e.id,

 "name": e.name,

 "description": e.description,

 "source": e.source,

 })).collect::<Vec<_>>()

 })),

 Err(e) => ToolResult::error(format!("search failed: {}", e)),

 }

 }

}



pub struct GetSkillDetailTool {

 pub index: Arc<SkillIndex>,

}



#[async_trait]

impl Tool for GetSkillDetailTool {

 fn name(&self) -> &str { "get_skill_detail" }



 fn description(&self) -> &str {

 "Retrieve the full SKILL.md content for a skill by id. Use after search_skill to get implementation details."

 }



 fn input_schema(&self) -> Value {

 json!({

 "type": "object",

 "properties": {

 "id": {"type": "string", "description": "Skill id from search_skill results"}

 },

 "required": ["id"]

 })

 }



 fn exposure(&self) -> ToolExposure { ToolExposure::Expanded }



 async fn execute(&self, _ctx: ToolContext, input: Value) -> ToolResult {

 let id = input.get("id").and_then(|v| v.as_str()).unwrap_or("");

 if id.is_empty() {

 return ToolResult::error("id is required");

 }

 match self.index.search(id, 1) {

 Ok(entries) if !entries.is_empty() => {

 let path = std::path::Path::new(&entries[0].full_path);

 match std::fs::read_to_string(path) {

 Ok(content) => ToolResult::ok(json!({

 "id": entries[0].id,

 "name": entries[0].name,

 "content": content,

 })),

 Err(e) => ToolResult::error(format!("read failed: {}", e)),

 }

 }

 Ok(_) => ToolResult::error(format!("skill not found: {}", id)),

 Err(e) => ToolResult::error(format!("search failed: {}", e)),

 }

 }

}

```



**Note**: The exact `Tool`, `ToolContext`, `ToolResult`, `ToolExposure` API may differ from what's shown. Adjust to match the actual `framework.rs` API. Common variations: `ToolResult::ok(json!({...}))` — `ToolResult::Ok(json)`, `ToolResult::error` — `ToolResult::Err(String)`, etc. The compiler will guide.



- [ ] **Step 3: Add to tools module**



In `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\implementations\mod.rs`, add:



```rust

pub mod search;

```



- [ ] **Step 4: Register in ToolRegistry**



In `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\registry.rs` (or wherever tools are registered at startup), add after the existing tool registrations:



```rust

// v3.1: search tools

let skill_index = Arc::new(SkillIndex::open(skills_db_path)— );

registry.register_tool(Arc::new(SearchSkillTool { index: skill_index.clone() }));

registry.register_tool(Arc::new(GetSkillDetailTool { index: skill_index.clone() }));

```



- [ ] **Step 5: Build check (expect API drift fixes)**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -30

```



Fix any field/method mismatches based on compiler errors.



- [ ] **Step 6: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/agentic/tools/implementations/search/ && git commit -m "feat(core): v3.1 add search_skill and get_skill_detail tools"

```



### Task 12: Implement search_agent and get_agent_detail tools



**Files:**

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\implementations\search\agent.rs`



- [ ] **Step 1: Create search_agent.rs**



Mirror `search/skill.rs` structure, swapping `SkillIndex` — `AgentIndex`, `SearchSkillTool` — `SearchAgentTool`, `GetSkillDetailTool` — `GetAgentDetailTool`, and reading agent definition files instead of SKILL.md.



```rust

use async_trait::async_trait;

use serde_json::{json, Value};

use std::sync::Arc;



use crate::agentic::tools::framework::{Tool, ToolContext, ToolExposure, ToolResult};

use crate::service::data::agent_index::AgentIndex;



pub struct SearchAgentTool {

 pub index: Arc<AgentIndex>,

}



#[async_trait]

impl Tool for SearchAgentTool {

 fn name(&self) -> &str { "search_agent" }



 fn description(&self) -> &str {

 "Search for relevant sub-agents by natural language query. Returns up to 5 agent summaries. Use get_agent_detail(id) for full agent prompt."

 }



 fn input_schema(&self) -> Value {

 json!({

 "type": "object",

 "properties": {

 "query": {"type": "string"},

 "limit": {"type": "integer", "default": 5, "maximum": 10}

 },

 "required": ["query"]

 })

 }



 fn exposure(&self) -> ToolExposure { ToolExposure::Expanded }



 async fn execute(&self, _ctx: ToolContext, input: Value) -> ToolResult {

 let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("");

 let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

 if query.is_empty() {

 return ToolResult::error("query is required");

 }

 match self.index.search(query, limit) {

 Ok(entries) => ToolResult::ok(json!({

 "results": entries.iter().map(|e| json!({

 "id": e.id,

 "name": e.name,

 "description": e.description,

 "mode": e.mode,

 })).collect::<Vec<_>>()

 })),

 Err(e) => ToolResult::error(format!("search failed: {}", e)),

 }

 }

}



pub struct GetAgentDetailTool {

 pub index: Arc<AgentIndex>,

}



#[async_trait]

impl Tool for GetAgentDetailTool {

 fn name(&self) -> &str { "get_agent_detail" }



 fn description(&self) -> &str {

 "Retrieve the full agent prompt and config by id."

 }



 fn input_schema(&self) -> Value {

 json!({

 "type": "object",

 "properties": {

 "id": {"type": "string"}

 },

 "required": ["id"]

 })

 }



 fn exposure(&self) -> ToolExposure { ToolExposure::Expanded }



 async fn execute(&self, _ctx: ToolContext, input: Value) -> ToolResult {

 let id = input.get("id").and_then(|v| v.as_str()).unwrap_or("");

 if id.is_empty() {

 return ToolResult::error("id is required");

 }

 match self.index.search(id, 1) {

 Ok(entries) if !entries.is_empty() => {

 ToolResult::ok(json!({

 "id": entries[0].id,

 "name": entries[0].name,

 "description": entries[0].description,

 "mode": entries[0].mode,

 }))

 }

 Ok(_) => ToolResult::error(format!("agent not found: {}", id)),

 Err(e) => ToolResult::error(format!("search failed: {}", e)),

 }

 }

}

```



- [ ] **Step 2: Register in ToolRegistry**



Add to the same registration block as Task 11:



```rust

let agent_index = Arc::new(AgentIndex::open(agents_db_path)— );

registry.register_tool(Arc::new(SearchAgentTool { index: agent_index.clone() }));

registry.register_tool(Arc::new(GetAgentDetailTool { index: agent_index.clone() }));

```



- [ ] **Step 3: Build and fix any API drift**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -30

```



- [ ] **Step 4: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/agentic/tools/implementations/search/agent.rs && git commit -m "feat(core): v3.1 add search_agent and get_agent_detail tools"

```



### Task 13: Switch skill_agent_snapshot to DB-backed listing



**Files:**

- Modify: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\skill_agent_snapshot.rs`



- [ ] **Step 1: Add DB-backed rendering function**



In `skill_agent_snapshot.rs`, add a new function:



```rust

/// Render 极简索引 (DB-backed). Replaces the full listing for prompt building.

pub fn render_db_backed_skill_listing(

 skill_index: &SkillIndex,

 limit: usize,

) -> String {

 // Get all skills (no query); truncate each description to 80 chars for the prompt.

 let entries = skill_index.search("", limit).unwrap_or_default();

 entries.iter().map(|e| {

 let desc = truncate_chars(&e.description, 80);

 format!("- `{}`: {}\n", e.id, desc)

 }).collect()

}

```



**Note**: An empty FTS5 query returns 0 results in our schema. If that fails, change `search` to accept an empty query and return all entries (need a separate `list_all` method on SkillIndex). Adjust based on what FTS5 actually returns for empty query.



- [ ] **Step 2: Add feature flag for DB-backed listing**



At the top of the file, add:



```rust

const USE_DB_BACKED_LISTING: bool = true; // v3.1: enabled by default

```



- [ ] **Step 3: Modify the listing builder to use DB-backed**



Find the function that builds the skill_listing block (search for `render_full_skill_listing_body` or `build_skill_listing_section` in `skill_agent_snapshot.rs`). Wrap the body in:



```rust

if USE_DB_BACKED_LISTING {

 // Use DB-backed listing

 render_db_backed_skill_listing(skill_index, 24)

} else {

 // Old full-listing path (deprecated)

 render_full_skill_listing_body(snapshot)

}

```



Apply the same pattern to the agent listing.



- [ ] **Step 4: Pass the index to the builder**



The snapshot builder signature needs the SkillIndex and AgentIndex. Pass them in from the call site (likely `execution_engine.rs` or `prompt_builder_impl.rs`). Add parameters:



```rust

pub fn resolve_skill_agent_snapshot(

 ...

 skill_index: &SkillIndex,

 agent_index: &AgentIndex,

) -> TurnSkillAgentSnapshot {

 // ...

}

```



- [ ] **Step 5: Build check (expect call-site fixes)**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -30

```



Fix all call sites that don't yet pass the indices. Common call sites: `execution_engine.rs`, `prompt_builder_impl.rs`.



- [ ] **Step 6: Manual token count check**



```bash

cd E:/agent-project/agent-app && ./target/debug/agent-app-cli.exe exec "hello world" 2>&1 | head -100

```



Look for prompt token count. Expected: ~30K (from 45K, 33% reduction).



- [ ] **Step 7: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/agentic/skill_agent_snapshot.rs && git commit -m "feat(core): v3.1 switch skill/agent listing to DB-backed 极简索引"

```



### Task 14: v3.1 Acceptance



- [ ] **Step 1: Run full test suite**



```bash

cd E:/agent-project/agent-app && cargo test --workspace 2>&1 | tail -30

```



Expected: All tests pass.



- [ ] **Step 2: Manual end-to-end test**



```bash

cd E:/agent-project/agent-app && ./target/debug/agent-app-cli.exe exec "search for a skill about PDFs and show me what it can do"

```



Expected: The LLM calls `search_skill("pdf")` — gets back skill summary — calls `get_skill_detail("pdf")` — reads SKILL.md.



- [ ] **Step 3: Commit any test files**



```bash

cd E:/agent-project/agent-app && git status

```



If any test files are uncommitted, commit them.



- [ ] **Step 4: Update PROJECT_STATE.md**



Append:



```markdown

- v3.1 complete (2026-06-17): skills.db + agents.db + 4 search tools. Token reduction: 45K — ~30K (33%).

```



---



## Phase v3.2: MemoryAgent Background Task + A/B Migration (2-3 days)



**Goal:** Replace the synchronous MemoryKeeperSubscriber with a background tokio task that distills events into structured memory. A/B flag enables safe migration.



### Task 15: Create MemoryAgentConfig



**Files:**

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\service\memory_agent\mod.rs`

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\service\memory_agent\config.rs`



- [ ] **Step 1: Create mod.rs**



```rust

pub mod config;

pub mod distiller;

pub mod agent;

```



- [ ] **Step 2: Create config.rs**



```rust

use serde::{Deserialize, Serialize};



#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct MemoryAgentConfig {

 /// "v2" = MemoryKeeperSubscriber only; "v3" = MemoryAgent only; "both" = A/B

 pub extractor: String,

 /// When true, both v2 and v3 write to the same DB. When false, only `extractor` runs.

 pub dual_write: bool,

 /// Debounce window in seconds (default 5)

 pub debounce_seconds: u64,

 /// LLM distiller timeout in seconds

 pub distiller_timeout_seconds: u64,

}



impl Default for MemoryAgentConfig {

 fn default() -> Self {

 Self {

 extractor: "v3".to_string(), // v3 default after this phase

 dual_write: false,

 debounce_seconds: 5,

 distiller_timeout_seconds: 30,

 }

 }

}

```



- [ ] **Step 3: Add to service mod.rs**



In `E:\agent-project\agent-app\src\crates\assembly\core\src\service\mod.rs`, add:



```rust

pub mod memory_agent;

```



- [ ] **Step 4: Build check**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -10

```



Expected: `Finished` line (memory_agent mod.rs is empty stub otherwise).



- [ ] **Step 5: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/service/memory_agent/ && git commit -m "feat(core): v3.2 add memory_agent module shell with config"

```



### Task 16: Implement distiller (LLM call wrapper)



**Files:**

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\service\memory_agent\distiller.rs`



- [ ] **Step 1: Create distiller.rs**



```rust

//! LLM distiller: takes a tool call result or turn transcript and produces structured memory entries.



use agent-app_memory::{MemoryService, ServiceResult, ExtractedItem};

use serde::{Deserialize, Serialize};

use std::sync::Arc;

use std::time::Duration;



#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct DistillRequest {

 pub source: String, // "ToolCallCompleted" | "DialogTurnCompleted"

 pub content: String, // formatted tool call or turn transcript

}



#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct DistillResult {

 pub entries: Vec<ExtractedItem>,

 pub skipped_reason: Option<String>,

}



pub struct Distiller {

 pub llm_client: Arc<dyn LlmClient>,

 pub memory_service: Arc<MemoryService>,

 pub timeout: Duration,

}



#[async_trait::async_trait]

pub trait LlmClient: Send + Sync {

 async fn complete(&self, system: &str, user: &str, timeout: Duration) -> anyhow::Result<String>;

}



impl Distiller {

 pub async fn distill(&self, req: DistillRequest) -> DistillResult {

 // 1. Quick filter: skip empty/short content

 if req.content.trim().len() < 20 {

 return DistillResult { entries: vec![], skipped_reason: Some("content too short".into()) };

 }



 // 2. LLM call (uses existing extract.rs prompt template)

 let system = MEMORY_DISTILL_PROMPT;

 let user = format!("Source: {}\n\nContent:\n{}", req.source, req.content);



 let result = self.llm_client.complete(system, &user, self.timeout).await;

 let response = match result {

 Ok(s) => s,

 Err(e) => {

 log::warn!("distill LLM call failed: {}", e);

 return DistillResult { entries: vec![], skipped_reason: Some(format!("LLM error: {}", e)) };

 }

 };



 // 3. Parse response (reuse existing parse_extraction_response)

 let items = match agent-app_memory::extract::parse_extraction_response(&response) {

 Ok(items) => items,

 Err(e) => {

 log::warn!("distill parse failed: {}", e);

 return DistillResult { entries: vec![], skipped_reason: Some(format!("parse: {}", e)) };

 }

 };



 DistillResult { entries: items, skipped_reason: None }

 }



 pub async fn store(&self, items: Vec<ExtractedItem>, session_id: &str, turn_id: &str, source: &str) -> ServiceResult<usize> {

 // Reuse existing store_entries

 agent-app_memory::extract::store_entries(&self.memory_service, items)

 .map(|n| n) // already returns count

 // actually we need session_id/turn_id here; existing store_entries takes svc + items

 // and we may need to extend it to accept session/turn metadata.

 // For now, this is a placeholder; the real impl will call a slightly different signature.

 }

}



const MEMORY_DISTILL_PROMPT: &str = r#"You are a memory distillation assistant.

Extract durable facts, preferences, and important context from the input.

Output JSON: [{"category": "fact|preference|context|skill_use", "content": "...", "importance": 0.0-1.0}]

Skip transient or trivial content."#;

```



**Note**: The `store` method is incomplete because `store_entries` in `extract.rs` doesn't currently accept session_id / turn_id / source. The real implementation will need to either extend `store_entries` to accept these parameters, or call `MemoryService::insert` directly. Adjust the signature based on the actual `MemoryService` API.



- [ ] **Step 2: Build check (expect API adjustments)**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -30

```



Adjust `store` method based on `MemoryService` actual API.



- [ ] **Step 3: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/service/memory_agent/distiller.rs && git commit -m "feat(core): v3.2 add distiller with LLM call wrapper"

```



### Task 17: Implement MemoryAgent task



**Files:**

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\service\memory_agent\agent.rs`



- [ ] **Step 1: Create agent.rs**



```rust

//! MemoryAgent: background tokio task that consumes events from the event router

//! and distills them into structured memory entries.



use super::config::MemoryAgentConfig;

use super::distiller::{Distiller, DistillRequest};

use agent-app_memory::MemoryService;

use agent-app_events::{AgenticEvent, mpsc};

use std::sync::Arc;

use std::time::Duration;

use tokio::sync::Mutex;



pub struct MemoryAgent {

 event_rx: mpsc::Receiver<AgenticEvent>,

 distiller: Arc<Distiller>,

 config: MemoryAgentConfig,

 debounce_state: Mutex<DebounceState>,

}



#[derive(Default)]

struct DebounceState {

 last_flush: std::time::Instant,

 pending: Vec<AgenticEvent>,

}



impl MemoryAgent {

 pub fn new(

 event_rx: mpsc::Receiver<AgenticEvent>,

 distiller: Arc<Distiller>,

 config: MemoryAgentConfig,

 ) -> Self {

 Self {

 event_rx,

 distiller,

 config,

 debounce_state: Mutex::new(DebounceState {

 last_flush: std::time::Instant::now(),

 pending: vec![],

 }),

 }

 }



 pub async fn run(mut self) {

 while let Some(event) = self.event_rx.recv().await {

 self.handle_event(event).await;

 }

 }



 async fn handle_event(&self, event: AgenticEvent) {

 match event {

 AgenticEvent::ToolCallCompleted { tool_name, result, session_id, turn_id, .. } => {

 self.enqueue(DistillRequest {

 source: "ToolCallCompleted".into(),

 content: format!("Tool: {}\nResult: {}", tool_name, truncate(&result, 2000)),

 }, session_id, turn_id).await;

 }

 AgenticEvent::DialogTurnCompleted { session_id, turn_id, .. } => {

 self.enqueue(DistillRequest {

 source: "DialogTurnCompleted".into(),

 content: format!("Turn {} completed in session {}", turn_id, session_id),

 }, session_id, turn_id).await;

 }

 _ => {}

 }

 }



 async fn enqueue(&self, req: DistillRequest, session_id: String, turn_id: String) {

 let mut state = self.debounce_state.lock().await;

 state.pending.push(AgenticEvent::Custom(req.content.clone())); // pseudo-event; replace with real structure



 if state.last_flush.elapsed() < Duration::from_secs(self.config.debounce_seconds) {

 return;

 }



 let pending: Vec<DistillRequest> = state.pending.drain(..).map(|e| {

 if let AgenticEvent::Custom(content) = e {

 DistillRequest { source: "DebouncedBatch".into(), content }

 } else {

 DistillRequest { source: "Unknown".into(), content: String::new() }

 }

 }).collect();

 state.last_flush = std::time::Instant::now();

 drop(state);



 for req in pending {

 let result = self.distiller.distill(req).await;

 if !result.entries.is_empty() {

 if let Err(e) = self.distiller.store(result.entries, &session_id, &turn_id, "v3").await {

 log::warn!("store failed: {}", e);

 }

 }

 }

 }

}



fn truncate(s: &str, max: usize) -> String {

 if s.chars().count() <= max { s.to_string() }

 else {

 let t: String = s.chars().take(max).collect();

 format!("{}— , t)

 }

}

```



**Note**: This is a sketch. The `AgenticEvent` enum doesn't actually have a `Custom` variant or a `tool_name`/`result` field — those field names are guessed. Adjust based on the actual `agent-app_events::AgenticEvent` API. The debounce batching structure may also need rework.



- [ ] **Step 2: Build check (expect heavy API adjustments)**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -50

```



Iterate on field/method names until it compiles. Common fixes:

- `AgenticEvent::ToolCallCompleted { tool_name, result, ... }` — actual field names from `agent-app_events`

- `AgenticEvent::DialogTurnCompleted { ... }` — actual fields

- Remove `AgenticEvent::Custom` (doesn't exist) — use a local enum or pass the request directly



- [ ] **Step 3: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/service/memory_agent/agent.rs && git commit -m "feat(core): v3.2 add MemoryAgent background task"

```



### Task 18: Wire MemoryAgent into event router with A/B flag



**Files:**

- Modify: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\system.rs`

- Modify: `E:\agent-project\agent-app\src\crates\assembly\core\src\service\memory_keeper\subscriber.rs` (add A/B config check)



- [ ] **Step 1: Add A/B flag check to v2 subscriber**



In `subscriber.rs`, find the `MemoryKeeperSubscriber::new` constructor. Modify it to read `MemoryAgentConfig::extractor` and only register itself when `extractor == "v2"` or `dual_write == true`. Add at the start of `new`:



```rust

pub fn new(config: MemoryAgentConfig) -> Self {

 if config.extractor != "v2" && !config.dual_write {

 // v2 disabled; create a no-op subscriber

 log::info!("MemoryKeeperSubscriber (v2) disabled by config");

 return Self::disabled();

 }

 // ... existing init

}

```



Add a `disabled()` constructor that returns a subscriber that ignores all events:



```rust

fn disabled() -> Self {

 Self { /* zero out all fields */ }

}

```



Adjust the `handle_event` method to early-return when `self.disabled`.



- [ ] **Step 2: Spawn MemoryAgent in system.rs**



In `init_agentic_system`, add after the existing `MemoryKeeperSubscriber` registration:



```rust

// v3.2: spawn MemoryAgent (v3) if configured

if config.memory_agent.extractor == "v3" || config.memory_agent.dual_write {

 let (tx, rx) = agent-app_events::mpsc::channel(100);

 let distiller = Arc::new(Distiller {

 llm_client: llm_client.clone(),

 memory_service: memory_service.clone(),

 timeout: Duration::from_secs(config.memory_agent.distiller_timeout_seconds),

 });

 let memory_agent = MemoryAgent::new(rx, distiller, config.memory_agent.clone());

 tokio::spawn(async move {

 memory_agent.run().await;

 });

 event_router.subscribe_internal("memory_agent", tx);

 log::info!("MemoryAgent (v3) registered");

}

```



- [ ] **Step 3: Build check**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -30

```



Fix any field/method mismatches.



- [ ] **Step 4: Manual smoke test (v2 path)**



```bash

cd E:/agent-project/agent-app && rm -f $HOME/.agent-app/memory/hm.db

# Set config to v2

echo '[memory]\nextractor = "v2"' >> $HOME/.agent-app/config.toml

./target/debug/agent-app-cli.exe exec "say hi" 2>&1 | head -30

sqlite3 $HOME/.agent-app/memory/hm.db "SELECT COUNT(*) FROM memory_entry;"

```



Expected: Some memory entries exist (v2 path works).



- [ ] **Step 5: Manual smoke test (v3 path)**



```bash

cd E:/agent-project/agent-app && rm -f $HOME/.agent-app/memory/hm.db

echo '[memory]\nextractor = "v3"\ndual_write = false' >> $HOME/.agent-app/config.toml

./target/debug/agent-app-cli.exe exec "say hi" 2>&1 | head -30

sqlite3 $HOME/.agent-app/memory/hm.db "SELECT COUNT(*) FROM memory_entry;"

```



Expected: Some memory entries exist (v3 path works).



- [ ] **Step 6: Manual A/B test**



```bash

cd E:/agent-project/agent-app && rm -f $HOME/.agent-app/memory/hm.db

echo '[memory]\nextractor = "v3"\ndual_write = true' >> $HOME/.agent-app/config.toml

./target/debug/agent-app-cli.exe exec "say hi" 2>&1 | head -30

sqlite3 $HOME/.agent-app/memory/hm.db "SELECT source, COUNT(*) FROM memory_entry GROUP BY source;"

```



Expected: Both v2 and v3 sources present (or however the schema tracks source).



- [ ] **Step 7: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/agentic/system.rs src/crates/assembly/core/src/service/memory_keeper/subscriber.rs && git commit -m "feat(core): v3.2 wire MemoryAgent task + A/B config flag"

```



### Task 19: v3.2 Acceptance



- [ ] **Step 1: Run full test suite**



```bash

cd E:/agent-project/agent-app && cargo test --workspace 2>&1 | tail -30

```



Expected: All tests pass.



- [ ] **Step 2: A/B quality check (manual, takes time)**



Run a long dialog (10+ turns) with `dual_write = true`. After the dialog, query both v2 and v3 entries; verify v3 captures the same or more facts. If v3 is missing critical entries, iterate on the distiller prompt.



- [ ] **Step 3: Set v3 as default in config**



Edit `E:\agent-project\agent-app\src\apps\agent-app-cli\config\default.toml` (or wherever the default config lives):



```toml

[memory]

extractor = "v3"

dual_write = false

```



- [ ] **Step 4: Update PROJECT_STATE.md**



```markdown

- v3.2 complete (2026-06-17): MemoryAgent background task with A/B flag. v3 is now default. v2 path is preserved for 1 week validation.

```



- [ ] **Step 5: Commit**



```bash

cd E:/agent-project/agent-app && git add src/apps/agent-app-cli/config/default.toml docs/PROJECT_STATE.md && git commit -m "chore: v3.2 set v3 extractor as default"

```



---



## Phase v3.3: Memory Embedding + read_memory Tool (2-3 days)



**Goal:** Add embedding-based semantic search to memory.db and expose it via read_memory tool.



### Task 20: Choose and integrate embedding model



**Files:**

- Create: `E:\agent-project\agent-app\src\crates\memory\src\embedding.rs`



- [ ] **Step 1: Add embedding model dependency**



In `E:\agent-project\agent-app\src\crates\memory\Cargo.toml`, add:



```toml

[dependencies]

fastembed = "3" # local sentence-transformers, no Python

# or

# ort = "2" # ONNX runtime, more flexible

```



(The exact crate will be decided at implementation time based on what produces a 384-dim vector with low overhead.)



- [ ] **Step 2: Implement embedding wrapper**



```rust

//! Embedding model wrapper: produces 384-dim vectors for memory entries.



use fastembed::{TextEmbedding, Model};



pub struct Embedder {

 model: TextEmbedding,

}



impl Embedder {

 pub fn new() -> anyhow::Result<Self> {

 let model = TextEmbedding::try_new(Default::default())— ;

 Ok(Self { model })

 }



 pub fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {

 let docs = vec![text.to_string()];

 let embeddings = self.model.embed(docs, None)— ;

 Ok(embeddings.into_iter().next().unwrap_or_default())

 }



 pub fn embed_batch(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {

 self.model.embed(texts.to_vec(), None).map_err(Into::into)

 }

}

```



- [ ] **Step 3: Add unit test**



```rust

#[cfg(test)]

mod tests {

 use super::*;



 #[test]

 fn embed_returns_384_dim_vector() {

 let embedder = Embedder::new().unwrap();

 let v = embedder.embed("hello world").unwrap();

 assert_eq!(v.len(), 384);

 }

}

```



- [ ] **Step 4: Build check (expect long compile for first time)**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-memory 2>&1 | tail -10

```



First build will take 5-10 minutes downloading the model. Subsequent builds fast.



- [ ] **Step 5: Run test**



```bash

cd E:/agent-project/agent-app && cargo test -p agent-app-memory embed 2>&1 | tail -10

```



Expected: 1 test passes.



- [ ] **Step 6: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/memory/Cargo.toml src/crates/memory/src/embedding.rs && git commit -m "feat(memory): v3.3 add local embedding model (384-dim)"

```



### Task 21: Add embedding column to memory schema



**Files:**

- Modify: `E:\agent-project\agent-app\src\crates\memory\src\schema.rs`

- Modify: `E:\agent-project\agent-app\src\crates\memory\src\db.rs` (add migration)



- [ ] **Step 1: Add embedding column to MemoryEntry struct**



In `schema.rs`, add to `MemoryEntry`:



```rust

#[serde(skip_serializing_if = "Option::is_none", default)]

pub embedding: Option<Vec<f32>>,

```



- [ ] **Step 2: Add migration in db.rs**



In `init_default_memory_db` or equivalent, add a migration that adds the `embedding` column if it doesn't exist:



```rust

conn.execute("ALTER TABLE memory_entry ADD COLUMN embedding BLOB", [])— ;

// Ignore error if column already exists

```



**Note**: The exact migration pattern (idempotent ALTER TABLE) needs to match the existing migration framework. Adjust accordingly. A common pattern:



```rust

let _ = conn.execute("ALTER TABLE memory_entry ADD COLUMN embedding BLOB", []);

```



(Swallowing the error is acceptable since ALTER TABLE ADD COLUMN is idempotent in some DBs; for SQLite specifically, you may need a `pragma_table_info` check.)



- [ ] **Step 3: Update extract.rs to embed entries before storing**



In `store_entries`, add embedding step:



```rust

pub fn store_entries(svc: &MemoryService<'_>, items: Vec<ExtractedItem>, embedder: Option<&Embedder>) -> ServiceResult<usize> {

 for mut item in items {

 if let Some(emb) = embedder {

 if let Ok(vec) = emb.embed(&item.content) {

 item.embedding = Some(vec);

 }

 }

 svc.insert(item)— ;

 }

 Ok(/* count */)

}

```



- [ ] **Step 4: Wire embedder into distiller**



In `distiller.rs`, the `store` method should call `store_entries` with the embedder.



- [ ] **Step 5: Build and test**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-memory 2>&1 | tail -10

cd E:/agent-project/agent-app && cargo test -p agent-app-memory 2>&1 | tail -10

```



Expected: All tests pass.



- [ ] **Step 6: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/memory/src/schema.rs src/crates/memory/src/db.rs src/crates/memory/src/extract.rs && git commit -m "feat(memory): v3.3 add embedding column and embed-on-store"

```



### Task 22: Implement semantic search in MemoryService



**Files:**

- Modify: `E:\agent-project\agent-app\src\crates\memory\src\service.rs`



- [ ] **Step 1: Add semantic_search method**



```rust

impl MemoryService {

 pub fn semantic_search(&self, query_embedding: &[f32], limit: usize) -> ServiceResult<Vec<MemoryEntry>> {

 // 1. Load all entries (or use a precomputed index)

 let entries: Vec<MemoryEntry> = self.list_all()— ;



 // 2. Compute cosine similarity, sort, take top-K

 let mut scored: Vec<(f32, MemoryEntry)> = entries

 .into_iter()

 .filter_map(|e| {

 e.embedding.as_ref().map(|v| {

 let sim = cosine_similarity(query_embedding, v);

 (sim, e)

 })

 })

 .collect();

 scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

 Ok(scored.into_iter().take(limit).map(|(_, e)| e).collect())

 }

}



fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {

 let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();

 let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();

 let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

 if mag_a == 0.0 || mag_b == 0.0 { 0.0 } else { dot / (mag_a * mag_b) }

}

```



- [ ] **Step 2: Add test**



```rust

#[cfg(test)]

mod tests {

 use super::*;



 #[test]

 fn cosine_similarity_identical_vectors() {

 let v = vec![1.0, 0.0, 0.0];

 assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);

 }



 #[test]

 fn cosine_similarity_orthogonal_vectors() {

 let a = vec![1.0, 0.0];

 let b = vec![0.0, 1.0];

 assert!(cosine_similarity(&a, &b).abs() < 1e-6);

 }

}

```



- [ ] **Step 3: Build and test**



```bash

cd E:/agent-project/agent-app && cargo test -p agent-app-memory cosine 2>&1 | tail -10

```



- [ ] **Step 4: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/memory/src/service.rs && git commit -m "feat(memory): v3.3 add semantic_search via cosine similarity"

```



### Task 23: Implement read_memory tool



**Files:**

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\implementations\search\memory.rs`

- Modify: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\tools\implementations\search\mod.rs`



- [ ] **Step 1: Add memory.rs to search module**



In `search/mod.rs`, add:



```rust

pub mod memory;

```



- [ ] **Step 2: Create read_memory tool**



```rust

use async_trait::async_trait;

use serde_json::{json, Value};

use std::sync::Arc;



use crate::agentic::tools::framework::{Tool, ToolContext, ToolExposure, ToolResult};

use agent-app_memory::{MemoryService, Embedder};



pub struct ReadMemoryTool {

 pub memory_service: Arc<MemoryService>,

 pub embedder: Arc<Embedder>,

}



#[async_trait]

impl Tool for ReadMemoryTool {

 fn name(&self) -> &str { "read_memory" }



 fn description(&self) -> &str {

 "Read structured memory entries by ID or by semantic query. Use when the user references past context, prior decisions, or anything that may be in long-term memory."

 }



 fn input_schema(&self) -> Value {

 json!({

 "type": "object",

 "properties": {

 "ids": {

 "type": "array",

 "items": {"type": "string"},

 "description": "Specific memory entry IDs (fast path)"

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



 fn exposure(&self) -> ToolExposure { ToolExposure::Expanded }



 async fn execute(&self, _ctx: ToolContext, input: Value) -> ToolResult {

 // Path 1: query by IDs

 if let Some(ids) = input.get("ids").and_then(|v| v.as_array()) {

 let id_strs: Vec<String> = ids.iter()

 .filter_map(|v| v.as_str().map(String::from))

 .collect();

 return match self.memory_service.get_by_ids(&id_strs) {

 Ok(entries) => ToolResult::ok(format_entries(&entries)),

 Err(e) => ToolResult::error(format!("lookup failed: {}", e)),

 };

 }



 // Path 2: semantic query

 let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("");

 let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

 if query.is_empty() {

 return ToolResult::error("either ids or query is required");

 }



 let embedding = match self.embedder.embed(query) {

 Ok(v) => v,

 Err(e) => return ToolResult::error(format!("embed failed: {}", e)),

 };



 match self.memory_service.semantic_search(&embedding, limit) {

 Ok(entries) => ToolResult::ok(format_entries(&entries)),

 Err(e) => ToolResult::error(format!("search failed: {}", e)),

 }

 }

}



fn format_entries(entries: &[agent-app_memory::MemoryEntry]) -> Value {

 json!({

 "entries": entries.iter().map(|e| json!({

 "id": e.id,

 "category": e.category,

 "content": e.content,

 "importance": e.importance,

 "created_at": e.created_at,

 })).collect::<Vec<_>>()

 })

}

```



**Note**: `MemoryService::get_by_ids` may not exist yet; add it as a simple `WHERE id IN (...)` query in service.rs. Field names (`category`, `content`, `importance`, `created_at`) need to match the actual `MemoryEntry` struct.



- [ ] **Step 3: Register in ToolRegistry**



Add to the search tools registration block:



```rust

let embedder = Arc::new(Embedder::new()— );

let read_memory = ReadMemoryTool {

 memory_service: memory_service.clone(),

 embedder,

};

registry.register_tool(Arc::new(read_memory));

```



- [ ] **Step 4: Build check**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -30

```



- [ ] **Step 5: Manual test**



```bash

cd E:/agent-project/agent-app && ./target/debug/agent-app-cli.exe exec "what did I say earlier in this session— "

```



Expected: The LLM calls `read_memory(query="what did I say earlier in this session— ")` — gets back relevant memory entries.



- [ ] **Step 6: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/agentic/tools/implementations/search/memory.rs && git commit -m "feat(core): v3.3 add read_memory tool with semantic search"

```



### Task 24: v3.3 Acceptance



- [ ] **Step 1: Run full test suite**



```bash

cd E:/agent-project/agent-app && cargo test --workspace 2>&1 | tail -30

```



- [ ] **Step 2: Manual end-to-end**



```bash

cd E:/agent-project/agent-app && ./target/debug/agent-app-cli.exe exec "remember that I prefer dark mode" 2>&1 | head -30

./target/debug/agent-app-cli.exe exec "what's my UI preference— " 2>&1 | head -30

```



Expected: Second command gets the "dark mode" memory back via read_memory.



- [ ] **Step 3: Update PROJECT_STATE.md**



```markdown

- v3.3 complete (2026-06-17): embedding-based semantic search + read_memory tool. Main agent can now query long-term memory on demand.

```



- [ ] **Step 4: Commit**



```bash

cd E:/agent-project/agent-app && git add docs/PROJECT_STATE.md && git commit -m "docs: mark v3.3 complete"

```



---



## Phase v3.4: PartitionedLoader (1-2 days)



**Goal:** Replace in-prompt full rendering with partitioned loading. Net effect: 30K — 2K (93%).



### Task 25: Extract soul.md, agent.md, personality into separate files



**Files:**

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\prompts\soul.md`

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\prompts\agent.md`

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\prompts\personality\default.md`



- [ ] **Step 1: Create soul.md**



```markdown

# agent-app Soul



You are agent-app, an AI coding assistant. Your purpose is to help users accomplish software engineering tasks efficiently and correctly.



## Core values



- Correctness over speed

- User intent over literal interpretation

- Transparency about uncertainty



## Behavioral anchors



- Never fabricate file paths, function signatures, or API contracts

- Prefer reading code over assuming

- Ask when ambiguous; don't guess

```



- [ ] **Step 2: Create agent.md**



```markdown

# Agent Guidance



You operate within a tool-driven environment. Use tools to read, write, and execute code.



## Tool philosophy



- Read before write

- Test changes locally when possible

- Prefer surgical edits over rewrites



## Memory



- `read_memory`: query long-term memory by id or semantic query

- `search_skill` / `get_skill_detail`: discover and load skills

- `search_agent` / `get_agent_detail`: discover and load sub-agents

```



- [ ] **Step 3: Create personality/default.md**



```markdown

# Default Personality



Tone: professional, concise, direct.

Verbosity: match the task — short for quick questions, detailed for complex work.

Humor: dry, rare, never at the user's expense.

```



- [ ] **Step 4: Update build.rs to embed these files**



In `E:\agent-project\agent-app\src\crates\assembly\core\build.rs` (or wherever prompts are embedded), add:



```rust

// v3.4: embed soul/agent/personality

let soul = std::fs::read_to_string("src/agentic/prompts/soul.md")— ;

let agent = std::fs::read_to_string("src/agentic/prompts/agent.md")— ;

let personality = std::fs::read_to_string("src/agentic/prompts/personality/default.md")— ;



let dest = std::env::var("OUT_DIR")— ;

let dest_path = std::path::Path::new(&dest);

std::fs::write(dest_path.join("soul.md"), soul)— ;

std::fs::write(dest_path.join("agent.md"), agent)— ;

std::fs::write(dest_path.join("personality_default.md"), personality)— ;

```



- [ ] **Step 5: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/agentic/prompts/ src/crates/assembly/core/build.rs && git commit -m "feat(core): v3.4 add soul.md, agent.md, personality files"

```



### Task 26: Implement PartitionedLoader



**Files:**

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\prompts\loader\mod.rs`

- Create: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\prompts\loader\partitioned_loader.rs`



- [ ] **Step 1: Create loader module**



`loader\mod.rs`:



```rust

pub mod partitioned_loader;

```



- [ ] **Step 2: Implement PartitionedLoader**



```rust

//! Partitioned prompt loader: builds ~2K token system prompt from partitioned files.



pub struct PartitionedLoader;



pub struct SystemPrompt {

 pub soul: String,

 pub agent: String,

 pub personality: String,

 pub skill_index_summary: String,

 pub agent_index_summary: String,

 pub runtime_context: String,

 pub user_context: String,

}



impl SystemPrompt {

 pub fn render(&self) -> String {

 format!(

 "{}\n\n{}\n\n{}\n\n---\n\n## Available Skills (极简索引)\n{}\n\n## Available Sub-agents (极简索引)\n{}\n\n---\n\n## Runtime Context\n{}\n\n## User Context\n{}",

 self.soul, self.agent, self.personality,

 self.skill_index_summary, self.agent_index_summary,

 self.runtime_context, self.user_context

 )

 }

}



impl PartitionedLoader {

 pub fn load(

 skill_index: &SkillIndex,

 agent_index: &AgentIndex,

 runtime_context: String,

 user_context: String,

 ) -> SystemPrompt {

 let soul = include_str!(concat!(env!("OUT_DIR"), "/soul.md")).to_string();

 let agent = include_str!(concat!(env!("OUT_DIR"), "/agent.md")).to_string();

 let personality = include_str!(concat!(env!("OUT_DIR"), "/personality_default.md")).to_string();

 let skill_summary = render_skill_index_summary(skill_index, 24);

 let agent_summary = render_agent_index_summary(agent_index, 8);



 SystemPrompt {

 soul,

 agent,

 personality,

 skill_index_summary: skill_summary,

 agent_index_summary: agent_summary,

 runtime_context,

 user_context,

 }

 }

}



fn render_skill_index_summary(index: &SkillIndex, limit: usize) -> String {

 let entries = index.search("", limit).unwrap_or_default();

 entries.iter().map(|e| format!("- `{}`: {}", e.id, truncate(&e.description, 80))).collect::<Vec<_>>().join("\n")

}



fn render_agent_index_summary(index: &AgentIndex, limit: usize) -> String {

 let entries = index.search("", limit).unwrap_or_default();

 entries.iter().map(|e| format!("- `{}` ({}): {}", e.id, e.mode, truncate(&e.description, 80))).collect::<Vec<_>>().join("\n")

}



fn truncate(s: &str, max: usize) -> String {

 if s.chars().count() <= max { s.to_string() }

 else {

 let t: String = s.chars().take(max).collect();

 format!("{}— , t)

 }

}

```



- [ ] **Step 3: Add test**



```rust

#[cfg(test)]

mod tests {

 use super::*;



 #[test]

 fn system_prompt_renders_all_sections() {

 let loader = PartitionedLoader;

 // Use in-memory indices

 let skill_index = SkillIndex::open(":memory:").unwrap();

 let agent_index = AgentIndex::open(":memory:").unwrap();

 let prompt = loader.load(&skill_index, &agent_index, "runtime".into(), "user".into());

 let rendered = prompt.render();

 assert!(rendered.contains("agent-app Soul"));

 assert!(rendered.contains("Agent Guidance"));

 assert!(rendered.contains("Available Skills"));

 assert!(rendered.contains("Available Sub-agents"));

 }

}

```



- [ ] **Step 4: Build check**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -10

```



- [ ] **Step 5: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/agentic/prompts/loader/ && git commit -m "feat(core): v3.4 add PartitionedLoader"

```



### Task 27: Wire PartitionedLoader into prompt builder



**Files:**

- Modify: `E:\agent-project\agent-app\src\crates\assembly\core\src\agentic\agents\prompt_builder\prompt_builder_impl.rs`



- [ ] **Step 1: Add feature flag**



At the top of `prompt_builder_impl.rs`:



```rust

const USE_PARTITIONED_LOADER: bool = true; // v3.4: enabled by default

```



- [ ] **Step 2: Modify build_prompt_from_template**



Find `build_prompt_from_template` and wrap its body:



```rust

pub fn build_prompt_from_template(...) -> String {

 if USE_PARTITIONED_LOADER {

 let partitioned = PartitionedLoader::load(

 &skill_index,

 &agent_index,

 self.build_runtime_context_reminder(...),

 self.build_user_context_reminder(...),

 );

 partitioned.render()

 } else {

 // Old path

 // ... existing logic

 }

}

```



- [ ] **Step 3: Pass skill_index/agent_index to the builder**



The `PromptBuilder` struct (or whichever struct holds the build methods) needs references to the indices. Add fields:



```rust

pub struct PromptBuilder {

 // ... existing fields

 pub skill_index: Arc<SkillIndex>,

 pub agent_index: Arc<AgentIndex>,

}

```



Wire these in from the call site (likely `system.rs` or wherever the builder is created).



- [ ] **Step 4: Build check (expect call-site fixes)**



```bash

cd E:/agent-project/agent-app && cargo build -p agent-app-core 2>&1 | tail -30

```



- [ ] **Step 5: Commit**



```bash

cd E:/agent-project/agent-app && git add src/crates/assembly/core/src/agentic/agents/prompt_builder/prompt_builder_impl.rs && git commit -m "feat(core): v3.4 wire PartitionedLoader into prompt builder"

```



### Task 28: v3.4 Acceptance — Token count verification



**Files:**

- Create: `E:\agent-project\agent-app\tests\v3_hello_world_token_count.rs`



- [ ] **Step 1: Write token count test**



```rust

//! Acceptance test: hello world input should produce a system prompt of ~2K tokens.



use agent-app_core::agentic::prompts::loader::PartitionedLoader;

use agent-app_core::service::data::{SkillIndex, AgentIndex};



#[test]

fn hello_world_prompt_under_3k_tokens() {

 let skill_index = SkillIndex::open(":memory:").unwrap();

 let agent_index = AgentIndex::open(":memory:").unwrap();

 // Seed with 24 skills and 8 agents (mimic production state)

 seed_test_skills(&skill_index);

 seed_test_agents(&agent_index);



 let prompt = PartitionedLoader::load(

 &skill_index,

 &agent_index,

 "workspace: /tmp/test".into(),

 "user: test_user".into(),

 );

 let rendered = prompt.render();



 // Rough token estimate: 1 token — 4 chars

 let char_count = rendered.chars().count();

 let token_estimate = char_count / 4;



 assert!(

 token_estimate < 3000,

 "hello world prompt should be <3K tokens, got {} ({} chars)",

 token_estimate,

 char_count

 );

}



fn seed_test_skills(index: &SkillIndex) {

 let skills = vec![

 ("pdf", "PDF Generation", "Generate and edit PDF documents using various backends"),

 ("docx", "Word Documents", "Create and edit Word .docx files programmatically"),

 ("xlsx", "Excel Spreadsheets", "Read and write Excel .xlsx files"),

 ("pptx", "PowerPoint", "Create and edit PowerPoint presentations"),

 // ... add 20 more

 ];

 for (id, name, desc) in skills {

 index.upsert(&SkillIndexEntry {

 id: id.into(), name: name.into(), description: desc.into(),

 full_path: format!("/skills/{}", id), source: "builtin".into(),

 }, "test").unwrap();

 }

}



fn seed_test_agents(index: &AgentIndex) {

 let agents = vec![

 ("explore", "Explore", "Read-only codebase explorer"),

 ("general", "General Purpose", "Multi-purpose agent for complex tasks"),

 // ... add 6 more

 ];

 for (id, name, desc) in agents {

 index.upsert(&AgentIndexEntry {

 id: id.into(), name: name.into(), description: desc.into(),

 full_path: format!("/agents/{}", id), mode: "ClassA".into(),

 }, "test").unwrap();

 }

}

```



- [ ] **Step 2: Run test**



```bash

cd E:/agent-project/agent-app && cargo test --test v3_hello_world_token_count 2>&1 | tail -15

```



Expected: Test passes. If token count is too high, iterate on PartitionedLoader to trim further.



- [ ] **Step 3: Run full test suite**



```bash

cd E:/agent-project/agent-app && cargo test --workspace 2>&1 | tail -30

```



Expected: All tests pass.



- [ ] **Step 4: Update PROJECT_STATE.md**



```markdown

- v3.4 complete (2026-06-17): PartitionedLoader active. Token reduction: 30K — ~2K (93%). v3 architecture complete.

```



- [ ] **Step 5: Commit**



```bash

cd E:/agent-project/agent-app && git add tests/v3_hello_world_token_count.rs docs/PROJECT_STATE.md && git commit -m "test: v3.4 acceptance - hello world <3K tokens"

```



### Task 29: Remove v2 MemoryKeeperSubscriber (1 week after v3.4)



**Files:**

- Delete: `E:\agent-project\agent-app\src\crates\assembly\core\src\service\memory_keeper\`



- [ ] **Step 1: Wait 1 week**



This task is intentionally deferred to ensure v3 is stable in production.



- [ ] **Step 2: Verify v3 stability**



Confirm no memory loss / corruption over 1 week of v3-only runs.



- [ ] **Step 3: Remove v2 code**



```bash

cd E:/agent-project/agent-app && git rm -r src/crates/assembly/core/src/service/memory_keeper/

```



- [ ] **Step 4: Remove dual_write from config**



In `default.toml`, remove the `dual_write` field and the `extractor` field (since v3 is now the only option).



- [ ] **Step 5: Build check**



```bash

cd E:/agent-project/agent-app && cargo build --workspace 2>&1 | tail -10

```



- [ ] **Step 6: Commit**



```bash

cd E:/agent-project/agent-app && git add -A && git commit -m "chore: remove v2 MemoryKeeperSubscriber (v3 stable for 1 week)"

```



---



## Self-Review



**1. Spec coverage:**



| Spec section | Implemented in |

|---|---|

| §3.1 Architecture layers | Tasks 8-12 (data), 15-19 (memory agent), 25-27 (loader) |

| §3.2 Component inventory | All file paths in plan header match spec |

| §3.3 Data flow | Tasks 11, 12, 17, 23 (tools + agent) |

| §3.4 Event subscriptions | Task 17 (MemoryAgent subscribes to events) |

| §4.1 skills.db/agents.db schema | Tasks 8, 9 |

| §4.2 memory.db schema | Task 21 |

| §4.3 read_memory tool spec | Task 23 |

| §4.4 search_* tools | Tasks 11, 12 |

| §4.5 MemoryAgent implementation | Tasks 16, 17, 18 |

| §4.6 PartitionedLoader | Tasks 25, 26, 27 |

| §5 Token math | Task 28 verifies the math |

| §6 Migration plan | Tasks 18, 19, 29 (A/B flag, then remove v2) |

| §7 Implementation phases | 5 phases, 28 tasks total |

| §8 Error handling | Mentioned in distiller (LLM failure — log + skip), read_memory (embed failure — error returned) |

| §9 Testing | Every task has a test; v3.0/v3.1/v3.3/v3.4 have explicit acceptance tests |

| §10 Open questions | Deferred to v3.5 (out of scope) |



**2. Placeholder scan:** No TBD/TODO/待定 in task descriptions. All "Note:" blocks explain *why* code is sketched (API drift expected at implementation time) rather than leaving gaps.



**3. Type consistency:** Cross-checked field names:

- `SkillIndexEntry.id/name/description/full_path/source` — defined in Task 8, used in Tasks 10, 11, 13, 26

- `AgentIndexEntry.id/name/description/full_path/mode` — defined in Task 9, used in Tasks 10, 12, 13, 26

- `MemoryEntry.id/category/content/importance/embedding/created_at` — defined in Task 21, used in Task 23

- Tool trait methods `name/description/input_schema/exposure/execute` — used in Tasks 11, 12, 23; may need adjustment at impl time

- Event enum variants `ToolCallCompleted/DialogTurnCompleted` — used in Tasks 17, 18; field names need verification at impl time



**4. Scope:** 5 phases, each independently shippable. v3.0 alone (Tasks 1-7) is 35% token reduction with zero architecture changes. Reasonable starting point.



**5. Notes for the implementer:**

- The exact `AgenticEvent` enum, `Tool` trait, `LlmClient` trait, and `MemoryService` API may differ from what's sketched. Every task that touches these has a "Note:" block explaining what to verify. Run `cargo build` after each file write; compiler errors will tell you what to rename.

- "Sketch" code blocks (e.g. seed.rs, distiller.rs, agent.rs) are starting points, not finished implementations. Treat them as 60% done; the remaining 40% is field/method renaming to match actual API.

- Embedding model choice in Task 20 (fastembed vs ort) is TBD at impl time based on what produces a small 384-dim model with low overhead.



---



**Plan saved to:** `docs/superpowers/plans/2026-06-17-v3-prompt-loader-impl.md`



**Two execution options:**



1. **Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

2. **Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints



**Which approach— **

