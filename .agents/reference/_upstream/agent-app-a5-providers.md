# Agent-app A5 — LLM Provider Abstraction (Actual Implementation)

> **Source:** `src/crates/adapters/ai-adapters/src/providers/`
> (last synced: 2813b36 / v3-restructure, 2026-06-19).
> **Purpose:** Document the actual A5 implementation, then compare to
> rig-core's provider abstraction. This is the local-source counterpart
> to `.agents/reference/_upstream/rig-core-providers.md`.

## Why this is here

HANDOFF.md says A5 is "Open architecture LLM provider abstraction"
but doesn't link to a trait definition. A direct search for
`ModelClient` or `ProviderClient` returns nothing in the workspace.
This file records the **actual** A5 shape so future implementers
don't try to find a `ModelClient` trait that doesn't exist.

## What's actually in the providers module

```
src/crates/adapters/ai-adapters/src/providers/
├── mod.rs                    (re-exports 3 modules: anthropic, gemini, openai, shared)
├── shared.rs                 (cross-provider helpers: header policy, body trim, log)
├── anthropic/
│   ├── mod.rs
│   ├── discovery.rs          (model catalog discovery)
│   ├── message_converter.rs  (northhing message format → Anthropic wire format)
│   └── request.rs            (Anthropic-specific request builder)
├── gemini/                   (same 4-file shape)
└── openai/                   (same 4-file shape)
```

**3 providers today:** Anthropic, Gemini, OpenAI-compatible.

## How the abstraction is actually structured

There is **no `ProviderClient` trait**. Each provider is an independent
module with its own:

- `discovery.rs` — returns the catalog of model IDs the provider supports.
- `message_converter.rs` — converts from northhing's internal
  `Message` representation to the provider's wire format.
- `request.rs` — builds the provider-specific HTTP request.

**Cross-provider sharing is in `shared.rs`:** a set of `pub(crate)` helper
functions (`apply_header_policy`, `apply_custom_headers`,
`protect_request_body`, etc.) that all three providers call into. This
is the actual "abstraction" — a shared utility module, not a trait.

## Comparison with rig-core

| Aspect | rig-core | northhing A5 |
|---|---|---|
| **Provider count** | 24 | 3 (Anthropic, Gemini, OpenAI) |
| **Abstraction mechanism** | `ProviderClient` + capability traits (`CompletionClient`, `EmbeddingsClient`) | Per-provider module + shared `pub(crate)` helpers |
| **Model IDs** | Module constants (`openai::GPT_5_2`) | (discovered dynamically via `discovery.rs`) |
| **Configuration** | `Client::from_env()` per provider | `AIClient` HTTP client + per-provider config struct |
| **Capability gating** | Trait not implemented if unsupported | n/a (each provider is monolithic) |
| **Customization** | `AgentBuilder` composes model + preamble | n/a (provider modules are hand-written) |

### Trade-offs

| Approach | Pros | Cons |
|---|---|---|
| **rig-core (trait-based)** | Adding a provider is one module + trait impl. Type system enforces capability gating. Mature community. | Heavier upfront. Per-provider configuration lives outside the trait. |
| **northhing A5 (module-based)** | Lightweight. Each provider is a self-contained module. Easy to read. | Adding a provider means writing 4 new files. No compile-time check that the provider supports what you ask for. Duplication risk across providers. |

## How to add a new provider in northhing

The A5 style requires:

1. Create `src/crates/adapters/ai-adapters/src/providers/<name>/` with:
   - `mod.rs` — module declaration + re-exports
   - `discovery.rs` — list the models
   - `message_converter.rs` — convert messages
   - `request.rs` — build the request
2. Add `<name>;` to `providers/mod.rs`.
3. Update shared helpers if the new provider needs new header / body logic.
4. If the new provider has a fundamentally different wire shape (e.g.
   gRPC, WebSocket), the current module-based shape may not fit — at
   that point, **consider** introducing a `ProviderClient` trait.

## What to copy from rig-core

If you decide to introduce a `ProviderClient` trait in northhing:

1. **Start with `ProviderClient` + `CompletionClient` capability split.**
   rig-core's split is well-considered; copy it.
2. **Use `Client::from_env()` constructors** for the env-var-based
   common case.
3. **Reuse `shared.rs` helpers** as a free-function module — they
   don't need to become trait methods.
4. **Capability-gate `EmbeddingsClient`**: only implement it for
   providers that have embeddings.

What to **NOT** copy from rig-core:

1. **24-provider breadth.** northhing's 3 is fine for now; don't try
   to support every provider upfront.
2. **`AgentBuilder`.** northhing's prompt assembly is different; don't
   introduce a parallel agent-builder abstraction.
3. **The shorthand `client.agent(...)` method.** It's a convenience
   for one-shot use; not needed if you already have a builder.

## Files to consult when extending providers

| You need to… | Read |
|---|---|
| Add a new provider in A5 style | `providers/mod.rs` + one of `{anthropic,gemini,openai}/` |
| Add a new cross-provider helper | `providers/shared.rs` |
| Add a new `ProviderClient` trait (if going that route) | `.agents/reference/_upstream/rig-core-providers.md` |
| Understand message conversion | one of `*/message_converter.rs` |
| Understand request building | one of `*/request.rs` |

## Status

- **3 providers** (Anthropic, Gemini, OpenAI-compatible).
- **No `ModelClient` trait** despite HANDOFF's wording — the abstraction
  is module-based with shared helpers.
- **No `ProviderClient` trait** either. This is a divergence from
  rig-core; document it explicitly because future implementers will
  expect a trait and not find one.
