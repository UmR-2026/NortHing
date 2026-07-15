<!-- LEGACY: 本文档是 v0.1.0 之前的历史计划，保留原 `agent-app` 名称作历史参考。
     Northing / 纳森 是 agent-app 的继任者（v0.1.0 之后改名）。
     本文件内容不被后续产品名替换脚本覆盖，保留 plan 当时的命名语境。 -->

# BitFun Remake Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **CONST-FLAG PATTERN:** every behavioral change ships behind a `const FLAG: bool = true;` gate + regression test + commit + PROJECT_STATE update, so it can be rolled back with one `git revert`.

**Goal:** Convert the remaining CODE_REVIEW findings (P1-2, P1-3, P2-1, P2-3, P3-1..5, S-1) from "known issues" into "shipped fixes" — without regressing the 821+ tests or the v3 prompt-token savings. This is **not** a rewrite; it is a sequence of bounded refactors against the working `v3-restructure` codebase.

**Architecture:** 5 phases, each independently shippable. No new crates, no new DBs, no dependency bumps. Each phase is one feature-flagged change set + tests + commit.

**Tech Stack:** Rust 2024 edition, workspace, const-flag rollout, TDD red→green.

**Spec / Parent doc:** `CODE_REVIEW.md` (sections 3.x and 7 — Top 10 action items)
**Already-fixed in commit `7a25b74`:** P1-1 (clarified, not removed), P2-2 (length assert), P2-4 (reserved comments), P2-5 (module docs).

**Working directory:** `E:\agent-project\BitFun-v3` (git worktree on `v3-restructure` branch)
**Toolchain:** `set PATH=C:\Users\UmR\.cargo\bin;C:\Users\UmR\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;%PATH%` *before every cargo command* — GNU toolchain ahead of MSVC in PATH breaks `getrandom`/`aws-lc-rs` with `dlltool.exe not found`.

---

## Phase Map (read this first)

| Phase | Topic | CODE_REVIEW #s | Effort | Risk | ROI |
|---|---|---|---|---|---|
| **R1** | Shell-exec sandbox + confirmation audit | S-1, P3-2 (partial) | 2d | 🔴 High (security) | Safety critical |
| **R2** | ChatView decomposition | P1-2, P1-3 | 2-3d | 🟡 Medium (UI behavior) | Maintainability |
| **R3** | SessionStoragePathResolution enum | P2-3 | 1-1.5d | 🟡 Medium (46 files) | Type safety |
| **R4** | Error + logging facade unification | P3-1, P3-2 | 1.5d | 🟢 Low | Observability |
| **R5** | Test-coverage backfill + dead-code sweep | P2-4 (verify), P3-3, P3-4, P3-5 | 2d | 🟢 Low | Quality bar |

**Sequencing rule:** R1 first (safety), then R2/R3 in parallel (different files, no merge conflicts), then R4 (cross-cutting), then R5 (last because R1-R4 generate new code to test).

**Out of scope (explicitly):** Perf-1/2/4 (paged turns, virtual scroll, dynamic concurrency) — these are tracked in `PROJECT_STATE.md` as separate performance work. P3-3 (Arc<Vec<Message>>) is a perf micro-optimization with unclear benefit; folded into R5 only if a benchmark shows it matters.

---

## File Structure (changes only)

| Phase | Files modified | Files created |
|---|---|---|
| R1 | `crates/assembly/core/src/agentic/tools/implementations/bash_tool.rs`, `.../exec_command/*.rs`, `crates/execution/tool-execution/src/pipeline.rs` | `.../tools/implementations/shell_safety.rs` (denylist + workspace guard) |
| R2 | `apps/cli/src/ui/chat/state.rs`, `apps/cli/src/ui/chat/mouse.rs`, every `state.rs` consumer | `apps/cli/src/ui/chat/state/{popups,selection,mouse,core}.rs` (split out) |
| R3 | 46 files (see audit) touching `SessionStoragePathResolution` | — |
| R4 | every crate's `lib.rs` (tracing init), `crates/contracts/runtime-ports/src/lib.rs` (error map) | `crates/contracts/runtime-ports/src/error_facade.rs` |
| R5 | test files under `crates/*/tests/`, `src/*/tests` modules | new test files for `partition_tool_batches` edge cases, `PopupStack`, retry logic |

**No new crates. No new tools. No DBs. No dependency additions.**

---

## Phase R1: Shell-exec sandbox + confirmation audit (S-1)

**Goal:** Prove that every shell-executing tool (bash, exec_command, terminal_control) routes through user confirmation for non-trivial commands and rejects a denylist of catastrophic commands before `spawn`.

**Why first:** Security finding. Every other phase is "nice to have"; this one is "ship-blocker for any production release".

### Task R1.0: Map the surface

- [ ] **Step 1:** Enumerate every `spawn`/`Command::new`/`create_tokio_command` site that can execute *user-or-LLM-supplied* input. Use:
  ```bash
  cd E:/agent-project/BitFun-v3/src && grep -rn "create_tokio_command\|create_command\|Command::new" --include="*.rs" crates/assembly/core/src/agentic/tools/
  ```
  Write the list into `docs/sdlc-harness/shell-exec-surface.md`.
- [ ] **Step 2:** For each site, record: (a) tool name, (b) does it accept raw shell strings or argv arrays?, (c) is there an `AwaitingConfirmation` gate in the pipeline?, (d) is the working directory clamped to workspace?

### Task R1.1: TDD red — denylist rejects known-catastrophic commands

- [ ] **Step 1:** Create `E:\agent-project\BitFun-v3\src\crates\assembly\core\src\agentic\tools\implementations\shell_safety.rs` with:
  ```rust
  //! Shell-command safety filter (S-1 hardening).
  pub const SHELL_DENYLIST_PATTERNS: &[&str] = &[
      r"(^|[\s;&|`])rm\s+-rf?\s+/(--no-preserve-root)?",
      r"\bmkfs\b",
      r"\bdd\b.*\bof=/dev/",
      r">\s*/dev/sd[a-z]",
      r"\bshutdown\b",
      r"\breboot\b",
      r":\(\)\s*\{\s*:\|:&\s*\}\s*;:", // fork bomb
      // curl|sh, wget|sh, etc.
      r"(curl|wget)\b[^|]*\|\s*(sh|bash|zsh)\b",
  ];
  pub fn is_command_allowed(command: &str) -> bool { /* regex match each pattern */ }
  ```
- [ ] **Step 2:** Write `mod tests` with 12 cases (each pattern + 3 benign commands that must pass). Run — fails (module doesn't exist yet).
- [ ] **Step 3:** Implement. `cargo test -p bitfun-assembly-core shell_safety`. Green.

### Task R1.2: Wire denylist into bash_tool + exec_command

- [ ] **Step 1:** In `bash_tool.rs` and `exec_command/command.rs`, *before* the existing confirmation logic, add:
  ```rust
  const ENABLE_SHELL_DENYLIST: bool = true; // R1
  if ENABLE_SHELL_DENYLIST && !shell_safety::is_command_allowed(&command) {
      return Err(ToolError::Denied(format!(
          "Command matched shell denylist (R1 safety filter). Refusing to execute: {command}"
      )));
  }
  ```
- [ ] **Step 2:** Add a regression test that runs the bash tool with `rm -rf /` and asserts a `Denied` error.
- [ ] **Step 3:** `cargo test -p bitfun-assembly-core bash_tool`. Green.

### Task R1.3: Workspace-clamp audit (best-effort)

- [ ] **Step 1:** Audit each spawn site for `current_dir(...)`. Document which tools pass an absolute workspace path vs. inherit CWD.
- [ ] **Step 2:** For tools that inherit CWD, add a `const CLAMP_WORKSPACE: bool = false;` flag + TODO rather than silently changing behavior. Changing CWD is risky; surface it in the audit doc and defer the flip to after manual QA.

### Task R1.4: Commit + update

- [ ] Commit: `feat(tools): R1 shell-exec denylist + workspace-clamp audit (S-1)`
- [ ] Update `CODE_REVIEW.md`: mark S-1 as **FIXED** with commit SHA + the `ENABLE_SHELL_DENYLIST` rollback flag noted.

---

## Phase R2: ChatView decomposition (P1-2, P1-3)

**Goal:** Split the 36-field `ChatView` God Object into 4 cohesive sub-structs *without* changing any external behavior. The const-flag pattern here is: keep `pub` field access working via `Deref`/accessor methods, gate the new layout behind `const USE_SPLIT_CHATVIEW: bool`.

**Current state:** `apps/cli/src/ui/chat/state.rs` is 314 lines, `ChatView` has ~36 fields spanning 5 concerns (core view, popup manager, selection, mouse, render cache).

### Task R2.0: TDD red — snapshot tests

- [ ] **Step 1:** Add `apps/cli/src/ui/chat/state_split_tests.rs` with:
  ```rust
  #[test] fn chatview_state_split_preserves_all_fields() {
      // Construct a ChatView, set every field to a sentinel, then assert
      // that after decomposition the sentinels round-trip via the new
      // accessor methods.
  }
  ```
  Run — fails (file doesn't exist).
- [ ] **Step 2:** Make it compile by creating the file with a trivial `assert!(true)` placeholder, then expand to the real round-trip test once R2.1 lands.

### Task R2.1: Extract `PopupManager`

- [ ] **Step 1:** Move these fields from `ChatView` into a new `PopupManager` struct in `apps/cli/src/ui/chat/state/popups.rs`:
  - `model_selector`, `agent_selector`, `session_selector`, `skill_selector`, `subagent_selector`, `mcp_selector`, `mcp_add_dialog`, `provider_selector`, `model_config_form`, `theme_selector`, `popup_stack`
- [ ] **Step 2:** In `ChatView`, replace the fields with `popups: PopupManager` and add accessor methods (`pub fn model_selector(&mut self) -> &mut ModelSelectorState` etc.) so existing call sites compile unchanged.
- [ ] **Step 3:** `cargo build -p bitfun-cli`. Fix any access-site errors by routing through accessors. Do **not** change behavior.
- [ ] **Step 4:** `cargo test -p bitfun-cli`. All existing tests must pass.

### Task R2.2: Extract `SelectionState` + `MouseState`

- [ ] **Step 1:** Move to `state/selection.rs`: `collapsed_tools`, `focused_block_tool`, `collapsed_thinking`.
- [ ] **Step 2:** Move to `state/mouse.rs`: `pending_command`, `pending_theme_preview`, `pending_skill_action`, `pending_subagent_action`, `pending_mcp_toggle`, `selection_anchor`, `selection_focus`, `selection_mouse_down`, `selection_dragged`.
- [ ] **Step 3:** Same accessor-migration pattern as R2.1.
- [ ] **Step 4:** Run `cargo test -p bitfun-cli`. Green.

### Task R2.3: Refactor mouse dispatch (P1-3)

- [ ] **Step 1:** In `mouse.rs`, replace the 8-branch `if self.X.captures_mouse(mouse)` chain with a `PopupManager::active_popup_mut()` that returns `Option<&mut dyn PopupMouseHandler>` based on `popup_stack.peek()`.
- [ ] **Step 2:** Add a trait `PopupMouseHandler` with `captures_mouse` + `handle_mouse_event`. Implement for each popup state.
- [ ] **Step 3:** `handle_mouse_event` becomes:
  ```rust
  if let Some(active) = self.popups.active_popup_mut() {
      active.handle_mouse_event(mouse);
      return true;
  }
  false
  ```
- [ ] **Step 4:** `const USE_POPUP_DISPATCH_TRAIT: bool = true;` — if true use the new dispatch, else fall through to the old chain (kept during QA).
- [ ] **Step 5:** Run the chat module's existing tests + a manual smoke test in the TUI.

### Task R2.4: Commit + update

- [ ] Commit (one per extracted struct, so reverts are granular):
  - `refactor(cli): R2.1 extract PopupManager from ChatView (P1-2)`
  - `refactor(cli): R2.2 extract SelectionState + MouseState (P1-2)`
  - `refactor(cli): R2.3 popup dispatch trait (P1-3)`
- [ ] Update `CODE_REVIEW.md`: mark P1-2, P1-3 **FIXED**.

---

## Phase R3: SessionStoragePathResolution enum (P2-3)

**Goal:** Replace the `Option<String>` remote fields with a type-safe enum so Local vs Remote cannot be misconstructed.

**Scope:** 46 files reference `remote_connection_id`/`remote_ssh_host`. This is the largest phase — **do not start until R2 is on a separate branch or merged**, because R3 touches many files and any concurrent edit to the same file will conflict.

### Task R3.0: Enum design + migration shim

- [ ] **Step 1:** In `crates/contracts/runtime-ports/src/lib.rs`, add the new enum alongside the existing struct (do not delete yet):
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum SessionStoragePathResolution {
      Local { requested: PathBuf, effective: PathBuf },
      Remote {
          requested: PathBuf,
          effective: PathBuf,
          connection_id: String,
          ssh_host: String,
      },
      UnresolvedRemote {
          requested: PathBuf,
          effective: PathBuf,
          ssh_host: String,
      },
  }
  ```
- [ ] **Step 2:** Add `From<OldStruct> for SessionStoragePathResolution` and `From<SessionStoragePathResolution> for OldStruct` shims so callers can migrate one file at a time.
- [ ] **Step 3:** `const USE_ENUM_STORAGE_RESOLUTION: bool = false;` — flag stays false until all 46 files are migrated.

### Task R3.1: Migrate file-by-file (mechanical)

- [ ] **Step 1:** List the 46 files from the audit (search `remote_connection_id|remote_ssh_host`).
- [ ] **Step 2:** For each file: change construction sites to use the enum, change destructure sites to `match`. Keep the From-shims as a fallback.
- [ ] **Step 3:** After every ~10 files, `cargo check --workspace` to catch regressions early.
- [ ] **Step 4:** When all 46 are migrated, flip `USE_ENUM_STORAGE_RESOLUTION = true`, delete the old struct + shims, remove the flag.

### Task R3.2: Commit + update

- [ ] Commit per batch of ~10 files: `refactor(contracts): R3.1 migrate SessionStoragePathResolution batch N/5 (P2-3)`
- [ ] Final commit: `refactor(contracts): R3 complete — drop legacy SessionStoragePathResolution struct (P2-3)`
- [ ] Update `CODE_REVIEW.md`: mark P2-3 **FIXED**.

---

## Phase R4: Error + logging facade unification (P3-1, P3-2)

**Goal:** Pick `tracing` as the single logging facade and `anyhow` for internal + `PortError` at boundaries. Eliminate the `log::*` / `tracing::*` mix.

### Task R4.0: Audit current usage

- [ ] **Step 1:** `grep -rn "use log::" --include="*.rs" src/ | wc -l` and same for `tracing::`. Record counts in `docs/development/logging-audit.md`.

### Task R4.1: Mechanical `log::*` → `tracing::*` swap

- [ ] **Step 1:** For each file using `log::*`, swap to `tracing::*`. Macros are 1:1 compatible (`info!`, `warn!`, `error!`, `debug!`).
- [ ] **Step 2:** `cargo check --workspace` after every crate.
- [ ] **Step 3:** Drop the `log` dep from `Cargo.toml` files where it's no longer used.
- [ ] **Step 4:** `const USE_TRACING_FACADE: bool = true;` at the workspace logger init site (likely in `apps/cli/src/main.rs` + `apps/desktop/src/lib.rs`).

### Task R4.2: Error-boundary policy doc

- [ ] **Step 1:** Write `docs/development/error-handling-policy.md` codifying: PortError at crate boundaries, anyhow internally, explicit `From` impls at FFI.
- [ ] **Step 2:** Audit `adapters/ai-adapters` (uses anyhow) and add `From<anyhow::Error> for PortError` where it crosses into contracts.

### Task R4.3: Commit + update

- [ ] Commit: `refactor(workspace): R4 unify logging on tracing facade (P3-1)`
- [ ] Commit: `docs(contracts): R4 error-handling policy (P3-2)`
- [ ] Update `CODE_REVIEW.md`: mark P3-1, P3-2 **FIXED**.

---

## Phase R5: Test-coverage backfill + dead-code sweep (P3-3, P3-4, P3-5)

**Goal:** Raise core-crate coverage from "unknown" to >60% on the algorithms the CODE_REVIEW flagged, and resolve every `#[allow(dead_code)]`.

### Task R5.1: Algorithm unit tests

- [ ] **Step 1:** Add edge-case tests for `partition_tool_batches`: empty input, single safe, single unsafe, all-safe, all-unsafe, mismatched lengths (must `debug_assert` panic).
- [ ] **Step 2:** Add `PopupStack` tests: push dedup, pop, peek, clear, nested push/pop ordering, `previous()`.
- [ ] **Step 3:** Add `ProcessManager` cross-platform tests under `#[cfg(...)]`: Windows Job assign-fail recovery, Unix process-group termination ordering.
- [ ] **Step 4:** Add AI client retry-logic tests: 10 retries cap, exponential backoff timing, Retryable vs Terminal classification.

### Task R5.2: Resolve dead-code

- [ ] **Step 1:** `grep -rn "#\[allow(dead_code)\]" --include="*.rs" src/` → for each hit, either: (a) use it in a test (R5.1 may consume some), (b) delete it, or (c) add a `// Reserved: <reason>` comment like R2.1 did.
- [ ] **Step 2:** The 3 `PopupStack` methods (`is_empty`, `remove`, `previous`) are already reserved per commit `7a25b74` — verify they're either used in R2.3's dispatch trait or stay reserved.

### Task R5.3: Config convergence (P3-5)

- [ ] **Step 1:** Document the existing config layering (CLI YAML, AIConfig env, runtime Options) in `docs/development/config-layering.md`. Don't merge them — just document so future work has a map.

### Task R5.4: Commit + update

- [ ] Commit: `test(execution): R5.1 partition_tool_batches edge cases (P3-4)`
- [ ] Commit: `test(cli): R5.1 PopupStack + ProcessManager coverage (P3-4)`
- [ ] Commit: `chore: R5.2 dead-code resolution sweep`
- [ ] Commit: `docs: R5.3 config-layering map (P3-5)`
- [ ] Update `CODE_REVIEW.md`: mark P3-3, P3-4, P3-5 **FIXED**.

---

## Verification Protocol (every phase)

After **every** task, before commit:

1. **Build:** `cargo check --workspace` (or at least the touched crate + dependents).
2. **Test:** `cargo test -p <crate>` for touched crates. Full workspace test before the phase-final commit.
3. **No new warnings:** `cargo build --workspace 2>&1 | grep -c warning` must not increase.
4. **Flag check:** if a `const FLAG: bool = true;` was added, grep to confirm it exists and the if-branch is taken.
5. **Doc update:** `CODE_REVIEW.md` status + `PROJECT_STATE.md` line added.
6. **Commit:** granular, one logical change per commit, with the rollback flag noted in the message.

## Rollback

Every behavioral change in this plan ships behind a `const FLAG: bool = true;`. To roll back any phase:
```bash
# Find the flag
git grep "const USE_.*: bool" -- src/
# Flip to false, commit, done. Or:
git revert <phase-commit-sha>
```

## Definition of Done (whole plan)

- [ ] All CODE_REVIEW items marked **FIXED** or **WONTFIX (with rationale)**.
- [ ] `cargo test --workspace` green (≥ 821 tests, no regressions).
- [ ] `cargo build --workspace` zero new warnings.
- [ ] `CODE_REVIEW.md` Top-10 table updated with commit SHAs.
- [ ] `PROJECT_STATE.md` reflects R1-R5 completion.
- [ ] No `log::*` calls remain outside the tracing shim (R4).

---

## Pitfall Log (carried forward from v3 work)

- **PATH order:** GNU toolchain before MSVC breaks `getrandom`/`aws-lc-rs`. Always prepend `C:\Users\UmR\.cargo\bin` and `C:\Users\UmR\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin`.
- **`nul` file:** Windows reserves the name `nul`. A stray `nul` file in the worktree breaks `git add -A`. If `git add -A` fails with `invalid path 'nul'`, `rm -f nul` then add files explicitly.
- **Subagent tool access:** Explore subagents have Read/Bash only — they cannot Edit/Write. For implementation tasks, execute edits from the main session.
- **AskUserQuestion schema:** keep options to 2-4, no "Other" (auto-provided), no multiSelect unless truly needed.
- **Test assertion drift:** when a behavior-change commit touches a test, update the assertion in the *same* commit — don't leave the tree red between commits.
- **`PromptBuilderContext` move errors (E0382):** after editing prompt assembly, watch for borrow-of-moved-value. Clone the context before the consuming call.

---

*Plan author: ZCode session, 2026-06-18. Based on `CODE_REVIEW.md` v3 audit + commit `7a25b74` (P1-1/P2-2/P2-4/P2-5 already shipped).*
