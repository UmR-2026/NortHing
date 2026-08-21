# Surface Status Ledger

> Single source of truth for what ships, what's frozen, and what's experimental.
> Aligned with `AGENTS.md` § "Backbone invariants" and `docs/tech-debt-cleanup-guide.md` §0.
> Update this file in the same commit that changes a surface's status.

## Shipping Surfaces (v0.1.0 baseline)

| Surface | Crate / Path | Toolchain | Status | Notes |
|---------|-------------|-----------|--------|-------|
| **Slint Desktop** | `src/apps/desktop` (`northhing`) | MSVC | ✅ Active | Primary user-facing surface. Slint UI + agent runtime. |
| **Installer** | `northing-installer/` | MSVC (rlib only) | ✅ Active | `embed-resource` pinned 3.0.5. `[lib] crate-type = ["rlib"]` only. |

## Frozen-Experimental Surfaces

These compile and may have partial functionality, but are **not** shipped, not tested in CI for user-facing flows, and may break without notice.

| Surface | Crate / Path | Status | Notes |
|---------|-------------|--------|-------|
| **CLI** | `src/apps/cli` (`northhing-cli`) | 🧊 Frozen | Compiles; no release artifact. `doctor` command has false positives. See tech-debt-ledger P2. |
| **Server** | `src/apps/server` | 🧊 Frozen | HTTP API surface; no auth layer. Not deployed. |
| **Tauri Desktop (candidate)** | `src/apps/desktop-tauri` | 🧊 Frozen | Tauri 2 + React candidate for the next baseline; flips at F4. src-tauri is its own cargo workspace (excluded from main). |

## Active Capability Crates (Agent Toolbox)

These are not user-facing surfaces but are actively maintained as the agent's tool layer:

| Crate | Path | Role |
|-------|------|------|
| `tool-contracts` | `src/crates/execution/tool-contracts` | Tool trait definitions |
| `tool-execution` | `src/crates/execution/tool-execution` | Tool execution engine |
| `agent-dispatch` | `src/crates/execution/agent-dispatch` | Agent dispatch (lightweight actor mode) |
| `agent-runtime` | `src/crates/execution/agent-runtime` | Agent runtime loop |
| `agent-stream` | `src/crates/execution/agent-stream` | Streaming response handling |
| `runtime-services` | `src/crates/execution/runtime-services` | Runtime support services |
| `services-core` | `src/crates/services/services-core` | Core services |
| `services-integrations` | `src/crates/services/services-integrations` | Integration services |
| `terminal` | `src/crates/services/terminal` | Terminal service |
| `debug-log` | `src/crates/services/debug-log` | Debug-mode runtime logging leaf crate (`log_event` + `COMP_*` constants); shared by desktop and core, re-exported from core (K4a-T5) |
| `ai-adapters` | `src/crates/adapters/ai-adapters` | AI provider adapters |
| `kernel-api` | `src/crates/contracts/kernel-api` | Kernel facade contracts — product surfaces reach core only through this facade (K1) |
| `acp` | `src/crates/interfaces/acp` | ACP interface |
| `product-capabilities` | `src/crates/assembly/product-capabilities` | Product capability assembly |
| `product-domains` | `src/crates/contracts/product-domains` | Product domain contracts |
| `core-types` | `src/crates/contracts/core-types` | Core type definitions |
| `events` | `src/crates/contracts/events` | Event contracts |
| `runtime-ports` | `src/crates/contracts/runtime-ports` | Runtime port contracts |
| `disposable` | `src/crates/contracts/disposable` | Reversible registration and disposal RAII primitives (PCS-1) |
| `assembly-core` | `src/crates/assembly/core` | Core assembly |
| `cli-internal` | `src/crates/support/cli-internal` | CLI internal utilities |
| `test-support` | `src/crates/support/test-support` | Test utilities |

## Change Protocol

1. **Promoting frozen → shipping**: Requires CI green, user-facing test pass, auth/timeout review, and a release note.
2. **Demoting shipping → frozen**: Update this file, add a release note, and tag the last-good commit.
3. **New surface**: Add a row with `🧊 Frozen` by default. Promote only after review.
