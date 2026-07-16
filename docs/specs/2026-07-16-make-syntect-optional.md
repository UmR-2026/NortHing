# Spec: Make syntect optional in northhing-cli

## Goal
Make `cargo test --workspace` pass by eliminating the `onig_sys` C dependency from the default CLI build path.

## Root Cause
- `onig_sys` (C Oniguruma) is pulled in by `syntect` → `fancy-regex` → `onig`
- When `cargo test --workspace` unifies features, `onig_sys` gets compiled with MSYS2 GCC and causes `0xC0000139 (STATUS_ENTRYPOINT_NOT_FOUND)` at runtime
- `syntect` + `syntect-tui` are used by `northhing-cli` only for syntax highlighting
- Making them optional with a fallback eliminates the C dependency while preserving functionality

## Changes Required

### 1. Modify `src/apps/cli/Cargo.toml` (lines 49-51)

**Before:**
```toml
# Syntax highlighting for code blocks and tool cards
syntect = { workspace = true }
syntect-tui = { workspace = true }
```

**After:**
```toml
# Syntax highlighting for code blocks and tool cards
# Optional: gates onig_sys (C Oniguruma) to avoid MSYS2 GCC entry-point issues
syntect = { workspace = true, optional = true }
syntect-tui = { workspace = true, optional = true }
```

### 2. Add feature gate in `src/apps/cli/Cargo.toml` [features] section

**Before:**
```toml
[features]
default = []
```

**After:**
```toml
[features]
default = ["syntax-highlight"]
syntax-highlight = ["dep:syntect", "dep:syntect-tui"]
```

### 3. Gate all syntect imports and usage in `src/apps/cli/src/` with `#[cfg(feature = "syntax-highlight")]`

Files to check:
- Search for `syntect` in all `.rs` files under `src/apps/cli/src/`

Each usage must be gated:
- `use syntect::...` → `#[cfg(feature = "syntax-highlight")] use syntect::...`
- Functions that use syntect types → gate with `#[cfg(feature = "syntax-highlight")]`
- Call sites that call gated functions → provide `#[cfg(not(feature = "syntax-highlight"))]` fallback

### 4. Acceptable fallback behavior when `syntax-highlight` is disabled

- Code blocks render as plain text (no syntax highlighting)
- No crash, no error — just uncolored output
- CLI help, session management, MCP, web fetch, all other features work identically

## Verification

```bash
# 1. Default build still works (with syntax-highlight)
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo check -p northhing-cli

# 2. Build WITHOUT syntax-highlight (no onig_sys compiled)
cargo check -p northhing-cli --no-default-features

# 3. Full workspace test
cargo test --workspace

# 4. CLI still runs
cargo run -p northhing-cli -- --help
```

## Done Criterion
- `cargo check -p northhing-cli` (default) → `Finished` ✅
- `cargo check -p northhing-cli --no-default-features` → `Finished` ✅
- `cargo test --workspace` → no `0xC0000139`, all tests pass or only pre-existing failures
- Code with `#[cfg(feature = "syntax-highlight")]` compiles cleanly with both feature on and off

## Hard Rules
- Do NOT modify any file outside `src/apps/cli/`
- Do NOT remove syntect — only make it optional
- Do NOT break the default build (`cargo check -p northhing-cli` must still work)
- Long-line ≤5 per file, cap 120 chars

## Review
- After coder completes, reviewer must verify BOTH feature configurations compile
- Run `cargo check -p northhing-cli` AND `cargo check -p northhing-cli --no-default-features`
