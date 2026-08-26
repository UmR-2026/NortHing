# Final Review — consult-room prescription v3 wave (d4f8779..f6e8c45)

Reviewer: independent final reviewer (fresh eyes, no per-task involvement). Read-only pass; tree/index/HEAD not mutated. Scope: P1c(+fix), P2a, P2b, P3a, P3b + SDD docs commits.

## Verdict

**NEEDS FIXES** — one blocking item, docs-only:

- **B-1**: Tech-debt ledger **P2-6** ("Event queue silently drops events when full") is still `active` although P2b (df47924) implemented **all three** of its Proposed-fix items (Err on full / Critical never dropped / StreamEventSink logs Err). House rule 2 (AGENTS.md: "resolving a tech-debt item requires flipping its ledger status in the same commit. No 'doc later'"). Fix = one ledger entry: status → `resolved` (evidence lines updated from stale `queue.rs:85/:127` to the post-P2b locations, ~`queue.rs:99-104` / `:241-247`). No code change required; after that commit the wave is CAN MERGE.

Everything else passes: code compiles at the tip (verified below), cross-task integration is sound or explicitly sanctioned, and the per-task verdicts hold up under fresh review.

## Cross-task findings

1. **Important (blocking)** — *P2b resolved ledger P2-6 without flipping it (house rule 2).*
   `docs/status/tech-debt-ledger.md:126-131` still reads `Status: active` with stale evidence (`queue.rs:85` drops+Ok, `queue.rs:127` `let _ =`). P2b's own ledger row (progress.md L438) describes implementing exactly P2-6's fix. `git show --stat df47924` = queue.rs only (1 file, +74/−5) — the ledger was never touched. The P2b brief scoped only queue.rs, so the miss originated at brief level, but house rule 2 applies regardless. This is precisely the cross-task integrity gap per-task review could not catch.

2. **Important (non-blocking — register as new debt)** — *Onboarding session is invisible to the room: workspace-scoping mismatch between P3a and P2a/P0b.*
   - P3a creates the onboarding session with `workspace_path: Some(ws)` (`pages_onboarding.rs:693-699`).
   - The room's `ensure_room_session` does `list_sessions()` → first (`api.rs:68-77`), and facade `list_sessions` is hard-scoped to `default_workspace_path()` = **`std::env::current_dir()`** (`kernel_facade/session.rs:38-49`, `helpers.rs:8-12`). `ensure_room_session`'s own create also uses `workspace_path: None` → CWD.
   - Consequence: unless the user's onboarding workspace happens to equal the process CWD (default input `E:\agent-project\northing\workspace` ≠ typical launch CWD), the onboarding session is **never in the room's candidate set** — the room silently starts/uses a parallel CWD session. Prescription §F6's "create_session → 启动首个 session" goal is unmet in the common case; "shadowed by an older session" is actually the milder variant of the same seam (first-item-wins would only matter inside the same workspace).
   - Why non-blocking: the P3a brief explicitly sanctioned `create_session` as **best-effort with `ensure_room_session` as the authoritative fallback** ("与 P0b 语义一致"), and the room stays fully functional in every branch. But the sanction's wording covered "create may fail", not "create succeeds yet is scoped out of the room's view" — so this deserves its own ledger entry with an owner (fix = facade workspace resolution / room-session identity, next wave). Neither per-task review could see this; both verified their own half correctly.

3. **Minor** — *Narrow double-session-creation TOCTOU between P2a startup future and P0b-era lazy ensure.*
   `app.rs:114-138` (startup `use_future` → `ensure_room_session`) and `app.rs:307-319` (`send_action` lazy ensure) are two independent callers of the non-atomic list→create in `api.rs:68-77`. If a user sends before the startup future finishes **and** no session exists yet (only possible when P3a's best-effort create failed or was scoped away per finding 2), both can see an empty list and both create → one orphaned session; `session_id_signal` last-write-wins. Window is milliseconds; pattern predates this wave (P0b), P2a adds the second caller. Fold into finding 2's fix (idempotent ensure / single session-identity owner). Signal-write ordering hazard is the already-recorded P2a M1 (entries.set may overwrite an Approval pushed during the startup window) — no new variant found: TextChunk flows to `assistant_draft`, which `entries.set` does not touch.

4. **Minor (verified clean)** — *P1c MCP-env keyring vs C3 provider api_key coexistence: no collision, no reentrancy.*
   - Sentinels: `"__kr__"` is an exact-match **string value** in a provider `api_key` field (`is_keyring_sentinel`, `keyring.rs:239-245` uses `==`); `"__kr_env__"` is a **map key** in `MCPServerConfig.env` (`is_env_sentinel` = `contains_key`). Different types, different fields, exact equality — no cross-trigger possible.
   - Keyring accounts: bare `provider_id` vs `mcp-env:{server_id}` — disjoint unless a provider is literally named `mcp-env:*` (negligible).
   - Migration order: io.rs load/save path contains **only** the MCP migration (`io.rs:50,53,63,89,138`); C3 provider-key callers live elsewhere (`callbacks_settings/provider.rs:5,45`, `ui_dioxus/api.rs:127`). No shared code path, no ordering dependency; both reuse `PRODUCTION_KEYRING`.
   - `SETTINGS_WRITE_LOCK`: tokio Mutex is non-reentrant but never double-acquired — inner `*_at` fns are lock-free; all production callers (`load_app_settings_locked`, `update_app_settings_at`) hold the lock exactly once; P1c's load-time save runs under the caller's guard. No deadlock path.

5. **Minor (verified clean)** — *P2b Result change: zero new callers from this wave.* `rg "\.enqueue\(" src/apps/` → no matches; P2a/P3a/P3b added no enqueue call sites. HEAD census matches the P2b review's 11 production sites, all `let _ =` fire-and-forget or explicit `if let Err` (turn_persist:113,190) — no Ok-always assumption anywhere.

6. **Minor (verified clean)** — *P3b cleanup scheduler vs other bootstrap spawns.* The spawn (`main.rs:66-80`) lands on the worker runtime after `init_core()` completes, kept alive by `shutdown_rx.recv()` (`main.rs:180`). Retention thresholds (`cleanup.rs:22-32`: temp 7d / logs 30d / cache only >1 GB) cannot touch files freshly created by concurrent bootstrap activity (MCP background init etc.); `PathManager::default()` resolves the same user-config root as core (`path_manager.rs:61-71,124-141`, non-panic fallback verified). No interference path found.

7. **Minor (new)** — *P3a orphaned i18n key `ONBOARDING_BTN_COMPLETE`* (`i18n.rs:387`): the 3-step button rewrite (5d2d22c, verified via `git log -S`) removed the last user, adding one compiler warning (35 → 36 in my HEAD check). Trivial; delete now or reuse when step buttons get i18n keys.

8. **Observation** — the dispatch text says "10 commits"; `git rev-list --count d4f8779..f6e8c45` = **9** (6 code + 3 docs). Clerical only; all ledger commit ranges are internally consistent.

## Requirements coverage table

| Prescription § | Task | Status | Notes |
|---|---|---|---|
| §B2 MCP env keyring | P1c | **done / deviated-sanctioned** | store_env/load_env + io load/save sentinel + 6 integration tests. Sanctioned: user ruled P1-8 stays `active` (field dead after K4a; real plaintext is core Cursor format) — ledger correctly reflects ruling; `MCPServerConfig` revived as `#[allow(dead_code)]` carrier for the io path (recorded). |
| §F1 room startup dataflow | P2a | **done / deviated-sanctioned** | Errata recorded in brief §已解决歧义 + ledger row: `get_session` → `get_messages` (SessionDto has no message body; real API session.rs:265). Seed kept as fallback, TODO(data) markers placed, 5 pure-fn tests. |
| §B4 event queue | P2b | **done — gap: ledger P2-6 not flipped** | Code 100% of prescription (Result + Critical bypass + trait impl error!, single-lock preserved). Blocking item B-1 is the docs sync, not the code. |
| §F6 onboarding | P3a | **done / deviated-sanctioned** | 3-step gate + real `test_provider_config` + side-effects in exact order (fail-closed/fail-closed/best-effort). Sanctioned known limitation: provider not registered in GlobalConfig (recorded in ledger row). New seam found: finding 2 (workspace scoping). |
| §B3 cleanup scheduling | P3b | **done / deviated-sanctioned** | Anchor drift lib.rs → main.rs (2nd recorded instance); `interval_at` over bare `interval` per brief preference (documented choice); ledger P2-4 narrowed same-commit with status kept `active` (partial fix — correct per house rule 2). |
| SDD docs commits | 2c54f33 / fa39edb / f6e8c45 | **done** | Ledger rows, briefs, reports, review artifacts all landed; progress.md rows match commit ranges. |

## Minor triage table

| origin | finding | recommendation |
|---|---|---|
| P1c r2 M1 | loader-internal save double-walk (`io.rs:51-55` + `:137-145`): 2nd migration always no-op but costs a clone + keyring rewrite per update | **defer-with-owner** — next B2 real-wiring wave; correct & idempotent today |
| P1c r2 M2 | `MCPServerConfig`/`mcp_servers` revival ~60L `#[allow(dead_code)]` awaiting production caller | **defer-with-owner** — at B2 facade wiring: drop allow + add production-caller test |
| P1c r2 M3 | ledger P1-8 evidence wording ("originally … in") reads oddly after type revival | **accept-and-close** — "Stale after K4a" note already carries the truth |
| P1c r2 M4 | fix commit message embeds "judge Important #2" | **accept-and-close** — traceability marker, no functional impact |
| P1c r2 M5 | rustup not on controller PATH (MSVC channel needs full cargo path) | **accept-and-close** — environment issue; workaround command now固化 in briefs |
| P2a M1 | startup `entries.set` may overwrite Approval card pushed by event future during hydrate window | **defer-with-owner** — next room-dataflow iteration (merge-not-replace, or startup suppression flag); fold with finding 3 |
| P2a M2 | sid filter `unwrap_or(true)` unchanged (accept-all before sid set) | **accept-and-close** — 禁区 correctly honored; v1 single-room semantics accepted since P0b |
| P2b M-1 | brief anchor line drift (turn_persist :118,198 → actual :113,190) | **accept-and-close** — planning-process note; report used live numbers |
| P2b M-2 | report's `cargo check --workspace` tail truncated (no Finished line) | **accept-and-close** — MSVC test pass is stronger evidence; paste-discipline note for future reports |
| P2b M-3 | `EventQueueFull` derives PartialEq/Eq beyond brief | **accept-and-close** — beneficial addition, exercised by the new test |
| P2b M-4 | `stats.pending_events` may exceed max after Critical bypass; no downstream consumer | **accept-and-close** — judge verified repo-wide zero consumers |
| P3a M1 | `pages_onboarding.rs` 866 lines > 800 warning line | **defer-with-owner** — split Step/step_gate/DTO-assembly before next expansion (house rule 3 warning zone) |
| P3a M2 | Card2 test button can bypass Step1 gate | **accept-and-close** — Step3 gate backstops; UX-preference level |
| P3a M3 | redundant `clone()` in `add_workspace` closure | **accept-and-close** — style, single occurrence |
| P3b M1 | spawn block fully-qualified paths could be `use` imports | **accept-and-close** — brief explicitly allowed either |
| final review | `ONBOARDING_BTN_COMPLETE` i18n key orphaned (new warning, finding 7) | **defer-with-owner** — delete or reuse at next onboarding touch |

Note: earlier-wave Minors (P0a–P1b rows in progress.md) remain carried in the ledger rows; they predate this range and stay open per their own triage notes.

## Evidence audit

- **Diff**: read the full package once (3156 lines incl. all briefs/reports/artifacts). Working tree clean, HEAD = f6e8c45.
- **My own check**: `cargo check -p northhing` @ f6e8c45 → `Finished dev profile in 2.39s`, 0 errors, 36 warnings (house rule 6 satisfied at branch tip). Warning count 35→36 vs P1c-era reports is explained by finding 7 (verified via `git log -S`).
- **P1c MSVC (settings 77 / keyring 23)**: second-hand. Numbers appear in `re-review.md` and the progress row as orchestrator-collected; the raw output tails were **not committed** to any artifact. The judge explicitly documented accepting controller-supplied evidence (rustup absent from PATH; `cargo check --tests` exit 0; semantic verification of the fix1 deletion). Trusted with that caveat — it is the only claim in the wave without a committed raw tail.
- **P2b MSVC (1 passed)**: raw tail committed in the report (`test result: ok. 1 passed; 0 failed; … 1049 filtered out`).
- **P3a MSVC (28 passed)**: raw tail committed with all 28 test names — which also constitutes the actual test-run evidence for P2a's 5 `messages_to_entries` tests (all appear by name in that run).
- **P2a / P3b cargo check tails**: committed in their reports (Finished lines present).
- **Integration facts**: verified directly against HEAD source, not reports — facade workspace scoping (`kernel_facade/session.rs`, `helpers.rs`), lock structure (`io.rs` full read), sentinel mechanics (`keyring.rs`), bootstrap topology (`main.rs` full read), cleanup policy (`cleanup.rs`), enqueue census (repo-wide grep), ledger states (P1-8/P2-4/P2-6 at HEAD).

## Reasoning

The wave's code is solid: all five §-blocks landed as narrowed and sanctioned, per-task reviews were accurate within their lanes, and HEAD compiles cleanly under house rule 6. The two genuinely new things this final pass found are both things no single-task review could see: P2b silently completed ledger P2-6 without the same-commit status flip house rule 2 demands (the one blocking item, docs-only), and the P3a↔P2a workspace-scoping mismatch that makes the onboarding session invisible to the room (acceptable this wave under the explicit best-effort sanction, but it must be registered as debt with an owner rather than discovered as a "mystery" later). Everything else is either verified-clean integration seams (keyring coexistence, enqueue census, cleanup spawn) or accumulated Minors triaged above.
