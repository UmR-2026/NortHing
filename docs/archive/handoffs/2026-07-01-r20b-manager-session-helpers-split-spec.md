# R20b Spec — bitfun-acp manager_session_helpers sub-domain split (close QClaw R20a P1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. This spec follows the R20a spec format + writing-plans task granularity standard.

**Goal:** Split `acp/client/manager_session_helpers.rs` (405 canonical lines, +67% over QClaw 242 tolerance) into 3 sibling sub-domain files to close the QClaw R20a P1 D-deviation recommendation. Pure structural split — no behavior change, no method-body refactoring.

**Architecture:** 3-way sub-domain split on the 16 free fns (no inherent methods on `AcpClientService` here, all are sibling-consumed free functions). Each new file owns one semantic sub-domain. NO facade. NO forwarder. Each call site updates `use super::manager_session_helpers::{...}` to the new sub-domain module path. R20a's "always `pub` default" rule applied (no cross-crate callers exist, but 4 sibling files + the 3 new siblings need to reach these helpers via `use super::...`).

**Tech Stack:** Rust 2021+, `bitfun-acp` (interfaces layer), M2.7-highspeed agent, cargo 1.85+, canonical line measurement via `[System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8).Count` (PowerShell) or `wc -l` (bash).

---

## 1. Background

### 1.1 Why this split

`acp/client/manager_session_helpers.rs` was created in R19 (`35790ad` "Merge branch 'impl/r19-acp-manager-split'") to host 16 free fns that were extracted from the `manager_session.rs` god-object during the R19 split. At that time, it landed at 405 canonical lines.

QClaw R20a review (8.8/10 APPROVE) flagged this as a **P1 D-deviation**:

> R20b: `manager_session_helpers.rs` 405 → 2-3 files. Major D-deviation (+67% over 242). 16 free fns.

Kimi R20a review independently confirmed: same P1 status. Both reviewers agree: 2-3 files, no facade.

### 1.2 Sub-domain analysis (4 natural clusters, must collapse to 3)

The 16 free fns cluster into 4 natural sub-domains (see §2 for line spans):

| Sub-domain | Function names | Span | Src lines | Why coherent |
|---|---|---:|---:|---|
| A. **Identity/Key/Status** | `parse_config_value`, `build_session_key`, `session_client_connection_id`, `aggregate_client_status` | L52-104 | ~52 | All produce strings / config / status from primitives; no session state mutation |
| B. **Session response builders** | `new_session_response_from_load`, `new_session_response_from_resume` | L105-126 | ~22 | Both build `NewSessionResponse` / `ResumeSessionResponse` from `AcpRemoteSession` |
| C. **Turn data drain** | `drain_pending_turn_updates`, `read_turn_to_string`, `drain_pending_turn_text`, `append_agent_text` | L127-260 | ~133 | All about pulling pending turn events out of session state and rendering them to text/Vec for stream consumers |
| D. **Session state mutations** | `drain_pending_session_metadata_updates`, `discard_pending_session_updates_if_needed`, `update_session_from_events`, `update_session_context_usage`, `update_session_available_commands`, `update_session_config_options` | L261-405 | ~144 | All about consuming ACP stream events and updating session state |

Constraint check:
- A + B + C + D split into 4 files works (all under 242 cap) but the spec asked for 2-3
- A + B + C = 207 src lines + 30 headers ≈ 237 canonical (under 242 ✓) — but mixes identity primitives with response builders (semantically weak)
- A + C + D = 329 src + 30 = 359 canonical — over 242 ✗
- B + C + D = 299 src + 30 = 329 — over 242 ✗
- A + B + D = 218 src + 30 = 248 — 6 lines over 242 ✗
- B + C = 155 src + 30 = 185 ✓ ; A = 80 ✓ ; D = 175 ✓ — but A alone is a thin file
- A alone is semantically fine as a tiny file (4 small config fns ≈ 50 src lines, all primitive)
- A + C = 185 src + 30 = 215 ✓ ; B = 52 ✓ ; D = 175 ✓ — but A+C is "identity + turn drain" which mixes session metadata lifecycle (A produces a key once) with turn data drain (C runs every turn)

**Decision: 3 files via grouping A + C together, with B and D as separate files.**

Rationale: A is a self-contained 4-fns config module, B is a 2-fns response builder, C is a 4-fns turn data drain, D is a 6-fns session state mutator. The cleanest 3-way split that respects "files change together should live together" is:

- File 1: **A** (identity/key/status) — 4 fns, 52 src + ~30 headers = ~82 canonical
- File 2: **B + C** (session response + turn data drain) — 6 fns, 155 src + ~30 headers = ~185 canonical
- File 3: **D** (session state mutations) — 6 fns, 144 src + ~30 headers = ~175 canonical

The B+C grouping is defensible: both deal with "the act of getting data out of an existing session in a particular shape" — B builds a response-shaped snapshot (load/resume response), C drains pending turn events into text representation. Both are read-side transforms of session state, while A is identity primitives and D is write-side session state mutation. The 4-way sub-domain (A/B/C/D) is documented in §2 but the 3-way physical split is A / (B+C) / D.

QClaw + Kimi "2-3 files" range: 3 satisfies.

### 1.3 Caller map (before split)

External (in-crate, sibling files) callers per fn:

| Fn | Called from | Notes |
|---|---|---|
| `parse_config_value` | `manager.rs:101`, `manager_config.rs:26` | 2 callers |
| `build_session_key` | `manager_cancel.rs:20`, `manager_session_resolve.rs:35` | 2 callers |
| `session_client_connection_id` | `manager_cancel.rs:20`, `manager_connection.rs:28` | 2 callers |
| `aggregate_client_status` | `manager_config.rs:26` | 1 caller |
| `new_session_response_from_load` | `manager_session_resolve.rs:35` | 1 caller |
| `new_session_response_from_resume` | `manager_session_resolve.rs:35` | 1 caller |
| `drain_pending_turn_updates` | `manager_prompt.rs:20` | 1 caller |
| `read_turn_to_string` | `manager_prompt.rs:20` | 1 caller |
| `drain_pending_turn_text` | internal only (called by `read_turn_to_string` and itself) | file-local |
| `append_agent_text` | internal only (called by `drain_pending_turn_text` and `drain_pending_turn_text` recursively) | file-local |
| `drain_pending_session_metadata_updates` | `manager_session_read.rs:28`, `manager_session_resolve.rs:35` | 2 callers (note: `manager_session_read.rs` + `manager_session_resolve.rs` are R20a-spawned; **R20a is NOT merged to main yet**, so this caller exists in `impl/r20a-manager-session-split` branch only; the R20b branch is forked from main `f579c71` where R20a doesn't exist yet, so this fn has 0 callers in R20b's branch) |
| `discard_pending_session_updates_if_needed` | `manager_prompt.rs:20` | 1 caller |
| `update_session_from_events` | `manager_prompt.rs:20` | 1 caller |
| `update_session_context_usage` | internal only (called by `update_session_from_events`) | file-local |
| `update_session_available_commands` | internal only (called by `update_session_from_events`) | file-local |
| `update_session_config_options` | internal only (called by `update_session_from_events`) | file-local |

Cross-crate callers: **0** (verified via `git grep 'manager_session_helpers::' -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/'` — no hits).

Caller-side impact: 7 sibling files need `use super::manager_session_helpers::{...}` updated to the new module paths. R20a-spawned `manager_session_read.rs` and `manager_session_resolve.rs` also need updating, but those files do not exist in this worktree (R20b is forked from main `f579c71` where R20a impl is not yet merged). The R20a branch will need to rebase / re-apply the same `use` change later; that is the user / R20a merger problem, not R20b.

---

## 2. File structure (post-split)

### 2.1 New files (3) and changes (1)

| File | Action | Approx canonical | Sub-domain | Method count | Visibility default |
|---|---|---:|---|---:|---|
| `src/crates/interfaces/acp/src/client/manager_session_helpers.rs` | **DELETED** (all bodies moved out) | 0 | n/a | 0 | n/a |
| `src/crates/interfaces/acp/src/client/manager_session_helpers_identity.rs` | **NEW** | ~82 | A | 4 (free fns) | `pub fn` |
| `src/crates/interfaces/acp/src/client/manager_session_helpers_session_response.rs` | **NEW** | ~185 | B + C | 6 (free fns) | `pub fn` |
| `src/crates/interfaces/acp/src/client/manager_session_helpers_session_state.rs` | **NEW** | ~175 | D | 6 (free fns) | `pub fn` (3) + private `fn` (3, file-local helpers) |
| `src/crates/interfaces/acp/src/client/mod.rs` | **MODIFIED** (replace `mod manager_session_helpers;` with 3 new mod declarations) | +2 lines | n/a | n/a | private `mod` (matching R19/R20a sibling pattern) |

**Total: 3 new files, 1 modified file, 1 deleted file. Net +~440 canonical (vs 405 original, +35 from new R20b headers + per-file imports).**

### 2.2 Visibility rule (R19 lesson applied, R20a spec deviation followup)

- **Cross-crate callers: 0** — so `pub` is sufficient everywhere
- **Sibling files (in `src/crates/interfaces/acp/src/client/`) need to call into the new helper modules** — they use `use super::manager_session_helpers_identity::{...}` etc. For this to compile, the new modules must be at minimum `pub(super)` (i.e. visible to the parent `client` module's siblings). Per R19 + R20a lessons, **default to `pub fn`** unless cross-crate evidence says otherwise. The 0 cross-crate callers + the sibling-visibility need = `pub fn` is correct.
- **3 file-local helpers** (`drain_pending_turn_text`, `append_agent_text`, `update_session_context_usage`, `update_session_available_commands`, `update_session_config_options` — 5 actually, not 3 — counted above) are called only from inside the same file. They should be plain `fn` (no `pub` prefix) to minimize the public surface.
- **Producer's per-fn visibility judgment** is allowed within the file they're working on, but the default template is: `pub fn` for externally-called helpers, plain `fn` for file-local helpers. The reviewer will check this.

### 2.3 Per-file naming

- `manager_session_helpers_identity.rs` — handles A (identity/key/status)
- `manager_session_helpers_session_response.rs` — handles B + C (session response + turn data drain)
- `manager_session_helpers_session_state.rs` — handles D (session state mutations)

Naming rationale: keep the `manager_session_helpers_` prefix so grep-ability and module discovery remain clusterable (e.g. `git grep -l 'manager_session_helpers_' src/crates/interfaces/acp/` finds all helpers). The suffix carries the sub-domain, not the round (i.e. NOT `_v2_` or `_split_`). This matches R19 / R20a precedent (`manager_session_lifecycle.rs`, `manager_session_resolve.rs`, `manager_session_read.rs`).

---

## 3. Tasks (bite-sized, with full code)

Each task produces a self-contained, committable change. DRY: if a task is "do X to file Y", the file Y is shown in full or with the exact diff. No "similar to Task N" placeholders.

### Task 1: Pre-flight — verify baseline + write the spec branch

**Files:**
- Read: `src/crates/interfaces/acp/src/client/manager_session_helpers.rs` (405 canonical)
- Read: `src/crates/interfaces/acp/src/client/mod.rs` (line 14 has `mod manager_session_helpers;`)

**Steps:**

- [ ] **Step 1.1: Verify worktree state**

```bash
git log --oneline -1
# Expected: f579c71 docs(spec): R20a close Critical D-deviation (manager_session.rs 486 → 2 files)
git status
# Expected: clean working tree
git config core.autocrlf
# Expected: false (set by R20a worktree init; verify it's still false after `git worktree add`)
```

If `core.autocrlf` is NOT `false`, set it:
```bash
git config core.autocrlf false
```

- [ ] **Step 1.2: Re-verify canonical line count + content sanity**

PowerShell:
```powershell
$path = 'src/crates/interfaces/acp/src/client/manager_session_helpers.rs'
$canon = [System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8).Count
# Expected: 405
$bytes = [System.IO.File]::ReadAllBytes($path)
$bom = if ($bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) { 'YES' } else { 'NO' }
$crlf = 0; for ($i = 0; $i -lt $bytes.Length - 1; $i++) { if ($bytes[$i] -eq 0x0D -and $bytes[$i+1] -eq 0x0A) { $crlf++ } }
# Expected: BOM=NO CRLF=0
```

Bash (cross-check):
```bash
wc -l src/crates/interfaces/acp/src/client/manager_session_helpers.rs
# Expected: 405 manager_session_helpers.rs
file src/crates/interfaces/acp/src/client/manager_session_helpers.rs
# Expected: UTF-8 (or ASCII) text, NO "with CRLF" mention
```

- [ ] **Step 1.3: Verify cargo baseline (no R20a-related code in this worktree)**

```bash
cargo check -p bitfun-acp 2>&1 | grep -cE '^error\['
# Expected: 0 (or 1 if R20a-introduced 2 E0624 are not in this worktree; verify what shows up)
cargo check -p northhing-acp 2>&1 | tail -5
# Expected: "Finished `dev` profile ..." line, no "error:" summary
```

Note: R20a was on `impl/r20a-manager-session-split` branch and is NOT merged to main. R20b is forked from main `f579c71` which has the R20a spec doc but not the impl. The `manager_session_helpers.rs` file in this worktree is the un-touched main version.

- [ ] **Step 1.4: Confirm branch + commit spec file**

```bash
git checkout -b impl/r20b-manager-session-helpers-split
# Branch already created by worktree add, but confirm
git branch --show-current
# Expected: impl/r20b-manager-session-helpers-split
git add docs/handoffs/2026-07-01-r20b-manager-session-helpers-split-spec.md
git commit -m "docs(spec): R20b close QClaw R20a P1 D-deviation (manager_session_helpers.rs 405 → 3 files)"
```

### Task 2: Pre-emptively extend timeout at dispatch (R19 lesson)

**Files:** none (worktree-level config)

**Steps:**

- [ ] **Step 2.1: When dispatching the implementation plan, immediately extend the team plan timeout**

Per R19 standing rule ("Extend-timeout at dispatch for >2000 line splits") + R20a retrospective ("R20a was 486 lines and finished in 37 min, R20b is 405 lines so similar timing expected — 30 min cap should suffice, but pre-emptively extend +30 min"):

After `mavis team plan run ... --no-wait <plan-id>`:
```bash
mavis team plan extend-timeout <plan-id> <task-id> --minutes 60
```

The producer is expected to finish in ~30 min; the +30 extension gives a 60-min window. This is a reactive mitigation against the R19 5-pass take-over pattern.

### Task 3: Create `manager_session_helpers_identity.rs` (file A)

**Files:**
- Create: `src/crates/interfaces/acp/src/client/manager_session_helpers_identity.rs`

**Steps:**

- [ ] **Step 3.1: Create the new file with sub-domain A content (verbatim from main)**

Use the exact code below. The 4 free fns are moved verbatim from `src/crates/interfaces/acp/src/client/manager_session_helpers.rs:52-104`. No body changes, no signature changes, no import reorder.

```rust
// R20b split: ACP session identity / key / status helpers.
// File: src/crates/interfaces/acp/src/client/manager_session_helpers_identity.rs
// Origin: manager_session_helpers.rs (405 lines, QClaw R20a P1 D-deviation
//        +67% over QClaw 242 tolerance)
// Mavis fix: R20b split helpers.rs into 3 files (this + session_response +
//        session_state) to close the 242 line cap. Sub-domain A: identity
//        primitives (4 free fns, ~52 src lines).
// R20b sibling: manager_session_helpers_session_response.rs (B+C sub-domain)
//             manager_session_helpers_session_state.rs (D sub-domain)
// R19 sibling files:
//             manager_config.rs
//             manager_cancel.rs
//             manager_connection.rs
//             manager_session_resolve.rs (in impl/r20a-manager-session-split;
//                                          rebase / re-apply will be needed when
//                                          R20a is merged; not in this worktree)
//             manager_prompt.rs (none of these 4 fns are used here; listed
//                                for grep-ability of the helpers cluster)
//
// All method bodies are moved verbatim from main. No behavior change.

use super::config::{AcpClientConfigFile, AcpClientStatus};
use northhing_core::util::errors::NortHingResult;
use std::path::Path;

pub fn parse_config_value(value: serde_json::Value) -> NortHingResult<AcpClientConfigFile> {
    serde_json::from_value(value).map_err(|err| {
        northhing_core::util::errors::NortHingError::Validation(format!(
            "Failed to parse ACP config value: {err}"
        ))
    })
}

pub fn build_session_key(northhing_session_id: &str, client_id: &str, cwd: &Path) -> String {
    format!("{northhing_session_id}:{client_id}:{}", cwd.display())
}

pub fn session_client_connection_id(client_id: &str, northhing_session_id: &str) -> String {
    format!("{client_id}:{northhing_session_id}")
}

pub fn aggregate_client_status(statuses: &[AcpClientStatus]) -> AcpClientStatus {
    if statuses.is_empty() {
        return AcpClientStatus::default();
    }
    let mut aggregated = statuses[0].clone();
    for status in &statuses[1..] {
        match (&aggregated, status) {
            (AcpClientStatus::Error { .. }, _) | (_, AcpClientStatus::Error { .. }) => {
                aggregated = status.clone();
            }
            _ => {}
        }
    }
    aggregated
}
```

- [ ] **Step 3.2: Verify file canonical line count**

PowerShell:
```powershell
$path = 'src/crates/interfaces/acp/src/client/manager_session_helpers_identity.rs'
[System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8).Count
# Expected: ~80 (cap 242, -67% headroom)
```

- [ ] **Step 3.3: Commit (NOT a separate commit — fold into the final impl commit)**

Hold off. The R20a work pattern was a single `refactor(bitfun-acp):` commit at the end. We'll commit everything together in Task 7.

### Task 4: Create `manager_session_helpers_session_response.rs` (file B + C)

**Files:**
- Create: `src/crates/interfaces/acp/src/client/manager_session_helpers_session_response.rs`

**Steps:**

- [ ] **Step 4.1: Create the new file with sub-domain B + C content (verbatim from main)**

Use the exact code below. The 6 free fns are moved verbatim from `src/crates/interfaces/acp/src/client/manager_session_helpers.rs:105-260`. The 2 B-fns (response builders) are public, the 4 C-fns split: 2 publicly called (`drain_pending_turn_updates`, `read_turn_to_string`) + 2 file-local (`drain_pending_turn_text`, `append_agent_text`).

```rust
// R20b split: ACP session response + turn data drain helpers.
// File: src/crates/interfaces/acp/src/client/manager_session_helpers_session_response.rs
// Origin: manager_session_helpers.rs (405 lines, QClaw R20a P1 D-deviation)
// Mavis fix: R20b sub-domain B (2 fns: response builders from AcpRemoteSession)
//        + sub-domain C (4 fns: turn data drain, 2 external + 2 file-local).
// R20b sibling: manager_session_helpers_identity.rs (A sub-domain)
//             manager_session_helpers_session_state.rs (D sub-domain)
// R19 sibling files:
//             manager_session_resolve.rs (caller of B fns; lives in
//                                          impl/r20a-manager-session-split
//                                          branch, not in this worktree)
//             manager_prompt.rs (caller of C fns)
//
// All method bodies are moved verbatim from main. No behavior change.

use super::manager::AcpRemoteSession;
use super::stream::{AcpClientStreamEvent, AcpStreamRoundTracker, AcpToolCallTracker};
use agent_client_protocol::schema::{
    LoadSessionResponse, NewSessionResponse, ResumeSessionResponse,
};
use std::time::{Duration, Instant};
use tracing::warn;

// =================================================================
// Sub-domain B: Session response builders (2 pub fns)
// =================================================================

pub fn new_session_response_from_load(
    session: &AcpRemoteSession,
) -> NortHingResult<NewSessionResponse> {
    // (verbatim body from manager_session_helpers.rs:105-115)
    let mut response = NewSessionResponse::default();
    response.session_id = session.session_id().to_string().into();
    Ok(response)
}

pub fn new_session_response_from_resume(
    session: &AcpRemoteSession,
) -> NortHingResult<ResumeSessionResponse> {
    // (verbatim body from manager_session_helpers.rs:116-126)
    let mut response = ResumeSessionResponse::default();
    response.session_id = session.session_id().to_string().into();
    Ok(response)
}

// =================================================================
// Sub-domain C: Turn data drain (2 pub + 2 file-local fns)
// =================================================================

pub async fn drain_pending_turn_updates<F>(
    session: &mut AcpRemoteSession,
    on_event: F,
) -> NortHingResult<()>
where
    F: FnMut(AcpClientStreamEvent),
{
    // (verbatim body from manager_session_helpers.rs:127-178)
    // ... (full body preserved verbatim from main)
}

pub async fn read_turn_to_string(session: &mut AcpRemoteSession) -> NortHingResult<String> {
    // (verbatim body from manager_session_helpers.rs:179-208)
    // ... (full body preserved verbatim from main)
}

async fn drain_pending_turn_text(
    session: &mut AcpRemoteSession,
    tool_call_tracker: &mut AcpToolCallTracker,
    output: &mut String,
) -> NortHingResult<()> {
    // (verbatim body from manager_session_helpers.rs:209-252)
    // ... (full body preserved verbatim from main, called by read_turn_to_string
    //      and self-recursive for sub-event draining)
}

fn append_agent_text(output: &mut String, events: Vec<AcpClientStreamEvent>) {
    // (verbatim body from manager_session_helpers.rs:253-260)
    // ... (full body preserved verbatim from main, called by
    //      drain_pending_turn_text twice: once at outer drain, once at inner
    //      recursive call when sub-events are appended)
}
```

> **Producer note:** The bodies shown as `// ... (verbatim body ...)` are placeholders for the producer's actual copy-paste. The producer MUST copy the FULL function body verbatim from `src/crates/interfaces/acp/src/client/manager_session_helpers.rs:127-260` (the 4 C fns) and `:105-126` (the 2 B fns). The reviewer (Kimi) will diff this file against main to verify zero behavior change.

- [ ] **Step 4.2: Verify file canonical line count + ≤5 long lines tolerance (R18 rule)**

```bash
# PowerShell
$path = 'src/crates/interfaces/acp/src/client/manager_session_helpers_session_response.rs'
$canon = [System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8).Count
# Expected: ~185 (cap 242, -24% headroom)

# Long-line check (≤5 per file is tolerable per R18 relaxed rule)
$lines = [System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8)
$longLines = 0
for ($i = 0; $i -lt $lines.Length; $i++) {
    if ($lines[$i].Length -gt 120) { $longLines++ }
}
# Expected: ≤5
```

### Task 5: Create `manager_session_helpers_session_state.rs` (file D)

**Files:**
- Create: `src/crates/interfaces/acp/src/client/manager_session_helpers_session_state.rs`

**Steps:**

- [ ] **Step 5.1: Create the new file with sub-domain D content (verbatim from main)**

Use the exact code below. The 6 free fns are moved verbatim from `src/crates/interfaces/acp/src/client/manager_session_helpers.rs:261-405`. 3 are pub (`drain_pending_session_metadata_updates`, `discard_pending_session_updates_if_needed`, `update_session_from_events`), 3 are file-local plain `fn` (`update_session_context_usage`, `update_session_available_commands`, `update_session_config_options`).

```rust
// R20b split: ACP session state mutation helpers.
// File: src/crates/interfaces/acp/src/client/manager_session_helpers_session_state.rs
// Origin: manager_session_helpers.rs (405 lines, QClaw R20a P1 D-deviation)
// Mavis fix: R20b sub-domain D (6 fns: session state mutations from event
//        stream; 3 pub + 3 file-local helpers).
// R20b sibling: manager_session_helpers_identity.rs (A sub-domain)
//             manager_session_helpers_session_response.rs (B+C sub-domain)
// R19 sibling files:
//             manager_prompt.rs (caller of the 3 pub fns)
//             manager_session_read.rs + manager_session_resolve.rs
//                (in impl/r20a-manager-session-split; rebase needed when
//                 R20a is merged; not in this worktree)
//
// All method bodies are moved verbatim from main. No behavior change.

use super::manager::AcpRemoteSession;
use super::stream::AcpClientStreamEvent;
use agent_client_protocol::schema::SessionConfigOption;
use std::time::Duration;
use tracing::info;

pub async fn drain_pending_session_metadata_updates(
    session: &mut AcpRemoteSession,
) -> NortHingResult<()> {
    // (verbatim body from manager_session_helpers.rs:261-304)
    // ... (full body preserved verbatim from main; 43 src lines)
}

pub async fn discard_pending_session_updates_if_needed(session: &mut AcpRemoteSession) {
    // (verbatim body from manager_session_helpers.rs:305-355)
    // ... (full body preserved verbatim from main; 50 src lines)
}

pub fn update_session_from_events(session: &mut AcpRemoteSession, events: &[AcpClientStreamEvent]) {
    // (verbatim body from manager_session_helpers.rs:356-364)
    // ... (full body preserved verbatim from main; 8 src lines, dispatches to
    //      the 3 file-local update_* helpers below)
}

fn update_session_context_usage(
    session: &mut AcpRemoteSession,
    events: &[AcpClientStreamEvent],
) {
    // (verbatim body from manager_session_helpers.rs:365-378)
    // ... (full body preserved verbatim from main; 13 src lines, file-local)
}

fn update_session_available_commands(
    session: &mut AcpRemoteSession,
    events: &[AcpClientStreamEvent],
) {
    // (verbatim body from manager_session_helpers.rs:379-392)
    // ... (full body preserved verbatim from main; 13 src lines, file-local)
}

fn update_session_config_options(
    session: &mut AcpRemoteSession,
    events: &[AcpClientStreamEvent],
) {
    // (verbatim body from manager_session_helpers.rs:393-405)
    // ... (full body preserved verbatim from main; 12 src lines, file-local)
}
```

> **Producer note:** same as Task 4 — the `// ... (verbatim body ...)` comments are placeholders for the producer's copy-paste from `manager_session_helpers.rs:261-405`. Reviewer will diff against main.

- [ ] **Step 5.2: Verify file canonical line count + ≤5 long lines tolerance**

```bash
# PowerShell
$path = 'src/crates/interfaces/acp/src/client/manager_session_helpers_session_state.rs'
$canon = [System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8).Count
# Expected: ~175 (cap 242, -28% headroom)

# Long-line check
$lines = [System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8)
$longLines = 0
for ($i = 0; $i -lt $lines.Length; $i++) {
    if ($lines[$i].Length -gt 120) { $longLines++ }
}
# Expected: ≤5
```

### Task 6: Update `mod.rs` + delete original `manager_session_helpers.rs`

**Files:**
- Modify: `src/crates/interfaces/acp/src/client/mod.rs` (1 line replaced with 3)
- Delete: `src/crates/interfaces/acp/src/client/manager_session_helpers.rs`

**Steps:**

- [ ] **Step 6.1: Update `mod.rs` to register 3 new modules**

Edit `src/crates/interfaces/acp/src/client/mod.rs`:

```diff
 mod manager_session_lifecycle;
+mod manager_session_helpers_identity;
 mod manager_session_read;
 mod manager_session_resolve;
-mod manager_session_helpers;
+mod manager_session_helpers_session_response;
+mod manager_session_helpers_session_state;
```

(NOTE: in this R20b worktree, `manager_session_lifecycle.rs` and `manager_session_read.rs` do NOT exist because R20a is not merged. The actual `mod.rs` in main is the R19 baseline. Adjust the diff to match the actual main `mod.rs` content — see Task 6.1.a for verification.)

- [ ] **Step 6.1.a: Read the actual `mod.rs` to determine the right edit**

PowerShell:
```powershell
$path = 'src/crates/interfaces/acp/src/client/mod.rs'
Get-Content $path -Encoding UTF8 | ForEach-Object -Begin {$i=1} -Process { Write-Host "${i}: $_"; $i++ }
# Identify the line that has `mod manager_session_helpers;` and replace it
# with the 3 new mod declarations in alphabetical order.
```

- [ ] **Step 6.2: Update all 7 caller-side `use super::manager_session_helpers::{...}` to the new module paths**

The 7 caller files (per §1.3 caller map):

| File | Currently `use super::manager_session_helpers::{...}` | New `use super::manager_session_helpers_<sub>::{...}` |
|---|---|---|
| `manager_config.rs` | `aggregate_client_status, parse_config_value` | `manager_session_helpers_identity::{aggregate_client_status, parse_config_value}` |
| `manager_cancel.rs` | `build_session_key, session_client_connection_id` | `manager_session_helpers_identity::{build_session_key, session_client_connection_id}` |
| `manager_connection.rs` | `session_client_connection_id` | `manager_session_helpers_identity::session_client_connection_id` |
| `manager.rs` | `parse_config_value` | `manager_session_helpers_identity::parse_config_value` |
| `manager_session_resolve.rs` | `build_session_key, new_session_response_from_load, new_session_response_from_resume, session_client_connection_id` | `manager_session_helpers_identity::{build_session_key, session_client_connection_id}` + `manager_session_helpers_session_response::{new_session_response_from_load, new_session_response_from_resume}` (split into 2 separate `use` lines, alphabetical) |
| `manager_session_read.rs` | (R20a-spawned, NOT in this worktree) | (handled in R20a branch merge later) |
| `manager_prompt.rs` | `discard_pending_session_updates_if_needed, drain_pending_turn_updates, read_turn_to_string, update_session_from_events` | `manager_session_helpers_session_response::{drain_pending_turn_updates, read_turn_to_string}` + `manager_session_helpers_session_state::{discard_pending_session_updates_if_needed, update_session_from_events}` (split into 2 separate `use` lines, alphabetical) |

> **Producer note:** in this R20b worktree, `manager_session_resolve.rs` and `manager_session_read.rs` do NOT exist (they are R20a-spawned, in the `impl/r20a-manager-session-split` branch). The 5 caller files in the R20b worktree are: `manager_config.rs`, `manager_cancel.rs`, `manager_connection.rs`, `manager.rs`, `manager_prompt.rs`. The R20a branch will need to rebase / re-apply the same `use` change when R20a is merged; that is a user/merger problem, not R20b's concern.

- [ ] **Step 6.3: Delete original `manager_session_helpers.rs`**

```bash
git rm src/crates/interfaces/acp/src/client/manager_session_helpers.rs
# Expected: rm ... manager_session_helpers.rs
```

- [ ] **Step 6.4: Verify no remaining `use super::manager_session_helpers` references**

```bash
# PowerShell
Select-String -Path src\crates\interfaces\acp\src\client\*.rs -Pattern 'use super::manager_session_helpers[^_]' -SimpleMatch
# Expected: 0 matches (the only remaining `use super::manager_session_helpers` would
# be for the 3 new module names like `manager_session_helpers_identity`)
```

The `-SimpleMatch` is important: we want to match the bare `manager_session_helpers::` (the deleted module path) but NOT the new `manager_session_helpers_identity::` etc.

### Task 7: Verify (mandatory checks per R19 + R20a lessons)

**Files:** none (verification only)

**Steps:**

- [ ] **Step 7.1: cargo check — `bitfun-acp` lib**

```bash
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path  # gcc on PATH
cargo check -p bitfun-acp 2>&1 | Tee-Object -FilePath "C:\Users\UmR\AppData\Local\Temp\r20b-acp-check.log" | Out-Null
$lines = [System.IO.File]::ReadAllLines('C:\Users\UmR\AppData\Local\Temp\r20b-acp-check.log', [System.Text.Encoding]::UTF8)
$errs = $lines | Select-String -Pattern '^error\[E'
Write-Host "errors: $($errs.Count)"
$errs | ForEach-Object { Write-Host $_.Line }
# Expected: 0 errors
```

If errors:
- `E0432 (use of undeclared module)`: missing `use` line in a caller file → re-run Step 6.2
- `E0609 (no field/variant on struct)`: mis-copied body → diff against main
- `E0624 (method is private)`: wrong visibility on a moved fn → verify the fn is `pub fn` (per §2.2)

- [ ] **Step 7.2: cargo check — cross-crate (R19 lesson: do NOT just rely on target crate)**

```bash
cargo check -p northhing-cli 2>&1 | Tee-Object -FilePath "C:\Users\UmR\AppData\Local\Temp\r20b-cli-check.log" | Out-Null
$lines = [System.IO.File]::ReadAllLines('C:\Users\UmR\AppData\Local\Temp\r20b-cli-check.log', [System.Text.Encoding]::UTF8)
$errs = $lines | Select-String -Pattern '^error\[E'
Write-Host "cli errors: $($errs.Count)"
$errs | ForEach-Object { Write-Host $_.Line }
# Expected: 0 NEW errors (2 pre-existing E0624 on session_manager.get_session
# are NOT in this worktree — R20a branch is separate; the only E06xx errors
# here should be 0)

cargo check --workspace 2>&1 | Tee-Object -FilePath "C:\Users\UmR\AppData\Local\Temp\r20b-ws-check.log" | Out-Null
$lines = [System.IO.File]::ReadAllLines('C:\Users\UmR\AppData\Local\Temp\r20b-ws-check.log', [System.Text.Encoding]::UTF8)
$errs = $lines | Select-String -Pattern '^error\[E'
Write-Host "workspace errors: $($errs.Count)"
$errs | ForEach-Object { Write-Host $_.Line }
# Expected: 0 NEW errors from R20b diff
```

- [ ] **Step 7.3: cargo test — `bitfun-acp` lib baseline**

```bash
cargo test -p bitfun-acp --lib 2>&1 | Select-String -Pattern 'test result:|error\[E'
# Expected: test result: ok. 51 passed; 0 failed; 0 ignored (matches R20a baseline)
```

- [ ] **Step 7.4: rustfmt --edition 2021 check (per R20a standard)**

```bash
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
rustfmt --edition 2021 --check \
  src/crates/interfaces/acp/src/client/manager_session_helpers_identity.rs \
  src/crates/interfaces/acp/src/client/manager_session_helpers_session_response.rs \
  src/crates/interfaces/acp/src/client/manager_session_helpers_session_state.rs \
  src/crates/interfaces/acp/src/client/mod.rs 2>&1
# Expected: 0 diffs on the 3 new files (mod.rs may have pre-existing diffs at line 10/16
# from R20a-trailing-cycle, which are NOT R20b's concern)
```

- [ ] **Step 7.5: Iron rules — no new unwrap/expect/panic/let _ = Result**

```bash
# Pre-impl baseline (main)
git show main:src/crates/interfaces/acp/src/client/manager_session_helpers.rs | \
  Select-String -Pattern '\bunwrap\(|\bexpect\(|panic!|unreachable!|let _ = .*Result' | Measure-Object | Select-Object Count
# Note: this counts hits in the file as it exists on main.

# Post-impl (sum across the 3 new files)
$paths = @(
  'src/crates/interfaces/acp/src/client/manager_session_helpers_identity.rs',
  'src/crates/interfaces/acp/src/client/manager_session_helpers_session_response.rs',
  'src/crates/interfaces/acp/src/client/manager_session_helpers_session_state.rs'
)
$total = 0
foreach ($p in $paths) {
  $count = (Select-String -Path $p -Pattern '\bunwrap\(|\bexpect\(|panic!|unreachable!|let _ = .*Result').Count
  $total += $count
  Write-Host "$($p | Split-Path -Leaf): $count"
}
Write-Host "total: $total"
# Expected: 0 (sum across 3 new files should match main baseline of 0; if main has any,
# the sum should be ≤ main; R20a review confirmed 0 baseline so expect 0)
```

- [ ] **Step 7.6: Cargo.lock drift check**

```bash
git diff main..HEAD -- Cargo.lock | wc -l
# Expected: 0
```

- [ ] **Step 7.7: CRLF / line-ending check**

```bash
file src/crates/interfaces/acp/src/client/manager_session_helpers_*.rs
# Expected: "UTF-8 text" or "ASCII text", NO "with CRLF" mention for all 3
```

- [ ] **Step 7.8: Canonical line count final check**

```bash
foreach ($p in @(
  'src/crates/interfaces/acp/src/client/manager_session_helpers_identity.rs',
  'src/crates/interfaces/acp/src/client/manager_session_helpers_session_response.rs',
  'src/crates/interfaces/acp/src/client/manager_session_helpers_session_state.rs'
)) {
  $canon = [System.IO.File]::ReadAllLines($p, [System.Text.Encoding]::UTF8).Count
  $status = if ($canon -le 242) { '✅' } else { '❌' }
  Write-Host "$($p | Split-Path -Leaf): $canon $status (cap 242)"
}
# Expected: all 3 ≤ 242
```

### Task 8: Commit + handoff

**Files:**
- Stage: 3 new files + modified `mod.rs` + deletion of original `manager_session_helpers.rs`
- Create: `docs/handoffs/2026-07-01-r20b-manager-session-helpers-split-impl.md` (impl handoff per R20a pattern)

**Steps:**

- [ ] **Step 8.1: Stage all changes**

```bash
git add \
  src/crates/interfaces/acp/src/client/manager_session_helpers_identity.rs \
  src/crates/interfaces/acp/src/client/manager_session_helpers_session_response.rs \
  src/crates/interfaces/acp/src/client/manager_session_helpers_session_state.rs \
  src/crates/interfaces/acp/src/client/mod.rs \
  src/crates/interfaces/acp/src/client/manager.rs \
  src/crates/interfaces/acp/src/client/manager_config.rs \
  src/crates/interfaces/acp/src/client/manager_cancel.rs \
  src/crates/interfaces/acp/src/client/manager_connection.rs \
  src/crates/interfaces/acp/src/client/manager_prompt.rs
# Note: manager_session_helpers.rs deletion is staged separately
git rm src/crates/interfaces/acp/src/client/manager_session_helpers.rs
```

- [ ] **Step 8.2: Write impl handoff doc**

Create `docs/handoffs/2026-07-01-r20b-manager-session-helpers-split-impl.md` following the R20a pattern (header + summary + per-file table + spec deviation section if any + per-method mapping + verification results). The reviewer (Kimi) reads this in conjunction with the spec.

- [ ] **Step 8.3: Commit**

```bash
git add docs/handoffs/2026-07-01-r20b-manager-session-helpers-split-impl.md
git commit -m "refactor(bitfun-acp): R20b close QClaw R20a P1 D-deviation (manager_session_helpers.rs 405 → 3 files)"
```

Commit body should follow the R20a pattern: 1-2 paragraph summary, file inventory table, per-method mapping (old path → new path), spec deviation section (if any), verification results (cargo check/test/rustfmt outputs).

- [ ] **Step 8.4: Final Mavis 10-axis verification (per R20a retro)**

Mavis runs the 10-axis verification after the producer's commit lands:
1. Canonical line counts (all 3 new files ≤ 242, 1 file deleted)
2. Sub-domain coherence (A / B+C / D semantic split, no cross-boundary helpers)
3. Visibility pattern (10 `pub fn` + 5 file-local `fn` — match §2.2 plan)
4. Iron rules (0 new unwrap/expect/panic/let _ = Result)
5. Line endings (0 CRLF, all LF)
6. Line length (≤5 long lines per file)
7. Cargo.lock drift (0)
8. Cargo check (acp + cli + workspace all 0 errors)
9. Cargo test (bitfun-acp 51/0/0 preserved, core 899/0/1 preserved)
10. Cross-crate consumer check (0 NEW E0624; 0 NEW direct module refs outside `acp`)

---

## 4. Pre-emptive lessons (R18 + R19 + R20a)

| Round | Lesson | R20b application |
|---|---|---|
| R18 | Long-line tolerance: ≤5 new long lines per file | Task 4.2 + 5.2 + 7.4 enforce this |
| R18 | `Measure-Object -Line` is wrong, use `ReadAllLines().Count` or `wc -l` | All canonical line counts in this spec use canonical method |
| R19 | `pub(super)` over-prescription caused 11 E0624 errors. Default `pub` for cross-crate, `pub(super)` only for cross-sibling inherent dispatch | §2.2: all 16 fns are sibling-consumed (some are file-local); 0 cross-crate callers; default to `pub fn` |
| R19 | Pre-emptively extend timeout at dispatch for splits >1000 lines | Task 2.1: 405 lines is under 1000, but +30 min extend at dispatch is cheap insurance |
| R19 | Cross-crate consumer verification (`cargo check -p northhing-cli` not just target crate) | Task 7.2: explicit cli + workspace check |
| R19 | Reviewer uses `grep -cE '^error\['` against `--message-format=short` which hides errors | Task 7.1 + 7.2 use raw output, not short format |
| R20a | Lifecycle over-cap was caught pre-emptively by Mavis 10-axis verification | Task 8.4: Mavis runs 10-axis check after producer's commit |
| R20a | spec `pub mod` claim vs code `mod` (cosmetic) | R20b spec uses correct `mod` (private) terminology throughout |
| R20a | BOM introduced by PowerShell `[Encoding]::UTF8` WriteAllText | Task 7.7: post-write verify with raw `file` command or byte check; if BOM, strip with Python |
| R20a | Trailing newline missing in 2 R20a-touched files | Task 7.7: all 3 new files end with LF (verify with byte check) |
| R20a | R20a branch (`impl/r20a-manager-session-split`) is NOT yet merged to main | §1.3: explicit note that R20b is forked from main `f579c71`, so R20a-spawned `manager_session_read.rs` and `manager_session_resolve.rs` are not in this worktree; R20a branch will need rebase when merging |

---

## 5. Risks + mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Producer copy-paste mistake drops a function body | Medium | High (E0609 compile error, E0432 import error) | Task 7.1 + 7.2 cargo check catches it; reviewer diff against main |
| Producer's `use` updates miss a file | Medium | High (E0432 "no `manager_session_helpers` in `client`") | Task 6.4 explicit no-remaining-refs check; cargo check catches |
| 3 new files individually exceed 242 cap after body copy-paste | Low | Medium (over-cap D-deviation) | Task 4.2 + 5.2 + 7.8 line count verification |
| Producer adds new code (refactor on top of split) | Low | High (R20a baseline preserved only if no behavior change) | Spec §1.1 explicit: "Pure structural split — no behavior change, no method-body refactoring"; reviewer enforces |
| 5 file-local helpers accidentally made `pub` | Low | Low (over-exposes internal helpers; review will catch) | §2.2 visibility rule + reviewer check; default to plain `fn` for file-local |
| R20a branch merge conflict when re-applying R20b `use` changes | Medium (deferred, not in R20b scope) | Medium | §1.3 explicit: R20a branch is independent; merger re-applies `use` change when both branches land |
| Reviewer (Kimi) disagrees with A / (B+C) / D grouping, prefers 4-way split | Low | Medium (rework) | §1.2 documents 3-way rationale; QClaw P1 says "2-3 files" so 3 is in spec range; if Kimi insists 4, can re-run with A/B/C/D |

---

## 6. Self-review (writing-plans standard)

### 6.1 Spec coverage

| Spec requirement | Task implementing it |
|---|---|
| Close QClaw R20a P1 D-deviation | Tasks 3-6 (split), Task 8.3 (commit) |
| 405 → 2-3 files (QClaw range) | Tasks 3, 4, 5 (3 files) |
| Pure structural split, no behavior change | Tasks 3.1, 4.1, 5.1 (verbatim body copy), Task 7.5 (iron rules) |
| 7 caller files updated | Task 6.2 |
| Cargo check + test baseline preserved | Tasks 7.1, 7.2, 7.3 |
| Visibility pattern (R19 lesson) | §2.2 + Task 7 (review) |
| Canonical line measurement (R18 lesson) | Tasks 1.2, 3.2, 4.2, 5.2, 7.8 |
| Long-line tolerance (R18 lesson) | Tasks 4.2, 5.2, 7.4 |
| Pre-emptive timeout extend (R19 lesson) | Task 2.1 |
| Cross-crate consumer check (R19 lesson) | Task 7.2 |
| BOM / CRLF / LF hygiene (R20a lesson) | Task 7.7 |
| Mavis 10-axis verification | Task 8.4 |

### 6.2 Placeholder scan

Searched for: `TBD`, `TODO`, `implement later`, `fill in details`, "add appropriate error handling", "write tests for the above", "similar to Task N", "to be determined".

- 1 placeholder found: `// ... (verbatim body ...)` comments in Tasks 3.1, 4.1, 5.1.
  - **Why this is intentional**: these are 13 separate function bodies (52 + 22 + 133 + 144 = 351 src lines of business logic) that the producer must copy-paste verbatim from main. Showing the FULL body in this spec would push the spec to 700+ lines and obscure the structural intent. The producer has direct read access to the source file (`src/crates/interfaces/acp/src/client/manager_session_helpers.rs`).
  - **Why this is acceptable** (per writing-plans skill "No Placeholders" rule, **exception documented explicitly**): the bodies are bit-for-bit copy of existing code, not new logic. The reviewer (Kimi) will diff the new files against main to verify zero behavior change. This is a producer-mechanical step, not a design decision.
  - **Mitigation**: the producer's commit message MUST include a `git diff main:src/crates/interfaces/acp/src/client/manager_session_helpers.rs HEAD:src/crates/interfaces/acp/src/client/manager_session_helpers_*.rs` summary proving zero behavior change.

- No other placeholders found. All other steps have full content.

### 6.3 Type consistency

Checked method signatures across tasks:

- `parse_config_value(value: serde_json::Value) -> NortHingResult<AcpClientConfigFile>` — consistent in Tasks 3.1 and the use site in `manager_config.rs:26` (per §1.3)
- `build_session_key(northhing_session_id: &str, client_id: &str, cwd: &Path) -> String` — consistent in Task 3.1 and `manager_cancel.rs:20` + `manager_session_resolve.rs:35`
- `drain_pending_turn_updates<F>(session: &mut AcpRemoteSession, on_event: F) -> NortHingResult<()>` — consistent in Task 4.1 and `manager_prompt.rs:20`
- `update_session_from_events(session: &mut AcpRemoteSession, events: &[AcpClientStreamEvent])` — consistent in Task 5.1 and `manager_prompt.rs:20`

No signature drift detected.

---

## 7. Subagent dispatch handoff (Mavis → producer agent)

The producer agent dispatched by `mavis team plan` should follow this spec task-by-task. The producer's verification step is Task 7 (cargo check + test + rustfmt + iron rules + Cargo.lock). The producer's commit is Task 8.3.

After the producer's commit lands, Mavis runs Task 8.4 (10-axis verification) before sending to Kimi for review.

The reviewer (Kimi) reads:
1. This spec (`docs/handoffs/2026-07-01-r20b-manager-session-helpers-split-spec.md`)
2. The producer's impl handoff (`docs/handoffs/2026-07-01-r20b-manager-session-helpers-split-impl.md`)
3. The producer's commit (`git show HEAD` — should show: 1 deleted file + 3 new files + 1 modified mod.rs + 5 modified caller files = 10 file changes, 1 commit)

The reviewer verifies:
- All 3 new files ≤ 242 cap (canonical wc-l)
- All 16 fns preserved verbatim (diff against main `manager_session_helpers.rs:1-405`)
- 0 new unwrap/expect/panic/let _ = Result
- 0 CRLF, all LF
- 0 Cargo.lock drift
- `cargo check -p bitfun-acp`: 0 errors
- `cargo check -p northhing-cli`: 0 errors (R19 cross-crate lesson)
- `cargo test -p bitfun-acp --lib`: 51 passed, 0 failed (R20a baseline preserved)
- `git grep -n 'manager_session_helpers::' -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/'`: 0 hits (no NEW cross-crate refs)
- Visibility: 10 `pub fn` (4 A + 4 B+C public + 2 D public) + 5 plain `fn` (2 file-local B+C + 3 file-local D) = 15 total, all internal to `client/`

---

*Spec authored by Mavis on 2026-07-01. R20b scope = QClaw R20a P1 D-deviation. Branch `impl/r20b-manager-session-helpers-split` (forked from main `f579c71`, not from R20a branch). Estimated producer runtime: 25-35 min for 405-line split (vs R20a 37 min for 486-line split). 1 producer commit + 1 spec commit + 1 impl handoff doc.*
