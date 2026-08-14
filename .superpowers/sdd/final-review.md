# Final Branch Review: fix/backend-debug-0731

**Reviewer**: Independent final reviewer (did not participate in any single-task implementation or review)
**Scope**: `git diff c6096cb..1a65fc1` (10 commits, 40 files, +4598/-516)
**Branch**: `fix/backend-debug-0731` (worktree `northing-backend-debug`)
**Baseline**: c6096cb (main, 2026-07-31)
**Materials**: `.superpowers/sdd/task-01..08` brief/report/review, `progress.md` ledger, direct code verification

---

## 1. Branch-Level Dual Verdict

| Dimension | Verdict | Basis |
|---|---|---|
| **Spec compliance** | **PASS** | All 8 tasks PASS at single-task level; audit items C-1/C-2/H-1/H-2/V-1/M-8/H-5/H-6/H-7/H-8/H-9/M-9/M-2 all addressed; briefs' "explicitly out-of-scope" items respected; no scope creep |
| **Code quality** | **PASS** | No Critical or Important findings across any task; cross-task patterns are isomorphic and consistent; no same-file multi-lock; logs English-only; all god-file thresholds within limits |

## 2. Merge Conclusion

**CAN MERGE.** No must-fix-before-merge items. All accumulated findings are Minor (triage below). The branch substantially improves the security posture across relay auth/path-containment, vault fail-closed, persistence transactions, config fail-closed, and settings atomicity.

---

## 3. Cross-Task Consistency

### 3.1 Two ID validation types: ValidatedRoomId vs ValidatedPluginId

**Verdict: Semantically identical, duplication justified.**

| Aspect | ValidatedRoomId (relay-core/validated.rs:55-68) | ValidatedPluginId (assembly/core/lsp/plugin_loader.rs:60-73) |
|---|---|---|
| Allowed charset | ASCII alphanumeric + `-` + `_` | ASCII alphanumeric + `-` + `_` |
| Length | 1..=64 | 1..=64 |
| Construction | `TryFrom<&str>` / `TryFrom<String>` | `TryFrom<&str>` / `TryFrom<String>` |
| Error variants | Empty / TooLong / InvalidCharacter | Empty / TooLong / InvalidCharacter |

The validation rules are **character-for-character identical**. The only differences are error type names (`RoomIdError` vs `PluginIdError`) and display strings ("room ID" vs "plugin ID").

**Rule drift**: None. Both implement the same invariant. Task 8 review (N-3) independently confirmed this correspondence.

**Justification for duplication**: relay-core is layer 4 (services); assembly/core is layer 2. Assembly *could* depend on relay-core, but the review (task-08-review.md) confirms `ValidatedPluginId` has no cross-crate dependency on relay-core. This is correct architectural judgment: pulling in relay-core (a relay-specific service crate) for a 15-line ID validator in the LSP module would create inappropriate coupling. The duplication is trivial and maintainable.

### 3.2 Three atomic write patterns: isomorphism check

**Verdict: Sufficiently isomorphic. No missing .bak or Windows fallback.**

| Feature | json_store (H-5 vault) | bot persistence (H-6) | desktop settings (H-9) |
|---|---|---|---|
| `.bak` before write | vault adds `backup_vault()` separately | built-in (L661-664) | built-in (io.rs:157-161) |
| tmp naming | `.{name}.{pid}.{nonce}.{attempt}.tmp` | `.{name}.{pid}.{nonce}.tmp` | `.{name}.{pid}.{nonce}.tmp` |
| explicit flush | no (`fs::write`) | no (`std::fs::write`) | **yes** (File::create + write_all + flush + drop) |
| retry loop | yes (MAX_RETRIES + delay) | no (single retry) | no (single retry) |
| PermissionDenied fallback | yes (direct `fs::write`) | no | no |
| Windows rename fallback | yes (remove + retry) | yes (remove + retry) | yes (remove + retry) |
| 0o600 after write | yes (vault adds) | n/a | n/a |

**Key findings**:

1. **All three have .bak and Windows rename fallback.** No implementation is missing either. The vault achieves .bak via a separate `backup_vault()` call before `write_atomic`; bot and settings build it into their write function. All three use `path.exists() -> copy(target, target.bak) -> warn on failure` semantics.

2. **Settings (Task 7) has explicit flush** -- this is *strictly better* than the other two. It opens a `File`, writes, flushes, then drops the handle before rename. This guarantees data is on disk before the atomic rename. The json_store and bot versions rely on `fs::write`'s implicit close, which is adequate but less explicit.

3. **Bot and settings lack the PermissionDenied fallback** that json_store has. Under sustained Windows AV scanner contention, json_store falls back to non-atomic direct write; bot/settings return Err. This is *arguably safer* for security-sensitive data (fail rather than risk non-atomic write). The trade-off is acceptable.

4. **Bot and settings lack the retry loop**. json_store retries rename up to MAX_RETRIES with delay; bot/settings do a single retry. Under transient contention, bot/settings may fail where json_store would eventually succeed. But failure returns Err (safe), and the scenario is rare.

5. **First-write .bak behavior is consistent**: all three skip .bak when the target doesn't exist yet (no previous version to back up). This is correct by design, documented in Task 4 M-1.

### 3.3 read pattern unity: read_vault_file vs read_optional_source_file

**Verdict: Unified pattern. Both classify by ErrorKind::NotFound, avoiding TOCTOU.**

```
read_vault_file (password_vault.rs:97-105, auth.rs:158-166):
  match read_to_string(path).await {
    Ok(body) => parse(body),
    Err(NotFound) => Ok(default),      // legal empty state
    Err(other) => Err(other),           // propagate
  }

read_optional_source_file (storage_app_io.rs:317-324):
  match read_to_string(path).await {
    Ok(content) => Ok(content),
    Err(NotFound) => Ok(String::new()), // legal empty state
    Err(other) => Err(other),           // propagate
  }
```

Both use `ErrorKind::NotFound` classification (not `.exists()` pre-check), which avoids the TOCTOU race between check and read. The vault version additionally parses JSON; the source version returns raw string. Semantically unified.

**Minor inconsistency**: `esm_deps.json` in `load_source_from_dirs` (storage_app_io.rs:119-127) uses `.exists()` pre-check instead of the match pattern. If the file is deleted between `exists()` and `read_to_string()`, the read returns `NotFound` which gets mapped to `MiniAppStorageError::io(...)` (fail-closed Err). This is correct behavior but pattern-inconsistent with `read_optional_source_file`. Not a security issue -- the worst case is an IO error instead of an empty Vec, which is the safer direction.

---

## 4. Cross-Task Interaction

### 4.1 Task 2 three-state + Task 3 e2e cross-validation

**Verdict: Adequately cross-validated.**

Task 3's e2e tests (`tests/e2e_web_assets.rs`) run against the **full router** with all Task 1+2 changes applied:
- `ws_upgrade_requires_api_key_on_full_router` -- exercises Task 2 C-2 auth gate via raw TCP
- `upload_requires_key_then_roundtrips_to_disk_and_serve` -- exercises Task 2 C-1 upload auth + Task 1 ValidatedRelPath + DiskAssetStore roundtrip
- `traversal_variants_never_leak_sibling_marker` -- exercises Task 1 ValidatedRoomId/ValidatedRelPath containment via 9 variants + disk scan
- `check_web_files_counts_uploaded_hashes` -- exercises Task 1 M-8 fix (map failure counting)

The e2e tests validate the **happy path** of `create_room` (single room creation works end-to-end), which indirectly confirms the three-state logic doesn't break normal operation. The three-state **conflict/takeover** branches are covered by Task 2 unit tests (`create_room_conflict_keeps_original_room_and_desktop`, `create_room_takes_over_after_disconnect`, `create_room_takes_over_stale_heartbeat_connection`). No e2e test exercises conflict/takeover, but this is acceptable -- the unit tests provide atomicity guarantees that the e2e layer builds on.

### 4.2 Three locks: no same-file multi-lock

**Verdict: Confirmed safe. Three independent locks guard three different files.**

| Lock | Type | Scope | Guards file | Write path |
|---|---|---|---|---|
| Vault lock (Task 4) | `tokio::sync::Mutex<()>` per-instance | SSHPasswordVault / MCPOAuthCredentialVault | `~/.northhing/ssh_password_vault.json`, `~/.northhing/mcp_oauth_vault.json` | store/remove/migrate_entry/clear |
| Bot persistence lock (Task 5) | `static std::sync::Mutex<()>` | process-global | `~/.northhing/bot_persistence.json` | update_bot_persistence |
| Settings lock (Task 7) | `static tokio::sync::Mutex<()>` | process-global | `~/.northhing/config/app.json` | update_app_settings |

**No same-file multi-lock**: each lock guards exactly one file (vault has two vault files but each vault instance has its own lock). No write path touches more than one of these files in a single transaction.

**No lock stacking on same write path**: 
- Settings callbacks (provider/workspace/misc) write to `app.json` only; they call `sync_providers_to_core` which pushes to core `GlobalConfig` in-memory, not to vault or bot files.
- Vault is accessed only via `RemoteSSHManager` (SSH connection management), independent of settings callbacks.
- Bot persistence is written by remote_connect bot handlers (command_router/feishu/telegram/weixin), triggered by bot messages, not by settings UI actions.
- These are independent code paths with no shared write path.

**Note on json_store's internal lock**: `JsonFileStore::write_atomic` (used by Task 4 vault) has its own per-path `Arc<Mutex<()>>` registry (`JSON_FILE_WRITE_LOCKS`). This is a **second lock** on the vault file path, in addition to the vault instance lock. This is safe: the vault instance lock serializes the load-modify-write transaction, and the json_store path lock serializes concurrent atomic writes to the same path. They are nested (vault lock acquired first, then json_store lock), but since no other code path acquires the json_store lock for vault paths without also going through the vault instance lock, there is no deadlock risk.

### 4.3 Task 6 read_optional_source_file vs Task 4 read_vault_file

Already covered in section 3.3. Pattern is unified.

---

## 5. Minor Triage (per-item disposition)

### Disposition categories
- **A (accept as-is)**: no action needed, behavior is correct or cosmetic
- **D (tech-debt)**: track for follow-up, non-blocking
- **F (must-fix-before-merge)**: blocking -- none in this branch

### Task 1

| ID | Description | Disposition | Reason |
|---|---|---|---|
| Q-1 | Linux Path semantics test (withdrawn) | A | Withdrawn by reviewer; M-1 fix resolved the concern |
| Q-2 | api.rs:507 unused import B64 | A | Fixed in commit e3d0e53 |
| Q-3 | validated.rs:177-182 redundant is_drive_letter guard + inaccurate comment | resolved | Task B5 (`6b6419b`): removed redundant guard from Normal component, unified scan |
| Q-4 | validated.rs:162-171 double split scan can be merged | resolved | Task B5 (`6b6419b`): merged into single-pass split loop |
| M-4 | Test name `preserves_existing_dest_on_validation_failure` doesn't truly cover validation failure | resolved | Task B5 (`6b6419b`): renamed to `map_to_room_overwrites_existing_dest_with_new_content` and added genuine validation rejection test |
| M-5 | map_to_room TOCTOU window (theoretical) | A | Theoretical micro-window between canonicalize and remove_file. Containment check is defense-in-depth, not sole barrier (ValidatedRelPath is primary). Accepted trade-off. |

### Task 2

| ID | Description | Disposition | Reason |
|---|---|---|---|
| M-1 | on_disconnect micro-window between conn_to_room.remove and rooms.get_mut | A | Alternative order (set tombstone first) introduces worse race (new conn marked tombstone by old disconnect). Current order is the better choice. Desktop retry recovers. |
| M-2 | handle_socket task panic won't release connection slot | resolved | Task B5 (`6b6419b`): local `ConnectionSlotGuard` RAII guard auto-releases on drop/panic/upgrade-fail |
| M-3 | handle_text_message return style mixed | resolved | Task B5 (`6b6419b`): unified expression/return convention across all match arms |
| M-4 | AuthExtractor::Clone expands public API | A | Clone is a common derive; no security surface expansion. |
| (deferred) | Capability token system | D | Brief explicitly deferred. Current atomic three-state is the scoped fix. Track as separate security enhancement. |

### Task 3

| ID | Description | Disposition | Reason |
|---|---|---|---|
| M-1 | is_genuine_traversal mirrors handler logic (drift risk) | resolved | Task B5 (`6b6419b`): added anchor comment pointing to `serve_room_web_catchall` line in relay-core |
| M-2 | 9 variants fixed, no fuzz | A | Disk recursive scan provides generalized backstop. Manual variants cover known attack vectors. |
| M-3 | attribution() only eprintln, not assertion | A | Attribution is diagnostic info; hard-asserting would overfit axum version behavior. |
| M-4 | dechunk boundary conditions untested | A | All test endpoints use Content-Length, not chunked. No real exposure. |
| M-5 | NUL/long/Unicode secondary attack surface not e2e tested | A | Unit tests (handler_tests) cover NUL/control chars and non-ASCII. E2e doesn't need to repeat. |
| M-6 | Room ID case sensitivity untested | A | Product semantics, not security. `e2e-room` and `E2E-ROOM` are different rooms by design. |
| (observation) | SPA fallback 200 on double-encoded variant | D | Product decision: `%252e%252e` returns 200 + index.html (not a traversal leak). If strict non-200 is desired, change `get_file` fallback strategy. Track as product decision. |
| (observation) | Single-decode semantics depends on axum 0.8.9 | D | If axum upgrades decode behavior, re-verify variants. Test is partially robust to drift. Track as upgrade checklist item. |

### Task 4

| ID | Description | Disposition | Reason |
|---|---|---|---|
| M-1 | First-write no .bak (design correct) | A | No previous version to back up on first write. Design correct. |
| M-2 | set_permissions failure silently swallowed | D | **resolved (Wave 2 B6)**: `tracing::warn!` added on chmod failure for ops visibility (SSH + MCP OAuth vaults). |
| M-3 | Report Cargo.lock attribution misread | A | Documentation error in report, not in code. Already clarified in review. |
| M-4 | vault filter naming causes 2 tests missed by filter | D | **resolved (Wave 2 B6)**: renamed tests to `vault_*` so the `vault` filter catches all four (SSH + MCP OAuth vaults). |

### Task 5

| ID | Description | Disposition | Reason |
|---|---|---|---|
| M-1 | Poison lock recovery no warn log | D | **resolved (Wave 2 B6)**: added `warn!("Bot persistence write lock poisoned, recovering")`. |
| M-2 | Concurrent test limited on single-core | A | Test validates correctness (serialization equivalence). Perf validation belongs in CI. |
| M-3 | NoHomeDirectory no recovery guidance | A | Pre-existing behavior. Not introduced by this branch. |
| M-4 | tmp write failure leaves orphan .bak | A | .bak retains old content (correct). Main file unchanged (correct). Best-effort cleanup is sufficient. |
| M-5 | Windows rename fallback TOCTOU | A | Same trade-off as json_store (H-5). Cross-process atomic write needs filesystem-level locks, out of scope. |
| M-6 | Cargo.lock attribution (same as Task 4 M-3) | A | Documentation, not code. |

### Task 6

| ID | Description | Disposition | Reason |
|---|---|---|---|
| (triage) | save_user_config same fail-open pattern as H-7 | D | **Same vulnerability class as H-7** (read-modify-write fail-open), but for user-level MCP config. Brief scoped H-7 to `project.mcp_servers` only. Recommend follow-up task to apply the same strict-variant fix to `save_user_config`. This is the highest-priority tech-debt item in this triage. |

### Task 7

| ID | Description | Disposition | Reason |
|---|---|---|---|
| M-1 | save_app_settings public wrapper dead code warning | D | `cargo check -p northhing` emits `warning: function save_app_settings is never used`. Recommend deleting the wrapper (option A, consistent with H-5/H-6 deleting old save APIs). Trivial. |
| M-2 | upsert_provider unknown-type branch UI text regression | D | **resolved (Wave 2 B7)**: restored the specific message (`不支持的服务类型: {ptype}`) via the validation_error channel. |
| M-3 | dedup migration save path unlocked in public load path | D | Residual race: `load_app_settings()` (read-only) triggers dedup write without lock. Window is narrow (dedup only fires on duplicate providers). Recommend extracting dedup from load path into `update_app_settings` explicitly. |

### Task 8

| ID | Description | Disposition | Reason |
|---|---|---|---|
| M-1 | Windows symlink test silently skips | D | **resolved (Wave 2 B7)**: `eprintln!` reports the skip instead of silently returning. |
| M-2 | get_plugin_dir API adaptation list minor doc omission | A | Documentation only. No code impact. |
| M-3 | get_server_path consistency suggestion | A | Pure readability. Current design is valid. |
| M-4 | plugin_dir.exists() TOCTOU (concurrent dual install) | D | Pre-existing. Two concurrent installs of same ID both pass exists(), both stage, one rename wins. Consider `create_dir(plugin_dir, exclusive)` to make install atomic. Track for follow-up. |
| M-5 | Logging exposes raw string not validated repr | D | **resolved (Wave 2 B7)**: log now prints only the validation error, not the raw dir name. |
| M-6 | cargo fmt observation | A | No action needed. |
| M-7 | schedule_repo_release test evidence strength | D | **resolved (Wave 2 B7)**: added `schedule_repo_release_for_test` seam in services-integrations that observes the idle-session release directly. |
| M-8 | LspManager::uninstall_plugin calls stop_server(language=plugin_id?) -- pre-existing path unmapping bug | D | **Pre-existing functional bug**: uninstall passes plugin_id to stop_server which expects language key. LSP process not actually stopped on uninstall. Not introduced by this branch (M-9 only adds validation). Recommend follow-up task. |

### Summary counts

| Disposition | Count | Items |
|---|---|---|
| F (must-fix-before-merge) | 0 | -- |
| D (tech-debt) | 18 | T1: Q-3, Q-4, M-4; T2: M-2, M-3, capability-token; T3: M-1, SPA-fallback, axum-dep; T4: M-2, M-4; T5: M-1; T6: save_user_config; T7: M-1, M-2, M-3; T8: M-1, M-4, M-5, M-7, M-8 |
| A (accept) | 16 | T1: Q-1, Q-2, M-5; T2: M-1, M-4; T3: M-2, M-3, M-4, M-5, M-6; T4: M-1, M-3; T5: M-2, M-3, M-4, M-5, M-6; T8: M-2, M-3, M-6 |

**Highest-priority tech-debt items** (recommend follow-up tasks):
1. **T6 save_user_config fail-open** -- same vulnerability class as H-7, user-level MCP config
2. **T8 M-8 LspManager uninstall stop_server path bug** -- pre-existing functional bug, LSP process not stopped on uninstall
3. **T7 M-3 dedup migration unlocked write** -- residual race in public load path
4. **T7 M-1 dead code warning** -- trivial delete, eliminates CI noise

---

## 6. Verification Completeness

### 6.1 Per-task focused verification (all confirmed via review evidence)

| Task | Verification command | Result |
|---|---|---|
| 1 | `cargo test -p northhing-relay-core -p northhing-relay-server` | 24/24 pass |
| 2 | `cargo test -p northhing-relay-core -p northhing-relay-server` | 37+7=44 pass |
| 3 | `cargo test -p northhing-relay-server --test e2e_web_assets` | 5/5 pass (49 total incl. prior) |
| 4 | `cargo test -p northhing-services-integrations --features product-full vault` | 17/17 pass |
| 5 | `cargo test -p northhing-core --features product-full remote_connect` | 7 new + 55 existing = 62 pass |
| 6 | `cargo test -p northhing-services-integrations --features product-full mcp` + `miniapp` + `sync_from_fs` | 11+29+1=41 pass |
| 7 | `cargo test -p northhing --lib settings` | 59/59 pass (6 new + 53 existing) |
| 8 | `cargo test -p northhing-core lsp` + `plugin` + `schedule_repo_release` | 12 lsp + M-2 warning gone |

### 6.2 Branch-level gaps

**Gap 1: No `cargo check --workspace` run.**
- All briefs note workspace-level check is blocked by upstream `embed-resource 3.0.11` / `webdriver -> tauri` chain issue.
- Each task ran its own crate-level `cargo check`. The crates are self-contained:
  - relay-core + relay-server (Tasks 1-3)
  - services-integrations (Tasks 4, 6)
  - assembly/core (Tasks 5, 8)
  - desktop (Task 7)
- Cross-crate dependencies: Task 8's `search/service.rs` depends on `services-integrations/workspace_search`; verified by `cargo check -p northhing-core`. Task 6 touched `assembly/core/miniapp/manager`; verified by `cargo test -p northhing-core sync_from_fs`.
- **Risk**: low. The upstream block is a build-tooling issue, not a code issue. CI will run the full workspace check.
- **Recommendation**: ensure CI runs `cargo check --workspace` (or the equivalent that works around the embed-resource issue) before merge.

**Gap 2: Embedded relay (api_key=None) not e2e tested.**
- Resolved in Task B5 (`6b6419b`): Added `open_relay_when_api_key_none_accepts_all_routes_without_auth` in `e2e_web_assets.rs` covering WebSocket upgrade, file upload, check-files, and asset serving on the full router without API key. Gap closed.

**Gap 3: No combined relay stress test.**
- The three-state logic, connection limit (512), bounded queue (256), and idle timeout (90s) are each unit-tested individually. No test exercises all four simultaneously under load.
- **Risk**: low. Each mechanism is independently verified. Combined stress testing belongs in CI/integration environments.
- **Recommendation**: acceptable as-is for merge. Track for CI integration test enhancement.

**Gap 4: Cross-module race (desktop settings lock vs core GlobalConfig write) acknowledged but untested.**
- Task 7 brief explicitly defers this: "core 写与 desktop 写的跨模块竞态记终审 triage". The desktop lock only guards desktop process-internal callbacks; core `ConfigManager` writes independently.
- **Risk**: low for this branch. The race is pre-existing (desktop and core always wrote independently). Task 7 doesn't make it worse -- it fixes the desktop-internal race.
- **Recommendation**: track as tech-debt. The `sync_providers_to_core` push path is the coordination point; if core-side locking is needed, it's a separate task.

### 6.3 Verification completeness verdict

**Adequate for merge.** All focused verifications pass. The gaps are either blocked by upstream tooling (workspace check), low-risk subset paths (api_key=None e2e), or pre-existing acknowledged limitations (cross-module race). CI should run the full workspace check.

---

## 7. Merge Risk Assessment

### 7.1 Conflict surface with main

**Zero file overlap. Confirmed conflict-free merge.**

Main has diverged from c6096cb with 8 commits, all `docs(design):` commits touching **only** files under `docs/design/2026-07-22-frontend-redesign/` (prototype HTML files, CSS, markdown handoff docs).

This branch touches 40 files, **none** of which are under `docs/design/`:
- `src/apps/desktop/` (8 files: settings callbacks + settings io + Cargo.toml)
- `src/apps/relay-server/` (5 files: lib, main, config, Cargo.toml, e2e test)
- `src/crates/assembly/core/` (10 files: bot, lsp, miniapp, search)
- `src/crates/services/relay-core/` (7 files: validated, room, websocket, api, lib, handler_tests, Cargo.toml)
- `src/crates/services/services-integrations/` (8 files: vault, auth, mcp config, miniapp storage, tests)
- `Cargo.lock`, `scripts/core-boundaries/rules/feature-rules.mjs`

The `Cargo.lock` is the only file that might conflict if main also adds dependencies -- but main's 8 commits are all documentation-only (no Cargo.lock changes). **No conflict expected.**

### 7.2 Desktop UI file overlap

The branch touches `src/apps/desktop/src/app_state/callbacks_settings/` and `src/apps/desktop/src/app_state/settings/`. These are **settings backend/callback** files, not UI rendering files. Visual iteration on main touches prototype HTML/CSS under `docs/design/`, not Rust desktop code.

If visual iteration later moves from prototype HTML to actual Slint/desktop Rust code, the overlap surface would be:
- `callbacks_settings/provider.rs` (if UI feedback wiring changes)
- `callbacks_settings/misc.rs` (if onboarding/default-model UI changes)
- `callbacks_settings/workspace.rs` (if workspace picker UI changes)

But as of the current main state, there is **zero overlap**.

### 7.3 Cargo.lock stability

`Cargo.lock` changes in this branch:
- `serde_json` + `base64` dev-deps added to relay-server (Task 3)
- `northhing-test-support` dev-dep added to desktop (Task 7) and assembly/core (Task 8)
- `northhing-services-core` feature dependency added to services-integrations (Task 4)

All are additive (new deps, no version bumps). Main's documentation-only commits don't touch Cargo.lock. **No conflict expected.**

---

## 8. Findings by Severity

### Critical
None.

### Important
None.

### Minor (branch-level observations, not per-task)

**FR-1 (Minor, consistency)**: The three atomic write implementations diverge in two features: explicit flush (only settings has it) and PermissionDenied fallback (only json_store has it). Consider backporting the explicit flush pattern from settings to bot persistence for consistency. Not blocking. → **resolved (Wave 2 B6)**: bot persistence now writes via `File::create` + `write_all` + `flush` + drop-before-rename, matching the settings pattern.

**FR-2 (Minor, consistency)**: `esm_deps.json` in `load_source_from_dirs` uses `.exists()` pre-check while `read_optional_source_file` uses `ErrorKind::NotFound` match. Pattern inconsistency, not a security issue (fail direction is safe). Consider unifying in a follow-up. → **resolved (Wave 2 B6)**: `load_source_from_dirs` now matches on `ErrorKind::NotFound`, unifying with `read_optional_source_file`.

**FR-3 (Minor, verification)**: No e2e test covers the embedded relay path (`api_key=None`). Unit tests cover the auth logic. Acceptable for merge; track for CI enhancement.

**FR-4 (Minor, tech-debt priority)**: The highest-priority tech-debt items from the triage are:
1. T6 `save_user_config` fail-open (same class as H-7)
2. T8 M-8 `stop_server` path unmapping bug (pre-existing functional bug)
3. T7 M-3 dedup migration unlocked write (residual race)

These should be tracked as follow-up tasks but do not block this merge.

---

## 9. Constraints Compliance

| Constraint | Status | Evidence |
|---|---|---|
| Logs must be English-only, with no emojis | PASS | All 8 reviews verified log strings; no CJK or emoji in new tracing calls. Pre-existing Chinese context strings in settings io.rs are noted but not changed (briefs explicitly allow). |
| Production .rs files < 800 lines (or `// allow-god-file` > 1000) | PASS | Largest file: `bot/mod.rs` at 774 lines (< 800). All others well under. |
| Concurrency test binding (rule 4) | PASS | Tasks 2, 5, 7 all ship automated concurrency tests (tokio::select!/cancellation/timeout races covered). |
| GlobalConfig single source of truth | PASS | Task 7 adds no new runtime-readable config file; `app.json` remains the only desktop settings file. |
| No coding on main | PASS | All work on branch `fix/backend-debug-0731`. |

---

## 10. Final Statement

The `fix/backend-debug-0731` branch addresses 13 audit items across 8 tasks with zero Critical or Important findings. Cross-task patterns (ID validation, atomic writes, file-read fail-closed) are isomorphic and consistent. Three independent persistence locks guard three different files with no same-file multi-lock or write-path stacking. The merge surface with main is zero-overlap (main moved on documentation-only prototype files).

**The branch is approved for merge.** The 18 tech-debt items should be tracked for follow-up, with T6 `save_user_config` and T8 M-8 `stop_server` as the highest priorities.
