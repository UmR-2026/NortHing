[中文](AGENTS-CN.md) | **English**

# AGENTS.md

northhing is a Rust workspace plus React frontends.

Repository rule: **keep product logic platform-agnostic, then expose it through platform adapters**.

## Quick start

1. Read `README.md` and `CONTRIBUTING.md` before architecture-sensitive changes.
2. For desktop development, use `pnpm run desktop:dev` — it builds and runs the Dioxus consult-room desktop app (cold start, no HMR). Use `pnpm run desktop:check` for faster compile-only verification.
3. After Rust file changes, prefer `pnpm run fmt:rs` to format only changed or staged `.rs` files. Use `cargo fmt` only when you intentionally want broader formatting coverage.
4. After changes, run the smallest matching verification from the table below.

## Layered Module Index

Dependencies flow top to bottom. A layer may depend on lower layers only; keep
crate dependencies inside each layer to the smallest set needed.

| # | Layer | Path | Owns | Modules / entries | Layer doc |
|---|---|---|---|---|---|
| 1 | Interfaces and entrypoints | `src/apps/*`, `northing-installer`, `tests/e2e`, `src/crates/interfaces` | Product hosts, commands, UI entrypoints, protocol interfaces, and cross-surface tests | desktop, CLI, server, installer, E2E, `acp` | nearest local `AGENTS.md`; [interfaces](src/crates/interfaces/AGENTS.md) |
| 2 | Product assembly | `src/crates/assembly` | Compatibility exports, product capability selection, product-full wiring, and adapter/service registration | `core`, `product-capabilities` | [AGENTS.md](src/crates/assembly/AGENTS.md) |
| 3 | Adapters | `src/crates/adapters` | AI protocol adapters and external-provider translation | `ai-adapters` | [AGENTS.md](src/crates/adapters/AGENTS.md) |
| 4 | Services | `src/crates/services` | Reusable OS, filesystem, terminal, MCP, remote, git, watch, process, session persistence primitives, and network implementations | `services-core`, `services-integrations`, `terminal` | [AGENTS.md](src/crates/services/AGENTS.md) |
| 5 | Execution primitives | `src/crates/execution` | Portable agent, stream, DeepReview policy/report, typed-service, tool-contract, and tool-execution building blocks | `agent-dispatch`, `agent-runtime`, `agent-stream`, `tool-contracts`, `runtime-services`, `tool-execution` | [AGENTS.md](src/crates/execution/AGENTS.md) |
| 6 | Stable contracts and product domains | `src/crates/contracts` | Shared DTOs, event shapes, runtime ports, and product domain contracts/policies | `core-types`, `events`, `runtime-ports`, `product-domains` | [AGENTS.md](src/crates/contracts/AGENTS.md) |

Boundary rules:

- Interfaces and app entrypoints expose selected product behavior; reusable behavior moves down.
- Assembly wires lower layers and selects product capability facts; it must not implement concrete adapter, OS, or service details.
- Adapters translate protocols and external systems; they should not own product capability selection or reusable OS service behavior.
- Services implement reusable concrete OS, process, terminal, MCP, remote, git, and filesystem capabilities.
- Execution crates are portable runtime building blocks, not host-specific or delivery-profile owners.
- Contracts stay behavior-light and must not depend upward.


## Common commands

These are command references, not a pre-PR checklist. Use the Verification table
to choose the smallest local precheck; broad suites and builds are mainly for CI
reproduction or build-impacting changes.

```bash
# Install
pnpm install

# Dev
pnpm run desktop:dev               # build and run Dioxus consult-room desktop app (cold start)
pnpm run desktop:preview:debug     # alias: same as desktop:dev (cargo run -p northhing)
pnpm run dev:web                   # [missing: src/web-ui — not available in v0.1.0]
pnpm run cli:dev                   # CLI runtime

# Check
pnpm run fmt:rs                     # format only changed / staged Rust files
pnpm run lint:web                  # [missing: src/web-ui]
pnpm run type-check:web            # [missing: src/web-ui]
pnpm run i18n:contract:test          # i18n contract / resources only [frozen: i18n engineering]
pnpm run i18n:audit                  # i18n contract / resources only [frozen: i18n engineering]
pnpm run check:repo-hygiene
pnpm run check:github-config
cargo check --workspace

# Test (prefer focused paths locally; broad suites are CI-backed)
# [missing: src/web-ui — frontend test suite absent in v0.1.0]
cargo test --workspace                  # broad suite; CI-backed

# Build (only for build-impacting changes or CI reproduction)
cargo build -p northhing                 # build-impacting changes / CI reproduction
# [missing: src/web-ui — build:web not available]

# Fast builds (manual build/debug flows)
pnpm run desktop:build:fast           # debug build, no bundling
pnpm run desktop:build:release-fast   # release with reduced LTO
pnpm run desktop:build:nsis:fast      # Windows installer, release-fast profile
```

For the full script list, see [`package.json`](package.json).

## Global rules

### Housekeeping rules (2026-07-22, apply to every commit)

0. **Lazy Senior Dev Rule (YAGNI)**: Before writing code, climb this ladder:
   1. Does it need to be built? (YAGNI)
   2. Already in this codebase? Reuse it.
   3. Stdlib does it? Use it.
   4. Native platform feature? Use it.
   5. Installed dependency? Use it.
   6. Can this be one line? Make it one line.
   7. Only then: write the minimum code that works.
   (Never compromise security, error handling, or trust boundaries for brevity).
1. **顺手清配额**: a commit may include small in-scope debt fixes found nearby (outdated docs, missing tests, file growth past 800 lines) — no separate cleanup task needed; keep them traceable in the commit message.
2. **Doc sync as hard rule**: changing crate structure (add/remove crate, move paths) requires updating `docs/status/surfaces.md` in the same commit; resolving a tech-debt item requires flipping its ledger status in the same commit. No "doc later".
3. **God-file defense**: production `.rs` files over 800 lines raise review pressure; over 1000 lines must be split or carry a `// allow-god-file` justification comment at the top of the file. New modules start below the line.
4. **Concurrency test binding**: changes touching `tokio::select!`, cancellation tokens, or timeout races must ship with at least one automated test; judge review does not substitute. Other change types may rely on judge review.
5. **Coding curfew**: no coding work after 03:00 daily (user health rule, recorded 2026-07-22).
6. **Desktop compile gate before merging to main** (recorded 2026-08-06): `cargo check -p northhing` must pass on the branch tip before it merges to main, and a round handoff must not carry forward a verification baseline it did not measure itself. Reason: P1-C3 landed on main with the desktop crate not compiling at all (keyring feature missing) and it went unnoticed across a whole round because the report's verification section was incomplete and the next handoff reused a pre-C3 test figure. See `docs/status/tech-debt-ledger.md` P2-15.
7. **Rot budget only decreases**: `scripts/rot-budget.json` ceilings may only go down in normal commits; lowering is welcome in-scope (house rule 1). Raising any ceiling or adding a >800-line file manifest entry requires explicit user sign-off recorded in the commit message. The `dir-entry-count` metric for `.superpowers/sdd` uses cap-and-archive semantics (triggers archiving rotation when full, rather than strictly decreasing).
8. **Commit-bound workflow gate**:
   1. Task acceptance is bounded by BASE_SHA / TIP_SHA + the brief's allowlist; mechanical verification command: `node scripts/verify-task-gate.mjs verify-attempt --base <sha> --tip <sha> --allowlist <file>`, failing immediately on any out-of-bounds change.
   2. Continuation = new attempt: must have an independent brief (with its own BASE and allowlist); ex-post narrative expansion is not accepted.
   3. Review verdict state machine: PASS / FAIL / CANNOT_VERIFY / BLOCKED; CANNOT_VERIFY is tiered per `cannotVerifyPolicy` in `scripts/workflow-policy.json` (decisive evidence blocks; auxiliary evidence ≤2 items and not touching trust boundary ⇒ verdict capped at APPROVE_WITH_CONCERNS + owner + deadline); direct promotion to APPROVE is forbidden.
   4. Meta-ratchet: commits modifying any file listed in `metaRatchetPaths` of `scripts/workflow-policy.json` automatically escalate to the highest review lane (dual judges + user sign-off).
   5. `APPROVE_WITH_CONCERNS` is a first-class verdict: "cannot verify" is not penalized, but must specify an owner and a deadline.

### Internationalization

> **v0.1.0 status**: Desktop UI uses hardcoded Chinese. i18n engineering is frozen.
> `src/web-ui` is absent from this snapshot. The rules below apply when i18n is unfrozen.

- Locale ids, aliases, fallback rules, and surface defaults are owned by
  `src/shared/i18n/contract/locales.json`. Run `pnpm run i18n:generate`
  after editing it.
- Shared stable labels live in
  `src/shared/i18n/resources/shared/<locale>/terms.json`; workflow copy stays
  in the owning product surface.
- Do not import Web UI locale resources into smaller product surfaces such as
  `northing-installer`. See `docs/architecture/i18n.md`.
- Static self-contained pages may use generated page-scoped shared-term files;
  they must not import Web UI locale catalogs.
- Web UI loads only bootstrap namespaces eagerly; use `useI18n(namespace)` for
  route or feature copy and keep direct `i18nService.t(...)` calls in bootstrap
  namespaces.
- Use shared i18n formatting helpers for user-visible dates, times, and
  numbers instead of direct `Intl.*` or `toLocale*` calls.
- `pnpm run i18n:audit` enforces key/placeholder parity, direct static key
  existence, dynamic key source proofs, literal fallback and locale-format
  no-growth baselines, shared-term/l10n governance baselines, non-blocking
  same-text locale inventory, and the no-hardcoded-CJK source budget.

### Logging

Logs must be English-only, with no emojis.

- Frontend: `[missing: src/web-ui/LOGGING.md — absent in v0.1.0]`
- Backend: [`src/crates/LOGGING.md`](src/crates/LOGGING.md)

### Tauri commands (installer only)

> **v0.1.0**: The Dioxus consult-room desktop app does not use Tauri. These rules apply to `northing-installer/src-tauri` only.

- Command names: `snake_case`
- TypeScript may wrap with `camelCase`, but invoke Rust with a structured `request`

```rust
#[tauri::command]
pub async fn your_command(
    state: State<'_, AppState>,
    request: YourRequest,
) -> Result<YourResponse, String>
```

```ts
await api.invoke('your_command', { request: { ... } });
```

### Platform boundaries

- Do not call Tauri APIs directly from UI components; go through the adapter/infrastructure layer.
- Desktop-only host adapters belong in `src/apps/desktop`, then flow back through transport/API layers.
- In shared core, avoid host-specific APIs such as `tauri::AppHandle`; use shared abstractions such as `northhing_events::EventEmitter`.

### Remote compatibility

- When adding features, consider remote workspace and remote control synchronization support from the start. Local-only behavior can silently leave remote scenarios incomplete.
- If a feature cannot reasonably support remote workspaces, gate it or show a clear unsupported-state message instead of letting it fail with a generic error.

### Agent loop behavior

- Do not add hard-coded limits or pattern checks to the agent loop as a first response to looping behavior, such as blocking repeated tool calls by string or count alone.
- Excessive hard-coding turns the agent loop into a brittle workflow engine. Investigate the root cause first: tool behavior, model interaction, session context packaging, prompt/tool schema design, or state synchronization issues.

## Backbone invariants (verified 2026-07-17)

Change these only with a flag flip + integration test, and update this section in the same commit.

- **Desktop package is `northhing`**, not `northhing-desktop`. **唯一壳 = Dioxus consult-room（Slint 已于 2026-08-28 物理删除，回退 = git revert）**。agent-dispatch flags: only `USE_LIGHTWEIGHT_ACTOR = true` remains; Phase 3 IPC (USE_ONESHOT_DISPATCHER / USE_ACTOR_IPC / USE_DISPATCHER_IPC + IpcSpawnAdapter) descoped and deleted 2026-07-20.
- **Config single source of truth = core `GlobalConfig`** (`dirs::config_dir()/northhing/config/app.json`). Single source of truth for providers and default_model is core GlobalConfig (Stage 1 de-mirroring; core does not persist `api_key` to disk per user-approved Scheme C; desktop pushes keys to memory via facade on startup/change; desktop AppSettings retains workspaces/onboarding, Stage 2 to migrate). Never add a second runtime-readable config file.
- **UI thread discipline**: the legacy rule `writing Slint properties from a non-event-loop thread is silently dropped; route through `slint::invoke_from_event_loop` (see `ad349f9`)` is no longer applicable because the Slint shell was physically deleted on 2026-08-28 (commit `707e414`). The Dioxus consult-room shell follows its own runtime contract; refer to the Dioxus 0.8 docs and the consult-room `ui_dioxus::launch` path for the current discipline. The `error_banners.rs` helpers (`slint::invoke_from_event_loop` wrappers) were deleted together with the Slint shell (`707e414`); no replacement is needed because Dioxus writes go through its own reactive runtime.
- **Shell safety**: `guard_command_execution` is wired into the `validate_input` path of Bash/ExecCommand and writes audit entries (see `9a1575d`). New shell-like tools must call it too.
- **Project runtime slug always carries a path hash** (CJK paths must not collide, see `c7e7218`).
- **Installer toolchain**: `northing-installer` `[lib] crate-type = ["rlib"]` only (cdylib/staticlib blow past the GNU ld export-ordinal limit); `embed-resource` pinned to 3.0.5 (3.0.11 fails on rustc 1.96 MSVC). Desktop builds use MSVC; repo dir override is GNU and `cargo +toolchain` is unavailable — use `rustup run <tc> cargo`.
- **v0.1.0 surface baseline**: only Dioxus consult-room desktop + `northing-installer` are shipping surfaces; server / SDLC harness are frozen-experimental. Capability crates (tools/MCP/search/terminal/git/ssh) are the agent toolbox and stay active. See `docs/tech-debt-cleanup-guide.md` §0.

## Architecture

### Core decomposition guardrails

For any `northhing-core` decomposition, feature-boundary, dependency-boundary, or
Rust build-speed refactor, read
[`docs/architecture/core-decomposition.md`](docs/architecture/core-decomposition.md)
before editing. Keep this file as an entry point; put module-specific ownership
details in the nearest module `AGENTS.md`.

Repository-level decomposition rules:

- Do not confuse DTO/contract extraction with runtime owner migration.
- Product surfaces may diverge; share stable facts or ports, not UI, protocol,
  lifecycle, or platform implementation.
- Moving runtime ownership requires a reviewed port/provider design, old-path
  compatibility, behavior equivalence tests, and explicit confirmation when a
  behavior boundary could change.

### SDLC quality guardrails

For lifecycle evidence, gates, Artifact Graph, Project Profile, Deep Review
policy, OpenCode compatibility, or target-project governance changes, read
[`docs/sdlc-harness/README.md`](docs/sdlc-harness/README.md)
first, then [`docs/sdlc-harness/design.md`](docs/sdlc-harness/design.md). If
module boundaries or behavior change, follow the matching design under
`docs/sdlc-harness/architecture/` or `docs/sdlc-harness/features/`.

Do not hard-code northhing repository assumptions as target-project rules; keep
quality protection behavior target-aware, evidence-backed, risk-tiered,
cost-aware, and auditable.

## Verification

Run the smallest local precheck that matches the touched files. CI is expected to
cover full builds and broad test suites; run heavier local commands only when the
change directly affects build, packaging, or CI cannot protect the path.

| Change type | Minimum verification |
|---|---|
| Frontend UI, state, or adapters without i18n resource/contract changes | *[missing: src/web-ui — type-check:web not available in v0.1.0]* |
| Locale resource-only changes | *[frozen: i18n engineering — run if unfrozen]* |
| Locale contract or shared terms | *[frozen: i18n engineering — run if unfrozen]* |
| Web UI i18n runtime, namespace loading, or direct `i18nService.t(...)` usage | *[missing: src/web-ui — not available in v0.1.0]* |
| Shared Rust logic in `core`, adapters, or services | `cargo check --workspace`, plus the nearest focused `cargo test` when behavior changed |
| Desktop integration, Dioxus UI, browser/computer-use, or desktop-only behavior | `cargo check -p northhing`, plus focused desktop tests when behavior changed |
| Behavior covered by desktop smoke/functional flows | Prefer the nearest focused E2E/smoke check; rely on CI for broad build/test coverage unless build behavior changed |
| `src/crates/adapters/ai-adapters` | Relevant Rust checks above; add `cargo test -p northhing-agent-stream` only when stream contracts changed |
| Installer frontend or i18n runtime without packaging changes | `pnpm --dir northing-installer run type-check` |
| Installer Rust changes | `cargo check --manifest-path northing-installer/src-tauri/Cargo.toml` |
| Installer packaging, payload, install/uninstall flow, or native bundling | `pnpm run installer:build` |

凡本表与 `package.json`/`Cargo.toml` 实际不符的条目，以实际为准并当场修正本表。

## Agent-doc priority

Prefer the nearest matching `AGENTS.md` / `AGENTS-CN.md` for the directory you are changing. If local guidance conflicts with this file, follow the more specific, nearer document.
