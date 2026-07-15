# northhing Desktop Shell

Slint + Material GUI application - the primary human-facing entry point for northhing.

## Architecture

Pure single-process architecture: UI calls into `northhing-core` directly over in-process function calls. No IPC companion process.

```
┌─────────────────────────────────────────┐
│  northhing (Slint GUI)                  │
│  ├── Sidebar (sessions)                 │
│  ├── ChatPane (messages + input)        │
│  └── Inspector (skills + settings)      │
├─────────────────────────────────────────┤
│  northhing-core (business logic)        │
│  ├── agent-runtime                      │
│  ├── tool execution                     │
│  └── LLM adapters                       │
└─────────────────────────────────────────┘
```

## Build

```powershell
# Build desktop app
cargo build -p northhing

# Run
cargo run -p northhing
```

## Features

- **Three-region layout**: Sidebar (sessions), ChatPane (messages), Inspector (skills/settings)
- **Material Design**: Custom Material components with dark/light theme
- **Slint reactive UI**: Declarative `.slint` markup with Rust backend binding
- **Transport adapter**: `SlintTransportAdapter` bridges `AgenticEvent` to UI updates
- **wgpu + software fallback**: Auto-fallback if GPU rendering fails

## Rollback Flags

```rust
const USE_SLINT_SHELL: bool = true;        // Disable to compile stub
const USE_SOFTWARE_FALLBACK: bool = true;  // Disable wgpu fallback
const SKILL_INSPECTOR_ENABLED: bool = false;  // A4 will enable
const SESSION_TREE_VIEW: bool = false;        // A6 will enable
```

## Dependencies

- `slint` 1.16+ (UI framework)
- `northhing-core` (business logic)
- `northhing-transport` (event bridge)
- `tokio` (async runtime)

## File Structure

```
src/apps/desktop/
├── Cargo.toml
├── src/
│   ├── main.rs          # Entry point + core init
│   ├── app_state.rs     # Slint UI creation + callbacks
│   └── lib.rs           # Re-exports
└── ui/
    ├── main.slint       # Root window + theme
    ├── components/      # Material components
    └── views/           # Layout views
```
