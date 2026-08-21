# Disposable Agent Guide

Scope: this guide applies to `src/crates/contracts/disposable`.

`northhing-disposable` owns reusable, reversible registration and disposable RAII primitives (`Disposable`, `DisposableList`, `DisposalGuard`). Keep it dependency-light, thread-safe, and stable for cross-crate reuse.

## Guardrails

- Do not depend on `northhing-core`, runtime owner crates, service crates, transport adapters, or external crates.
- Keep additions limited to portable disposable lifecycle primitives.
- Must remain pure stdlib and panic-safe against lock poisoning.

## Verification

```bash
cargo test -p northhing-disposable
node scripts/check-core-boundaries.mjs
```
