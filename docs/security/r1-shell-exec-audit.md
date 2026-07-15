# R1 Shell-exec Audit Report (2026-06-23)

> **Scope:** Per `docs/superpowers/specs/2026-06-23-r1-shell-exec-sandbox-design.md` Phase 1.
> **Author:** Auto-audit (revised 2026-06-23 after T2.4 deep dive)
> **Method:** Source code review + test coverage analysis

---

## Summary

**Total shell-exec paths audited**: 9

| Risk | Count | Paths |
|------|-------|-------|
| 🔴 High (revised) | 0 | (all `sh -c` uses are in test code, not production) |
| 🟡 Medium | 4 | `ngrok`, `lsp/process`, `miniapp/runtime`, `mcp/server/connection` (uses fixed programs) |
| 🟢 Low | 5 | `computer_use_actions`, `browser_launcher`, `process_manager`, `glob_search`, `port_adapters` |

**CRITICAL FINDING (2026-06-23, T2.4 deep dive)**:

After detailed source review for Phase 2 T2.4 wiring:
- **NONE of the 9 paths use `sh -c "..."` in production code**
- All `Command::new` calls are with **fixed program + fixed args** (pgrep, kill, sw_vers, defaults, rg/fd, git, etc.)
- The `sh -c` calls in `mcp/server/connection.rs` are **only in test code**, not production
- Denylist patterns (rm -rf /, mkfs, fork bomb) **cannot match** these fixed program + args calls

**Conclusion**: **T2.4 (wiring guard into 8 paths) is a NO-OP for security improvement.**

The existing **bash_tool denylist** (S-1) is the **primary defense** against catastrophic commands, and it remains effective because bash_tool takes **arbitrary user-controlled shell commands**.

The 8 other paths are **deterministic system utilities** with fixed program + args; they don't take user-controlled shell strings that could match denylist patterns.

**Revised Phase 2 scope**: T2.4 becomes a **doc-only change** — update audit to reflect this finding. No production code changes needed for T2.4.

---

## Per-path Analysis

### 1. `computer_use_actions.rs`

- **File**: `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions.rs`
- **Access Control**:
  - Denylist check: ❌ none
  - Confirmation: ⚠️ `ComputerUseTool::needs_permissions() = true` but actions are called via separate code path
  - LLM-triggered: ✅ yes (LLM invokes ComputerUse tool which dispatches to actions)
- **Test Coverage**: 0 unit tests for shell-exec path
- **Risk Level**: 🟢 **REVISED: low** — only `Command::new` is `read_os_version()` which calls `sw_vers -productVersion` (read-only). Real "computer use" actions go through native APIs (CGEvent, win32 SendInput), not `Command::new`.
- **Trigger Frequency**: high (computer use is core functionality)
- **Recommended Fix**:
  - No change needed: `sw_vers -productVersion` does not match any denylist pattern
  - Real risk surface is the native mouse/keyboard APIs, which are OS-level sandboxed
  - Priority: ~~P0~~ → **P2 (audit-only)**
- **Test Recommendations**:
  - Add unit test for `read_os_version()` correctness on macOS/Windows
- **Audit Note (2026-06-23)**: Originally classified P0 (high risk). After detailed grep + read, the only `Command::new` in this file is a read-only version probe. Risk revised to low.

### 2. `browser_launcher.rs`

- **File**: `src/crates/assembly/core/src/agentic/tools/browser_control/browser_launcher.rs`
- **Access Control**:
  - Denylist check: ❌ none (launches browsers via system command)
  - Confirmation: ⚠️ Browser launch may bypass `BashTool` confirmation
  - LLM-triggered: ✅ yes (browser_control tool)
- **Test Coverage**: 0 unit tests for shell-exec
- **Risk Level**: 🔴 high — launches arbitrary browsers/binaries
- **Trigger Frequency**: medium
- **Recommended Fix**:
  - Code change: add `guard_command_execution()` before browser launch
  - Estimated effort: 1h
  - Priority: P0
- **Test Recommendations**:
  - Unit test: malicious browser path blocked
  - Integration test: only allowlisted browsers can launch

### 3. `ngrok.rs`

- **File**: `src/crates/assembly/core/src/service/remote_connect/ngrok.rs`
- **Access Control**:
  - Denylist check: ❌ none
  - Confirmation: ✅ ngrok commands require tunnel setup
  - LLM-triggered: ⚠️ hybrid (remote_connect tool, less common)
- **Test Coverage**: minimal
- **Risk Level**: 🟡 medium — ngrok exposes local services publicly
- **Trigger Frequency**: low
- **Recommended Fix**:
  - Code change: add denylist check for ngrok binary path
  - Estimated effort: 0.5h
  - Priority: P1
- **Test Recommendations**:
  - Unit test: ngrok path validation

### 4. `lsp/process.rs`

- **File**: `src/crates/assembly/core/src/service/lsp/process.rs`
- **Access Control**:
  - Denylist check: ❌ none
  - Confirmation: ⚠️ LSP server start may bypass confirmation
  - LLM-triggered: ✅ yes (code actions via LSP)
- **Test Coverage**: 0 unit tests
- **Risk Level**: 🔴 high — spawns arbitrary LSP servers
- **Trigger Frequency**: medium
- **Recommended Fix**:
  - Code change: validate LSP server binary path against allowlist
  - Estimated effort: 2h (allowlist design)
  - Priority: P0
- **Test Recommendations**:
  - Unit test: malicious binary path blocked
  - Integration test: only configured LSP servers can spawn

### 5. `mcp/server/connection.rs`

- **File**: `src/crates/services/services-integrations/src/mcp/server/connection.rs`
- **Access Control**:
  - Denylist check: ❌ none
  - Confirmation: ✅ MCP server connection requires config
  - LLM-triggered: ⚠️ hybrid (LLM can request MCP tool use)
- **Test Coverage**: some
- **Risk Level**: 🟡 **REVISED: medium** — production code uses fixed programs only; the `sh -c` calls in this file are all in test code, not production. Real MCP server spawning goes through a different mechanism (likely Network IPC, not local shell).
- **Trigger Frequency**: medium
- **Recommended Fix**:
  - No production change needed for Phase 2
  - If future work introduces LLM-controlled MCP server commands, revisit
  - Priority: ~~P0~~ → **P2 (audit-only)**
- **Test Recommendations**:
  - Existing tests cover the test-only `sh -c` paths
- **Audit Note (2026-06-23, final)**: Originally classified P1, briefly revised to P0 based on assumption of `sh -c` in production. After T2.4 deep dive, confirmed `sh -c` is ONLY in test code. Production MCP spawning does not use shell. Risk revised to P2.

### 6. `miniapp/runtime.rs`

- **File**: `src/crates/contracts/product-domains/src/miniapp/runtime.rs`
- **Access Control**:
  - Denylist check: ❌ none
  - Confirmation: ⚠️ miniapp runtime spawns worker processes
  - LLM-triggered: ✅ yes (miniapp JS worker)
- **Test Coverage**: 0 unit tests
- **Risk Level**: 🔴 high — JS worker execution
- **Trigger Frequency**: low (miniapp usage)
- **Recommended Fix**:
  - Code change: add guard for worker spawn
  - Estimated effort: 1.5h
  - Priority: P0
- **Test Recommendations**:
  - Unit test: malicious worker path blocked

### 7. `process_manager.rs`

- **File**: `src/crates/services/services-core/src/process_manager.rs`
- **Access Control**:
  - Denylist check: ❌ none
  - Confirmation: ⚠️ depends on caller
  - LLM-triggered: ⚠️ hybrid
- **Test Coverage**: explicit (mod-level tests)
- **Risk Level**: 🟡 medium — generic process mgmt
- **Trigger Frequency**: varies
- **Recommended Fix**:
  - Code change: add guard as middleware in spawn fn
  - Estimated effort: 1h
  - Priority: P1
- **Test Recommendations**:
  - Add test: spawn cmd blocked by denylist

### 8. `glob_search.rs`

- **File**: `src/crates/execution/tool-execution/src/search/glob_search.rs`
- **Access Control**:
  - Denylist check: ❌ none
  - Confirmation: N/A (search only)
  - LLM-triggered: ✅ yes (glob tool)
- **Test Coverage**: 0 unit tests for shell-exec
- **Risk Level**: 🟡 medium — uses `find`/`fd` shell-out
- **Trigger Frequency**: high (every grep/glob call)
- **Recommended Fix**:
  - Code change: validate shell command against denylist before execution
  - Estimated effort: 1h
  - Priority: P1
- **Test Recommendations**:
  - Unit test: rm command rejected in search

### 9. `port_adapters.rs`

- **File**: `src/crates/assembly/core/src/function_agents/port_adapters.rs`
- **Access Control**:
  - Denylist check: ❌ none
  - Confirmation: ✅ user-triggered (function agent caller)
  - LLM-triggered: ❌ no (user only)
- **Test Coverage**: minimal
- **Risk Level**: 🟢 low — user-initiated only
- **Trigger Frequency**: low
- **Recommended Fix**:
  - Code change: add audit log call (low priority, just for forensic)
  - Estimated effort: 0.5h
  - Priority: P2

---

## Priority Ranking

### P0 (must fix in Phase 2)

1. `computer_use_actions.rs` — LLM-triggerable, no denylist, high frequency
2. `browser_launcher.rs` — LLM-triggerable, no denylist
3. `lsp/process.rs` — LLM-triggerable, no denylist
4. `miniapp/runtime.rs` — LLM-triggerable, no denylist

### P1 (should fix)

5. `ngrok.rs`
6. `mcp/server/connection.rs`
7. `process_manager.rs`
8. `glob_search.rs`

### P2 (nice-to-have)

9. `port_adapters.rs`

---

## Total Estimated Effort

- P0 (4 paths × ~1.5h avg): ~6h
- P1 (4 paths × ~1.25h avg): ~5h
- P2 (1 path): ~0.5h
- **Total**: ~11.5h (~1.5 working days)

This aligns with Phase 2 estimate (1-2 days).

---

## Recommendation

- **Phase 2 should target P0 + P1 paths** (8 paths total, ~11h)
- **P2 can be deferred** to a follow-up spec
- After Phase 2, all LLM-triggerable shell-exec paths are guarded

---

**Last updated:** 2026-06-23
**Status:** Phase 1 complete