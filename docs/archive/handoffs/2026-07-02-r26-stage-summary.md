# R26 stage summary — `contracts/runtime-ports/src/lib.rs` 2460 → facade 863 + 4 sibling

> R26 god-object split **SUCCEEDED** on second take-over attempt.
> lib.rs 2460 → facade 863 (-65%), split into 4 sibling sub-domain files.
> All tests green (43 runtime-ports + 102 consumer crates), 0 errors
> workspace-wide, 0 regressions.

## Result

| Metric | Value |
|---|---|
| lib.rs before | 2460 lines |
| lib.rs after (facade) | 863 lines (-65%) |
| 4 sibling files | 851 + 588 + 151 + 98 (total 1688 lines) |
| Total lines | 2551 (vs 2460; +91 from headers/use blocks) |
| `cargo check -p northhing-runtime-ports` | 0 errors |
| `cargo check --workspace` | 0 errors |
| `cargo test -p northhing-runtime-ports` | 43 passed, 0 failed |
| Consumer crates (5) tests | 102 passed, 0 failed |
| Cargo.lock drift | none |

## Sibling layout

| File | Lines | Sub-domain |
|---|---|---|
| `port_core.rs` | 98 | PortError, PortErrorKind, PortResult, RuntimeServiceCapability, RuntimeServicePort (base) |
| `session_workspace.rs` | 588 | Session storage types, Workspace FS/Shell/Services, Permission, Clock, Terminal, Network, Git, McpCatalog, RemoteConnection |
| `remote.rs` | 151 | RemoteWorkspaceKind, Remote*, RemoteWorkspacePort, RemoteInitialSyncRuntimeHost, RemoteProjectionPort, RemoteCapabilityPort |
| `agent.rs` | 851 | AgentSession*, AgentDialog*, ThreadGoal*, CompressionContract, AgentInputAttachment, Agent*Port traits, RemoteControl, RuntimeEvent*, ConfigReadPort, SessionTranscript*, DelegationPolicy, SubagentContextMode |

## Sub-domain boundaries

- **port_core**: error/result types + base port trait (4 items)
- **session_workspace**: file/shell/process/permission/clock/network/git/mcp/remote-connection port traits (28 items, largest sibling)
- **remote**: remote-specific runtime hosts + projection/capability ports (12 items)
- **agent**: agent session, dialog, thread-goal, dialog round injection, submission, lifecycle, turn-cancellation, remote control, runtime event, dynamic tool, config read, session transcript, delegation policy, subagent context (60+ items)

The first attempt tried 5 siblings (separating agent_dialog from submission_events) but had heavy cross-references between them (AgentSubmissionPort uses AgentSessionCreateRequest, etc.) — 37 errors. **Merging into single `agent.rs` sibling** reduced cross-refs to one `use super::port_core::{PortError, PortResult};` import.

## Cross-sibling use

- `session_workspace.rs`: `use super::port_core::{PortError, PortResult, RuntimeServicePort};`
- `remote.rs`: `use super::port_core::RuntimeServicePort; use super::session_workspace::WorkspaceFileSystem;` (unused warning, kept for clarity)
- `agent.rs`: `use super::port_core::{PortError, PortResult};`

## Visibility: `pub` (NOT `pub(super)`)

`northhing-runtime-ports` is an **interface crate** per AGENTS.md. All items are `pub` to enable `pub use {module}::*` re-exports in lib.rs facade for cross-crate API stability. This is the opposite of R23/R24 impl-block splits which use `pub(super)` for cross-sibling internal helpers.

## Off-by-one / extraction lessons

1. **f-string `{{` and `}}` literal** in Python script → produces `{{` and `}}` in output. Use single `{` and `}` for the actual brace (Python f-string `{{` = literal `{`). 4 sibling files all had `use serde::{{Deserialize, Serialize}};` (broken) until fixed via `Replace -replace 'use serde::\{\{Deserialize, Serialize\}\};'`.
2. **Range off-by-one**: original agent range (846, 1273) = L847-L1274 was supposed to include the `#[derive(...)]` line for the first struct, but extracted content started at `#[serde(...)]` (L848). Lost 1 line. Fix: add `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` before the first `#[serde(...)]` in agent.rs L10-11.
3. **`#[serde(...)]` attribute scope**: requires `#[derive(Serialize, Deserialize)]` OR `use serde;` in scope BEFORE the first `#[serde(...)]`. Without it: `error: cannot find attribute 'serde' in this scope`.
4. **Range end dangling `#[derive(...)]`**: my remote range (704, 845) = L705-L846 captured the first `#[derive(...)]` of the NEXT range (agent_dialog at L847). Trimmed the dangling line.

## Cross-crate consumer verification (R19 lesson applied)

5 consumer crates that use `northhing-runtime-ports`:
- `northhing-services-integrations` (99 tests passed)
- `northhing-runtime-services` (3 tests passed)
- `northhing-agent-runtime` (0 tests, 0 errors)
- `northhing-agent-tools` (0 tests, 0 errors)
- `northhing-product-capabilities` (0 tests, 0 errors)

All 5 compile clean. `pub use {module}::*` re-exports in lib.rs facade preserve public API.

## Refs

- R26 spec: `docs/handoffs/2026-07-02-r26-runtime-ports-split-spec.md` (5 sibling original plan)
- R24 stage summary (sibling R22-R24 impl-block split pattern): `docs/handoffs/2026-07-02-r24-stage-summary.md`
- R25 stage summary (deferred): `docs/handoffs/2026-07-02-r25-stage-summary.md` (lessons applied to R26)
- R21+ parallel sub-rounds flow: `memory/northing-god-object-split.md` R21+ 段
- AGENTS.md: `src/crates/contracts/runtime-ports/AGENTS.md` (interface crate constraints)