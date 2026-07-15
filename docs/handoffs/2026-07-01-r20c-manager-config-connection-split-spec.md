# R20c Spec — bitfun-acp manager_config + manager_connection sub-domain split (close QClaw R20a P2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. This spec follows the R20a spec format + writing-plans task granularity standard. R20c is structurally a "double R20a" — same pattern (inherent methods on `AcpClientService`, no caller `use` updates needed, sibling inherent dispatch).

**Goal:** Split `acp/client/manager_config.rs` (292 canonical lines, +21% over QClaw 242) AND `acp/client/manager_connection.rs` (287 canonical lines, +19% over QClaw 242) into 2 + 2 = 4 sibling sub-domain files. Closes the QClaw R20a P2 D-deviations. Pure structural split — no behavior change, no method-body refactoring, no caller `use` updates.

**Architecture:** 2-way sub-domain split per file (4 new sibling files total). All method bodies move verbatim. NO facade. NO forwarder. All methods remain on `impl AcpClientService` and are accessible via inherent dispatch from any sibling `impl AcpClientService` block. R20a's "always `pub` default" rule applied. Zero cross-crate callers verified.

**Tech Stack:** Rust 2021+, `bitfun-acp` (interfaces layer), M2.7-highspeed agent, cargo 1.85+, canonical line measurement via `[System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8).Count` (PowerShell) or `wc -l <file>` (bash).

---

## 1. Background

### 1.1 Why this split

QClaw R20a review (8.8/10 APPROVE) flagged two simultaneous P2 D-deviations:

> P2 R20c: `manager_config.rs` 292 → split into 2 files. Medium D-deviation (+21%). 8 methods, 100-line `register_configured_tools`.
> P2 R20c: `manager_connection.rs` 287 → split into 2 files. Medium D-deviation (+19%). 6 methods, 147-line `start_client_connection`.

Kimi R20a review independently confirmed. Both reviewers recommended "split into 2 files" per file.

R20a + R20b + R20c together close the 6 over-cap D-deviations in the acp `manager_*` family that Kimi R19 flagged (R20a closed the **Critical** one; R20b closed the next P1; R20c closes the 2 P2s).

### 1.2 Sub-domain analysis (2 natural splits per file)

#### manager_config.rs (292 canonical, 8 methods on `AcpClientService`)

| Sub-domain | Methods | Span | Src lines | Why coherent |
|---|---|---:|---:|---|
| **A. Client listing** | `list_clients` | L43-76 | ~33 | "List available clients" — read-only access to config state |
| **B. Requirement probing** | `probe_client_requirements`, `refresh_remote_client_requirements`, `probe_remote_client_requirements` | L77-253 | ~174 | All about probing client capabilities / requirements |
| **C. Config loading** (private helpers) | `load_configs`, `load_config_file`, `load_config_value` | L254-269 | ~13 | All about reading config from disk — private helpers |
| **D. Tool registration** | `register_configured_tools` | L270-end | ~23 | Registers tools based on config |

2-way split per QClaw recommendation:
- **manager_config_loading.rs**: A + C (config reading + listing) = 4 methods, ~46 src + ~30 headers = ~80 canonical
  - "What clients are configured and how to load them" (read side)
- **manager_config_requirements.rs**: B + D (probing + tool registration) = 4 methods, ~197 src + ~30 headers = ~230 canonical
  - "What can these clients do and what tools do they provide" (use side)

#### manager_connection.rs (287 canonical, 6 methods on `AcpClientService`)

| Sub-domain | Methods | Span | Src lines | Why coherent |
|---|---|---:|---:|---|
| **A. Connection start lifecycle** | `initialize_all`, `start_client_for_session`, `start_client_connection` | L48-238 | ~188 | All about starting/initializing a connection |
| **B. Connection stop lifecycle** | `cleanup_failed_startup`, `stop_client`, `stop_connection` | L239-end | ~46 | All about stopping/cleaning up a connection |

2-way split per QClaw recommendation:
- **manager_connection_start.rs**: A (3 methods) = ~188 src + ~30 headers = ~220 canonical
  - "Bring a connection up" (positive lifecycle)
- **manager_connection_stop.rs**: B (3 methods) = ~46 src + ~30 headers = ~75 canonical
  - "Tear a connection down" (negative lifecycle)

All 4 new files ≤242 with 9-69% headroom. The "start" / "stop" split for connection is the most natural R20a-pattern boundary (positive vs negative lifecycle). The "load" / "use" split for config is the most natural read/write split for configuration.

### 1.3 Caller map (before split) — KEY: zero external callers

External (in-crate, sibling files) callers per method: **0**.

Verification:
- `git grep 'use super::manager_config' -- 'src/crates/interfaces/acp/'`: 0 hits
- `git grep 'use super::manager_connection' -- 'src/crates/interfaces/acp/'`: 0 hits
- `git grep 'manager_config::' -- 'src/crates/interfaces/acp/'`: 0 hits
- `git grep 'manager_connection::' -- 'src/crates/interfaces/acp/'`: 0 hits

All 14 methods (8 in config + 6 in connection) are called ONLY via inherent dispatch on `AcpClientService` (e.g., `self.start_client_connection(...)` from another sibling's `impl AcpClientService` block). Inherent dispatch works across sibling files without `use` imports — same as R20a pattern.

Cross-crate callers: 0 (verified by `git grep 'manager_config::|manager_connection::' -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/'`).

**Caller-side impact: zero caller files need `use` updates. The split is pure file-organization.**

### 1.4 Branch forking note

R20c is forked from main `f579c71` (R20a spec only). R20a + R20b branches (`impl/r20a-manager-session-split`, `impl/r20b-manager-session-helpers-split`) are NOT merged. The R20a + R20b + R20c branch dance:

- Each is independent (no file conflicts because they touch different files)
- All 3 will need to be merged together or in sequence
- This is a user/merger concern, not R20c's

---

## 2. File structure (post-split)

### 2.1 New files (4) and changes (1) and deletions (2)

| File | Action | Approx canonical | Sub-domain | Method count | Visibility default |
|---|---|---:|---|---:|---|
| `src/crates/interfaces/acp/src/client/manager_config.rs` | **DELETED** (all bodies moved out) | 0 | n/a | 0 | n/a |
| `src/crates/interfaces/acp/src/client/manager_connection.rs` | **DELETED** (all bodies moved out) | 0 | n/a | 0 | n/a |
| `src/crates/interfaces/acp/src/client/manager_config_loading.rs` | **NEW** | ~80 | A + C | 4 | `pub async fn` (1 A) + `async fn` (3 C private) |
| `src/crates/interfaces/acp/src/client/manager_config_requirements.rs` | **NEW** | ~230 | B + D | 4 | `pub async fn` (4) |
| `src/crates/interfaces/acp/src/client/manager_connection_start.rs` | **NEW** | ~220 | A | 3 | `pub async fn` (3) |
| `src/crates/interfaces/acp/src/client/manager_connection_stop.rs` | **NEW** | ~75 | B | 3 | `pub async fn` (3) |
| `src/crates/interfaces/acp/src/client/mod.rs` | **MODIFIED** (replace 2 mod declarations with 4 new, alphabetical) | +2 lines | n/a | n/a | private `mod` (matching R20a/R20b pattern) |

**Total: 4 new files, 1 modified, 2 deleted. Net +~605 canonical (vs 579 original = 292+287, +26 from new R20c headers + per-file imports).**

### 2.2 Visibility rule (R19 + R20a lessons applied)

- **Cross-crate callers: 0** — `pub` is sufficient everywhere
- **Sibling files (in `src/crates/interfaces/acp/src/client/`) need to call into the new method groups** — they use `self.method()` via inherent dispatch. For inherent dispatch across siblings to work, methods must be at least `pub(super)` (visible to parent `client` module's siblings). Per R19 + R20a lessons, **default to `pub fn`** unless file-local. 0 cross-crate callers + sibling visibility need = `pub fn` is correct.
- **3 file-local helpers** in `manager_config_loading.rs` (`load_configs`, `load_config_file`, `load_config_value`) are called only from within the same file (by `list_clients` and each other). They should be plain `fn` (no `pub` prefix) — no sibling consumer.
- All 11 other methods are externally called (via inherent dispatch from other `impl AcpClientService` blocks) → `pub fn` or `pub async fn`.

### 2.3 Per-method visibility table

**manager_config_loading.rs** (4 methods):
- `list_clients` — `pub async fn` (called by sibling `impl AcpClientService` blocks, e.g. `manager_session_lifecycle.rs::release_northhing_session` may call it)
- `load_configs` — plain `async fn` (file-local, called by `list_clients`)
- `load_config_file` — plain `async fn` (file-local, called by `load_configs`)
- `load_config_value` — plain `async fn` (file-local, called by `load_config_file`)

**manager_config_requirements.rs** (4 methods):
- `probe_client_requirements` — `pub async fn` (sibling caller)
- `refresh_remote_client_requirements` — `pub async fn` (sibling caller)
- `probe_remote_client_requirements` — `pub async fn` (sibling caller)
- `register_configured_tools` — `pub fn` (sibling caller; the 23-line one QClaw flagged)

**manager_connection_start.rs** (3 methods):
- `initialize_all` — `pub async fn` (sibling caller)
- `start_client_for_session` — `pub async fn` (sibling caller)
- `start_client_connection` — `pub async fn` (sibling caller; the 147-line one QClaw flagged)

**manager_connection_stop.rs** (3 methods):
- `cleanup_failed_startup` — `pub async fn` (sibling caller; called by `start_client_connection` in start.rs)
- `stop_client` — `pub async fn` (sibling caller)
- `stop_connection` — `pub async fn` (sibling caller; called by `release_northhing_session` in `manager_session_lifecycle.rs`)

**Total**: 11 `pub fn` / `pub async fn` + 3 plain `fn` (file-local in `manager_config_loading.rs`) = 14 methods preserved verbatim.

### 2.4 Per-file naming

- `manager_config_loading.rs` — config reading + client listing (4 methods)
- `manager_config_requirements.rs` — requirement probing + tool registration (4 methods)
- `manager_connection_start.rs` — connection start lifecycle (3 methods)
- `manager_connection_stop.rs` — connection stop lifecycle (3 methods)

Naming rationale: keep the `manager_config_*` and `manager_connection_*` prefix so grep-ability and module discovery remain clusterable. Suffix carries the sub-domain (load / requirements / start / stop). Matches R20a / R20b precedent (`manager_session_lifecycle.rs`, `manager_session_resolve.rs`, `manager_session_read.rs`, `manager_session_helpers_*.rs`).

---

## 3. Tasks (bite-sized, with full code)

Each task produces a self-contained, committable change. DRY: if a task is "do X to file Y", the file Y is shown in full or with the exact diff. No "similar to Task N" placeholders.

### Task 1: Pre-flight — verify baseline + worktree

**Files:**
- Read: `src/crates/interfaces/acp/src/client/manager_config.rs` (292 canonical)
- Read: `src/crates/interfaces/acp/src/client/manager_connection.rs` (287 canonical)
- Read: `src/crates/interfaces/acp/src/client/mod.rs` (line 6, 7 has `mod manager_config;` and `mod manager_connection;`)

**Steps:**

- [ ] **Step 1.1: Verify worktree state**

```bash
cd E:/agent-project/northing-impl-r20c-manager-config-connection-split
git log --oneline -1
# Expected: f579c71 docs(spec): R20a close Critical D-deviation (manager_session.rs 486 → 2 files)
git status
# Expected: clean working tree
git config --local --get core.autocrlf
# Expected: false (set by R20c worktree init; verify it's still false)
```

If `core.autocrlf` is NOT `false`:
```bash
git config --local --get core.autocrlf
# If returns nothing or "true", set to false
git config core.autocrlf false
```

- [ ] **Step 1.2: Re-verify canonical line count + content sanity**

PowerShell:
```powershell
$paths = @(
  'src/crates/interfaces/acp/src/client/manager_config.rs',
  'src/crates/interfaces/acp/src/client/manager_connection.rs'
)
foreach ($p in $paths) {
  $canon = [System.IO.File]::ReadAllLines($p, [System.Text.Encoding]::UTF8).Count
  Write-Host "$p: canonical=$canon (expected 292 / 287)"
  $bytes = [System.IO.File]::ReadAllBytes($p)
  $bom = if ($bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) { 'YES' } else { 'NO' }
  $crlf = 0; for ($i = 0; $i -lt $bytes.Length - 1; $i++) { if ($bytes[$i] -eq 0x0D -and $bytes[$i+1] -eq 0x0A) { $crlf++ } }
  Write-Host "  BOM=$bom CRLF=$crlf last_byte=$($bytes[$bytes.Length-1]) (expected 10)"
}
```

Bash (cross-check):
```bash
wc -l src/crates/interfaces/acp/src/client/manager_config.rs src/crates/interfaces/acp/src/client/manager_connection.rs
# Expected: 292 manager_config.rs
#           287 manager_connection.rs
file src/crates/interfaces/acp/src/client/manager_config.rs src/crates/interfaces/acp/src/client/manager_connection.rs
# Expected: UTF-8 text, NO "with CRLF" mention
```

- [ ] **Step 1.3: Verify cargo baseline (no R20a-related code in this worktree)**

```bash
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo check -p northhing-acp 2>&1 | tee baseline-acp.log
cargo check -p northhing-cli 2>&1 | tee baseline-cli.log
cargo check --workspace 2>&1 | tee baseline-ws.log
cargo test -p northhing-acp --lib 2>&1 | tee baseline-acp-test.log
cargo test -p northhing-core --features 'service-integrations,product-full' --lib 2>&1 | tee baseline-core-test.log

$baselineAcpErrors = (Select-String -Path baseline-acp.log -Pattern "error\[" | Measure-Object).Count
$baselineCliErrors = (Select-String -Path baseline-cli.log -Pattern "error\[" | Measure-Object).Count
$baselineWorkspaceErrors = (Select-String -Path baseline-ws.log -Pattern "error\[" | Measure-Object).Count
$baselineAcpTests = (Select-String -Path baseline-acp-test.log -Pattern "test result:" | Select-Object -First 1).ToString()
$baselineCoreTests = (Select-String -Path baseline-core-test.log -Pattern "test result:" | Select-Object -First 1).ToString()
Write-Host "BASELINE_ACP_ERRORS=$baselineAcpErrors (expected 0)"
Write-Host "BASELINE_CLI_ERRORS=$baselineCliErrors (expected 2 — pre-existing pub(crate) E0624 on session_manager.get_session, NOT R20c scope; R20a branch has fe87083 fix that this worktree lacks)"
Write-Host "BASELINE_WORKSPACE_ERRORS=$baselineWorkspaceErrors (expected 0)"
Write-Host "BASELINE_ACP_TESTS=$baselineAcpTests (expected 51 passed; 0 failed)"
Write-Host "BASELINE_CORE_TESTS=$baselineCoreTests (expected 899 passed; 0 failed; 1 ignored)"
```

- [ ] **Step 1.4: Confirm branch + commit spec file**

Branch already created by worktree add. Confirm:
```bash
git branch --show-current
# Expected: impl/r20c-manager-config-connection-split
git add docs/handoffs/2026-07-01-r20c-manager-config-connection-split-spec.md
git commit -m "docs(spec): R20c close QClaw R20a P2 D-deviations (manager_config 292 + manager_connection 287 → 4 files)"
```

### Task 2: Pre-emptively extend timeout at dispatch (R19 standing rule)

**Files:** none (worktree-level config)

**Steps:**

- [ ] **Step 2.1: When dispatching the implementation plan, immediately extend the team plan timeout**

Per R19 standing rule ("Extend-timeout at dispatch for >2000 line splits") + R20a/R20b retrospective (R20a was 486 lines, finished in 37 min; R20b was 405 lines, finished in 32 min; R20c covers 2 files of 292 + 287 = 579 total lines, ~1.2x R20a/R20b):

After `mavis team plan run ... --no-wait <plan-id>`:
```bash
mavis team plan extend-timeout <plan-id> <task-id> --minutes 60
```

The producer is expected to finish in ~40-50 min for the 2-file split; the +60 extension gives a 100-110 min window. This is pre-emptive mitigation against the R19 5-pass take-over pattern.

### Task 3: Create `manager_config_loading.rs` (file A + C)

**Files:**
- Create: `src/crates/interfaces/acp/src/client/manager_config_loading.rs`

**Steps:**

- [ ] **Step 3.1: Create the new file with sub-domain A + C content (verbatim from main)**

Use the exact code below. The 4 free fns (well, 1 method + 3 private helpers in this case — all in `impl AcpClientService`) are moved verbatim from `src/crates/interfaces/acp/src/client/manager_config.rs:42-269`. The `impl AcpClientService { ... }` block is preserved. No body changes, no signature changes, no import reorder.

```rust
// R20c split: ACP client config loading + client listing.
// File: src/crates/interfaces/acp/src/client/manager_config_loading.rs
// Origin: manager_config.rs (292 lines, QClaw R20a P2 D-deviation +21%
//        over QClaw 242 tolerance)
// Mavis fix: R20c split manager_config.rs into 2 files (this +
//        manager_config_requirements.rs) to close the 242 line cap.
//        Sub-domain A + C: config reading (3 private helpers) +
//        client listing (1 pub method).
// R20c sibling: manager_config_requirements.rs (sub-domain B + D)
// R20c sibling: manager_connection_start.rs + manager_connection_stop.rs
// R19 sibling files:
//             manager_install.rs
//             manager_session.rs
//             manager_prompt.rs
//             manager_cancel.rs
//             manager_permission.rs
//             manager_process.rs
//             manager_process_lifecycle.rs
//             manager_session_helpers_identity.rs
//             manager_session_helpers_session_response.rs
//             manager_session_helpers_session_state.rs
//             manager_errors.rs
//
// All method bodies are moved verbatim from main. No behavior change.

use super::config::{AcpClientConfig, AcpClientConfigFile, AcpClientInfo, AcpClientStatus};
// ... (other verbatim imports from main:291-...)

impl AcpClientService {
    // ... (verbatim body of list_clients from main:43-76)

    async fn load_configs(&self) -> ... { ... }
    async fn load_config_file(&self, ...) -> ... { ... }
    async fn load_config_value(&self, ...) -> ... { ... }
}
```

> **Producer note:** The bodies shown as `// ... (verbatim body ...)` are placeholders for the producer's actual copy-paste. The producer MUST copy the FULL function bodies verbatim from `src/crates/interfaces/acp/src/client/manager_config.rs:43-269` (the 1 pub method + 3 private helpers + their `impl AcpClientService {` block + imports). The reviewer (Kimi) will diff this file against main to verify zero behavior change.

- [ ] **Step 3.2: Verify file canonical line count**

PowerShell:
```powershell
$path = 'src/crates/interfaces/acp/src/client/manager_config_loading.rs'
[System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8).Count
# Expected: ~80 (cap 242, -67% headroom)
```

- [ ] **Step 3.3: Hold off commit (fold into final impl commit)**

Per R20a pattern: a single `refactor(bitfun-acp):` commit at the end with all 4 new files + mod.rs change + 2 deletions. We'll commit everything together in Task 8.

### Task 4: Create `manager_config_requirements.rs` (file B + D)

**Files:**
- Create: `src/crates/interfaces/acp/src/client/manager_config_requirements.rs`

**Steps:**

- [ ] **Step 4.1: Create the new file with sub-domain B + D content (verbatim from main)**

Use the exact code below. The 4 methods (3 requirement probing + 1 tool registration, all pub) are moved verbatim from `src/crates/interfaces/acp/src/client/manager_config.rs:77-end`. The `impl AcpClientService { ... }` block is preserved.

```rust
// R20c split: ACP client requirement probing + tool registration.
// File: src/crates/interfaces/acp/src/client/manager_config_requirements.rs
// Origin: manager_config.rs (292 lines, QClaw R20a P2 D-deviation)
// Mavis fix: R20c sub-domain B (3 fns: requirement probing) + sub-domain D
//        (1 fn: tool registration). All 4 are pub fn / pub async fn
//        (sibling-inherent-dispatch consumers).
// R20c sibling: manager_config_loading.rs (sub-domain A + C)
// R20c sibling: manager_connection_start.rs + manager_connection_stop.rs
// R19 sibling files (consumers of B + D methods):
//             manager_session.rs (probe_remote_client_requirements for session init)
//             manager_install.rs (probe_client_requirements for install flow)
//             manager_prompt.rs (register_configured_tools for prompt-time tool setup)
//             ... (other siblings that call these via self.method())
//
// All method bodies are moved verbatim from main. No behavior change.

use super::config::{...};  // (verbatim from main)
// ... (other verbatim imports)

impl AcpClientService {
    pub async fn probe_client_requirements(&self, ...) -> ... { ... }
    pub async fn refresh_remote_client_requirements(&self, ...) -> ... { ... }
    pub async fn probe_remote_client_requirements(&self, ...) -> ... { ... }
    pub fn register_configured_tools(&self, ...) -> ... { ... }
}
```

> **Producer note:** same as Task 3 — the `// ...` comments are placeholders for the producer's copy-paste from `manager_config.rs:77-end`. Reviewer will diff against main.

- [ ] **Step 4.2: Verify file canonical line count + ≤5 long lines tolerance (R18 rule)**

```powershell
$path = 'src/crates/interfaces/acp/src/client/manager_config_requirements.rs'
$canon = [System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8).Count
# Expected: ~230 (cap 242, -5% headroom — borderline, but producer can shave header comments if it goes over)

# Long-line check (≤5 per file is tolerable per R18 relaxed rule)
$lines = [System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8)
$longLines = 0
for ($i = 0; $i -lt $lines.Length; $i++) {
    if ($lines[$i].Length -gt 120) { $longLines++ }
}
# Expected: ≤5
```

### Task 5: Create `manager_connection_start.rs` (file A)

**Files:**
- Create: `src/crates/interfaces/acp/src/client/manager_connection_start.rs`

**Steps:**

- [ ] **Step 5.1: Create the new file with sub-domain A content (verbatim from main)**

Use the exact code below. The 3 methods (initialize_all, start_client_for_session, start_client_connection) are moved verbatim from `src/crates/interfaces/acp/src/client/manager_connection.rs:47-238`. The `start_client_connection` is the 147-line fn QClaw flagged; preserve it exactly.

```rust
// R20c split: ACP client connection start lifecycle.
// File: src/crates/interfaces/acp/src/client/manager_connection_start.rs
// Origin: manager_connection.rs (287 lines, QClaw R20a P2 D-deviation +19%
//        over QClaw 242 tolerance)
// Mavis fix: R20c split manager_connection.rs into 2 files (this +
//        manager_connection_stop.rs) to close the 242 line cap. Sub-domain A:
//        connection start lifecycle (3 pub methods, includes the
//        147-line start_client_connection QClaw flagged in R20a review).
// R20c sibling: manager_connection_stop.rs (sub-domain B)
// R20c sibling: manager_config_loading.rs + manager_config_requirements.rs
// R19 sibling files (consumers of start methods):
//             manager_session.rs (start_client_for_session for session init)
//             manager_install.rs (initialize_all for fresh install)
//             manager_prompt.rs (... possibly)
//
// All method bodies are moved verbatim from main. No behavior change.

use super::manager::{...};  // (verbatim from main)
// ... (other verbatim imports)

impl AcpClientService {
    pub async fn initialize_all(&self, ...) -> ... { ... }
    pub async fn start_client_for_session(&self, ...) -> ... { ... }
    pub async fn start_client_connection(&self, ...) -> ... { ... }
    // ↑ start_client_connection is the 147-line fn QClaw flagged in R20a
    //   (line 92-238 in main). Preserve byte-for-byte.
}
```

> **Producer note:** the `start_client_connection` body is 147 lines and includes nested `cleanup_failed_startup(...)` calls (which lives in the stop.rs sibling — cross-sibling inherent dispatch, no `use` import needed). Preserve verbatim.

- [ ] **Step 5.2: Verify file canonical line count + ≤5 long lines tolerance**

```powershell
$path = 'src/crates/interfaces/acp/src/client/manager_connection_start.rs'
$canon = [System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8).Count
# Expected: ~220 (cap 242, -9% headroom)

# Long-line check
$lines = [System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8)
$longLines = 0
for ($i = 0; $i -lt $lines.Length; $i++) {
    if ($lines[$i].Length -gt 120) { $longLines++ }
}
# Expected: ≤5
```

### Task 6: Create `manager_connection_stop.rs` (file B)

**Files:**
- Create: `src/crates/interfaces/acp/src/client/manager_connection_stop.rs`

**Steps:**

- [ ] **Step 6.1: Create the new file with sub-domain B content (verbatim from main)**

Use the exact code below. The 3 methods (cleanup_failed_startup, stop_client, stop_connection) are moved verbatim from `src/crates/interfaces/acp/src/client/manager_connection.rs:239-end`.

```rust
// R20c split: ACP client connection stop lifecycle.
// File: src/crates/interfaces/acp/src/client/manager_connection_stop.rs
// Origin: manager_connection.rs (287 lines, QClaw R20a P2 D-deviation)
// Mavis fix: R20c sub-domain B: connection stop lifecycle (3 pub methods).
// R20c sibling: manager_connection_start.rs (sub-domain A)
// R20c sibling: manager_config_loading.rs + manager_config_requirements.rs
// R19 sibling files (consumers of stop methods):
//             manager_session_lifecycle.rs (stop_connection for release_northhing_session)
//             manager_install.rs (... possibly)
//             start_client_connection in manager_connection_start.rs (cleanup_failed_startup for
//                                                                failed-start cleanup)
//
// All method bodies are moved verbatim from main. No behavior change.

use super::manager::{...};  // (verbatim from main)
// ... (other verbatim imports)

impl AcpClientService {
    pub async fn cleanup_failed_startup(&self, ...) -> ... { ... }
    pub async fn stop_client(&self, ...) -> ... { ... }
    pub async fn stop_connection(&self, ...) -> ... { ... }
}
```

> **Producer note:** same as previous — copy-paste verbatim from `manager_connection.rs:239-end`. `cleanup_failed_startup` is called from `start_client_connection` in `manager_connection_start.rs` via inherent dispatch — no `use` import needed for cross-sibling inherent dispatch.

- [ ] **Step 6.2: Verify file canonical line count + ≤5 long lines tolerance**

```powershell
$path = 'src/crates/interfaces/acp/src/client/manager_connection_stop.rs'
$canon = [System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8).Count
# Expected: ~75 (cap 242, -69% headroom)

# Long-line check
$lines = [System.IO.File]::ReadAllLines($path, [System.Text.Encoding]::UTF8)
$longLines = 0
for ($i = 0; $i -lt $lines.Length; $i++) {
    if ($lines[$i].Length -gt 120) { $longLines++ }
}
# Expected: ≤5
```

### Task 7: Update `mod.rs` + delete original files

**Files:**
- Modify: `src/crates/interfaces/acp/src/client/mod.rs` (2 lines replaced with 4)
- Delete: `src/crates/interfaces/acp/src/client/manager_config.rs`
- Delete: `src/crates/interfaces/acp/src/client/manager_connection.rs`

**Steps:**

- [ ] **Step 7.1: Update `mod.rs` to register 4 new modules**

Edit `src/crates/interfaces/acp/src/client/mod.rs`:

```diff
+mod manager_config_loading;
+mod manager_config_requirements;
-mod manager_config;
-mod manager_connection;
+mod manager_connection_start;
+mod manager_connection_stop;
```

(Adjustment to match actual `mod.rs` content — read it first in Step 7.1.a.)

- [ ] **Step 7.1.a: Read the actual `mod.rs` to determine the right edit**

```powershell
$path = 'src/crates/interfaces/acp/src/client/mod.rs'
Get-Content $path -Encoding UTF8 | ForEach-Object -Begin {$i=1} -Process { Write-Host "${i}: $_"; $i++ }
# Identify the lines that have `mod manager_config;` and `mod manager_connection;`
# and replace with 4 new mod declarations in alphabetical order.
```

- [ ] **Step 7.2: Verify no remaining `manager_config::` or `manager_connection::` references (should already be 0 per §1.3)**

```powershell
Select-String -Path src\crates\interfaces\acp\src\client\*.rs -Pattern 'use super::manager_config[^_a-z]|use super::manager_connection[^_a-z]|manager_config::|manager_connection::' -SimpleMatch
# Expected: 0 matches
```

(If any hits appear, they are pre-existing R19 code that needs review — flag in handoff.)

- [ ] **Step 7.3: Delete the 2 original files**

```bash
git rm src/crates/interfaces/acp/src/client/manager_config.rs
git rm src/crates/interfaces/acp/src/client/manager_connection.rs
# Expected: rm ... manager_config.rs
#           rm ... manager_connection.rs
```

### Task 8: Verify (mandatory checks per R19 + R20a + R20b lessons)

**Files:** none (verification only)

**Steps:**

- [ ] **Step 8.1: cargo check — `northhing-acp` lib**

```bash
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo check -p northhing-acp 2>&1 | Tee-Object -FilePath "C:\Users\UmR\AppData\Local\Temp\r20c-acp-check.log" | Out-Null
$lines = [System.IO.File]::ReadAllLines('C:\Users\UmR\AppData\Local\Temp\r20c-acp-check.log', [System.Text.Encoding]::UTF8)
$errs = $lines | Select-String -Pattern '^error\[E'
Write-Host "errors: $($errs.Count)"
$errs | ForEach-Object { Write-Host $_.Line }
# Expected: 0 errors
```

If errors:
- `E0624 (method is private)`: wrong visibility on a moved method → verify all 14 methods are `pub fn` (or `pub async fn`) per §2.3
- `E0609 (no field/variant on struct)`: mis-copied body → diff against main
- `E0432 (use of undeclared module)`: missing `mod` declaration in `mod.rs` → re-run Step 7.1

- [ ] **Step 8.2: cargo check — cross-crate (R19 + R20a lessons) + workspace**

```bash
cargo check -p northhing-cli 2>&1 | Tee-Object -FilePath "C:\Users\UmR\AppData\Local\Temp\r20c-cli-check.log" | Out-Null
$lines = [System.IO.File]::ReadAllLines('C:\Users\UmR\AppData\Local\Temp\r20c-cli-check.log', [System.Text.Encoding]::UTF8)
$errs = $lines | Select-String -Pattern '^error\[E'
Write-Host "cli errors: $($errs.Count)"
$errs | ForEach-Object { Write-Host $_.Line }
# Expected: 2 pre-existing E0624 on session_manager.get_session (NOT introduced by R20c — pre-existing on main HEAD f579c71, NOT R20c scope; Mavis can address in a follow-up Mavis-fix commit per "Mavis owns all refactor in this project" rule)

cargo check --workspace 2>&1 | Tee-Object -FilePath "C:\Users\UmR\AppData\Local\Temp\r20c-ws-check.log" | Out-Null
$lines = [System.IO.File]::ReadAllLines('C:\Users\UmR\AppData\Local\Temp\r20c-ws-check.log', [System.Text.Encoding]::UTF8)
$errs = $lines | Select-String -Pattern '^error\[E'
Write-Host "workspace errors: $($errs.Count)"
$errs | ForEach-Object { Write-Host $_.Line }
# Expected: 2 NEW errors matching the 2 cli errors (since cli is part of workspace)
# All 2 errors are pre-existing session_manager.get_session E0624 — NOT R20c regression
```

- [ ] **Step 8.3: cargo test — `northhing-acp` lib baseline**

```bash
cargo test -p northhing-acp --lib 2>&1 | Select-String -Pattern 'test result:|error\[E'
# Expected: test result: ok. 51 passed; 0 failed; 0 ignored (matches R20a/R20b baseline)
```

- [ ] **Step 8.4: rustfmt --edition 2021 check (per R20a/R20b standard)**

```bash
rustfmt --edition 2021 --check \
  src/crates/interfaces/acp/src/client/manager_config_loading.rs \
  src/crates/interfaces/acp/src/client/manager_config_requirements.rs \
  src/crates/interfaces/acp/src/client/manager_connection_start.rs \
  src/crates/interfaces/acp/src/client/manager_connection_stop.rs \
  src/crates/interfaces/acp/src/client/mod.rs 2>&1
# Expected: 0 diffs on the 4 new files (mod.rs may have pre-existing diffs at line 10/16 from R20a-trailing-cycle, which are NOT R20c's concern)
```

- [ ] **Step 8.5: Iron rules — no new unwrap/expect/panic/let _ = Result**

```bash
# Pre-impl baseline (sum of 2 original files on main)
$pre = (git show main:src/crates/interfaces/acp/src/client/manager_config.rs | Select-String -Pattern '\bunwrap\(|\bexpect\(|panic!|unreachable!|let _\s*=\s*Result' | Measure-Object).Count + (git show main:src/crates/interfaces/acp/src/client/manager_connection.rs | Select-String -Pattern '\bunwrap\(|\bexpect\(|panic!|unreachable!|let _\s*=\s*Result' | Measure-Object).Count
Write-Host "Pre-split sum: $pre"

# Post-impl (sum of 4 new files)
$paths = @(
  'src/crates/interfaces/acp/src/client/manager_config_loading.rs',
  'src/crates/interfaces/acp/src/client/manager_config_requirements.rs',
  'src/crates/interfaces/acp/src/client/manager_connection_start.rs',
  'src/crates/interfaces/acp/src/client/manager_connection_stop.rs'
)
$post = 0
foreach ($p in $paths) {
  $count = (Select-String -Path $p -Pattern '\bunwrap\(|\bexpect\(|panic!|unreachable!|let _\s*=\s*Result').Count
  $post += $count
  Write-Host "$($p | Split-Path -Leaf): $count"
}
Write-Host "Post-split sum: $post"
# Expected: $post -le $pre (any delta indicates new code; should be 0 because split is pure structural)
```

- [ ] **Step 8.6: Cargo.lock drift check**

```bash
git diff main..HEAD -- Cargo.lock | wc -l
# Expected: 0
```

- [ ] **Step 8.7: CRLF / line-ending check**

```bash
file src/crates/interfaces/acp/src/client/manager_config_*.rs src/crates/interfaces/acp/src/client/manager_connection_*.rs
# Expected: "UTF-8 text" or "ASCII text" for all 4, NO "with CRLF" mention
```

- [ ] **Step 8.8: Canonical line count final check**

```powershell
foreach ($p in @(
  'src/crates/interfaces/acp/src/client/manager_config_loading.rs',
  'src/crates/interfaces/acp/src/client/manager_config_requirements.rs',
  'src/crates/interfaces/acp/src/client/manager_connection_start.rs',
  'src/crates/interfaces/acp/src/client/manager_connection_stop.rs'
)) {
  $canon = [System.IO.File]::ReadAllLines($p, [System.Text.Encoding]::UTF8).Count
  $status = if ($canon -le 242) { 'OK' } else { 'OVER' }
  Write-Host "$($p | Split-Path -Leaf): $canon $status (cap 242)"
}
# Expected: all 4 OK
```

- [ ] **Step 8.9: Method count preserved**

```powershell
$total = 0
foreach ($p in @(
  'src/crates/interfaces/acp/src/client/manager_config_loading.rs',
  'src/crates/interfaces/acp/src/client/manager_config_requirements.rs',
  'src/crates/interfaces/acp/src/client/manager_connection_start.rs',
  'src/crates/interfaces/acp/src/client/manager_connection_stop.rs'
)) {
  $count = (Select-String -Path $p -Pattern 'pub(?:\([^)]+\))? (async )?fn |^(async )?fn ' | Measure-Object).Count
  $total += $count
  Write-Host "$($p | Split-Path -Leaf): $count fns"
}
Write-Host "TOTAL: $total (expected 14)"
```

- [ ] **Step 8.10: Cross-crate consumer check (R19 + R20a + R20b lessons)**

```bash
# No NEW cross-crate refs to the new sub-domain modules (matches R20b pattern)
git grep -n 'manager_config_loading::' -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/' | wc -l
# Expected: 0
git grep -n 'manager_config_requirements::' -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/' | wc -l
# Expected: 0
git grep -n 'manager_connection_start::' -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/' | wc -l
# Expected: 0
git grep -n 'manager_connection_stop::' -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/' | wc -l
# Expected: 0
```

### Task 9: Commit + handoff

**Files:**
- Stage: 4 new files + modified `mod.rs` + deletion of 2 original files
- Create: `docs/handoffs/2026-07-01-r20c-manager-config-connection-split-impl.md` (impl handoff per R20a pattern)

**Steps:**

- [ ] **Step 9.1: Stage all changes**

```bash
git add \
  src/crates/interfaces/acp/src/client/manager_config_loading.rs \
  src/crates/interfaces/acp/src/client/manager_config_requirements.rs \
  src/crates/interfaces/acp/src/client/manager_connection_start.rs \
  src/crates/interfaces/acp/src/client/manager_connection_stop.rs \
  src/crates/interfaces/acp/src/client/mod.rs
# Note: deletions are staged separately
git rm src/crates/interfaces/acp/src/client/manager_config.rs
git rm src/crates/interfaces/acp/src/client/manager_connection.rs
```

- [ ] **Step 9.2: Write impl handoff doc**

Create `docs/handoffs/2026-07-01-r20c-manager-config-connection-split-impl.md` following the R20a pattern (header + summary + per-file table + per-method mapping + verification results).

- [ ] **Step 9.3: Commit**

```bash
git add docs/handoffs/2026-07-01-r20c-manager-config-connection-split-impl.md
git commit -m "refactor(bitfun-acp): R20c close QClaw R20a P2 D-deviations (manager_config 292 + manager_connection 287 → 4 files)"
```

Commit body should follow the R20a pattern: 1-2 paragraph summary, file inventory table, per-method mapping (old path → new path), spec deviation section (if any), verification results (cargo check/test/rustfmt outputs).

- [ ] **Step 9.4: Final Mavis 10-axis verification (per R20a/R20b retro)**

Mavis runs the 10-axis verification after the producer's commit lands:
1. Canonical line counts (all 4 new files ≤242, 2 files deleted)
2. Sub-domain coherence (load/requirements + start/stop splits, no cross-boundary helpers)
3. Visibility pattern (11 `pub fn`/`pub async fn` + 3 file-local `fn` in config_loading — match §2.3)
4. Iron rules (0 new unwrap/expect/panic/let _ = Result; pre = post baseline preserved)
5. Line endings (0 CRLF, all LF)
6. Line length (≤5 long lines per file)
7. Cargo.lock drift (0)
8. Cargo check (acp = 0 errors; cli = 2 pre-existing E0624; workspace = 2 pre-existing; **Mavis to address cli pre-existing in a follow-up commit per "Mavis owns all refactor in this project" rule**)
9. Cargo test (bitfun-acp 51/0/0 preserved, core 899/0/1 preserved)
10. Cross-crate consumer check (0 NEW E06xx; 0 NEW direct module refs outside `acp`)

### Task 10: Mavis follow-up — fix the pre-existing 2 E0624 on session_manager (tech debt)

Per user's "整个项目的重构代码部分都是 Mavis 的" rule, the pre-existing 2 E0624 on `apps/cli` calling `pub(crate) fn get_session` should be fixed in a follow-up commit. Same root cause as R20a `fe87083` and R20b `5424460`.

- [ ] **Step 10.1: Apply the visibility fix**

```bash
# Verify the file
Select-String -Path src\crates\assembly\core\src\agentic\session\session_manager_lifecycle.rs -Pattern 'pub fn get_session|pub\(crate\) fn get_session'
# Expected: 1 match (pub(crate) — needs fix)

# Apply Edit
# oldString: '    pub(crate) fn get_session(&self, session_id: &str) -> Option<Session> {'
# newString: '    pub fn get_session(&self, session_id: &str) -> Option<Session> {'
```

- [ ] **Step 10.2: Verify cargo check cli re-passes**

```bash
cargo check -p northhing-cli 2>&1 | Select-String -Pattern 'error\[E|Finished'
# Expected: 0 errors
```

- [ ] **Step 10.3: Commit**

```bash
git add src/crates/assembly/core/src/agentic/session/session_manager_lifecycle.rs
git commit -m "fix(session-manager): make get_session pub (cli cross-crate E0624 cleanup, R20c worktree)"
# Commit message follows the same template as R20a fe87083 and R20b 5424460.
```

### Task 11: Stage review prep (for the next "stage review" task)

After R20c commit lands + Mavis fix lands, prepare for the stage review by:

- [ ] **Step 11.1: Generate review guide**

Create `docs/handoffs/2026-07-01-r20a-r20b-r20c-stage-review.md` (or similar) that documents:
- R20a (Critical D-deviation closed)
- R20b (P1 closed)
- R20c (P2 closed)
- Mavis fix follow-ups (fe87083 + R20b 5424460 + R20c follow-up)
- Stage-wide 10-axis verification
- Stage-wide cross-crate consumer check
- Stage-wide iron rules

This is the document Kimi/QClaw reads to review the full R20a-R20c stage as a single unit.

---

## 4. Pre-emptive lessons (R18 + R19 + R20a + R20b)

| Round | Lesson | R20c application |
|---|---|---|
| R18 | Long-line tolerance: ≤5 new long lines per file | Tasks 4.2 + 5.2 + 6.2 + 8.4 enforce this |
| R18 | `Measure-Object -Line` is wrong, use `ReadAllLines().Count` or `wc -l` | All canonical line counts in this spec use canonical method |
| R19 | `pub(super)` over-prescription caused 11 E0624 errors. Default `pub` for cross-crate, `pub(super)` only for cross-sibling inherent dispatch | §2.2: all 14 methods are sibling-consumed via inherent dispatch; 0 cross-crate callers; default to `pub fn` |
| R19 | Pre-emptively extend timeout at dispatch for splits >1000 lines | Task 2.1: 579 lines is between 1000 (R20a retro threshold) and 2000 (R19 rule); +60 min extend is cheap insurance |
| R19 | Cross-crate consumer verification (`cargo check -p northhing-cli` not just target crate) | Task 8.2: explicit cli + workspace check |
| R19 | Reviewer uses `grep -cE '^error\['` against `--message-format=short` which hides errors | Task 8.1 + 8.2 use raw output, not short format |
| R20a | R20a's Mavis visibility fix (fe87083) — same root cause as R20b's Mavis fix (5424460) and R20c's Mavis fix (Task 10) | Task 10: explicit follow-up commit for the same pre-existing pub(crate) E0624 |
| R20a | Lifecycle over-cap was caught pre-emptively by Mavis 10-axis verification | Task 9.4: Mavis runs 10-axis check after producer's commit |
| R20a | spec `pub mod` claim vs code `mod` (cosmetic) | R20c spec uses correct `mod` (private) terminology throughout |
| R20a | BOM introduced by PowerShell `[Encoding]::UTF8` WriteAllText | Task 8.7: post-write verify with raw `file` command or byte check; if BOM, strip with Python |
| R20a | Trailing newline missing in 2 R20a-touched files | Task 8.7: all 4 new files end with LF (verify with byte check) |
| R20b | Producer caught spec deviation D1 (6th caller file which spec missed) | R20c: explicitly listed all 8 + 6 methods in §1.2; zero external callers verified; spec deviation D2 is "pre-existing cli E0624 not in R20c scope" — handled by Task 10 |
| R20b | Producer session entered error state mid-verification, but commit was already in place | R20c: same 32-min cap; pre-emptive +60 min extend gives 92 min window; Mavis take-over pattern documented in Task 9.4 |
| R20b | "0 fns dropped" = 16 = 4+4+6+2; spec deviation D1 accepted as in-scope | R20c: "0 fns dropped" = 14 = 1+3+3+3; spec deviation D2 is pre-existing cli E0624 handled by Task 10 |

---

## 5. Risks + mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Producer copy-paste mistake drops a method body | Medium | High (E0609 compile error) | Task 8.1 + 8.2 cargo check catches it; reviewer diff against main |
| Producer applies `pub(super)` over-prescription (R19 lesson) | Low | High (re-introduces 11 E0624 regression) | §2.2 + §2.3 explicit visibility table + Task 8.1 review |
| 2 new files (manager_config_requirements.rs at ~230 + manager_connection_start.rs at ~220) individually exceed 242 cap | Low | Medium (over-cap D-deviation) | Task 4.2 + 5.2 + 8.8 line count verification |
| Producer adds new code (refactor on top of split) | Low | High (R20a baseline preserved only if no behavior change) | Spec §1.1 explicit: "Pure structural split — no behavior change, no method-body refactoring"; reviewer enforces |
| Pre-existing 2 E0624 on session_manager.get_session pollute the cli check | High (deterministic) | Medium (cli check fails; technically NOT R20c regression) | Task 8.2 explicit: "Expected: 2 pre-existing E0624 ... NOT introduced by R20c — pre-existing on main HEAD f579c71"; Task 10: Mavis follow-up fix to make cli check pass |
| Producer session enters error state mid-verification (R20b pattern) | Medium | Medium (Mavis take-over) | Task 2.1 pre-emptive +60 min timeout; R20a retro pattern: Mavis 5-min finish the verification |
| Reviewer (Kimi) disagrees with 2-way (load+use / start+stop) split, prefers 3-way or 4-way | Low | Medium (rework) | §1.2 documents 2-way rationale; QClaw P2 says "2 files" so 2-way is in spec range; if Kimi insists 3 or 4, can re-run |
| Mavis-fix on session_manager_lifecycle.rs causes R20a + R20b branches merge conflict | Medium (deferred) | Low (this is Mavis's 3rd time fixing the same issue across 3 branches) | Documented in commit message + handoff: same root cause as fe87083 and 5424460; when all 3 branches land, the same 1-line change is in all 3 commits, no conflict |

---

## 6. Self-review (writing-plans standard)

### 6.1 Spec coverage

| Spec requirement | Task implementing it |
|---|---|
| Close QClaw R20a P2 D-deviations (config 292 + connection 287) | Tasks 3-7 (split), Task 9.3 (commit) |
| 2 files each → 2-way split (QClaw range) | Tasks 3, 4 (config) + Tasks 5, 6 (connection) |
| Pure structural split, no behavior change | Tasks 3.1, 4.1, 5.1, 6.1 (verbatim body copy), Task 8.5 (iron rules) |
| 0 caller file updates (inherent methods) | Task 7.2 (verify 0 remaining `manager_config::`/`manager_connection::` refs) |
| Cargo check + test baseline preserved | Tasks 8.1, 8.2, 8.3 |
| Visibility pattern (R19 lesson) | §2.2 + Task 8 (review) |
| Canonical line measurement (R18 lesson) | Tasks 1.2, 3.2, 4.2, 5.2, 6.2, 8.8 |
| Long-line tolerance (R18 lesson) | Tasks 4.2, 5.2, 6.2, 8.4 |
| Pre-emptive timeout extend (R19 lesson) | Task 2.1 |
| Cross-crate consumer check (R19 lesson) | Task 8.10 |
| BOM / CRLF / LF hygiene (R20a lesson) | Task 8.7 |
| Mavis 10-axis verification | Task 9.4 |
| Mavis follow-up fix for pre-existing E0624 | Task 10 |
| Stage review prep | Task 11 |

### 6.2 Placeholder scan

Searched for: `TBD`, `TODO`, `implement later`, "add appropriate error handling", "similar to Task N", "to be determined".

- 4 placeholders found: `// ... (verbatim body ...)` comments in Tasks 3.1, 4.1, 5.1, 6.1.
  - **Why this is intentional**: these are 14 separate method bodies (1 A + 3 C private + 3 B + 1 D + 3 A + 3 B = 14 methods, totaling ~243 + ~234 = ~477 src lines of business logic) that the producer must copy-paste verbatim from main. Showing the FULL body in this spec would push the spec to 1500+ lines and obscure the structural intent. The producer has direct read access to the source files.
  - **Why this is acceptable** (per writing-plans skill "No Placeholders" rule, **exception documented explicitly**): the bodies are bit-for-bit copy of existing code, not new logic. The reviewer (Kimi) will diff the new files against main to verify zero behavior change. This is a producer-mechanical step, not a design decision.
  - **Mitigation**: the producer's commit message MUST include a `git diff main:src/crates/interfaces/acp/src/client/manager_config.rs HEAD:src/crates/interfaces/acp/src/client/manager_config_*.rs` summary AND `git diff main:src/crates/interfaces/acp/src/client/manager_connection.rs HEAD:src/crates/interfaces/acp/src/client/manager_connection_*.rs` summary proving zero behavior change.

- No other placeholders found. All other steps have full content.

### 6.3 Type consistency

Checked method signatures across tasks. All 14 method signatures are preserved verbatim from main (no renames, no parameter changes, no return type changes). The cross-sibling inherent dispatch (e.g., `start_client_connection` calling `cleanup_failed_startup` across sibling files) works without `use` imports because Rust resolves methods via inherent impl blocks on the type.

No signature drift detected.

---

## 7. Subagent dispatch handoff (Mavis → producer agent)

The producer agent dispatched by `mavis team plan` should follow this spec task-by-task. The producer's verification step is Task 8 (cargo check + test + rustfmt + iron rules + Cargo.lock). The producer's commit is Task 9.3.

After the producer's commit lands, Mavis runs Task 9.4 (10-axis verification) before sending to Kimi for review. Mavis also runs Task 10 (visibility follow-up) to address the pre-existing 2 E0624 on session_manager.

The reviewer (Kimi) reads:
1. This spec (`docs/handoffs/2026-07-01-r20c-manager-config-connection-split-spec.md`)
2. The producer's impl handoff (`docs/handoffs/2026-07-01-r20c-manager-config-connection-split-impl.md`)
3. The producer's commit (`git show HEAD` — should show: 2 deleted files + 4 new files + 1 modified mod.rs = 7 file changes, 1 commit)
4. Mavis's follow-up commit (visibility fix per Task 10)

The reviewer verifies:
- All 4 new files ≤ 242 cap (canonical wc-l)
- All 14 methods preserved verbatim (diff against main `manager_config.rs:42-end` and `manager_connection.rs:47-end`)
- 0 new unwrap/expect/panic/let _ = Result
- 0 CRLF, all LF
- 0 Cargo.lock drift
- `cargo check -p northhing-acp`: 0 errors
- `cargo check -p northhing-cli`: 0 errors (after Mavis Task 10 fix)
- `cargo check --workspace`: 0 errors
- `cargo test -p northhing-acp --lib`: 51 passed, 0 failed (R20a/R20b baseline preserved)
- `git grep -n 'manager_config::|manager_connection::' -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/'`: 0 hits (no NEW cross-crate refs)
- Visibility: 11 `pub fn` / `pub async fn` (1 A + 4 B+D + 3 A connection + 3 B connection) + 3 plain `fn` (3 file-local in `manager_config_loading.rs`)

---

*Spec authored by Mavis on 2026-07-01. R20c scope = QClaw R20a P2 D-deviations (manager_config.rs 292 + manager_connection.rs 287). Branch `impl/r20c-manager-config-connection-split` (forked from main `f579c71`, not from R20a/R20b branches). Estimated producer runtime: 40-50 min for the 2-file split (vs R20a 37 min for 1 file, R20b 32 min for 1 file). 1 producer commit + 1 spec commit + 1 Mavis follow-up fix + 1 impl handoff doc + 1 stage review prep doc.*
