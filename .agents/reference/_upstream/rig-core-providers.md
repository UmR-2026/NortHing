# Upstream: rig-core Provider Abstraction

> **Source:** WebFetch from `https://docs.rs/rig-core/latest/rig_core/providers/index.html` (2026-06-19).
> **Version referenced:** rig-core v0.38.2.
> **Purpose:** Reference for the northhing A5 (LLM provider abstraction).
> The northhing A5 already shipped its own `ModelClient` trait; this doc
> is a sanity check against an existing mature abstraction.

## Why this is here

The northhing A5 (`.agents/reference/session/`) chose an LLM provider
abstraction. Before adopting the same shape, it's worth looking at
how a mature community library does it. `rig-core` is the most
established Rust LLM-agnostic library as of mid-2026; its
provider abstraction is a useful comparison.

## rig-core's provider model

rig-core has three capability traits, implemented per-provider:

| Trait | Module | Purpose |
|---|---|---|
| `ProviderClient` | `client` | Base client; every provider implements it. |
| `CompletionClient` | `client::completion` | Chat completion. Implemented when the provider supports it. |
| `EmbeddingsClient` | `client::embeddings` | Text embeddings. Implemented when the provider supports it. |

> "Each provider module defines a Client type and model types for the
> capabilities it supports. Capability traits such as `CompletionClient`
> and `EmbeddingsClient` are implemented only when the provider
> declares that capability."

This is **capability-gated** — a provider without embeddings support
just doesn't implement `EmbeddingsClient`. The compiler enforces
"you can only call `.embeddings()` on a provider that supports it".

## Supported providers (rig-core v0.38.2)

| Provider | Notes |
|---|---|
| Anthropic | Native. |
| Azure OpenAI | Native. |
| ChatGPT | OAuth-backed (special). |
| Cohere | Native. |
| DeepSeek | Native. |
| Galadriel | Native. |
| Gemini | Native. |
| Groq | Native. |
| Hugging Face | Native. |
| Hyperbolic | Native. |
| Llamafile | Native. |
| **MiniMax** | Native. |
| Mira | Native. |
| Mistral | Native. |
| Moonshot | Native. |
| Ollama | Native. |
| OpenAI | Native. |
| OpenRouter | Native. |
| Perplexity | Native. |
| Together | Native. |
| Voyage AI | Native (embeddings). |
| xAI | Native. |
| Xiaomi MiMo | Native. |
| Z.ai | Native. |
| copilot (GitHub Copilot) | In module listing but not in prose list. |

That's **24 providers** as of v0.38.2. The breadth shows the value of
the abstraction: a community library can support 24 providers with a
uniform API surface.

## Usage shape (from the docs example)

```rust
// Client is ProviderClient; it implements CompletionClient
let openai = openai::Client::from_env()?;

// completion_model returns the CompletionModel bound to a concrete model id
let model = openai.completion_model(openai::GPT_5_2);

// The model is handed to the generic agent builder
let agent = AgentBuilder::new(model).preamble("...").build();
```

A shorthand `openai.agent(...)` is also provided. The pattern is:
1. Construct a provider-specific `Client` (often from env).
2. Call `client.completion_model(MODEL_CONST)` to get a typed model.
3. Hand the model to a generic agent / pipeline.

## How this compares to the northhing A5

| Aspect | rig-core | northhing A5 |
|---|---|---|
| Provider count | 24 | ~8 (Anthropic, OpenAI, Ollama, vLLM, …) |
| Capability traits | `ProviderClient` + `CompletionClient` + `EmbeddingsClient` | (Need to verify) |
| Configuration | `Client::from_env()` + per-provider module | (Need to verify) |
| Model ID | Module constants (e.g. `openai::GPT_5_2`) | (Need to verify) |

The northhing A5 spec at `docs/HANDOFF.md` mentions "Open provider
architecture via `ModelClient` trait" but does not enumerate the
capability split. If extending A5, consider:

1. **Capability-gated traits** (rig-core's pattern). If a provider
   doesn't support embeddings, it doesn't implement `EmbeddingsClient`.
   The northhing runtime can then type-check "this provider supports
   what I'm asking for" at compile time.
2. **Per-provider `from_env()` constructors** (rig-core's pattern).
   A new provider is one module, one `Client::from_env()`, and one
   capability-trait impl.
3. **Module-level MODEL_CONST** (rig-core's pattern). Model IDs are
   compile-time constants, not strings, so typos are caught at build.

## What this doc is NOT

- Not a recommended migration path. A5 is shipped; this is just
  background reading for future extensions.
- Not a substitute for reading rig-core's `mod.rs` source. The trait
  signatures are not in the page I fetched; the doc only links to them.
