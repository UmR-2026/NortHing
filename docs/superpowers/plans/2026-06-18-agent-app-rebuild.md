<!-- LEGACY: 本文档是 v0.1.0 之前的历史计划，保留原 `agent-app` 名称作历史参考。
 Northing / 纳森 是 agent-app 的继任者（v0.1.0 之后改名）。
 本文件内容不被后续产品名替换脚本覆盖，保留 plan 当时的命名语境。 -->

# agent-app Rebuild Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **CONST-FLAG PATTERN:** every behavioral change ships behind a `const FLAG: bool = true;` gate + regression test + commit + PROJECT_STATE update, so it can be rolled back with one `git revert`.

**Goal:** Stop iterating agent-app-v3 as a fork. **Repurpose the v3 codebase as the starting point for a personal agent app** ("agent-app" working name) that the user owns end-to-end. Cherry-pick the universal crates (contracts, adapters, execution, services). Replace v3-specific parts (Tauri + React desktop, product-domain mini-apps, agent-app branding) with a Slint + Material desktop shell and a hidden internal CLI. Repo path is renamed from `agent-app-v3/` to `agent-app/`; v3 brand is removed; no upstream sync.

**Architecture:** 5 phases, each independently shippable. Mix of cherry-pick-and-keep, rewrite-from-scratch, and deprecation-and-remove. Each phase is one feature-flagged change set + tests + commit.

**Tech Stack:**
- Backend: Rust 2024 edition, workspace, same port-adapter layout as v3
- Frontend: **Slint** (declarative `.slint` markup) + **Material component library**
- Internal CLI: clap-based, hidden subcommand surface (`agent-app internal ...`), not advertised to end users
- LLM transport: HTTP only (no WebSocket); OpenAI-compatible protocol works for OpenAI / Anthropic / Ollama / vLLM
- Windows-first; cross-platform later

**Spec / Parent doc:** `docs/superpowers/plans/2026-06-18-agent-app-remake.md` (prior 5-phase fork-mode plan, kept as history)
**Already-shipped (commit `7a25b74` + `8bf283c` + `7afcdcb`):** Code-review P1-1, P2-2, P2-4, P2-5 fixes; PROJECT_STATE skill section; 5 superpowers workflow skills bundled in project `.agents/skills/`.

**Working directory:** `E:\agent-project\agent-app-v3` (git worktree on `v3-restructure` branch, **will be renamed to `agent-app` in Phase A0**)
**Toolchain:** `set PATH=C:\Users\UmR\.cargo\bin;C:\Users\UmR\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;%PATH%` *before every cargo command* — GNU toolchain ahead of MSVC in PATH breaks `getrandom`/`aws-lc-rs` with `dlltool.exe not found`.

---

## Phase Map (read this first)

| Phase | Topic | Action | Effort | Risk | ROI |
|---|---|---|---|---|---|
| **A0** | Repo rename + brand purge + Cargo workspace name | rename path + grep-replace strings | 0.5d | 🟢 Low (mechanical) | Unblocks everything else |
| **A1** | Slint desktop shell (replace Tauri + React) | delete + write from scratch | 4-5d | 🟡 Medium (UI logic) | **Visual identity** |
| **A2** | Cherry-pick universal crates (contracts, adapters, execution, services) | keep as-is, rename if needed | 1-2d | 🟢 Low | Reuses 821+ tests |
| **A3** | Internal CLI surface (`agent-app internal ...`) | new crate | 1-1.5d | 🟢 Low | Powers subagents + skills |
| **A4** | Skill system v2 (Markdown loader + registry) | rewrite `builtin_skills` against Slint UI | 2d | 🟡 Medium | Replaces v3's heavy catalog |
| **A5** | Multi-LLM provider abstraction | keep v3 adapter layer, remove product-specific models | 1d | 🟢 Low | Reuses 8 adapters |
| **A6** | Multi-session / multi-agent UI | new state management in Slint | 3d | 🟡 Medium | User-visible feature |
| **A7** | Deprecation sweep (remove product-domain mini-apps) | delete crates | 1d | 🟢 Low | Cuts binary size |
| **A8** | Verification + docs + first release | docs, CI, v0.1.0 tag | 1.5d | 🟢 Low | Ship signal |

**Sequencing rule:** A0 first (mechanical, blocks everything). A1 + A2 + A5 are mostly independent (different files), can run as parallel worktrees. A3 after A0 (needs `agent-app` CLI name). A4 + A6 after A1 (need Slint shell). A7 after A4+A6 (otherwise delete-but-keep risks broken references). A8 last.

---

## Phase A0 — Repo rename + brand purge

**Goal:** Every artifact that says "agent-app" says "agent-app" instead. Repo path moves from `E:\agent-project\agent-app-v3` to `E:\agent-project\agent-app`. No code changes, no behavior changes.

**Why first:** All later phases will add new files using the new name; mixing old and new names in one commit is messy.

### Tasks

- [ ] **A0.1** Pick the exact product name (`agent-app` working title) and write it down in `docs/agent-app-name.md` (5 lines: chosen name, CLI binary name, repo path, namespace in Cargo, log filename prefix).
- [ ] **A0.2** Move repo directory:
 ```powershell
 Rename-Item 'E:\agent-project\agent-app-v3' 'E:\agent-project\agent-app'
 ```
 Open any worktrees, update `.git/worktrees/<name>/gitdir` if needed. Re-open shell so PATH/CWD pick up new path.
- [ ] **A0.3** Grep + edit all "agent-app" / "agent-app" occurrences. Hot files (estimate counts from grep):
 - `Cargo.toml` (workspace + every crate): `name`, `[package]`, README
 - `src/apps/cli/src/main.rs` and `src/apps/desktop/`: binary name
 - `docs/PROJECT_STATE.md`, `docs/HANDOFF.md`, all `docs/superpowers/plans/*.md`: title + body
 - `README.md`, `CONTRIBUTING.md`, `LICENSE` (if it cites agent-app)
 - `.agents/skills/agent-app-v3-workflow/SKILL.md`: rename to `agent-app-workflow/SKILL.md`
- [ ] **A0.4** Update git remote (if any). If you forked from `obra/agent-app-v3`, remove that remote and any syncing machinery. This is the point of no return on the fork relationship.
- [ ] **A0.5** Update Cargo workspace binary names. The CLI binary becomes `agent-app`, the desktop binary becomes `agent-app-desktop` (delete or leave stub).
- [ ] **A0.6** Add `Cargo.toml` `[workspace.metadata]` with `name = "agent-app"`, `version = "0.1.0"`, `repository = ""`.
- [ ] **A0.7** Commit: `chore(rebrand): rename to agent-app (A0)` — single commit, all edits.
- [ ] **A0.8** Update `docs/PROJECT_STATE.md` "一句话状— and `HANDOFF.md` "Last updated" + "Total commits".

**Verification:**
```bash
git grep -in "agent-app" # should return 0 hits in tracked files (LICENSE / 3rd-party may remain)
git grep -in "agent-app" | wc -l # should be > 50
cargo metadata --format-version=1 | jq -r '.workspace_root' # should point to E:\agent-project\agent-app
```

**Rollback:** `git reset --hard HEAD~1`.

---

## Phase A1 — Slint desktop shell

**Goal:** Replace Tauri + React frontend with a Slint + Material desktop shell. The shell renders a single window: chat pane on the left, file/session pane on the right, status bar at the bottom. It calls into the existing `execution/agent-runtime` crate over an in-process channel (no IPC yet).

**Why second:** Without a UI, the user can't see anything; with only Tauri/React, the rebuild is a half-measure.

### Tasks

- [ ] **A1.0** Map the v3 React surface (read `apps/desktop/src/` tree, count files, list components). Identify what Slint must replicate: chat list, message bubble, file picker, session sidebar, settings modal.
- [ ] **A1.1** Decide shell architecture:
 - Option (a): Pure Slint window + Material library (1 process, no IPC).
 - Option (b): Slint window + thin Rust "tray + hotkey" companion process.
 - Default: **(a)** — fewer moving parts.
- [ ] **A1.2** Create `crates/ui/slint-shell/` workspace crate. Add `slint`, `slint-material`, `i-slint-core`, `i-slint-backend-winit` deps.
- [ ] **A1.3** Scaffold `crates/ui/slint-shell/ui/main.slint` with three regions (left sidebar, center chat, right inspector) and Material `NavigationDrawer`, `Card`, `Button`, `TextEdit` widgets. Render "agent-app v0.1.0" splash on first run.
- [ ] **A1.4** Wire entry point: `src/main.rs` in `slint-shell` opens the window, hooks `on_close_requested` to call into agent-runtime shutdown.
- [ ] **A1.5** Add Cargo `[[bin]] name = "agent-app"` in the new crate.
- [ ] **A1.6** Delete Tauri-side code: `src/apps/desktop/`, `src-tauri/`, `src/web-ui/`, `pnpm-lock.yaml` (if present), `apps/desktop/package.json`. Keep `apps/cli/` for now (Phase A3 turns it into hidden internal CLI).
- [ ] **A1.7** Update workspace `Cargo.toml` `members` list.
- [ ] **A1.8** Update `.gitignore` to drop `pnpm-debug.log*`, `dist/`, `dist-ssr/`, webview paths.
- [ ] **A1.9** Commit: `feat(ui): replace Tauri+React with Slint+Material shell (A1)`.
- [ ] **A1.10** Manual smoke test: `cargo run -p agent-app` opens a window, three regions visible, splash renders, no Tauri config files referenced anywhere.

**Verification:**
```bash
cargo build -p agent-app --release
./target/release/agent-app.exe # opens a window with sidebar + chat + inspector regions
git grep -i "tauri" | wc -l # should be 0
git grep -i "react" | wc -l # should be 0 in source code (docs OK)
```

**Pitfalls:**
- Slint Material is a community library (`slint-material` crate); pin a specific minor version because API changes.
- Windows MSVC toolchain + wgpu backend sometimes fails with "could not create surface"; fall back to `i-slint-backend-software` if wgpu panics.
- Slint markup does not support runtime conditionals the same way as React; use `if condition { ... } : ...` blocks.

**Rollback:** `git revert HEAD`.

---

## Phase A2 — Cherry-pick universal crates

**Goal:** Keep the v3 crates that solve universal problems. Drop crates that solve agent-app-specific product problems. Final shape is ~12 crates instead of 25.

**Why third:** The shell needs something to talk to. Cherry-picking now means A1's window can wire to a real agent runtime instead of a stub.

### Tasks

- [ ] **A2.1** Classify all 25 v3 crates into 3 buckets:
 - **KEEP** (cherry-pick, rename if needed): `contracts/*` (5 crates), `adapters/ai-adapters` (LLM providers), `execution/agent-runtime`, `execution/tool-execution`, `services/services-core` (process_manager), `services/services-integrations`
 - **DROP** (agent-app-specific product domains): `crates/contracts/product-domains/*` (mini-apps, PPT, podcast, video generation)
 - **REWRITE** (v3 shell + UI): `apps/cli/` — hidden internal CLI (Phase A3), `apps/desktop/` — deleted in A1
- [ ] **A2.2** For each KEEP crate, scan for agent-app-specific identifiers and rename: `agent-app` — `agent_app`, `agent-app` — `agent-app`. Do not change behavior.
- [ ] **A2.3** Delete DROP crates from `Cargo.toml` workspace `members`. Remove their references from any KEEP crate.
- [ ] **A2.4** Add `crates/agent-app-core/` that re-exports the public surface from KEEP crates. Acts as the single entry point for the UI shell.
- [ ] **A2.5** Add `Cargo.toml` feature flags so the shell can opt into specific providers:
 ```toml
 [features]
 default = ["provider-openai", "provider-anthropic", "provider-ollama"]
 provider-openai = []
 provider-anthropic = []
 provider-ollama = []
 ```
- [ ] **A2.6** Run full test suite. The 821+ v3 tests should still pass after rename.
- [ ] **A2.7** Commit: `refactor(core): cherry-pick 12 universal crates, drop product-domains (A2)`.

**Verification:**
```bash
cargo test --workspace --all-features
git grep "agent-app" | wc -l # only docs/CHANGELOG/history may remain
ls crates/ # count == 12-14, not 25
```

**Pitfalls:**
- `crates/contracts/product-domains` contains mini-app `Skill` definitions that some other crates import. Find every `use agent-app_product_domains::...` and replace with the bare tool/agent types.
- The `state.rs` ChatView God Object (314 lines, 36 fields) lives in `apps/cli/src/ui/chat/state.rs` — that's CLI, gets dropped in A3. Don't waste time refactoring it now.
- Tests reference internal paths like `crates/execution/tool-execution/src/pipeline.rs`; if any test still imports a DROP crate, fix the test before deleting.

**Rollback:** `git revert HEAD` then re-add deleted crates from git history.

---

## Phase A3 — Internal CLI surface

**Goal:** Replace `apps/cli/` (public CLI) with `agent-app internal ...` (hidden, capability-gated, same engine). Used by agent itself to spawn subagents and by skill authors to invoke tools headlessly.

**Why fourth:** Phase A1's shell calls the runtime directly (no IPC); Phase A3 introduces the IPC bridge that lets the shell and a subagent talk, and lets skills invoke the engine.

### Tasks

- [ ] **A3.1** Pick capability flags. Suggested:
 - `agent-app internal run --skill <name> --input <json>` — run a skill headless
 - `agent-app internal session list` — list sessions
 - `agent-app internal session new --model <id>` — start new session
 - `agent-app internal send --session <id> --prompt <text>` — send to running session
 - `agent-app internal tools list` — list registered tools
- [ ] **A3.2** Move `apps/cli/src/` to `crates/cli-internal/`. Rename binary to `agent-app-internal` (separate binary, hidden from default build).
- [ ] **A3.3** Replace v3 CLI flag parser with `clap` v4 derive macros. Keep subcommands but gate every one behind `InternalCommand` enum.
- [ ] **A3.4** Add capability check: every subcommand requires `AGENT_APP_INTERNAL_TOKEN` env var or `--internal-token <hex>` arg. Without it, exit code 77 ("capability denied") and print a one-line hint to read docs.
- [ ] **A3.5** Wire `agent-app internal run` to spawn an in-process agent runtime (not a new process). Subagent results come back as JSON to stdout.
- [ ] **A3.6** Add `crates/cli-internal/README.md` documenting the token mechanism and the threat model ("local-only; protects against accidental invocation, not malicious actors").
- [ ] **A3.7** Add `scripts/install-agent-app.ps1` that adds `crates/cli-internal/target/release/agent-app-internal.exe` to a non-PATH location (e.g., `%LOCALAPPDATA%\agent-app\bin\`). Never in user PATH.
- [ ] **A3.8** Commit: `feat(cli): internal CLI surface with token gate (A3)`.

**Verification:**
```bash
cargo build -p agent-app-internal --release
./target/release/agent-app-internal.exe # exit 77 + hint
AGENT_APP_INTERNAL_TOKEN=$(uuidgen) ./target/release/agent-app-internal.exe tools list
 # should print the tool list as JSON
git grep "agent-app-cli" | wc -l # 0
git grep "agent-app" apps/ | wc -l # 0
```

**Pitfalls:**
- Token check must run **before** anything else, including logging setup, to avoid leaking capabilities in error messages.
- The "hidden but usable" constraint means docs should NOT be linked from README. Put them in `docs/internal/cli.md` only.
- The shell (Phase A1) does not yet spawn subagents — A3 lays the API surface for A4/A6 to consume.

**Rollback:** `git revert HEAD`.

---

## Phase A4 — Skill system v2

**Goal:** Replace v3's heavy skill catalog (24 skills, ~12-15K tokens per turn listing them) with a Markdown-based loader that picks skills on-demand from disk + a small registry.

**Why fifth:** Skills are how the agent learns to use tools. Once the shell is up and the runtime is cherry-picked, skills become the next "where do capabilities live" decision.

### Tasks

- [ ] **A4.1** Audit v3's `crates/assembly/core/builtin_skills/`. For each of the 24 skills, decide:
 - **KEEP** as builtin (essential, e.g., `memory`, `file-search`)
 - **CONVERT** to on-demand loader (markdown file in `skills/`, loaded when prompt asks for it)
 - **DROP** (agent-app-product-specific, e.g., `ppt-generate`, `podcast-script`)
- [ ] **A4.2** Create `crates/skill-loader/` that reads `skills/<name>/SKILL.md` (Markdown + YAML frontmatter), parses the description and body, and exposes a `SkillRegistry` trait.
- [ ] **A4.3** Move 5-8 essential skills from v3's builtin to `skills/<name>/SKILL.md` in the new repo. Use the format from `.agents/skills/agent-app-v3-workflow/SKILL.md` as a template.
- [ ] **A4.4** Add `SkillRegistry::resolve_for_prompt(prompt: &str) -> Vec<SkillRef>`. Implementation: keyword match on `description` field against the prompt; return top 3 by score.
- [ ] **A4.5** Replace v3's `render_full_skill_listing_body` (12-15K tokens/turn) with `resolve_for_prompt` output. Const-flag: `USE_SKILL_REGISTRY: bool = true;` in `prompt_builder.rs`. On false, fall back to v3 listing for comparison.
- [ ] **A4.6** Wire the registry into the Slint shell: a "skills" tab in the right inspector lists currently-loaded skills, lets user enable/disable per-session.
- [ ] **A4.7** Add tests:
 - `resolve_for_prompt("debug my Rust code")` returns `[systematic-debugging]`
 - `resolve_for_prompt("make me a slide deck")` returns `[]` (PPT skill dropped)
 - Token count of resolved set — 5K per turn (vs v3's 12-15K)
- [ ] **A4.8** Commit: `feat(skills): markdown loader + on-demand registry (A4)`.

**Verification:**
```bash
cargo test -p skill-loader
cargo bench -p prompt-builder -- "skill_listing|skill_registry"
git grep "render_full_skill_listing" | wc -l # 0 in source (only in commit history)
```

**Pitfalls:**
- The keyword match in `resolve_for_prompt` is intentionally simple (TF-IDF or BM25). Don't over-engineer with embeddings — that adds a model dependency and 100ms+ latency per turn.
- Some v3 builtin skills have inline code (e.g., `memory` has 12K of Rust snippets). Don't try to port those; rewrite as markdown + pointers to source.
- Skill descriptions must stay short (— 200 chars). v3 habit of 500+ char descriptions is exactly what we're trying to fix.

**Rollback:** flip `USE_SKILL_REGISTRY = false`.

---

## Phase A5 — Multi-LLM provider abstraction

**Goal:** Keep v3's `adapters/ai-adapters` (OpenAI, Anthropic, Ollama, vLLM, etc.) but strip out anything agent-app-specific. Make it trivial to add a new provider.

**Why sixth (could merge with A2):** A2 cherry-picked the crate; A5 makes it ergonomic. A2 proves the code works; A5 makes it nice to use.

### Tasks

- [ ] **A5.1** Read `crates/adapters/ai-adapters/src/`. Count providers, count tests, count dead code.
- [ ] **A5.2** Drop v3-specific providers (if any): `agent-app-router`, internal mocks used only for E2E tests.
- [ ] **A5.3** Standardize provider trait. v3's `ModelClient` may have grown organically; rewrite as:
 ```rust
 #[async_trait]
 pub trait ModelClient: Send + Sync {
 fn id(&self) -> &str;
 async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse>;
 fn capabilities(&self) -> Capabilities;
 }
 ```
- [ ] **A5.4** Add `AgentAppProviderRegistry`: hash-map from `provider_id` — `Box<dyn ModelClient>`. Populated at startup from `[providers]` section in `agent-app.toml`.
- [ ] **A5.5** Add `agent-app.toml` schema (TOML). Reference example in `docs/config-example.toml`.
- [ ] **A5.6** Wire Slint shell: settings pane lists configured providers, "test connection" button calls a stub `complete()` and reports latency.
- [ ] **A5.7** Commit: `feat(providers): unified ModelClient trait + registry (A5)`.

**Verification:**
```bash
cargo test -p ai-adapters
cargo run -p agent-app -- --config docs/config-example.toml provider list
```

**Pitfalls:**
- v3 has a 200+ line `client.rs` with healthcheck logic using `TEST_IMAGE_PNG_BASE64`. Don't lose it; document why (vision-capability healthcheck ships in production).
- v3 used `aws-lc-rs` for some TLS paths — make sure the rename preserves all feature flags.
- Don't break the 821+ existing tests; A5 is refactor-only.

**Rollback:** `git revert HEAD`.

---

## Phase A6 — Multi-session / multi-agent UI

**Goal:** Sidebar lists all sessions, can switch between them, can spawn subagents that show as nested entries. Each session has its own conversation history, model selection, and skill set.

**Why seventh:** This is the user-visible "magic" feature that makes the app feel like a workspace, not a chat box. Comes after A4 (skills) and A5 (providers) so sessions can compose them.

### Tasks

- [ ] **A6.1** Design session state. v3's `SessionStorage` has a 46-file footprint and a missing `SessionStoragePathResolution` enum (was R3 in prior plan). For agent-app, design a flat `Session` struct:
 ```rust
 pub struct Session {
 pub id: SessionId,
 pub parent: Option<SessionId>,
 pub model_provider: String,
 pub model_id: String,
 pub skill_set: SkillSet,
 pub messages: Vec<Message>,
 pub created_at: DateTime,
 pub updated_at: DateTime,
 }
 ```
- [ ] **A6.2** Pick persistence: SQLite via `rusqlite`. Schema in `crates/session-store/migrations/0001_init.sql`. One table `sessions`, one `messages`.
- [ ] **A6.3** Create `crates/session-store/` with `SessionStore` trait + SQLite impl.
- [ ] **A6.4** Wire Slint sidebar: left pane lists sessions (tree view, indented for subagents), right pane shows session detail.
- [ ] **A6.5** Add "new session" and "spawn subagent" buttons. Subagent gets parent session's context (read-only) and its own skill set.
- [ ] **A6.6** Const-flag: `MULTI_SESSION_UI: bool = true;` in `slint-shell/main.slint` (Slint supports runtime conditionals). On false, single-session chat only.
- [ ] **A6.7** Commit: `feat(ui): multi-session sidebar + subagent spawning (A6)`.

**Verification:**
- Create session — appears in sidebar — send message — reply in chat pane
- Spawn subagent — child session appears under parent
- Close + reopen app — all sessions restored from SQLite
- Flip `MULTI_SESSION_UI = false` — app falls back to single-session chat (rollback test)

**Pitfalls:**
- Slint's reactive bindings can lag under high message throughput. Throttle redraws to 30fps.
- SQLite write contention: use WAL mode and batch writes per session tick (e.g., every 500ms or on idle).
- Subagent context inheritance is a footgun — over-share — token bloat; under-share — confused agent. Start with "inherit parent's first message only" and iterate.

**Rollback:** flip `MULTI_SESSION_UI = false`.

---

## Phase A7 — Deprecation sweep

**Goal:** Delete v3-specific product domains (mini-apps, content generation). Cuts binary size, removes maintenance burden, makes the repo "agent-app only".

**Why eighth:** Earlier phases may still reference deleted crates (for tests, examples, docs). Doing this last means we delete what's actually unused.

### Tasks

- [ ] **A7.1** Grep for any remaining references to `crates/contracts/product-domains` and `crates/execution/content-*`. If any references remain in active code, fix them.
- [ ] **A7.2** Delete directories:
 - `crates/contracts/product-domains/`
 - Any `crates/execution/content-*/`
 - Any `crates/integrations/specialized-*/` that depends on product domains
- [ ] **A7.3** Run `cargo build --workspace`. If anything fails, restore the offending crate from git history (means A2 missed a reference).
- [ ] **A7.4** Remove product-domain code from `Cargo.toml` `[workspace.members]`.
- [ ] **A7.5** Run `cargo build --release` and record binary size delta in commit message.
- [ ] **A7.6** Commit: `chore(deps): remove product-domain crates (A7)`.

**Verification:**
```bash
cargo build --workspace --release
ls -lh target/release/agent-app.exe # should be 30-50% smaller than A6 baseline
git grep "ppt-generate\|podcast-script\|video-script" | wc -l # 0
```

**Pitfalls:**
- Some v3 skills point to product-domain code. They were dropped in A4 — verify no orphan references remain.
- The `agent-app-v3-workflow` skill (now renamed `agent-app-workflow`) may have references to removed crates. Update its body.

**Rollback:** `git revert HEAD` — git will restore deleted directories.

---

## Phase A8 — Verification + docs + first release

**Goal:** Production-ready v0.1.0. CI green, docs accurate, install script tested, one happy-path walkthrough recorded.

**Why last:** Only after A0-A7 can we ship something a user (you) can use daily without finding broken edges.

### Tasks

- [ ] **A8.1** Add GitHub Actions (or local CI script) running `cargo test --workspace --all-features`, `cargo clippy -- -D warnings`, `cargo fmt --check`.
- [ ] **A8.2** Write `README.md`: project name, one-sentence description, screenshot of the Slint shell, 3-step install, 5-step first-session walkthrough.
- [ ] **A8.3** Write `docs/architecture.md`: the 14-crate layout, port-adapter boundaries, "where to add a new provider", "where to add a new skill", "where to add a new tool".
- [ ] **A8.4** Write `docs/agent-app-name.md` (already started in A0) with the final chosen name and rename history.
- [ ] **A8.5** Smoke test the full install path on a clean Windows machine (or VM): install — launch — create session — send message — spawn subagent — quit — relaunch (sessions persist).
- [ ] **A8.6** Tag `v0.1.0`. Write `CHANGELOG.md` entry.
- [ ] **A8.7** Update `docs/PROJECT_STATE.md` to reflect post-rebuild state. Update `HANDOFF.md` (or delete it — it's a fork-era document).
- [ ] **A8.8** Commit: `release: v0.1.0 (A8)`. Tag.

**Verification:**
- Clean machine can install in — 5 minutes
- All CI checks pass
- Binary opens, accepts input, persists state across restart
- Internal CLI works with token
- Multi-session + subagents work
- Skills load on demand

**Pitfalls:**
- The "clean machine" test is the one that's easy to skip — do it on a VM or Windows Sandbox.
- README screenshot: render the Slint shell once with `slint-viewer` to capture a clean shot.
- Don't write `docs/architecture.md` aspirationally — match the actual code. Run `cargo metadata --format-version=1 | jq` to count crates accurately.

**Rollback:** Tag is movable (`git tag -d v0.1.0 && git push --delete origin v0.1.0`); commit reverts as usual.

---

## Cross-cutting verification protocol

Run these after every phase commit:

```powershell
# 1. Build clean
$env:PATH = "C:\Users\UmR\.cargo\bin;C:\Users\UmR\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;$env:PATH"
Set-Location 'E:\agent-project\agent-app'
cargo build --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings

# 2. Brand hygiene
git grep -i "agent-app" # only docs/CHANGELOG may match
git grep "agent-app" | wc -l # growing monotonically

# 3. Pitfall checks
Get-ChildItem -Filter nul -Force -ErrorAction SilentlyContinue | Remove-Item -Force # phantom nul
ls -la $env:USERPROFILE\.cargo\registry # PATH order check
```

---

## Pitfall log (carry forward + new)

| Pitfall | Symptom | Fix |
|---|---|---|
| PATH order: GNU before MSVC | `dlltool.exe not found` during cargo build | Prepend MSVC toolchain path before any cargo command |
| Phantom `nul` file | `git add -A` fails "invalid path 'nul'" | `.gitignore` has `nul` (commit `8bf283c`) |
| Subagent tool access | Skill can't reach certain tool API | Const-flag pattern: gate, test, flip, commit, document |
| AskUserQuestion schema | 4 questions per call max, 4 options per question | Batch related questions; split unrelated across calls |
| Test assertion drift | Test asserts specific string, refactor renames field | Snapshot tests for output, not for implementation detail |
| PromptBuilderContext move errors | Compile error after refactor | Run `cargo check --workspace` after every signature change |
| Slint Material API changes | `slint-material` minor version bump breaks UI | Pin exact version in `Cargo.toml` |
| SQLite write contention | Multi-session UI freezes under load | WAL mode + batched writes per tick |
| Internal CLI token bypass | `--internal-token` accidentally in shell history | Token file at `$LOCALAPPDATA\agent-app\token`, not env |

---

## Out of scope (explicit YAGNI)

- **Linux / macOS first-class support.** Phase A0-A8 target Windows. Cross-platform is Phase B (separate plan).
- **Web UI / cloud sync.** This is a desktop app. If you want a web version, that's Phase C.
- **Mobile (iOS/Android).** Not on the table.
- **Marketplace / plugin distribution.** Skill loading from local markdown only.
- **Multi-user / auth.** Single-user desktop app. The "internal CLI token" is capability gating, not auth.
- **Telemetry / analytics.** None. Local-only by design.

---

## Decision log

| Date | Decision | Rationale |
|---|---|---|
| 2026-06-18 | **Repo rename** (agent-app-v3 — agent-app) | User explicit choice over "keep repo, swap theme" |
| 2026-06-18 | **GUI = Slint + Material** | User chose after seeing visual refs; Material library gives modern look fastest |
| 2026-06-18 | **CLI = hidden internal** | User said "GUI + 隐藏的cli，需要能完成正常的编程任务，cli无法通过正常手段调用" |
| 2026-06-18 | **Internal CLI for both subagents AND skill authors** | User picked "两者都— over single-purpose |
| 2026-06-18 | **Token gate is "deny-by-default capability check", not auth** | User clarified "只是表达 不指望用户用" |
| 2026-06-18 | **Windows-first, cross-platform later** | User explicit choice |
| 2026-06-18 | **Cherry-pick universal crates from v3** | User chose over "rewrite from scratch" — preserves 821+ tests |
| 2026-06-18 | **Skip `agent-app-remake.md` 5-phase plan** | That plan was fork-mode. Kept as history; this file supersedes it for active work |

---

## Related

- Prior fork-mode plan (kept for history): `docs/superpowers/plans/2026-06-18-agent-app-remake.md`
- Original v3 prompt loader plans: `docs/superpowers/plans/2026-06-17-v3-prompt-loader-impl*.md`
- CODE_REVIEW baseline: `CODE_REVIEW.md` (sections 3 and 7)
- Project state: `docs/PROJECT_STATE.md`
- Handoff doc: `HANDOFF.md` (will be retired in A8)
- Skills system: `.agents/skills/` (18 skills after commit `7afcdcb`)
