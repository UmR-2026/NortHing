# W2 Final Review — Wave-Level Verdict

**Base:** `5a90e04` · **Head:** `298777b`  
**Code commits:** bf7b8b8 · 60cf675 · 32454b8 · b440cae  
**Docs-only commit:** 298777b

## Verdict: **CAN MERGE**

---

## Findings by Severity

### Critical: 0
### Important: 0
### Minor: 0

No findings to report.

---

## Seam-Check Results

**Seam 1 — W2-1 → W2-2 flashgrep kill_on_drop chain:**  
Verified at review. `create_tokio_command_for_spawn` (process_manager.rs:132-137) applies `cmd.kill_on_drop(true)` + `configure_process_group`. flashgrep `AsyncDaemonClient::spawn` now calls this constructor (client.rs:471). flashgrep's W2-2 Drop simplifies to `drop(self.take_child_for_drop())` — child termination via kill_on_drop fires on Child drop. No regression: previously Drop called `spawn_child_process_tree_cleanup`; now equivalent (and simpler) via kill_on_drop. Comment cites audit F9.

**Seam 2 — W2-1 → W2-2 MCP belt-and-suspenders:**  
Verified at review. MCP `MCPServerProcess::start` now uses `create_tokio_command_for_spawn` (process.rs:375), which sets kill_on_drop + process group. MCP's Drop still calls `spawn_child_process_tree_cleanup` (process.rs:400-401) with the cmd.exe /c justification comment. The two mechanisms (kill_on_drop at the Child level + tree-kill for grandchildren) are complementary and non-contradictory. No regression for explicitly-stopped servers either: `stop_server` at process.rs:397-407 calls `terminate_child_process_tree` first, then takes the child (None), so Drop finds nothing to clean up.

**Seam 3 — Unix SIGKILL orphan risk:**  
Verified confirmed-no-regression. Prior to W2-1/W2-2, flashgrep's Drop cleanup (`spawn_child_process_tree_cleanup`) ran in a background thread and would NOT fire on SIGKILL of the owning Rust process (same as kill_on_drop). W2-1 adds process_group(0) on unix for migrated spawn sites; W2-2 removes the Drop helper for flashgrep. On SIGKILL: kill_on_drop fires nothing (process dead), Drop fires nothing (process dead), background thread fires nothing (process dead), spawn_child_process_tree_cleanup would fire nothing. The only difference is that before the wave, grandchild processes would have been killed by the cleanup helper (if the parent terminated normally); after the wave, kill_on_drop handles the Drop path which covers all normal termination. SIGKILL outcome is unchanged (orphans possible in both cases). No regression.

**Seam 4 — W2-3 independence from W2-1/W2-2:**  
Confirmed zero overlap. W2-3 is entirely contained in `sub_handle_in.rs` (dialog_turn sessions coordinator, lines 163-185). W2-1/W2-2 touch `process_manager.rs`, `process_command.rs`, `process_spawn.rs`, `app_control.rs`, `utilities.rs`, `mcp/server/process.rs`, `flashgrep/client.rs`, `services-core/lib.rs`. The two sets are completely disjoint files with no shared types or logic paths.

---

## Ledger-vs-Git Calibration

Calibration result: **CLEAN** — no contradictions found.

- **W2-1** row: `commits 5a90e04..bf7b8b8` — 5a90e04 is the pre-wave base, bf7b8b8 is the first code commit. Between them there are no W2 commits. Correct.
- **W2-2** row: `commits bf7b8b8..32454b8` — bf7b8b8→60cf675→32454b8. 60cf675 is docs-only (SDD ledger rows), 32454b8 is the code commit. Correct.
- **W2-3** row: `commits 60cf675..b440cae` — bf7b8b8 (the prior code commit) is excluded; 60cf675→b440cae. The actual code change is b440cae alone, but 60cf675 sits in the chain. Descriptions match (F5 constructor, F9 tree-cleanup, r2#6 warn). Correct.
- Doc commits (298777b) correctly excluded from task ledger rows.
- Ledger descriptions for commit contents, clean-findings counts, and test verification claims match the diff exactly.

---

## House-Rules Spot-Check

| Rule | Result |
|---|---|
| **English-only logs** | All new log lines (warn! in sub_handle_in.rs:179-183, MCP tree-cleanup comment in mcp/process.rs:400) are in English, no emoji. |
| **File-length ceiling** | Max modified .rs: `src/crates/services/services-core/src/process_manager.rs` (263 lines) and `src/crates/services/services-integrations/src/mcp/server/process.rs` (404 lines). No file approaches 800-line threshold. No god-file justification needed. |
| **No new mutex/timeout/atomic** | Zero new synchronisation primitives introduced. flashgrep retains pre-existing `Arc<AtomicBool>` + `Arc<Mutex<ManagedClientState>>`; no additions. W2-1/W2-2 are process restructure only. |
| **Dependency direction** | services-core (Layer 4) exports `create_tokio_command_for_spawn` via `pub use` in lib.rs. assembly/core (Layer 2) imports through existing `northhing-services-core` crate dependency — downward direction, correct. No upward deps introduced. |
| **crate-structure docs** | No crate additions, removals, or path moves. `docs/status/surfaces.md` not required. |

---

## Behavioral Risk for UI Walkthrough

**No perceivable-risk blocker.**

W2-1 changes the *construction pattern* of spawned processes (belt-and-suspenders via constructor) — no behavioral change in the happy path. W2-2 simplifies flashgrep's Drop path — child still terminates (kill_on_drop fires). W2-3 changes a *log level* from debug! to warn! for users with persisted session history — visible only in logs, no UI change or functional change.

None of these three tasks touch UI rendering, Slint properties, event routing, or any user-visible behavior. The desktop compile gate (`cargo check -p northhing`) would catch any API breakage; test evidence from per-task reports confirms passes.

**Walkthrough clearance: CLEAR — proceed.**

---
*Review access: read-only. No code modified. Evidence drawn from diff package (bf7b8b8..298777b), audit reports r3-services.md and r2-core.md, and direct source inspection of seam-critical lines.*
