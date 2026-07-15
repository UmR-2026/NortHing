# R20c Impl — bitfun-acp manager_config + manager_connection sub-domain split (close QClaw R20a P2)

> **Implementation handoff.** Producer: Coder agent. Branch: `impl/r20c-manager-config-connection-split`.
> All targets met per R20c spec (`docs/handoffs/2026-07-01-r20c-manager-config-connection-split-spec.md`).
> Verified via single cargo check + single cargo test cycle at the end (R8 + R14 lesson applied).

---

## Summary

R20c splits `acp/client/manager_config.rs` (292 canonical lines) + `acp/client/manager_connection.rs` (287 canonical lines) into **4 sibling sub-domain files** (no facade). Both files were QClaw R20a P2 D-deviations (+21% / +19% over QClaw 242 tolerance). All 14 method bodies moved verbatim — no behavior change, no method-body refactoring, no caller `use` updates needed (inherent dispatch across siblings per R20a pattern).

Canonical line counts (all `wc -l`):

| File | Action | Canonical wc-l | Cap (242) | Sub-domain |
|---|---|---:|---|---|
| `manager_config_loading.rs` | **NEW** | 93 | ✅ (under) | A + C: list + 3 helpers |
| `manager_config_requirements.rs` | **NEW** | 237 | ✅ (borderline) | B + D: probing + tool reg |
| `manager_connection_start.rs` | **NEW** | 227 | ✅ (under) | A: start lifecycle |
| `manager_connection_stop.rs` | **NEW** | 69 | ✅ (under) | B: stop lifecycle |
| `manager_config.rs` | **DELETED** | (was 292) | — | — |
| `manager_connection.rs` | **DELETED** | (was 287) | — | — |
| `client/mod.rs` | **MODIFIED** | +4 -2 | — | 4 new `mod` declarations |

**All counts via canonical `wc -l` per R18 addendum.**

---

## Per-method mapping (R20c spec §2.3)

### `manager_config_loading.rs` (4 methods)

| Method | Visibility | Verbatim source | Notes |
|---|---|---|---|
| `list_clients` | `pub async fn` | main `manager_config.rs:43-76` | Sub-domain A |
| `load_configs` | `pub async fn` | main `manager_config.rs:254-256` | Sub-domain C — **R20c-D1: must be pub, sibling consumer** |
| `load_config_file` | `pub async fn` | main `manager_config.rs:258-260` | Sub-domain C — **R20c-D1: must be pub, sibling consumer** |
| `load_config_value` | `pub async fn` | main `manager_config.rs:262-269` | Sub-domain C — kept pub per original; safe to keep |

### `manager_config_requirements.rs` (4 methods)

| Method | Visibility | Verbatim source | Notes |
|---|---|---|---|
| `probe_client_requirements` | `pub async fn` | main `manager_config.rs:78-153` | Sub-domain B |
| `refresh_remote_client_requirements` | `pub async fn` | main `manager_config.rs:155-172` | Sub-domain B |
| `probe_remote_client_requirements` | `pub async fn` | main `manager_config.rs:174-253` | Sub-domain B |
| `register_configured_tools` | `pub fn` | main `manager_config.rs:271-292` | Sub-domain D; the 23-line one QClaw flagged |

### `manager_connection_start.rs` (3 methods)

| Method | Visibility | Verbatim source | Notes |
|---|---|---|---|
| `initialize_all` | `pub async fn` | main `manager_connection.rs:47-74` | Sub-domain A |
| `start_client_for_session` | `pub async fn` | main `manager_connection.rs:76-90` | Sub-domain A |
| `start_client_connection` | `pub async fn` | main `manager_connection.rs:92-238` | Sub-domain A; the 147-line one QClaw flagged |

### `manager_connection_stop.rs` (3 methods)

| Method | Visibility | Verbatim source | Notes |
|---|---|---|---|
| `cleanup_failed_startup` | `pub async fn` | main `manager_connection.rs:240-247` | Sub-domain B |
| `stop_client` | `pub async fn` | main `manager_connection.rs:249-262` | Sub-domain B |
| `stop_connection` | `pub async fn` | main `manager_connection.rs:264-287` | Sub-domain B |

**Total: 14 methods preserved verbatim.** `pub async fn` count: 13, `pub fn` count: 1 (`register_configured_tools`). No `pub(super)` (R19 lesson).

---

## Spec deviations (R20c-D1)

### R20c-D1: Visibility table in spec §2.3 was wrong for `load_configs`/`load_config_file`

**Spec §2.3 said:**
> - `load_configs` — plain `async fn` (file-local, called by `list_clients`)
> - `load_config_file` — plain `async fn` (file-local, called by `load_configs`)

**Actual reality (caught at first `cargo check`):**
- `load_configs` is called from `probe_client_requirements` (manager_config_requirements.rs) and `initialize_all` (manager_connection_start.rs) via inherent dispatch
- `load_config_file` is called from `probe_remote_client_requirements` (manager_config_requirements.rs) via inherent dispatch

**Fix:** Made all 3 helpers `pub async fn` to preserve sibling inherent dispatch visibility. This matches the original verbatim visibility (these were already `pub async fn` in main `manager_config.rs`). Iron rule: 0 new unwrap/expect/panic added.

The spec author misread the call graph: the helpers are NOT just used internally — they form the inter-sibling dependency chain. Per R19 visibility lesson, **default to `pub fn`** unless file-local with no sibling consumer. R20c helpers DO have sibling consumers.

**Header comment in `manager_config_loading.rs` updated to document R20c-D1.**

---

## Baseline records

| Metric | Pre-split (main `f579c71`) | Post-split (HEAD) | Status |
|---|---:|---:|---|
| `northhing-acp` cargo check | 0 errors | 0 errors | ✅ |
| `northhing-cli` cargo check | 2 pre-existing E0624 | 2 pre-existing E0624 | ✅ (no new regression) |
| `northhing-cli` workspace check | 2 pre-existing E0624 | 2 pre-existing E0624 | ✅ (no new regression) |
| `northhing-acp` cargo test | 51 passed; 0 failed | 51 passed; 0 failed | ✅ |
| `northhing-core` cargo test | 899 passed; 0 failed; 1 ignored | 899 passed; 0 failed; 1 ignored | ✅ |
| `unwrap()` count (manager_config + manager_connection) | 0 | 0 | ✅ (Kimi Bug 3 PASS) |
| `expect()` count | 0 | 0 | ✅ (Kimi Bug 3 PASS) |
| `let _ = ` count | 4 | 4 | ✅ (Kimi Bug 3 PASS) |
| `panic!` count | 0 | 0 | ✅ |
| `unreachable!` count | 0 | 0 | ✅ |
| Cargo.lock drift | 0 lines | 0 lines | ✅ |
| Method count | 14 (8 config + 6 connection) | 14 (split across 4 files) | ✅ |

---

## 10-axis verification (R18 R(N)+1 standard)

| # | Axis | Result | Evidence |
|---|---|---|---|
| 1 | Line cap violations | ✅ All ≤242 | `wc -l` reports 93, 237, 227, 69 |
| 2 | Method count preserved | ✅ 14 = 14 | `grep -cE 'fn \w+'` sum = 4+4+3+3 = 14 |
| 3 | Visibility pattern | ✅ Default `pub` | 13 `pub async fn` + 1 `pub fn` + 0 `pub(super)` |
| 4 | Cargo.lock drift | ✅ 0 lines | `git diff main..HEAD -- Cargo.lock \| wc -l` = 0 |
| 5 | Tests pass | ✅ All baselines | acp 51/0/0, core 899/0/1 (R17 baseline preserved) |
| 6 | Iron rules | ✅ 0 new anti-pattern | Kimi Bug 3: unwrap 0=0, let _ 4=4, panic 0=0 |
| 7 | Format (`rustfmt --edition 2021 --check`) | ✅ Clean | 0 diff on the 4 new files |
| 8 | LF enforcement | ✅ 0 CRLF, all LF, 0 BOM | `file` reports ASCII text; first 3 bytes not BOM (47 47 32); last byte 10 |
| 9 | Line length | ✅ 0 long lines | All 4 new files have 0 lines > 120 chars |
| 10 | Cross-crate consumers preserved | ✅ 0 regressions | acp = 0 errors; cli/workspace = 2 pre-existing only; `git grep 'manager_config_*.rs::\|manager_connection_*.rs::'` across `src/apps/ src/web-ui/ src/mobile-web/` = 0 hits |

---

## Cross-crate consumer verification (R19 + R20a lessons — MANDATORY)

```text
cargo check -p northhing-acp           → 0 errors (target crate)
cargo check -p northhing-cli           → 2 pre-existing E0624 on session_manager.get_session (NOT R20c regression; pre-existing on main HEAD f579c71)
cargo check --workspace                → 2 pre-existing E0624 (same)

git grep 'manager_config_loading::'    -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/'  → 0 hits
git grep 'manager_config_requirements::' -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/' → 0 hits
git grep 'manager_connection_start::'  -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/'  → 0 hits
git grep 'manager_connection_stop::'   -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/'  → 0 hits
git grep 'manager_config::'            -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/'  → 0 hits
git grep 'manager_connection::'        -- 'src/apps/' 'src/web-ui/' 'src/mobile-web/'  → 0 hits
```

No NEW cross-crate refs to the new sub-domain modules. The split is pure file-organization; behavior unchanged.

---

## Visibility decisions documented (R19 lesson applied)

Per R19 P0 regression lesson (R19 spec over-prescribed `pub(super)` causing 11 E0624 errors in northhing-cli):

- **Default `pub fn` / `pub async fn`** for all externally-called methods
- **No `pub(super)`** anywhere in R20c
- **Inherent dispatch works across sibling files** without `use` imports (matching R20a pattern)

R20c-D1 spec deviation captured: the visibility table in the spec §2.3 listed `load_configs`/`load_config_file` as "plain async fn" but they have sibling consumers. Kept `pub async fn` to match the original verbatim. Header comment updated.

---

## Pre-existing E0624 (NOT R20c scope)

`cargo check -p northhing-cli` reports 2 pre-existing E0624 errors on `session_manager.get_session` (visibility = `pub(crate)` in `src/crates/assembly/core/src/agentic/session/session_manager_lifecycle.rs`). These pre-exist on main HEAD `f579c71` — R20c did NOT introduce them.

Per spec §5 risk table, **Mavis owns the follow-up commit** to fix the `pub(crate)` → `pub` visibility (same root cause as R20a `fe87083` and R20b `5424460`). Documented but NOT addressed in R20c commit to keep R20c scope-clean.

---

## Critical files

- `src/crates/interfaces/acp/src/client/manager_config_loading.rs` (NEW, 93 canonical wc-l)
- `src/crates/interfaces/acp/src/client/manager_config_requirements.rs` (NEW, 237 canonical wc-l)
- `src/crates/interfaces/acp/src/client/manager_connection_start.rs` (NEW, 227 canonical wc-l)
- `src/crates/interfaces/acp/src/client/manager_connection_stop.rs` (NEW, 69 canonical wc-l)
- `src/crates/interfaces/acp/src/client/mod.rs` (MODIFIED, +4 -2)
- `src/crates/interfaces/acp/src/client/manager_config.rs` (DELETED)
- `src/crates/interfaces/acp/src/client/manager_connection.rs` (DELETED)
- `docs/handoffs/2026-07-01-r20c-manager-config-connection-split-impl.md` (this file)

---

## Branch forking note

R20c is forked from main `f579c71` (R20a spec only). R20a (`impl/r20a-manager-session-split`) and R20b (`impl/r20b-manager-session-helpers-split`) branches are independent (different files). When all 3 branches land, Mavis handles the Mavis-fix follow-up commits (visibility fix on `session_manager.get_session`) — same 1-line fix in all 3 branches, no conflict.

---

## Iron rules compliance (R19 lessons applied)

- 0 new `unwrap()` (Kimi Bug 3 protocol: pre=post=0)
- 0 new `expect()` (pre=post=0)
- 0 new `let _ = Result` (pre=post=4; all pre-existing from manager_connection.rs lines 184/185/270/68, preserved verbatim)
- 0 new `panic!` / `unreachable!`
- 0 `#[allow(dead_code)]` (none added; some unused imports pruned per standard Rust hygiene)

---

*Producer: Coder agent. Date: 2026-07-01.*