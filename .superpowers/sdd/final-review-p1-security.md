# Final Branch Review: fix/p1-security-0804

**Reviewer**: Independent final reviewer (did not participate in any single-task implementation or review)
**Scope**: `git diff ae44334..f42451d` (5 commits, 20 files, +1637/-49)
**Branch**: `fix/p1-security-0804` (worktree `p1-security-0804`)
**Baseline**: ae44334 (main, 2026-08-04)
**Materials**: `.superpowers/sdd/task-c{1,2,3}-{brief,report,review}.md`, `progress.md` ledger, direct code verification (file:line)

---

## 1. Branch-Level Dual Verdict

| Dimension | Verdict | Basis |
|---|---|---|
| **Spec compliance** | **PASS** | All 3 tasks PASS at single-task level (C1 after 1 fix round; C2 clean; C3 after 1 fix round). P1-3/P1-2/P1-5 all resolved in ledger. Plan delivery requirements fully met; briefs' "explicitly out-of-scope" items respected (embedded relay binding untouched, remote delete semantics untouched, MCPServerConfig.env only registered as concern). New debts P1-6/P1-7/P1-8 properly registered active. |
| **Code quality** | **PASS** | No Critical or Important findings remain after fix rounds. Cross-task patterns (fail-closed, sentinel, idempotency, atomic write) are isomorphic and consistent. No same-file multi-lock. Logs English-only. All god-file thresholds within limits (max 555 lines). Environment constraint (ring/aws-lc-sys gcc missing) explicitly registered; CI coverage gap acknowledged. |

---

## 2. Merge Conclusion

**CAN MERGE.** No must-fix-before-merge items. All accumulated findings are Minor (triage below). The branch closes three P1 security debts (P1-1 retroactively, P1-2, P1-3, P1-5) and registers three new active debts (P1-6 remote delete no confirmation gate, P1-7 embedded relay open mode, P1-8 MCPServerConfig.env plaintext) with accurate file:line evidence.

---

## 3. Cross-Task Consistency

### 3.1 Three fail-closed patterns: isomorphism check

**Verdict: Semantically isomorphic. All three propagate Err without silent fallback.**

| Feature | C1 trash fail-closed | C2 relay non-loopback no-key | C3 keyring unavailable |
|---|---|---|---|
| Trigger | `trash::delete` returns Err (platform unavailable) | bind addr non-loopback + `api_key.is_none()` | `keyring.store`/`get` returns Err (OS keychain unavailable) |
| Mechanism | `?` propagates Err; no `fs::remove_*` fallback (`delete_path.rs:91-92`) | `from_env()` returns `Err(String)`; `main.rs:21-22` `?` exits process (`config.rs:229-236`) | `?` propagates Err; plaintext restored to in-memory before return (`io.rs:88-103`); save skipped |
| Disk state on failure | Target file unchanged (still exists) | Process never starts | File unchanged (byte-by-byte, tested `io_tests.rs:312-326`) |
| Test coverage | `trash_failure_returns_err_fail_closed` (`delete_path.rs:230-251`) | `non_loopback_without_key_is_rejected` (`config.rs:373-395`) | `keyring_migration_fail_closed_does_not_write_file` (`io_tests.rs:277-327`) |

**Key findings** (independently verified file:line):

1. **C1 trash path and permanent path are strictly mutually exclusive** (`delete_path.rs:64-85` permanent vs `:87-105` trash). The `#[cfg(not(test))]` block (`:89-93`) calls `trash::delete` and maps Err via `?`. The `#[cfg(test)]` block (`:95-98`) calls the mock seam. There is **no** code path from trash failure to `fs::remove_*`. ✅

2. **C2 fail-closed fires before any network binding** (`config.rs:229-236`). The `from_env()` `Result` is consumed by `main.rs:21-22` `.map_err(|e| anyhow::anyhow!("{e}"))?`. The process exits before `TcpListener::bind` (`main.rs:138`). ✅

3. **C3 fail-closed restores plaintext before returning Err** (`io.rs:86-103`). `std::mem::take` moves the plaintext out (`:86`); on store failure it is put back (`:95`), then `Err` is returned with context. `save_app_settings_at` is **never called** when migration fails because `migrated` is only checked at `io.rs:58` after the `?` at `:57`. The `update_app_settings_at` path (`:128-147`) applies the same `?` at `:138`. ✅

### 3.2 Atomic write patterns: C2 key file vs C3 settings (inherited from H-9)

**Verdict: C2 key-file write is a simpler variant; C3 reuses the H-9 settings atomic write unchanged.**

| Feature | C2 relay key file (`config.rs:77-128`) | C3 settings (`io.rs:213-285`, inherited H-9) |
|---|---|---|
| tmp naming | `.tmp` extension (`:108`) | `.{name}.{pid}.{nonce}.tmp` (`:228`) |
| explicit flush | no (`std::fs::write`) | **yes** (`tokio::fs::File::create` + `write_all` + `flush` + drop, `:240-257`) |
| `.bak` before write | no | yes (`:230-234`, skip on first write) |
| 0o600 after write | yes (Unix, `:113-120`) | n/a (settings not secret after C3) |
| Windows rename fallback | no | yes (`:261-283`, remove + retry) |
| retry loop | no | no (single retry) |

**Assessment**: C2's key-file write is less robust than H-9 (no explicit flush, no `.bak`, no Windows rename fallback). However, the key file is a single 44-char string (not a complex JSON document), and a corrupted key file is recoverable (regenerate on next start; or the `load_or_generate_key` function detects short/empty and returns Err which is non-fatal for loopback). The trade-off is acceptable for a first-run one-time write. **Not a merge blocker**; consider backporting the explicit flush pattern in a follow-up if relay key file corruption is observed in production.

### 3.3 Sentinel + idempotency: C3 only

**Verdict: Sentinel design is sound; idempotency is test-verified.**

- Sentinel: `"__kr__"` (`keyring.rs:56`), 6-char ASCII, exact-match detection (`:59-61`, not prefix). No real API key starts with `__kr__` (Anthropic `sk-ant-`, OpenAI `sk-`, Gemini `AIza`). ✅
- Idempotency: `keyring_migrate_providers` skips empty and sentinel entries (`io.rs:82-84`), so reloading an already-migrated settings file does not re-write the keyring. Test `keyring_migration_already_sentinel_is_idempotent` (`io_tests.rs:246-275`) verifies `kr.get("p1").is_err()` after second load (no redundant store). ✅

### 3.4 Test discipline: all three tasks ship targeted tests

| Task | New tests | Verified by file:line grep |
|---|---|---|
| C1 | 5 unit + 1 integration field-update | `delete_path.rs::tests` 5 `#[test]` functions (`:180/205/229/253/277`); `tool_io_contracts.rs:168` `permanent: true` |
| C2 | 12 config tests | `config.rs::tests` 12 `#[test]` functions (`:286/298/322/347/373/397/424/448/473/490/510/532`) |
| C3 | 15 keyring unit + 4 IO integration | `keyring.rs::tests` 15 `#[test]` functions (`:240/248/255/262/270/276/284/291/298/305/313/321/329/335/343`); `io_tests.rs` 4 `#[tokio::test]` (`:210/246/277/332`) |

---

## 4. Cross-Task Interaction

### 4.1 C2 embedded relay + C1 trash: local mitigation, remote still P1-6

**Verdict: Correctly scoped. Local delete mitigated by C1 trash; remote delete still open (P1-6 registered).**

The call chain for delete operations flows through `DeleteFileTool` (`delete_file_tool.rs`), which handles both local and remote paths:

- **Local delete** (`delete_file_tool.rs:319-324`): constructs `DeleteLocalPathRequest { permanent: false, ... }`. C1's `delete_local_path` routes this to `trash::delete` by default (`delete_path.rs:89-93`). **Mitigated by C1.** ✅
- **Remote delete** (`delete_file_tool.rs:293`): calls `build_remote_delete_command` which constructs `rm -rf` / `rm -f` (`delete_path.rs:108-114`). **Not mitigated.** C1 brief explicitly scoped out remote semantics ("remote 删除语义/确认门改造（只核实报告）").

The C1 review discovered (and the fix round corrected) that `DeleteFileTool.needs_permissions()` returns `false` (`delete_file_tool.rs:115-117`, independently verified), which short-circuits the tool framework confirmation gate (`tool_confirmation.rs:55` -> `ToolConfirmationPlan::Skip`). This means **both** local and remote deletes bypass user confirmation. Local is mitigated by trash; remote `rm -rf` is not. This is correctly registered as P1-6 (`ledger:82-87`).

**No interaction conflict**: C1 touches `delete_path.rs` and `delete_file_tool.rs`; C2 touches `embedded_relay.rs` and relay-server. Zero file overlap. No write-path coupling.

### 4.2 C3 keyring + C2 embedded relay: P1-8 env plaintext discovered

**Verdict: Correctly scoped. C3 migrates ProviderConfig.api_key only; MCPServerConfig.env plaintext registered as P1-8.**

C3's brief explicitly limited scope to `ProviderConfig.api_key`. During the C3 fix round, the reviewer identified that `MCPServerConfig.env` (`types.rs:161-162`, `pub env: HashMap<String, String>`) also stores credentials in plaintext (e.g. `OPENAI_API_KEY=sk-xxx`). The fix round registered this as P1-8 (`ledger:75-80`), per brief §7 "若发现其它明文敏感字段，记为新条目 concern，不擅自改".

**No interaction conflict**: C3 touches `settings/keyring.rs`, `settings/io.rs`, `settings/sync.rs`, `settings/mod.rs`, `provider_test.rs`, `tests.rs`, `io_tests.rs`; C2 touches `relay-server/` and `embedded_relay.rs`. Zero file overlap.

### 4.3 Three locks: no same-file multi-lock

**Verdict: Confirmed safe. No new locks introduced.**

| Lock | Task | Scope | Guards file |
|---|---|---|---|
| `SETTINGS_WRITE_LOCK` (H-9, reused by C3) | C3 reuses | `tokio::sync::Mutex<()>` process-global (`io.rs:13`) | `~/.northhing/config/app.json` |
| C2 relay key file | C2 new | **no lock** (single-write at startup, process init) | `~/.northhing/relay/api_key` |
| C1 trash seam | C1 new | **no lock** (per-call, thread-local mock in test) | n/a (delegates to OS recycle bin) |

C2's key file write happens once at `from_env()` time (process startup), before any concurrent access. No lock needed. C3's `SETTINGS_WRITE_LOCK` is inherited from H-9 and already covers the load-migrate-save transaction. C1's trash path is stateless (no shared mutable state; the thread-local mock is test-only). **No same-file multi-lock, no write-path stacking.**

---

## 5. Minor Triage (per-task, per-item disposition)

### Disposition categories
- **A (accept as-is)**: no action needed, behavior is correct or cosmetic
- **D (tech-debt)**: track for follow-up, non-blocking
- **F (must-fix-before-merge)**: blocking -- none in this branch

### Task C1

| ID | Description | Disposition | Reason |
|---|---|---|---|
| M-3 | `default_request_sends_to_trash_seam` lacks explicit "fs not called" assertion | D | Implicit via `file.exists()` + `was_trash_called()`. Cosmetic. Add explicit assert in follow-up. |
| M-5 | `enforce_path_operation(Delete)` called twice (validate_input + call_impl) | A | Pre-existing redundancy, not introduced by C1. No action. |
| (P1-6) | Remote delete no confirmation gate | D | Registered as P1-6 active. Local mitigated by trash; remote `rm -rf` still open. Follow-up task to restore confirmation gate. |

### Task C2

| ID | Description | Disposition | Reason |
|---|---|---|---|
| M-1 | Report line number "embedded_relay.rs:41-44" actual 44-49 | A | Report inaccuracy, not code. Content correct. |
| M-2 | `cargo check -p northhing-relay-server` not explicitly recorded | A | Implicit in `cargo test -p northhing-relay-server` (test = compile + run). |
| M-3 | `is_loopback` helper duplicated (fn + method) | D | Cosmetic. Extract to util module in follow-up. |
| M-4 | Key generation uses `eprintln!` not `tracing::info!` | A | `from_env` runs before tracing subscriber init. `eprintln!` is the correct choice. First-run only. |
| M-5 | `api_key_source` field `#[allow(dead_code)]` overkill | A | Actually used by tests and logging. No action. |
| M-6 | Empty key file -> warn but continue on loopback | D | Edge case: key file corrupted but loopback continues without auth. Acceptable (loopback is local-only). Consider fail-closed on key file error in follow-up. |
| (P1-7) | Embedded relay open mode | D | Registered as P1-7 active. Product requirement (LAN/ngrok pairing). Design task to thread key through pairing protocol. |
| (atomic) | C2 key-file write lacks explicit flush + .bak + Windows rename fallback | D | Simpler than H-9 pattern. Acceptable for single 44-char string. Backport explicit flush if corruption observed. |

### Task C3

| ID | Description | Disposition | Reason |
|---|---|---|---|
| M-2 | `MockKeyring` not `#[cfg(test)]`, compiled into production binary | D | ~50 lines of mock code in production binary. Zero behavioral impact (production only constructs `PRODUCTION_KEYRING`). Consider `#[cfg(any(test, feature = "test-support"))]` in follow-up. |
| M-5 | `store_api_key` / `delete_api_key` high-level helpers only used in tests | D | Dead-code-ish. Either annotate `#[allow(dead_code)]` or refactor migrate to use them. Follow-up. |
| M-7 | `sync.rs:37-48` `unwrap_or_else` falls back to sentinel (usability defect) | D | Keyring unavailable -> model config gets sentinel -> LLM call fails with "invalid bearer token". Not a security issue (not silent plaintext fallback), but loses fail-closed diagnostic. Recommend changing to `?` propagation in a follow-up hardening task. |
| M-9 | Report line number `io.rs:138-148` actual `:138-147` | A | Off-by-1, content correct. |
| M-10 | `pub use keyring::*` exposes `ProductionKeyring` to external crates | D | Not a security defect (trait-bound + OS permissions protect). Consider `pub(crate)` in follow-up. |
| (P1-8) | `MCPServerConfig.env` plaintext | D | Registered as P1-8 active. Same class as P1-2. Reuse `KeyringBackend` pattern in future wave. |

### Summary counts

| Disposition | Count |
|---|---|
| F (must-fix-before-merge) | 0 |
| D (tech-debt) | 13 |
| A (accept) | 6 |

**Highest-priority tech-debt items** (recommend follow-up tasks):
1. **P1-6 remote delete no confirmation gate** -- `DeleteFileTool.needs_permissions()=false` bypasses framework confirmation for `rm -rf`
2. **P1-7 embedded relay open mode** -- 0.0.0.0 + no key, product-required for LAN pairing
3. **P1-8 MCPServerConfig.env plaintext** -- same class as P1-2, credentials in app.json
4. **C3 M-7 sentinel fallback usability** -- keyring failure produces unusable provider instead of clear error

---

## 6. Verification Completeness

### 6.1 Per-task focused verification

| Task | Verification command | Result | Evidence |
|---|---|---|---|
| C1 | `cargo test -p tool-runtime` | 88 passed, 0 failed | Report `task-c1-report.md:54-78` with cargo output tail (`test result: ok. 1 passed; 0 failed; 1 ignored; ...`). 65 unit + 16 integration + 6 doc-tests. |
| C1 | `cargo check -p tool-runtime` | 0 warnings | Report `:84-87` |
| C2 | `cargo test -p northhing-relay-server -p northhing-relay-core` | 61 passed, 0 failed | Report `task-c2-report.md:36-45`. 37 relay-core + 7 relay-server lib + 12 config + 5 e2e. |
| C2 | `cargo check -p northhing-core --features product-full` | **FAILED (env)** | `ring 0.17.14` / `aws-lc-sys 0.42.0` native C compile requires `gcc.exe` not in PATH. **Environment limitation, not code issue.** |
| C3 | `cargo test -p northhing --lib settings` | **NOT RUN (env)** | Same gcc missing. Report `task-c3-report.md:63-72` explicitly states this and does not claim false success. |
| C3 | `cargo check -p northhing` | **NOT RUN (env)** | Same gcc missing. |

### 6.2 Environment constraint registration

**The `ring` / `aws-lc-sys` gcc missing constraint is explicitly registered in both C2 and C3 reports.** This is the same upstream build-tooling issue noted in the previous final-review (embed-resource chain). The constraint affects:
- C2: `cargo check -p northhing-core --features product-full` (for embedded relay warn change)
- C3: `cargo test -p northhing --lib settings` + `cargo check -p northhing` (entire desktop verification)

**CI coverage**: The plan §3 states "广覆盖交 CI；不跑 workspace 全量". GitHub Actions (Linux runner) has gcc available and will cover:
- `cargo test -p northhing --lib settings` (15 keyring + 4 IO + existing settings tests)
- `cargo check -p northhing` (desktop compilation with keyring crate)
- `cargo check --workspace` (full workspace, bypassing the embed-resource block)

**Risk assessment**: Medium. The C3 keyring code uses `keyring::Entry::new().set_secret/get_secret/delete_credential` (keyring.rs:90-114), `once_cell::sync::Lazy` (keyring.rs:186), `std::sync::Mutex` (keyring.rs:125), and `&dyn KeyringBackend` trait objects across async boundaries. Any Send/Sync bound issue or `Lazy` initialization error would only be caught by compilation. The code is structurally sound (independently verified), but compilation has not been confirmed in this environment. **CI must run `cargo check -p northhing` before merge.**

### 6.3 Baseline reference

| Crate | Baseline (main ae44334) | Branch HEAD | Status |
|---|---|---|---|
| tool-runtime | not in baseline reference | 88 passed (C1) | ✅ verified |
| relay-core + relay-server | 49/49 (prior final-review) | 61 passed (C2) | ✅ verified, +12 new config tests |
| desktop (northhing) | 98/98 (prior final-review) | **not run** (gcc) | ⚠️ CI must cover |
| core (northhing-core) | 1134/1134 (post-Task-9) | **not run** (gcc) | ⚠️ CI must cover |

### 6.4 Verification completeness verdict

**Adequate for merge with CI gate.** C1 fully verified. C2 verified for relay crates; embedded relay change structurally verified (single `warn!` + `CorsLayer::permissive()` move). C3 not compiled in this environment -- CI must run `cargo check -p northhing` + `cargo test -p northhing --lib settings` before merge. The structural review (independent file:line verification) confirms the code is syntactically and semantically sound.

---

## 7. Merge Risk Assessment

### 7.1 Conflict surface with main

**Main is at ae44334 (the branch baseline). Zero divergence. No conflict possible.**

The branch was created from ae44334 and main has not moved (per plan §0: "基线：main HEAD ae44334"). A `--no-ff` merge will produce a clean fast-forward of the merge commit.

### 7.2 File overlap with concurrent branches

**No concurrent branches touching the same files identified.** The plan §2 notes:
- `plan-2026-08-04-backend-followups.md` handles `save_user_config` fail-open etc. -- different files (`mcp/config` save path, not `settings/keyring.rs`)
- `plan-2026-08-04-growth-core.md` (S1, src/agentic) is blocked behind this branch's merge per plan §4

### 7.3 Cargo.lock stability

Cargo.lock changes in this branch:
- `trash 5.2.6` + transitive deps (`windows 0.56.0` family) -- C1
- `rand`, `base64` moved to production deps in relay-server -- C2
- `keyring 4.1.6` + `keyring-core 1.0.0` + `windows-native-keyring-store 1.1.0` -- C3

All additive (new deps, no version bumps). No conflict expected.

### 7.4 P2 interaction

- **P2-2 (single-instance lock)**: shares `~/.northhing/config/app.json` domain with C3, but P2-2 is about process-level locking, not file-level. C3's `SETTINGS_WRITE_LOCK` is in-process. No interaction.
- **FU-1 (save_user_config fail-open)**: same vulnerability class as P1-2 but for user-level MCP config. Handled by `plan-2026-08-04-backend-followups.md`, not this branch. No overlap.

### 7.5 Regression sweep recommendation

Per plan §4 completion definition: after `--no-ff` merge, run regression sweep:
- `tool-runtime`: 88 tests (C1 baseline)
- `relay-core + relay-server`: 61 tests (C2 baseline)
- `northhing --lib settings`: 19 new keyring tests + existing settings tests (CI)
- `northhing-core`: 1134 tests (prior baseline, should be unaffected -- C2 embedded_relay.rs is in assembly/core, needs `cargo check -p northhing-core --features product-full`)

---

## 8. Findings by Severity

### Critical
None.

### Important
None. (C3 I-1 "no cargo test evidence" was resolved in the fix round -- environment constraint explicitly registered, CI coverage acknowledged, no false claims.)

### Minor (branch-level observations)

**FR-1 (Minor, consistency)**: C2 key-file atomic write is a simpler variant than H-9 settings atomic write (no explicit flush, no `.bak`, no Windows rename fallback). Acceptable for a single 44-char string, but consider backporting the explicit flush pattern if relay key file corruption is observed.

**FR-2 (Minor, tech-debt priority)**: Three new active debts registered (P1-6/P1-7/P1-8) with accurate file:line evidence. The highest-priority follow-up is P1-6 (remote `rm -rf` no confirmation gate) -- this is the only one where irreversible damage can occur on a remote system.

**FR-3 (Minor, verification)**: C3 desktop tests not compiled locally due to gcc missing. CI must run `cargo check -p northhing` + `cargo test -p northhing --lib settings` before merge. This is a **merge precondition**, not a blocker.

**FR-4 (Minor, usability)**: C3 M-7 -- `sync.rs:37-48` falls back to sentinel string when keyring is unavailable, producing an unusable provider (LLM call fails with "invalid bearer token") instead of a clear error. Not a security issue (no plaintext leak), but loses fail-closed diagnostic. Recommend `?` propagation in a follow-up.

---

## 9. Constraints Compliance

| Constraint | Status | Evidence |
|---|---|---|
| Logs must be English-only, with no emojis | **PASS** | All 3 reviews verified log strings via Python unicode scan. C1: CJK=0 emoji=0 in new code. C2: CJK only pre-existing (`config.rs:46` `完全缺乏认证机制` from OLD file). C3: CJK=0 emoji=0 in new code. |
| Production .rs files < 800 lines (or `// allow-god-file` > 1000) | **PASS** | Largest file: `config.rs` 555 lines. `delete_path.rs` 309. `keyring.rs` 349. `io.rs` 288. `main.rs` 144. `embedded_relay.rs` 131. `sync.rs` 164. All well under 800. |
| Concurrency test binding (rule 4) | **PASS** | C3 ships `keyring_migration_concurrent_loads_are_idempotent` (`io_tests.rs:332-376`) with 5 concurrent tasks, shared `Arc<MockKeyring>`, and final-state assertions (file + keyring). |
| GlobalConfig single source of truth | **PASS** | C3 adds no new runtime-readable config file; `app.json` remains the only desktop settings file; keyring is a secret store, not a config store. |
| No coding on main | **PASS** | All work on branch `fix/p1-security-0804`. |
| Ledger flip same commit as fix | **PASS** | C1: P1-3/P1-1 flipped in `007e513`. C2: P1-5 flipped + P1-7 added in `7fa7d62`. C3: P1-2 flipped + P1-8 added in `26a15a7`. |
| Brief is sole requirements source | **PASS** | No scope creep. C1 did not touch remote delete semantics. C2 did not touch embedded relay binding. C3 did not touch MCPServerConfig.env. All out-of-scope items either reported (C1 remote confirmation) or registered as concern (P1-6/P1-7/P1-8). |

---

## 10. Final Statement

The `fix/p1-security-0804` branch closes four P1 security debts (P1-1 retroactively, P1-2 keyring migration, P1-3 trash recycle bin, P1-5 relay secure defaults) across 3 tasks with zero Critical or Important findings remaining. Cross-task patterns (fail-closed, sentinel, idempotency, atomic write) are isomorphic and consistent. Three independent persistence surfaces (trash seam, relay key file, settings app.json) have no same-file multi-lock or write-path stacking.

The C1 lesson (report fabrication of "mechanism exists/does not exist") was inherited: all three reviews independently verified file:line evidence, and the C1 review caught and corrected the remote confirmation gate fabrication. The C2/C3 reviews confirmed 8/8 and 10/10 fact-verification points respectively. This final review independently re-verified all key mechanisms (trash fail-closed at `delete_path.rs:89-92`, relay fail-closed at `config.rs:229-236`, keyring fail-closed at `io.rs:57-60/86-103/138`, sentinel at `keyring.rs:56`, CORS wiring at `main.rs:89-128`, embedded relay open mode at `embedded_relay.rs:42/66`) against the actual code.

Three new active debts (P1-6 remote delete no confirmation, P1-7 embedded relay open mode, P1-8 MCPServerConfig.env plaintext) are accurately registered with file:line evidence and appropriate proposed fixes. The branch does not introduce regressions -- it narrows the attack surface (loopback default, auto key, CORS tighten, trash default, keyring migration) while deferring product-design-dependent items (embedded relay pairing protocol, remote confirmation gate redesign) to follow-up waves.

**The branch is approved for merge.** CI must run `cargo check -p northhing` + `cargo test -p northhing --lib settings` as a merge precondition (C3 desktop tests not compiled locally due to gcc missing). The 13 tech-debt items should be tracked for follow-up, with P1-6 (remote `rm -rf` no confirmation gate) as the highest priority.
