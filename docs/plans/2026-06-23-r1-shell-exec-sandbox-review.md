# R1 Shell-exec Sandbox — Review Guide (Human Reviewer)

> **Audience:** Human reviewer (YOU)
> **Spec:** `docs/superpowers/specs/2026-06-23-r1-shell-exec-sandbox-design.md`
> **Plan:** `docs/plans/2026-06-23-r1-shell-exec-sandbox-impl.md`

---

## Phase 1 — Audit Pass

### Per-path review checklist (each entry in audit doc)

For each shell-exec path documented:

- [ ] **Access Control** correctly identifies:
  - Denylist check presence/absence
  - Confirmation gate presence/absence
  - LLM-triggered vs user-triggered
- [ ] **Test Coverage** has actual test count (not estimated)
- [ ] **Risk Level** (🔴/🟡/🟢) matches the actual findings (high = no denylist + LLM-triggered + frequent)
- [ ] **Recommended Fix** has specific code reference (file:line)

### Audit summary review

- [ ] All 9+ paths are documented (computer_use_actions, browser_launcher, ngrok, lsp/process, mcp/server/connection, miniapp/runtime, process_manager, glob_search, port_adapters)
- [ ] P0/P1/P2 priorities are reasonable
- [ ] Total estimated effort matches plan (~3-5 days)

---

## Phase 2 — Guard Function + Denylist Extension

### T2.1: Guard function skeleton

- [ ] `GuardOutcome` enum has 4 variants (Allowed, DeniedByDenylist, DeniedByConfirmation, AwaitingConfirmation)
- [ ] Function signature is `async fn guard_command_execution(cmd: &str, tool_name: &str, context: &ToolUseContext) -> Result<GuardOutcome, NortHingError>`
- [ ] Stub returns Ok(Allowed) - no behavior change yet

### T2.2: Denylist branch

- [ ] Calls `check_command_denied` (sync, fast)
- [ ] Logs audit event (Phase 3 dependency, but stub for now)
- [ ] Returns `DeniedByDenylist { pattern }` on match
- [ ] Unit tests cover: rm -rf /, mkfs, dd, fork bomb, etc.

### T2.3: Confirmation branch

- [ ] Reads `context.skip_tool_confirmation()` 
- [ ] If skip, logs and returns Allowed
- [ ] If not skip, awaits `request_user_confirmation()`
- [ ] Handles all 4 outcomes: Confirmed, Rejected, Timeout, ChannelClosed
- [ ] Unit tests for each outcome

### T2.4: 8 paths wiring

For each path, verify:
- [ ] `guard_command_execution()` called before `Command::new()`
- [ ] Tool name passed correctly
- [ ] Existing behavior preserved when guard returns Allowed
- [ ] Error handling correct (returns NortHingError on Denied)

### T2.5: Phase 2 verification

- [ ] `cargo test --workspace --lib` returns 0 failed
- [ ] BashTool denylist behavior unchanged (verify with `shell_safety::is_command_allowed` tests)
- [ ] No new clippy warnings

---

## Phase 3 — Mode-based Confirmation + Audit Log

### T3.1: ConfirmationMode + ShellSecurityConfig

- [ ] `ConfirmationMode::{ Permissive, Strict }` enum defined
- [ ] `ShellSecurityConfig { confirmation_mode, mode_overrides }` struct defined
- [ ] Default value: `Permissive` (backward compat)
- [ ] 8 coding modes mapped to Permissive
- [ ] Tests for default + override

### T3.2: round_executor wiring

- [ ] Reads `ShellSecurityConfig.mode_overrides[mode]` first
- [ ] Falls back to `confirmation_mode`
- [ ] Falls back to legacy `skip_tool_confirmation` for backward compat
- [ ] Tests for: override hit, override miss, legacy field

### T3.3: Audit log writer

- [ ] New module `service/audit_log.rs`
- [ ] NDJSON format (one JSON object per line)
- [ ] Required fields: timestamp, mode, tool_name, command, decision, reason, session_id
- [ ] File rotation: 10MB or 7 days
- [ ] Wired into `guard_command_execution()` at all 5 call sites

### T3.4: Final verification

- [ ] `cargo test --workspace --lib` returns 0 failed
- [ ] `.northhing/audit.log` created on first shell-exec test
- [ ] HANDOVER + PROJECT_STATE updated

---

## Pre-Existing Issues (NOT blocks)

These are pre-existing and should NOT block the review:

- 37 pre-existing test build errors in `coordinator.rs` K.2.2 boundary tests
- `northhing-webdriver` DLL link error (STATUS_ENTRYPOINT_NOT_FOUND)
- Pre-existing clippy errors in unrelated packages

---

## Decision Tree

| If T2.5 or T3.4 reports... | Then... |
|---------------------------|---------|
| All tests pass | ✅ APPROVE — R1 complete |
| Pre-existing errors only | ⚠️ CONDITIONAL APPROVE — note + accept |
| New test failure | ❌ REJECT — find regression, fix, re-verify |

---

**Last updated:** 2026-06-23
**Reviewer:** Human reviewer (you)