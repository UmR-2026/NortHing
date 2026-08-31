# northhing Desktop Shell (Dioxus consult-room)

The Dioxus 0.8 consult-room shell — the primary human-facing entry point for
northhing. Single-process: the UI calls into `northhing-core` directly over
in-process function calls (no IPC companion process).

Historical note: this directory was previously home to a Slint + Material
shell, which was physically removed on 2026-08-28 (commit `707e414`). The
Dioxus consult-room shell is now the sole production desktop UI.

## Architecture

```
┌──────────────────────────────────────────┐
│  northhing (Dioxus consult-room shell)   │
│  ├── room window (main consult pane)     │
│  ├── inner window (overlay)              │
│  └── outer window (floating modules)     │
├──────────────────────────────────────────┤
│  northhing-core (business logic)         │
│  ├── agent-runtime                       │
│  ├── tool execution                      │
│  └── LLM adapters                        │
└──────────────────────────────────────────┘
```

The three windows cooperate via the consult-room module registry
(`src/apps/desktop/src/ui_dioxus/registry.rs`) and a watch-channel-based theme
and geometry bus (`src/apps/desktop/src/ui_dioxus/state.rs`).

## Build

```powershell
# Build desktop app
cargo build -p northhing

# Run
cargo run -p northhing
```

## Features

- **Three-window consult-room layout**: room (main pane), inner (overlay),
  outer (floating modules). All three driven from `ui_dioxus::launch()`.
- **Dioxus reactive UI**: Rust components rendered with the Dioxus 0.8
  desktop runtime; no `.slint` markup.
- **Self-drawn chrome + OS shadow**: frameless window with tao 0.16.2
  providing 8-way border resize, plus native drop shadow via
  `with_undecorated_shadow`.
- **Event bridge**: kernel events stream to the UI through the desktop event
  bridge (`src/apps/desktop/src/ui_dioxus/api_events.rs`).
- **i18n**: locale-pulled at startup (`ui_dioxus/i18n.rs`), three FTL
  catalogs (`zh-CN` / `en-US` / `zh-TW`).
- **Capability flags**: see `src/apps/desktop/src/flags.rs` (e.g.
  `DEFAULT_MODE_ID` for the skill panel). Legacy Slint-era flags
  (`USE_SLINT_SHELL` / `USE_SOFTWARE_FALLBACK` / `SKILL_INSPECTOR_ENABLED` /
  `SESSION_TREE_VIEW`) no longer exist.

## Dependencies

- `dioxus =0.8.0-alpha.1` (UI framework, `desktop` feature)
- `dioxus-logger =0.8.0-alpha.1`
- `tao` / `wry` (transitive via dioxus-desktop; window + webview)
- `northhing-core` (business logic)
- `tokio` (async runtime)

## File Structure

```
src/apps/desktop/
├── Cargo.toml
├── build.rs                 # Windows ComCtl32 v6 manifest embed
├── northhing.exe.manifest   # see comments inside for Slint-era context
├── northhing.rc             # Windows resource file
├── README.md                # this file
└── src/
    ├── main.rs              # Entry point; calls ui_dioxus::launch() unconditionally
    ├── lib.rs               # Re-exports
    ├── flags.rs             # Desktop const flags (presentation)
    ├── mcp_adapter.rs       # Adapter wrapping kernel_facade for the MCP catalog port
    ├── app_state/
    │   ├── log.rs           # Background debug-log forwarder
    │   ├── settings/        # AppSettings (UI-facing config)
    │   └── turn_runtime.rs  # Per-turn runtime state
    └── ui_dioxus/           # Dioxus consult-room shell
        ├── entry.rs         #   launch() + window builders
        ├── state.rs         #   Shared state (theme / geometry channels)
        ├── registry.rs      #   Module registry
        ├── app.rs           #   Room root
        ├── page_shell.rs    #   Shared page chrome
        ├── api*.rs          #   Bridge wrappers (api, api_events, api_fs, ...)
        ├── pages_*.rs       #   Route-level pages (onboarding, settings, archive, ...)
        ├── panel_files.rs   #   File-tree / preview panel
        ├── approval_card.rs #   Tool-confirmation card
        ├── turn_banner.rs   #   Turn-status banner
        ├── color.rs         #   Color helpers + tests
        ├── css.rs           #   Truth CSS (byte-locked) + variable CSS
        ├── css_files.rs     #   CSS file route table
        ├── i18n.rs          #   Locale pack + t() helper
        ├── window_ops.rs    #   Win32 window-ops FFI
        └── windows/         #   Per-window module split
            ├── mod.rs
            ├── self_app.rs
            ├── facility.rs
            └── work.rs
```