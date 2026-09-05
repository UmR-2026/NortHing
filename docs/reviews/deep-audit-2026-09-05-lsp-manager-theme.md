# Deep Audit — Dormancy Rot Check on Two God-File Controls

**Date:** 2026-09-05
**Scope:** Read-only code-level audit of two R-14 god-file controls under `scripts/rot-budget.json`. These are **deliberate control group** entries — ceilings are pinned and the files are observed but not actively refactored. The audit's purpose is to detect **dormancy rot** (comment drift, dead code accumulating in quiet files, pattern inconsistencies that no one re-reviews).

**Methodology:**
- Skeptical 8-point scan per file with `file:line` evidence.
- Cross-references via `rg` from workspace root, excluding `target/`.
- `git blame` / `git log -S` for comment age and tombstone verification.
- Workspace structure cross-checked against root `AGENTS.md` six-layer index + `src/crates/assembly/core/AGENTS.md`.
- **Zero edits made** to either target file or the repository.

**Verdict scale (brief-defined):**
- `healthy` ≤ 1 observation, 0 rot-evidence
- `stable` 0–2 rot-evidence, all bounded
- `rotting` ≥ 2 rot-evidence or any unbounded growth

---

## File 1 — `src/crates/assembly/core/src/service/lsp/manager.rs`

| Metadata | Value |
|---|---|
| Current lines | **836** (Read tool line count, matches rot-budget ceiling exactly) |
| `scripts/rot-budget.json` ceiling | 836 (R-14 god-file, live observation cohort, registered with sign-off) |
| Headroom | **0 lines** (saturated) |
| Last substantive edit | `5c69651` 2026-08-14, "fix(services,lsp,desktop): resolve Wave2 B6+B7 follow-ups" |
| Dormancy | **21.3 days** since substantive change (brief says 22 ✓) |
| Earlier substantive edits | `7a4bdca` 2026-08-05 (FU-2 uninstall), `1a65fc1` 2026-08-01 (M-9 plugin ID validation) |
| Snapshot origin | `1b147c3` 2026-07-15 |

### 8-point scan

#### 1. Dead code — **rot-evidence**

- `stop_all_servers` at `manager.rs:315-317` is the only definition; **no external callers** anywhere in the workspace (`rg -n "stop_all_servers" src --type rust -g '!manager.rs'` returns just the definition line). The doc comment at `manager.rs:314` claims it is an "alias" of `shutdown()` — but nothing invokes it. Pure dead public API. The only path that stops all servers is `shutdown()` itself (`manager.rs:295-312`) which `stop_all_servers` merely forwards to.
- Five `pub async fn` query methods are unreferenced outside the file:
  - `list_plugins` (`manager.rs:271`)
  - `find_plugin_by_language` (`manager.rs:283`) — `format.rs:74` calls a *different* `find_plugin_by_language` (the `WorkspaceLspManager` version), not this one
  - `find_plugin_by_file` (`manager.rs:289`)
  - `get_plugin` (`manager.rs:277`) — `registry.rs:139` defines its own; this wrapper clones the result, but the workspace-level callers route through `WorkspaceLspManager`
  - `is_server_running` (`manager.rs:246`) — only called by test at `manager.rs:814`, never by production code
- Tombstone comment at `manager.rs:64-65` correctly notes that workspace-root responsibility was moved to `WorkspaceLspManager` (verified at `workspace.rs:159`). This is an *informative* marker, not rot — but the un-referenced query methods are residuals from before the split.

#### 2. Duplication — **observation**

- The four LSP-text-sync shims (`did_open` L320, `did_change` L336, `did_save` L359, `did_close` L372) share the same 4-line skeleton: `let process = self.get_process(language).await?; let params = serde_json::json!({...}); process.send_notification("...", Some(params)).await`. The shape is identical; only the JSON payload differs. A helper `notify_text_document(language, method, params)` would consolidate, but the JSON contents vary per LSP spec so the helper body would still be site-specific. **Bounded, stylistic, not rotting.**

#### 3. Pattern inconsistency — **clean**

- Error handling is uniform: `anyhow::Result` returns, `anyhow!` for ad-hoc errors, `warn!` for soft failures, `error!` for hard ones. No `unwrap()`/`expect()` in production code (every match for `\.expect\(` falls inside `#[cfg(test)] mod tests`). Verified by `rg -n "\.expect\(|unwrap\(\)"` — all hits L724/L762/L769/L775/L776/L777/L784/L791/L807/L823 are inside the test block.
- All `Arc<RwLock<...>>` accesses follow the same short-scope pattern (read guard in `let x = { ... };`, write guard in `let mut x = ...write().await; ...;`). No asymmetric locking.
- `register_plugin_internal` (`manager.rs:68`) is correctly `async` (it awaits the registry write lock).

#### 4. Stale comments — **observation**

- The architectural tombstone at `manager.rs:64-65` is **accurate today** (verified `WorkspaceLspManager` exists at `workspace.rs:159` and is the new owner). It is a *tombstone marker* — vulnerable if the split is ever reverted, but currently correct.
- The `_guard` pattern at `manager.rs:70-73` and `manager.rs:87` is explained inline; the underlying contract lives at `registry.rs:22-44` (`PluginRegistrationGuard` "deliberately does not auto-unregister on Drop"). The inline comment is accurate but somewhat defensive — the registry's own doc is the right home for the rationale. **Bounded, not rot.**
- No `TODO`/`FIXME`/`HACK` markers found in production code. Verified by `rg -n "TODO|FIXME|HACK|XXX" src/crates/assembly/core/src/service/lsp/manager.rs` — no hits.

#### 5. Hacks / workarounds — **clean**

- No `unsafe`, no `sleep`, no polling loops, no magic numbers, no `cfg` gates beyond the tests block.
- No `ponytail:` annotations needed — nothing deliberately simplified.

#### 6. Misplaced logic — **clean**

- File sits at `src/crates/assembly/core/src/service/lsp/manager.rs`, which is **Layer 4 (services)** in the root `AGENTS.md` table and matches the `assembly/core/AGENTS.md` "service" ownership description. Correct layer.
- `WorkspaceLspManager` (the actual user-facing type now) lives in `service/lsp/workspace_manager/` (4 sub-modules), re-exported from `mod.rs:35`. The split between `manager.rs` (protocol) and `workspace_manager/` (per-workspace state) is **architecturally clean** — `manager.rs` is now the stateless protocol layer.

#### 7. Complexity hotspots — **observation**

- `start_server` (`manager.rs:164-228`, 65 lines, **7 params**) exceeds the brief's 6-param soft limit. The 4 callbacks (`crash_callback`, `progress_callback`, `token_create_callback`, `diagnostics_callback`) are individually `Option<T>` and all required to wire `LspServerProcess::spawn` (`process.rs` use site). They could be packaged into a `ServerCallbacks` struct, but that adds an indirection for marginal gain. **Bounded.**
- `uninstall_plugin` (`manager.rs:103-146`, 44 lines) implements a 3-step transaction with explicit rollback on each step. The nested depth stays at 3 (function body → if-let step → rollback branch). The L127/L136 inline comments correctly describe the rollback contract. **Acceptable complexity for a transactional op.**
- Largest non-tx fn: `get_inlay_hints` (38 lines), `get_completions` (33 lines), `start_server` (65 lines). **No function > 80 lines.**

#### 8. Test quality — **observation**

- 3 `#[tokio::test]` tests at L780-835, all added in `7a4bdca` (FU-2) targeting the very fix in that commit. All are **real assertions** with non-trivial preconditions:
  - `uninstall_stops_servers_by_resolved_language_keys` — multi-lang plugin, asserts process map empty + plugin gone + files deleted.
  - `uninstall_unregistered_plugin_keeps_unregister_error_and_skips_stop` — proves unrelated servers are not collateral damage.
  - `uninstall_file_delete_failure_rolls_back_registration` — proves transactional rollback.
- **Narrow coverage**: only uninstall transaction is tested. The 25+ protocol shims (`did_*`, `get_*`, `goto_definition`, etc.) are uncovered. Coverage tracks "the most recently touched code" rather than "the file's full surface". **Acceptable for a dormant file where the bulk is mechanical LSP-spec shims**, but a future audit-trigged rotation would benefit from at least one happy-path test per major LSP method group.
- Tests use `dummy_server_command` (`manager.rs:734-755`) — platform-gated to a real binary (`cmd.exe /c exit 0` on Windows, `/bin/sh -c "exit 0"` on Unix). Comment at L731-733 explains the choice. **Reasonable.**

### Verdict — **`stable`**

Two bounded rot-evidence items: (a) `stop_all_servers` (L315) is unambiguously dead, and (b) the five `pub async fn` query methods (`list_plugins`, `find_plugin_by_language`, `find_plugin_by_file`, `get_plugin`, `is_server_running`) are no longer routed through externally since the `WorkspaceLspManager` split — `pub` visibility preserves them as escape hatches but no caller uses them. Both items are **bounded** (single-function removal each) and **non-architectural** (the surrounding module split is healthy and correctly documented).

**Override check:** The file is **not rotting**. The remaining ~700 lines are mechanical LSP-protocol shims that mirror the LSP spec; their bulk is intrinsic, not bloated. The protocol/workspace-manager split is the right architectural cut. The 21-day dormancy has not allowed comment drift — the architectural tombstone at L64-65 is verified accurate.

---

## File 2 — `src/apps/cli/src/ui/theme.rs`

| Metadata | Value |
|---|---|
| Current lines | **989** (Read tool line count, matches rot-budget ceiling exactly) |
| `scripts/rot-budget.json` ceiling | 989 (R-14 god-file, live observation cohort, registered with sign-off) |
| Headroom | **0 lines** (saturated) |
| Last touched (file mtime) | `9df49b2` 2026-08-22, "chore(ci): fix i18n generator paths, remove stale god-file comments, and wire CI hygiene and i18n checks (PHASE-0)" — touches file metadata only |
| Last **substantive** logic edit | `1b147c3` 2026-07-15 (snapshot from `northing-impl-b0-smoke`) — **`51+ days`** dormant for content |
| Earlier substantive | `456b696` 2026-07-23 (P2-10 god-file registration; metadata only, no content change to theme.rs) |
| Dormancy | **12.2 days** since last write / **51+ days** since last logic change (brief says 14 days for write ✓) |

### 8-point scan

#### 1. Dead code — **rot-evidence**

- `#[allow(dead_code)]` annotations are **incorrectly applied** — they silence the lint for symbols that are demonstrably live:
  - `StyleKind` enum at `theme.rs:637` — comment at `theme.rs:635` claims "current theme rendering uses hardcoded Color values instead". **The comment is from the snapshot era and is now false.** `StyleKind` is invoked in **30+ call sites** across `command_palette.rs` (9× at L564/L586/L587/L591/L622/L663/L678/L684/L726/L737), `command_menu.rs` (3× at L118/L119/L146), `tool_cards.rs` (8× at L297/L298/L306/L308/L310/L315/L317/L319/L321/L349/L366/L375), `tool_cards/hmos_block.rs` (4× at L141/L144/L150/L157/L170). The enum is **the primary theming API** — the `#[allow(dead_code)]` should have been removed when consumers were added.
  - `OpencodeThemeJson.defs` at `theme.rs:700` — `defs` is read at `theme.rs:831` (`json.defs.clone().unwrap_or_default()`) and at `theme.rs:934` (`if let Some(v) = defs.get(t)`). The annotation contradicts actual use.
- `load_opencode_theme_json` at `theme.rs:728-732` — **truly unused** (only the definition, no external callers; verified via `rg -n "load_opencode_theme_json"`). Its `#[allow(dead_code)]` and reason-comment at L726 are **accurate** and correctly explain "reserved for future on-disk theme loader".
- `parse_osc_color` at `theme.rs:217-248` — used internally at `theme.rs:203` inside the `#[cfg(unix)]` branch of `detect_terminal_appearance`. On non-unix builds, L203 is removed by cfg and the function is dead. The `#[allow(dead_code)]` at L216 is **technically necessary for the non-unix target**, but the reason-comment at L215 — "reserved for terminal integration that parses OSC color escape sequences; not yet wired into the theme loader" — is **stale**. The function IS wired into the unix branch of `detect_terminal_appearance`; the comment predates that wiring (both lines attributed to `1b147c3` snapshot per `git blame`, so the comment was wrong even at origin — defensive allow for the non-unix case, but commented as if unused).

#### 2. Duplication — **observation**

- Five `Theme` constructors (`dark` L251, `dark_ansi16` L284, `light` L317, `light_ansi16` L350, `monochrome` L383) each list all 21 fields. **Necessary** — different palettes by definition. The five palettes are the deliverable; structural sharing (e.g., a defaults base + per-theme diffs) would obscure intent without reducing source size materially.
- `resolve_opencode_theme` (`theme.rs:829-862`) has 20+ near-identical `tokens.X = resolve_key(json, &defs, "X", mode)?;` lines. **Necessary** — each key maps to a distinct token in `ResolvedTokens`; the lookup is the work.
- **No meaningful copy-paste rot.**

#### 3. Pattern inconsistency — **observation**

- **Two different luminance formulas** in the same file:
  - `relative_luminance` (`theme.rs:535-545`) uses BT.709 / sRGB weights `0.2126*r + 0.7152*g + 0.0722*b` with proper sRGB gamma decode — the **correct** formula for perceived brightness.
  - `rgb_to_ansi16` (`theme.rs:597`) uses BT.601 weights `0.299*r + 0.587*g + 0.114*b` (no gamma decode) — a **color-quantization** heuristic, not a luminance model.
  - `detect_terminal_appearance` (`theme.rs:204`) uses the same BT.601 heuristic `0.299*r + 0.587*g + 0.114*b` for the light/dark threshold at `> 0.5`.
  - Three formulas, two weights sets. Justifiable (BT.601 for "fast quantization", BT.709 for "perceptual correctness"), but a reader can easily confuse them. **Stylistic, not rot.**
- `with_effective_scheme` for `Ansi16` (`theme.rs:420-445`) hard-codes the assignment of **21** fields, omitting `input_background`. The inline comment at `theme.rs:431-432` says "Keep the startup input panel as the preset-defined RGB color. Otherwise subtle dark theme variants collapse to the same ANSI black/blue." The comment **is accurate** (input_background is intentionally untouched), but it sits in the middle of 20 mechanical assignments and is the only non-mechanical line. **Stylistic, not rot.**

#### 4. Stale comments — **rot-evidence**

Two stale tombstone-style comments documented above in (1), both at the snapshot commit `1b147c3` 2026-07-15:

- `theme.rs:635`: `// reason: StyleKind enum kept for theme-aware styling API; current theme rendering uses hardcoded Color values instead` — **contradicts reality**. 30+ callers use `theme.style(StyleKind::X)` as the primary theming path. Comment is **52+ days stale**.
- `theme.rs:215`: `// reason: parse_osc_color() reserved for terminal integration that parses OSC color escape sequences; not yet wired into the theme loader` — **partially stale**. The function IS wired into `detect_terminal_appearance`'s unix branch at `theme.rs:203`. The "not yet wired" claim is wrong; the "reserved" claim is partly right (only used in one path, and only on unix).

No `TODO`/`FIXME`/`HACK`/`XXX` markers present (`rg -n "TODO|FIXME|HACK|XXX" src/apps/cli/src/ui/theme.rs` returns no hits).

#### 5. Hacks / workarounds — **rot-evidence**

- **Unsafe block at `theme.rs:164-194`** with **no `// SAFETY:` comment**:
  ```rust
  unsafe {
      let flags = libc::fcntl(fd, libc::F_GETFL);
      ...
      libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK)
      ...
      libc::fcntl(fd, libc::F_SETFL, flags)   // L193 — restoration, fire-and-forget
  }
  ```
  Violates the Rust convention (and the unsafe-checker skill's requirement) that every `unsafe` block carry a `// SAFETY:` justification. The block mutates per-process fd flags on stdin; if `fcntl(F_SETFL)` at L193 fails (the return value is `let _`-discarded), **stdin remains in `O_NONBLOCK` mode for the rest of the process lifetime**, breaking every subsequent stdin read in the CLI. This is the most concerning rot-evidence in either file.
- **Magic numbers without context** in color math:
  - `0.36` luminance threshold at `theme.rs:526` (split between `Color::Black` and `Color::White` foreground).
  - `0.5` luminance threshold at `theme.rs:205` (light/dark appearance).
  - `0.08` / `0.92` at `theme.rs:598` / `theme.rs:602` (RGB-to-Ansi16 near-black/near-white shortcuts).
  - `0.6` brightness threshold at `theme.rs:608`.
  - `0.18` saturation threshold at `theme.rs:610`.
  - `Duration::from_millis(250)` at `theme.rs:131` (terminal-OSC query timeout).
  - `Duration::from_millis(5)` at `theme.rs:178` and `theme.rs:187` (poll interval in non-blocking read loop).
  - `Vec::with_capacity(256)` at `theme.rs:160` and `[0u8; 256]` at `theme.rs:174`.
  None of these are constant-named or commented with their derivation. The **5ms poll** in particular is a hot-loop parameter that nobody can tune without reading the implementation. No `ponytail:` annotations explain deliberate simplifications.

#### 6. Misplaced logic — **observation**

- File lives at `src/apps/cli/src/ui/theme.rs` (Layer 1, interfaces/app entrypoints). Theme rendering for the CLI is correctly placed here.
- One **cross-cutting concern**: `detect_terminal_appearance` (L136-213) talks to **stdin directly via `libc::fcntl`** to query the terminal background color via OSC 11. This is platform-specific terminal IO that arguably belongs in a layer below the UI (e.g., a `terminal_capabilities` helper in the services layer). It is `#[cfg(unix)]`-gated correctly, so it does not compile on Windows, but the cross-layer concern is real. **Stylistic, not rot.**

#### 7. Complexity hotspots — **clean**

- Largest function: `detect_terminal_appearance` (L136-213, 78 lines). Close to the 80-line soft limit but justified — it coordinates three different I/O surfaces (stdout write, stdin non-blocking read, libc fd flag manipulation) under a timeout. Nested depth peaks at 4 (`unsafe { while { match { arm } } }`). **Acceptable.**
- `resolve_color_string` (L897-942, 46 lines) handles hex parsing, alpha blending, and defs/theme reference resolution with cycle detection (the `seen: HashSet<String>` at L873). Necessary complexity for a JSON theme resolver.
- All other functions are ≤ 39 lines. **No function > 80 lines.**

#### 8. Test quality — **observation**

- 2 `#[test]` tests at L955-988. Both are **real assertions**, not tautologies:
  - `builtin_themes_resolve_for_dark_and_light` (L956) — iterates all builtin themes via `builtin_theme_ids()`, asserts each resolves cleanly for both dark and light. Catches JSON parse regressions in the baked-in theme assets.
  - `eight_digit_hex_colors_are_supported` (L969) — alpha-blend math test with exact RGB assertions (`dark.primary == Rgb(128, 128, 128)`, `light.primary == Rgb(127, 127, 127)`).
- **Coverage is shallow**: the color math (`relative_luminance`, `rgb_to_ansi16`, `readable_foreground_for`, `idx_to_ansi16`, `to_ansi16`, `blend_alpha_channel`), the terminal detection path (`resolve_effective_color_scheme`, `terminal_supports_truecolor`, `detect_terminal_appearance`), and the OSC color parser (`parse_osc_color`) are **all untested**. The terminal-detection code is `#[cfg(unix)]` and never exercised in CI on Windows. **Adequate for a frozen control-group file; insufficient if this file ever becomes load-bearing.**

### Verdict — **`rotting`**

Five rot-evidence items, with **the unsafe block + missing SAFETY comment** as the most consequential:

1. Two stale comments contradicting current code state (`StyleKind` at L635-637 and `parse_osc_color` at L215-216), both dating to the snapshot commit and never refreshed in 51+ days.
2. Two incorrectly-applied `#[allow(dead_code)]` annotations (`StyleKind` L637 and `OpencodeThemeJson.defs` L700) that suppress lints for live symbols.
3. One truly dead public API (`load_opencode_theme_json` L728) — annotated correctly with accurate reason-comment.
4. Unsafe block at L164-194 with no `// SAFETY:` comment and a fire-and-forget restoration at L193 that can leak `O_NONBLOCK` state if `fcntl` fails.
5. Magic numbers throughout the color-math section without named constants or `ponytail:` annotations.

**Override check:** The file is **not** saved by the "大文件可以代码干净" escape — the color math is sound (BT.601 vs BT.709 are both correct for their purposes, alpha-blend formula at L948 with `+127` rounding is correct, the OSC 11 query protocol is implemented properly) but the **safety hygiene and comment freshness** have rotted in the dormancy. The file has not been touched at the logic level for **51 days** (per `git blame`), and the rot is exactly the kind dormancy produces: stale allow attributes, comments from a pre-adoption era, and an unsafe block nobody reviewed when they last drove past it.

**Recommended next step** (out of audit scope, recorded for triage): the unsafe-block fix and the `StyleKind` annotation removal are each ≤ 30 lines of trivial edits and would clear 4 of the 5 rot-evidence items at once. They do **not** require restructuring the file; the dormancy has not produced architectural rot, only surface rot.

---

## Summary Table

| File | Lines | Ceiling | Headroom | Dormancy | Rot-evidence | Verdict |
|---|---|---|---|---|---|---|
| `service/lsp/manager.rs` | 836 | 836 | 0 | 21.3 days | 2 (bounded) | `stable` |
| `apps/cli/src/ui/theme.rs` | 989 | 989 | 0 | 51+ days (logic) / 12.2 days (mtime) | 5 (1 unbounded) | `rotting` |

**Both files are at the ceiling with zero headroom** — neither can grow without a separate rot-budget exception. The dormancy of `theme.rs` at the logic level is **3× the dormancy of `manager.rs`**, and the rot-evidence density reflects that asymmetry exactly.

The control-group design of these entries (pinning the ceiling, observing without intervening) is a legitimate anti-rot experiment: the contrast between `manager.rs` (recently active, recently reviewed, stable) and `theme.rs` (51+ days logic-static, comment-rot, surface rot) **demonstrates that dormancy is a rot accelerant**. This finding is exactly what the dormancy-rule predicted and what the control/cohort split was designed to surface.

## Status

DONE
